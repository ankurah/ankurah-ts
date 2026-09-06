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
                const _r9 = castExprTypes(left, schema);
                if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                let _moved10 = false;
                const castLeft = _r9.unwrap();
                try {
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
      try {
        const _r13 = castExprTypes(expr, schema);
        if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
        return Result.Ok(new Predicate('IsNull', { _0: _r13.unwrap() }));
      } finally {
        dropOwned(expr);
      }
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      try {
        try {
          const _r14 = castPredicateTypes(left, schema);
          if (_r14.isErr()) return Result.Err(_r14.unwrapErr());
          const _r15 = castPredicateTypes(right, schema);
          if (_r15.isErr()) return Result.Err(_r15.unwrapErr());
          return Result.Ok(new Predicate('And', { _0: _r14.unwrap(), _1: _r15.unwrap() }));
        } finally {
          dropOwned(right);
        }
      } finally {
        dropOwned(left);
      }
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      try {
        try {
          const _r16 = castPredicateTypes(left, schema);
          if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
          const _r17 = castPredicateTypes(right, schema);
          if (_r17.isErr()) return Result.Err(_r17.unwrapErr());
          return Result.Ok(new Predicate('Or', { _0: _r16.unwrap(), _1: _r17.unwrap() }));
        } finally {
          dropOwned(right);
        }
      } finally {
        dropOwned(left);
      }
    },
    Not: (v) => {
      const pred = v._0;
      try {
        const _r18 = castPredicateTypes(pred, schema);
        if (_r18.isErr()) return Result.Err(_r18.unwrapErr());
        return Result.Ok(new Predicate('Not', { _0: _r18.unwrap() }));
      } finally {
        dropOwned(pred);
      }
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
      try {
        try {
          const _r1 = castExprTypes(left, schema);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          const _r2 = castExprTypes(right, schema);
          if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
          return Result.Ok(new Expr('InfixExpr', { left: _r1.unwrap(), operator: operator, right: _r2.unwrap() }));
        } finally {
          dropOwned(right);
        }
      } finally {
        dropOwned(left);
      }
    },
    ExprList: (v) => {
      const exprs = v._0;
      try {
        const _r4 = unsupported('`collect` into `Result<unknown[], unknown>` is a `FromIterator` the port has no construction for');
        const castExprs = _r4;
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
    const _r0 = Value_castTo(value, targetType).mapErr((e) => new RetrievalError('StorageError', { _0: `Type casting error: ${e}` }));
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

