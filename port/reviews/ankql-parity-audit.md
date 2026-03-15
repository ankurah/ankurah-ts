# ankql Test Parity & Source Audit

**Date**: 2026-03-15
**Auditor**: ankql-auditor agent
**Rust source**: `/Users/daniel/ak/ankurah/ankql/src/`
**TS port**: `/Users/daniel/ak/ankurah-ts/packages/ankql/src/`
**Test result**: 76 tests pass, 0 fail, 445 expect() calls

---

## 1. Test Parity: Rust test -> TS test mapping

### grammar.rs (10 tests)

| # | Rust test | TS test (grammar.test.ts) | Status | Notes |
|---|-----------|---------------------------|--------|-------|
| 1 | `test_literal_comparison` | `literal comparison: a=1` | MATCH | Same input `a=1`, verifies path `a`, op `Equal`, literal `I32(1)` |
| 2 | `test_path_comparison` | `path comparison: a.foo = b.foo` | MATCH | Same input, verifies both paths |
| 3 | `test_boolean_expression` | `boolean expression: a.foo = b.foo AND...` | MATCH | Same input, verifies OR(AND(...), Comparison) structure |
| 4 | `test_boolean_expression_parenthetical` | `parenthetical: (a.foo = b.foo AND...)` | MATCH | Same input, verifies nested structure |
| 5 | `test_order_by_clause_basic` | `ORDER BY basic` | MATCH | Same input `true ORDER BY name` |
| 6 | `test_order_by_clause_with_direction` | `ORDER BY with direction` | MATCH | Same input `true ORDER BY name DESC` |
| 7 | `test_limit_clause` | `LIMIT clause` | MATCH | Same input `true LIMIT 10` |
| 8 | `test_order_by_and_limit` | `ORDER BY and LIMIT` | MATCH | Same input with combined clauses |
| 9 | `test_order_by_multiple_items` | `ORDER BY multiple items` | MATCH | Same input including intentional no-space on last item |
| 10 | `test_pathological_cases` | `pathological cases: keywords as identifiers` | MATCH | Both sub-cases: `limit = 1` and `order = 1 ORDER BY name` |

**Note**: Rust tests use `parses_to!` macro checking token positions; TS tests verify the resulting AST structure. This is an expected divergence due to the TS hand-written parser (E6).

An additional TS test `raw parsing: various inputs parse without error` covers the same 15 inputs as the Rust `test_raw_parsing` test, validating they all parse without error.

### ast.rs (9 tests)

| # | Rust test | TS test (ast.test.ts) | Status | Notes |
|---|-----------|------------------------|--------|-------|
| 1 | `test_single_comparison_null_handling` | `single comparison null handling` | MATCH | All 7 sub-assertions identical: status=active, age>30, count>=100, name<Z, score<=90, IS NULL, unrelated column |
| 2 | `nested_predicate_null_handling` | `nested predicate null handling` | MATCH | All 4 sub-assertions identical: charlie, beta+charlie, alpha, other |
| 3 | `test_populate_single_placeholder` | `single placeholder` | MATCH | Same input `name = ?`, populate with "Alice" |
| 4 | `test_populate_multiple_placeholders` | `multiple placeholders` | MATCH | Same input `age > ? AND name = ?`, populate with [25, "Bob"] |
| 5 | `test_populate_in_clause` | `IN clause placeholders` | MATCH | Same input `status IN (?, ?, ?)`, populate with 3 strings |
| 6 | `test_populate_mixed_types` | `mixed types` | MATCH | Same input `active = ? AND score > ? AND name = ?`, populate with [true, 95.5, "Charlie"] |
| 7 | `test_populate_too_few_values` | `too few values` | MATCH | Same input, expects error with "Not enough values" |
| 8 | `test_populate_too_many_values` | `too many values` | MATCH | Same input, expects error with "Too many values" |
| 9 | `test_populate_no_placeholders` | `no placeholders` | MATCH | Same input `name = 'Alice'`, empty values array |

### parser.rs (26 tests)

| # | Rust test | TS test (parser.test.ts) | Status | Notes |
|---|-----------|--------------------------|--------|-------|
| 1 | `test_parse_selection_status_active` | `parse selection: status = active` | MATCH | Identical input/output |
| 2 | `test_parse_selection_user_and_status` | `parse selection: user AND status` | MATCH | Identical input/output, I32(123) |
| 3 | `test_parse_selection_user_or_and_status` | `parse selection: (user OR user) AND status` | MATCH | Identical input/output |
| 4 | `test_parse_selection_status_is_null` | `parse selection: status IS NULL` | MATCH | |
| 5 | `test_parse_selection_status_is_not_null` | `parse selection: status IS NOT NULL` | MATCH | |
| 6 | `unary_not_parenthesized` | `unary NOT parenthesized` | MATCH | |
| 7 | `unary_not_unparenthesized` | `unary NOT unparenthesized fails` | MATCH | Both expect parse failure |
| 8 | `test_parse_empty_string` | `parse empty string` | MATCH | Both expect Predicate::True |
| 9 | `test_parse_true_literal` | `parse true literal` | MATCH | |
| 10 | `test_parse_selection_in_clause` | `parse selection: IN clause with strings` | MATCH | |
| 11 | `test_parse_selection_in_clause_numbers` | `parse selection: IN clause with numbers` | MATCH | Values [1,2,3] as I32 |
| 12 | `test_comparison_to_true` | `comparison to true` | MATCH | `bool_field = true` |
| 13 | `test_comparison_to_false` | `comparison to false` | MATCH | `bool_field <> false` |
| 14 | `test_comparison_to_left_operand_boolean` | `comparison with left operand boolean` | MATCH | `false <> bool_field` |
| 15 | `test_placeholders` (7 sub-tests) | `placeholders` describe block (7 tests) | MATCH | All 7 sub-cases match: single, multiple AND, IN clause, `? AND ?`, `? OR ?`, single `?`, mixed |
| 16 | `test_boolean_literals` (2 sub-tests) | `boolean literals` describe block (2 tests) | MATCH | true -> True, false -> False |
| 17 | `test_order_by_basic` | `basic ORDER BY` | MATCH | |
| 18 | `test_order_by_with_direction` | `ORDER BY with direction` | MATCH | `created_at DESC` |
| 19 | `test_order_by_dotted_identifier_not_supported` | `ORDER BY dotted identifier not supported` | MATCH | Both expect error |
| 20 | `test_limit_basic` | `basic LIMIT` | MATCH | `LIMIT 10` |
| 21 | `test_order_by_and_limit` | `both ORDER BY and LIMIT` | MATCH | `user_id > 100 ORDER BY created_at DESC LIMIT 5` |
| 22 | `test_limit_only` | `LIMIT only` | MATCH | `true LIMIT 100` |
| 23 | `test_order_by_only` | `ORDER BY only` | MATCH | `true ORDER BY score` |
| 24 | `test_order_by_multiple_items` | `ORDER BY multiple items` | MATCH | 3 items with mixed directions |
| 25 | `test_pathological_keyword_cases` | `pathological keyword cases` describe block | MATCH | `limit = 1` and `order = 2 ORDER BY name` (note: Rust uses `order = 2`, verified same) |
| 26 | `test_raw_parsing` | covered by grammar.test.ts `raw parsing` test | MATCH | Same 15 input strings |

**TS-only tests (no Rust equivalent):**
- `parse false literal` -- reasonable addition (Rust only tests "true" standalone but tests "false" in `test_boolean_literals`)
- `dotted path in comparison` -- `person.name = 'Alice'` (covered implicitly by SQL test `test_including_collection_identifier`)
- `dotted paths on both sides` -- `a.foo = b.foo` (covered in grammar tests already)
- `case insensitivity` tests (AND/and/And, OR/or, IS NULL/is null, TRUE/true, IN/in) -- good TS-specific additions since the TS parser is hand-written
- `!= as NotEqual` -- good addition testing the `!=` alias for `<>`

### selection/sql.rs (14 tests)

| # | Rust test | TS test (selection/sql.test.ts) | Status | Notes |
|---|-----------|----------------------------------|--------|-------|
| 1 | `test_simple_equality` | `simple equality` | MATCH | `"name" = 'Alice'` |
| 2 | `test_and_condition` | `AND condition` | MATCH | |
| 3 | `test_complex_condition` | `complex condition` | MATCH | Same nested OR+AND structure |
| 4 | `test_including_collection_identifier` | `including collection identifier (dotted path)` | MATCH | `"person"."name" = 'Alice'` |
| 5 | `test_in_operator` | `IN operator` | MATCH | |
| 6 | `test_placeholder_with_none_count` | `placeholder with None count` | MATCH | |
| 7 | `test_placeholder_with_exact_count` | `placeholder with exact count` | MATCH | `Some(2)` -> `2` |
| 8 | `test_placeholder_count_mismatch_too_few` | `placeholder count mismatch: too few expected` | MATCH | Expected 1, found 2 |
| 9 | `test_placeholder_count_mismatch_too_many` | `placeholder count mismatch: too many expected` | MATCH | Expected 2, found 1 |
| 10 | `test_placeholder_in_lists` | `placeholder in lists` | MATCH | `Some(3)` -> `3` |
| 11 | `test_placeholder_with_zero_count` | `placeholder with zero count (no placeholders)` | MATCH | |
| 12 | `test_string_escaping` | `string escaping: single quotes` | MATCH | `O'Brien` -> `O''Brien` |
| 13 | `test_null_byte_handling` | `null byte handling` | MATCH | `test\0data` -> `testdata` |
| 14 | `test_placeholder_with_zero_count_but_has_placeholder` | `placeholder with zero count but has placeholder` | MATCH | Expected 0, found 1 |

---

## 2. Source Parity

### ast.rs vs ast.ts

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `enum Expr` (6 variants) | `class Expr extends Enum<ExprV>` (6 variants) | MATCH |
| `enum Literal` (10 variants) | `class Literal extends Enum<LiteralV>` (10 variants) | MATCH |
| `struct PathExpr` (steps, simple, is_simple, first, property, Display) | `class PathExpr extends Struct` (same methods) | MATCH |
| `struct Selection` (predicate, order_by, limit, Display, assume_null, referenced_columns) | `class Selection extends Struct` (same methods) | MATCH |
| `struct OrderByItem` (path, direction, Display) | `class OrderByItem extends Struct` (same) | MATCH |
| `enum OrderDirection` (Asc, Desc) | `class OrderDirection extends Enum` (Asc, Desc) | MATCH |
| `enum Predicate` (8 variants) | `class Predicate extends Enum<PredicateV>` (8 variants) | MATCH |
| `Predicate::walk` | `Predicate.walk` | MATCH |
| `Predicate::referenced_columns` | `Predicate.referencedColumns` | MATCH |
| `Predicate::assume_null` | `Predicate.assumeNull` + free `assumeNull()` | MATCH |
| `Predicate::populate` + `populate_recursive` | `Predicate.populate` + `populateRecursive` | MATCH |
| `Expr::populate_recursive` | `Expr.populateRecursive` | MATCH |
| `enum ComparisonOperator` (8 variants) | `class ComparisonOperator extends Enum` (8 variants) | MATCH |
| `enum InfixOperator` (4 variants) | `class InfixOperator extends Enum` (4 variants) | MATCH |
| `From<Predicate> for Selection` | `Selection.fromPredicate` | MATCH |
| `From<String/&str/i64/f64/bool> for Expr` | `exprFromString/exprFromI64/exprFromF64/exprFromBool` | MATCH |
| `From<Vec<T>>/[T;N]/&[T] for Expr` | Not ported | GAP (minor) |

**Gap**: Rust has `From<Vec<T>>`, `From<[T; N]>`, and `From<&[T]>` impls to create `Expr::ExprList` from arrays/slices. The TS port doesn't have an equivalent `exprFromArray` helper. This is a minor gap since `Expr.ExprList(arr)` can be used directly.

### parser.rs vs parser.ts

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `parse_selection` | `parseSelection` | MATCH |
| `parse_expr` | `parseOr` + `parseAnd` + `parseNotOrComparison` + `parseComparison` | MATCH (precedence-climbing replaces flat Pest-driven parsing) |
| `create_comparison` | Inline in `parseComparison` | MATCH |
| `create_logical_op` | `parseAnd` / `parseOr` methods | MATCH |
| `parse_atomic_expr` | `parsePrimaryExpr` | MATCH |
| `parse_path_expr` | `parsePathExpr` | MATCH |
| `parse_string_literal` | `parseStringLiteral` | MATCH |
| `parse_number` | `parseNumber` | MATCH (both use I32 for small, I64 for large) |
| `parse_limit_clause` | Inline in `parseSelection` | MATCH |
| `parse_order_by_clause` | `parseOrderByItems` | MATCH |
| `parse_order_by_item` | `parseOrderByItem` | MATCH |
| Empty string -> Predicate::True | Empty string -> Predicate.True() | MATCH |

**Note**: The TS parser is a hand-written recursive descent parser (E6 decision) replacing Pest. This is an architectural difference but the parsing behavior is equivalent.

### conversion.rs vs ast.ts (no separate conversion.ts)

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `TryFrom<&str> for Predicate` | Not ported (use `parseSelection(...).predicate`) | OK |
| `TryFrom<String> for Predicate` | Not ported | OK |
| `TryFrom<&str> for Selection` | `parseSelection()` serves this role | MATCH |
| `TryFrom<Expr> for Predicate` | `exprToPredicate()` in ast.ts | MATCH |
| `TryFrom<JsValue> for Expr` (wasm) | N/A -- WASM feature, not applicable to TS port | N/A |

**No dedicated conversion.ts needed**: The conversions are either in ast.ts (`exprToPredicate`) or trivially accessed via `parseSelection`. This is appropriate.

### selection/sql.rs vs selection/sql.ts

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `generate_expr_sql` | `generateExprSql` | MATCH |
| `generate_selection_sql` | `generateSelectionSql` | MATCH |
| `generate_selection_sql_inner` | `generatePredicateSql` | MATCH |
| `comparison_op_to_sql` | `comparisonOpToSql` | MATCH |
| String escaping (single quotes, null bytes) | Same logic | MATCH |
| EntityId base64url encoding | Same logic | MATCH |
| Object/Binary handling | Same logic | MATCH |
| JSON value handling | Same logic | MATCH |
| Placeholder counting/mismatch | Same logic | MATCH |
| BETWEEN -> UnsupportedOperator error | Same | MATCH |

### error.rs vs error.ts

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `ParseError::SyntaxError` | `SyntaxError` class | MATCH |
| `ParseError::EmptyExpression` | `EmptyExpressionError` class | MATCH |
| `ParseError::UnexpectedRule` | `UnexpectedTokenError` class | MATCH (adapted: Rust uses Rule, TS uses token type) |
| `ParseError::InvalidPredicate` | `InvalidPredicateError` class | MATCH |
| `ParseError::MissingOperand` | `MissingOperandError` class | MATCH |
| `SqlGenerationError::PlaceholderCountMismatch` | `PlaceholderCountMismatchError` class | MATCH (with .expected/.found fields) |
| `SqlGenerationError::InvalidExpression` | `InvalidExpressionError` class | MATCH |
| `SqlGenerationError::UnsupportedOperator` | `UnsupportedOperatorError` class | MATCH |

### grammar.rs vs grammar.ts

| Rust construct | TS equivalent | Status |
|----------------|---------------|--------|
| `AnkqlParser` (pest-derived) | Hand-written `Lexer` + `Parser` in parser.ts | MATCH (E6) |
| `ankql.pest` grammar rules | `TokenType`, `Token`, char classification in grammar.ts | MATCH |
| Keywords/reserved words | `KEYWORDS`, `RESERVED_WORDS` arrays | MATCH |

---

## 3. Doc Tests

No runnable doc test examples (`/// ``` ... ``` `) found in any Rust source file in `ankql/src/`. No gap.

---

## 4. Correctness Spot-Checks

### Spot-check 1: `test_populate_in_clause` / `IN clause placeholders`

**Rust** (ast.rs:500-515):
- Input: `status IN (?, ?, ?)`
- Populate with `["active", "pending", "review"]` (as `&str` -> `Expr::Literal(Literal::String(...))`)
- Expected: `Predicate::Comparison { left: Path("status"), op: In, right: ExprList([String("active"), String("pending"), String("review")]) }`
- Verified: Direct `assert_eq!` on the AST

**TS** (ast.test.ts:119-149):
- Input: `status IN (?, ?, ?)`
- Populate with `[exprFromString('active'), exprFromString('pending'), exprFromString('review')]`
- Expected: Same structure verified through `.is('Comparison')`, `.value.operator.type === 'In'`, `.is('ExprList')`, and individual element checks
- **Verdict**: CORRECT -- same inputs, same expected structure. TS checks are more verbose due to tagged-union style but verify the same things.

### Spot-check 2: `nested_predicate_null_handling`

**Rust** (ast.rs:455-461):
- Input: `alpha = 1 AND (beta = 2 OR charlie = 3)`
- Test 1: nullify `["charlie"]` -> `"alpha" = 1 AND "beta" = 2` (charlie=3 becomes FALSE, OR(beta=2, FALSE) simplifies to beta=2)
- Test 2: nullify `["beta", "charlie"]` -> `"FALSE"` (entire OR becomes FALSE, AND(alpha=1, FALSE) -> FALSE)
- Test 3: nullify `["alpha"]` -> `"FALSE"` (AND(FALSE, ...) -> FALSE)
- Test 4: nullify `["other"]` -> original preserved

**TS** (ast.test.ts:73-80):
- Identical input string and all 4 expected outputs match exactly.
- The `nullifyColumns` helper mirrors the Rust helper: parse -> `assumeNull` -> `generateSelectionSql`.
- **Verdict**: CORRECT -- perfect match on all 4 sub-cases including the simplification logic.

### Spot-check 3: `test_placeholder_count_mismatch_too_few`

**Rust** (sql.rs:310-319):
- Input: `user_id = ? AND status = ?` (2 placeholders)
- Call `generate_selection_sql` with `Some(1)` (expect 1)
- Expected: `PlaceholderCountMismatch { expected: 1, found: 2 }`
- Verified by destructuring the error variant

**TS** (sql.test.ts:56-68):
- Same input, call `generateSelectionSql(pred, 1)`
- Expected: `PlaceholderCountMismatchError` with `.expected === 1`, `.found === 2`
- Verified via `instanceof` check + field assertions
- **Verdict**: CORRECT -- same input, same expected error, same field values.

---

## 5. Summary

### Test counts

| File | Rust tests | TS tests | Parity |
|------|-----------|----------|--------|
| grammar.rs / grammar.test.ts | 10 | 11 (10 matching + 1 raw-parsing batch) | FULL |
| ast.rs / ast.test.ts | 9 | 9 | FULL |
| parser.rs / parser.test.ts | 26 | 33 (26 matching + 7 TS-only additions) | FULL + extras |
| selection/sql.rs / sql.test.ts | 14 | 14 | FULL |
| **Total** | **59** | **67** | **59/59 matched** |

### Source parity gaps

1. **Minor**: Rust `From<Vec<T>>`, `From<[T; N]>`, `From<&[T]>` for `Expr::ExprList` -- no TS helper. Direct `Expr.ExprList(arr)` works fine. Low impact.
2. **No conversion.ts file**: Not needed. The `TryFrom` impls are either in ast.ts or trivially available through `parseSelection`. Appropriate.

### Issues found

None.

---

## Verdict: PASS

All 59 Rust `#[test]` functions have corresponding TS tests with:
- Same inputs
- Same expected outputs
- Same edge cases
- Same error conditions

The TS port adds 8 extra tests covering case insensitivity and `!=` operator syntax, which are appropriate additions for the hand-written parser. Source parity is complete with one negligible gap (array-to-ExprList convenience helpers).
