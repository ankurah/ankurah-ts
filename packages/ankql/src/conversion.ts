// MIRRORS: ankurah/ankql/src/conversion.rs

import type { Predicate } from './ast.ts';
import { Selection } from './ast.ts';
import { parseSelection } from './parser.ts';

/** Parse a predicate string into a Predicate AST node. */
export function parsePredicate(input: string): Predicate {
  return parseSelection(input).predicate;
}

/** Parse a selection string into a Selection AST node. */
export function selectionFromString(input: string): Selection {
  return parseSelection(input);
}
