// MIRRORS: ankurah/core/src/reactor/fetch_gap.rs
import { Struct, Result, Weak, derivedClone } from '@ankurah/base';
import { ComparisonOperator, Expr, Literal, OrderDirection, PathExpr, Predicate, OrderByItem, Selection } from '@ankurah/ankql';
import { NodeAndContext } from '../context';
import { Entity } from '../entity';
import { RetrievalError } from '../error';
import { Comparison } from '../lineage';
import { ContextData, MatchArgs, Node, NodeInner } from '../node';
import { AbstractEntity } from '../reactor';
import { ValueType } from '../value/index';
import { CollectionId, EntityId } from '@ankurah/proto';

export class QueryGapFetcher<SE extends StorageEngine, PA extends PolicyAgent> extends Struct implements GapFetcher<Entity> {
  weakNode: Weak<NodeInner<SE, PA>>;
  cdata: ContextData;

  constructor(weakNode: Weak<NodeInner<SE, PA>>, cdata: ContextData) {
    super();
    this.weakNode = weakNode;
    this.cdata = cdata;
  }

  static new<SE, PA>(node: Node<SE, PA>, cdata: ContextData): QueryGapFetcher<SE, PA> {
    return new QueryGapFetcher(node._0.downgrade(), cdata);
  }

  async fetchGap(collectionId: CollectionId, selection: Selection, lastEntity: Entity | null, gapSize: number): Promise<Result<Entity[], RetrievalError>> {
    const _m0 = this.weakNode.upgrade();
    const _r1 = (_m0 != null ? Result.Ok(_m0!) : Result.Err((() => RetrievalError.storage(io.Error.other('Node has been dropped, cannot fill gap')))()));
    if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
    let _moved2 = false;
    const nodeInner = _r1.unwrap();
    try {
      _moved2 = true;
      const node = new Node(nodeInner);
      const nodeContext = new NodeAndContext(node, this.cdata.clone());
      const _m6 = (() => {
        {
          const _v1 = lastEntity;
          if (_v1 != null) {
            const last = _v1;
            const _m4 = (() => {
              {
                const _v = selection.orderBy;
                if (_v != null) {
                  const orderBy = _v;
                  const _r3 = buildContinuationPredicate(selection.predicate, orderBy, last).mapErr((e) => RetrievalError.storage(io.Error.other(e)));
                  if (_r3.isErr()) return { $jump: 'return', $value: Result.Err(_r3.unwrapErr()) };
                  return _r3.unwrap();
                } else {
                return selection.predicate.clone();
              }
              }
            })();
            if ((_m4 as any)?.$jump === 'return') return _m4;
            let _moved5 = false;
            const gapPredicate = (_m4 as any);
            try {
              _moved5 = true;
              return new Selection(gapPredicate, selection.orderBy.clone(), BigInt(gapSize));
            } finally {
              if (!_moved5) gapPredicate.drop();
            }
          } else {
          return new Selection(selection.predicate.clone(), selection.orderBy.clone(), BigInt(gapSize));
        }
        }
      })();
      if ((_m6 as any)?.$jump === 'return') return (_m6 as any).$value;
      let _moved7 = false;
      const gapSelection = (_m6 as any);
      try {
        _moved7 = true;
        let _moved8 = false;
        const matchArgs = new MatchArgs(gapSelection, false);
        try {
          _moved8 = true;
          return await nodeContext.fetchEntities(collectionId, matchArgs);
        } finally {
          if (!_moved8) matchArgs.drop();
        }
      } finally {
        if (!_moved7) gapSelection.drop();
      }
    } finally {
      if (!_moved2) nodeInner.drop();
    }
  }

  clone(): QueryGapFetcher<SE, PA> {
    return new QueryGapFetcher(this.weakNode.clone(), derivedClone(this.cdata));
  }
}

export interface GapFetcher<E extends AbstractEntity> {
  fetchGap(collectionId: CollectionId, selection: Selection, lastEntity: E | null, gapSize: number): Promise<Result<E[], RetrievalError>>;
}

export function buildContinuationPredicate<E extends AbstractEntity>(originalPredicate: Predicate, orderBy: OrderByItem[], lastEntity: E): Result<Predicate, string> {
  let gapConditions = [];
  gapConditions.push(originalPredicate.clone());
  for (const orderItem of orderBy) {
    const fieldName = orderItem.path.property();
    {
      const _v = lastEntity.value(fieldName);
      if (_v != null) {
        const fieldValue = _v;
        try {
          const _m0 = (() => {
            if (fieldValue.is('String')) {
              const { _0: s } = fieldValue.value;
              return new Literal('String', { _0: s });
            } else if (fieldValue.is('I16')) {
              const { _0: i } = fieldValue.value;
              return new Literal('I16', { _0: i });
            } else if (fieldValue.is('I32')) {
              const { _0: i } = fieldValue.value;
              return new Literal('I32', { _0: i });
            } else if (fieldValue.is('I64')) {
              const { _0: i } = fieldValue.value;
              return new Literal('I64', { _0: i });
            } else if (fieldValue.is('F64')) {
              const { _0: f } = fieldValue.value;
              return new Literal('F64', { _0: f });
            } else if (fieldValue.is('Bool')) {
              const { _0: b } = fieldValue.value;
              return new Literal('Bool', { _0: b });
            } else if (fieldValue.is('EntityId')) {
              const { _0: id } = fieldValue.value;
              return new Literal('EntityId', { _0: Ulid_fromEntityId(id) });
            } else {
              return { $jump: 'continue' };
            }
          })();
          if ((_m0 as any)?.$jump === 'continue') continue;
          let _moved1 = false;
          const literal = (_m0 as any);
          try {
            let _moved2 = false;
            const operator = orderItem.direction.match({
              Asc: () => new ComparisonOperator('GreaterThanOrEqual', {}),
              Desc: () => new ComparisonOperator('LessThanOrEqual', {}),
            });
            try {
              _moved2 = true;
              _moved1 = true;
              let _moved3 = false;
              const condition = new Predicate('Comparison', { left: new Expr('Path', { _0: orderItem.path.clone() }), operator: operator, right: new Expr('Literal', { _0: literal }) });
              try {
                _moved3 = true;
                gapConditions.push(condition);
              } finally {
                if (!_moved3) condition.drop();
              }
            } finally {
              if (!_moved2) operator.drop();
            }
          } finally {
            if (!_moved1) literal.drop();
          }
        } finally {
          fieldValue.drop();
        }
      }
    }
  }
  const idExclusion = new Predicate('Comparison', { left: new Expr('Path', { _0: PathExpr.simple('id') }), operator: new ComparisonOperator('NotEqual', {}), right: new Expr('Literal', { _0: new Literal('EntityId', { _0: Ulid_fromEntityId((lastEntity.id())) }) }) });
  gapConditions.push(idExclusion);
  const result = [...gapConditions].reduce((acc, condition) => new Predicate('And', { _0: acc, _1: condition })).unwrapOr(new Predicate('True', {}));
  return Result.Ok(result);
}

export function inferValueTypeForField<E extends AbstractEntity>(entities: E[], fieldName: string): ValueType {
  for (const entity of entities) {
    {
      const _v = entity.value(fieldName);
      if (_v != null) {
        const value = _v;
        try {
          return ValueType.of(value);
        } finally {
          value.drop();
        }
      }
    }
  }
  return new ValueType('String', {});
}

