// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/weakref-deref-null-check
// Rust equivalent: Weak::upgrade() returns Option<Arc<T>>, forcing None handling.
//
// Flag any .deref() call on a WeakRef whose result is used without a
// null/undefined check.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'weakref-deref-null-check';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'WeakRef.deref() results must be null-checked before use. ' +
        'This replaces Rust Weak::upgrade() returning Option<Arc<T>>.',
    },
    messages: {
      uncheckedDeref:
        'WeakRef.deref() result must be null-checked before use. ' +
        'If the strong reference holder has been garbage collected, deref() returns undefined. ' +
        '(Rust equivalent: Weak::upgrade() returns Option<Arc<T>>, forcing the caller to handle None.)',
      directPropertyAccess:
        'Direct property access on WeakRef.deref() without null check. ' +
        'Use optional chaining (.deref()?.prop) or assign to a variable and check for undefined first. ' +
        '(Rust equivalent: Weak::upgrade() returns Option<Arc<T>>.)',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    return {
      // Catch: weakRef.deref().something (direct property access without null check)
      MemberExpression(node) {
        if (isDerefCall(node.object) && !node.optional) {
          context.report({
            node,
            messageId: 'directPropertyAccess',
          });
        }
      },

      // Catch: variable = weakRef.deref(); variable.something (without null check)
      VariableDeclarator(node) {
        if (!node.init || !isDerefCall(node.init)) return;
        if (node.id.type !== AST_NODE_TYPES.Identifier) return;

        const varName = node.id.name;
        const scope = (context.sourceCode ?? context.getSourceCode()).getScope(node);

        // Check if the variable is used without a null check in the same scope
        const variable = scope.variables.find((v) => v.name === varName);
        if (!variable) return;

        for (const ref of variable.references) {
          if (ref.isWrite()) continue;
          const refNode = ref.identifier;

          // Check if the reference is inside a null check
          if (isInsideNullCheck(refNode, varName)) continue;

          // Check if used as property access without optional chaining
          const parent = refNode.parent;
          if (
            parent?.type === AST_NODE_TYPES.MemberExpression &&
            parent.object === refNode &&
            !parent.optional
          ) {
            // Check if there's a preceding null check in the same block
            if (!hasPrecedingNullCheck(refNode, varName)) {
              context.report({
                node: refNode,
                messageId: 'uncheckedDeref',
              });
            }
          }
        }
      },
    };
  },
});

function isDerefCall(node: TSESTree.Node): boolean {
  return (
    node.type === AST_NODE_TYPES.CallExpression &&
    node.callee.type === AST_NODE_TYPES.MemberExpression &&
    node.callee.property.type === AST_NODE_TYPES.Identifier &&
    node.callee.property.name === 'deref'
  );
}

function isInsideNullCheck(node: TSESTree.Node, varName: string): boolean {
  let current: TSESTree.Node | undefined = node.parent;
  while (current) {
    // if (var !== undefined) { ... } or if (var != null) { ... } or if (var) { ... }
    if (current.type === AST_NODE_TYPES.IfStatement) {
      if (isNullCheckCondition(current.test, varName)) {
        // Check we're in the consequent (truthy branch), not the alternate
        const consequent = current.consequent;
        if (isAncestor(consequent, node)) return true;
      }
    }
    current = current.parent;
  }
  return false;
}

function isNullCheckCondition(node: TSESTree.Node, varName: string): boolean {
  // var !== undefined, var !== null, var != null, var != undefined
  if (node.type === AST_NODE_TYPES.BinaryExpression) {
    if (node.operator === '!==' || node.operator === '!=') {
      return (
        (isIdentifier(node.left, varName) && isNullOrUndefined(node.right)) ||
        (isIdentifier(node.right, varName) && isNullOrUndefined(node.left))
      );
    }
  }
  // Simple truthiness check: if (var) { ... }
  if (isIdentifier(node, varName)) return true;

  return false;
}

function isIdentifier(node: TSESTree.Node, name: string): boolean {
  return node.type === AST_NODE_TYPES.Identifier && node.name === name;
}

function isNullOrUndefined(node: TSESTree.Node): boolean {
  if (node.type === AST_NODE_TYPES.Literal && node.value === null) return true;
  if (node.type === AST_NODE_TYPES.Identifier && node.name === 'undefined') return true;
  return false;
}

function isAncestor(ancestor: TSESTree.Node, descendant: TSESTree.Node): boolean {
  let current: TSESTree.Node | undefined = descendant;
  while (current) {
    if (current === ancestor) return true;
    current = current.parent;
  }
  return false;
}

function hasPrecedingNullCheck(refNode: TSESTree.Node, varName: string): boolean {
  // Walk up to find the containing block
  let block: TSESTree.BlockStatement | null = null;
  let current: TSESTree.Node | undefined = refNode.parent;
  while (current) {
    if (current.type === AST_NODE_TYPES.BlockStatement) {
      block = current;
      break;
    }
    current = current.parent;
  }
  if (!block) return false;

  // Check if there's an if-check for the variable before this reference
  const refLine = refNode.loc?.start.line ?? 0;
  for (const stmt of block.body) {
    if ((stmt.loc?.start.line ?? 0) >= refLine) break;
    if (
      stmt.type === AST_NODE_TYPES.IfStatement &&
      isNullCheckCondition(stmt.test, varName)
    ) {
      return true;
    }
  }
  return false;
}
