// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
//
// Rule: ankurah/no-unhandled-fire-and-forget
// Rust equivalent: task::spawn makes async fire-and-forget explicit.
//
// Flag un-awaited async method calls unless annotated with
// `// fire-and-forget: <justification>`.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'no-unhandled-fire-and-forget';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Async method calls must be awaited or annotated with `// fire-and-forget: <justification>`. ' +
        'This replaces Rust task::spawn that makes fire-and-forget explicit.',
    },
    messages: {
      unawaited:
        'Async call to "{{methodName}}" is not awaited. Either `await` it or add ' +
        '`// fire-and-forget: <justification>` on the preceding line. ' +
        'Un-awaited async calls that mutate shared state cause race conditions. ' +
        '(Rust equivalent: task::spawn makes fire-and-forget explicit; Mutex protects shared state.)',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    return {
      ExpressionStatement(node) {
        const expr = node.expression;

        // We're looking for bare call expressions (not awaited)
        // Awaited calls would be wrapped in AwaitExpression
        if (expr.type !== AST_NODE_TYPES.CallExpression) return;

        // Check if the method name suggests it's async
        const methodName = getCallName(expr);
        if (!methodName) return;

        // Check if parent is an await expression — if so, it's fine
        // (But ExpressionStatement > CallExpression means it's NOT awaited)

        // Check for fire-and-forget comment on the preceding line
        const sourceCode = context.sourceCode ?? context.getSourceCode();
        const comments = sourceCode.getCommentsBefore(node);
        const hasFireAndForget = comments.some(
          (comment: TSESTree.Comment) =>
            comment.value.trim().toLowerCase().startsWith('fire-and-forget:'),
        );
        if (hasFireAndForget) return;

        // Check inline trailing comment on the same line
        const commentsAfter = sourceCode.getCommentsAfter(node);
        const hasInlineFireAndForget = commentsAfter.some(
          (comment: TSESTree.Comment) =>
            comment.value.trim().toLowerCase().startsWith('fire-and-forget:'),
        );
        if (hasInlineFireAndForget) return;

        // Heuristic: only flag calls that look async (common async patterns)
        if (isLikelyAsyncCall(expr, methodName)) {
          context.report({
            node: expr,
            messageId: 'unawaited',
            data: { methodName },
          });
        }
      },
    };
  },
});

function getCallName(node: TSESTree.CallExpression): string | null {
  if (node.callee.type === AST_NODE_TYPES.MemberExpression) {
    const prop = node.callee.property;
    if (prop.type === AST_NODE_TYPES.Identifier) return prop.name;
  }
  if (node.callee.type === AST_NODE_TYPES.Identifier) return node.callee.name;
  return null;
}

// Async-suggestive method name patterns
const ASYNC_PREFIXES = ['fetch', 'load', 'save', 'send', 'connect', 'sync', 'commit', 'rollback'];
const ASYNC_METHOD_NAMES = new Set([
  'then',
  'catch',
  'finally',
]);

function isLikelyAsyncCall(node: TSESTree.CallExpression, methodName: string): boolean {
  // .then()/.catch()/.finally() chains are always async
  if (ASYNC_METHOD_NAMES.has(methodName)) return true;

  // Check if the method name starts with a common async prefix
  const lower = methodName.toLowerCase();
  if (ASYNC_PREFIXES.some((prefix) => lower.startsWith(prefix))) return true;

  return false;
}
