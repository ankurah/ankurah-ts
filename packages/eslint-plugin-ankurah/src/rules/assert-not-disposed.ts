// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/assert-not-disposed
// Rust equivalent: Lifetime enforcement — compiler prevents use-after-free.
//
// Every public method on a Disposable subclass must call this.assertNotDisposed()
// (or this.#guard.assertNotDisposed() / this.guard.assertNotDisposed()) as its
// first statement. Excludes dispose(), [Symbol.dispose](), onDispose(), isDisposed.

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
        'Public methods on Disposable subclasses must call this.assertNotDisposed() as their first statement. ' +
        'This replaces Rust lifetime enforcement that prevents use-after-free.',
    },
    messages: {
      missingAssertNotDisposed:
        'Public method "{{methodName}}" on Disposable subclass must call this.assertNotDisposed() ' +
        '(or this.#guard.assertNotDisposed()) as its first statement. ' +
        'Without this check, use-after-dispose bugs can occur silently. ' +
        '(Rust equivalent: lifetime enforcement prevents use-after-free at compile time.)',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    // Track classes that extend Disposable or have a DisposeGuard field
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
  'dispose',
  'onDispose',
  'isDisposed',
  // Symbol.dispose is handled separately via computed property check
]);

function isDisposableSubclass(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  // Check for `extends Disposable` or `extends SomethingDisposable`
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Disposable';
  }
  // Check for `extends SomeModule.Disposable`
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    if (prop.type === AST_NODE_TYPES.Identifier) {
      return prop.name === 'Disposable';
    }
  }
  return false;
}

function hasDisposeGuardField(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  return node.body.body.some((member) => {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) return false;
    // Check type annotation for DisposeGuard
    const typeAnnotation = member.typeAnnotation?.typeAnnotation;
    if (
      typeAnnotation?.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier &&
      typeAnnotation.typeName.name === 'DisposeGuard'
    ) {
      return true;
    }
    // Check value for `new DisposeGuard(...)`
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
  // [disposeSymbol]() — the polyfill pattern used in disposable.ts
  if (member.computed && member.key.type === AST_NODE_TYPES.Identifier) {
    if (member.key.name === 'disposeSymbol' || member.key.name === 'Symbol.dispose') {
      return true;
    }
  }
  // isDisposed getter
  if (
    member.kind === 'get' &&
    member.key.type === AST_NODE_TYPES.Identifier &&
    member.key.name === 'isDisposed'
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

function isAssertNotDisposedCall(statement: TSESTree.Statement): boolean {
  if (statement.type !== AST_NODE_TYPES.ExpressionStatement) return false;
  const expr = statement.expression;
  if (expr.type !== AST_NODE_TYPES.CallExpression) return false;

  const callee = expr.callee;

  // this.assertNotDisposed()
  if (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    callee.object.type === AST_NODE_TYPES.ThisExpression
  ) {
    const prop = callee.property;
    if (prop.type === AST_NODE_TYPES.Identifier && prop.name === 'assertNotDisposed') {
      return true;
    }
    // this.#guard.assertNotDisposed() — check for chained member expression
    if (
      prop.type === AST_NODE_TYPES.Identifier ||
      prop.type === AST_NODE_TYPES.PrivateIdentifier
    ) {
      // Actually need to check: this.#guard.assertNotDisposed() or this.guard.assertNotDisposed()
      // The callee would be MemberExpression(MemberExpression(this, #guard), assertNotDisposed)
    }
  }

  // this.#guard.assertNotDisposed() or this.guard.assertNotDisposed()
  if (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    callee.property.type === AST_NODE_TYPES.Identifier &&
    callee.property.name === 'assertNotDisposed' &&
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
  const isDisposable = isDisposableSubclass(node) || hasDisposeGuardField(node);
  if (!isDisposable) return;

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
    if (!isAssertNotDisposedCall(firstStatement)) {
      context.report({
        node: member,
        messageId: 'missingAssertNotDisposed',
        data: { methodName: getMethodName(member) },
      });
    }
  }
}
