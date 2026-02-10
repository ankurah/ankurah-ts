// MIRRORS: ankurah/ankql/src/lib.rs
//
// @ankurah/ankql — AnkQL query language parser and predicate evaluator.
//
// Hand-written recursive descent parser (Exception E6: no Pest equivalent in TS).
// Parses AnkQL query strings into AST nodes and evaluates predicates locally
// for optimistic filtering.
//
// Rust crate: ankql
// Key types: Selection, Predicate, Expression, Operator, Value
//
// TODO: Port parser and AST types from ankurah/ankql/src/
