// MIRRORS: ankurah/ankql/src/lib.rs
//
// @ankurah/ankql -- AnkQL query language parser and predicate evaluator.
//
// Hand-written grammar matcher and AST builder (Exception E6: no Pest equivalent
// in TS) — grammar.ts stands in for the pest derive, parser.ts is a port of
// parser.rs. Parses AnkQL query strings into AST nodes and evaluates predicates
// locally for optimistic filtering.
//
// Rust crate: ankql

export * from './ast.ts';
export * from './conversion.ts';
export * from './error.ts';
export * from './grammar.ts';
export * from './parser.ts';
export * from './selection/index.ts';
