// A loop's pattern is typed by the sequence's element, and the element by what
// was put into the sequence.
//
// `for (entity, event) in entity_events` names two values whose types nothing
// in the loop head writes: they are the halves of the pair the `Vec` holds, and
// the `Vec` was written `Vec::new()`. With the element unknown both names were
// bound untyped, so neither was released and neither could answer a call.

pub struct Entity {
    pub id: u64,
}

pub struct Event {
    pub name: String,
}

pub fn describe(entity_events: Vec<(Entity, Event)>) -> Vec<String> {
    let mut lines = Vec::new();
    for (entity, event) in entity_events {
        lines.push(event.name);
        lines.push(format!("{}", entity.id));
    }
    lines
}

pub fn describe_built() -> Vec<String> {
    let mut entity_events = Vec::new();
    entity_events.push((Entity { id: 1 }, Event { name: String::from("created") }));
    let mut lines = Vec::new();
    for (entity, event) in entity_events {
        lines.push(event.name);
        lines.push(format!("{}", entity.id));
    }
    lines
}

pub fn collected() -> usize {
    describe_built().len()
}
