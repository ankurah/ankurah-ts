// RETIRED 2026-09-02 — not registered by the plugin and never runs.
//
// Rule: ankurah/require-using-for-guards
//
// This rule demanded `using` declarations. Hermes refuses to run `using` at all
// (facebook/hermes lib/Sema/SemanticResolver.cpp raises "using declarations are
// not yet supported", pinned by test/Parser/using-declaration-error.js), and
// Expo Go runs Hermes, so `using` is not the ownership model any more. The
// transpiler emits explicit `.drop()` calls: a block-owned value is dropped in a
// `finally`, a guard temporary at the end of its statement and again in the
// enclosing `finally`. Every one of this rule's nine findings in packages/core
// and packages/signals asked for code the target runtime rejects.
//
// Nothing survives a rewrite: whether the emitter placed the try/finally
// correctly is a property of generated code that the emitter itself has to
// guarantee, and the runtime reports a value nobody dropped through the leak
// registry. See port/ownership.md and port/retractions-2026-09-02.md.
//
// The file is kept only so its removal is a staged deletion Daniel reads rather
// than one that disappears inside another diff.
//
// Original description follows.
//
// Rust equivalent: Guard values are always dropped at scope exit.
//
// Methods returning a Drop guard type must be called with `using`,
// not bare `const`/`let`. Without `using`, drop() may never fire.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'require-using-for-guards';

// Method names that return Drop guards and must use `using`.
// This list should be maintained as the codebase grows.
const GUARD_FACTORY_METHODS = new Set([
  'write',      // ResultSet.write() -> ResultSetWrite (Drop guard)
  'subscribe',  // node.subscribe() -> Subscription (Drop)
]);

// Type names that are Drop guards (short-lived, must be dropped)
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
        'Methods returning Drop guard types must use `using` declarations. ' +
        'This replaces Rust automatic guard Drop at scope exit.',
    },
    messages: {
      requireUsing:
        'Call to "{{methodName}}" returns a Drop guard and must use `using` instead of `{{declarationKind}}`. ' +
        'Without `using`, drop() (and its side effects like broadcasts) may never fire, causing silent data staleness. ' +
        '(Rust equivalent: guard values are automatically Dropped at scope exit.)',
      requireUsingForNew:
        '`new {{className}}(...)` creates a Drop type and should use `using` instead of `{{declarationKind}}`. ' +
        'Without `using`, drop() may never fire. ' +
        '(Rust equivalent: RAII values are automatically Dropped at scope exit.)',
    },
    schema: [
      {
        type: 'object',
        properties: {
          guardFactoryMethods: {
            type: 'array',
            items: { type: 'string' },
            description: 'Additional method names that return Drop guards',
          },
          guardTypeNames: {
            type: 'array',
            items: { type: 'string' },
            description: 'Additional type names that are Drop guards',
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
