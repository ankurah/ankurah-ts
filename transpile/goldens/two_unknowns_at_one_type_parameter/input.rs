// Two collections handed to one generic parameter are the same type, and each
// says what the other holds.
//
// Read through the references the caller writes, `&mut source` and `&mut moved`
// both name `Vec<T>`, so `source` says what `moved` holds. Read with the
// reference erased neither said anything, and the Tag moved into it leaked.

pub struct Tag {
    pub n: u64,
}

impl Tag {
    pub fn new(n: u64) -> Tag {
        Tag { n }
    }
}

pub fn move_one<T>(from: &mut Vec<T>, into: &mut Vec<T>) {
    if let Some(item) = from.pop() {
        into.push(item);
    }
}

pub fn moved() -> usize {
    let mut source = Vec::new();
    source.push(Tag::new(4));
    let mut taken = Vec::new();
    move_one(&mut source, &mut taken);
    taken.len()
}
