// A hole is what an emitted file carries where the port has no lowering: it
// stops the program and names the Rust shape, rather than running the nearest
// thing the engine could write. See port/ownership.md, "Holes".

import { expect, test } from 'bun:test';
import { unsupported, UnsupportedShape } from '../src/index.ts';

test('a hole stops the program and names the shape it stands on', () => {
  expect(() => unsupported('a consuming match arm with a guard')).toThrow(UnsupportedShape);
  try {
    unsupported('a consuming match arm with a guard');
  } catch (e) {
    expect(e).toBeInstanceOf(UnsupportedShape);
    expect((e as UnsupportedShape).what).toBe('a consuming match arm with a guard');
    expect((e as Error).message).toContain('a consuming match arm with a guard');
    expect((e as Error).name).toBe('UnsupportedShape');
  }
});

test('a hole is not an Error the port hands back as a value', () => {
  // It is thrown, never returned: the emitter writes it where an expression
  // stands, and its `never` return type is what lets it stand there.
  const reached: string[] = [];
  const run = (): number => {
    reached.push('before');
    unsupported('a dropped `..rest`');
  };
  expect(run).toThrow(/no translation/);
  expect(reached).toEqual(['before']);
});
