// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/dispose-owned-fields
// Rust equivalent: Auto-Drop cascade through owned fields.
//
// If a class extends Disposable and has fields typed as Disposable (or with
// a dispose() method), its onDispose() must call .dispose() on each of them.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'dispose-owned-fields';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Disposable subclasses must dispose all owned Disposable fields in onDispose(). ' +
        'This replaces Rust auto-Drop cascade through owned fields.',
    },
    messages: {
      missingFieldDispose:
        'Disposable field "{{fieldName}}" is not disposed in onDispose(). ' +
        'All owned Disposable fields must have .dispose() called in onDispose() to prevent resource leaks. ' +
        '(Rust equivalent: Drop is automatically cascaded to all owned fields.)',
      missingOnDispose:
        'Class extends Disposable and has Disposable fields ({{fieldNames}}) but no onDispose() method. ' +
        'Implement onDispose() to dispose these fields. ' +
        '(Rust equivalent: Drop auto-cascades; in TS you must do this explicitly.)',
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

// Type names that are known to be Disposable
const DISPOSABLE_TYPE_NAMES = new Set([
  'Disposable',
  'DisposeGuard',
]);

// Heuristic: type names that likely extend Disposable
// (ending in Subscription, Guard, LiveQuery, etc.)
const DISPOSABLE_SUFFIXES = [
  'Subscription',
  'LiveQuery',
  'Guard',
  'Watcher',
  'Connection',
];

function isDisposableSubclass(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Disposable';
  }
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    if (prop.type === AST_NODE_TYPES.Identifier) {
      return prop.name === 'Disposable';
    }
  }
  return false;
}

function isDisposableTypeName(name: string): boolean {
  if (DISPOSABLE_TYPE_NAMES.has(name)) return true;
  return DISPOSABLE_SUFFIXES.some((suffix) => name.endsWith(suffix));
}

function getFieldDisposableType(member: TSESTree.PropertyDefinition): string | null {
  // Check type annotation
  const typeAnnotation = member.typeAnnotation?.typeAnnotation;
  if (typeAnnotation) {
    if (
      typeAnnotation.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier
    ) {
      const typeName = typeAnnotation.typeName.name;
      if (isDisposableTypeName(typeName)) {
        return typeName;
      }
    }
  }

  // Check value for `new SomethingDisposable(...)`
  const value = member.value;
  if (
    value?.type === AST_NODE_TYPES.NewExpression &&
    value.callee.type === AST_NODE_TYPES.Identifier
  ) {
    if (isDisposableTypeName(value.callee.name)) {
      return value.callee.name;
    }
  }

  return null;
}

function getFieldName(member: TSESTree.PropertyDefinition): string | null {
  if (member.key.type === AST_NODE_TYPES.Identifier) return member.key.name;
  if (member.key.type === AST_NODE_TYPES.PrivateIdentifier) return member.key.name;
  return null;
}

function getDisposedFieldsInOnDispose(
  classNode: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
): { method: TSESTree.MethodDefinition | null; disposedFields: Set<string> } {
  const onDisposeMethod = classNode.body.body.find(
    (member): member is TSESTree.MethodDefinition =>
      member.type === AST_NODE_TYPES.MethodDefinition &&
      member.key.type === AST_NODE_TYPES.Identifier &&
      member.key.name === 'onDispose',
  );

  if (!onDisposeMethod?.value.body) {
    return { method: onDisposeMethod ?? null, disposedFields: new Set() };
  }

  const disposedFields = new Set<string>();
  collectDisposeCallsFromStatements(onDisposeMethod.value.body.body, disposedFields);

  return { method: onDisposeMethod, disposedFields };
}

function collectDisposeCallsFromStatements(
  statements: TSESTree.Statement[],
  disposedFields: Set<string>,
): void {
  for (const stmt of statements) {
    collectDisposeCallsFromNode(stmt, disposedFields);
  }
}

// Keys to skip when traversing the AST (these cause circular references)
const SKIP_KEYS = new Set(['parent']);

function collectDisposeCallsFromNode(
  node: TSESTree.Node,
  disposedFields: Set<string>,
  visited?: Set<TSESTree.Node>,
): void {
  const seen = visited ?? new Set<TSESTree.Node>();
  if (seen.has(node)) return;
  seen.add(node);

  // Look for this.field.dispose() or this.#field.dispose()
  if (
    node.type === AST_NODE_TYPES.CallExpression &&
    node.callee.type === AST_NODE_TYPES.MemberExpression &&
    node.callee.property.type === AST_NODE_TYPES.Identifier &&
    node.callee.property.name === 'dispose'
  ) {
    const obj = node.callee.object;
    // this.field.dispose()
    if (
      obj.type === AST_NODE_TYPES.MemberExpression &&
      obj.object.type === AST_NODE_TYPES.ThisExpression
    ) {
      if (obj.property.type === AST_NODE_TYPES.Identifier) {
        disposedFields.add(obj.property.name);
      }
      if (obj.property.type === AST_NODE_TYPES.PrivateIdentifier) {
        disposedFields.add(obj.property.name);
      }
    }

    // Also handle optional chaining: this.field?.dispose()
    if (
      obj.type === AST_NODE_TYPES.ChainExpression &&
      obj.expression.type === AST_NODE_TYPES.MemberExpression &&
      obj.expression.object.type === AST_NODE_TYPES.ThisExpression
    ) {
      const prop = obj.expression.property;
      if (prop.type === AST_NODE_TYPES.Identifier) disposedFields.add(prop.name);
      if (prop.type === AST_NODE_TYPES.PrivateIdentifier) disposedFields.add(prop.name);
    }
  }

  // Recurse into child nodes, skipping circular-reference keys
  for (const key of Object.keys(node)) {
    if (SKIP_KEYS.has(key)) continue;
    const child = (node as any)[key];
    if (child && typeof child === 'object') {
      if (Array.isArray(child)) {
        for (const item of child) {
          if (item && typeof item === 'object' && 'type' in item) {
            collectDisposeCallsFromNode(item as TSESTree.Node, disposedFields, seen);
          }
        }
      } else if ('type' in child) {
        collectDisposeCallsFromNode(child as TSESTree.Node, disposedFields, seen);
      }
    }
  }
}

function checkClass(
  context: any,
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
) {
  if (!isDisposableSubclass(node)) return;

  // Find all fields typed as Disposable
  const disposableFields: { name: string; member: TSESTree.PropertyDefinition }[] = [];
  for (const member of node.body.body) {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) continue;
    const fieldName = getFieldName(member);
    if (!fieldName) continue;
    const disposableType = getFieldDisposableType(member);
    if (disposableType) {
      disposableFields.push({ name: fieldName, member });
    }
  }

  if (disposableFields.length === 0) return;

  const { method: onDisposeMethod, disposedFields } = getDisposedFieldsInOnDispose(node);

  if (!onDisposeMethod) {
    context.report({
      node: node,
      messageId: 'missingOnDispose',
      data: { fieldNames: disposableFields.map((f) => f.name).join(', ') },
    });
    return;
  }

  for (const field of disposableFields) {
    if (!disposedFields.has(field.name)) {
      context.report({
        node: field.member,
        messageId: 'missingFieldDispose',
        data: { fieldName: field.name },
      });
    }
  }
}
