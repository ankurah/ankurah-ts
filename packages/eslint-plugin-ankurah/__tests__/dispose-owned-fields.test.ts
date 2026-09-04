// Tests for a rule RETIRED on 2026-09-02. The plugin no longer registers it, so
// these cases exercise a rule that never runs against the repository. Kept with
// the rule file so that deleting both is one staged decision; the rule file's
// header says why it was retired.
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
    // Drop field properly dropped in onDrop
    {
      code: `
        class MyService extends Drop {
          sub: Subscription;
          protected onDrop() {
            this.sub.drop();
          }
        }
      `,
    },
    // Private field with # properly dropped
    {
      code: `
        class MyService extends Drop {
          #watcher: Watcher;
          protected onDrop() {
            this.#watcher.drop();
          }
        }
      `,
    },
    // No Drop fields — no onDrop needed
    {
      code: `
        class MyService extends Drop {
          name: string;
          count: number;
          protected onDrop() {}
        }
      `,
    },
    // Non-Drop class — no check
    {
      code: `
        class RegularService {
          sub: Subscription;
          cleanup() {
            this.sub.drop();
          }
        }
      `,
    },
    // Multiple fields all dropped
    {
      code: `
        class MyService extends Drop {
          sub: Subscription;
          liveQuery: LiveQuery;
          protected onDrop() {
            this.sub.drop();
            this.liveQuery.drop();
          }
        }
      `,
    },
  ],
  invalid: [
    // Drop field not dropped in onDrop
    {
      code: `
        class MyService extends Drop {
          sub: Subscription;
          protected onDrop() {
            // forgot to drop sub
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
    // Missing onDrop entirely
    {
      code: `
        class MyService extends Drop {
          sub: Subscription;
        }
      `,
      errors: [{ messageId: 'missingOnDispose' }],
    },
    // One of two fields not dropped
    {
      code: `
        class MyService extends Drop {
          sub: Subscription;
          watcher: Watcher;
          protected onDrop() {
            this.sub.drop();
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
    // Field created with new DropGuard not dropped
    {
      code: `
        class MyService extends Drop {
          guard = new DropGuard(this, 'MyService');
          protected onDrop() {
            // missing guard.drop()
          }
        }
      `,
      errors: [{ messageId: 'missingFieldDispose' }],
    },
  ],
});
