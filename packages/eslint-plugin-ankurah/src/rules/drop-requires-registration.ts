// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/drop-requires-registration
// A value that can be dropped can also be forgotten. The leak registry is what
// turns a forgotten value into a diagnostic, so a class that defines drop()
// without registering its instances leaks in silence.
//
// There are three ways a class registers, and the rule accepts all three:
// inheriting a registering base (AkObject and everything under it), holding a
// DropGuard, or calling leakRegistry.register() itself — which is what AkObject
// and Arc do, since they are the bottom of the hierarchy and have nothing to
// inherit from.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'drop-requires-registration';

// Base classes whose constructor registers the instance with the leak registry.
// Every one of these reaches AkObject's constructor, which registers.
const REGISTERING_BASES = new Set([
  'AkObject',
  'Struct',
  'Enum',
  'Drop',
  'Guard',
  'ReadGuard',
  'WriteGuard',
]);

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Classes with drop() must register their instances with the leak registry, so that an ' +
        'instance nobody dropped is reported instead of disappearing.',
    },
    messages: {
      noRegistration:
        'Class "{{className}}" has a drop() method but never registers its instances with the leak ' +
        'registry, so an instance nobody drops is collected without a word. ' +
        'Extend a registering base (AkObject, Struct, Enum, Drop, or a Guard), hold a DropGuard field, ' +
        'or call leakRegistry.register() in the constructor.',
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

function extendsRegisteringBase(
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
): boolean {
  if (!node.superClass) return false;
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return REGISTERING_BASES.has(node.superClass.name);
  }
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    return prop.type === AST_NODE_TYPES.Identifier && REGISTERING_BASES.has(prop.name);
  }
  return false;
}

// A class that registers by hand: somewhere in its body it calls
// leakRegistry.register(...). AkObject and Arc do this in their constructors.
function callsLeakRegistryRegister(
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
): boolean {
  let found = false;
  const seen = new Set<TSESTree.Node>();

  const walk = (current: TSESTree.Node): void => {
    if (found || seen.has(current)) return;
    seen.add(current);

    if (
      current.type === AST_NODE_TYPES.CallExpression &&
      current.callee.type === AST_NODE_TYPES.MemberExpression &&
      current.callee.property.type === AST_NODE_TYPES.Identifier &&
      current.callee.property.name === 'register' &&
      current.callee.object.type === AST_NODE_TYPES.Identifier &&
      current.callee.object.name === 'leakRegistry'
    ) {
      found = true;
      return;
    }

    for (const key of Object.keys(current)) {
      if (key === 'parent') continue;
      const child = (current as unknown as Record<string, unknown>)[key];
      if (!child || typeof child !== 'object') continue;
      if (Array.isArray(child)) {
        for (const item of child) {
          if (item && typeof item === 'object' && 'type' in item) walk(item as TSESTree.Node);
        }
      } else if ('type' in child) {
        walk(child as TSESTree.Node);
      }
    }
  };

  walk(node.body);
  return found;
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
  if (extendsRegisteringBase(node)) return;
  if (hasDropGuardField(node)) return;
  if (callsLeakRegistryRegister(node)) return;

  // The Drop class itself defines drop() — don't flag it
  const className = getClassName(node);
  if (className === 'Drop' || className === 'DropGuard') return;

  context.report({
    node,
    messageId: 'noRegistration',
    data: { className },
  });
}
