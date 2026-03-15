// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/require-using-for-guards';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('require-using-for-guards', rule, {
  valid: [
    // using declaration with guard factory method
    {
      code: `using rw = resultset.write();`,
    },
    // using declaration with subscribe
    {
      code: `using sub = node.subscribe(query);`,
    },
    // Regular const with non-guard method
    {
      code: `const result = obj.getData();`,
    },
    // Bare call (no assignment) — not flagged
    {
      code: `resultset.write();`,
    },
    // const with non-guard new expression
    {
      code: `const service = new RegularService();`,
    },
  ],
  invalid: [
    // const instead of using for guard factory method
    {
      code: `const rw = resultset.write();`,
      errors: [{ messageId: 'requireUsing' }],
    },
    // let instead of using for guard factory method
    {
      code: `let rw = resultset.write();`,
      errors: [{ messageId: 'requireUsing' }],
    },
    // const with subscribe (guard factory)
    {
      code: `const sub = node.subscribe(query);`,
      errors: [{ messageId: 'requireUsing' }],
    },
    // const with new guard type
    {
      code: `const sub = new Subscription();`,
      errors: [{ messageId: 'requireUsingForNew' }],
    },
    // const with new LiveQuery
    {
      code: `const lq = new LiveQuery();`,
      errors: [{ messageId: 'requireUsingForNew' }],
    },
  ],
});
