// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/no-guard-escape
//
// A lock or borrow guard belongs to the block that took it. That block drops it
// on the way out, so a copy of the guard kept in an outer variable points at a
// dropped guard the moment the block ends, and the next use of it is a fatal
// use-after-drop. Rust cannot express this bug at all: the guard's lifetime is
// tied to the borrow, and the borrow checker refuses the assignment. This rule
// is the closest static stand-in.
//
// Rewritten 2026-09-02. It used to look for `using` declarations, which Hermes
// refuses to run; the transpiler emits `const guard = x.lock()` plus a
// `finally` that drops it, so the rule now recognises guards by the call that
// produced them.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'no-guard-escape';

type ScopeContainer = TSESTree.BlockStatement | TSESTree.Program;

// Methods that hand back a guard: the container keeps the value, the guard is
// the block's temporary permission to touch it. From @ankurah/base — Mutex,
// RwLock, RefCell and AsyncMutex.
const GUARD_FACTORY_METHODS = new Set([
  'lock',
  'borrow',
  'borrowMut',
  'read',
  'write',
  'acquire',
]);

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'A guard must not be assigned to a variable declared outside the block that drops it. ' +
        'The outer variable would hold a dropped guard, and using it is fatal.',
    },
    messages: {
      guardEscape:
        'Guard "{{guardName}}" is assigned to "{{targetName}}", which is declared outside the block ' +
        'that holds the guard. The block drops the guard on its way out, so "{{targetName}}" is left ' +
        'pointing at a dropped guard and the next use of it is a fatal use-after-drop. ' +
        'Do the work inside the block, or read the value out of the guard and pass that. ' +
        '(Rust equivalent: the borrow checker refuses to let a guard outlive its borrow.)',
    },
    schema: [
      {
        type: 'object',
        properties: {
          guardFactoryMethods: {
            type: 'array',
            items: { type: 'string' },
            description: 'Additional method names that return a guard',
          },
        },
        additionalProperties: false,
      },
    ],
  },
  defaultOptions: [{}] as [{ guardFactoryMethods?: string[] }],
  create(context, [options]) {
    const extraMethods = new Set(options.guardFactoryMethods ?? []);

    // Guard variable name -> the block it was taken in.
    const guardVars = new Map<string, ScopeContainer>();

    function isGuardFactoryCall(expr: TSESTree.Node): boolean {
      let call = expr;
      if (call.type === AST_NODE_TYPES.AwaitExpression) call = call.argument;
      if (call.type !== AST_NODE_TYPES.CallExpression) return false;
      const callee = call.callee;
      if (callee.type !== AST_NODE_TYPES.MemberExpression) return false;
      const prop = callee.property;
      if (prop.type !== AST_NODE_TYPES.Identifier) return false;
      return GUARD_FACTORY_METHODS.has(prop.name) || extraMethods.has(prop.name);
    }

    return {
      VariableDeclaration(node) {
        const container = findContainer(node);
        if (!container) return;

        for (const declarator of node.declarations) {
          if (declarator.id.type !== AST_NODE_TYPES.Identifier) continue;
          if (!declarator.init || !isGuardFactoryCall(declarator.init)) continue;
          guardVars.set(declarator.id.name, container);
        }
      },

      AssignmentExpression(node) {
        // Check if the RHS is a guard variable
        if (node.right.type !== AST_NODE_TYPES.Identifier) return;
        const guardName = node.right.name;
        const guardContainer = guardVars.get(guardName);
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
