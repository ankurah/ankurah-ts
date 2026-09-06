// MIRRORS: ankurah/core/src/selection/filter.rs
import { Struct, Enum, Result, invokeRef, dropOwned, derivedEquals, derivedClone } from '@ankurah/base';
import { ComparisonOperator, Expr, Predicate, Literal } from '@ankurah/ankql';
import { Comparison } from '../lineage';
import { Value_castTo } from '../value/cast';
import { Value, ValueType } from '../value/index';

export class FilterIterator<I extends Iterator> extends Struct {
  iter: I;
  predicate: Predicate;

  constructor(iter: I, predicate: Predicate) {
    super();
    this.iter = iter;
    this.predicate = predicate;
  }

  static new<I>(iter: I, predicate: Predicate): FilterIterator<I> {
    return new FilterIterator(iter, predicate);
  }

  next<R>(): FilterResult<R> | null {
    const _m0 = this.iter.next();
    return (_m0 != null ? ((item) => (() => {
      const _v4 = evaluatePredicate(item, this.predicate);
      if (_v4.isOk()) {
        const _v5 = _v4.unwrap();
        if (_v5 === true) {
          const _v6 = _v5;
          return new FilterResult('Pass', { _0: item });
        }
        {
          const _v7 = _v5;
          return new FilterResult('Skip', { _0: item });
        }
      } else {
        const e = _v4.unwrapErr();
        return new FilterResult('Error', { _0: item, _1: e });
      }
    })())(_m0!) : null);
  }
}

export type ErrorV = {
  CollectionMismatch: { expected: string; actual: string };
  PropertyNotFound: { _0: string };
  UnsupportedExpression: { _0: string };
  UnsupportedOperator: { _0: string };
};

export class Error extends Enum<ErrorV> {

  equals(other: Error): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'CollectionMismatch': {
        if ((this.value as any).expected !== (other.value as any).expected) return false;
        if ((this.value as any).actual !== (other.value as any).actual) return false;
        break;
      }
      case 'PropertyNotFound': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'UnsupportedExpression': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
      case 'UnsupportedOperator': {
        if ((this.value as any)._0 !== (other.value as any)._0) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      CollectionMismatch: (v) => `CollectionMismatch { expected: ${JSON.stringify(v.expected)}, actual: ${JSON.stringify(v.actual)} }`,
      PropertyNotFound: (v) => `PropertyNotFound(${JSON.stringify(v._0)})`,
      UnsupportedExpression: (v) => `UnsupportedExpression(${JSON.stringify(v._0)})`,
      UnsupportedOperator: (v) => `UnsupportedOperator(${JSON.stringify(v._0)})`,
    });
  }

  override toString(): string {
    return this.match({
      CollectionMismatch: (v) => `collection mismatch: expected ${v.expected}, got ${v.actual}`,
      PropertyNotFound: (v) => `property not found: ${v._0}`,
      UnsupportedExpression: (v) => `Unsupported expression: ${v._0}`,
      UnsupportedOperator: (v) => `Unsupported operator: ${v._0}`,
    });
  }
}

export type ExprOutputV<T> = {
  List: { _0: ExprOutput<T>[] };
  Value: { _0: T };
  None: {};
};

export class ExprOutput<T> extends Enum<ExprOutputV<T>> {

  asValue(): T | null {
    return this.match({
      Value: (_v) => {
        const v = _v._0;
        return v;
      },
      List: () => null,
      None: () => null,
    });
  }

  asList(): ExprOutput<T>[] | null {
    return this.match({
      List: (v) => {
        const l = v._0;
        return l;
      },
      Value: () => null,
      None: () => null,
    });
  }

  isNone(): boolean {
    return this.is('None');
  }

  clone(): ExprOutput<T> {
    return this.match({
      List: (v) => new ExprOutput<T>('List', { _0: v._0.map(e => e.clone()) }),
      Value: (v) => new ExprOutput<T>('Value', { _0: derivedClone(v._0) }),
      None: () => new ExprOutput<T>('None', {}),
    });
  }

  equals(other: ExprOutput<T>): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'List': {
        { if ((this.value as any)._0.length !== (other.value as any)._0.length) return false; for (let i = 0; i < (this.value as any)._0.length; i++) { if (!(this.value as any)._0[i].equals((other.value as any)._0[i])) return false; } }
        break;
      }
      case 'Value': {
        if (!derivedEquals((this.value as any)._0, (other.value as any)._0)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      List: (v) => `List(${`[${Array.from(v._0).map((e) => e.debug()).join(', ')}]`})`,
      Value: (v) => `Value(${v._0})`,
      None: () => 'None',
    });
  }
}

export type FilterResultV<R> = {
  Pass: { _0: R };
  Skip: { _0: R };
  Error: { _0: R; _1: Error };
};

export class FilterResult<R> extends Enum<FilterResultV<R>> {

  equals(other: FilterResult<R>): boolean {
    if (this.type !== other.type) return false;
    switch (this.type) {
      case 'Pass': {
        if (!derivedEquals((this.value as any)._0, (other.value as any)._0)) return false;
        break;
      }
      case 'Skip': {
        if (!derivedEquals((this.value as any)._0, (other.value as any)._0)) return false;
        break;
      }
      case 'Error': {
        if (!derivedEquals((this.value as any)._0, (other.value as any)._0)) return false;
        if (!(this.value as any)._1.equals((other.value as any)._1)) return false;
        break;
      }
    }
    return true;
  }

  debug(): string {
    return this.match({
      Pass: (v) => `Pass(${v._0})`,
      Skip: (v) => `Skip(${v._0})`,
      Error: (v) => `Error(${v._0}, ${v._1.debug()})`,
    });
  }
}

export interface Filterable {
  collection(): string;
  value(name: string): Value | null;
}

function evaluateExpr<I extends Filterable>(item: I, expr: Expr): Result<ExprOutput<Value>, Error> {
  return expr.match({
    Placeholder: () => Result.Err(new Error('PropertyNotFound', { _0: 'Placeholder values must be replaced before filtering' })),
    Literal: (v) => {
      const lit = v._0;
      return Result.Ok(new ExprOutput('Value', { _0: lit.clone() }));
    },
    Path: (v) => {
      const path = v._0;
      if (path.isSimple()) {
        const name = path.first();
        const _m0 = item.value(name);
        const _r1 = (_m0 != null ? Result.Ok(_m0!) : Result.Err((() => new Error('PropertyNotFound', { _0: name }))()));
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        return Result.Ok(new ExprOutput('Value', { _0: _r1.unwrap() }));
      } else {
        const first = path.first();
        if (first === item.collection()) {
          const remaining = path.steps.slice(1);
          if (remaining.length === 1) {
            const name = remaining[0];
            const _m2 = item.value(name);
            const _r3 = (_m2 != null ? Result.Ok(_m2!) : Result.Err((() => new Error('PropertyNotFound', { _0: name }))()));
            if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
            return Result.Ok(new ExprOutput('Value', { _0: _r3.unwrap() }));
          }
          const propertyName = remaining[0];
          const subPath = remaining.slice(1);
          return evaluateSubPath(item, propertyName, subPath);
        }
        const propertyName = first;
        const subPath = [...path.steps.slice(1)].map((s) => s);
        return evaluateSubPath(item, propertyName, subPath);
      }
    },
    ExprList: (v) => {
      const exprs = v._0;
      let result = [];
      for (const expr of exprs) {
        const _r4 = evaluateExpr(item, expr);
        if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
        result.push(_r4.unwrap());
      }
      return Result.Ok(new ExprOutput('List', { _0: result }));
    },
    Predicate: () => Result.Err(new Error('UnsupportedExpression', { _0: 'Only literal, path, and list expressions are supported' })),
    InfixExpr: () => Result.Err(new Error('UnsupportedExpression', { _0: 'Only literal, path, and list expressions are supported' })),
  });
}

function evaluateSubPath<I extends Filterable>(item: I, propertyName: string, subPath: string[]): Result<ExprOutput<Value>, Error> {
  const _m0 = item.value(propertyName);
  const _r1 = (_m0 != null ? Result.Ok(_m0!) : Result.Err((() => new Error('PropertyNotFound', { _0: propertyName }))()));
  if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
  const propertyValue = _r1.unwrap();
  try {
    const path = [...subPath].map((s) => s.asRef());
    const _m2 = propertyValue.extractAtPath(path);
    const _m3 = (_m2 != null ? (ExprOutput.Value)(_m2!) : null);
    return (_m3 != null ? Result.Ok(_m3!) : Result.Err((() => {
      return new Error('PropertyNotFound', { _0: `Sub-path '${[...subPath].map((s) => s.asRef()).join('.')}' not found in property '${propertyName}'` });
    })()));
  } finally {
    propertyValue.drop();
  }
}

function compareValuesWithCast(left: Value, right: Value, op: (arg0: Value, arg1: Value) => boolean): boolean {
  try {
    if (ValueType.of(left).equals(ValueType.of(right))) {
      return invokeRef(op, left, right);
    }
    {
      const _v = Value_castTo(right, ValueType.of(left));
      if (_v.isOk()) {
        const castedRight = _v.unwrap();
        try {
          return invokeRef(op, left, castedRight);
        } finally {
          castedRight.drop();
        }
      } else {
      _v.drop();
    }
    }
    {
      const _v1 = Value_castTo(left, ValueType.of(right));
      if (_v1.isOk()) {
        const castedLeft = _v1.unwrap();
        try {
          return invokeRef(op, castedLeft, right);
        } finally {
          castedLeft.drop();
        }
      } else {
      _v1.drop();
    }
    }
    return false;
  } finally {
    dropOwned(op);
  }
}

export function evaluatePredicate<I extends Filterable>(item: I, predicate: Predicate): Result<boolean, Error> {
  return predicate.match({
    Comparison: (v) => {
      const left = v.left;
      const operator = v.operator;
      const right = v.right;
      const _r0 = evaluateExpr(item, left);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const leftVal = _r0.unwrap();
      try {
        const _r1 = evaluateExpr(item, right);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        const rightVal = _r1.unwrap();
        try {
          const _m27 = (() => {
            return operator.match<any>({
              Equal: () => {
                const _m2 = leftVal.asValue().zip(rightVal.asValue());
                return (_m2 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => a.equals(b)))(_m2!) : null) ?? false;
              },
              NotEqual: () => {
                const _m5 = leftVal.asValue().zip(rightVal.asValue());
                return (_m5 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => !a.equals(b)))(_m5!) : null) ?? false;
              },
              GreaterThan: () => {
                const _m8 = leftVal.asValue().zip(rightVal.asValue());
                return (_m8 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => ((a.partialCompareTo(b) ?? NaN) > 0)))(_m8!) : null) ?? false;
              },
              GreaterThanOrEqual: () => {
                const _m11 = leftVal.asValue().zip(rightVal.asValue());
                return (_m11 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => ((a.partialCompareTo(b) ?? NaN) >= 0)))(_m11!) : null) ?? false;
              },
              LessThan: () => {
                const _m14 = leftVal.asValue().zip(rightVal.asValue());
                return (_m14 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => ((a.partialCompareTo(b) ?? NaN) < 0)))(_m14!) : null) ?? false;
              },
              LessThanOrEqual: () => {
                const _m17 = leftVal.asValue().zip(rightVal.asValue());
                return (_m17 != null ? (([l, r]) => compareValuesWithCast(l, r, (a, b) => ((a.partialCompareTo(b) ?? NaN) <= 0)))(_m17!) : null) ?? false;
              },
              In: () => {
                const _m20 = leftVal.asValue();
                const _r21 = (_m20 != null ? Result.Ok(_m20!) : Result.Err((() => new Error('PropertyNotFound', { _0: 'Expected single value for IN left operand' }))()));
                if (_r21.isErr()) return { $jump: 'return', $value: Result.Err(_r21.unwrapErr()) };
                const value = _r21.unwrap();
                const _m22 = rightVal.asList();
                const _r23 = (_m22 != null ? Result.Ok(_m22!) : Result.Err((() => new Error('PropertyNotFound', { _0: 'Expected list for IN right operand' }))()));
                if (_r23.isErr()) return { $jump: 'return', $value: Result.Err(_r23.unwrapErr()) };
                const list = _r23.unwrap();
                return [...list].some((item) => {
                  const _m24 = item.asValue();
                  return (_m24 != null ? ((v) => compareValuesWithCast(value, v, (a, b) => a.equals(b)))(_m24!) : null) ?? false;
                });
              },
              Between: () => {
                return { $jump: 'return', $value: Result.Err(new Error('UnsupportedOperator', { _0: 'BETWEEN operator not yet supported' })) }
              },
            });
          })();
          if ((_m27 as any)?.$jump === 'return') return (_m27 as any).$value;
          return Result.Ok((_m27 as any));
        } finally {
          rightVal.drop();
        }
      } finally {
        leftVal.drop();
      }
    },
    And: (v) => {
      const left = v._0;
      const right = v._1;
      const _r28 = evaluatePredicate(item, left);
      if (_r28.isErr()) return Result.Err(_r28.unwrapErr());
      const _r29 = evaluatePredicate(item, right);
      if (_r29.isErr()) return Result.Err(_r29.unwrapErr());
      return Result.Ok(_r28.unwrap() && _r29.unwrap());
    },
    Or: (v) => {
      const left = v._0;
      const right = v._1;
      const _r30 = evaluatePredicate(item, left);
      if (_r30.isErr()) return Result.Err(_r30.unwrapErr());
      const _r31 = evaluatePredicate(item, right);
      if (_r31.isErr()) return Result.Err(_r31.unwrapErr());
      return Result.Ok(_r30.unwrap() || _r31.unwrap());
    },
    Not: (v) => {
      const pred = v._0;
      const _r32 = evaluatePredicate(item, pred);
      if (_r32.isErr()) return Result.Err(_r32.unwrapErr());
      return Result.Ok(!_r32.unwrap());
    },
    IsNull: (v) => {
      const expr = v._0;
      const _r33 = evaluateExpr(item, expr);
      if (_r33.isErr()) return Result.Err(_r33.unwrapErr());
      const _t34 = _r33.unwrap();
      try {
        return Result.Ok(_t34.isNone());
      } finally {
        _t34.drop();
      }
    },
    True: () => Result.Ok(true),
    False: () => Result.Ok(false),
    Placeholder: () => Result.Err(new Error('PropertyNotFound', { _0: 'Placeholder must be transformed before filtering' })),
  });
}

