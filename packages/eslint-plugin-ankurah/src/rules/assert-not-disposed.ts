// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/assert-not-dropped
// Rust equivalent: Lifetime enforcement — compiler prevents use-after-free.
//
// Every public method on a Drop subclass must call this.assertNotDropped()
// (or this.#guard.assertNotDropped() / this.guard.assertNotDropped()) as its
// first statement. Excludes drop(), [Symbol.dispose](), onDrop(), isDropped.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'assert-not-disposed';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Public methods on Drop subclasses must call this.assertNotDropped() as their first statement. ' +
        'This replaces Rust lifetime enforcement that prevents use-after-free.',
    },
    messages: {
      missingAssertNotDisposed:
        'Public method "{{methodName}}" on Drop subclass must call this.assertNotDropped() ' +
        '(or this.#guard.assertNotDropped()) as its first statement. ' +
        'Without this check, use-after-drop bugs can occur silently. ' +
        '(Rust equivalent: lifetime enforcement prevents use-after-free at compile time.)',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    // Track classes that extend Drop or have a DropGuard field
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

const EXCLUDED_METHODS = new Set([
  'drop',
  'onDrop',
  'isDropped',
  // Symbol.dispose is handled separately via computed property check
]);

function isDropSubclass(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  // Check for `extends Drop`
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Drop';
  }
  // Check for `extends SomeModule.Drop`
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    if (prop.type === AST_NODE_TYPES.Identifier) {
      return prop.name === 'Drop';
    }
  }
  return false;
}

function hasDropGuardField(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some((member) => {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) return false;
    // Check type annotation for DropGuard
    const typeAnnotation = member.typeAnnotation?.typeAnnotation;
    if (
      typeAnnotation?.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier &&
      typeAnnotation.typeName.name === 'DropGuard'
    ) {
      return true;
    }
    // Check value for `new DropGuard(...)`
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

function isPublicMethod(member: TSESTree.ClassElement): member is TSESTree.MethodDefinition {
  if (member.type !== AST_NODE_TYPES.MethodDefinition) return false;
  if (member.kind === 'get' || member.kind === 'set') return true; // getters/setters are public API
  if (member.kind === 'constructor') return false;
  if (member.accessibility === 'private' || member.accessibility === 'protected') return false;
  // Private names (#foo) are not public
  if (member.key.type === AST_NODE_TYPES.PrivateIdentifier) return false;
  return true;
}

function isExcludedMethod(member: TSESTree.MethodDefinition): boolean {
  // Named methods
  if (member.key.type === AST_NODE_TYPES.Identifier && EXCLUDED_METHODS.has(member.key.name)) {
    return true;
  }
  // [Symbol.dispose]()
  if (member.computed && member.key.type === AST_NODE_TYPES.MemberExpression) {
    const obj = member.key.object;
    const prop = member.key.property;
    if (
      obj.type === AST_NODE_TYPES.Identifier &&
      obj.name === 'Symbol' &&
      prop.type === AST_NODE_TYPES.Identifier &&
      prop.name === 'dispose'
    ) {
      return true;
    }
  }
  // [disposeSymbol]() — the polyfill pattern used in drop.ts
  if (member.computed && member.key.type === AST_NODE_TYPES.Identifier) {
    if (member.key.name === 'disposeSymbol' || member.key.name === 'Symbol.dispose') {
      return true;
    }
  }
  // isDropped getter
  if (
    member.kind === 'get' &&
    member.key.type === AST_NODE_TYPES.Identifier &&
    member.key.name === 'isDropped'
  ) {
    return true;
  }
  return false;
}

function getMethodName(member: TSESTree.MethodDefinition): string {
  if (member.key.type === AST_NODE_TYPES.Identifier) return member.key.name;
  if (member.key.type === AST_NODE_TYPES.Literal) return String(member.key.value);
  return '[computed]';
}

function isAssertNotDroppedCall(statement: TSESTree.Statement): boolean {
  if (statement.type !== AST_NODE_TYPES.ExpressionStatement) return false;
  const expr = statement.expression;
  if (expr.type !== AST_NODE_TYPES.CallExpression) return false;

  const callee = expr.callee;

  // this.assertNotDropped()
  if (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    callee.object.type === AST_NODE_TYPES.ThisExpression
  ) {
    const prop = callee.property;
    if (prop.type === AST_NODE_TYPES.Identifier && prop.name === 'assertNotDropped') {
      return true;
    }
    // this.#guard.assertNotDropped() — check for chained member expression
    if (
      prop.type === AST_NODE_TYPES.Identifier ||
      prop.type === AST_NODE_TYPES.PrivateIdentifier
    ) {
      // Actually need to check: this.#guard.assertNotDropped() or this.guard.assertNotDropped()
      // The callee would be MemberExpression(MemberExpression(this, #guard), assertNotDropped)
    }
  }

  // this.#guard.assertNotDropped() or this.guard.assertNotDropped()
  if (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    callee.property.type === AST_NODE_TYPES.Identifier &&
    callee.property.name === 'assertNotDropped' &&
    callee.object.type === AST_NODE_TYPES.MemberExpression &&
    callee.object.object.type === AST_NODE_TYPES.ThisExpression
  ) {
    return true;
  }

  return false;
}

function checkClass(
  context: any,
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
) {
  const isDrop = isDropSubclass(node) || hasDropGuardField(node);
  if (!isDrop) return;

  for (const member of node.body.body) {
    if (!isPublicMethod(member)) continue;
    if (isExcludedMethod(member)) continue;

    const body = member.value.body;
    if (!body || body.body.length === 0) {
      context.report({
        node: member,
        messageId: 'missingAssertNotDisposed',
        data: { methodName: getMethodName(member) },
      });
      continue;
    }

    const firstStatement = body.body[0];
    if (!isAssertNotDroppedCall(firstStatement)) {
      context.report({
        node: member,
        messageId: 'missingAssertNotDisposed',
        data: { methodName: getMethodName(member) },
      });
    }
  }
}
