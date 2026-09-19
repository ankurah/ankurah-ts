// Runs the emitted an_atomics_operands_run_once against the runtime.
//
// The tick count is the operand's own witness: it is what Rust leaves behind.

import { afterAll, expect, test } from 'bun:test';
import { expectNoOwnershipReports } from './leaks.ts';
import {
  aHighBitStaysUnsigned,
  boundsReadTheirOperandOnce,
  compareExchangeReadsItsNewValueOnce,
  logicReadsItsOperandOnce,
} from './input.ts';

test('fetch_max and fetch_min read their operand once, stored or not', () => {
  expect(boundsReadTheirOperandOnce()).toEqual([0, 1, 1, 1, 2]);
});

test('an AtomicBool reads its operand whichever way its own value goes', () => {
  expect(logicReadsItsOperandOnce()).toEqual([false, false, true, true, true]);
});

test('compare_exchange reads its new value even where the comparison fails', () => {
  expect(compareExchangeReadsItsNewValueOnce()).toEqual([false, true, true, true]);
});

test('an unsigned atomic stores the bits back at its own width', () => {
  expect(aHighBitStaysUnsigned()).toEqual([1, 2147483649, 2147483649, 2147483648]);
});

afterAll(async () => {
  await expectNoOwnershipReports();
});
