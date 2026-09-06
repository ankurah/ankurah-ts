// MIRRORS: ankurah/ankql/src/ast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from './ast';
import { Result, dropOwned } from '@ankurah/base';
import { ParseError } from './error';
import { parseSelection } from './parser';
import { generateSelectionSql } from './selection/sql';

describe('ast unit tests', () => {
  function nullifyColumns(input: string, nullColumns: string[]): Result<string, ParseError> {
    const _r0 = parseSelection(input);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const selection = _r0.unwrap();
    try {
      const result = selection.predicate.assumeNull([...nullColumns].map((s) => s));
      try {
        return generateSelectionSql(result, null).mapErr((_) => new ParseError('InvalidPredicate', { _0: 'SQL generation failed' }));
      } finally {
        result.drop();
      }
    } finally {
      selection.drop();
    }
  }

  test('test_single_comparison_null_handling', () => {
    expect(nullifyColumns('status = \'active\'', ['status']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns('age > 30', ['age']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns('count >= 100', ['count']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns('name < \'Z\'', ['name']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns('score <= 90', ['score']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns('status IS NULL', ['status']).unwrap()).toEqual('TRUE');
    expect(nullifyColumns('role = \'admin\'', ['other']).unwrap()).toEqual('"role" = \'admin\'');
  });

  test('nested_predicate_null_handling', () => {
    const input = 'alpha = 1 AND (beta = 2 OR charlie = 3)';
    expect(nullifyColumns(input, ['charlie']).unwrap()).toEqual('"alpha" = 1 AND "beta" = 2');
    expect(nullifyColumns(input, ['beta', 'charlie']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns(input, ['alpha']).unwrap()).toEqual('FALSE');
    expect(nullifyColumns(input, ['other']).unwrap()).toEqual('"alpha" = 1 AND ("beta" = 2 OR "charlie" = 3)');
  });

  test('test_populate_single_placeholder', () => {
    const selection = parseSelection('name = ?').unwrap();
    try {
      const populated = selection.takeField('predicate').populate(['Alice']).unwrap();
      try {
        const expected = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'Alice' }) }) });
        try {
          expect(populated).toEqual(expected);
        } finally {
          expected.drop();
        }
      } finally {
        populated.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_multiple_placeholders', () => {
    const selection = parseSelection('age > ? AND name = ?').unwrap();
    try {
      const values = [(25n), 'Bob'];
      const populated = selection.takeField('predicate').populate(values).unwrap();
      try {
        const expected = new Predicate('And', { _0: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('age') }), operator: new ComparisonOperator('GreaterThan', {}), right: new Expr('Literal', { _0: new Literal('I64', { _0: 25n }) }) }), _1: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'Bob' }) }) }) });
        try {
          expect(populated).toEqual(expected);
        } finally {
          expected.drop();
        }
      } finally {
        populated.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_in_clause', () => {
    const selection = parseSelection('status IN (?, ?, ?)').unwrap();
    try {
      const populated = selection.takeField('predicate').populate(['active', 'pending', 'review']).unwrap();
      try {
        const expected = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('status') }), operator: new ComparisonOperator('In', {}), right: new Expr('ExprList', { _0: [new Expr('Literal', { _0: new Literal('String', { _0: 'active' }) }), new Expr('Literal', { _0: new Literal('String', { _0: 'pending' }) }), new Expr('Literal', { _0: new Literal('String', { _0: 'review' }) })] }) });
        try {
          expect(populated).toEqual(expected);
        } finally {
          expected.drop();
        }
      } finally {
        populated.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_mixed_types', () => {
    const selection = parseSelection('active = ? AND score > ? AND name = ?').unwrap();
    try {
      const values = [true, (95.5), 'Charlie'];
      const populated = selection.takeField('predicate').populate(values).unwrap();
      {
        const _v4 = populated;
        if (_v4.is('And')) {
          const { _0: left, _1: right } = _v4.value;
          try {
            try {
              {
                const _v2 = left;
                if (_v2.is('And')) {
                  const { _0: innerLeft, _1: innerRight } = _v2.value;
                  try {
                    try {
                      {
                        const _v = innerLeft;
                        if (_v.is('Comparison')) {
                          const { right: val } = _v.value;
                          try {
                            const _t0 = new Expr('Literal', { _0: new Literal('Bool', { _0: true }) });
                            try {
                              expect(val).toEqual(_t0);
                            } finally {
                              _t0.drop();
                            }
                          } finally {
                            dropOwned(val);
                          }
                        } else {
                        _v.drop();
                      }
                      }
                      {
                        const _v1 = innerRight;
                        if (_v1.is('Comparison')) {
                          const { right: val } = _v1.value;
                          try {
                            const _t1 = new Expr('Literal', { _0: new Literal('F64', { _0: 95.5 }) });
                            try {
                              expect(val).toEqual(_t1);
                            } finally {
                              _t1.drop();
                            }
                          } finally {
                            dropOwned(val);
                          }
                        } else {
                        _v1.drop();
                      }
                      }
                    } finally {
                      dropOwned(innerRight);
                    }
                  } finally {
                    dropOwned(innerLeft);
                  }
                } else {
                _v2.drop();
              }
              }
              {
                const _v3 = right;
                if (_v3.is('Comparison')) {
                  const { right: val } = _v3.value;
                  try {
                    const _t2 = new Expr('Literal', { _0: new Literal('String', { _0: 'Charlie' }) });
                    try {
                      expect(val).toEqual(_t2);
                    } finally {
                      _t2.drop();
                    }
                  } finally {
                    dropOwned(val);
                  }
                } else {
                _v3.drop();
              }
              }
            } finally {
              dropOwned(right);
            }
          } finally {
            dropOwned(left);
          }
        } else {
        _v4.drop();
      }
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_too_few_values', () => {
    const selection = parseSelection('name = ? AND age = ?').unwrap();
    try {
      const result = selection.takeField('predicate').populate(['Alice']);
      if (!(result.isErr())) throw new Error('assertion failed');
      const _t0 = result.unwrapErr();
      try {
        if (!(_t0.toString().includes('Not enough values'))) throw new Error('assertion failed');
      } finally {
        _t0.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_too_many_values', () => {
    const selection = parseSelection('name = ?').unwrap();
    try {
      const result = selection.takeField('predicate').populate(['Alice', 'Bob']);
      if (!(result.isErr())) throw new Error('assertion failed');
      const _t0 = result.unwrapErr();
      try {
        if (!(_t0.toString().includes('Too many values'))) throw new Error('assertion failed');
      } finally {
        _t0.drop();
      }
    } finally {
      selection.drop();
    }
  });

  test('test_populate_no_placeholders', () => {
    const selection = parseSelection('name = \'Alice\'').unwrap();
    try {
      const _t0 = selection.clone();
      try {
        const populated = _t0.predicate.populate([]).unwrap();
        try {
          expect(populated).toEqual(selection.predicate);
        } finally {
          populated.drop();
        }
      } finally {
        _t0.drop();
      }
    } finally {
      selection.drop();
    }
  });

});
