// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs
import { Struct, Arc, dropOwned, derivedClone, HashMap } from '@ankurah/base';
import { QueryId } from '@ankurah/proto';
import { IVec } from '../util/ivec';

export class CandidateChanges<C> extends Struct {
  changes: Arc<C[]>;
  queryOffsets: HashMap<QueryId, IVec<number>>;
  entityOffsets: IVec<number>;

  constructor(changes: Arc<C[]>, queryOffsets: HashMap<QueryId, IVec<number>>, entityOffsets: IVec<number>) {
    super();
    this.changes = changes;
    this.queryOffsets = queryOffsets;
    this.entityOffsets = entityOffsets;
  }

  static new<C>(changes: Arc<C[]>): CandidateChanges<C> {
    let _moved0 = false;
    try {
      const _b1 = new HashMap<QueryId, IVec<number>>();
      const _b2 = IVec.new();
      _moved0 = true;
      return new CandidateChanges(changes, _b1, _b2);
    } finally {
      if (!_moved0) changes.drop();
    }
  }

  addEntity(offset: number): void {
    this.entityOffsets.push(offset);
  }

  addQuery(queryId: QueryId, offset: number): void {
    this.queryOffsets.entry(queryId).orDefault(() => IVec.default()).value.add(offset);
  }

  isEmpty(): boolean {
    return this.queryOffsets.size === 0 && this.entityOffsets.isEmpty();
  }

  queryCount(): number {
    return this.queryOffsets.size;
  }

  queryIter(): QueryCandidate<C>[] {
    return [...this.queryOffsets].map(([queryId, offsets]) => new QueryCandidate(queryId, this.changes, offsets.asSlice()));
  }

  entityIter(): C[] {
    return this.entityOffsets.iter().map((offset) => this.changes[offset]);
  }

  changes(): Arc<C[]> {
    return this.changes;
  }

  clone(): CandidateChanges<C> {
    let _moved1 = false;
    const _b0 = this.changes.value.map((e) => derivedClone(e));
    try {
      const _b2 = this.queryOffsets.clone();
      const _b3 = this.entityOffsets.clone();
      _moved1 = true;
      return new CandidateChanges(_b0, _b2, _b3);
    } finally {
      if (!_moved1) dropOwned(_b0);
    }
  }
}

export class QueryCandidate<C> extends Struct {
  readonly queryId: QueryId;
  changes: Arc<C[]>;
  offsets: number[];

  constructor(queryId: QueryId, changes: Arc<C[]>, offsets: number[]) {
    super();
    this.queryId = queryId;
    this.changes = changes;
    this.offsets = offsets;
  }

  // A `&T` field is a borrow: dropping this releases the borrow and nothing
  // else, so the cascade must not walk it.
  protected override ownedFields(): unknown[] {
    return [];
  }

  iter(): C[] {
    return [...this.offsets].map((offset) => this.changes[offset]);
  }
}

