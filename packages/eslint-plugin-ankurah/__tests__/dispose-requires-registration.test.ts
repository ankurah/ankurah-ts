// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/dispose-requires-registration';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('dispose-requires-registration', rule, {
  valid: [
    // Class with drop() that extends Drop
    {
      code: `
        class MyService extends Drop {
          drop() {
            super.drop();
          }
        }
      `,
    },
    // Class with drop() that has DropGuard
    {
      code: `
        class MyService {
          guard = new DropGuard(this, 'MyService');
          drop() {
            this.guard.markDropped(this);
          }
        }
      `,
    },
    // Class with DropGuard type annotation
    {
      code: `
        class MyService {
          guard: DropGuard;
          drop() {
            this.guard.markDropped(this);
          }
        }
      `,
    },
    // Class without drop() — no check needed
    {
      code: `
        class RegularService {
          doWork() {
            return 42;
          }
        }
      `,
    },
  ],
  invalid: [
    // Class with drop() but no Drop/DropGuard
    {
      code: `
        class MyService {
          drop() {
            this.cleanup();
          }
        }
      `,
      errors: [{ messageId: 'noRegistration' }],
    },
    // Ad-hoc drop without any registration
    {
      code: `
        class LeakyConnection {
          drop() {
            this.socket.close();
          }
        }
      `,
      errors: [{ messageId: 'noRegistration' }],
    },
  ],
});
