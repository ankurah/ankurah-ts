// MIRRORS: ankurah/proto/src/peering.rs
import { Struct, Result, JsonError } from '@ankurah/base';
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
    if (this.systemRoot === null && other.systemRoot === null) { /* both null, ok */ }
    else if (this.systemRoot === null || other.systemRoot === null) return false;
    else if (!this.systemRoot.equals(other.systemRoot)) return false;
    return true;
  }

  clone(): Presence {
    return new Presence(this.nodeId.clone(), this.durable, this.systemRoot?.clone() ?? null);
  }

  debug(): string {
    return `Presence { nodeId: ${this.nodeId}, durable: ${String(this.durable)}, systemRoot: ${this.systemRoot} }`;
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
    return {
      'node_id': this.nodeId,
      'durable': this.durable,
      'system_root': this.systemRoot,
    };
  }

  static fromJson(value: unknown): Result<Presence, JsonError> {
    try {
      const _take = <T,>(r: Result<T, JsonError>): T => { if (r.isErr()) throw r.unwrapErr(); return r.unwrap(); };
      const o = value as Record<string, unknown>;
      const nodeId = ((v: unknown) => _take(EntityId.fromJson(v)))(o['node_id']);
      const durable = ((v: unknown) => v as boolean)(o['durable']);
      const systemRoot = ((v: unknown) => (v == null ? null : ((v) => _take(Attested.fromJson(v)))(v)))(o['system_root']);
      return Result.Ok(new Presence(nodeId, durable, systemRoot));
    } catch (e) {
      return Result.Err(JsonError.fromException(e));
    }
  }
}

