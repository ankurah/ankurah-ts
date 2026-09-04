// RETIRED 2026-09-02 — not registered by the plugin and never runs.
//
// Rule: ankurah/no-await-in-using-guard
//
// This rule only fired on `using` declarations, and `using` is retired: Hermes
// refuses to run it, so the transpiler emits explicit `.drop()` calls and
// try/finally blocks instead. With no `using` in the tree the rule reports
// nothing, whatever the code does.
//
// The invariant underneath it — do not hold a lock guard across an await — is
// already enforced twice over, and neither place is this one. Rust's
// `MutexGuard` is `!Send`, so rustc rejects holding one across an await in the
// source we transpile; and where ankurah does need to hold a lock across an
// await it uses `tokio::sync::Mutex`, which maps to AsyncMutex and is built for
// exactly that. See port/ownership.md and port/retractions-2026-09-02.md.
//
// The file is kept only so its removal is a staged deletion Daniel reads rather
// than one that disappears inside another diff.
//
// Original description follows.
//
// Flag await expressions inside using blocks where the guard is
// correctness-critical. An await means other code can interleave while
// the guard is active.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'no-await-in-using-guard';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'suggestion',
    docs: {
      description:
        'Flag await inside using blocks where the guard has side effects on dispose. ' +
        'An await allows other code to interleave while the guard is active.',
    },
    messages: {
      awaitInsideUsingGuard:
        'Avoid `await` inside a `using` block with guard "{{guardName}}". ' +
        'An await yields execution, allowing other code to interleave while the guard is held. ' +
        'If the guard has side effects on dispose (e.g., broadcasts), this can cause subtle ordering bugs.',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    return {
      AwaitExpression(node) {
        // Walk up to find the nearest enclosing block, stopping at function boundaries
        let current: TSESTree.Node | undefined = node.parent;
        while (current) {
          // Stop at function boundaries — the await is in a nested function scope
          if (
            current.type === AST_NODE_TYPES.FunctionDeclaration ||
            current.type === AST_NODE_TYPES.FunctionExpression ||
            current.type === AST_NODE_TYPES.ArrowFunctionExpression
          ) {
            // Check if this function itself is inside a block with using
            current = current.parent;
            continue;
          }

          if (current.type === AST_NODE_TYPES.BlockStatement) {
            // Check if this block contains any `using` declarations that precede the await
            const usingGuards = findUsingDeclarationsBefore(current, node);
            if (usingGuards.length > 0) {
              // Check that the await isn't inside a nested function within the block
              if (!isInsideNestedFunction(node, current)) {
                context.report({
                  node,
                  messageId: 'awaitInsideUsingGuard',
                  data: { guardName: usingGuards[0] },
                });
                return;
              }
            }
          }

          current = current.parent;
        }
      },
    };
  },
});

function findUsingDeclarationsBefore(
  block: TSESTree.BlockStatement,
  awaitNode: TSESTree.Node,
): string[] {
  const guards: string[] = [];
  const awaitStart = awaitNode.range?.[0] ?? 0;

  for (const stmt of block.body) {
    if (
      stmt.type === AST_NODE_TYPES.VariableDeclaration &&
      stmt.kind === 'using'
    ) {
      const stmtEnd = stmt.range?.[1] ?? 0;
      if (stmtEnd <= awaitStart) {
        for (const decl of stmt.declarations) {
          if (decl.id.type === AST_NODE_TYPES.Identifier) {
            guards.push(decl.id.name);
          }
        }
      }
    }
  }

  return guards;
}

function isInsideNestedFunction(
  node: TSESTree.Node,
  stopAt: TSESTree.Node,
): boolean {
  let current: TSESTree.Node | undefined = node.parent;
  while (current && current !== stopAt) {
    if (
      current.type === AST_NODE_TYPES.FunctionDeclaration ||
      current.type === AST_NODE_TYPES.FunctionExpression ||
      current.type === AST_NODE_TYPES.ArrowFunctionExpression
    ) {
      return true;
    }
    current = current.parent;
  }
  return false;
}
