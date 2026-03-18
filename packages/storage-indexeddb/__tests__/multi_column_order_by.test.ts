// MIRRORS: ankurah/storage/indexeddb-wasm/tests/multi_column_order_by.rs

import { describe, test, expect } from 'bun:test';
import {
  createIndexedDBNode, createProducts, Product,
  matchArgs, IndexedDBStorageEngine,
} from './common.ts';

function productNames(products: any[]): string[] { return products.map((p: any) => p.name()); }
function productPrices(products: any[]): number[] { return products.map((p: any) => p.price()); }
function productTuples(products: any[]): [string, string, number, number][] {
  return products.map((p: any) => [p.category(), p.name(), p.price(), p.stock()]);
}

describe('multi_column_order_by', () => {
  // Same-Direction ORDER BY Tests
  test('test_secondary_sort_asc_asc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
      ['Books', 'Textbook', 50, 30],
      ['Books', 'Magazine', 10, 200],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category ASC, name ASC'));
    expect(productNames(results)).toEqual(['Magazine', 'Novel', 'Textbook', 'Laptop', 'Phone', 'Tablet']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_secondary_sort_desc_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
      ['Books', 'Textbook', 50, 30],
      ['Books', 'Magazine', 10, 200],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category DESC, name DESC'));
    expect(productNames(results)).toEqual(['Tablet', 'Phone', 'Laptop', 'Textbook', 'Novel', 'Magazine']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Mixed-Direction ORDER BY Tests (Require order_by_spill)
  test('test_secondary_sort_asc_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
      ['Books', 'Textbook', 50, 30],
      ['Books', 'Magazine', 10, 200],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category ASC, name DESC'));
    expect(productNames(results)).toEqual(['Textbook', 'Novel', 'Magazine', 'Tablet', 'Phone', 'Laptop']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_secondary_sort_desc_asc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
      ['Books', 'Textbook', 50, 30],
      ['Books', 'Magazine', 10, 200],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category DESC, name ASC'));
    expect(productNames(results)).toEqual(['Laptop', 'Phone', 'Tablet', 'Magazine', 'Novel', 'Textbook']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_three_column_order_by', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['A', 'X', 100, 1],
      ['A', 'X', 200, 2],
      ['A', 'X', 50, 3],
      ['A', 'Y', 100, 4],
      ['A', 'Y', 200, 5],
      ['B', 'X', 100, 6],
      ['B', 'X', 200, 7],
    ]);

    const results = await ctx.fetch(Product, matchArgs('stock > 0 ORDER BY category ASC, name ASC, price DESC'));
    expect(productPrices(results)).toEqual([200, 100, 50, 200, 100, 200, 100]);

    expect(productTuples(results)).toEqual([
      ['A', 'X', 200, 2],
      ['A', 'X', 100, 1],
      ['A', 'X', 50, 3],
      ['A', 'Y', 200, 5],
      ['A', 'Y', 100, 4],
      ['B', 'X', 200, 7],
      ['B', 'X', 100, 6],
    ]);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Blocked by #210 in Rust
  test.skip('test_three_column_desc_desc_asc', () => {
    // Blocked by #210: i64 sorted lexicographically instead of numerically
  });

  // LIMIT with Multi-Column ORDER BY Tests (TopK Path)
  test('test_topk_desc_asc_limit', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['C', 'Apple', 100, 1],
      ['C', 'Banana', 100, 2],
      ['C', 'Cherry', 100, 3],
      ['B', 'Date', 100, 4],
      ['B', 'Elderberry', 100, 5],
      ['A', 'Fig', 100, 6],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category DESC, name ASC LIMIT 4'));
    expect(productNames(results)).toEqual(['Apple', 'Banana', 'Cherry', 'Date']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Blocked by #210 in Rust
  test.skip('test_topk_three_column_asc_asc_desc_limit', () => {
    // Blocked by #210: i64 sorted lexicographically instead of numerically
  });

  test.skip('test_topk_three_column_desc_desc_asc_limit', () => {
    // Blocked by #210: i64 sorted lexicographically instead of numerically
  });

  test('test_limit_respects_secondary_order_asc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['A', 'Zebra', 100, 1], ['A', 'Apple', 100, 2], ['A', 'Mango', 100, 3],
      ['B', 'Zebra', 100, 4], ['B', 'Apple', 100, 5],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category ASC, name ASC LIMIT 3'));
    expect(productNames(results)).toEqual(['Apple', 'Mango', 'Zebra']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_limit_respects_secondary_order_desc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['A', 'Zebra', 100, 1], ['A', 'Apple', 100, 2], ['A', 'Mango', 100, 3],
      ['B', 'Zebra', 100, 4], ['B', 'Apple', 100, 5],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category ASC, name DESC LIMIT 3'));
    expect(productNames(results)).toEqual(['Zebra', 'Mango', 'Apple']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_limit_at_category_boundary', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['A', 'Item1', 100, 1], ['A', 'Item2', 100, 2],
      ['B', 'Item3', 100, 3], ['B', 'Item4', 100, 4],
      ['C', 'Item5', 100, 5],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price > 0 ORDER BY category ASC, name ASC LIMIT 3'));
    expect(productTuples(results)).toEqual([
      ['A', 'Item1', 100, 1],
      ['A', 'Item2', 100, 2],
      ['B', 'Item3', 100, 3],
    ]);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Inequality + Multi-Column ORDER BY Tests
  test('test_inequality_with_secondary_sort', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
      ['Books', 'Textbook', 50, 30],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price >= 50 ORDER BY category ASC, name ASC'));
    expect(productNames(results)).toEqual(['Textbook', 'Laptop', 'Phone', 'Tablet']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_range_with_secondary_sort', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['A', 'P1', 100, 1], ['A', 'P2', 200, 2], ['A', 'P3', 300, 3],
      ['B', 'P4', 150, 4], ['B', 'P5', 250, 5], ['B', 'P6', 350, 6],
    ]);

    const results = await ctx.fetch(Product, matchArgs('price >= 150 AND price <= 300 ORDER BY category ASC, name DESC'));
    expect(productNames(results)).toEqual(['P3', 'P2', 'P5', 'P4']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Equality Prefix + Multi-Column ORDER BY Tests
  test('test_equality_prefix_with_secondary_sort_asc', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
    ]);

    const results = await ctx.fetch(Product, matchArgs("category = 'Electronics' ORDER BY name ASC"));
    expect(productNames(results)).toEqual(['Laptop', 'Phone', 'Tablet']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_equality_prefix_with_secondary_sort_mixed', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Laptop', 1000, 5],
      ['Electronics', 'Phone', 500, 10],
      ['Electronics', 'Tablet', 300, 15],
      ['Books', 'Novel', 20, 100],
    ]);

    const results = await ctx.fetch(Product, matchArgs("category = 'Electronics' ORDER BY name ASC, price DESC"));
    expect(productNames(results)).toEqual(['Laptop', 'Phone', 'Tablet']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_equality_prefix_with_duplicate_secondary', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Electronics', 'Widget', 100, 1],
      ['Electronics', 'Widget', 200, 2],
      ['Electronics', 'Widget', 50, 3],
      ['Electronics', 'Gadget', 150, 4],
    ]);

    const results = await ctx.fetch(Product, matchArgs("category = 'Electronics' ORDER BY name ASC, price DESC"));
    expect(productTuples(results)).toEqual([
      ['Electronics', 'Gadget', 150, 4],
      ['Electronics', 'Widget', 200, 2],
      ['Electronics', 'Widget', 100, 1],
      ['Electronics', 'Widget', 50, 3],
    ]);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  // Edge Cases
  test('test_empty_result_multi_column_order', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [['A', 'P1', 100, 1]]);

    const results = await ctx.fetch(Product, matchArgs("category = 'NonExistent' ORDER BY name ASC"));
    expect(results.length).toBe(0);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_single_result_multi_column_order', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [['A', 'P1', 100, 1]]);

    const results = await ctx.fetch(Product, matchArgs("category = 'A' ORDER BY name ASC"));
    expect(results.length).toBe(1);
    expect(productNames(results)).toEqual(['P1']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_all_duplicates_primary_same_direction', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Same', 'C', 300, 1], ['Same', 'A', 100, 2], ['Same', 'B', 200, 3], ['Same', 'D', 50, 4],
    ]);

    const results = await ctx.fetch(Product, matchArgs('stock > 0 ORDER BY category ASC, name ASC'));
    expect(productNames(results)).toEqual(['A', 'B', 'C', 'D']);

    await IndexedDBStorageEngine.cleanup(dbName);
  });

  test('test_all_duplicates_primary_mixed_direction', async () => {
    const { node, dbName } = await createIndexedDBNode();
    const ctx = node.context();

    await createProducts(ctx, [
      ['Same', 'C', 300, 1],
      ['Same', 'A', 100, 2],
      ['Same', 'B', 200, 3],
      ['Same', 'A', 50, 4], // Duplicate name, different price
    ]);

    const results = await ctx.fetch(Product, matchArgs('stock > 0 ORDER BY category ASC, name ASC, price DESC'));
    expect(productTuples(results)).toEqual([
      ['Same', 'A', 100, 2],
      ['Same', 'A', 50, 4],
      ['Same', 'B', 200, 3],
      ['Same', 'C', 300, 1],
    ]);

    await IndexedDBStorageEngine.cleanup(dbName);
  });
});
