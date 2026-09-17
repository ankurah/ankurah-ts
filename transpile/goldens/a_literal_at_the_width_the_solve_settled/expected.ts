// MIRRORS: ankurah/a_literal_at_the_width_the_solve_settled/src/input.rs
import { checkedAdd } from '@ankurah/base';

export function takeU64(v: bigint): bigint {
  return v;
}

export function settledLater(): bigint {
  const x = 3n;
  takeU64(x);
  return checkedAdd(x, 1n, 'u64');
}

export function settledByAReturn(): bigint {
  const y = 5n;
  return y;
}

