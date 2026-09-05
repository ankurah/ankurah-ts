// MIRRORS: ankurah/core/src/transaction.rs
import { Struct, Drop, Result, Arc, RwLock, HashSet } from '@ankurah/base';
import { EntityId } from '@ankurah/proto';
import { TContext } from './context';
import { Entity } from './entity';
import { MutationError, RetrievalError } from './error';
import { Model, Mutable, MutableBorrow } from './indexel';
import { AccessDenied } from './policy';
import { EntityId, TransactionId } from '@ankurah/proto';

export class Transaction extends Drop {
  dyncontext: Arc<TContext>;
  id: TransactionId;
  entities: AppendOnlyVec<Entity>;
  alive: Arc<boolean>;
  createdEntityIds: RwLock<HashSet<EntityId>>;

  constructor(dyncontext: Arc<TContext>, id: TransactionId, entities: AppendOnlyVec<Entity>, alive: Arc<boolean>, createdEntityIds: RwLock<HashSet<EntityId>>) {
    super();
    this.dyncontext = dyncontext;
    this.id = id;
    this.entities = entities;
    this.alive = alive;
    this.createdEntityIds = createdEntityIds;
  }

  static new(dyncontext: Arc<TContext>): Transaction {
    return new Transaction(dyncontext, proto.TransactionId.new(), AppendOnlyVec.new(), Arc.new(AtomicBool.new(true)), new RwLock(new HashSet()));
  }

  addEntity(entity: Entity): Entity {
    const index = this.entities.push(entity);
    return this.entities.index(index);
  }

  async create<M extends Model>(model: M): Promise<Result<MutableBorrow<Mutable>, MutationError>> {
    let _moved0 = false;
    const entity = this.dyncontext.value.createEntity(M.collection(), this.alive.clone());
    try {
      model.initializeNewEntity(entity);
      const _r1 = this.dyncontext.value.checkWrite(entity);
      if (_r1.isErr()) return Result.Err(MutationError.fromAccessDenied(_r1.unwrapErr()));
      _r1.drop();
      const _t2 = this.createdEntityIds.write();
      try {
        _t2.value.add(entity.deref().id);
      } finally {
        _t2.drop();
      }
      _moved0 = true;
      const entityRef = this.addEntity(entity);
      return Result.Ok(MutableBorrow.new(entityRef));
    } finally {
      if (!_moved0) entity.drop();
    }
  }

  getTrxEntity(id: EntityId): Entity | null {
    return this.entities.iter().find((e) => e.id === id);
  }

  async get<M extends Model>(id: EntityId): Promise<Result<MutableBorrow<Mutable>, RetrievalError>> {
    const _v = this.getTrxEntity(id);
    if (_v != null) {
      const entity = _v;
      return Result.Ok(MutableBorrow.new(entity));
    } else {
      {
        const _r0 = await this.dyncontext.value.getEntity(id, M.collection(), false);
        if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
        const retrievedEntity = _r0.unwrap();
        try {
          {
            const _v1 = this.getTrxEntity(retrievedEntity.deref().id);
            if (_v1 != null) {
              const entity = _v1;
              return Result.Ok(MutableBorrow.new(entity));
            } else {
            return Result.Ok(MutableBorrow.new(this.addEntity(retrievedEntity.snapshot(this.alive.clone()))));
          }
          }
        } finally {
          retrievedEntity.drop();
        }
      }
    }
  }

  edit<M extends Model>(entity: Entity): Result<MutableBorrow<Mutable>, AccessDenied> {
    {
      const _v = this.getTrxEntity(entity.deref().id);
      if (_v != null) {
        const entity = _v;
        return Result.Ok(MutableBorrow.new(entity));
      }
    }
    const _r0 = this.dyncontext.value.checkWrite(entity);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    _r0.drop();
    return Result.Ok(MutableBorrow.new(this.addEntity(entity.snapshot(this.alive.clone()))));
  }

  async commit(): Promise<Result<void, MutationError>> {
    try {
      return await this.dyncontext.value.commitLocalTrx(this);
    } finally {
      this.drop();
    }
  }

  rollback(): void {
    try {
      this.alive.value = false;
    } finally {
      this.drop();
    }
  }

  protected override onDrop(): void {
    this.alive.value = false;
  }
}

