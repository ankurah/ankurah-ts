// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/dispose-requires-registration
// Any class with a drop() method must extend Drop or use DropGuard.
// Ad-hoc drop() without FR registration means leaked instances produce zero diagnostics.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'dispose-requires-registration';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Classes with drop() must extend Drop or use DropGuard for FinalizationRegistry leak detection.',
    },
    messages: {
      noRegistration:
        'Class "{{className}}" has a drop() method but does not extend Drop or use DropGuard. ' +
        'Without FinalizationRegistry registration, leaked instances produce zero diagnostics. ' +
        'Either extend Drop or add a DropGuard field.',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    return {
      ClassDeclaration(node) {
        checkClass(context, node);
      },
      ClassExpression(node) {
        checkClass(context, node);
      },
    };
  },
});

function extendsDrop(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Drop';
  }
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    return prop.type === AST_NODE_TYPES.Identifier && prop.name === 'Drop';
  }
  return false;
}

function hasDropGuardField(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some((member) => {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) return false;
    const typeAnnotation = member.typeAnnotation?.typeAnnotation;
    if (
      typeAnnotation?.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier &&
      typeAnnotation.typeName.name === 'DropGuard'
    ) {
      return true;
    }
    const value = member.value;
    if (
      value?.type === AST_NODE_TYPES.NewExpression &&
      value.callee.type === AST_NODE_TYPES.Identifier &&
      value.callee.name === 'DropGuard'
    ) {
      return true;
    }
    return false;
  });
}

function hasDropMethod(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some(
    (member) =>
      member.type === AST_NODE_TYPES.MethodDefinition &&
      member.key.type === AST_NODE_TYPES.Identifier &&
      member.key.name === 'drop',
  );
}

function getClassName(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): string {
  return node.id?.name ?? '(anonymous)';
}

function checkClass(
  context: any,
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
) {
  if (!hasDropMethod(node)) return;
  if (extendsDrop(node)) return;
  if (hasDropGuardField(node)) return;

  // The Drop class itself defines drop() — don't flag it
  const className = getClassName(node);
  if (className === 'Drop' || className === 'DropGuard') return;

  context.report({
    node,
    messageId: 'noRegistration',
    data: { className },
  });
}
