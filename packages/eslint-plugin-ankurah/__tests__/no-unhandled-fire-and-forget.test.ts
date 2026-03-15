// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/no-unhandled-fire-and-forget';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('no-unhandled-fire-and-forget', rule, {
  valid: [
    // Awaited async call
    {
      code: `await fetchData();`,
    },
    // Fire-and-forget with justification comment
    {
      code: `
        // fire-and-forget: background refresh, not critical
        fetchData();
      `,
    },
    // Regular synchronous call
    {
      code: `doWork();`,
    },
    // Non-async-looking method name
    {
      code: `calculate();`,
    },
  ],
  invalid: [
    // Un-awaited fetch call
    {
      code: `fetchData();`,
      errors: [{ messageId: 'unawaited' }],
    },
    // Un-awaited sync call
    {
      code: `syncToServer();`,
      errors: [{ messageId: 'unawaited' }],
    },
    // Un-awaited .then() chain
    {
      code: `promise.then(handler);`,
      errors: [{ messageId: 'unawaited' }],
    },
  ],
});
