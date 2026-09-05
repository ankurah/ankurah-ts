//! The types a pattern's names take from the value being taken apart.
//!
//! A `match` arm, an `if let`, a `for` loop and a destructuring `let` all
//! introduce names, and each one has a type the scrutinee decides. Split out of
//! `context.rs`, which had grown past the point where a reader could hold it.

use super::context::member_name;
use super::TypeContext;
use crate::name_map;
use crate::ty::{bind_params, TraitRef, Ty};

impl TypeContext<'_> {
    // ── Patterns ───────────────────────────────────────────────────────

    /// Bind every name a pattern introduces, typed from the value it destructures.
    ///
    /// Returns the names it bound without a type, so the caller can say once
    /// which part of the pattern the engine could not read rather than once per
    /// use of each name.
    pub fn bind_pattern(&mut self, pat: &syn::Pat, ty: Option<&Ty>) -> Vec<String> {
        let mut untyped = Vec::new();
        self.bind_pattern_into(pat, ty, Mode::Move, &mut untyped);
        untyped
    }

    /// Bind one pattern against one value.
    ///
    /// `mode` is Rust's default binding mode (RFC 2005). Matching a
    /// non-reference pattern against a reference peels one layer and remembers
    /// that it did, so every name underneath binds by reference:
    /// `match &entry { Entry { key } => .. }` gives `key: &K`, not `K`. An
    /// explicit `&pat` consumes exactly one layer and puts the mode back, and
    /// `ref x` binds a reference whatever the mode is.
    fn bind_pattern_into(
        &mut self,
        pat: &syn::Pat,
        ty: Option<&Ty>,
        mode: Mode,
        untyped: &mut Vec<String>,
    ) {
        match pat {
            syn::Pat::Ident(ident) => {
                let name = ident.ident.to_string();
                // A bare uppercase name in a pattern may be a unit variant of
                // what is being matched rather than a new binding, which is how
                // `None` and `Ordering::Less` are written.
                if let Some(Ty::Named { id, .. }) = ty.map(|t| t.peel_refs()) {
                    if self.registry.is_variant_of(*id, &name) {
                        return;
                    }
                }
                if let Some(sub) = &ident.subpat {
                    self.bind_pattern_into(&sub.1, ty, mode, untyped);
                }
                // `ref x` and `ref mut x` say the borrow outright; otherwise the
                // default binding mode says it.
                let bound = match (&ident.by_ref, ident.mutability.is_some()) {
                    (Some(_), mutable) => mode_of(mutable),
                    (None, _) => mode,
                };
                let local = name_map::to_camel_case(&name);
                match ty.map(|t| bound.apply(t)) {
                    Some(ty) => self.bind(&local, ty),
                    None => {
                        self.bind_untyped(&local);
                        untyped.push(local);
                    }
                }
            }

            // `&pat` matches the reference itself: one layer off, and the mode
            // starts again from the type underneath.
            syn::Pat::Reference(r) => {
                let inner = match ty {
                    Some(Ty::Ref { inner, .. }) => Some((**inner).clone()),
                    // Matching `&pat` against a value already peeled by the
                    // default mode is what `match &v { &x => .. }` does.
                    other => other.cloned(),
                };
                self.bind_pattern_into(&r.pat, inner.as_ref(), Mode::Move, untyped)
            }

            syn::Pat::Paren(p) => self.bind_pattern_into(&p.pat, ty, mode, untyped),

            syn::Pat::Type(t) => {
                // A written type is the whole answer, borrows included.
                let written = self.resolve_written_type(&t.ty).ok();
                match written {
                    Some(written) => {
                        self.bind_pattern_into(&t.pat, Some(&written), Mode::Move, untyped)
                    }
                    None => self.bind_pattern_into(&t.pat, ty, mode, untyped),
                }
            }

            // Every alternative binds the same names, so each is bound against
            // the same value.
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    self.bind_pattern_into(case, ty, mode, untyped);
                }
            }

            syn::Pat::Tuple(t) => {
                let (scrutinee, mode) = peel(ty, mode);
                let elems = match scrutinee.as_ref() {
                    Some(Ty::Tuple(elems)) if elems.len() == t.elems.len() => Some(elems.clone()),
                    _ => None,
                };
                for (i, elem) in t.elems.iter().enumerate() {
                    self.bind_pattern_into(elem, elems.as_ref().map(|e| &e[i]), mode, untyped);
                }
            }

            syn::Pat::TupleStruct(ts) => {
                let (scrutinee, mode) = peel(ty, mode);
                let fields = self.payload_of(&ts.path, scrutinee.as_ref());
                for (i, elem) in ts.elems.iter().enumerate() {
                    let field = fields
                        .as_ref()
                        .and_then(|f| f.iter().find(|(n, _)| *n == format!("_{}", i)))
                        .map(|(_, t)| t.clone());
                    self.bind_pattern_into(elem, field.as_ref(), mode, untyped);
                }
            }

            syn::Pat::Struct(s) => {
                let (scrutinee, mode) = peel(ty, mode);
                let fields = self.payload_of(&s.path, scrutinee.as_ref());
                for field in &s.fields {
                    let name = member_name(&field.member);
                    let found = fields
                        .as_ref()
                        .and_then(|f| f.iter().find(|(n, _)| *n == name))
                        .map(|(_, t)| t.clone());
                    self.bind_pattern_into(&field.pat, found.as_ref(), mode, untyped);
                }
            }

            syn::Pat::Slice(slice) => {
                let (scrutinee, mode) = peel(ty, mode);
                let elem = scrutinee.as_ref().and_then(|t| self.element_of(t));
                for pat in &slice.elems {
                    self.bind_pattern_into(pat, elem.as_ref(), mode, untyped);
                }
            }

            // A path, a literal, a range, `_` and `..` bind nothing.
            _ => {}
        }
    }

    /// What the variant or struct a pattern names carries, with the matched
    /// value's own type arguments substituted in.
    pub fn payload_of(&self, path: &syn::Path, ty: Option<&Ty>) -> Option<Vec<(String, Ty)>> {
        let ty = ty?.peel_refs();
        let Ty::Named { id, args } = ty else {
            return None;
        };
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let last = segments.last()?.clone();

        // `Some(x)`, `Ok(x)` and `Err(e)` destructure the two system types the
        // port writes as a nullable and as a Result; neither is declared as an
        // enum, so their payloads are named here.
        if self.is_system(*id, "std::option::Option") || self.is_system(*id, "std::result::Result") {
            let payload = match last.as_str() {
                "Some" | "Ok" => args.first().cloned(),
                "Err" => args.get(1).cloned(),
                _ => None,
            };
            return payload.map(|t| vec![("_0".to_string(), t)]);
        }

        let def = self.registry.def(*id)?;
        let subst = bind_params(&def.type_params, args);
        // A struct pattern reads the struct's own fields; a variant pattern
        // reads the payload of the variant it names.
        let fields = match self.registry.variant_fields(*id, &last) {
            Some(fields) => fields.to_vec(),
            None if def.name == last || matches!(def.kind, crate::registry::TypeKind::Struct) => {
                def.fields.clone()
            }
            None => return None,
        };
        Some(
            fields
                .into_iter()
                .map(|(n, t)| (n, t.substitute(&subst)))
                .collect(),
        )
    }

    /// What one turn of a `for` loop hands out: `<T as IntoIterator>::Item`.
    ///
    /// `for x in vec` hands out the element and `for x in &vec` a reference to
    /// it, because `IntoIterator for Vec<T>` and `IntoIterator for &Vec<T>` are
    /// two impls with two different `Item`s. Both are declared in the std
    /// surface, so this is a projection through the impl table and not a list of
    /// collections. A type with no `IntoIterator` impl in reach has no item
    /// type, and the loop variable is bound without one rather than guessed at.
    pub fn iteration_item(&self, ty: &Ty) -> Option<Ty> {
        self.project_through(ty, "std::iter::IntoIterator", "Item")
    }

    /// The type a projection `<ty as Trait>::name` normalises to, or nothing
    /// when no impl in the table supplies it.
    pub(crate) fn project_through(&self, ty: &Ty, trait_path: &str, name: &str) -> Option<Ty> {
        let trait_id = self.registry.system_type(trait_path)?;
        self.project_with(
            ty,
            TraitRef {
                id: trait_id,
                args: Vec::new(),
                bindings: Vec::new(),
            },
            name,
        )
    }

    pub(super) fn project_with(&self, ty: &Ty, trait_ref: TraitRef, name: &str) -> Option<Ty> {
        let projection = Ty::Assoc {
            base: Box::new(ty.clone()),
            trait_: Some(Box::new(trait_ref)),
            name: name.to_string(),
        };
        let normalized = self.probe().normalize(&projection);
        // A projection that did not normalise comes back as itself, which is the
        // truth about it and not an answer the translator can use.
        (normalized != projection).then_some(normalized)
    }

    /// The element type of a sequence, for a slice pattern. A slice pattern
    /// matches through a reference, so the element is what iterating a
    /// reference to the sequence hands out.
    fn element_of(&self, ty: &Ty) -> Option<Ty> {
        self.iteration_item(ty)
    }
}

/// Rust's default binding mode: what a name in a pattern binds by when the
/// pattern itself does not say (RFC 2005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Move,
    Ref,
    RefMut,
}

impl Mode {
    /// The type a name bound in this mode has.
    fn apply(self, ty: &Ty) -> Ty {
        match self {
            Mode::Move => ty.clone(),
            Mode::Ref => Ty::Ref {
                mutable: false,
                inner: Box::new(ty.clone()),
            },
            Mode::RefMut => Ty::Ref {
                mutable: true,
                inner: Box::new(ty.clone()),
            },
        }
    }
}

fn mode_of(mutable: bool) -> Mode {
    if mutable {
        Mode::RefMut
    } else {
        Mode::Ref
    }
}

/// Take one reference layer off the value a non-reference pattern is matched
/// against, and remember that everything underneath now binds through it. A
/// `&mut` inside a `&` still only reaches as far as the outer one allows.
fn peel(ty: Option<&Ty>, mode: Mode) -> (Option<Ty>, Mode) {
    let mut current = match ty {
        Some(ty) => ty.clone(),
        None => return (None, mode),
    };
    let mut mode = mode;
    while let Ty::Ref { mutable, inner } = current {
        mode = match (mode, mutable) {
            (Mode::Ref, _) | (_, false) => Mode::Ref,
            _ => Mode::RefMut,
        };
        current = *inner;
    }
    (Some(current), mode)
}
