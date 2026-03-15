// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/no-await-in-using-guard
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
