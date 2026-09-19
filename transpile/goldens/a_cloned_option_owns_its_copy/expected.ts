// MIRRORS: ankurah/a_cloned_option_owns_its_copy/src/input.rs
import { Struct, Drop, dropOwned, HashMap } from '@ankurah/base';

export class Subscription extends Drop {
  readonly id: bigint;

  constructor(id: bigint) {
    super();
    this.id = id;
  }

  protected override onDrop(): void {

  }

  clone(): Subscription {
    return new Subscription(this.id);
  }
}

export function idOfOneSubscription(): bigint {
  const id = 7n;
  let subscriptions = new HashMap<bigint, Subscription>();
  try {
    subscriptions.set(id, new Subscription(id));
    const found = subscriptions.get(id)?.clone() ?? null;
    if (found != null) {
      const subscription = found;
      try {
        return subscription.id;
      } finally {
        subscription.drop();
      }
    } else {
      return 0n;
    }
  } finally {
    dropOwned(subscriptions);
  }
}

