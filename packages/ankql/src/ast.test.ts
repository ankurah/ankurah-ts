// MIRRORS: ankurah/ankql/src/ast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from './ast';
import { Result, dropOwned, dropUnbound } from '@ankurah/base';
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
        return generateSelectionSql(result, null).mapErr((_) => {
          try {
            return new ParseError('InvalidPredicate', { _0: 'SQL generation failed' });
          } finally {
            _.drop();
          }
        });
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
      const populated = selection.takeField('predicate').populate(['Alice'], (value: string) => Result.Ok(Expr.fromString(value))).unwrap();
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
      const values = [Expr.fromI64((25n)), Expr.fromString('Bob')];
      const populated = selection.takeField('predicate').populate(values, (value: Expr) => Result.Ok(value)).unwrap();
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
      const populated = selection.takeField('predicate').populate(['active', 'pending', 'review'], (value: string) => Result.Ok(Expr.fromString(value))).unwrap();
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
      const values = [Expr.fromBool(true), Expr.fromF64((95.5)), Expr.fromString('Charlie')];
      const populated = selection.takeField('predicate').populate(values, (value: Expr) => Result.Ok(value)).unwrap();
      return populated.intoMatch({
        And: (v) => {
          const left = v._0;
          const right = v._1;
          let _moved0 = false;
          let _moved1 = false;
          try {
            try {
              _moved0 = true;
              left.intoMatch({
                And: (v) => {
                  const innerLeft = v._0;
                  const innerRight = v._1;
                  let _moved2 = false;
                  let _moved3 = false;
                  try {
                    try {
                      _moved2 = true;
                      innerLeft.intoMatch({
                        Comparison: (v) => {
                          const val = v.right;
                          try {
                            try {
                              const _t4 = new Expr('Literal', { _0: new Literal('Bool', { _0: true }) });
                              try {
                                expect(val).toEqual(_t4);
                              } finally {
                                _t4.drop();
                              }
                            } finally {
                              dropOwned(val);
                            }
                          } finally {
                            dropUnbound(v, ['right']);
                          }
                        },
                        IsNull: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        And: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        Or: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        Not: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        True: () => {},
                        False: () => {},
                        Placeholder: () => {},
                      });
                      _moved3 = true;
                      return innerRight.intoMatch({
                        Comparison: (v) => {
                          const val = v.right;
                          try {
                            try {
                              const _t5 = new Expr('Literal', { _0: new Literal('F64', { _0: 95.5 }) });
                              try {
                                expect(val).toEqual(_t5);
                              } finally {
                                _t5.drop();
                              }
                            } finally {
                              dropOwned(val);
                            }
                          } finally {
                            dropUnbound(v, ['right']);
                          }
                        },
                        IsNull: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        And: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        Or: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        Not: (v) => {
                          try {
                          } finally {
                            dropUnbound(v, []);
                          }
                        },
                        True: () => {},
                        False: () => {},
                        Placeholder: () => {},
                      });
                    } finally {
                      if (!_moved3) dropOwned(innerRight);
                    }
                  } finally {
                    if (!_moved2) dropOwned(innerLeft);
                  }
                },
                Comparison: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                IsNull: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                Or: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                Not: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                True: () => {},
                False: () => {},
                Placeholder: () => {},
              });
              _moved1 = true;
              return right.intoMatch({
                Comparison: (v) => {
                  const val = v.right;
                  try {
                    try {
                      const _t6 = new Expr('Literal', { _0: new Literal('String', { _0: 'Charlie' }) });
                      try {
                        expect(val).toEqual(_t6);
                      } finally {
                        _t6.drop();
                      }
                    } finally {
                      dropOwned(val);
                    }
                  } finally {
                    dropUnbound(v, ['right']);
                  }
                },
                IsNull: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                And: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                Or: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                Not: (v) => {
                  try {
                  } finally {
                    dropUnbound(v, []);
                  }
                },
                True: () => {},
                False: () => {},
                Placeholder: () => {},
              });
            } finally {
              if (!_moved1) dropOwned(right);
            }
          } finally {
            if (!_moved0) dropOwned(left);
          }
        },
        Comparison: (v) => {
          try {
          } finally {
            dropUnbound(v, []);
          }
        },
        IsNull: (v) => {
          try {
          } finally {
            dropUnbound(v, []);
          }
        },
        Or: (v) => {
          try {
          } finally {
            dropUnbound(v, []);
          }
        },
        Not: (v) => {
          try {
          } finally {
            dropUnbound(v, []);
          }
        },
        True: () => {},
        False: () => {},
        Placeholder: () => {},
      });
    } finally {
      selection.drop();
    }
  });

  test('test_populate_too_few_values', () => {
    const selection = parseSelection('name = ? AND age = ?').unwrap();
    try {
      const result = selection.takeField('predicate').populate(['Alice'], (value: string) => Result.Ok(Expr.fromString(value)));
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
      const result = selection.takeField('predicate').populate(['Alice', 'Bob'], (value: string) => Result.Ok(Expr.fromString(value)));
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
        const populated = _t0.takeField('predicate').populate([], (value: string) => Result.Ok(Expr.fromString(value))).unwrap();
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
