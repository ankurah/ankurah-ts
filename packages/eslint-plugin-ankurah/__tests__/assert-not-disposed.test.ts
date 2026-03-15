// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/assert-not-disposed';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('assert-not-disposed', rule, {
  valid: [
    // Public method with assertNotDisposed as first statement
    {
      code: `
        class MyService extends Disposable {
          doWork() {
            this.assertNotDisposed();
            return 42;
          }
        }
      `,
    },
    // Private method — not required
    {
      code: `
        class MyService extends Disposable {
          private internalWork() {
            return 42;
          }
        }
      `,
    },
    // Protected method — not required
    {
      code: `
        class MyService extends Disposable {
          protected internalWork() {
            return 42;
          }
        }
      `,
    },
    // Private name (#) method — not required
    {
      code: `
        class MyService extends Disposable {
          #internalWork() {
            return 42;
          }
        }
      `,
    },
    // dispose() is excluded
    {
      code: `
        class MyService extends Disposable {
          dispose() {
            super.dispose();
          }
        }
      `,
    },
    // onDispose() is excluded
    {
      code: `
        class MyService extends Disposable {
          protected onDispose() {
            this.cleanup();
          }
        }
      `,
    },
    // isDisposed getter is excluded
    {
      code: `
        class MyService extends Disposable {
          get isDisposed() {
            return this.#disposed;
          }
        }
      `,
    },
    // Non-Disposable class — no check needed
    {
      code: `
        class RegularService {
          doWork() {
            return 42;
          }
        }
      `,
    },
    // DisposeGuard-based class with guard.assertNotDisposed()
    {
      code: `
        class MyService {
          guard: DisposeGuard;
          doWork() {
            this.guard.assertNotDisposed();
            return 42;
          }
        }
      `,
    },
    // Constructor is excluded
    {
      code: `
        class MyService extends Disposable {
          constructor() {
            super('MyService');
          }
        }
      `,
    },
  ],
  invalid: [
    // Public method without assertNotDisposed
    {
      code: `
        class MyService extends Disposable {
          doWork() {
            return 42;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // Empty body method
    {
      code: `
        class MyService extends Disposable {
          doWork() {}
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // assertNotDisposed is not the first statement
    {
      code: `
        class MyService extends Disposable {
          doWork() {
            const x = 1;
            this.assertNotDisposed();
            return x;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // Multiple public methods, some missing
    {
      code: `
        class MyService extends Disposable {
          goodMethod() {
            this.assertNotDisposed();
            return 1;
          }
          badMethod() {
            return 2;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // DisposeGuard class without guard check
    {
      code: `
        class MyService {
          guard = new DisposeGuard(this, 'MyService');
          doWork() {
            return 42;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // Getter without assertNotDisposed (getters are public API)
    {
      code: `
        class MyService extends Disposable {
          get value() {
            return this.#value;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
  ],
});
