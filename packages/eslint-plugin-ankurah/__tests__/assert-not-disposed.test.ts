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
    // Public method with assertNotDropped as first statement
    {
      code: `
        class MyService extends Drop {
          doWork() {
            this.assertNotDropped();
            return 42;
          }
        }
      `,
    },
    // Private method — not required
    {
      code: `
        class MyService extends Drop {
          private internalWork() {
            return 42;
          }
        }
      `,
    },
    // Protected method — not required
    {
      code: `
        class MyService extends Drop {
          protected internalWork() {
            return 42;
          }
        }
      `,
    },
    // Private name (#) method — not required
    {
      code: `
        class MyService extends Drop {
          #internalWork() {
            return 42;
          }
        }
      `,
    },
    // drop() is excluded
    {
      code: `
        class MyService extends Drop {
          drop() {
            super.drop();
          }
        }
      `,
    },
    // onDrop() is excluded
    {
      code: `
        class MyService extends Drop {
          protected onDrop() {
            this.cleanup();
          }
        }
      `,
    },
    // isDropped getter is excluded
    {
      code: `
        class MyService extends Drop {
          get isDropped() {
            return this.#dropped;
          }
        }
      `,
    },
    // Non-Drop class — no check needed
    {
      code: `
        class RegularService {
          doWork() {
            return 42;
          }
        }
      `,
    },
    // DropGuard-based class with guard.assertNotDropped()
    {
      code: `
        class MyService {
          guard: DropGuard;
          doWork() {
            this.guard.assertNotDropped();
            return 42;
          }
        }
      `,
    },
    // Constructor is excluded
    {
      code: `
        class MyService extends Drop {
          constructor() {
            super('MyService');
          }
        }
      `,
    },
  ],
  invalid: [
    // Public method without assertNotDropped
    {
      code: `
        class MyService extends Drop {
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
        class MyService extends Drop {
          doWork() {}
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // assertNotDropped is not the first statement
    {
      code: `
        class MyService extends Drop {
          doWork() {
            const x = 1;
            this.assertNotDropped();
            return x;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // Multiple public methods, some missing
    {
      code: `
        class MyService extends Drop {
          goodMethod() {
            this.assertNotDropped();
            return 1;
          }
          badMethod() {
            return 2;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // DropGuard class without guard check
    {
      code: `
        class MyService {
          guard = new DropGuard(this, 'MyService');
          doWork() {
            return 42;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
    // Getter without assertNotDropped (getters are public API)
    {
      code: `
        class MyService extends Drop {
          get value() {
            return this.#value;
          }
        }
      `,
      errors: [{ messageId: 'missingAssertNotDisposed' }],
    },
  ],
});
