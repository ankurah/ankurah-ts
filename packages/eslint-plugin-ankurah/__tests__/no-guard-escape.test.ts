// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/no-guard-escape';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('no-guard-escape', rule, {
  valid: [
    // Guard used only within the block that drops it
    {
      code: `
        {
          const guard = state.lock();
          try {
            guard.value.push(entry);
          } finally {
            guard.drop();
          }
        }
      `,
    },
    // Reading a value out of the guard and keeping that is fine — the value is
    // the container's, the guard is the block's.
    {
      code: `
        let count;
        {
          const guard = state.lock();
          try {
            count = guard.value.length;
          } finally {
            guard.drop();
          }
        }
      `,
    },
    // Assignment to a variable declared in the same block
    {
      code: `
        {
          const guard = cell.borrowMut();
          let alias = guard;
          alias.value = 1;
        }
      `,
    },
    // A plain value, not a guard
    {
      code: `
        let outer;
        {
          const inner = getValue();
          outer = inner;
        }
      `,
    },
  ],
  invalid: [
    // Lock guard assigned to an outer-scope variable
    {
      code: `
        let leaked;
        {
          const guard = state.lock();
          leaked = guard;
        }
      `,
      errors: [{ messageId: 'guardEscape' }],
    },
    // RefCell borrow guard assigned out
    {
      code: `
        let leaked;
        {
          const guard = cell.borrow();
          leaked = guard;
        }
      `,
      errors: [{ messageId: 'guardEscape' }],
    },
    // An awaited AsyncMutex guard escapes the same way
    {
      code: `
        async function f() {
          let leaked;
          {
            const guard = await mutex.acquire();
            leaked = guard;
          }
        }
      `,
      errors: [{ messageId: 'guardEscape' }],
    },
  ],
});
