// MIRRORS: ankurah/storage/common/src/predicate.rs

import { Predicate } from '@ankurah/ankql';

/**
 * Extracts conjuncts that can be flattened from a predicate tree.
 *
 * Conjuncts are predicates connected by AND operations that can be
 * extracted and used independently for index planning. OR operations
 * break conjunct chains since they require different evaluation logic.
 *
 * Example: `(foo = 1 AND (? AND bar = 2)) AND (? OR (baz = 3 AND zed = 4))`
 * - `foo = 1` and `bar = 2` are conjuncts (can be flattened)
 * - `baz = 3` and `zed = 4` are NOT conjuncts (blocked by OR)
 *
 * Future optimization potential: Special handling for patterns like
 * `foo > 10 OR foo > 20` where both branches use the same field.
 *
 * Rust: `pub struct ConjunctFinder`
 */
export class ConjunctFinder {
  /**
   * Extract all top-level conjuncts from a predicate tree.
   * Returns predicates in order of appearance.
   */
  // Rust: fn find
  static find(predicate: Predicate): Predicate[] {
    const conjuncts: Predicate[] = [];
    ConjunctFinder.extractConjuncts(predicate, conjuncts);
    return conjuncts;
  }

  /**
   * Recursively extract conjuncts, stopping at OR boundaries.
   */
  // Rust: fn extract_conjuncts
  private static extractConjuncts(predicate: Predicate, conjuncts: Predicate[]): void {
    predicate.match({
      And: (v) => {
        // Recursively extract from both sides of AND
        ConjunctFinder.extractConjuncts(v.left, conjuncts);
        ConjunctFinder.extractConjuncts(v.right, conjuncts);
      },
      Or: () => {
        // OR breaks conjunct chains - treat the entire OR as a single conjunct
        conjuncts.push(predicate);
      },
      Comparison: () => { conjuncts.push(predicate); },
      IsNull: () => { conjuncts.push(predicate); },
      Not: () => { conjuncts.push(predicate); },
      True: () => { conjuncts.push(predicate); },
      False: () => { conjuncts.push(predicate); },
      Placeholder: () => { conjuncts.push(predicate); },
    });
  }
}
