// MIRRORS: ankurah/proto/src/peering.rs

import { Struct } from '@ankurah/base';
import { BincodeReader, BincodeWriter } from './codec';
import { Attested } from './auth';
import { EntityId } from './id';
import { EntityState } from './data';

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

  // impl Display for Presence
  toString(): string {
    if (this.systemRoot !== null) {
      return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable} system_root: ${this.systemRoot.payload}]`;
    }
    return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable}]`;
  }

  // derive(PartialEq)
  equals(other: Presence): boolean {
    if (!this.nodeId.equals(other.nodeId)) return false;
    if (this.durable !== other.durable) return false;
    if (this.systemRoot === null && other.systemRoot === null) return true;
    if (this.systemRoot === null || other.systemRoot === null) return false;
    return true;
  }

  // derive(Clone)
  // Divergence: shallow clone — EntityId and Attested don't have clone() yet
  clone(): Presence {
    return new Presence(this.nodeId, this.durable, this.systemRoot);
  }

  encode(writer: BincodeWriter): void {
    this.nodeId.encode(writer);
    writer.writeBool(this.durable);
    writer.writeOption(this.systemRoot, (w, v) => {
      v.encode(w, (w2, es) => es.encode(w2));
    });
  }

  static decode(reader: BincodeReader): Presence {
    const nodeId = EntityId.decode(reader);
    const durable = reader.readBool();
    const systemRoot = reader.readOption(r =>
      Attested.decode(r, r2 => EntityState.decode(r2))
    );
    return new Presence(nodeId, durable, systemRoot);
  }
}
