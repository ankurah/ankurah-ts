// MIRRORS: ankurah/an_atomic_write_through_a_holder/src/input.rs
import { Arc, unsupported } from '@ankurah/base';

export function readBack(): number {
  const counter = Arc.new(7);
  try {
    return counter.value;
  } finally {
    counter.drop();
  }
}

export function bump(): number {
  const counter = Arc.new(0);
  try {
    const clone = counter.clone();
    try {
      unsupported('`fetch_add` WRITES what the `Arc<AtomicUsize>` holds, and it is reached through an accessor that hands out the value rather than the place');
      return counter.value;
    } finally {
      clone.drop();
    }
  } finally {
    counter.drop();
  }
}

