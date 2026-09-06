//! One entry of `[provided_impls]`: a type whose TypeScript a person wrote, and
//! the facts about that file only the entry can state.

#[derive(Debug, Clone)]
pub struct ProvidedImpl {
    pub path: String,
    /// Does the hand-written file declare `static fromJson` for this type?
    ///
    /// The engine never reads the TypeScript it did not write, so a provided
    /// type's members are whatever the person who wrote the file wrote. Reading
    /// "it is hand-written" as "it reads JSON" put `Attested.fromJson` in three
    /// emitted call sites where `auth.provided.ts` declares no such static.
    /// Each entry says which it is, and a type that does not is refused —
    /// transitively, so nothing holding one gets a JSON half either.
    pub reads_json: bool,
    /// Does the hand-written file declare `debug(): string` for this type?
    ///
    /// Same reason as `reads_json`: the engine never reads the TypeScript it
    /// did not write, so only the entry can say. Without it a `#[derive(Debug)]`
    /// on a type holding one of these printed the field through `toString`,
    /// which for a class is `[object Object]` — forty-five emitted fields in
    /// `proto` alone, every one of them an `EntityId`, a `Clock` or an
    /// `Attested` in a `Debug` line.
    pub has_debug: bool,
}
