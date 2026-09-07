// MIRRORS: ankurah/core/src/value/cast_predicate.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { castPredicateTypes } from './cast_predicate';
import { Result, Struct, dropOwned, dropUnbound } from '@ankurah/base';
import { Comparison } from '../lineage';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

class TestSchema extends Struct implements CollectionSchema {

  fieldType(path: PathExpr): Result<ValueType, PropertyError> {
    const propertyName = path.property();
    if (propertyName === 'id') {
      return Result.Ok(new ValueType('EntityId', {}));
    } else {
      return Result.Ok(new ValueType('String', {}));
    }
  }
}

describe('cast_predicate unit tests', () => {
  test('test_cast_id_field_string_to_entity_id', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('id') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    return castPredicate.intoMatch({
      Comparison: (v) => {
        const right = v.right;
        try {
          let _moved0 = false;
          try {
            _moved0 = true;
            return right.intoMatch({
              Literal: (v) => {
                if (v._0.is('EntityId')) {
                  const { _0: ulid } = v._0.value;
                  try {
                    expect(EntityId.fromUlid(ulid)).toEqual(entityId);
                  } finally {
                    dropUnbound(v, []);
                  }
                } else {
                  try {
                    throw new Error(`Expected EntityId literal, got ${right.debug()}`);
                  } finally {
                    dropUnbound(v, []);
                  }
                }
              },
              Path: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${right.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              Predicate: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${right.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              InfixExpr: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${right.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              ExprList: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${right.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              Placeholder: () => {
                throw new Error(`Expected EntityId literal, got ${right.debug()}`);
              },
            });
          } finally {
            if (!_moved0) dropOwned(right);
          }
        } finally {
          dropUnbound(v, ['right']);
        }
      },
      IsNull: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      And: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Or: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Not: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      True: () => {
        throw new Error('Expected Comparison predicate');
      },
      False: () => {
        throw new Error('Expected Comparison predicate');
      },
      Placeholder: () => {
        throw new Error('Expected Comparison predicate');
      },
    });
  });

  test('test_cast_literal_equals_field', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('Comparison', { left: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Path', { _0: PathExpr.simple('id') }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    return castPredicate.intoMatch({
      Comparison: (v) => {
        const left = v.left;
        try {
          let _moved0 = false;
          try {
            _moved0 = true;
            return left.intoMatch({
              Literal: (v) => {
                if (v._0.is('EntityId')) {
                  const { _0: ulid } = v._0.value;
                  try {
                    expect(EntityId.fromUlid(ulid)).toEqual(entityId);
                  } finally {
                    dropUnbound(v, []);
                  }
                } else {
                  try {
                    throw new Error(`Expected EntityId literal, got ${left.debug()}`);
                  } finally {
                    dropUnbound(v, []);
                  }
                }
              },
              Path: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${left.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              Predicate: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${left.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              InfixExpr: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${left.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              ExprList: (v) => {
                try {
                  throw new Error(`Expected EntityId literal, got ${left.debug()}`);
                } finally {
                  dropUnbound(v, []);
                }
              },
              Placeholder: () => {
                throw new Error(`Expected EntityId literal, got ${left.debug()}`);
              },
            });
          } finally {
            if (!_moved0) dropOwned(left);
          }
        } finally {
          dropUnbound(v, ['left']);
        }
      },
      IsNull: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      And: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Or: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Not: (v) => {
        try {
          throw new Error('Expected Comparison predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      True: () => {
        throw new Error('Expected Comparison predicate');
      },
      False: () => {
        throw new Error('Expected Comparison predicate');
      },
      Placeholder: () => {
        throw new Error('Expected Comparison predicate');
      },
    });
  });

  test('test_cast_complex_predicate', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const predicate = new Predicate('And', { _0: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('id') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: base64Str }) }) }), _1: new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'test' }) }) }) });
    const schema = new TestSchema();
    const castPredicate = castPredicateTypes(predicate, schema).unwrap();
    return castPredicate.intoMatch({
      And: (v) => {
        const leftPred = v._0;
        const rightPred = v._1;
        let _moved0 = false;
        let _moved1 = false;
        try {
          try {
            _moved0 = true;
            leftPred.intoMatch({
              Comparison: (v) => {
                const right = v.right;
                try {
                  let _moved2 = false;
                  try {
                    _moved2 = true;
                    return right.intoMatch({
                      Literal: (v) => {
                        if (v._0.is('EntityId')) {
                          const { _0: ulid } = v._0.value;
                          try {
                            expect(EntityId.fromUlid(ulid)).toEqual(entityId);
                          } finally {
                            dropUnbound(v, []);
                          }
                        } else {
                          try {
                            throw new Error('Expected EntityId literal for id field');
                          } finally {
                            dropUnbound(v, []);
                          }
                        }
                      },
                      Path: (v) => {
                        try {
                          throw new Error('Expected EntityId literal for id field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      Predicate: (v) => {
                        try {
                          throw new Error('Expected EntityId literal for id field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      InfixExpr: (v) => {
                        try {
                          throw new Error('Expected EntityId literal for id field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      ExprList: (v) => {
                        try {
                          throw new Error('Expected EntityId literal for id field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      Placeholder: () => {
                        throw new Error('Expected EntityId literal for id field');
                      },
                    });
                  } finally {
                    if (!_moved2) dropOwned(right);
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
            _moved1 = true;
            return rightPred.intoMatch({
              Comparison: (v) => {
                const right = v.right;
                try {
                  let _moved3 = false;
                  try {
                    _moved3 = true;
                    return right.intoMatch({
                      Literal: (v) => {
                        if (v._0.is('String')) {
                          const { _0: s } = v._0.value;
                          try {
                            expect(s).toEqual('test');
                          } finally {
                            dropUnbound(v, []);
                          }
                        } else {
                          try {
                            throw new Error('Expected String literal for name field');
                          } finally {
                            dropUnbound(v, []);
                          }
                        }
                      },
                      Path: (v) => {
                        try {
                          throw new Error('Expected String literal for name field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      Predicate: (v) => {
                        try {
                          throw new Error('Expected String literal for name field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      InfixExpr: (v) => {
                        try {
                          throw new Error('Expected String literal for name field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      ExprList: (v) => {
                        try {
                          throw new Error('Expected String literal for name field');
                        } finally {
                          dropUnbound(v, []);
                        }
                      },
                      Placeholder: () => {
                        throw new Error('Expected String literal for name field');
                      },
                    });
                  } finally {
                    if (!_moved3) dropOwned(right);
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
            if (!_moved1) dropOwned(rightPred);
          }
        } finally {
          if (!_moved0) dropOwned(leftPred);
        }
      },
      Comparison: (v) => {
        try {
          throw new Error('Expected And predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      IsNull: (v) => {
        try {
          throw new Error('Expected And predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Or: (v) => {
        try {
          throw new Error('Expected And predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      Not: (v) => {
        try {
          throw new Error('Expected And predicate');
        } finally {
          dropUnbound(v, []);
        }
      },
      True: () => {
        throw new Error('Expected And predicate');
      },
      False: () => {
        throw new Error('Expected And predicate');
      },
      Placeholder: () => {
        throw new Error('Expected And predicate');
      },
    });
  });

});
