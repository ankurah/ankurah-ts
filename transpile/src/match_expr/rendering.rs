//! How one arm of the runtime's `.match({..})` is WRITTEN, once the decisions
//! about it have been made.
//!
//! For: `enum_match_over` writes an arm the source named and `catch_all` writes
//! one per variant the source left to its `_`. Both need the same decisions
//! made the same way — whether the arm's value needs a `return`, whether its
//! releases need a block, whether it is `async` — so both build an `ArmParts`
//! and hand it here. The catch-all used to format its arms itself and lost the
//! match's value in every position but the enclosing function's return.

use super::arms::{arm_statements, as_arm_value};
use crate::body::{indent, BodyTranslator};

/// One arm of a `.match({..})`, as the pieces the two callers assemble it from.
///
/// `enum_match_over` writes an arm the source named; `catch_all` writes one per
/// variant the source left to its `_`. Both need the same decisions made the
/// same way — whether the arm's value needs a `return`, whether its releases
/// need a block, whether it is `async` — so both build one of these and hand it
/// to `render_arm`. The catch-all used to format its arms itself and lost the
/// match's value in every position but the enclosing function's return.
pub(super) struct ArmParts<'a> {
    /// The key the runtime's match dispatches on.
    pub variant: &'a str,
    /// What the arm declares before its body: the drop flags a hand-away owes,
    /// and the names the arm takes out of the value it was given.
    pub bindings: String,
    /// The payload parameter, where the arm takes one.
    pub param: Option<String>,
    pub body: &'a str,
    pub owned: &'a [crate::ownership::Owned],
    pub lifted: &'a [crate::ownership::Hoist],
    /// Whether the match hands a value back at all.
    pub produces: bool,
    /// Is `body` one EXPRESSION whose value the arm may take, or a run of
    /// statements the lowering has already finished? Carried from the
    /// lowering that wrote it (K2), never read back out of the text.
    pub value: bool,
    pub is_async: bool,
    /// What this arm's outermost `finally` says about the parts of the payload
    /// no name took. A consuming arm owns the whole payload from the moment it
    /// is called — `intoMatch` releases nothing of its own, on any path — so an
    /// arm that binds only some of it releases the rest here.
    pub release_rest: String,
    /// Is the arm's Rust body a TUPLE literal? TypeScript takes a `match`'s
    /// result type from the first arm it reads, and a tuple written in one arm
    /// makes every later arm an error against it — so that arm is cast. Read
    /// from the ARM, never from the emitted text: `[...exprs].every(p)` starts
    /// with a bracket and is a boolean, and `sql_builder.ts`'s
    /// `can_pushdown_expr` was cast for it (E16).
    pub tuple: bool,
}

/// One arm of a `.match({..})`.
///
/// The payload's names are declared inside the arm, from the value the arm is
/// handed. They used to be substituted into the rendered TypeScript by walking
/// its characters, which could not tell a binding from the same word inside a
/// string literal or a comment, and knew nothing of a name shadowed further in.
pub(super) fn render_arm(parts: ArmParts<'_>, t: &BodyTranslator) -> String {
    let ArmParts { variant, bindings, param, body, owned, lifted, produces, value, is_async, release_rest, tuple } =
        parts;
    // An arm is an arrow function, and JavaScript's `await` belongs to the
    // nearest one — so an arm that awaits is `async`, and the whole `.match`
    // is awaited where it stands.
    let keyword = if is_async { "async " } else { "" };
    let head = match &param {
        Some(param) => format!("  {}: {}({}) => ", variant, keyword, param),
        None => format!("  {}: {}() => ", variant, keyword),
    };
    if owned.is_empty() && lifted.is_empty() && release_rest.is_empty() {
        return format!("{}{},\n", head, as_arm_value(body, &bindings, produces, value, tuple));
    }
    // An arm that owns what it was handed, that lifted a declaration out of its
    // own body, or that owes the payload a release, is always a block: the
    // release goes in a `finally`, so the arm cannot be the bare expression
    // form.
    let inner = arm_block(
        ArmParts { variant, bindings, param: None, body, owned, lifted, produces, value, is_async, release_rest, tuple },
        t,
    );
    format!("{}{{\n{}  }},\n", head, indent(&indent(&inner)))
}

/// One arm's body as STATEMENTS: what `render_arm` puts inside the arrow.
///
/// A link of a chain has already been handed the payload by the key around it,
/// so it needs the same statements without an arrow of its own.
pub(super) fn arm_block(parts: ArmParts<'_>, t: &BodyTranslator) -> String {
    let (bindings, inner) = arm_block_parts(parts, t);
    format!("{}{}", bindings, inner)
}

/// The same, with the DECLARATIONS kept apart from the body.
///
/// A guarded link needs the two separately: the names the pattern took have to
/// stand before the guard, because the guard reads them, and the body has to
/// stand inside the `if` the guard opens.
pub(super) fn arm_block_parts(parts: ArmParts<'_>, t: &BodyTranslator) -> (String, String) {
    let ArmParts { bindings, body, owned, lifted, produces, value, release_rest, .. } = parts;
    let mut inner = t.wrap_bindings(
        owned,
        crate::ownership::hoisted(&arm_statements(body, produces, value), lifted),
    );
    if !release_rest.is_empty() {
        inner = format!("try {{\n{}}} finally {{\n{}}}\n", indent(&inner), indent(&release_rest));
    }
    (bindings, inner)
}
