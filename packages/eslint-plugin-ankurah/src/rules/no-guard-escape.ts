// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/no-guard-escape
// Flag assignments of Disposable-typed values to variables declared in an
// outer scope from inside using blocks. This is the `using` escape hatch —
// the bug pattern where a guard reference leaks beyond its intended scope.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'no-guard-escape';

type ScopeContainer = TSESTree.BlockStatement | TSESTree.Program;

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Flag assignments of using-declared guard values to outer-scope variables. ' +
        'This catches the `using` escape hatch that causes use-after-dispose bugs.',
    },
    messages: {
      guardEscape:
        'Guard "{{guardName}}" declared with `using` is being assigned to outer-scope variable "{{targetName}}". ' +
        'This defeats the purpose of `using` — the guard will be disposed at block exit, but the outer ' +
        'variable will hold a reference to the disposed object. ' +
        '(Rust equivalent: lifetimes prevent references from outliving their referent.)',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    // Collect all `using` variable names with their declaring block
    const usingVars = new Map<string, ScopeContainer>();

    return {
      VariableDeclaration(node) {
        if (node.kind !== 'using') return;

        const container = findContainer(node);
        if (!container) return;

        for (const declarator of node.declarations) {
          if (declarator.id.type === AST_NODE_TYPES.Identifier) {
            usingVars.set(declarator.id.name, container);
          }
        }
      },

      AssignmentExpression(node) {
        // Check if the RHS is a using-declared variable
        if (node.right.type !== AST_NODE_TYPES.Identifier) return;
        const guardName = node.right.name;
        const guardContainer = usingVars.get(guardName);
        if (!guardContainer) return;

        // Check if the LHS is a variable
        if (node.left.type !== AST_NODE_TYPES.Identifier) return;
        const targetName = node.left.name;

        // Find where the target variable is declared
        const targetContainer = findDeclaringContainer(node, targetName, context);
        if (!targetContainer) return;

        // If the target is declared in a different (outer) container from the guard
        if (targetContainer !== guardContainer && isAncestor(targetContainer, guardContainer)) {
          context.report({
            node,
            messageId: 'guardEscape',
            data: { guardName, targetName },
          });
        }
      },
    };
  },
});

function findContainer(node: TSESTree.Node): ScopeContainer | null {
  let current: TSESTree.Node | undefined = node.parent;
  while (current) {
    if (
      current.type === AST_NODE_TYPES.BlockStatement ||
      current.type === AST_NODE_TYPES.Program
    ) {
      return current;
    }
    current = current.parent;
  }
  return null;
}

function findDeclaringContainer(
  node: TSESTree.Node,
  varName: string,
  context: any,
): ScopeContainer | null {
  const sourceCode = context.sourceCode ?? context.getSourceCode();
  const scope = sourceCode.getScope(node);

  let current = scope;
  while (current) {
    for (const variable of current.variables) {
      if (variable.name === varName) {
        for (const def of variable.defs) {
          return findContainer(def.node);
        }
      }
    }
    current = current.upper;
  }
  return null;
}

function isAncestor(ancestor: TSESTree.Node, descendant: TSESTree.Node): boolean {
  let current: TSESTree.Node | undefined = descendant.parent;
  while (current) {
    if (current === ancestor) return true;
    current = current.parent;
  }
  return false;
}
