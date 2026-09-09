// A bound projects through the type AS WRITTEN.
//
// `&Vec<Tag>` iterates `&Tag`, so `Bag::new(held)` with `I: IntoIterator<Item
// = T>` holds borrows, and what is drained out of it belongs to the caller.
// Read through the by-value impl instead, the bag holds owned `Tag`s and the
// loop below releases what the caller still owns.
//
// `held` inside `new` is untyped, and stays so: its element comes through a
// bound on this function's own type parameter, which one emitted body cannot
// decide (spec 4.4b).

pub struct Tag {
    pub text: String,
}

impl Tag {
    pub fn new(text: String) -> Tag {
        Tag { text }
    }
}

pub struct Bag<T> {
    items: Vec<T>,
}

impl<T> Bag<T> {
    pub fn new<I>(items: I) -> Bag<T>
    where
        I: IntoIterator<Item = T>,
    {
        let mut held = Vec::new();
        for item in items {
            held.push(item);
        }
        Bag { items: held }
    }

    pub fn drain(&mut self) -> Vec<T> {
        let mut out = Vec::new();
        while let Some(item) = self.items.pop() {
            out.push(item);
        }
        out
    }
}

pub fn widths_of(held: &Vec<Tag>) -> usize {
    let mut bag = Bag::new(held);
    let taken = bag.drain();
    let mut total = 0;
    for tag in taken {
        total += tag.text.len();
    }
    total
}

pub fn run() -> usize {
    let tags = vec![Tag::new("aa".to_string()), Tag::new("b".to_string())];
    widths_of(&tags)
}
