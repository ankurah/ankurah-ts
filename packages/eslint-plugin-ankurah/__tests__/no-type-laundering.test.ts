// TS-ONLY: ESLint plugin enforcing port fidelity
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/no-type-laundering';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('no-type-laundering', rule, {
  valid: [
    // as unknown as with Divergence comment on same line
    {
      code: `const x = foo as unknown as Bar; // Divergence: reason here`,
    },
    // as unknown as with Divergence comment on preceding line
    {
      code: `
        // Divergence: reason here
        const x = foo as unknown as Bar;
      `,
    },
    // Single `as` cast (not type laundering)
    {
      code: `const x = foo as Bar;`,
    },
    // as unknown (no second cast)
    {
      code: `const x = foo as unknown;`,
    },
    // as X as Y (not through unknown)
    {
      code: `const x = (foo as any) as Bar;`,
    },
  ],
  invalid: [
    // as unknown as with no comment at all
    {
      code: `const x = foo as unknown as Bar;`,
      errors: [{ messageId: 'missingDivergenceComment' }],
    },
    // as unknown as with a comment that doesn't say Divergence:
    {
      code: `const x = foo as unknown as Bar; // this is fine`,
      errors: [{ messageId: 'missingDivergenceComment' }],
    },
    // as unknown as with Divergence comment too far above
    {
      code: `
        // Divergence: reason here

        const y = 1;
        const x = foo as unknown as Bar;
      `,
      errors: [{ messageId: 'missingDivergenceComment' }],
    },
  ],
});
