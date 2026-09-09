//! What the registry promises about the names it holds: one leaf declared in
//! two modules stays two types, a provided type still takes impls, a foreign
//! path is stable, and an alias cycle stops instead of recursing.

use super::*;

fn a_struct(name: &str) -> TypeDecl {
    TypeDecl {
        name: name.to_string(),
        kind: TypeKind::Struct,
        type_params: Vec::new(),
        vis: Vis::Public,
    }
}

#[test]
fn declaring_the_same_leaf_name_in_two_modules_keeps_both() {
    let mut reg = TypeRegistry::new("signals");
    let broadcast = reg.modules_mut().module_for_file("broadcast.rs");
    let system = reg.system_root();
    let crate_ref = reg.declare_type(broadcast, a_struct("Ref")).unwrap();
    let system_ref = reg.declare_type(system, a_struct("Ref")).unwrap();

    assert_ne!(crate_ref, system_ref);
    assert!(!reg.is_system(crate_ref));
    assert!(reg.is_system(system_ref));
    assert_eq!(reg.modules().get(broadcast).path, vec!["broadcast"]);
    assert!(reg.modules().get(reg.system_root()).is_system);
}

/// "Hand-written" answered two questions at once, and they have different
/// answers for a SIBLING crate's provided type: proto's `EntityId` has no
/// emitted `debug()` for core to call — its TypeScript is in
/// `id.provided.ts` — and core's `impl OrderedCollation for EntityId` is
/// core's own code and still emits.
#[test]
fn a_siblings_provided_type_has_no_members_and_still_takes_impls() {
    let mut reg = TypeRegistry::new("core");
    let here = reg.modules_mut().module_for_file("entity.rs");
    let ours = reg.declare_type(here, a_struct("Attested")).unwrap();
    let theirs = reg.declare_type(here, a_struct("EntityId")).unwrap();

    reg.mark_hand_written(ours);
    reg.mark_members_hand_written(theirs);

    // This crate's own provided type answers both questions the same way.
    assert!(reg.is_hand_written(ours));
    assert!(reg.members_are_hand_written(ours));

    // A sibling's answers only the members question, so an impl this crate
    // writes for it is still emitted.
    assert!(!reg.is_hand_written(theirs));
    assert!(reg.members_are_hand_written(theirs));

    // And a type nobody wrote by hand answers neither.
    let emitted = reg.declare_type(here, a_struct("Entity")).unwrap();
    assert!(!reg.is_hand_written(emitted));
    assert!(!reg.members_are_hand_written(emitted));
}

#[test]
fn foreign_types_are_stable_and_named_by_their_last_segment() {
    let reg = TypeRegistry::new("proto");
    let a = reg.foreign(&["ulid".into(), "Ulid".into()]).unwrap();
    let b = reg.foreign(&["ulid".into(), "Ulid".into()]).unwrap();
    let c = reg.foreign(&["anyhow".into(), "Error".into()]).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.is_foreign());
    assert_eq!(reg.name_of(a), "Ulid");
    assert!(reg.def(a).is_none());
    assert_eq!(reg.undeclared_reported(), 0, "nothing has been reported yet");
    reg.mark_reported(a);
    assert_eq!(reg.undeclared_reported(), 1);
}

#[test]
fn an_alias_cycle_stops_instead_of_recursing() {
    let reg = TypeRegistry::new("c");
    let id = AliasId(0);
    let inner = reg.expanding_alias(id, || reg.expanding_alias(id, || 1u8));
    assert_eq!(inner, Some(None), "the second entry is refused");
    assert_eq!(
        reg.expanding_alias(id, || 2u8),
        Some(2),
        "and the stack unwinds"
    );
}
