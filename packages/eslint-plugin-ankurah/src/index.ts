// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Custom lint rules that replace Rust's compile-time ownership guarantees
// (lifetimes, Drop, borrow checker) with static analysis at dev time.

import { rule as assertNotDisposed } from './rules/assert-not-disposed';
import { rule as disposeOwnedFields } from './rules/dispose-owned-fields';
import { rule as requireUsingForGuards } from './rules/require-using-for-guards';
import { rule as noUnhandledFireAndForget } from './rules/no-unhandled-fire-and-forget';
import { rule as weakrefDerefNullCheck } from './rules/weakref-deref-null-check';
import { rule as disposeRequiresRegistration } from './rules/dispose-requires-registration';
import { rule as noAwaitInUsingGuard } from './rules/no-await-in-using-guard';
import { rule as noGuardEscape } from './rules/no-guard-escape';

const rules = {
  'assert-not-disposed': assertNotDisposed,
  'dispose-owned-fields': disposeOwnedFields,
  'require-using-for-guards': requireUsingForGuards,
  'no-unhandled-fire-and-forget': noUnhandledFireAndForget,
  'weakref-deref-null-check': weakrefDerefNullCheck,
  'dispose-requires-registration': disposeRequiresRegistration,
  'no-await-in-using-guard': noAwaitInUsingGuard,
  'no-guard-escape': noGuardEscape,
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
        'ankurah/assert-not-disposed': 'error' as const,
        'ankurah/dispose-owned-fields': 'error' as const,
        'ankurah/require-using-for-guards': 'error' as const,
        // Tier 2 — Should Have (warn)
        'ankurah/no-unhandled-fire-and-forget': 'warn' as const,
        'ankurah/weakref-deref-null-check': 'error' as const,
        // Tier 3 — Nice to Have (warn)
        'ankurah/dispose-requires-registration': 'warn' as const,
        'ankurah/no-await-in-using-guard': 'warn' as const,
        'ankurah/no-guard-escape': 'error' as const,
      },
    },
  },
};

export default plugin;
export { rules };
