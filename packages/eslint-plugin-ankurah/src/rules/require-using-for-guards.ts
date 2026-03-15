// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/require-using-for-guards
// Rust equivalent: Guard values are always dropped at scope exit.
//
// Methods returning a Disposable guard type must be called with `using`,
// not bare `const`/`let`. Without `using`, dispose() may never fire.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'require-using-for-guards';

// Method names that return Disposable guards and must use `using`.
// This list should be maintained as the codebase grows.
const GUARD_FACTORY_METHODS = new Set([
  'write',      // ResultSet.write() -> ResultSetWrite (Disposable guard)
  'subscribe',  // node.subscribe() -> Subscription (Disposable)
]);

// Type names that are Disposable guards (short-lived, must be disposed)
const GUARD_TYPE_NAMES = new Set([
  'ResultSetWrite',
  'Subscription',
  'ReactorSubscription',
  'EntityLiveQuery',
  'LiveQuery',
]);

// Class names known to produce guards from their methods
const GUARD_FACTORY_SUFFIXES = [
  'Write',
  'Guard',
];

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Methods returning Disposable guard types must use `using` declarations. ' +
        'This replaces Rust automatic guard Drop at scope exit.',
    },
    messages: {
      requireUsing:
        'Call to "{{methodName}}" returns a Disposable guard and must use `using` instead of `{{declarationKind}}`. ' +
        'Without `using`, dispose() (and its side effects like broadcasts) may never fire, causing silent data staleness. ' +
        '(Rust equivalent: guard values are automatically Dropped at scope exit.)',
      requireUsingForNew:
        '`new {{className}}(...)` creates a Disposable and should use `using` instead of `{{declarationKind}}`. ' +
        'Without `using`, dispose() may never fire. ' +
        '(Rust equivalent: RAII values are automatically Dropped at scope exit.)',
    },
    schema: [
      {
        type: 'object',
        properties: {
          guardFactoryMethods: {
            type: 'array',
            items: { type: 'string' },
            description: 'Additional method names that return Disposable guards',
          },
          guardTypeNames: {
            type: 'array',
            items: { type: 'string' },
            description: 'Additional type names that are Disposable guards',
          },
        },
        additionalProperties: false,
      },
    ],
  },
  defaultOptions: [{}] as [{ guardFactoryMethods?: string[]; guardTypeNames?: string[] }],
  create(context, [options]) {
    const extraMethods = new Set(options.guardFactoryMethods ?? []);
    const extraTypes = new Set(options.guardTypeNames ?? []);

    function isGuardFactoryCall(expr: TSESTree.Expression): { methodName: string } | null {
      if (expr.type !== AST_NODE_TYPES.CallExpression) return null;
      const callee = expr.callee;

      // obj.method() pattern
      if (callee.type === AST_NODE_TYPES.MemberExpression) {
        const prop = callee.property;
        if (prop.type === AST_NODE_TYPES.Identifier) {
          if (GUARD_FACTORY_METHODS.has(prop.name) || extraMethods.has(prop.name)) {
            return { methodName: prop.name };
          }
        }
      }

      return null;
    }

    function isGuardConstructor(expr: TSESTree.Expression): { className: string } | null {
      if (
        expr.type === AST_NODE_TYPES.NewExpression &&
        expr.callee.type === AST_NODE_TYPES.Identifier
      ) {
        const name = expr.callee.name;
        if (
          GUARD_TYPE_NAMES.has(name) ||
          extraTypes.has(name) ||
          GUARD_FACTORY_SUFFIXES.some((s) => name.endsWith(s))
        ) {
          return { className: name };
        }
      }
      return null;
    }

    return {
      VariableDeclaration(node) {
        // `using` declarations have kind === 'using' or 'await using' in the AST
        // We only flag const/let/var declarations
        if (node.kind === 'using' || node.kind === ('await using' as string)) return;

        for (const declarator of node.declarations) {
          if (!declarator.init) continue;

          const guardCall = isGuardFactoryCall(declarator.init);
          if (guardCall) {
            context.report({
              node: declarator,
              messageId: 'requireUsing',
              data: {
                methodName: guardCall.methodName,
                declarationKind: node.kind,
              },
            });
            continue;
          }

          const guardNew = isGuardConstructor(declarator.init);
          if (guardNew) {
            context.report({
              node: declarator,
              messageId: 'requireUsingForNew',
              data: {
                className: guardNew.className,
                declarationKind: node.kind,
              },
            });
          }
        }
      },
    };
  },
});
