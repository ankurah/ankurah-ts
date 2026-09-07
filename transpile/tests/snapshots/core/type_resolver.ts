// MIRRORS: ankurah/core/src/type_resolver.rs
import { Struct, dropOwned, dropUnbound } from '@ankurah/base';
import { Expr, Literal, PathExpr, Predicate, Selection } from '@ankurah/ankql';
import { Value_castTo } from './value/cast';
import { Value, ValueType } from './value/index';

export class TypeResolver extends Struct {

  static new(): TypeResolver {
    return new TypeResolver();
  }

  resolvePath(path: PathExpr): ValueType | null {
    if (!path.isSimple()) {
      return new ValueType('Json', {});
    }
    if (path.first() === 'id') {
      return new ValueType('EntityId', {});
    }
    return null;
  }

  static literalType(literal: Literal): ValueType {
    return literal.match({
      I16: (v) => new ValueType('I16', {}),
      I32: (v) => new ValueType('I32', {}),
      I64: (v) => new ValueType('I64', {}),
      F64: (v) => new ValueType('F64', {}),
      Bool: (v) => new ValueType('Bool', {}),
      String: (v) => new ValueType('String', {}),
      EntityId: (v) => new ValueType('EntityId', {}),
      Object: (v) => new ValueType('Object', {}),
      Binary: (v) => new ValueType('Binary', {}),
      Json: (v) => new ValueType('Json', {}),
    });
  }

  static literalToJson(literal: Literal): Literal {
    const value = Value.fromRefAstLiteral(literal);
    try {
      const _v = Value_castTo(value, new ValueType('Json', {}));
      if (_v.isOk()) {
        const jsonValue = _v.unwrap();
        return Literal.fromValue(jsonValue);
      } else {
        const _v1 = _v.unwrapErr();
        try {
          return literal.clone();
        } finally {
          _v1.drop();
        }
      }
    } finally {
      value.drop();
    }
  }

  resolveExprType(expr: Expr): ValueType | null {
    return expr.match({
      Path: (v) => {
        const path = v._0;
        return this.resolvePath(path);
      },
      Literal: (v) => {
        const lit = v._0;
        return TypeResolver.literalType(lit);
      },
      Predicate: () => null,
      InfixExpr: () => null,
      ExprList: () => null,
      Placeholder: () => null,
    });
  }

  convertExpr(expr: Expr, targetType: ValueType | null): Expr {
    const _v = [expr, targetType];
    if ((_v[0].is('Literal')) && (_v[1] != null)) {
      const { _0: lit } = _v[0].value;
      const target = _v[1];
      let _moved0 = false;
      try {
        {
          const value = Value.fromAstLiteral((lit));
          try {
            const _v1 = Value_castTo(value, target);
            if (_v1.isOk()) {
              const casted = _v1.unwrap();
              return new Expr('Literal', { _0: Literal.fromValue(casted) });
            } else {
              const _v2 = _v1.unwrapErr();
              try {
                _moved0 = true;
                return new Expr('Literal', { _0: lit });
              } finally {
                _v2.drop();
              }
            }
          } finally {
            value.drop();
          }
        }
      } finally {
        if (!_moved0) lit.drop();
      }
    } else {
      const other = _v[0];
      return other;
    }
  }

  resolveSelectionTypes(selection: Selection): Selection {
    try {
      return new Selection(this.resolveTypes(selection.takeField('predicate')), selection.orderBy, selection.limit);
    } finally {
      selection.drop();
    }
  }

  resolveTypes(predicate: Predicate): Predicate {
    return predicate.intoMatch({
      Comparison: (v) => {
        const left = v.left;
        const operator = v.operator;
        const right = v.right;
        let _moved0 = false;
        let _moved1 = false;
        let _moved2 = false;
        try {
          try {
            try {
              const leftType = this.resolveExprType(left);
              const rightType = this.resolveExprType(right);
              _moved0 = true;
              const newLeft = this.convertExpr(left, rightType);
              _moved2 = true;
              const newRight = this.convertExpr(right, leftType);
              _moved1 = true;
              return new Predicate('Comparison', { left: newLeft, operator: operator, right: newRight });
            } finally {
              if (!_moved2) dropOwned(right);
            }
          } finally {
            if (!_moved1) operator.drop();
          }
        } finally {
          if (!_moved0) dropOwned(left);
        }
      },
      And: (v) => {
        const left = v._0;
        const right = v._1;
        return new Predicate('And', { _0: this.resolveTypes(left), _1: this.resolveTypes(right) });
      },
      Or: (v) => {
        const left = v._0;
        const right = v._1;
        return new Predicate('Or', { _0: this.resolveTypes(left), _1: this.resolveTypes(right) });
      },
      Not: (v) => {
        const inner = v._0;
        return new Predicate('Not', { _0: this.resolveTypes(inner) });
      },
      IsNull: (v) => {
        try {
          return predicate;
        } finally {
          dropUnbound(v, []);
        }
      },
      True: () => predicate,
      False: () => predicate,
      Placeholder: () => predicate,
    });
  }

  clone(): TypeResolver {
    return new TypeResolver();
  }

  static default(): TypeResolver {
    return new TypeResolver();
  }

  debug(): string {
    return 'TypeResolver';
  }
}

