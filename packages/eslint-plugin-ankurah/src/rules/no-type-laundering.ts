// TS-ONLY: ESLint plugin enforcing port fidelity
//
// Rule: ankurah/no-type-laundering
// Flag `as unknown as X` type casts that lack a `// Divergence:` comment
// on the same or preceding line. This pattern bypasses TypeScript's type
// system entirely and must be justified with a Divergence annotation.

import { ESLintUtils, AST_NODE_TYPES } from '@typescript-eslint/utils';
import type { TSESTree } from '@typescript-eslint/utils';

export const RULE_NAME = 'no-type-laundering';

export const rule = ESLintUtils.RuleCreator(
  (name) => `https://github.com/nickthedick69/ankurah-ts/blob/main/specs/ownership/lint-rules.md#${name}`,
)({
  name: RULE_NAME,
  meta: {
    type: 'problem',
    docs: {
      description:
        'Require a `// Divergence: <reason>` comment on `as unknown as` type casts. ' +
        'This pattern bypasses type safety and must be justified.',
    },
    messages: {
      missingDivergenceComment:
        '`as unknown as` type cast must have a `// Divergence: <reason>` comment on the same or preceding line. ' +
        'This pattern bypasses type safety and must be justified.',
    },
    schema: [],
  },
  defaultOptions: [],
  create(context) {
    return {
      TSAsExpression(node: TSESTree.TSAsExpression) {
        // We want the outer `as X` where the inner expression is `expr as unknown`
        if (node.expression.type !== AST_NODE_TYPES.TSAsExpression) return;

        const innerCast = node.expression;
        // Check that the inner cast is `as unknown`
        if (
          innerCast.typeAnnotation.type !== AST_NODE_TYPES.TSUnknownKeyword
        ) {
          return;
        }

        // Now we have confirmed `expr as unknown as X` — check for Divergence comment
        const sourceCode = context.sourceCode ?? context.getSourceCode();
        const outerLine = node.loc.start.line;

        // Get all comments in the file
        const comments = sourceCode.getAllComments();

        const hasDivergenceComment = comments.some((comment) => {
          const commentLine = comment.loc.end.line;
          // Same line or preceding line
          return (
            (commentLine === outerLine || commentLine === outerLine - 1) &&
            comment.value.includes('Divergence:')
          );
        });

        if (!hasDivergenceComment) {
          context.report({
            node,
            messageId: 'missingDivergenceComment',
          });
        }
      },
    };
  },
});
