//! The names `@ankurah/base` exports that an emitted file may have to import.
//!
//! The import list is decided by reading the EMITTED TEXT: a name in this table
//! that appears in the file as a whole word, and that the file does not declare
//! itself, is imported. Keeping the table here rather than beside the emitter
//! means adding a runtime helper is a one-line edit to a list, not a line added
//! to a file that is already long.

pub(crate) const BASE_RUNTIME_SYMBOLS: [&str; 117] = [
    // Rust's two byte-to-text answers: the fatal decode every reader goes
    // through, and the lossy one `String::from_utf8_lossy` asks for.
    "decodeUtf8Lossy",
    "Result", "Arc", "Weak", "Mutex", "MutexGuard",
    "RwLock", "RwLockReadGuard", "RwLockWriteGuard",
    "RefCell", "Ref", "RefMut", "ThreadLocal",
    // The closure that owns its captures, and the error `?` converts into.
    // R10: `invoke` is the one place a bound closure parameter is called, so a
    // callee cannot be handed a shape it does not know how to invoke.
    "OwnedClosure", "invoke", "invokeRef", "Invocable", "AnyhowError", "anyhow",
    // What an emitted `fromJson` answers with: serde_json::Error's stand-in,
    // the lossless reader and writer, and the two combinators a list or a map
    // reads through. `dropOwned` releases what a failed decode had already
    // built, and `OwnershipFatal` and `UnsupportedShape` are the two its
    // `catch` has to rethrow — one is the ownership runtime saying the program
    // is broken, the other is an R12 hole saying the ENGINE is.
    "JsonError", "serde_json", "jsonAll", "jsonMap", "dropOwned", "OwnershipFatal",
    "UnsupportedShape",
    // What a derived `equals` and a derived `clone` ask of a field written as
    // the type's own PARAMETER: `T` is a number in one instantiation and a class
    // in another, so the decision is the value's own surface at run time.
    "derivedEquals", "derivedClone", "derivedHash",
    // I8: `==` between two values the operator table could not route to an
    // impl. `===` compares identity where Rust compares contents.
    "valueEquals", "valueNotEquals",
    // The four float methods whose JavaScript spelling answers something else:
    // half away from zero rather than half up, a signum with no zero, and a
    // `NaN` operand ignored rather than spreading.
    "floatRound", "floatSignum", "floatMin", "floatMax",
    // The logger every `tracing::` macro writes a call on.
    "tracing",
    // What a consuming match arm releases the payload it took no name for
    // with, and Rust's two eager boolean operators.
    "dropUnbound", "boolAnd", "boolOr",
    // R12: the hole an emitted file carries where the port has no lowering.
    "unsupported",
    // C1: the cell a `&mut` to a JavaScript VALUE is passed in.
    "BorrowMut",
    // R7: arithmetic on a fixed-width integer PANICS on overflow, as the
    // `debug_assertions = true` build this port mirrors does, and the four
    // families Rust offers for saying what should happen instead.
    "checkedAdd", "checkedSub", "checkedNeg", "checkedMul", "checkedDiv", "checkedRem",
    "wrappingAdd", "wrappingSub", "wrappingMul",
    "checkedAddOption", "checkedSubOption", "checkedMulOption", "checkedDivOption", "checkedRemOption",
    "saturatingAdd", "saturatingSub", "saturatingMul",
    "overflowingAdd", "overflowingSub", "overflowingMul",
    // J1: the iterator and slice readers Rust answers an `Option` with. Their
    // JavaScript spellings answer `-1` and `undefined`, and `-1 != null` reads
    // as PRESENT.
    "iterPosition", "iterRposition", "iterFind", "iterFindMap", "iterLast", "iterFirst",
    "iterGet", "iterMaxBy", "iterMinBy", "iterMaxByKey", "iterMinByKey", "iterReduce",
    "iterFilterMap",
    // F1: the same terminals over a sequence the expression OWNS, which release
    // every element they do not hand back.
    "iterPositionOwned", "iterRpositionOwned", "iterFindOwned", "iterFindMapOwned",
    "iterLastOwned", "iterMaxByOwned", "iterMinByOwned", "iterMaxByKeyOwned",
    "iterMinByKeyOwned", "iterReduceOwned",
    // A bounded range, as the sequence of its values: the port has no `Range`,
    // and `step_by` over one is every nth element of the sequence it built.
    "range", "rangeIncl", "stepBy",
    // F7: `{:?}` for a value whose type the emitter could not see, and the
    // `char` escaping Rust writes.
    "debugValue", "debugChar", "debugString",
    // The keyed containers a `HashMap`/`HashSet` becomes, and the hash a
    // derived key writes itself with.
    "HashMap", "HashSet", "keyHash",
    "AsyncMutex", "AsyncMutexGuard",
    "AsyncRwLock", "AsyncRwLockReadGuard", "AsyncRwLockWriteGuard",
    "Notify", "Notified", "TryLockError",
    "JoinHandle", "JoinError", "Elapsed",
    "tokio", "oneshot", "mpsc", "select", "spawn", "spawn_local", "yield_now",
    "sleep", "timeout",
    // The channel ends, which `mpsc::channel` hands back and a dispatcher names.
    "Sender", "UnboundedSender", "Receiver", "UnboundedReceiver",
];
