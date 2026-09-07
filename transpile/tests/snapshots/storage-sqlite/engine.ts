// MIRRORS: ankurah/storage/sqlite/src/engine.rs
import { Struct, Result, Arc, RwLock, OwnedClosure, serde_json, dropOwned, tracing, checkedAdd, iterFilterMap, range, debugString, HashMap, HashSet, AsyncMutex, tokio } from '@ankurah/base';
import { MutationError, RetrievalError, StorageCollection, StorageEngine, TemporaryEntity, State, backendFromString, evaluatePredicate } from '@ankurah/core';
import { AttestationSet, Attested, Clock, CollectionId, EntityId, EntityState, Event, EventId, OperationSet, State, StateBuffers } from '@ankurah/proto';
import { PooledConnection, SqliteConnectionManager } from './connection';
import { SqliteError } from './error';
import { SqlBuilder, splitPredicateForSqlite } from './sql_builder';
import { Predicate, Selection } from '@ankurah/ankql';

export class SqliteStorageEngine extends Struct implements StorageEngine {
  pool: Pool<SqliteConnectionManager>;

  constructor(pool: Pool<SqliteConnectionManager>) {
    super();
    this.pool = pool;
  }

  static new(pool: Pool<SqliteConnectionManager>): SqliteStorageEngine {
    return new SqliteStorageEngine(pool);
  }

  static async open(path: Path): Promise<Result<SqliteStorageEngine, Error>> {
    const manager = SqliteConnectionManager.file(path.asRef());
    const _r0 = await bb8.Pool.builder().maxSize(DEFAULT_POOL_SIZE).build(manager);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const pool = _r0.unwrap();
    return Result.Ok(SqliteStorageEngine.new(pool));
  }

  static async openInMemory(): Promise<Result<SqliteStorageEngine, Error>> {
    const manager = SqliteConnectionManager.memory();
    const _r0 = await bb8.Pool.builder().maxSize(1).build(manager);
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const pool = _r0.unwrap();
    return Result.Ok(SqliteStorageEngine.new(pool));
  }

  static saneName(collection: string): boolean {
    for (const char of [...collection]) {
      _match0: {
        {
          const c = char;
          if (c.isAlphanumeric()) {

            break _match0;
          }
        }
        if ((char === '_') || (char === '.') || (char === ':')) {

          break _match0;
        }
        {
          return false
        }
      }
    }
    return true;
  }

  pool(): Pool<SqliteConnectionManager> {
    return this.pool;
  }

  async collection(collectionId: CollectionId): Promise<Result<Arc<StorageCollection>, RetrievalError>> {
    if (!SqliteStorageEngine.saneName(collectionId.asStr())) {
      return Result.Err(new RetrievalError('InvalidBucketName', {}));
    }
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new SqliteError('Pool', { _0: e.toString() });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      let _moved1 = false;
      const bucket = SqliteBucket.new(this.pool.clone(), collectionId.clone());
      try {
        const collectionIdClone = collectionId.clone();
        const _r4 = await conn.withConnection(new OwnedClosure([collectionIdClone], (c: Connection) => {
          const _r2 = createStateTable(c, collectionIdClone);
          if (_r2.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r2.unwrapErr()));
          _r2.drop();
          const _r3 = createEventTable(c, collectionIdClone);
          if (_r3.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r3.unwrapErr()));
          _r3.drop();
          return Result.Ok([]);
        }));
        if (_r4.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r4.unwrapErr()));
        _r4.drop();
        const _r5 = await bucket.rebuildColumnsCache(conn);
        if (_r5.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r5.unwrapErr()));
        _r5.drop();
        _moved1 = true;
        return Result.Ok(Arc.new(bucket));
      } finally {
        if (!_moved1) bucket.drop();
      }
    } finally {
      dropOwned(conn);
    }
  }

  async deleteAllCollections(): Promise<Result<boolean, MutationError>> {
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new MutationError('General', { _0: new SqliteError('Pool', { _0: e.toString() }) });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      return (await conn.withConnection((c) => {
        const _r1 = c.prepare('SELECT name FROM sqlite_master WHERE type=\'table\' AND name NOT LIKE \'sqlite_%\'');
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        let stmt = _r1.unwrap();
        const _r2 = stmt.queryMap([], (row) => row.get(0));
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        const tables = iterFilterMap(_r2.unwrap(), (r) => r.ok());
        if (tables.length === 0) {
          return Result.Ok(false);
        }
        for (const table of tables) {
          const _r3 = c.execute(`DROP TABLE IF EXISTS "${table}"`, []);
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          _r3.drop();
        }
        return Result.Ok(true);
      })).mapErr((e) => new MutationError('General', { _0: e }));
    } finally {
      dropOwned(conn);
    }
  }
}

export class SqliteColumn extends Struct {
  readonly name: string;
  readonly dataType: string;

  constructor(name: string, dataType: string) {
    super();
    this.name = name;
    this.dataType = dataType;
  }

  clone(): SqliteColumn {
    return new SqliteColumn(this.name, this.dataType);
  }

  debug(): string {
    return `SqliteColumn { name: ${debugString(this.name)}, dataType: ${debugString(this.dataType)} }`;
  }
}

export class SqliteBucket extends Struct implements StorageCollection {
  pool: Pool<SqliteConnectionManager>;
  collectionId: CollectionId;
  stateTableName: string;
  eventTableName: string;
  columns: Arc<RwLock<SqliteColumn[]>>;
  ddlLock: Arc<AsyncMutex<void>>;

  constructor(pool: Pool<SqliteConnectionManager>, collectionId: CollectionId, stateTableName: string, eventTableName: string, columns: Arc<RwLock<SqliteColumn[]>>, ddlLock: Arc<AsyncMutex<void>>) {
    super();
    this.pool = pool;
    this.collectionId = collectionId;
    this.stateTableName = stateTableName;
    this.eventTableName = eventTableName;
    this.columns = columns;
    this.ddlLock = ddlLock;
  }

  static new(pool: Pool<SqliteConnectionManager>, collectionId: CollectionId): SqliteBucket {
    let _moved0 = false;
    let _moved1 = false;
    try {
      try {
        const stateTableName = collectionId.asStr();
        const eventTableName = `${collectionId.asStr()}_event`;
        const _b2 = Arc.new(new RwLock([]));
        const _b3 = Arc.new(tokio.sync.Mutex.new([]));
        _moved0 = true;
        _moved1 = true;
        return new SqliteBucket(pool, collectionId, stateTableName, eventTableName, _b2, _b3);
      } finally {
        if (!_moved1) collectionId.drop();
      }
    } finally {
      if (!_moved0) dropOwned(pool);
    }
  }

  stateTable(): string {
    return this.stateTableName;
  }

  eventTable(): string {
    return this.eventTableName;
  }

  existingColumns(): string[] {
    const columns = this.columns.value.read();
    try {
      return [...columns.value].map((c) => c.name);
    } finally {
      columns.drop();
    }
  }

  hasColumn(name: string): boolean {
    const columns = this.columns.value.read();
    try {
      return [...columns.value].some((c) => c.name === name);
    } finally {
      columns.drop();
    }
  }

  async rebuildColumnsCache(conn: PooledConnection): Promise<Result<void, SqliteError>> {
    const tableName = this.stateTable();
    const _r5 = await conn.withConnection((c) => {
      const _r0 = c.prepare(`PRAGMA table_info("${tableName}")`);
      if (_r0.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r0.unwrapErr()));
      let stmt = _r0.unwrap();
      const _r3 = stmt.queryMap([], (row) => {
        const _r1 = row.get(1);
        if (_r1.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r1.unwrapErr()));
        try {
          const _r2 = row.get(2);
          if (_r2.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r2.unwrapErr()));
          try {
            return Result.Ok(new SqliteColumn(_r1.unwrap(), _r2.unwrap()));
          } finally {
            if (_r2 != null && !(_r2 as any).isMoved && !(_r2 as any).isDropped) dropOwned(_r2);
          }
        } finally {
          if (_r1 != null && !(_r1 as any).isMoved && !(_r1 as any).isDropped) dropOwned(_r1);
        }
      });
      if (_r3.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r3.unwrapErr()));
      let _moved4 = false;
      const columns = iterFilterMap(_r3.unwrap(), (r) => r.ok());
      try {
        _moved4 = true;
        return Result.Ok(columns);
      } finally {
        if (!_moved4) dropOwned(columns);
      }
    });
    if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
    const newColumns = _r5.unwrap();
    let columns = this.columns.value.write();
    try {
      columns.value = newColumns;
      return Result.Ok([]);
    } finally {
      columns.drop();
    }
  }

  async addMissingColumns(conn: PooledConnection, missing: [string, string][]): Promise<Result<void, SqliteError>> {
    if (missing.length === 0) {
      return Result.Ok([]);
    }
    const _lock = await this.ddlLock.value.lock();
    try {
      const _r0 = await this.rebuildColumnsCache(conn);
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      _r0.drop();
      const tableName = this.stateTable();
      for (const [column, datatype] of missing) {
        if (SqliteStorageEngine.saneName(column) && !this.hasColumn(column)) {
          const alterQuery = `ALTER TABLE "${tableName}" ADD COLUMN "${column}" ${datatype}`;
          tracing.debug(`Adding column: ${alterQuery}`);
          const query = alterQuery;
          const _r2 = await conn.withConnection((c) => {
            const _r1 = c.execute(query, []);
            if (_r1.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r1.unwrapErr()));
            _r1.drop();
            return Result.Ok([]);
          });
          if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
          _r2.drop();
        }
      }
      const _r3 = await this.rebuildColumnsCache(conn);
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      _r3.drop();
      return Result.Ok([]);
    } finally {
      _lock.drop();
    }
  }

  async setState(state: Attested<EntityState>): Promise<Result<boolean, MutationError>> {
    try {
      const _r0 = (await this.pool.get()).mapErr((e) => {
        try {
          return new MutationError('General', { _0: new SqliteError('Pool', { _0: e.toString() }) });
        } finally {
          dropOwned(e);
        }
      });
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const conn = _r0.unwrap();
      try {
        if (state.payload.state.head.isEmpty()) {
          tracing.warn(`Warning: Empty head detected for entity ${state.payload.entityId}`);
        }
        const _r1 = (() => { const _w = new BincodeWriter(); state.payload.state.stateBuffers.encode(_w); return _w.finish(); })();
        if (_r1.isErr()) return Result.Err(MutationError.fromBincodeError(_r1.unwrapErr()));
        const stateBuffers = _r1.unwrap();
        const _r2 = serde_json.stringify((state.payload.state.head).toJSON()).mapErr((e) => new MutationError('General', { _0: e }));
        if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
        const headJson = _r2.unwrap();
        const _r3 = (() => { const _w = new BincodeWriter(); state.attestations.encode(_w); return _w.finish(); })();
        if (_r3.isErr()) return Result.Err(MutationError.fromBincodeError(_r3.unwrapErr()));
        const attestationsBlob = _r3.unwrap();
        const id = state.payload.entityId.toBase64();
        const idClone = id;
        let materialized = [];
        try {
          let seenProperties = new HashSet();
          for (const [name, stateBuffer] of [...state.payload.state.stateBuffers.deref()]) {
            const _r4 = backendFromString(name, stateBuffer);
            if (_r4.isErr()) return Result.Err(MutationError.fromRetrievalError(_r4.unwrapErr()));
            const backend = _r4.unwrap();
            try {
              const _seq8 = backend.value.propertyValues().intoEntries();
              let _at9 = 0;
              try {
                while (_at9 < _seq8.length) {
                  const [column, value] = _seq8[_at9++];
                  let _moved5 = false;
                  try {
                    if (!seenProperties.insert(column)) {
                      continue;
                    }
                    _moved5 = true;
                    let _moved6 = false;
                    const sqliteValue = (value != null ? ((v) => v)(value!) : null);
                    try {
                      const isJsonb = (sqliteValue != null && ((v) => v.isJsonb())(sqliteValue!));
                      if (!this.hasColumn(column)) {
                        {
                          const _v = sqliteValue;
                          if (_v != null) {
                            const sv = _v;
                            const _r7 = await this.addMissingColumns(conn, [[column, sv.sqliteType()]]);
                            if (_r7.isErr()) return Result.Err(MutationError.fromSqliteError(_r7.unwrapErr()));
                            _r7.drop();
                          } else {
                          continue;
                        }
                        }
                      }
                      _moved6 = true;
                      materialized.push([column, sqliteValue, isJsonb]);
                    } finally {
                      if (!_moved6) dropOwned(sqliteValue);
                    }
                  } finally {
                    if (!_moved5) dropOwned(value);
                  }
                }
              } finally {
                dropOwned(_seq8.slice(_at9));
              }
            } finally {
              backend.drop();
            }
          }
          const BASE_COLUMNS = ['id', 'state_buffer', 'head', 'attestations'];
          const tableName = this.stateTable();
          const numColumns = checkedAdd(BASE_COLUMNS.length, materialized.length, 'usize');
          let columns = [];
          columns.extendFromSlice(BASE_COLUMNS);
          let values = [];
          values.push(new rusqlite.types.Value('Text', { _0: id }));
          values.push(new rusqlite.types.Value('Blob', { _0: stateBuffers }));
          values.push(new rusqlite.types.Value('Text', { _0: headJson }));
          values.push(new rusqlite.types.Value('Blob', { _0: attestationsBlob }));
          let placeholderIsJsonb = [];
          placeholderIsJsonb.resize(BASE_COLUMNS.length, false);
          for (const [name, value, isJsonb] of materialized) {
            columns.push(name);
            values.push((() => {
              if (value != null) {
                const v = value;
                return v.toSql();
              } else {
                return rusqlite.types.Value.Null;
              }
            })());
            placeholderIsJsonb.push(isJsonb);
          }
          const columnsStr = [...columns].map((c) => `"${c}"`).join(', ');
          const placeholders = [...placeholderIsJsonb].map((isJsonb) => (isJsonb ? 'jsonb(?)' : '?')).join(', ');
          const updateStr = [...columns].slice(1).map((c) => `"${c}" = excluded."${c}"`).join(', ');
          const query = `INSERT INTO "${tableName}"(${columnsStr}) VALUES(${placeholders})\n               ON CONFLICT("id") DO UPDATE SET ${updateStr}`;
          tracing.debug(`set_state query: ${query}`);
          const newHead = state.payload.state.head.clone();
          const tableNameClone = tableName;
          const queryClone = query;
          const valuesClone = values.map((e) => e.clone());
          const _r14 = await conn.withConnection(new OwnedClosure([newHead], (c: Connection) => {
            const _m10 = (() => {
              const _v1 = c.queryRow(`SELECT "head" FROM "${tableNameClone}" WHERE "id" = ?`, [idClone], (row) => row.get(0));
              if (_v1.isOk()) {
                const json = _v1.unwrap();
                return json;
              } else {
                const _v2 = _v1.unwrapErr();
                if (_v2.is('QueryReturnedNoRows')) {
                  return null;
                }
                {
                  const e = _v2;
                  return { $jump: 'return', $value: Result.Err(new SqliteError('Rusqlite', { _0: e })) };
                }
              }
            })();
            if ((_m10 as any)?.$jump === 'return') return (_m10 as any).$value;
            const oldHeadJson = (_m10 as any);
            const _r11 = c.execute(queryClone, paramsFromIter([...valuesClone])).mapErr((e) => new SqliteError('Rusqlite', { _0: e }));
            if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
            _r11.drop();
            const _m13 = (() => {
              if (oldHeadJson != null) {
                const json = oldHeadJson;
                {
                  const _r12 = serde_json.parse(json).andThen((v) => Clock.fromJson(v)).mapErr((e) => new SqliteError('Json', { _0: e }));
                  if (_r12.isErr()) return { $jump: 'return', $value: Result.Err(_r12.unwrapErr()) };
                  const oldHead = _r12.unwrap();
                  try {
                    return !oldHead.equals(newHead);
                  } finally {
                    oldHead.drop();
                  }
                }
              } else {
                return true;
              }
            })();
            if ((_m13 as any)?.$jump === 'return') return (_m13 as any).$value;
            const changed = (_m13 as any);
            return Result.Ok(changed);
          }, undefined, true));
          if (_r14.isErr()) return Result.Err(MutationError.fromSqliteError(_r14.unwrapErr()));
          const changed = _r14.unwrap();
          tracing.debug(`set_state: Changed: ${changed}`);
          return Result.Ok(changed);
        } finally {
          dropOwned(materialized);
        }
      } finally {
        dropOwned(conn);
      }
    } finally {
      state.drop();
    }
  }

  async getState(id: EntityId): Promise<Result<Attested<EntityState>, RetrievalError>> {
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new SqliteError('Pool', { _0: e.toString() });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      const tableName = this.stateTable();
      const idStr = id.toBase64();
      const collectionId = this.collectionId.clone();
      const _r12 = (await conn.withConnection(new OwnedClosure([collectionId], (c: Connection) => {
        const query = `SELECT "id", "state_buffer", "head", "attestations" FROM "${tableName}" WHERE "id" = ?`;
        const result = c.queryRow(query, [idStr], (row) => {
          const _r1 = row.get(0);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          const _rowId = _r1.unwrap();
          const _r2 = row.get(1);
          if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
          const stateBuffer = _r2.unwrap();
          const _r3 = row.get(2);
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          const headJson = _r3.unwrap();
          const _r4 = row.get(3);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          const attestationsBlob = _r4.unwrap();
          return Result.Ok([stateBuffer, headJson, attestationsBlob]);
        });
        if (result.isOk()) {
          const _v = result.unwrap();
          {
            const _r5 = (() => { const _r = new BincodeReader(stateBuffer); return (() => { const _m = new HashMap<string, Uint8Array>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(_r.readString(), _r.readByteVec()); } return _m; })(); })().mapErr((e) => new SqliteError('Serialization', { _0: e }));
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            const stateBuffers = _r5.unwrap();
            const _r6 = serde_json.parse(headJson).andThen((v) => Clock.fromJson(v)).mapErr((e) => new SqliteError('Json', { _0: e }));
            if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
            let _moved7 = false;
            const head = _r6.unwrap();
            try {
              const _r8 = (() => { const _r = new BincodeReader(attestationsBlob); return AttestationSet.decode(_r); })().mapErr((e) => new SqliteError('Serialization', { _0: e }));
              if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
              let _moved9 = false;
              const attestations = _r8.unwrap();
              try {
                const _b10 = new StateBuffers(stateBuffers);
                _moved7 = true;
                _moved9 = true;
                return Result.Ok(new Attested(new EntityState(id, collectionId, new State(_b10, head)), attestations));
              } finally {
                if (!_moved9) attestations.drop();
              }
            } finally {
              if (!_moved7) head.drop();
            }
          }
        } else {
          const _v1 = result.unwrapErr();
          if (_v1.is('QueryReturnedNoRows')) {
            {
              const _ = createStateTable(c, collectionId);
              return Result.Err(new SqliteError('Rusqlite', { _0: rusqlite.Error.QueryReturnedNoRows }));
            }
          }
          {
            const e = _v1;
            return Result.Err(new SqliteError('Rusqlite', { _0: e }));
          }
        }
      }, undefined, true))).mapErr((e) => {
        let _moved11 = false;
        try {
          return (() => {
            if (e.is('Rusqlite') && (e.value._0.is('QueryReturnedNoRows'))) {
              return new RetrievalError('EntityNotFound', { _0: id });
            } else {
              _moved11 = true;
              return new RetrievalError('StorageError', { _0: e });
            }
          })();
        } finally {
          if (!_moved11) e.drop();
        }
      });
      if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
      const result = _r12.unwrap();
      return Result.Ok(result);
    } finally {
      dropOwned(conn);
    }
  }

  async fetchStates(selection: Selection): Promise<Result<Attested<EntityState>[], RetrievalError>> {
    tracing.debug(`SqliteBucket(${this.collectionId}).fetch_states: ${selection.debug()}`);
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new SqliteError('Pool', { _0: e.toString() });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      const referenced = selection.referencedColumns();
      const cached = this.existingColumns();
      const unknownToCache = [...referenced].filter((col) => !cached.includes(col));
      if (!(unknownToCache.length === 0)) {
        tracing.debug(`SqliteBucket(${this.collectionId}).fetch_states: Unknown columns ${`[${Array.from(unknownToCache).map((e) => debugString(e)).join(', ')}]`}, refreshing schema cache`);
        const _r1 = await this.rebuildColumnsCache(conn);
        if (_r1.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r1.unwrapErr()));
        _r1.drop();
      }
      const existing = this.existingColumns();
      const missing = [...referenced].filter((col) => !existing.includes(col));
      const effectiveSelection = (() => {
        if (missing.length === 0) {
          return selection.clone();
        } else {
          tracing.debug(`SqliteBucket(${this.collectionId}).fetch_states: Columns ${`[${Array.from(missing).map((e) => debugString(e)).join(', ')}]`} don't exist, treating as NULL`);
          return selection.assumeNull(missing);
        }
      })();
      try {
        const split = splitPredicateForSqlite(effectiveSelection.predicate);
        try {
          const needsPostFilter = split.needsPostFilter();
          const remainingPredicate = split.remainingPredicate.clone();
          try {
            const sqlSelection = new Selection(split.takeField('sqlPredicate'), effectiveSelection.orderBy.clone(), (needsPostFilter ? null : effectiveSelection.limit));
            try {
              let _moved2 = false;
              let builder = SqlBuilder.withFields(['id', 'state_buffer', 'head', 'attestations']);
              try {
                builder.tableName(this.stateTable());
                const _r3 = builder.selection(sqlSelection).mapErr((e) => {
                  try {
                    return new SqliteError('SqlGeneration', { _0: e.toString() });
                  } finally {
                    e.drop();
                  }
                });
                if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
                _r3.drop();
                _moved2 = true;
                const _r4 = builder.build().mapErr((e) => {
                  try {
                    return new SqliteError('SqlGeneration', { _0: e.toString() });
                  } finally {
                    e.drop();
                  }
                });
                if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
                const [sql, params] = _r4.unwrap();
                tracing.debug(`fetch_states SQL: ${sql} with ${params.length} params`);
                const collectionId = this.collectionId.clone();
                const _r19 = await conn.withConnection(new OwnedClosure([collectionId], (c: Connection) => {
                  const _r5 = c.prepare(sql);
                  if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
                  let stmt = _r5.unwrap();
                  const _r10 = stmt.queryMap(paramsFromIter([...params]), (row) => {
                    const _r6 = row.get(0);
                    if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
                    const idStr = _r6.unwrap();
                    const _r7 = row.get(1);
                    if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
                    const stateBuffer = _r7.unwrap();
                    const _r8 = row.get(2);
                    if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
                    const headJson = _r8.unwrap();
                    const _r9 = row.get(3);
                    if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
                    const attestationsBlob = _r9.unwrap();
                    return Result.Ok([idStr, stateBuffer, headJson, attestationsBlob]);
                  });
                  if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
                  const rows = _r10.unwrap();
                  let results = [];
                  for (const row of rows) {
                    const _r11 = row;
                    if (_r11.isErr()) return Result.Err(_r11.unwrapErr());
                    const [idStr, stateBuffer, headJson, attestationsBlob] = _r11.unwrap();
                    const _r12 = EntityId.fromBase64(idStr).mapErr((e) => {
                      return new rusqlite.Error('FromSqlConversionFailure', { _0: 0, _1: rusqlite.types.Type.Text, _2: io.Error.other(e) });
                    });
                    if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
                    const id = _r12.unwrap();
                    const _r13 = (() => { const _r = new BincodeReader(stateBuffer); return (() => { const _m = new HashMap<string, Uint8Array>(); const _len = _r.readLength(); for (let _i = 0; _i < _len; _i++) { _m.set(_r.readString(), _r.readByteVec()); } return _m; })(); })().mapErr((e) => {
                      return new rusqlite.Error('FromSqlConversionFailure', { _0: 1, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
                    });
                    if (_r13.isErr()) return Result.Err(_r13.unwrapErr());
                    const stateBuffers = _r13.unwrap();
                    const _r14 = serde_json.parse(headJson).andThen((v) => Clock.fromJson(v)).mapErr((e) => {
                      return new rusqlite.Error('FromSqlConversionFailure', { _0: 2, _1: rusqlite.types.Type.Text, _2: io.Error.other(e) });
                    });
                    if (_r14.isErr()) return Result.Err(_r14.unwrapErr());
                    let _moved15 = false;
                    const head = _r14.unwrap();
                    try {
                      const _r16 = (() => { const _r = new BincodeReader(attestationsBlob); return AttestationSet.decode(_r); })().mapErr((e) => {
                        return new rusqlite.Error('FromSqlConversionFailure', { _0: 3, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
                      });
                      if (_r16.isErr()) return Result.Err(_r16.unwrapErr());
                      let _moved17 = false;
                      const attestations = _r16.unwrap();
                      try {
                        const _b18 = new StateBuffers(stateBuffers);
                        _moved15 = true;
                        _moved17 = true;
                        results.push(new Attested(new EntityState(id, collectionId.clone(), new State(_b18, head)), attestations));
                      } finally {
                        if (!_moved17) attestations.drop();
                      }
                    } finally {
                      if (!_moved15) head.drop();
                    }
                  }
                  return Result.Ok(results);
                }));
                if (_r19.isErr()) return Result.Err(RetrievalError.fromSqliteError(_r19.unwrapErr()));
                let results = _r19.unwrap();
                if (needsPostFilter) {
                  tracing.debug(`Post-filtering ${results.len()} results`);
                  results = postFilterStates(results, remainingPredicate, this.collectionId);
                  {
                    const _v = effectiveSelection.limit;
                    if (_v != null) {
                      const limit = _v;
                      results.truncate(Number(BigInt.asUintN(32, limit)));
                    }
                  }
                }
                return Result.Ok(results);
              } finally {
                if (!_moved2) builder.drop();
              }
            } finally {
              sqlSelection.drop();
            }
          } finally {
            remainingPredicate.drop();
          }
        } finally {
          split.drop();
        }
      } finally {
        effectiveSelection.drop();
      }
    } finally {
      dropOwned(conn);
    }
  }

  async addEvent(entityEvent: Attested<Event>): Promise<Result<boolean, MutationError>> {
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new MutationError('General', { _0: new SqliteError('Pool', { _0: e.toString() }) });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      const _r1 = (() => { const _w = new BincodeWriter(); entityEvent.payload.operations.encode(_w); return _w.finish(); })();
      if (_r1.isErr()) return Result.Err(MutationError.fromBincodeError(_r1.unwrapErr()));
      const operations = _r1.unwrap();
      const _r2 = (() => { const _w = new BincodeWriter(); entityEvent.attestations.encode(_w); return _w.finish(); })();
      if (_r2.isErr()) return Result.Err(MutationError.fromBincodeError(_r2.unwrapErr()));
      const attestations = _r2.unwrap();
      const _r3 = serde_json.stringify((entityEvent.payload.parent).toJSON()).mapErr((e) => new MutationError('General', { _0: e }));
      if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
      const parentJson = _r3.unwrap();
      const tableName = this.eventTable();
      const _t4 = entityEvent.payload.id();
      try {
        const eventId = _t4.toBase64();
        const entityId = entityEvent.payload.entityId.toBase64();
        const query = `INSERT INTO "${tableName}"("id", "entity_id", "operations", "parent", "attestations") VALUES(?, ?, ?, ?, ?)\n               ON CONFLICT ("id") DO NOTHING`;
        return (await conn.withConnection((c) => {
          const _r5 = c.execute(query, [eventId, entityId, operations, parentJson, attestations]);
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          const affected = _r5.unwrap();
          return Result.Ok(affected > 0);
        })).mapErr((e) => new MutationError('General', { _0: e }));
      } finally {
        _t4.drop();
      }
    } finally {
      dropOwned(conn);
    }
  }

  async getEvents(eventIds: EventId[]): Promise<Result<Attested<Event>[], RetrievalError>> {
    try {
      if (eventIds.length === 0) {
        return Result.Ok([]);
      }
      const _r0 = (await this.pool.get()).mapErr((e) => {
        try {
          return new SqliteError('Pool', { _0: e.toString() });
        } finally {
          dropOwned(e);
        }
      });
      if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
      const conn = _r0.unwrap();
      try {
        const tableName = this.eventTable();
        const collectionId = this.collectionId.clone();
        const idStrings = [...eventIds].map((id) => id.toBase64());
        const numIds = idStrings.length;
        return (await conn.withConnection(new OwnedClosure([collectionId], (c: Connection) => {
          const placeholders = (range(0, numIds)).map((_) => '?').join(', ');
          const query = `SELECT "id", "entity_id", "operations", "parent", "attestations" FROM "${tableName}" WHERE "id" IN (${placeholders})`;
          const _r1 = c.prepare(query);
          if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
          let stmt = _r1.unwrap();
          const params = [...idStrings].map((s) => s);
          const _r7 = stmt.queryMap(params.asSlice(), (row) => {
            const _r2 = row.get(0);
            if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
            const _eventId = _r2.unwrap();
            const _r3 = row.get(1);
            if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
            const entityIdStr = _r3.unwrap();
            const _r4 = row.get(2);
            if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
            const operations = _r4.unwrap();
            const _r5 = row.get(3);
            if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
            const parentJson = _r5.unwrap();
            const _r6 = row.get(4);
            if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
            const attestationsBlob = _r6.unwrap();
            return Result.Ok([entityIdStr, operations, parentJson, attestationsBlob]);
          });
          if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
          const rows = _r7.unwrap();
          let events = [];
          for (const row of rows) {
            const _r8 = row;
            if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
            const [entityIdStr, operationsBlob, parentJson, attestationsBlob] = _r8.unwrap();
            const _r9 = EntityId.fromBase64(entityIdStr).mapErr((e) => {
              return new rusqlite.Error('FromSqlConversionFailure', { _0: 1, _1: rusqlite.types.Type.Text, _2: io.Error.other(e) });
            });
            if (_r9.isErr()) return Result.Err(_r9.unwrapErr());
            const entityId = _r9.unwrap();
            const _r10 = (() => { const _r = new BincodeReader(operationsBlob); return OperationSet.decode(_r); })().mapErr((e) => {
              return new rusqlite.Error('FromSqlConversionFailure', { _0: 2, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
            });
            if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
            let _moved11 = false;
            const operations = _r10.unwrap();
            try {
              const _r12 = serde_json.parse(parentJson).andThen((v) => Clock.fromJson(v)).mapErr((e) => {
                return new rusqlite.Error('FromSqlConversionFailure', { _0: 3, _1: rusqlite.types.Type.Text, _2: io.Error.other(e) });
              });
              if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
              let _moved13 = false;
              const parent = _r12.unwrap();
              try {
                const _r14 = (() => { const _r = new BincodeReader(attestationsBlob); return AttestationSet.decode(_r); })().mapErr((e) => {
                  return new rusqlite.Error('FromSqlConversionFailure', { _0: 4, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
                });
                if (_r14.isErr()) return Result.Err(_r14.unwrapErr());
                let _moved15 = false;
                const attestations = _r14.unwrap();
                try {
                  const _b16 = collectionId.clone();
                  _moved11 = true;
                  _moved13 = true;
                  _moved15 = true;
                  events.push(new Attested(new Event(_b16, entityId, operations, parent), attestations));
                } finally {
                  if (!_moved15) attestations.drop();
                }
              } finally {
                if (!_moved13) parent.drop();
              }
            } finally {
              if (!_moved11) operations.drop();
            }
          }
          return Result.Ok(events);
        }, undefined, true))).mapErr((e) => new RetrievalError('StorageError', { _0: e }));
      } finally {
        dropOwned(conn);
      }
    } finally {
      dropOwned(eventIds);
    }
  }

  async dumpEntityEvents(entityId: EntityId): Promise<Result<Attested<Event>[], RetrievalError>> {
    const _r0 = (await this.pool.get()).mapErr((e) => {
      try {
        return new SqliteError('Pool', { _0: e.toString() });
      } finally {
        dropOwned(e);
      }
    });
    if (_r0.isErr()) return Result.Err(_r0.unwrapErr());
    const conn = _r0.unwrap();
    try {
      const tableName = this.eventTable();
      const collectionId = this.collectionId.clone();
      const entityIdStr = entityId.toBase64();
      return (await conn.withConnection(new OwnedClosure([collectionId], (c: Connection) => {
        const query = `SELECT "id", "operations", "parent", "attestations" FROM "${tableName}" WHERE "entity_id" = ?`;
        const _r1 = c.prepare(query);
        if (_r1.isErr()) return Result.Err(_r1.unwrapErr());
        let stmt = _r1.unwrap();
        const _r6 = stmt.queryMap([entityIdStr], (row) => {
          const _r2 = row.get(0);
          if (_r2.isErr()) return Result.Err(_r2.unwrapErr());
          const _eventId = _r2.unwrap();
          const _r3 = row.get(1);
          if (_r3.isErr()) return Result.Err(_r3.unwrapErr());
          const operations = _r3.unwrap();
          const _r4 = row.get(2);
          if (_r4.isErr()) return Result.Err(_r4.unwrapErr());
          const parentJson = _r4.unwrap();
          const _r5 = row.get(3);
          if (_r5.isErr()) return Result.Err(_r5.unwrapErr());
          const attestationsBlob = _r5.unwrap();
          return Result.Ok([operations, parentJson, attestationsBlob]);
        });
        if (_r6.isErr()) return Result.Err(_r6.unwrapErr());
        const rows = _r6.unwrap();
        let events = [];
        for (const row of rows) {
          const _r7 = row;
          if (_r7.isErr()) return Result.Err(_r7.unwrapErr());
          const [operationsBlob, parentJson, attestationsBlob] = _r7.unwrap();
          const _r8 = (() => { const _r = new BincodeReader(operationsBlob); return OperationSet.decode(_r); })().mapErr((e) => {
            return new rusqlite.Error('FromSqlConversionFailure', { _0: 1, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
          });
          if (_r8.isErr()) return Result.Err(_r8.unwrapErr());
          let _moved9 = false;
          const operations = _r8.unwrap();
          try {
            const _r10 = serde_json.parse(parentJson).andThen((v) => Clock.fromJson(v)).mapErr((e) => {
              return new rusqlite.Error('FromSqlConversionFailure', { _0: 2, _1: rusqlite.types.Type.Text, _2: io.Error.other(e) });
            });
            if (_r10.isErr()) return Result.Err(_r10.unwrapErr());
            let _moved11 = false;
            const parent = _r10.unwrap();
            try {
              const _r12 = (() => { const _r = new BincodeReader(attestationsBlob); return AttestationSet.decode(_r); })().mapErr((e) => {
                return new rusqlite.Error('FromSqlConversionFailure', { _0: 3, _1: rusqlite.types.Type.Blob, _2: io.Error.other(e) });
              });
              if (_r12.isErr()) return Result.Err(_r12.unwrapErr());
              let _moved13 = false;
              const attestations = _r12.unwrap();
              try {
                const _b14 = collectionId.clone();
                _moved9 = true;
                _moved11 = true;
                _moved13 = true;
                events.push(new Attested(new Event(_b14, entityId, operations, parent), attestations));
              } finally {
                if (!_moved13) attestations.drop();
              }
            } finally {
              if (!_moved11) parent.drop();
            }
          } finally {
            if (!_moved9) operations.drop();
          }
        }
        return Result.Ok(events);
      }, undefined, true))).mapErr((e) => new RetrievalError('StorageError', { _0: e }));
    } finally {
      dropOwned(conn);
    }
  }
}

function createStateTable(conn: Connection, collectionId: CollectionId): Result<void, SqliteError> {
  const tableName = collectionId.asStr();
  const query = `CREATE TABLE IF NOT EXISTS "${tableName}"(\n            "id" TEXT PRIMARY KEY,\n            "state_buffer" BLOB NOT NULL,\n            "head" TEXT NOT NULL,\n            "attestations" BLOB\n        )`;
  tracing.debug(`Creating state table: ${query}`);
  const _r0 = conn.execute(query, []);
  if (_r0.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r0.unwrapErr()));
  _r0.drop();
  return Result.Ok([]);
}

function createEventTable(conn: Connection, collectionId: CollectionId): Result<void, SqliteError> {
  const tableName = `${collectionId.asStr()}_event`;
  const query = `CREATE TABLE IF NOT EXISTS "${tableName}"(\n            "id" TEXT PRIMARY KEY,\n            "entity_id" TEXT,\n            "operations" BLOB,\n            "parent" TEXT,\n            "attestations" BLOB\n        )`;
  tracing.debug(`Creating event table: ${query}`);
  const _r0 = conn.execute(query, []);
  if (_r0.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r0.unwrapErr()));
  _r0.drop();
  const indexQuery = `CREATE INDEX IF NOT EXISTS "${tableName}_entity_id_idx" ON "${tableName}"("entity_id")`;
  const _r1 = conn.execute(indexQuery, []);
  if (_r1.isErr()) return Result.Err(SqliteError.fromRusqliteError(_r1.unwrapErr()));
  _r1.drop();
  return Result.Ok([]);
}

function postFilterStates(states: Attested<EntityState>[], predicate: Predicate, collectionId: CollectionId): Attested<EntityState>[] {
  return [...[...states].filter((attested) => (() => {
    const _v2 = TemporaryEntity.new(attested.payload.entityId, collectionId.clone(), attested.payload.state);
    if (_v2.isOk()) {
      const tempEntity = _v2.unwrap();
      try {
        const _v3 = evaluatePredicate(tempEntity, predicate);
        if (_v3.isOk()) {
          const result = _v3.unwrap();
          return result;
        } else {
          const e = _v3.unwrapErr();
          try {
            {
              tracing.warn(`Post-filter evaluation error for entity ${attested.payload.entityId}: ${e}`);
              return false;
            }
          } finally {
            e.drop();
          }
        }
      } finally {
        tempEntity.drop();
      }
    } else {
      const e = _v2.unwrapErr();
      try {
        {
          tracing.warn(`Failed to create TemporaryEntity for post-filtering ${attested.payload.entityId}: ${e}`);
          return false;
        }
      } finally {
        e.drop();
      }
    }
  })())];
}

export const DEFAULT_POOL_SIZE: number = 10;

