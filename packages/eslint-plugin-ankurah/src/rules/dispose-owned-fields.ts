// RETIRED 2026-09-02 — not registered by the plugin and never runs.
//
// Rule: ankurah/dispose-owned-fields
//
// This rule told a Drop subclass to drop each of its owned fields by hand in
// onDrop(). The runtime now does that itself: `AkObject.drop()` runs onDrop()
// and then, in a `finally`, drops everything `ownedFields()` returns, which by
// default is every own property. Dropping an own field in onDrop() as well
// drops it twice, and a second drop is fatal — so the rule was asking for the
// one thing the runtime refuses. Its three findings (packages/core
// livequery.ts, packages/signals signal/index.ts) are all values the cascade
// already releases.
//
// The inverse check — never drop an own field in onDrop() — is not worth
// writing: dropping a field and then setting it to null is a legitimate way to
// hand ownership away early, and telling the two apart needs the types. The
// runtime reports the real double drop where it happens, by name.
//
// What does still need saying belongs in the spec, not a lint rule: a type that
// keeps owned state in #private fields must override ownedFields(), because the
// cascade cannot see private state. See port/ownership.md and
// port/retractions-2026-09-02.md.
//
// The file is kept only so its removal is a staged deletion Daniel reads rather
// than one that disappears inside another diff.
//
// Original description follows.
//
// Rust equivalent: Auto-Drop cascade through owned fields.
//
// If a class extends Drop and has fields typed as Drop (or with
// a drop() method), its onDrop() must call .drop() on each of them.

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
        'Drop subclasses must drop all owned Drop fields in onDrop(). ' +
        'This replaces Rust auto-Drop cascade through owned fields.',
    },
    messages: {
      missingFieldDispose:
        'Drop field "{{fieldName}}" is not dropped in onDrop(). ' +
        'All owned Drop fields must have .drop() called in onDrop() to prevent resource leaks. ' +
        '(Rust equivalent: Drop is automatically cascaded to all owned fields.)',
      missingOnDispose:
        'Class extends Drop and has Drop fields ({{fieldNames}}) but no onDrop() method. ' +
        'Implement onDrop() to drop these fields. ' +
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

// Type names that are known to be Drop
const DROP_TYPE_NAMES = new Set([
  'Drop',
  'DropGuard',
]);

// Heuristic: type names that likely extend Drop
// (ending in Subscription, Guard, LiveQuery, etc.)
const DROP_SUFFIXES = [
  'Subscription',
  'LiveQuery',
  'Guard',
  'Watcher',
  'Connection',
];

function isDropSubclass(node: TSESTree.ClassDeclaration | TSESTree.ClassExpression): boolean {
  if (!node.superClass) return false;
  if (node.superClass.type === AST_NODE_TYPES.Identifier) {
    return node.superClass.name === 'Drop';
  }
  if (node.superClass.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.superClass.property;
    if (prop.type === AST_NODE_TYPES.Identifier) {
      return prop.name === 'Drop';
    }
  }
  return false;
}

function isDropTypeName(name: string): boolean {
  if (DROP_TYPE_NAMES.has(name)) return true;
  return DROP_SUFFIXES.some((suffix) => name.endsWith(suffix));
}

function getFieldDropType(member: TSESTree.PropertyDefinition): string | null {
  // Check type annotation
  const typeAnnotation = member.typeAnnotation?.typeAnnotation;
  if (typeAnnotation) {
    if (
      typeAnnotation.type === AST_NODE_TYPES.TSTypeReference &&
      typeAnnotation.typeName.type === AST_NODE_TYPES.Identifier
    ) {
      const typeName = typeAnnotation.typeName.name;
      if (isDropTypeName(typeName)) {
        return typeName;
      }
    }
  }

  // Check value for `new SomethingDrop(...)`
  const value = member.value;
  if (
    value?.type === AST_NODE_TYPES.NewExpression &&
    value.callee.type === AST_NODE_TYPES.Identifier
  ) {
    if (isDropTypeName(value.callee.name)) {
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

function getDroppedFieldsInOnDrop(
  classNode: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
): { method: TSESTree.MethodDefinition | null; droppedFields: Set<string> } {
  const onDropMethod = classNode.body.body.find(
    (member): member is TSESTree.MethodDefinition =>
      member.type === AST_NODE_TYPES.MethodDefinition &&
      member.key.type === AST_NODE_TYPES.Identifier &&
      member.key.name === 'onDrop',
  );

  if (!onDropMethod?.value.body) {
    return { method: onDropMethod ?? null, droppedFields: new Set() };
  }

  const droppedFields = new Set<string>();
  collectDropCallsFromStatements(onDropMethod.value.body.body, droppedFields);

  return { method: onDropMethod, droppedFields };
}

function collectDropCallsFromStatements(
  statements: TSESTree.Statement[],
  droppedFields: Set<string>,
): void {
  for (const stmt of statements) {
    collectDropCallsFromNode(stmt, droppedFields);
  }
}

// Keys to skip when traversing the AST (these cause circular references)
const SKIP_KEYS = new Set(['parent']);

function collectDropCallsFromNode(
  node: TSESTree.Node,
  droppedFields: Set<string>,
  visited?: Set<TSESTree.Node>,
): void {
  const seen = visited ?? new Set<TSESTree.Node>();
  if (seen.has(node)) return;
  seen.add(node);

  // Look for this.field.drop() or this.#field.drop()
  if (
    node.type === AST_NODE_TYPES.CallExpression &&
    node.callee.type === AST_NODE_TYPES.MemberExpression &&
    node.callee.property.type === AST_NODE_TYPES.Identifier &&
    node.callee.property.name === 'drop'
  ) {
    const obj = node.callee.object;
    // this.field.drop()
    if (
      obj.type === AST_NODE_TYPES.MemberExpression &&
      obj.object.type === AST_NODE_TYPES.ThisExpression
    ) {
      if (obj.property.type === AST_NODE_TYPES.Identifier) {
        droppedFields.add(obj.property.name);
      }
      if (obj.property.type === AST_NODE_TYPES.PrivateIdentifier) {
        droppedFields.add(obj.property.name);
      }
    }

    // Also handle optional chaining: this.field?.drop()
    if (
      obj.type === AST_NODE_TYPES.ChainExpression &&
      obj.expression.type === AST_NODE_TYPES.MemberExpression &&
      obj.expression.object.type === AST_NODE_TYPES.ThisExpression
    ) {
      const prop = obj.expression.property;
      if (prop.type === AST_NODE_TYPES.Identifier) droppedFields.add(prop.name);
      if (prop.type === AST_NODE_TYPES.PrivateIdentifier) droppedFields.add(prop.name);
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
            collectDropCallsFromNode(item as TSESTree.Node, droppedFields, seen);
          }
        }
      } else if ('type' in child) {
        collectDropCallsFromNode(child as TSESTree.Node, droppedFields, seen);
      }
    }
  }
}

function checkClass(
  context: any,
  node: TSESTree.ClassDeclaration | TSESTree.ClassExpression,
) {
  if (!isDropSubclass(node)) return;

  // Find all fields typed as Drop
  const dropFields: { name: string; member: TSESTree.PropertyDefinition }[] = [];
  for (const member of node.body.body) {
    if (member.type !== AST_NODE_TYPES.PropertyDefinition) continue;
    const fieldName = getFieldName(member);
    if (!fieldName) continue;
    const dropType = getFieldDropType(member);
    if (dropType) {
      dropFields.push({ name: fieldName, member });
    }
  }

  if (dropFields.length === 0) return;

  const { method: onDropMethod, droppedFields } = getDroppedFieldsInOnDrop(node);

  if (!onDropMethod) {
    context.report({
      node: node,
      messageId: 'missingOnDispose',
      data: { fieldNames: dropFields.map((f) => f.name).join(', ') },
    });
    return;
  }

  for (const field of dropFields) {
    if (!droppedFields.has(field.name)) {
      context.report({
        node: field.member,
        messageId: 'missingFieldDispose',
        data: { fieldName: field.name },
      });
    }
  }
}
