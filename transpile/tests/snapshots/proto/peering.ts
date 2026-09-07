// MIRRORS: ankurah/proto/src/peering.rs
import { Struct, Result, JsonError, dropOwned, OwnershipFatal, UnsupportedShape } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { Attested } from './auth';
import { EntityState } from './data';
import { EntityId } from './id';

export class Presence extends Struct {
  readonly nodeId: EntityId;
  readonly durable: boolean;
  readonly systemRoot: Attested<EntityState> | null;

  constructor(nodeId: EntityId, durable: boolean, systemRoot: Attested<EntityState> | null) {
    super();
    this.nodeId = nodeId;
    this.durable = durable;
    this.systemRoot = systemRoot;
  }

  toString(): string {
    if (this.systemRoot != null) {
      const r = this.systemRoot;
      return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable} system_root: ${r.payload}]`;
    } else {
      return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable}]`;
    }
  }

  equals(other: Presence): boolean {
    if (!this.nodeId.equals(other.nodeId)) return false;
    if (this.durable !== other.durable) return false;
    { if ((this.systemRoot == null) !== (other.systemRoot == null)) return false; if (this.systemRoot != null) { if (!this.systemRoot!.equals(other.systemRoot!)) return false; } }
    return true;
  }

  clone(): Presence {
    return new Presence(this.nodeId.clone(), this.durable, this.systemRoot?.clone() ?? null);
  }

  debug(): string {
    return `Presence { nodeId: ${this.nodeId.debug()}, durable: ${String(this.durable)}, systemRoot: ${(($v) => $v === null ? 'None' : `Some(${$v.debug()})`)(this.systemRoot)} }`;
  }

  encode(writer: BincodeWriter): void {
    this.nodeId.encode(writer);
    writer.writeBool(this.durable);
    writer.writeOption(this.systemRoot, (w, v) => v.encode(w, (w2: BincodeWriter, p: EntityState) => p.encode(w2)));
  }

  static decode(reader: BincodeReader): Presence {
    const nodeId = EntityId.decode(reader);
    const durable = reader.readBool();
    const systemRoot = reader.readOption((r) => Attested.decode(r, (r2: BincodeReader) => EntityState.decode(r2)));
    return new Presence(nodeId, durable, systemRoot);
  }

  toJSON(): unknown {
    return { 'node_id': this.nodeId.toJSON(), 'durable': this.durable, 'system_root': (this.systemRoot == null ? null : this.systemRoot.toJSON(($p0: EntityState) => $p0.toJSON())) };
  }

  static fromJson(value: unknown): Result<Presence, JsonError> {
    const $built: unknown[] = [];
    let $kept = false;
    try {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return Result.Err(JsonError.custom('expected an object for `Presence`'));
      }
      const _o = value as Record<string, unknown>;
      if (!('node_id' in _o)) {
        return Result.Err(JsonError.custom('missing field `node_id`'));
      }
      const _rnodeId = ((v: unknown) => EntityId.fromJson(v))(_o['node_id']);
      if (_rnodeId.isErr()) return Result.Err(_rnodeId.unwrapErr());
      const nodeId = _rnodeId.unwrap();
      $built.push(nodeId);
      if (!('durable' in _o)) {
        return Result.Err(JsonError.custom('missing field `durable`'));
      }
      const _rdurable = ((v: unknown) => (typeof v === 'boolean' ? Result.Ok(v as boolean) : Result.Err(JsonError.custom('expected a boolean'))))(_o['durable']);
      if (_rdurable.isErr()) return Result.Err(_rdurable.unwrapErr());
      const durable = _rdurable.unwrap();
      const _rsystemRoot = ((v: unknown) => (v == null ? Result.Ok(null) : ((v: unknown) => Attested.fromJson(v, (v: unknown) => EntityState.fromJson(v)))(v)))(_o['system_root']);
      if (_rsystemRoot.isErr()) return Result.Err(_rsystemRoot.unwrapErr());
      const systemRoot = _rsystemRoot.unwrap();
      $built.push(systemRoot);
      const $out = new Presence(nodeId, durable, systemRoot);
      $kept = true;
      return Result.Ok($out);
    } catch (e) {
      if (e instanceof OwnershipFatal || e instanceof UnsupportedShape) throw e;
      return Result.Err(JsonError.fromException(e));
    } finally {
      if (!$kept) dropOwned($built);
    }
  }
}

