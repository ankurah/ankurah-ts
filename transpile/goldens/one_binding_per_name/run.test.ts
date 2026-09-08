// Runs the emitted one_binding_per_name against the real runtime.
//
// Against the parent's engine the first call is a `ReferenceError`: the
// declaring module exports `with_` and `unsupported_`, the calling module writes
// `with_()` and `unsupported_()`, and the import line names neither — because
// the map that says which module a name comes from held the UNESCAPED spelling.
// There was no diagnostic; the file simply named bindings nothing declares.

import { expect, test } from 'bun:test';
import { caller } from './input.ts';

test('a call across modules reaches the binding the declaration wrote', () => {
  expect(caller()).toBe(6n);
});
