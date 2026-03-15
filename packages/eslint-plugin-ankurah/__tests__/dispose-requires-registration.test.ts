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
    // Class with dispose() that extends Disposable
    {
      code: `
        class MyService extends Disposable {
          dispose() {
            super.dispose();
          }
        }
      `,
    },
    // Class with dispose() that has DisposeGuard
    {
      code: `
        class MyService {
          guard = new DisposeGuard(this, 'MyService');
          dispose() {
            this.guard.markDisposed(this);
          }
        }
      `,
    },
    // Class with DisposeGuard type annotation
    {
      code: `
        class MyService {
          guard: DisposeGuard;
          dispose() {
            this.guard.markDisposed(this);
          }
        }
      `,
    },
    // Class without dispose() — no check needed
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
    // Class with dispose() but no Disposable/DisposeGuard
    {
      code: `
        class MyService {
          dispose() {
            this.cleanup();
          }
        }
      `,
      errors: [{ messageId: 'noRegistration' }],
    },
    // Ad-hoc dispose without any registration
    {
      code: `
        class LeakyConnection {
          dispose() {
            this.socket.close();
          }
        }
      `,
      errors: [{ messageId: 'noRegistration' }],
    },
  ],
});
