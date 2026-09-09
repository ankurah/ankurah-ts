//! The unknowns of one function body, and the solver that binds them.
//!
//! A local written `let mut entities = Vec::new();` has no type until something
//! below it says so. The table mints a variable for that unknown, constraints
//! collected over the whole body are unified against each other through the
//! table, and what stays unbound at the end is reported rather than guessed.

use std::collections::{HashMap, HashSet};

use super::def::{InferId, TraitRef, Ty};
use super::unify::{unify_with, Mismatch, Unknowns};

/// What a variable is allowed to stand for.
///
/// An unsuffixed literal is Rust's `{integer}` or `{float}`: an unknown that
/// only an integer, or only a float, may bind, and that takes `i32` or `f64`
/// once the solve is over and nothing stronger has bound it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Integral,
    Float,
}

impl VarKind {
    /// May a variable of this kind stand for that type?
    pub fn admits(self, ty: &Ty) -> bool {
        match (self, ty) {
            (VarKind::Integral, Ty::Prim(prim)) => prim.is_integer(),
            (VarKind::Float, Ty::Prim(prim)) => matches!(prim, super::def::Prim::F32 | super::def::Prim::F64),
            _ => false,
        }
    }

    /// What a literal of this kind is where nothing said otherwise, which is
    /// Rust's own fallback.
    pub fn fallback(self) -> Ty {
        match self {
            VarKind::Integral => Ty::Prim(super::def::Prim::I32),
            VarKind::Float => Ty::Prim(super::def::Prim::F64),
        }
    }
}

/// A constraint the solver could not meet, held until the solve is over.
///
/// For: the walk that collects constraints is quiet, so a message filed while
/// it runs is thrown away with the rest of its diagnostics. A contradiction is
/// a fact about the table rather than a message, so the table keeps it and the
/// site that asked re-files it once the solve is done.
#[derive(Clone, Debug)]
pub struct Contradiction {
    pub span: proc_macro2::Span,
    pub want: Ty,
    pub found: Ty,
    pub mismatch: Mismatch,
    /// The unknowns the unification walked through before it failed. They are
    /// what the constraint stood on, and what it poisons once the solve is
    /// over: a method's parameter is the receiver's own type argument
    /// substituted in, so `xs.push("x")` on a `Vec<?0>` stands on `?0` even
    /// though neither side still spells it.
    pub touched: Vec<InferId>,
    /// Would Rust coerce here? It does where a value MEETS a declared type and
    /// nowhere inside one, and the re-check at the end of the solve has to ask
    /// the same question the site asked.
    pub coerces: bool,
}

/// Which unknown at a site a refusal takes. One site can also carry an
/// omitted type argument and a literal, and this index is clear of both.
const REFUSAL: usize = usize::MAX;

/// Which unknown at a site a method call takes while which method it lands on
/// still rests on a bound the solve has not settled. Clear of the others.
pub const UNDECIDED_RESULT: usize = usize::MAX - 1;

/// Every inference variable of one body, and what each stands for.
///
/// A binding is made once and never revised, which is what makes the occurs
/// check enough to keep the table acyclic and the solve terminating.
#[derive(Clone, Debug, Default)]
pub struct InferTable {
    bound: Vec<Option<Ty>>,
    /// The variable standing at each written site. Two passes over one body ask
    /// about the same site, and the second must get the unknown the first
    /// bound rather than a new one.
    sites: HashMap<(usize, usize, usize), InferId>,
    /// Bindings in the order they were made, so a constraint that cannot be met
    /// leaves the table as it found it.
    journal: Vec<InferId>,
    /// Is the constraint walk running? Only it binds: the walk that WRITES the
    /// body reads a table already solved, so every ownership question it asks
    /// gets the answer the statements above it got.
    solving: bool,
    /// The written default standing behind an argument the source left off, and
    /// where it was left off. Rust prefers what the body says to what the
    /// declaration defaults to, so these are unified LAST.
    defaults: Vec<(InferId, Ty, proc_macro2::Span)>,
    /// Variables a constraint with no solution touched. What such a variable
    /// stands for is a type the engine cannot defend, so it answers as an
    /// unknown and the binding that happened to come first stops answering.
    poisoned: HashSet<InferId>,
    /// Constraints with no solution, in the order they were met.
    contradictions: Vec<Contradiction>,
    /// What each restricted variable may stand for. Only an unsuffixed literal
    /// mints one, and only it is ever defaulted.
    kinds: HashMap<InferId, VarKind>,
    /// The restricted variables the fallback decided, rather than the body. A
    /// width the engine fell back to is not a fact about the program, so a
    /// constraint it fails is the engine's own guess and says nothing.
    defaulted: HashSet<InferId>,
    /// How many bindings a constraint asked of a SCRATCH copy of this table
    /// would have made. The walk that writes a body asks against a copy, so
    /// counting this table's own bindings cannot see a late answer; this can.
    scratch_bindings: usize,
    /// The unknowns the last unification walked through, in the order it met
    /// them. A failed constraint stands on exactly these.
    touched: Vec<InferId>,
}

impl InferTable {
    pub fn new() -> InferTable {
        InferTable::default()
    }

    /// Remember the default an omitted argument falls back to. The same site
    /// asked twice records it once.
    pub fn defer_default(&mut self, var: InferId, default: Ty, span: proc_macro2::Span) {
        if !self.defaults.iter().any(|(id, ..)| *id == var) {
            self.defaults.push((var, default, span));
        }
    }

    /// Those defaults, in the order they were met.
    pub fn deferred_defaults(&self) -> Vec<(InferId, Ty, proc_macro2::Span)> {
        self.defaults.clone()
    }

    /// Say whether the constraint walk is running.
    pub fn set_solving(&mut self, running: bool) {
        self.solving = running;
    }

    /// Is it?
    pub fn solving(&self) -> bool {
        self.solving
    }

    /// A new unknown, standing for nothing yet.
    pub fn fresh(&mut self) -> InferId {
        self.bound.push(None);
        InferId(self.bound.len() as u32 - 1)
    }

    /// The unknown a refusal answers with: minted at the refusing site and
    /// already standing for nothing.
    ///
    /// A refusal is the engine's own gap, so it must contribute no type: what
    /// it answers meets anything without binding, and every spelling path
    /// reads it as unknown. Answering `()` instead made a later constraint
    /// report the gap as the program's contradiction.
    pub fn unresolvable_at_site(&mut self, line: usize, col: usize) -> InferId {
        let id = self.at_site(line, col, REFUSAL);
        self.poisoned.insert(id);
        id
    }

    /// The unknown standing at one written site — a source position and, where
    /// the site carries several, which one. Minted the first time and the same
    /// one after.
    pub fn at_site(&mut self, line: usize, col: usize, index: usize) -> InferId {
        match self.sites.get(&(line, col, index)) {
            Some(id) => *id,
            None => {
                let id = self.fresh();
                self.sites.insert((line, col, index), id);
                id
            }
        }
    }

    /// How many variables this body minted. The solve's round bound is read
    /// off it: no run may outlast one round per variable.
    pub fn len(&self) -> usize {
        self.bound.len()
    }

    /// How many variables the solve has decided — bound, or poisoned and so
    /// decided to be undecidable. A round that leaves this unchanged has
    /// reached its fixed point, which is why poison counts: it only ever grows.
    pub fn bound_count(&self) -> usize {
        (0..self.bound.len())
            .filter(|i| self.bound[*i].is_some() || self.poisoned.contains(&InferId(*i as u32)))
            .count()
    }

    /// Note that a constraint asked of a scratch copy bound something the
    /// solve had not. Nothing is bound here: the count is what the walk that
    /// writes a body is held to.
    pub fn note_scratch_bindings(&mut self, made: usize) {
        self.scratch_bindings += made;
    }

    /// How many such bindings have been noted.
    pub fn scratch_bindings(&self) -> usize {
        self.scratch_bindings
    }

    /// How many variables stand for a type. Minting an unknown that stands for
    /// nothing is not a binding, so this is what the walk that writes a body is
    /// held to.
    pub fn bindings_made(&self) -> usize {
        self.bound.iter().filter(|b| b.is_some()).count()
    }

    /// What this variable stands for, one step, without following further. A
    /// poisoned variable stands for nothing: the constraint that touched it had
    /// no solution, and the type it held before is not an answer.
    pub fn binding(&self, var: InferId) -> Option<&Ty> {
        if self.poisoned.contains(&var) {
            return None;
        }
        self.bound.get(var.0 as usize).and_then(|b| b.as_ref())
    }

    /// The unknowns the last unification walked through. Read by the caller of
    /// a constraint that failed: they are what it stood on, rather than
    /// anything recoverable from the types after the solve replaced them.
    pub fn touched(&self) -> Vec<InferId> {
        self.touched.clone()
    }

    /// Mark these unknowns as standing for nothing.
    pub fn poison_ids(&mut self, ids: impl IntoIterator<Item = InferId>) {
        self.poisoned.extend(ids);
    }

    /// Does any of these stand for nothing already?
    pub fn any_poisoned(&self, ids: &[InferId]) -> bool {
        ids.iter().any(|id| self.poisoned.contains(id))
    }

    /// Does this type name an unknown that stands for nothing?
    fn mentions_poisoned(&self, ty: &Ty) -> bool {
        let mut found = Vec::new();
        collect_vars(ty, &mut found);
        found.iter().any(|id| self.poisoned.contains(id))
    }

    /// What a variable is restricted to, where it is one an unsuffixed literal
    /// minted.
    pub fn kind_of(&self, id: InferId) -> Option<VarKind> {
        self.kinds.get(&id).copied()
    }

    /// Does this type stand on a width the fallback chose rather than the body?
    pub fn mentions_defaulted(&self, ty: &Ty) -> bool {
        let mut found = Vec::new();
        collect_vars(ty, &mut found);
        found.iter().any(|id| self.defaulted.contains(id))
    }

    /// The variable standing at one site, restricted to what a kind admits.
    pub fn kinded_at_site(
        &mut self,
        line: usize,
        col: usize,
        index: usize,
        kind: VarKind,
    ) -> InferId {
        let id = self.at_site(line, col, index);
        self.kinds.entry(id).or_insert(kind);
        id
    }

    /// The variable standing at one site, if anything has minted one.
    pub fn site(&self, line: usize, col: usize, index: usize) -> Option<InferId> {
        self.sites.get(&(line, col, index)).copied()
    }

    /// Bind every restricted variable nothing else bound to what its kind falls
    /// back to. Rust defaults `{integer}` to `i32` and `{float}` to `f64` once
    /// its own inference is over, and this is that step and the only default.
    pub fn settle_kinds(&mut self) {
        let open: Vec<(InferId, VarKind)> = self
            .kinds
            .iter()
            .filter(|(id, _)| self.binding(**id).is_none() && !self.poisoned.contains(id))
            .map(|(id, kind)| (*id, *kind))
            .collect();
        for (id, kind) in open {
            if self.assign(id, &kind.fallback()).is_ok() {
                self.defaulted.insert(id);
            }
        }
    }

    /// Keep a constraint that had no solution, to be re-checked and reported
    /// once the solve is done.
    pub fn record_contradiction(&mut self, found: Contradiction) {
        self.contradictions.push(found);
    }

    /// Take the constraints that had no solution, leaving none behind.
    pub fn take_contradictions(&mut self) -> Vec<Contradiction> {
        std::mem::take(&mut self.contradictions)
    }

    /// The type with every bound variable replaced by what it stands for, all
    /// the way down. Variables nothing bound are left in place for the caller
    /// to report.
    pub fn resolve(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(id) => match self.binding(*id) {
                Some(bound) => self.resolve(bound),
                None => ty.clone(),
            },
            Ty::Named { id, args } => Ty::Named {
                id: *id,
                args: args.iter().map(|a| self.resolve(a)).collect(),
            },
            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| self.resolve(e)).collect()),
            Ty::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.resolve(inner)),
            },
            Ty::Slice(inner) => Ty::Slice(Box::new(self.resolve(inner))),
            Ty::Array { elem, len } => Ty::Array {
                elem: Box::new(self.resolve(elem)),
                len: len.clone(),
            },
            Ty::Dyn { traits } => Ty::Dyn {
                traits: traits.iter().map(|t| self.resolve_trait(t)).collect(),
            },
            Ty::ImplTrait { bounds } => Ty::ImplTrait {
                bounds: bounds.iter().map(|t| self.resolve_trait(t)).collect(),
            },
            Ty::Assoc { base, trait_, name } => Ty::Assoc {
                base: Box::new(self.resolve(base)),
                trait_: trait_.as_ref().map(|t| Box::new(self.resolve_trait(t))),
                name: name.clone(),
            },
            Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => ty.clone(),
        }
    }

    fn resolve_trait(&self, trait_: &TraitRef) -> TraitRef {
        TraitRef {
            id: trait_.id,
            args: trait_.args.iter().map(|a| self.resolve(a)).collect(),
            bindings: trait_
                .bindings
                .iter()
                .map(|(n, t)| (n.clone(), self.resolve(t)))
                .collect(),
        }
    }

    /// Constrain two types to be the same, binding variables on either side.
    ///
    /// A shape the walk cannot reconcile is refused and binds nothing: a
    /// constraint with no solution must not leave behind the part of itself
    /// that did fit. The caller reports it at the site the constraint came
    /// from, naming both types.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), Mismatch> {
        self.touched.clear();
        let made = self.journal.len();
        let answer = unify_with(&mut BodyVars { table: self }, a, b);
        if answer.is_err() {
            for var in self.journal.split_off(made) {
                self.bound[var.0 as usize] = None;
            }
        }
        answer
    }
}

/// The table as a unification walk sees it: an unknown on either side, bound
/// into the table itself.
struct BodyVars<'a> {
    table: &'a mut InferTable,
}

impl Unknowns for BodyVars<'_> {
    fn follow(&mut self, ty: &Ty) -> Option<Ty> {
        let Ty::Var(id) = ty else { return None };
        // Every unknown the walk reaches, bound or not. A constraint with no
        // solution stands on exactly these, and they are what it poisons.
        self.table.touched.push(*id);
        self.table.binding(*id).cloned()
    }

    fn bind(&mut self, a: &Ty, b: &Ty) -> Option<Result<(), Mismatch>> {
        // Both sides arrive already followed, so a variable here stands for
        // nothing yet and binding it cannot overwrite an answer.
        match (a, b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => Some(Ok(())),
            (Ty::Var(x), other) => Some(self.table.assign(*x, other)),
            (other, Ty::Var(y)) => Some(self.table.assign(*y, other)),
            // A bound and a type that meets it are not equal and not a
            // mismatch: `Arc<dyn TNode<CD>>` accepts an `Arc<MockNode>` by
            // coercion. The walk stops here rather than refusing, and what the
            // bound projects is constrained where the argument is read.
            (Ty::Dyn { .. } | Ty::ImplTrait { .. }, other)
            | (other, Ty::Dyn { .. } | Ty::ImplTrait { .. })
                if !matches!(other, Ty::Dyn { .. } | Ty::ImplTrait { .. }) =>
            {
                Some(Ok(()))
            }
            _ => None,
        }
    }
}

impl InferTable {
    fn assign(&mut self, var: InferId, ty: &Ty) -> Result<(), Mismatch> {
        // A poisoned variable meets anything and holds nothing: one constraint
        // with no solution is the report, and every constraint after it would
        // say the same failure again in another position.
        if self.poisoned.contains(&var) {
            return Ok(());
        }
        // Through the table: `a = b` then `b = Vec<a>` mentions `a` only once
        // `b` is followed, and binding it would make `resolve` recurse forever.
        let ty = &self.resolve(ty);
        // Neither does anything bind TO one: a type naming an unknown that
        // stands for nothing is not an answer, and binding it would spread
        // that gap to every variable the constraint reaches.
        if self.mentions_poisoned(ty) {
            return Ok(());
        }
        if ty.mentions_var(var) {
            return Err(Mismatch::VarOccurs {
                var,
                ty: ty.clone(),
            });
        }
        // A restricted variable stands only for what its kind admits, and a
        // variable it meets takes the same restriction: `let mut n = 0; n =
        // xs.len()` is a `usize`, and `n = ()` has no solution.
        if let Some(kind) = self.kinds.get(&var).copied() {
            match ty {
                // Two restricted variables that alias take one restriction,
                // and `{integer}` against `{float}` has no solution: `let mut x
                // = 1; x = 2.0` was silent and its arithmetic went unchecked.
                Ty::Var(other) => match self.kinds.get(other).copied() {
                    Some(theirs) if theirs != kind => {
                        return Err(Mismatch::Kind {
                            var,
                            ty: ty.clone(),
                        })
                    }
                    Some(_) => {}
                    None => {
                        self.kinds.insert(*other, kind);
                    }
                },
                _ if !kind.admits(ty) => {
                    return Err(Mismatch::Kind {
                        var,
                        ty: ty.clone(),
                    })
                }
                _ => {}
            }
        }
        self.bound[var.0 as usize] = Some(ty.clone());
        self.journal.push(var);
        Ok(())
    }
}

/// Every inference variable written anywhere inside a type.
fn collect_vars(ty: &Ty, into: &mut Vec<InferId>) {
    match ty {
        Ty::Var(id) => into.push(*id),
        Ty::Named { args, .. } | Ty::Tuple(args) => {
            args.iter().for_each(|a| collect_vars(a, into))
        }
        Ty::Ref { inner, .. } | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
            collect_vars(inner, into)
        }
        Ty::Dyn { traits } | Ty::ImplTrait { bounds: traits } => {
            for trait_ in traits {
                trait_.args.iter().for_each(|a| collect_vars(a, into));
                trait_.bindings.iter().for_each(|(_, t)| collect_vars(t, into));
            }
        }
        Ty::Assoc { base, trait_, .. } => {
            collect_vars(base, into);
            if let Some(trait_) = trait_ {
                trait_.args.iter().for_each(|a| collect_vars(a, into));
                trait_.bindings.iter().for_each(|(_, t)| collect_vars(t, into));
            }
        }
        Ty::Param(_) | Ty::Prim(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Infer => {}
    }
}
