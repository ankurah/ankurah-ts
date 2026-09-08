pub fn plain() -> i64 {
    1
}

/// `with` is a reserved word in JavaScript.
pub fn with() -> i64 {
    2
}

/// `unsupported` is the name the port itself writes for an R12 hole, so a crate
/// declaring one of its own would shadow it.
pub fn unsupported() -> i64 {
    3
}
