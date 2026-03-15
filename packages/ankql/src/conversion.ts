// MIRRORS: ankurah/ankql/src/conversion.rs

import { Expr, Predicate, Literal, Selection } from './ast.ts';
import { InvalidPredicateError } from './error.ts';
import { parseSelection } from './parser.ts';

// impl TryFrom<&str> for Predicate
// impl TryFrom<String> for Predicate
/** Parse a predicate string into a Predicate AST node. */
export function parsePredicate(input: string): Predicate {
  return parseSelection(input).predicate;
}

// impl TryFrom<&str> for Selection
// impl TryFrom<String> for Selection
/** Parse a selection string into a Selection AST node. */
export function selectionFromString(input: string): Selection {
  return parseSelection(input);
}

// impl TryFrom<Expr> for Predicate
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

// #[cfg(feature = "wasm")] TryFrom<JsValue> for Expr — skipped (E9)
