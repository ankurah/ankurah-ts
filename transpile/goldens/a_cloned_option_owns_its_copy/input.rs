// `cloned` over an option hands the caller its OWN value behind the null
// guard, so the collection keeps the one it holds.
//
// Written as the receiver itself it handed back the value the map still owns,
// and the release the caller owes then dropped that value a second time.

use std::collections::HashMap;

pub struct Subscription {
    pub id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {}
}

impl Clone for Subscription {
    fn clone(&self) -> Subscription {
        Subscription { id: self.id }
    }
}

pub fn id_of_one_subscription() -> u64 {
    let id: u64 = 7;
    let mut subscriptions: HashMap<u64, Subscription> = HashMap::new();
    subscriptions.insert(id, Subscription { id });
    let found = subscriptions.get(&id).cloned();
    match found {
        Some(subscription) => subscription.id,
        None => 0,
    }
}
