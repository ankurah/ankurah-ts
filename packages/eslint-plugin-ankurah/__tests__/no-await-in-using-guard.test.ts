// Tests for a rule RETIRED on 2026-09-02. The plugin no longer registers it, so
// these cases exercise a rule that never runs against the repository. Kept with
// the rule file so that deleting both is one staged decision; the rule file's
// header says why it was retired.
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/no-await-in-using-guard';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('no-await-in-using-guard', rule, {
  valid: [
    // No using — await is fine
    {
      code: `
        async function work() {
          const result = getData();
          await fetchMore();
        }
      `,
    },
    // await BEFORE the using declaration is fine
    {
      code: `
        async function work() {
          await setup();
          {
            using guard = getGuard();
            guard.doSync();
          }
        }
      `,
    },
    // using without any await
    {
      code: `
        function work() {
          {
            using guard = getGuard();
            guard.doSync();
          }
        }
      `,
    },
    // await in a nested function (different execution context)
    {
      code: `
        function work() {
          {
            using guard = getGuard();
            const fn = async () => {
              await fetchData();
            };
          }
        }
      `,
    },
  ],
  invalid: [
    // await inside using block
    {
      code: `
        async function work() {
          {
            using guard = getGuard();
            await fetchData();
          }
        }
      `,
      errors: [{ messageId: 'awaitInsideUsingGuard' }],
    },
    // Multiple awaits inside using block
    {
      code: `
        async function work() {
          {
            using guard = getGuard();
            await step1();
            await step2();
          }
        }
      `,
      errors: [
        { messageId: 'awaitInsideUsingGuard' },
        { messageId: 'awaitInsideUsingGuard' },
      ],
    },
  ],
});
