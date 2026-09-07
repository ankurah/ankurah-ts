// MIRRORS: ankurah/core/src/value/cast_predicate.rs
import { Result, dropOwned, unsupported } from '@ankurah/base';
import { Expr, Literal, Predicate } from '@ankurah/ankql';
import { RetrievalError } from '../error';
import { CollectionSchema } from '../schema';
import { Value_castTo } from './cast';
import { Value, ValueType } from './index';

export function castPredicateTypes<S extends CollectionSchema>(predicate: Predicate, schema: S): Result<Predicate, RetrievalError> {
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
            const _v = [left.asRef(), right.asRef()];
            if ((_v[0].is('Path')) && (_v[1].is('Literal'))) {
              const { _0: path } = _v[0].value;
              const { _0: literal } = _v[1].value;
              {
                const _r3 = schema.fieldType(path);
                if (_r3.isErr()) return Result.Err(RetrievalError.fromPropertyError(_r3.unwrapErr()));
                const targetType = _r3.unwrap();
                const _r4 = castLiteralToType(literal.clone(), targetType);
                if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
                let _moved5 = false;
                const castLiteral = _r4.unwrap();
                try {
                  _moved0 = true;
                  _moved1 = true;
                  _moved5 = true;
                  return Result.Ok(new Predicate('Comparison', { left: left, operator: operator, right: castLiteral }));
                } finally {
                  if (!_moved5) castLiteral.drop();
                }
              }
            } else if ((_v[0].is('Literal')) && (_v[1].is('Path'))) {
              const { _0: literal } = _v[0].value;
              const { _0: path } = _v[1].value;
              {
                const _r6 = schema.fieldType(path);
                if (_r6.isErr()) return Result.Err(RetrievalError.fromPropertyError(_r6.unwrapErr()));
                const targetType = _r6.unwrap();
                const _r7 = castLiteralToType(literal.clone(), targetType);
                if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                let _moved8 = false;
                const castLiteral = _r7.unwrap();
                try {
                  _moved1 = true;
                  _moved2 = true;
                  _moved8 = true;
                  return Result.Ok(new Predicate('Comparison', { left: castLiteral, operator: operator, right: right }));
                } finally {
                  if (!_moved8) castLiteral.drop();
                }
              }
            } else {
              {
                _moved0 = true;
                const _r9 = castExprTypes(left, schema);
                if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                let _moved10 = false;
                const castLeft = _r9.unwrap();
                try {
                  _moved2 = true;
                  const _r11 = castExprTypes(right, schema);
                  if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
                  let _moved12 = false;
                  const castRight = _r11.unwrap();
                  try {
                    _moved1 = true;
                    _moved10 = true;
                    _moved12 = true;
                    return Result.Ok(new Predicate('Comparison', { left: castLeft, operator: operator, right: castRight }));
                  } finally {
                    if (!_moved12) castRight.drop();
                  }
                } finally {
                  if (!_moved10) castLeft.drop();
                }
              }
            }
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
    IsNull: (v) => {
      const expr = v._0;
      const _r13 = castExprTypes(expr, schema);
      if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
      return Result.Ok(new Predicate('IsNull', { _0: _r13.unwrap() }));
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      let _moved14 = false;
      let _moved15 = false;
      try {
        try {
          _moved14 = true;
          const _r16 = castPredicateTypes(left, schema);
          if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
          try {
            _moved15 = true;
            const _r17 = castPredicateTypes(right, schema);
            if (_r17.isErr()) return Result.Err(_r17.unwrapErr());
            try {
              return Result.Ok(new Predicate('And', { _0: _r16.unwrap(), _1: _r17.unwrap() }));
            } finally {
              if (_r17 != null && !(_r17 as any).isMoved && !(_r17 as any).isDropped) dropOwned(_r17);
            }
          } finally {
            if (_r16 != null && !(_r16 as any).isMoved && !(_r16 as any).isDropped) dropOwned(_r16);
          }
        } finally {
          if (!_moved15) dropOwned(right);
        }
      } finally {
        if (!_moved14) dropOwned(left);
      }
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      let _moved18 = false;
      let _moved19 = false;
      try {
        try {
          _moved18 = true;
          const _r20 = castPredicateTypes(left, schema);
          if (_r20.isErr()) return Result.Err(_r20.unwrapErr());
          try {
            _moved19 = true;
            const _r21 = castPredicateTypes(right, schema);
            if (_r21.isErr()) return Result.Err(_r21.unwrapErr());
            try {
              return Result.Ok(new Predicate('Or', { _0: _r20.unwrap(), _1: _r21.unwrap() }));
            } finally {
              if (_r21 != null && !(_r21 as any).isMoved && !(_r21 as any).isDropped) dropOwned(_r21);
            }
          } finally {
            if (_r20 != null && !(_r20 as any).isMoved && !(_r20 as any).isDropped) dropOwned(_r20);
          }
        } finally {
          if (!_moved19) dropOwned(right);
        }
      } finally {
        if (!_moved18) dropOwned(left);
      }
    },
    Not: (v) => {
      const pred = v._0;
      const _r22 = castPredicateTypes(pred, schema);
      if (_r22.isErr()) return Result.Err(_r22.unwrapErr());
      return Result.Ok(new Predicate('Not', { _0: _r22.unwrap() }));
    },
    True: () => Result.Ok(predicate),
    False: () => Result.Ok(predicate),
    Placeholder: () => Result.Ok(predicate),
  });
}

function castExprTypes<S extends CollectionSchema>(expr: Expr, schema: S): Result<Expr, RetrievalError> {
  return expr.intoMatch({
    Literal: (v) => {
      const literal = v._0;
      return Result.Ok(new Expr('Literal', { _0: literal }));
    },
    Path: (v) => {
      const path = v._0;
      return Result.Ok(new Expr('Path', { _0: path }));
    },
    Predicate: (v) => {
      const predicate = v._0;
      const _r0 = castPredicateTypes(predicate, schema);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      return Result.Ok(new Expr('Predicate', { _0: _r0.unwrap() }));
    },
    InfixExpr: (v) => {
      const left = v.left;
      const operator = v.operator;
      const right = v.right;
      let _moved1 = false;
      try {
        const _r2 = castExprTypes(left, schema);
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        try {
          const _r3 = castExprTypes(right, schema);
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          try {
            _moved1 = true;
            return Result.Ok(new Expr('InfixExpr', { left: _r2.unwrap(), operator: operator, right: _r3.unwrap() }));
          } finally {
            if (_r3 != null && !(_r3 as any).isMoved && !(_r3 as any).isDropped) dropOwned(_r3);
          }
        } finally {
          if (_r2 != null && !(_r2 as any).isMoved && !(_r2 as any).isDropped) dropOwned(_r2);
        }
      } finally {
        if (!_moved1) operator.drop();
      }
    },
    ExprList: (v) => {
      const exprs = v._0;
      try {
        const _r5 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
        const castExprs = _r5;
        return Result.Ok(new Expr('ExprList', { _0: castExprs }));
      } finally {
        dropOwned(exprs);
      }
    },
    Placeholder: () => Result.Ok(new Expr('Placeholder', {})),
  });
}

function castLiteralToType(literal: Literal, targetType: ValueType): Result<Expr, RetrievalError> {
  const value = Value.fromAstLiteral(literal);
  try {
    const _r0 = Value_castTo(value, targetType).mapErr((e) => {
      try {
        return new RetrievalError('StorageError', { _0: `Type casting error: ${e}` });
      } finally {
        e.drop();
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    let _moved1 = false;
    const castValue = _r0.unwrap();
    try {
      _moved1 = true;
      let _moved2 = false;
      const castLiteral = Literal.fromValue(castValue);
      try {
        _moved2 = true;
        return Result.Ok(new Expr('Literal', { _0: castLiteral }));
      } finally {
        if (!_moved2) castLiteral.drop();
      }
    } finally {
      if (!_moved1) castValue.drop();
    }
  } finally {
    value.drop();
  }
}

