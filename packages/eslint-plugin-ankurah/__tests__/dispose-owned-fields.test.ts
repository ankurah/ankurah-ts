// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/dispose-owned-fields';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('dispose-owned-fields', rule, {
  valid: [
    // Disposable field properly disposed in onDispose
    {
      code: `
        class MyService extends Disposable {
          sub: Subscription;
          protected onDispose() {
            this.sub.dispose();
          }
        }
      `,
    },
    // Private field with # properly disposed
    {
      code: `
        class MyService extends Disposable {
          #watcher: Watcher;
          protected onDispose() {
            this.#watcher.dispose();
          }
        }
      `,
    },
    // No Disposable fields — no onDispose needed
    {
      code: `
        class MyService extends Disposable {
          name: string;
          count: number;
          protected onDispose() {}
        }
      `,
    },
    // Non-Disposable class — no check
    {
      code: `
        class RegularService {
          sub: Subscription;
          cleanup() {
            this.sub.dispose();
          }
        }
      `,
    },
    // Multiple fields all disposed
    {
      code: `
        class MyService extends Disposable {
          sub: Subscription;
          liveQuery: LiveQuery;
          protected onDispose() {
            this.sub.dispose();
            this.liveQuery.dispose();
          }
        }
      `,
    },
  ],
  invalid: [
    // Disposable field not disposed in onDispose
    {
      code: `
        class MyService extends Disposable {
          sub: Subscription;
          protected onDispose() {
            // forgot to dispose sub
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
    // Missing onDispose entirely
    {
      code: `
        class MyService extends Disposable {
          sub: Subscription;
        }
      `,
      errors: [{ messageId: 'missingOnDispose' }],
    },
    // One of two fields not disposed
    {
      code: `
        class MyService extends Disposable {
          sub: Subscription;
          watcher: Watcher;
          protected onDispose() {
            this.sub.dispose();
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
    // Field created with new DisposeGuard not disposed
    {
      code: `
        class MyService extends Disposable {
          guard = new DisposeGuard(this, 'MyService');
          protected onDispose() {
            // missing guard.dispose()
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
  ],
});
