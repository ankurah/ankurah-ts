// A closure's parameters come from the position it stands in, and the walk that
// collects a body's constraints reads that position before it walks in.
//
// `bus.each(|entity| ..)` types `entity` from `F: Fn(&Entity) -> usize`. The
// walk met the closure without that bound, so the parameter stood for nothing,
// `entity.tag()` did not resolve, and the collection inside the body kept an
// element the engine never named: nothing released the tags it held.

pub struct Tag {
    pub text: String,
}

pub struct Entity {
    pub id: u64,
}

impl Entity {
    pub fn tag(&self) -> Tag {
        Tag { text: "e".to_string() }
    }
}

pub struct Bus {
    pub entities: Vec<Entity>,
}

impl Bus {
    pub fn each<F: Fn(&Entity) -> usize>(&self, f: F) -> usize {
        let mut total = 0;
        for entity in &self.entities {
            total += f(entity);
        }
        total
    }
}

pub fn over_each<F: Fn(&Entity) -> usize>(entities: &Vec<Entity>, f: F) -> usize {
    let mut total = 0;
    for entity in entities {
        total += f(entity);
    }
    total
}

pub fn tag_lengths_through_a_method(bus: &Bus) -> usize {
    bus.each(|entity| {
        let mut tags = Vec::new();
        tags.push(entity.tag());
        tags.len()
    })
}

pub fn tag_lengths_through_a_function(entities: &Vec<Entity>) -> usize {
    over_each(entities, |entity| {
        let mut tags = Vec::new();
        tags.push(entity.tag());
        tags.len()
    })
}
