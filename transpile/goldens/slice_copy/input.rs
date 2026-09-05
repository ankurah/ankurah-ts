// `to_vec` and `to_owned` on a slice COPY it, and Rust's own signature says how:
// `T: Clone`, one clone per element. `slice()` copies the ARRAY and leaves both
// copies holding the same elements, so the port had two owners for one value and
// the second drop was a fatal — core's `subscription_state.ts` and `node.ts`,
// proto's `clock.ts`.
//
// `Vec::clone` is the same operation under a THIRD name — Rust's own signature
// says `T: Clone`, one clone per element — and it fell through to a bare
// `.clone()` on a JavaScript array with no diagnostic at all, live at core's
// `reactor.test.ts`. And `String::clone` is the identity, because a JavaScript
// string is a value: `name.clone()` called a method a string has not got, live
// at `core/entity.ts:61`.

pub struct Event {
    pub n: u32,
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Event { n: self.n }
    }
}

pub struct Batch {
    pub events: Vec<Event>,
}

impl Batch {
    /// The caller gets its OWN events, which it drops; this one keeps its own.
    pub fn copy_of_events(&self) -> Vec<Event> {
        self.events.to_vec()
    }

    /// The same for the numbers, where there is nothing inside to copy.
    pub fn copy_of_counts(counts: &[u32]) -> Vec<u32> {
        counts.to_owned()
    }

    /// `to_owned` goes through the conversion table rather than the array one,
    /// and used to write `[...xs]` — the same shallow copy under another name.
    pub fn owned_events(events: &Vec<Event>) -> Vec<Event> {
        events.to_owned()
    }

    /// `Vec::clone` is `to_vec` under a third name, and used to be a bare
    /// `.clone()` on an array.
    pub fn cloned_events(&self) -> Vec<Event> {
        self.events.clone()
    }
}

pub struct Named {
    pub name: String,
}

impl Named {
    /// A JavaScript string is a value, so all three of Rust's spellings for
    /// "give me an owned `String`" are the receiver itself.
    pub fn spellings(&self) -> (String, String, String) {
        (self.name.clone(), self.name.to_owned(), self.name.to_string())
    }
}
