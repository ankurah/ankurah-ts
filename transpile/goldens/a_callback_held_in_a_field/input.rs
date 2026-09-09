// A field that holds a callback holds whatever was put in it.
//
// A closure that captured values with drop glue is an `OwnedClosure` at run
// time, and a plain arrow otherwise; the field's type has to have room for both
// and the call through it has to tell them apart. Written as an arrow and
// called directly, the field refused the wrapper the caller handed it.

pub struct Tag {
    pub id: u64,
}

impl Tag {
    pub fn new(id: u64) -> Tag {
        Tag { id }
    }
}

pub struct Holder {
    pub compute: Box<dyn Fn() -> u64>,
}

impl Holder {
    pub fn new<F>(compute: F) -> Holder
    where
        F: Fn() -> u64 + 'static,
    {
        Holder { compute: Box::new(compute) }
    }

    pub fn read(&self) -> u64 {
        (self.compute)()
    }
}

pub fn over_a_capture() -> u64 {
    let tag = Tag::new(6);
    let holder = Holder::new(move || tag.id);
    holder.read() + holder.read()
}
