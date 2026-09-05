// MIRRORS: ankurah/ankql/src/conversion.rs
import { Result, dropUnbound } from '@ankurah/base';
import { Expr, Literal, Predicate, Selection } from './ast';
import { ParseError } from './error';
import { parseSelection } from './parser';

export function Predicate_tryFrom(value: string): Result<Predicate, ParseError> {
  const _r0 = parser.parseSelection(value);
  if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
  const _t1 = _r0.unwrap();
  try {
    return Result.Ok(_t1.predicate);
  } finally {
    _t1.drop();
  }
}

export function Selection_tryFrom(value: string): Result<Selection, ParseError> {
  return parser.parseSelection(value);
}

export function Predicate_tryFromExpr(value: Expr): Result<Predicate, ParseError> {
  return value.intoMatch({
    Predicate: (v) => {
      const p = v._0;
      return Result.Ok(p);
    },
    Placeholder: () => Result.Ok(new Predicate('Placeholder', {})),
    Literal: (v) => Result.Ok(new Predicate('True', {})),
    Path: (v) => {
      try {
        return Result.Err(new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }));
      } finally {
        dropUnbound(v, []);
      }
    },
    InfixExpr: (v) => {
      try {
        return Result.Err(new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }));
      } finally {
        dropUnbound(v, []);
      }
    },
    ExprList: (v) => {
      try {
        return Result.Err(new ParseError('InvalidPredicate', { _0: 'Expression is not a predicate' }));
      } finally {
        dropUnbound(v, []);
      }
    },
  });
}

export function Expr_tryFromJsValue(value: unknown): Result<Expr, ParseError> {
  if ((value === null) || (value === undefined)) {
    return Result.Ok(new ast.Expr('Literal', { _0: new ast.Literal('String', { _0: 'NULL_IMPROBABLE_VALUE' }) }));
  }
  {
    const _v = (typeof value === 'string' ? value : null);
    if (_v != null) {
      const s = _v;
      return Result.Ok(new ast.Expr('Literal', { _0: new ast.Literal('String', { _0: s }) }));
    }
  }
  {
    const _v1 = (typeof value === 'boolean' ? value : null);
    if (_v1 != null) {
      const b = _v1;
      return Result.Ok(new ast.Expr('Literal', { _0: new ast.Literal('Bool', { _0: b }) }));
    }
  }
  {
    const _v2 = (typeof value === 'number' ? value : null);
    if (_v2 != null) {
      const n = _v2;
      if (n.fract() === 0.0) {
        const nInt = (($v) => $v < -9223372036854775808n ? -9223372036854775808n : $v > 9223372036854775807n ? 9223372036854775807n : $v)(BigInt(Math.min(Math.max(Math.trunc(n) || 0, -9223372036854775808), 9223372036854775807)));
        return Result.Ok(new ast.Expr('Literal', { _0: new ast.Literal('I64', { _0: nInt }) }));
      } else {
        return Result.Ok(new ast.Expr('Literal', { _0: new ast.Literal('F64', { _0: n }) }));
      }
    }
  }
  return Result.Err(new ParseError('InvalidPredicate', { _0: 'Unsupported JsValue type for conversion to Expr' }));
}

