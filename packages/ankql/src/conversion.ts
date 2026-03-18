// MIRRORS: ankurah/ankql/src/conversion.rs

import { Expr, Predicate, Literal, Selection } from './ast.ts';
import { InvalidPredicateError } from './error.ts';
import { parseSelection } from './parser.ts';

// Rust: fn try_from (TryFrom<&str> for Predicate, TryFrom<String> for Predicate)
/** Parse a predicate string into a Predicate AST node. */
export function parsePredicate(input: string): Predicate {
  return parseSelection(input).predicate;
}

// Rust: fn try_from (TryFrom<&str> for Selection, TryFrom<String> for Selection)
/** Parse a selection string into a Selection AST node. */
export function selectionFromString(input: string): Selection {
  return parseSelection(input);
}

// Rust: fn try_from (TryFrom<Expr> for Predicate)
/** Convert an Expr to a Predicate. Throws InvalidPredicateError if not convertible. */
export function predicateFromExpr(expr: Expr): Predicate {
  return expr.match({
    Predicate: (v) => v.predicate,
    Placeholder: () => Predicate.Placeholder(),
    Literal: (v) => {
      if (v.literal.is('Bool')) {
        const boolVal = (v.literal.value as { value: boolean }).value;
        return boolVal ? Predicate.True() : Predicate.False();
      }
      throw new InvalidPredicateError('Expression is not a predicate');
    },
    Path: () => { throw new InvalidPredicateError('Expression is not a predicate'); },
    InfixExpr: () => { throw new InvalidPredicateError('Expression is not a predicate'); },
    ExprList: () => { throw new InvalidPredicateError('Expression is not a predicate'); },
  });
}

// Rust: fn try_from (TryFrom<JsValue> for Expr) — SKIP: #[cfg(feature = "wasm")] [E9]
