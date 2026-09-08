//! The goldens that are exceptions, and why each one is.
//!
//! For: a list of exceptions is only worth anything if every line says what it
//! is waiting on. Split out of `golden_run.rs`, which was over the 600-line
//! rule, so that the two ledgers stay readable as they grow and the run's own
//! logic stays readable beside them.

/// Which goldens are allowed to have no `run.test.ts`, and why each one has
/// none.
///
/// A driver is what makes a golden's claim about ownership checkable, so a
/// golden losing its driver has to fail rather than quietly stop being
/// executed. Everything under `goldens/` owes one unless it is named here.
///
/// Each of these pins the shape of a declaration and its derived codec. Running
/// one would prove nothing the text does not already say: the encode and decode
/// pair means something only against bytes Rust produced, and those bytes are
/// what the wire-protocol fixtures compare, not this runner.
pub const TEXT_ONLY: [(&str, &str); 4] = [
    (
        "struct_bincode",
        "a named-field struct and a byte newtype, with the encode/decode pair the derive writes",
    ),
    (
        "enum_payload",
        "an enum of one unit, one tuple and one named-field variant, with its variant-tagged codec",
    ),
    (
        "option_result_fields",
        "`Option<T>` fields and a method returning `Result<T, E>`; the README records its emitted \
         error construction as unvetted, so a driver would pin output nobody has read yet",
    ),
    (
        "question_mark",
        "where the emitted `?` puts its early return. Nothing the golden constructs owns anything, \
         so executing it exercises no release",
    ),
];

/// The goldens whose emitted TypeScript does not compile yet, with the error
/// codes each one produces and the defect behind them.
///
/// This is a ledger of debt, matched exactly in both directions: a golden that
/// starts compiling has to come off the list, and a golden that starts
/// producing a new kind of error fails even though it was already failing. Both
/// halves matter — a fix that goes unrecorded here looks the same as no fix, and
/// a new defect hiding behind an old one is how this check would rot.
///
/// Every entry is a defect in the transpiler or in a decision the goldens' own
/// README already doubts. None of them is a reason to relax the check.
/// What each golden still fails to compile with, as one entry per error:
/// `<file>:<code>`, sorted. Every entry is a decision somebody read.
pub const TYPECHECK_DEBT: [(&str, &[&str], &str); 2] = [
    (
        "blanket_free_fn",
        &["blanket_free_fn/run.test.ts:TS2345"],
        "the driver hands `fromAny` a closure, and `fromAny` is emitted with the bound Rust \
         wrote — `L extends IntoListener` — which a closure does not implement structurally \
         in TypeScript, though the blanket impl makes it an `IntoListener` in Rust. The call \
         inside the function now goes through the run-time dispatcher and reaches every impl; \
         what is left is the signature, and what a bound with a blanket impl behind it should \
         emit as is open",
    ),
    (
        "a_constructor_typed_by_its_closure",
        &["a_constructor_typed_by_its_closure/input.ts:TS2322"],
        "the engine types `counted` as `Calculated<usize>` from what the closure answers, and \
         says so at every site it writes. TypeScript cannot follow it: a parameter written \
         `F: Fn() -> T` is emitted as `Invocable<[], T>`, and a plain arrow does not settle \
         that wrapper's `T`, so the class's own `T` reads as `unknown` at the call. What a \
         callable bound should emit as is the open question, and it is the same one the \
         `blanket_free_fn` line above records",
    ),
];
