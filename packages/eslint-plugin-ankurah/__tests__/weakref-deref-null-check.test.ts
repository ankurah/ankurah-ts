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
    // Q2: a `deref` the file has not shown to be a `WeakRef`'s is left alone.
    // The port writes Rust's `Deref` impl under that name — `impl Deref for
    // AttestationSet` is `.deref()` here — and reading through one has no
    // `None` to handle. Matched by the method's name alone this was one error
    // on every clean run of `npx eslint packages/proto/src`.
    {
      code: `const count = this.attestations.deref().length;`,
    },
    {
      code: `
        const set = holder.deref();
        return set.length;
      `,
    },
  ],
  invalid: [
    // Direct property access on deref without null check
    {
      code: `
        const weakRef = new WeakRef(target);
        const value = weakRef.deref().name;
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // Direct method call on deref without null check
    {
      code: `
        const weakRef: WeakRef<Target> = held;
        const value = weakRef.deref().doSomething();
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // And on the construction itself
    {
      code: `const value = new WeakRef(target).deref().name;`,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
  ],
});
