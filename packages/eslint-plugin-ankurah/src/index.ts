// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Custom lint rules that replace Rust's compile-time ownership guarantees
// (lifetimes, Drop, borrow checker) with static analysis at dev time.
//
// Three rules were retired on 2026-09-02 and are deliberately not imported
// here: require-using-for-guards and no-await-in-using-guard enforced `using`
// declarations, which Hermes refuses to run, and dispose-owned-fields demanded
// by-hand field drops that the runtime's cascade now performs — doing both
// drops the field twice, which is fatal. Their files carry the reasoning and
// are waiting to be deleted. See port/retractions-2026-09-02.md.

import { rule as assertNotDropped } from './rules/assert-not-dropped';
import { rule as noUnhandledFireAndForget } from './rules/no-unhandled-fire-and-forget';
import { rule as weakrefDerefNullCheck } from './rules/weakref-deref-null-check';
import { rule as dropRequiresRegistration } from './rules/drop-requires-registration';
import { rule as noGuardEscape } from './rules/no-guard-escape';
import { rule as noTypeLaundering } from './rules/no-type-laundering';

const rules = {
  'assert-not-dropped': assertNotDropped,
  'no-unhandled-fire-and-forget': noUnhandledFireAndForget,
  'weakref-deref-null-check': weakrefDerefNullCheck,
  'drop-requires-registration': dropRequiresRegistration,
  'no-guard-escape': noGuardEscape,
  'no-type-laundering': noTypeLaundering,
};

const plugin = {
  rules,
  configs: {
    recommended: {
      plugins: {
        ankurah: { rules },
      },
      rules: {
        // Tier 1 — Must Have (error)
        'ankurah/assert-not-dropped': 'error' as const,
        // Tier 2 — Should Have (warn)
        'ankurah/no-unhandled-fire-and-forget': 'warn' as const,
        'ankurah/weakref-deref-null-check': 'error' as const,
        // Tier 3 — Nice to Have (warn)
        'ankurah/drop-requires-registration': 'warn' as const,
        'ankurah/no-guard-escape': 'error' as const,
        'ankurah/no-type-laundering': 'error' as const,
      },
    },
  },
};

export default plugin;
export { rules };
