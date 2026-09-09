// A bound says what the argument can DO, not what it is; what the two agree on
// is what the bound PROJECTS.
//
// `Holder::new` declares `I: IntoIterator<Item = F>`, so a `Vec<Tag>` handed to
// it says `F` is `Tag` — the item the impl table reads out of a `Vec<Tag>`.
// Unifying the bound with the argument instead says nothing, and the holder's
// element stayed an unknown: what came back out of it was released by nobody.
//
// `held` inside `new` is still untyped, and stays so: its element comes through
// a bound on this function's OWN type parameter, which one emitted body cannot
// decide (spec 4.4b).

pub struct Tag {
    pub text: String,
}

pub struct Holder<F> {
    items: Vec<F>,
}

impl<F> Holder<F> {
    pub fn new<I>(items: I) -> Holder<F>
    where
        I: IntoIterator<Item = F>,
    {
        let mut held = Vec::new();
        for item in items {
            held.push(item);
        }
        Holder { items: held }
    }

    pub fn take(&mut self) -> F {
        self.items.remove(0)
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }
}

pub fn take_one(tags: Vec<Tag>) -> usize {
    let mut holder = Holder::new(tags);
    let taken = holder.take();
    let left = holder.count();
    left + taken.text.len()
}
