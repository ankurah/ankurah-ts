//! The small pieces of TypeScript the translator writes by hand.
//!
//! For: a translator is mostly decisions, and a few of them come out as one
//! short piece of text — a quoted string, an operator, an immediately-called
//! function, the parentheses a numeric literal receiver needs. Each is one
//! rule, each is read by several of the modules above, and none of them is
//! about the shape of the Rust that reached it.

use super::{indent, BodyTranslator};
use crate::name_map;

/// The BASE of a postfix expression, parenthesised where JavaScript would read
/// it differently from Rust.
///
/// Every postfix form asks this — a method receiver, a field's base, the base
/// of an index or a slice, and the callee of a direct call — because what makes
/// the parentheses necessary is the base, not what follows it. Applied to `.`
/// alone, `get_vec().await[0]` came out `await getVec()[0]`, which indexes the
/// promise and answers `undefined`, and `get_function().await(8)` called it.
///
/// Two rules. A numeric literal: `0xFFu8.wrapping_sub(b)` is written
/// `255.wrappingSub(b)`, and JavaScript reads the `.` after a digit as a decimal
/// point rather than a member access. Rust has no such ambiguity, so nothing in
/// the source says the parentheses are needed; the literal is what says it. And
/// an `await`, below.
pub(crate) fn parenthesise_receiver(receiver: &syn::Expr, written: String) -> String {
    // Rust's `.await` is postfix and binds tighter than whatever follows it;
    // JavaScript's `await` is a PREFIX operator that binds LOOSER than member
    // access. So `parse(s).await.unwrap()` came out `await parse(s).unwrap()`,
    // which JavaScript reads as `await (parse(s).unwrap())` — `unwrap` asked of
    // the promise, which has none. 52 sites, core's `lineage.test.ts` and
    // storage-sqlite's `engine.ts` among them.
    if written.starts_with("await ") {
        return format!("({})", written);
    }
    let is_number = match receiver {
        syn::Expr::Lit(lit) => matches!(lit.lit, syn::Lit::Int(_) | syn::Lit::Float(_)),
        // `(-1).abs()`: the minus is part of the literal as far as this is
        // concerned, and the whole of it needs the parentheses.
        syn::Expr::Unary(unary) => matches!(
            (&unary.op, &*unary.expr),
            (syn::UnOp::Neg(_), syn::Expr::Lit(lit))
                if matches!(lit.lit, syn::Lit::Int(_) | syn::Lit::Float(_))
        ),
        _ => false,
    };
    if is_number && !written.starts_with('(') {
        return format!("({})", written);
    }
    written
}

/// The names a `let` pattern introduces, in the TypeScript spelling.
/// The closure a callee expression is, where it is one written in place.
pub(crate) fn as_closure(expr: &syn::Expr) -> Option<&syn::ExprClosure> {
    match expr {
        syn::Expr::Closure(closure) => Some(closure),
        syn::Expr::Paren(p) => as_closure(&p.expr),
        syn::Expr::Group(g) => as_closure(&g.expr),
        _ => None,
    }
}

/// The same, restricted to a `move` closure, which is the only kind that takes
/// its captures by value.
pub(crate) fn as_move_closure(expr: &syn::Expr) -> Option<&syn::ExprClosure> {
    as_closure(expr).filter(|closure| closure.capture.is_some())
}

/// The names a pattern binds, in the TypeScript spelling they are emitted
/// under.
pub fn pattern_names(pat: &syn::Pat) -> Vec<String> {
    bound_names(pat)
}

pub(crate) fn bound_names(pat: &syn::Pat) -> Vec<String> {
    let mut out = Vec::new();
    collect_bound(pat, &mut out);
    out
}

pub(crate) fn collect_bound(pat: &syn::Pat, out: &mut Vec<String>) {
    match pat {
        // A SCREAMING_SNAKE_CASE name in a pattern is a CONST, not a binding:
        // Rust compares the subject against its value. Read as a binding, the
        // arm owned a value nothing declared — `match p { ORIGIN => true, .. }`
        // released `oRIGIN`, an identifier the emitted file never introduces.
        // The convention is the one every other path in this file already
        // reads a const by; `names_a_const` asks the registry where it can, and
        // this function has no registry to ask.
        syn::Pat::Ident(ident)
            if crate::body::pat_shape::names_a_constant(&ident.ident.to_string()) =>
        {
            if let Some((_, sub)) = &ident.subpat {
                collect_bound(sub, out);
            }
        }
        syn::Pat::Ident(ident) => {
            out.push(name_map::escape_reserved(&name_map::to_camel_case(
                &ident.ident.to_string(),
            )));
            if let Some((_, sub)) = &ident.subpat {
                collect_bound(sub, out);
            }
        }
        syn::Pat::Tuple(t) => t.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::TupleStruct(t) => t.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::Slice(s) => s.elems.iter().for_each(|p| collect_bound(p, out)),
        syn::Pat::Struct(s) => s.fields.iter().for_each(|f| collect_bound(&f.pat, out)),
        syn::Pat::Reference(r) => collect_bound(&r.pat, out),
        syn::Pat::Type(t) => collect_bound(&t.pat, out),
        syn::Pat::Paren(p) => collect_bound(&p.pat, out),
        // Every alternative of an or-pattern binds the SAME names — rustc
        // refuses one that does not — so the alternatives together introduce
        // one set of names, and reading all of them listed each name once per
        // alternative. The scope then claimed `literal` twice and released it
        // twice, which the strict registry aborts on. One alternative is the
        // whole answer.
        syn::Pat::Or(or) => {
            if let Some(first) = or.cases.first() {
                collect_bound(first, out);
            }
        }
        _ => {}
    }
}

/// What `e?` became: the statements that test it, the expression that stands in
/// its place, and the `Result` wrapper — which a statement-position `?` has to
/// release, because nothing downstream consumes it.
pub(crate) struct Lowered {
    pub(crate) declaration: String,
    pub(crate) value: String,
    /// The `Result` this `?` tested, where there is one to release. An
    /// `Option` has none: the port writes it as a nullable.
    pub(crate) wrapper: Option<String>,
    /// The identifier the declaration introduced, whatever it holds. Read on
    /// the path where the statement refused, where nothing consumed it (I4).
    pub(crate) temp: Option<String>,
}

/// The type a call's turbofish names: `from_str::<EntityId>(s)` says which
/// type is being read out of the parsed value.
/// The type a turbofish names, as the syntax wrote it.
pub(crate) fn turbofish_written(callee: Option<&syn::Path>) -> Option<syn::Type> {
    let segment = callee?.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    }
}

pub(crate) fn turbofish_type(callee: Option<&syn::Path>) -> Option<String> {
    let segment = callee?.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(ty) => Some(name_map::map_type(ty)),
        _ => None,
    }
}

/// Is this the `+=` family, which reads a place and writes back to it?
pub(crate) fn is_assign_op(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Does this expression name storage that already exists, rather than produce
/// a value? Rust drops what a statement produced and nothing else, so only the
/// second kind needs a release written for it.
pub fn is_place(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(_) | syn::Expr::Field(_) | syn::Expr::Index(_) => true,
        syn::Expr::Unary(u) => matches!(u.op, syn::UnOp::Deref(_)) && is_place(&u.expr),
        syn::Expr::Reference(r) => is_place(&r.expr),
        syn::Expr::Paren(p) => is_place(&p.expr),
        syn::Expr::Group(g) => is_place(&g.expr),
        // `x?` hands back what was inside a Result the lowering already
        // consumed; `x.await` takes the future by value. Neither leaves a value
        // behind for the statement to release.
        syn::Expr::Try(t) => is_place(&t.expr),
        _ => false,
    }
}

/// Look through the wrappers a binding can be written behind — `let mut x`,
/// `let x: T`, `let (x)` — to whatever it really binds.
pub fn strip_binding(pat: &syn::Pat) -> &syn::Pat {
    match pat {
        syn::Pat::Type(t) => strip_binding(&t.pat),
        syn::Pat::Paren(p) => strip_binding(&p.pat),
        other => other,
    }
}

/// A scope holding one pattern's bindings; it closes when this drops.
///
/// It also carries the TYPE of the value being taken apart, restored when the
/// scope closes: a match over an owned value written inside a borrowed one
/// answers for itself.
pub struct PatternScope<'t, 'a> {
    pub(crate) translator: &'t BodyTranslator<'a>,
    pub(crate) subject_before: Option<crate::ty::Ty>,
}

impl Drop for PatternScope<'_, '_> {
    fn drop(&mut self) {
        *self.translator.subject_ty.borrow_mut() = self.subject_before.take();
        self.translator.pop_scope();
    }
}

// ── Standalone helpers ──────────────────────────────────────────────────

/// What `*x = y` falls back to when the engine cannot say what `x` wraps.
pub(crate) const ASSUMED_ACCESSOR: &str = "the wrapper accessor is assumed to be `value`";

/// Path segments dropped when a written path becomes a TypeScript expression.
/// Resolving the path properly is the value-namespace work in the engine; this
/// list is what stands in for it, and dropping any of them is recorded.
pub(crate) const STD_QUALIFIERS: [&str; 11] = [
    "std", "core", "alloc", "sync", "collections", "convert", "fmt", "ops", "iter", "atomic",
    "marker",
];

/// Does this `let` pattern bind a name the body writes to?
///
/// I7: only `Pat::Ident` was read, and an ANNOTATION wraps the pattern —
/// `let mut t: u32 = 0` is a `Pat::Type` whose inner pattern carries the `mut`.
/// So the annotated form emitted `const t = 0` and every later `t = ..` was a
/// `TypeError: Assignment to constant variable`. A `ref` pattern wraps it the
/// same way.
pub(crate) fn is_mut_binding(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(ident) => ident.mutability.is_some(),
        syn::Pat::Type(annotated) => is_mut_binding(&annotated.pat),
        syn::Pat::Reference(reference) => is_mut_binding(&reference.pat),
        _ => false,
    }
}

/// A block written as an immediately-called arrow function.
///
/// JavaScript's `await` belongs to the nearest function, so a block that awaits
/// becomes an `async` arrow and the call is awaited where it stands — otherwise
/// the value is a promise nobody unwrapped, and TypeScript refuses the `await`
/// inside outright.
pub(crate) fn iife(params: &str, body: &str, args: &str, awaits: bool) -> String {
    if awaits {
        format!("await (async {} => {{\n{}}})({})", params, indent(body), args)
    } else {
        format!("({} => {{\n{}}})({})", params, indent(body), args)
    }
}

/// One text value as a single-quoted TypeScript literal.
///
/// Rust's `'\''` and `"a\\b"` and `'\0'` are ordinary values by the time syn
/// hands them over — a quote, a backslash, a NUL — and writing them back out
/// between quotes without escaping them again produced `'''`, `'a\b'` and a raw
/// NUL in the middle of a source file. ankql's SQL renderer, which escapes SQL
/// quotes by writing `'\''`, is where that showed: 106 parse errors from one
/// unescaped character.
pub fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // NUL goes out as `\u{0}` and NOT as `\0`: `\0` followed by a
            // decimal digit is a LEGACY OCTAL escape, which a JavaScript engine
            // reads as a different character in sloppy mode and refuses
            // outright in a module. Rust's `"nul\01digit"` is NUL then `'1'`;
            // `'nul\01digit'` is U+0001 then `digit`, and Node ESM will not
            // parse it at all. It falls out of the control-character arm below,
            // which is where every other one goes.
            // A lone surrogate or a control character has no printable form;
            // `\u{..}` is what both TypeScript and Rust write.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// A literal as the port writes it, or `None` where the port has no spelling
/// for that literal form and the site has to say so.
pub(crate) fn translate_lit(lit: &syn::Lit) -> Option<String> {
    Some(match lit {
        syn::Lit::Str(s) => quoted(&s.value()),
        syn::Lit::Int(i) => i.base10_digits().to_string(),
        syn::Lit::Float(f) => f.base10_digits().to_string(),
        syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
        syn::Lit::Char(c) => quoted(&c.value().to_string()),
        syn::Lit::Byte(b) => format!("{}", b.value()),
        // `b"abc"` is a `&[u8; 3]`, and the port writes a byte sequence as a
        // `Uint8Array`. It used to come out as a comment, which is not an
        // expression at all.
        syn::Lit::ByteStr(bytes) => {
            let values: Vec<String> =
                bytes.value().iter().map(|b| b.to_string()).collect();
            format!("new Uint8Array([{}])", values.join(", "))
        }
        _ => return None,
    })
}

/// Check if an expression references a variable name as a standalone path
/// (not as a field name in `a.field`). Used for shadow detection.
pub(crate) fn references_var(expr: &syn::Expr, name: &str) -> bool {
    match expr {
        syn::Expr::Path(path) => {
            // Standalone variable reference: just the name
            path.path.segments.len() == 1
                && path.path.segments[0].ident == name
        }
        syn::Expr::MethodCall(call) => {
            // Check receiver and args, but NOT the method name
            references_var(&call.receiver, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Call(call) => {
            references_var(&call.func, name)
                || call.args.iter().any(|a| references_var(a, name))
        }
        syn::Expr::Field(field) => {
            // Check the base, but NOT the field name
            references_var(&field.base, name)
        }
        syn::Expr::Binary(bin) => {
            references_var(&bin.left, name) || references_var(&bin.right, name)
        }
        syn::Expr::Unary(unary) => references_var(&unary.expr, name),
        syn::Expr::Reference(r) => references_var(&r.expr, name),
        syn::Expr::Paren(p) => references_var(&p.expr, name),
        syn::Expr::Block(b) => {
            b.block.stmts.iter().any(|s| match s {
                syn::Stmt::Expr(e, _) => references_var(e, name),
                _ => false,
            })
        }
        syn::Expr::Closure(c) => references_var(&c.body, name),
        _ => false,
    }
}

/// What the position wants of a shift's left operand: everything it wants of
/// the shift, because a shift hands back the type of the value it shifted.
pub(crate) fn shift_expectation<'e>(
    op: &syn::BinOp,
    expected: Option<&'e crate::ty::Ty>,
) -> Option<&'e crate::ty::Ty> {
    matches!(op, syn::BinOp::Shl(_) | syn::BinOp::Shr(_))
        .then_some(expected)
        .flatten()
}

pub(crate) fn translate_binop(op: &syn::BinOp) -> &'static str {
    match op {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::BitXor(_) => "^",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::BitOr(_) => "|",
        syn::BinOp::Shl(_) => "<<",
        syn::BinOp::Shr(_) => ">>",
        syn::BinOp::Eq(_) => "===",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Ne(_) => "!==",
        syn::BinOp::Ge(_) => ">=",
        syn::BinOp::Gt(_) => ">",
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => "/* unknown op */",
    }
}

#[cfg(test)]
mod literal_tests {
    use super::quoted;

    #[test]
    fn a_quote_and_a_backslash_are_escaped_again() {
        // ankql's SQL renderer writes `buffer.push('\'')` to open a quoted
        // string and `push_str("''")` to escape one inside it.
        assert_eq!(quoted("'"), r"'\''");
        assert_eq!(quoted("''"), r"'\'\''");
        assert_eq!(quoted(r"a\b"), r"'a\\b'");
    }

    /// PREMISE CHANGED: NUL is `\u{0}`, not `\0`.
    ///
    /// `\0` followed by a decimal digit is a legacy octal escape: Rust's
    /// `"nul\01digit"` is NUL then `'1'`, and `'nul\01digit'` is U+0001 then
    /// `digit` — a different string in sloppy mode and a parse error in a
    /// module, which is what the port emits.
    #[test]
    fn control_characters_keep_their_escape() {
        assert_eq!(quoted("\0"), r"'\u{0}'");
        assert_eq!(quoted("\u{0}1"), r"'\u{0}1'");
        assert_eq!(quoted("\n"), r"'\n'");
        assert_eq!(quoted("\t"), r"'\t'");
        assert_eq!(quoted("\u{1}"), r"'\u{1}'");
    }

    /// The template-literal escaper is the same rule. It used to handle neither
    /// NUL nor any other control character, and a raw one in the middle of a
    /// template is a character nothing reading the output can see.
    #[test]
    fn a_template_escapes_every_control_character_too() {
        use crate::macros::format_emit::escape_template;
        assert_eq!(escape_template("a\0b"), r"a\u{0}b");
        assert_eq!(escape_template("a\u{1}b"), r"a\u{1}b");
        assert_eq!(escape_template("a`b"), r"a\`b");
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        assert_eq!(quoted("hello"), "'hello'");
        assert_eq!(quoted("héllo →"), "'héllo →'");
    }
}
