// TS-ONLY: the operators Rust spells with a symbol that JavaScript spells
// differently, or does not have at all.

/**
 * Rust's `a & b` on `bool`.
 *
 * `&&` short-circuits and `&` does not: `false & touch()` calls `touch`, and
 * the emitted `false && touch()` did not. That difference is invisible until an
 * operand has an effect — a call that mutates, logs, or advances an iterator —
 * and then the ported program takes a different path from the Rust one with
 * nothing to show for it. JavaScript evaluates both arguments of a call, left
 * to right, exactly once, so a call is the eager form.
 */
export function boolAnd(left: boolean, right: boolean): boolean {
  return left && right;
}

/** Rust's `a | b` on `bool`. Eager for the same reason as `boolAnd`. */
export function boolOr(left: boolean, right: boolean): boolean {
  return left || right;
}
