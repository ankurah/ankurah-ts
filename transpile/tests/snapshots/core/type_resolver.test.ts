// MIRRORS: ankurah/core/src/type_resolver.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { TypeResolver } from './type_resolver';
import { dropOwned } from '@ankurah/base';
import { Comparison } from './lineage';
import { Json } from './property/value/json';
import { ValueType } from './value/index';
import { ComparisonOperator, Expr, Literal, PathExpr, Predicate } from '@ankurah/ankql';
import { EntityId } from '@ankurah/proto';

describe('type_resolver unit tests', () => {
  test('test_resolve_simple_path', () => {
    const resolver = TypeResolver.new();
    try {
      const path = PathExpr.simple('name');
      try {
        expect(resolver.resolvePath(path)).toEqual(null);
      } finally {
        path.drop();
      }
    } finally {
      resolver.drop();
    }
  });

  test('test_resolve_id_path', () => {
    const resolver = TypeResolver.new();
    try {
      const path = PathExpr.simple('id');
      try {
        expect(resolver.resolvePath(path)).toEqual(new ValueType('EntityId', {}));
      } finally {
        path.drop();
      }
    } finally {
      resolver.drop();
    }
  });

  test('test_resolve_json_path', () => {
    const resolver = TypeResolver.new();
    try {
      const path = new PathExpr(['data', 'number']);
      try {
        expect(resolver.resolvePath(path)).toEqual(new ValueType('Json', {}));
      } finally {
        path.drop();
      }
    } finally {
      resolver.drop();
    }
  });

  test('test_literal_to_json_string', () => {
    const lit = new Literal('String', { _0: 'hello' });
    try {
      let _moved0 = false;
      const jsonLit = TypeResolver.literalToJson(lit);
      try {
        if (jsonLit.is('Json') && (jsonLit.value._0.is('String'))) {
          const { _0: s } = jsonLit.value._0.value;
          expect(s).toEqual('hello');
        } else {
          _moved0 = true;
          const other = jsonLit;
          try {
            throw new Error(`Expected Json(String), got ${other.debug()}`);
          } finally {
            other.drop();
          }
        }
      } finally {
        if (!_moved0) jsonLit.drop();
      }
    } finally {
      lit.drop();
    }
  });

  test('test_literal_to_json_number', () => {
    const lit = new Literal('I64', { _0: 42n });
    try {
      let _moved0 = false;
      const jsonLit = TypeResolver.literalToJson(lit);
      try {
        if (jsonLit.is('Json') && (jsonLit.value._0.is('Number'))) {
          const { _0: n } = jsonLit.value._0.value;
          expect(n.asI64()).toEqual(42n);
        } else {
          _moved0 = true;
          const other = jsonLit;
          try {
            throw new Error(`Expected Json(Number), got ${other.debug()}`);
          } finally {
            other.drop();
          }
        }
      } finally {
        if (!_moved0) jsonLit.drop();
      }
    } finally {
      lit.drop();
    }
  });

  test('test_resolve_types_converts_literal_for_json_path', () => {
    const resolver = TypeResolver.new();
    try {
      const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: new PathExpr(['data', 'number']) }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('I64', { _0: 9n }) }) });
      const resolved = resolver.resolveTypes(predicate);
      {
        const _v = resolved;
        if (_v.is('Comparison')) {
          const { right } = _v.value;
          try {
            return right.intoMatch({
              Literal: (v) => {
                if (v._0.is('Json') && (v._0.value._0.is('Number'))) {
                  const { _0: n } = v._0.value._0.value;
                  expect(n.asI64()).toEqual(9n);
                } else {
                  const other = new Expr('Literal', v);
                  try {
                    throw new Error(`Expected Json(Number), got ${other.debug()}`)
                  } finally {
                    other.drop();
                  }
                }
              },
              Path: (v) => {
                const other = new Expr('Path', v);
                try {
                  throw new Error(`Expected Json(Number), got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              Predicate: (v) => {
                const other = new Expr('Predicate', v);
                try {
                  throw new Error(`Expected Json(Number), got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              InfixExpr: (v) => {
                const other = new Expr('InfixExpr', v);
                try {
                  throw new Error(`Expected Json(Number), got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              ExprList: (v) => {
                const other = new Expr('ExprList', v);
                try {
                  throw new Error(`Expected Json(Number), got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              Placeholder: (v) => {
                const other = new Expr('Placeholder', v);
                try {
                  throw new Error(`Expected Json(Number), got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
            });
          } finally {
            dropOwned(right);
          }
        } else {
        _v.drop();
        throw new Error('Expected Comparison predicate');
      }
      }
    } finally {
      resolver.drop();
    }
  });

  test('test_resolve_types_leaves_simple_path_literal_alone', () => {
    const resolver = TypeResolver.new();
    try {
      const predicate = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('name') }), operator: new ComparisonOperator('Equal', {}), right: new Expr('Literal', { _0: new Literal('String', { _0: 'test' }) }) });
      const resolved = resolver.resolveTypes(predicate);
      {
        const _v = resolved;
        if (_v.is('Comparison')) {
          const { right } = _v.value;
          try {
            return right.intoMatch({
              Literal: (v) => {
                if (v._0.is('String')) {
                  const { _0: s } = v._0.value;
                  expect(s).toEqual('test');
                } else {
                  const other = new Expr('Literal', v);
                  try {
                    throw new Error(`Expected String literal, got ${other.debug()}`)
                  } finally {
                    other.drop();
                  }
                }
              },
              Path: (v) => {
                const other = new Expr('Path', v);
                try {
                  throw new Error(`Expected String literal, got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              Predicate: (v) => {
                const other = new Expr('Predicate', v);
                try {
                  throw new Error(`Expected String literal, got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              InfixExpr: (v) => {
                const other = new Expr('InfixExpr', v);
                try {
                  throw new Error(`Expected String literal, got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              ExprList: (v) => {
                const other = new Expr('ExprList', v);
                try {
                  throw new Error(`Expected String literal, got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
              Placeholder: (v) => {
                const other = new Expr('Placeholder', v);
                try {
                  throw new Error(`Expected String literal, got ${other.debug()}`)
                } finally {
                  other.drop();
                }
              },
            });
          } finally {
            dropOwned(right);
          }
        } else {
        _v.drop();
        throw new Error('Expected Comparison predicate');
      }
      }
    } finally {
      resolver.drop();
    }
  });

});
