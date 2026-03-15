// MIRRORS: ankurah/core/src/property/value/json.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { Json } from './json.ts';
import { PropertyError } from '../traits.ts';

describe('Json', () => {
  // Rust: fn test_json_roundtrip()
  test('json roundtrip', () => {
    const original = Json.object([
      ['name', 'test'],
      ['count', 42],
      ['nested', { inner: 'value' }],
    ]);

    // Convert to Value and back
    const value = original.intoValue()!;
    const recovered = Json.fromValue(value);

    expect(recovered.inner).toEqual(original.inner);
  });

  // Rust: fn test_json_get_path()
  test('json get_path', () => {
    const json = Json.new({
      licensing: {
        territory: 'US',
        rights: {
          holder: 'Label',
        },
      },
    });

    expect(json.getPath(['licensing', 'territory'])).toBe('US');
    expect(json.getPath(['licensing', 'rights', 'holder'])).toBe('Label');
    expect(json.getPath(['licensing', 'nonexistent'])).toBeUndefined();
    expect(json.getPath(['nonexistent'])).toBeUndefined();
  });

  // Rust: fn test_json_null()
  test('json null', () => {
    const json = Json.null();
    expect(json.isNull()).toBe(true);

    const value = json.intoValue()!;
    const recovered = Json.fromValue(value);
    expect(recovered.isNull()).toBe(true);
  });

  // Rust: fn test_json_missing()
  test('json missing', () => {
    expect(() => Json.fromValue(null)).toThrow();
    try {
      Json.fromValue(null);
    } catch (e) {
      expect(e).toBeInstanceOf(PropertyError);
      expect((e as PropertyError).kind).toBe('Missing');
    }
  });

  // Rust: fn test_json_invalid_variant()
  test('json invalid variant', () => {
    expect(() => Json.fromValue({ type: 'String', value: 'not json bytes' })).toThrow();
    try {
      Json.fromValue({ type: 'String', value: 'not json bytes' });
    } catch (e) {
      expect(e).toBeInstanceOf(PropertyError);
      expect((e as PropertyError).kind).toBe('InvalidVariant');
    }
  });

  // Rust: fn test_json_deref()
  test('json deref (object access)', () => {
    const json = Json.new({ key: 'value' });

    // Divergence: No Deref in TS — use isObject() and getPath() instead
    expect(json.isObject()).toBe(true);
    expect(json.getPath(['key'])).toBe('value');
  });
});
