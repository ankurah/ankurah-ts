//! What the end of the solve says: the three reports that outlive the quiet
//! walk.
//!
//! The constraint walk runs several times over one body and its sink is
//! rewound when it ends, so a report made while it ran would be made again and
//! taken back again. These three are what the solve KNOWS rather than what it
//! could not do: a constraint that still has no solution at the fixed point, an
//! answer the solve read through a bound it has since decided, and a value
//! handed over inside a holder. Each is re-checked against the table the solve
//! left and filed from there, past the rewind.

use super::context::TypeContext;
use crate::ty::{Mismatch, Ty};

impl TypeContext<'_> {
    /// Re-check every constraint the solve could not meet, poison what each one
    /// touched and say it once.
    ///
    /// A constraint that failed in an early round may be met by a later one, so
    /// only what still has no solution at the fixed point is a fact about the
    /// body; what it touched then stands for nothing, and the local that held
    /// it takes the untyped path.
    pub(super) fn settle_contradictions(&self) {
        let mut said = std::collections::HashSet::new();
        let collected = self.vars.borrow_mut().take_contradictions();
        for found in collected {
            let want = self.solved(&found.want);
            let actual = self.solved(&found.found);
            if self.vars.borrow().clone().unify(&want, &actual).is_ok() {
                continue;
            }
            // One failure poisons what it stood on, and every later constraint
            // reaching the same unknown fails for that one reason: saying it
            // again would report the first contradiction in another position.
            if self.vars.borrow().any_poisoned(&found.touched) {
                continue;
            }
            if !matches!(found.mismatch, Mismatch::Kind { .. })
                && !self.disagrees(&self.probe(), &want, &actual, found.site)
            {
                continue;
            }
            // A type still carrying an unknown is one the engine did not
            // finish reading, and what it could not read is no evidence: the
            // unknown is reported where it was bound.
            if !matches!(found.mismatch, Mismatch::Kind { .. })
                && (want.mentions_any_var() || actual.mentions_any_var())
            {
                continue;
            }
            let message = self.mismatch_message(&want, &actual, &found.mismatch);
            if !said.insert((crate::body::span_position(found.span), message.clone())) {
                continue;
            }
            self.vars.borrow_mut().poison_ids(found.touched);
            self.sink.report(found.span, message);
        }
    }

    /// Take back every answer the solve read through a bound it has since
    /// decided to be false.
    ///
    /// A bound the solve never settled is not a false one, so an answer
    /// standing on an unknown that nothing decided is left where it is: only a
    /// subject the impl table has read and found no impl for withdraws its
    /// answer, and with it every unknown that took it. An answer left standing
    /// says so itself while it still names the unknown that decides the bound;
    /// where the type handed out names no unknown at all, this is the only
    /// place a reader could learn what it rests on.
    pub(super) fn settle_obligations(&self) {
        let held = self.vars.borrow().provisional_answers();
        let probe = self.probe();
        for (var, answer) in held {
            let mut unsettled: Option<(Ty, crate::ty::TraitRef)> = None;
            let mut withdrawn = false;
            for (subject, bound) in &answer.stood_on {
                let subject = self.solved(subject);
                if subject.mentions_any_var() {
                    unsettled.get_or_insert((subject, bound.clone()));
                    continue;
                }
                if !probe.rules_out(&subject, bound) {
                    continue;
                }
                self.vars.borrow_mut().withdraw(var);
                self.sink.report(
                    answer.span,
                    format!(
                        "`{}` was read through an impl that requires `{}: {}`, which does not \
                         hold; every type taken from it is left unknown",
                        self.registry.describe(&self.solved(&answer.read)),
                        self.registry.describe(&subject),
                        self.registry.describe_traits(std::slice::from_ref(bound)),
                    ),
                );
                withdrawn = true;
                break;
            }
            let Some((subject, bound)) = unsettled.filter(|_| !withdrawn) else { continue };
            if self.solved(&Ty::Var(var)).mentions_any_var() {
                continue;
            }
            self.sink.report(
                answer.span,
                format!(
                    "`{}` was read through an impl that requires `{}: {}`, which nothing in this \
                     body settles; the type handed out carries no unknown, so nothing else says \
                     it rests on a bound nobody proved",
                    self.registry.describe(&self.solved(&answer.read)),
                    self.registry.describe(&subject),
                    self.registry.describe_traits(std::slice::from_ref(&bound)),
                ),
            );
        }
    }

    /// Say, once the solve is over, which coercion sites hand their value over
    /// inside a holder rather than writing the accessor the payload is behind.
    pub(super) fn settle_holder_coercions(&self) {
        for (span, holder, want) in self.vars.borrow_mut().take_inside_a_holder() {
            self.sink.report(
                span,
                format!(
                    "`{}` stands where `{}` is declared, and the port writes its payload behind \
                     an accessor this position does not; the callee is handed the holder",
                    self.registry.describe(&holder),
                    self.registry.describe(&want),
                ),
            );
        }
    }
}
