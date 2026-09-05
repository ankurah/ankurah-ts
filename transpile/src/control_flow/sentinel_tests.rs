//! Where the sentinel's pieces stand in the emitted text.
//!
//! A test that asks whether a string appears ANYWHERE in the output cannot tell
//! a reader written beside the lifted value from one written inside the arrow
//! that produced it, and the difference is the whole defect: a `return` inside
//! the arrow returns from the arrow. So these tests ask where each piece sits.

use crate::testing::Fixture;

/// Is this line inside the body of an arrow function?
///
/// The scan keeps a stack of the blocks open at each point and marks the ones a
/// `=>` introduced. A line reached with an arrow on the stack is inside a
/// function the emitter wrote, where `return`, `break` and `continue` mean
/// something other than what the Rust said.
pub(crate) fn inside_an_arrow(ts: &str, line_holds: &str) -> bool {
    let mut arrows: Vec<bool> = Vec::new();
    for line in ts.lines() {
        if line.contains(line_holds) {
            return arrows.iter().any(|is_arrow| *is_arrow);
        }
        // `=> {` on this line opens an arrow body; every other `{` opens an
        // ordinary block. Reading the line's own text before its braces are
        // counted is enough here: the emitter writes one opener per line.
        let opens_arrow = line.contains("=> {") || line.contains("=> ({");
        for ch in line.chars() {
            match ch {
                '{' => arrows.push(opens_arrow),
                '}' => {
                    arrows.pop();
                }
                _ => {}
            }
        }
    }
    panic!("no line of the output holds {line_holds:?}:\n{ts}");
}

fn built(src: &str) -> Fixture {
    Fixture::build(&[("lib.rs", src)])
}

/// D1: the `continue` names the `for` written inside the lifted arm, and that
/// loop is in the same arrow — so it is an ordinary `continue`. Handed back as
/// a sentinel it left the arm on the first NUL byte, which is how ankql's
/// `generate_expr_sql` wrote an unterminated SQL literal.
#[test]
fn a_jump_to_a_loop_inside_the_lift_stays_a_jump() {
    let mut f = built(
        "pub enum E { Refused }\n\
         pub enum Lit { S(String), N(u32) }\n\
         pub fn render(lit: &Lit, out: &mut String) -> Result<(), E> {\n\
           match lit {\n\
             Lit::S(s) => {\n\
               for c in s.chars() {\n\
                 if c == '\\0' { continue; }\n\
                 out.push(c);\n\
               }\n\
             }\n\
             Lit::N(n) => { if *n == 0 { return Err(E::Refused); } }\n\
           }\n\
           Ok(())\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "render");
    assert!(
        ts.contains("continue;"),
        "the `continue` belongs to the `for` written in the arm:\n{ts}"
    );
    assert!(
        !ts.contains("$jump: 'continue'"),
        "and nothing hands it back to the caller:\n{ts}"
    );
    // The arm's `return` is still an exit that travels out.
    assert!(
        inside_an_arrow(&ts, "$jump: 'return'"),
        "the `return` is written inside the arm's arrow:\n{ts}"
    );
    assert!(
        !inside_an_arrow(&ts, "$jump === 'return'"),
        "and the test that performs it stands outside every arrow:\n{ts}"
    );
}

/// A `break` naming a loop OUTSIDE the lifted body still travels out, and the
/// analysis has to see it through a loop of the arm's own — spelling it as a
/// plain `break outer` inside an arrow is a SyntaxError.
#[test]
fn a_labelled_jump_past_an_inner_loop_still_travels_out() {
    let expr: syn::Expr = syn::parse_str(
        "{ for c in s.chars() { if c == '!' { break 'rows; } } }",
    )
    .expect("parses");
    assert_eq!(
        crate::control_flow::sentinel::jumps_out_of(&expr),
        vec!["break#rows".to_string()],
        "a labelled break reaches past the arm's own loop"
    );
    let caught: syn::Expr =
        syn::parse_str("{ for c in s.chars() { if c == '!' { break; } } }").expect("parses");
    assert!(
        crate::control_flow::sentinel::jumps_out_of(&caught).is_empty(),
        "a bare break is caught by that loop"
    );
    let named: syn::Expr =
        syn::parse_str("{ 'inner: for c in s.chars() { break 'inner; } }").expect("parses");
    assert!(
        crate::control_flow::sentinel::jumps_out_of(&named).is_empty(),
        "and so is one naming that loop"
    );
}

/// D2: a lift inside a lift. The inner reader cannot perform the `return`
/// either — it is written inside the outer arrow — so it hands the whole
/// sentinel on, and only the outermost reader unwraps `$value`. Unwrapped at
/// the inner one, the arm's value became a bare `Result.Err`, which the test
/// above it does not recognise as an exit: core's `fetch_gap` handed a failed
/// `build_continuation_predicate` on as the gap selection.
#[test]
fn a_reader_inside_a_lift_re_raises_the_whole_sentinel() {
    let mut f = built(
        "pub struct Oops { pub why: String }\n\
         pub fn fallible(ok: bool) -> Result<u32, Oops> { Ok(7) }\n\
         pub fn run(first: Option<bool>, ok: bool) -> Result<u32, Oops> {\n\
           let n = if let Some(f) = first {\n\
             let inner = { let v = fallible(ok)?; v + 1 };\n\
             inner\n\
           } else { 0 };\n\
           Ok(n)\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "run");
    let readers: Vec<&str> = ts
        .lines()
        .filter(|line| line.contains("$jump === 'return'"))
        .collect();
    assert_eq!(readers.len(), 2, "one reader per lift:\n{ts}");
    // The inner lift is written first, inside the outer arrow.
    assert!(
        inside_an_arrow(&ts, readers[0]),
        "the first reader stands inside the outer arrow:\n{ts}"
    );
    assert!(
        !readers[0].contains(".$value"),
        "so it cannot perform the return, and hands the whole sentinel on:\n{ts}"
    );
    assert!(
        !inside_an_arrow(&ts, readers[1]),
        "the second reader stands in the function itself:\n{ts}"
    );
    assert!(
        readers[1].contains(".$value"),
        "so it performs the return with the value the exit carried:\n{ts}"
    );
}

/// D7: a consuming match written as a STATEMENT inside a lift keeps its `?`
/// exit. In RETURN position inside a lift the arm's sentinel travels through
/// the enclosing arrow to the reader already standing there, and a second test
/// would read a value nobody put there.
#[test]
fn a_statement_match_inside_a_lift_keeps_its_exit() {
    let mut f = built(
        "pub enum E { Refused }\n\
         pub enum Step { Skip, Apply(bool) }\n\
         pub fn apply(ok: bool) -> Result<bool, E> { Ok(ok) }\n\
         pub fn run(first: Option<Step>) -> Result<u32, E> {\n\
           let n = if let Some(step) = first {\n\
             match step {\n\
               Step::Skip => {}\n\
               Step::Apply(ok) => { if apply(ok)? { } }\n\
             }\n\
             1u32\n\
           } else { 0u32 };\n\
           Ok(n)\n\
         }",
    );
    let ts = f.translated_method("lib.rs", "run");
    let readers = ts.matches("$jump === 'return'").count();
    assert_eq!(
        readers, 2,
        "the statement match reads its own arms' exit, and the lift reads that:\n{ts}"
    );
}
