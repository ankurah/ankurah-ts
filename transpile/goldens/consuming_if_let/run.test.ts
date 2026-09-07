// Runs the emitted consuming_if_let against the real runtime. What is under
// test is who owns the value an `if let` takes a payload OUT of.
//
// Three defective paths are driven. `rightLeaf` on a `Pair` takes `right` out
// and leaves `left` and `op` behind: the parent released neither and then
// released the whole node again with the taken binding inside it, which the
// runtime reports as a use after move. `onlyPairs` on an `End` takes the path
// the pattern did not match, where Rust drops the value the `if let` read.
// `fromAClone` moves a field out of a temporary the port holds, whose own
// release cascaded into the field the callee had already taken.

import { expect, test } from 'bun:test';
import { Holder, Inner, Node, Op, fromAClone, onlyPairs, rightLeaf } from './input.ts';
import { expectNoOwnershipReports } from './leaks.ts';

const pair = (left: Node, op: Op, right: Node) => new Node('Pair', { left, op, right });

test('rightLeaf walks into the field the pattern took', () => {
  expect(rightLeaf(pair(Node.leaf(1), new Op(9), Node.leaf(4)))).toBe(4);
});

test('a nested right side is walked too', () => {
  const inner = pair(Node.leaf(2), new Op(8), Node.leaf(3));
  expect(rightLeaf(pair(Node.leaf(1), new Op(9), inner))).toBe(8 + 2 + 3);
});

test('rightLeaf hands back zero for a node the pattern does not match', () => {
  expect(rightLeaf(Node.leaf(5))).toBe(0);
});

test('onlyPairs reads the field it bound', () => {
  expect(onlyPairs(pair(Node.leaf(1), new Op(9), Node.leaf(4)))).toBe(9);
});

test('onlyPairs takes the node it did not match', () => {
  const node = Node.leaf(5);
  expect(onlyPairs(node)).toBe(7);
  // `intoMatch` MOVES the enum and hands the payload to the arm, which released
  // it: the enum object is the port's own and is never dropped or reported.
  expect(node.isMoved).toBe(true);
});

test('a field moved out of a clone leaves the rest of the clone releasable', () => {
  expect(fromAClone(new Holder(new Inner(4), 3))).toBe(7);
});

test('nothing leaked and nothing was dropped twice', async () => {
  await expectNoOwnershipReports();
});
