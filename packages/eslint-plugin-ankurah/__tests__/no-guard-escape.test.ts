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
    // Using guard only within its block
    {
      code: `
        {
          using guard = getGuard();
          guard.doWork();
        }
      `,
    },
    // Assignment to a variable in the same block (not outer)
    {
      code: `
        {
          using guard = getGuard();
          const result = guard.getValue();
        }
      `,
    },
    // No using declaration
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
    // Guard reference assigned to outer-scope variable
    {
      code: `
        let leaked;
        {
          using guard = getGuard();
          leaked = guard;
        }
      `,
      errors: [{ messageId: 'guardEscape' }],
    },
  ],
});
