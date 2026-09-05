//! A path in expression position, and the values a path names.
//!
//! For: Rust writes a name with as much of its module tree in front of it as
//! the reader needs, and the port has no module tree — a crate is a package and
//! its names are imported by their leaves. So a path is not written out; it is
//! resolved, and what stands is whatever the emitted module has for the thing
//! it named: a class, a variant built with its constructor, a local under the
//! identifier a shadow was freshened to, a number for an ordering.

use crate::name_map;

use super::{BodyTranslator, STD_QUALIFIERS};

impl BodyTranslator<'_> {
    // ── Path translation ────────────────────────────────────────────

    /// A path in expression position. The standard-library qualifiers are
    /// dropped so that `std::sync::Arc::new` becomes `Arc.new`, which is a
    /// guess about what the remaining segments mean; it is recorded as one.
    /// `std::cmp::Ordering::Greater`, as the number the port writes an ordering
    /// as.
    ///
    /// Three different types are called `Ordering` in reach of the corpus:
    /// `std::cmp`'s, `std::sync::atomic`'s, and core's own in `lineage.rs`. The
    /// registry says which one this path names, so only the first takes a
    /// number and the other two are left to be written as themselves.
    pub(crate) fn ordering_variant(&self, path: &syn::Path) -> Option<&'static str> {
        let mut segments = path.segments.iter().rev();
        let variant = segments.next()?.ident.to_string();
        if segments.next()?.ident != "Ordering" {
            return None;
        }
        let number = crate::native_types::ordering::variant(&variant)?;
        let tc = self.types.as_ref()?;
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let resolved = tc.resolve_expr(&syn::Expr::Path(syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: path.clone(),
        }));
        tc.sink.rewind(mark);
        crate::native_types::ordering::is_ordering(tc.registry, &resolved.ok()?).then_some(number)
    }

    /// `std::sync::atomic::Ordering::SeqCst` and its four siblings. A JavaScript
    /// program is single-threaded and every atomic is a plain value, so the
    /// ordering says nothing — the method translations drop the argument, and
    /// this is what stands where one is written anywhere else.
    fn atomic_ordering(&self, path: &syn::Path) -> Option<String> {
        let mut segments = path.segments.iter().rev();
        let variant = segments.next()?.ident.to_string();
        if segments.next()?.ident != "Ordering" {
            return None;
        }
        if !matches!(variant.as_str(), "Relaxed" | "Acquire" | "Release" | "AcqRel" | "SeqCst") {
            return None;
        }
        Some(format!("undefined /* atomic Ordering::{} */", variant))
    }

    /// Is this expression one of the three numbers the port writes an ordering
    /// as? A `match` on one is a comparison, not a dispatch on a variant name.
    pub(crate) fn is_ordering_value(&self, expr: &syn::Expr) -> bool {
        let Some(tc) = self.types.as_ref() else { return false };
        let tc = tc.borrow();
        let mark = tc.sink.mark();
        let resolved = tc.resolve_expr(expr);
        tc.sink.rewind(mark);
        match resolved {
            Ok(ty) => crate::native_types::ordering::is_ordering(tc.registry, &ty),
            Err(_) => false,
        }
    }

    /// Is this the `Ok(())` a formatter's `fmt` ends with?
    ///
    /// Rust's `fmt` answers `fmt::Result`, and every path out of it that did
    /// not fail answers `Ok(())`. The port's `toString()` answers the string,
    /// so that value is the accumulator.
    pub(crate) fn is_formatter_done(&self, expr: &syn::Expr) -> bool {
        if !self.formatter {
            return false;
        }
        let syn::Expr::Call(call) = expr else { return false };
        let syn::Expr::Path(path) = call.func.as_ref() else { return false };
        if path.path.segments.last().map(|s| s.ident.to_string()).as_deref() != Some("Ok") {
            return false;
        }
        matches!(call.args.first(), Some(syn::Expr::Tuple(t)) if t.elems.is_empty())
    }

    /// The item a two-segment path on a primitive names, where the port has a
    /// spelling, and an R12 hole where it has none — the alternative is the
    /// path written out, `f64.MIN_POSITIVE`, a name the file never declares.
    fn primitive_item(&self, path: &syn::Path) -> Option<String> {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        match crate::ty::prim_consts::written_or_reason(&segments)? {
            Ok(written) => Some(written),
            Err(why) => Some(self.hole(syn::spanned::Spanned::span(path), why)),
        }
    }

    pub(crate) fn path_expr(&self, path: &syn::Path) -> String {
        // A path of one segment is a name — a local, a parameter, a free
        // function — and never a module qualifier. Filtering it as one deleted
        // every local called `ops`, `iter` or `fmt`: `ops.iter()` came out as
        // `[...]`, a spread of nothing.
        let dropped: Vec<String> = if path.segments.len() == 1 {
            Vec::new()
        } else {
            path.segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .filter(|name| STD_QUALIFIERS.contains(&name.as_str()))
                .collect()
        };
        // A `tokio` path keeps its segments, so nothing was given up.
        let dropped = if path.segments.first().is_some_and(|s| s.ident == "tokio") {
            Vec::new()
        } else {
            dropped
        };
        if !dropped.is_empty() {
            self.fallback(
                syn::spanned::Spanned::span(path),
                format!("path qualifiers {} are dropped by name", dropped.join(", ")),
            );
        }
        // A path through another in-family crate — `ankql::ast::Expr::Literal`
        // — names a type this file imports by its leaf, because the port
        // flattens a crate's module tree into a package's exports. Keeping the
        // qualifiers wrote `ankql.ast.Expr`, and nothing called `ankql` exists
        // in the emitted module.
        if let Some(trimmed) = self.through_sibling_crate(path) {
            return trimmed;
        }
        // The same for one of THIS crate's own modules: the port flattens the
        // module tree into a package's exports, so `ast::Expr` is the `Expr`
        // this file imports and `parser::parse_selection` the `parseSelection`
        // beside it.
        if let Some(trimmed) = self.through_local_module(path) {
            return trimmed;
        }
        // `f64::EPSILON`, `u32::MAX` — see `ty::prim_consts`.
        if let Some(written) = self.primitive_item(path) {
            return written;
        }
        // `Ordering::Greater` is the number `1`: the port writes an ordering as
        // the number a comparison answers, which is what `compareTo` returns.
        // Written as a member of a class, it named `undefined /* Ordering */`.
        if let Some(number) = self.ordering_variant(path) {
            return number.to_string();
        }
        if let Some(written) = self.atomic_ordering(path) {
            return written;
        }
        // `ParseError::Empty` is a value, and building it is what every other
        // construction of that enum does. Written as a member of the class it
        // named a static nothing declares.
        if let Some(built) = self.unit_variant(path) {
            return built;
        }
        // A single name may be a local the translator had to emit under a
        // different identifier, because a Rust shadow cannot be declared twice
        // in one JavaScript scope.
        if path.segments.len() == 1 {
            // A body emitted as a module-level function has no `this`: its
            // receiver arrived as an ordinary first parameter, under the name
            // `self_name`.
            if path.segments[0].ident == "self" {
                return self.self_name.to_string();
            }
            let written = Self::path_static(path);
            // A path of one lowercase segment names a local, a parameter or a
            // free function — a binding, and JavaScript will not accept every
            // Rust name in that position. `Type::new()` is not one: `new` there
            // is a property, which may be a keyword, so the escape is confined
            // to the single-segment case and to the names this function merely
            // camel-cased. `self` becomes `this` and `None` becomes `null`,
            // which are the keywords themselves and not names.
            let ident = path.segments[0].ident.to_string();
            let written = if written == name_map::to_camel_case(&ident) {
                name_map::escape_reserved(&written)
            } else {
                written
            };
            let written = self.emitted_name(&written).unwrap_or(written);
            // C1: a name the body holds in a runtime cell is read through it.
            if self.boxed.borrow().iter().any(|name| *name == written) {
                return format!("{}.value", written);
            }
            // A non-`Copy` `const` is a fresh value at each use, so the port
            // emitted it as a function and this use calls it.
            if self.names_a_fresh_const(std::slice::from_ref(&ident)) {
                return format!("{}()", written);
            }
            return written;
        }
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        if self.names_a_fresh_const(&segments) {
            return format!("{}()", Self::path_static(path));
        }
        Self::path_static(path)
    }

    /// `Enum::Variant { field: .. }` built the way the port builds a variant.
    pub(crate) fn struct_variant_literal(&self, s: &syn::ExprStruct) -> Option<String> {
        let segments: Vec<String> =
            s.path.segments.iter().map(|seg| seg.ident.to_string()).collect();
        if segments.len() < 2 {
            return None;
        }
        let tc = self.types.as_ref()?;
        let (owner, variant) = tc.borrow().variant_of_emitted_enum(&segments)?;
        let want = self.struct_field_types(s);
        let fields: Vec<String> = s
            .fields
            .iter()
            .map(|f| {
                let member = crate::infer::member_name(&f.member);
                let ty = want.iter().find(|(name, _)| *name == member).map(|(_, t)| t);
                let value = self.expecting(&f.expr, ty, || self.moved_value(&f.expr));
                format!("{}: {}", name_map::to_camel_case(&member), value)
            })
            .collect();
        Some(format!("new {}('{}', {{ {} }})", owner, variant, fields.join(", ")))
    }

    /// `Rec { third: c, first: a }` as the call to `Rec`'s constructor.
    ///
    /// Rust checks each value against the FIELD it stands beside; a ported
    /// struct is built through a constructor whose parameters are its fields in
    /// DECLARATION order. Writing the values in the order the literal happened
    /// to name them handed `c` to `first` — silently, wherever the two fields
    /// have the same TypeScript type, which is exactly the case tsc cannot
    /// catch. `connectors/local-process/src/lib.rs:70` writes six fields in an
    /// order the constructor does not share, two of them `EntityId`.
    pub(crate) fn struct_literal(&self, s: &syn::ExprStruct, name: &str) -> String {
        // The declared fields, in the order the constructor takes them, and
        // the type of each where the engine has one. The two come from
        // different places on purpose: the ORDER covers every field, including
        // one whose type did not resolve, and a field left out of the call
        // would shift every argument after it.
        let typed = self.struct_field_types(s);
        let declared: Vec<(String, Option<crate::ty::Ty>)> = self
            .struct_field_order(s)
            .into_iter()
            .map(|name| {
                let ty = typed.iter().find(|(n, _)| *n == name).map(|(_, t)| t.clone());
                (name, ty)
            })
            .collect();
        let written = |f: &syn::FieldValue, ty: Option<&crate::ty::Ty>| {
            self.expecting(&f.expr, ty, || self.moved_value(&f.expr))
        };
        if declared.is_empty() {
            // The engine could not name the struct, so it cannot say what order
            // the constructor takes. Source order is what it has; the site says
            // the order is a guess.
            let values: Vec<String> = s
                .fields
                .iter()
                .map(|f| written(f, None))
                .collect();
            if s.fields.len() > 1 {
                self.fallback(
                    syn::spanned::Spanned::span(s),
                    format!(
                        "`{}` is built here and the engine could not name its declaration, \
                         so the values are handed to the constructor in the order the literal \
                         writes them rather than in the order the fields are declared",
                        name
                    ),
                );
            }
            return format!("new {}({})", name, values.join(", "));
        }
        // `..rest` fills every field the literal does not name from another
        // value of the same type. Nothing here reads it, so the fields it would
        // have filled would be `undefined`.
        if let Some(rest) = &s.rest {
            self.fallback(
                syn::spanned::Spanned::span(rest),
                format!(
                    "`..` fills the fields `{}` does not name from another value, and the port \
                     has no writing for it, so those fields are left undefined",
                    name
                ),
            );
        }
        let mut values: Vec<String> = Vec::new();
        for (field, ty) in &declared {
            match s
                .fields
                .iter()
                .find(|f| crate::infer::member_name(&f.member) == *field)
            {
                Some(f) => values.push(written(f, ty.as_ref())),
                // Named by neither the literal nor anything else: only `..rest`
                // can produce this, and the line above says so.
                None => values.push("undefined".to_string()),
            }
        }
        // A field the literal names and the declaration does not: the engine
        // resolved the literal to the wrong type, or the source does not
        // compile. Either way it must not vanish.
        for f in &s.fields {
            let member = crate::infer::member_name(&f.member);
            if !declared.iter().any(|(name, _)| *name == member) {
                self.fallback(
                    syn::spanned::Spanned::span(f),
                    format!(
                        "`{}` is not a field of `{}` as the engine read it, so the value \
                         written for it reaches no constructor parameter",
                        member, name
                    ),
                );
            }
        }
        format!("new {}({})", name, values.join(", "))
    }

    /// A path whose first segment names another in-family crate, written the
    /// way this file reaches it: from the type onwards, since that is what the
    /// import brings in.
    pub(crate) fn through_sibling_crate(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let head = path.segments.first()?.ident.to_string();
        let tc = self.types.as_ref()?;
        // `use ankurah_proto as proto;` gives the crate a LOCAL name, and the
        // code below writes `proto::Presence`. Asked by the written head alone
        // the registry says no such sibling, so the path kept its qualifier and
        // the emitted file said `new proto.Presence(..)` while importing
        // `Presence` — a `proto` that exists nowhere in the module.
        let head = {
            let ctx = tc.borrow();
            if ctx.registry.sibling_crate(&head).is_some() {
                head
            } else {
                let aliased = ctx
                    .registry
                    .modules()
                    .get(ctx.module)
                    .uses
                    .iter()
                    .find(|u| u.local.as_deref() == Some(head.as_str()) && u.path.len() == 1)
                    .map(|u| u.path[0].clone())
                    .filter(|target| ctx.registry.sibling_crate(target).is_some());
                aliased?
            }
        };
        let _ = &head;
        Some(self.without_module_qualifiers(path)).filter(|w| !w.is_empty())
    }

    /// A path through one of THIS crate's own modules — `ast::Expr`,
    /// `parser::parse_selection` — where the port flattens the module tree into
    /// a package's exports, so the type is imported by its leaf and the
    /// qualifier names nothing in the emitted file.
    ///
    /// `new ast.Expr(..)` and `parser.parseSelection(..)` stood in ankql's
    /// emitted `conversion.ts` beside `import { Expr } from './ast'` and
    /// `import { parseSelection } from './parser'`: an `ast` and a `parser` that
    /// exist nowhere in the module, so every one of them raised at run time.
    pub(crate) fn through_local_module(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() < 2 || path.leading_colon.is_some() {
            return None;
        }
        let head = path.segments.first()?.ident.to_string();
        // A module name is lowercase; a type is not, and `Enum::Variant` must
        // keep its enum.
        if head.chars().next().is_some_and(|c| c.is_uppercase()) {
            return None;
        }
        let tc = self.types.as_ref()?;
        let ctx = tc.borrow();
        let modules = ctx.registry.modules();
        // `use self::ast;` or `mod ast;` — reachable from where the code is
        // written, or from the crate root, which is how `crate::ast::Expr` and
        // a `use crate::ast;` above it both spell it.
        let root = ctx.registry.crate_root_of(ctx.module);
        let names_a_module = modules.get(ctx.module).children.contains_key(&head)
            || modules.get(root).children.contains_key(&head);
        if !names_a_module {
            return None;
        }
        drop(ctx);
        Some(self.without_module_qualifiers(path)).filter(|written| !written.is_empty())
    }

    /// The path with its leading module segments taken off: everything from the
    /// first capitalised segment, or the last segment alone where the path
    /// names a free function.
    fn without_module_qualifiers(&self, path: &syn::Path) -> String {
        let rest: Vec<&syn::PathSegment> = path
            .segments
            .iter()
            .skip_while(|seg| !seg.ident.to_string().chars().next().is_some_and(|c| c.is_uppercase()))
            .collect();
        if rest.is_empty() {
            // Every segment is a module name: the path names a free function
            // of that crate, which the import map brings in by its own name.
            return path
                .segments
                .last()
                .map(|s| crate::name_map::to_camel_case(&s.ident.to_string()))
                .unwrap_or_default();
        }
        let mut trimmed = syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in rest {
            trimmed.segments.push(seg.clone());
        }
        self.unit_variant(&trimmed)
            .unwrap_or_else(|| Self::path_static(&trimmed))
    }

    /// A unit enum variant written as a path, built the way one is built.
    fn unit_variant(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let segments: Vec<String> =
            path.segments.iter().map(|s| s.ident.to_string()).collect();
        let tc = self.types.as_ref()?;
        let (owner, variant) = tc.borrow().unit_variant_of_emitted_enum(&segments)?;
        Some(format!("new {}('{}', {{}})", owner, variant))
    }

    pub(crate) fn path_static(path: &syn::Path) -> String {
        let single = path.segments.len() == 1;
        // A path through `tokio` keeps every segment and every name as written:
        // `@ankurah/base` mirrors the crate's module tree and spells the
        // functions the way tokio spells them, so `tokio::sync::mpsc::
        // unbounded_channel` is `tokio.sync.mpsc.unbounded_channel`.
        let through_tokio = path.segments.first().is_some_and(|s| s.ident == "tokio");
        if through_tokio {
            let names: Vec<String> =
                path.segments.iter().map(|seg| seg.ident.to_string()).collect();
            return names.join(".");
        }
        let segments: Vec<String> = path.segments.iter().map(|seg| {
            let name = seg.ident.to_string();
            match name.as_str() {
                "self" => "this".to_string(),
                "Self" => "Self".to_string(),
                "None" => "null".to_string(),
                "true" | "false" => name,
                "Ok" | "Some" | "Err" => name,
                "std" | "core" | "alloc" | "crate" | "super" | "marker" => name,
                "PhantomData" => return "undefined /* PhantomData */".to_string(),
                _ => {
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        name
                    } else {
                        name_map::to_camel_case(&name)
                    }
                }
            }
        }).collect();

        // Strip std/core/alloc module prefixes, keep type+method. A lone
        // segment is a name, not a qualifier: a local called `ops` is `ops`.
        let segments: Vec<String> = segments.into_iter()
            .filter(|s| single || !STD_QUALIFIERS.contains(&s.as_str()))
            .collect();
        let joined = segments.join(".");
        match joined.as_str() {
            // `crate::` names this crate's own module tree, which the port
            // flattens: a module is a file and its names are imported. What
            // survives is the *type* and whatever is written after it —
            // `crate::TypeResolver::new` is `TypeResolver.new` — while a path
            // that names no type is a free function and keeps its own name.
            // Taking the last segment alone took the type away with the
            // modules and left a bare `new()`.
            s if s.starts_with("crate.") => {
                let at = segments
                    .iter()
                    .position(|seg| seg.chars().next().is_some_and(|c| c.is_uppercase()));
                match at {
                    Some(at) => segments[at..].join("."),
                    None => segments.last().cloned().unwrap_or(joined),
                }
            }
            _ => joined,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Fixture;

    /// `use ankurah_proto as proto;` gives another crate a LOCAL name, and the
    /// code below writes `proto::Presence`. The port flattens a crate into a
    /// package, so the type is imported by its leaf; keeping the qualifier
    /// emitted `new proto.Presence(..)` against a `proto` that exists nowhere
    /// in the module. Live at `connectors/local-process/src/lib.rs:59`.
    #[test]
    fn a_crate_under_a_local_name_is_still_a_package() {
        let mut f = Fixture::build_with_siblings(
            "connector-local",
            &[(
                "lib.rs",
                "use ankurah_proto as proto;\n\
                 pub struct Sender { pub id: usize }\n\
                 impl Sender {\n\
                   pub fn announce(&self) -> proto::Presence { proto::Presence { node_id: 1 } }\n\
                 }",
            )],
            &[("ankurah_proto", &[("lib.rs", "pub struct Presence { pub node_id: usize }")])],
        );
        let ts = f.translated_method("lib.rs", "announce");
        assert!(!ts.contains("proto."), "the qualifier names nothing here:\n{}", ts);
        assert!(ts.contains("new Presence("), "{}", ts);
    }

    /// A temporary the translator needs must not take the name of a binding
    /// that is live: `const _v2 = ..` written into a body that declared its own
    /// `let _v2` shadows it for the rest of the block, and every later read of
    /// the Rust name reads the temporary instead. The names are the shape the
    /// translator hands out, so a body that happens to use them is exactly the
    /// collision.
    #[test]
    fn a_temporary_does_not_shadow_a_binding_in_scope() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub fn find(n: usize) -> Option<usize> { Some(n) }\n\
             pub fn read(n: usize) -> usize {\n\
               let _v = 1;\n\
               let _v1 = 2;\n\
               let _v2 = 3;\n\
               let _v3 = 4;\n\
               let m = match find(n) { Some(x) => x, None => 0 };\n\
               _v + _v1 + _v2 + _v3 + m\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "read");
        for name in ["_v", "_v1", "_v2", "_v3"] {
            assert_eq!(
                ts.matches(&format!("const {} = ", name)).count(),
                1,
                "`{}` is declared twice — the match's temporary took a live binding's name:\n{}",
                name,
                ts
            );
        }
    }

    /// A path through one of this crate's OWN modules keeps no qualifier: the
    /// port flattens the module tree into a package's exports, so `ast::Expr`
    /// is the `Expr` the emitted file imports from `./ast` and
    /// `parser::parse_selection` the `parseSelection` beside it. ankql's
    /// `conversion.ts` wrote `new ast.Expr(..)` and `parser.parseSelection(..)`
    /// against an `ast` and a `parser` that exist nowhere in the module.
    #[test]
    fn a_module_of_this_crate_is_not_a_name_in_the_emitted_file() {
        let mut f = Fixture::build_named(
            "testcrate",
            &[
                ("lib.rs", "pub mod ast;\npub mod parser;\npub mod use_it;"),
                ("ast.rs", "pub enum Literal { I64(i64) }\npub struct Path { pub n: usize }"),
                ("parser.rs", "pub fn parse_one(n: usize) -> usize { n }"),
                (
                    "use_it.rs",
                    "use crate::{ast, parser};\n\
                     pub fn make(v: i64) -> ast::Literal { ast::Literal::I64(v) }\n\
                     pub fn build(n: usize) -> ast::Path { ast::Path { n } }\n\
                     pub fn call(n: usize) -> usize { parser::parse_one(n) }",
                ),
            ],
        );
        let make = f.translated_method("use_it.rs", "make");
        assert!(make.contains("new Literal('I64', { _0: v })"), "{}", make);
        assert!(!make.contains("ast."), "{}", make);
        let build = f.translated_method("use_it.rs", "build");
        assert!(build.contains("new Path("), "{}", build);
        assert!(!build.contains("ast."), "{}", build);
        let call = f.translated_method("use_it.rs", "call");
        assert!(call.contains("parseOne(n)"), "{}", call);
        assert!(!call.contains("parser."), "{}", call);
    }

    /// `Option::unwrap` and `Option::expect` PANIC when there is nothing
    /// there. Written as the identity — which is right for a guard, whose
    /// `unwrap` the port's `lock()` has already performed — they handed the
    /// `null` on to be read further down, and `expect`'s message was thrown
    /// away with it: ankql's `PathExpr::property` answered `undefined` for an
    /// empty path where Rust stops the program.
    #[test]
    fn unwrap_on_an_option_panics_rather_than_handing_the_nothing_on() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Path { pub steps: Vec<String> }\n\
             impl Path {\n\
               pub fn property(&self) -> &str { self.steps.last().expect(\"needs a step\") }\n\
               pub fn first(&self) -> &str { self.steps.first().unwrap() }\n\
             }",
        )]);
        let expect = f.translated_method("lib.rs", "property");
        assert!(expect.contains("?? (() => { throw new Error('needs a step'); })()"), "{}", expect);
        let unwrap = f.translated_method("lib.rs", "first");
        assert!(
            unwrap.contains("throw new Error('called `Option::unwrap()` on a `None` value')"),
            "{}",
            unwrap
        );
    }

    /// `Self::setup_receiver(..)` inside an impl is a static of that class. The
    /// path had already been written in TypeScript by the time the call was
    /// built — `Self.setupReceiver` — and splitting it on `::` alone left the
    /// whole of it, so connector-local's emitted file called
    /// `LocalProcessConnection.Self.setupReceiver`.
    #[test]
    fn a_self_qualified_static_is_the_class_and_the_method() {
        let mut f = Fixture::build(&[(
            "lib.rs",
            "pub struct Conn { pub n: usize }\n\
             impl Conn {\n\
               pub fn setup_receiver(n: usize) -> usize { n }\n\
               pub fn start(&self) -> usize { Self::setup_receiver(1) }\n\
             }",
        )]);
        let ts = f.translated_method("lib.rs", "start");
        assert!(ts.contains("Conn.setupReceiver(1)"), "{}", ts);
        assert!(!ts.contains("Self"), "{}", ts);
    }
}
