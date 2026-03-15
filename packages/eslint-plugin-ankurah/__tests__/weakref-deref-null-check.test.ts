// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/weakref-deref-null-check';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('weakref-deref-null-check', rule, {
  valid: [
    // Optional chaining on deref result
    {
      code: `const value = weakRef.deref()?.doSomething();`,
    },
    // Deref with null check before use
    {
      code: `
        const obj = weakRef.deref();
        if (obj !== undefined) {
          obj.doSomething();
        }
      `,
    },
    // Deref result not used (just checking existence)
    {
      code: `
        const obj = weakRef.deref();
        if (obj) {
          console.log('alive');
        }
      `,
    },
    // Not a deref call
    {
      code: `const result = someObj.getValue().name;`,
    },
  ],
  invalid: [
    // Direct property access on deref without null check
    {
      code: `const value = weakRef.deref().name;`,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // Direct method call on deref without null check
    {
      code: `const value = weakRef.deref().doSomething();`,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
  ],
});
