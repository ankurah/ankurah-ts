//! What the port knows about a type whose TypeScript a PERSON wrote.
//!
//! The declaration is still the engine's — the type resolves, its fields have
//! types, its derives register impls — but the members are whatever that person
//! wrote, and the engine never reads the file. So each fact a hook needs is
//! stated once, by the `[provided_impls]` entry, and asked here.

use super::TypeRegistry;
use crate::ty::TypeId;

impl TypeRegistry {
    /// Record that this type's TypeScript is written by hand rather than
    /// emitted — a `[provided_impls]` entry, or a type declared in a
    /// `[hardcode]` file.
    ///
    /// The declaration is still the engine's: the type resolves, its fields
    /// have types, and its derives register impls, because the Rust source says
    /// so. What changes is that the *members* are whatever the hand-written file
    /// wrote, so a hook that would call a method the emitter generates — the
    /// `debug()` a `#[derive(Debug)]` writes, the `toJSON` a serde derive writes
    /// — has to say it cannot, instead of calling a method that is not there.
    pub fn mark_hand_written(&mut self, id: TypeId) {
        self.hand_written.insert(id);
    }

    /// Is this type's TypeScript written by hand?
    ///
    /// Answers ONE of the two questions "hand-written" used to answer at once:
    /// **may this run emit an impl whose self type is this?** It must not, where
    /// the class the methods would join is hand-written in THIS crate —
    /// `Attested<T>`'s conversions are in `auth.provided.ts`, and emitting them
    /// again would give the port two of each. An impl a DIFFERENT crate writes
    /// for such a type is that crate's own code and is emitted, as the
    /// module-level functions an impl away from its class becomes: core's
    /// `impl OrderedCollation for EntityId` is one.
    pub fn is_hand_written(&self, id: TypeId) -> bool {
        self.hand_written.contains(&id)
    }

    /// Record that this hand-written type's file declares `debug(): string`.
    pub fn mark_declares_debug(&mut self, id: TypeId) {
        self.declares_debug.insert(id);
    }

    /// Does this hand-written type's file declare `debug(): string`?
    ///
    /// Only the `[provided_impls]` entry can say: the engine never reads the
    /// TypeScript it did not write. Without it a `#[derive(Debug)]` on a type
    /// holding one printed the field through `toString`, which for a class is
    /// `[object Object]` — forty-five emitted fields in `proto` alone.
    pub fn declares_debug(&self, id: TypeId) -> bool {
        self.declares_debug.contains(&id)
    }

    /// Record that this type's MEMBERS are whatever a hand-written file wrote,
    /// wherever that file lives.
    pub fn mark_members_hand_written(&mut self, id: TypeId) {
        self.members_hand_written.insert(id);
    }

    /// The other question: **does this class have emitted members I may call?**
    ///
    /// It does not, for a type whose TypeScript a person wrote — this crate's
    /// or a sibling's. The engine has not read that file, so the `debug()` a
    /// `#[derive(Debug)]` would have written and the `toJSON` a serde derive
    /// would have written are not there to call. Asked through
    /// `is_hand_written`, which only ever knew THIS crate's provided types, 26
    /// emitted `${x.debug()}` calls in core named a method
    /// `id.provided.ts` does not declare.
    pub fn members_are_hand_written(&self, id: TypeId) -> bool {
        self.hand_written.contains(&id) || self.members_hand_written.contains(&id)
    }

    /// Record that this type's `#[derive(Deserialize)]` writes it a
    /// `static fromJson`.
    pub fn mark_reads_json(&mut self, id: TypeId) {
        self.reads_json.insert(id);
    }

    /// Does a `static fromJson` exist on this type's emitted class?
    ///
    /// `serde_json::from_str::<T>(text)` is written as `T.fromJson(JSON.parse(
    /// text))`, and a `T` with no such static turns a parse error into a
    /// `TypeError` at the call. Asking here is what makes the emission
    /// conditional rather than hopeful.
    pub fn reads_json(&self, id: TypeId) -> bool {
        self.reads_json.contains(&id)
    }
}
