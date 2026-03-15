// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/dispose-requires-registration
// Any class with a dispose() method must extend Disposable or use DisposeGuard.
// Ad-hoc dispose() without FR registration means leaked instances produce zero diagnostics.

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
        'Classes with dispose() must extend Disposable or use DisposeGuard for FinalizationRegistry leak detection.',
    },
    messages: {
      noRegistration:
        'Class "{{className}}" has a dispose() method but does not extend Disposable or use DisposeGuard. ' +
        'Without FinalizationRegistry registration, leaked instances produce zero diagnostics. ' +
        'Either extend Disposable or add a DisposeGuard field.',
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

function extendsDisposable(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Disposable';
  }
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    return prop.type === AST_NODE_TYPES.Identifier && prop.name === 'Disposable';
  }
  return false;
}

function hasDisposeGuardField(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some((member) => {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) return false;
    const typeAnnotation = member.typeAnnotation?.typeAnnotation;
    if (
      typeAnnotation?.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier &&
      typeAnnotation.typeName.name === 'DisposeGuard'
    ) {
      return true;
    }
    const value = member.value;
    if (
      value?.type === AST_NODE_TYPES.NewExpression &&
      value.callee.type === AST_NODE_TYPES.Identifier &&
      value.callee.name === 'DisposeGuard'
    ) {
      return true;
    }
    return false;
  });
}

function hasDisposeMethod(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some(
    (member) =>
      member.type === AST_NODE_TYPES.MethodDefinition &&
      member.key.type === AST_NODE_TYPES.Identifier &&
      member.key.name === 'dispose',
  );
}

function getClassName(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): string {
  return node.id?.name ?? '(anonymous)';
}

function checkClass(
  context: any,
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
) {
  if (!hasDisposeMethod(node)) return;
  if (extendsDisposable(node)) return;
  if (hasDisposeGuardField(node)) return;

  // The Disposable class itself defines dispose() — don't flag it
  const className = getClassName(node);
  if (className === 'Disposable' || className === 'DisposeGuard') return;

  context.report({
    node,
    messageId: 'noRegistration',
    data: { className },
  });
}
