// MIRRORS: ankurah/proto/src/peering.rs

import { BincodeReader, BincodeWriter } from './codec';
import { Attested } from './auth';
import { EntityId } from './id';
import { EntityState } from './data';

/**
 * Presence: node identity announcement.
 * Derived serde — struct { node_id: EntityId, durable: bool, system_root: Option<Attested<EntityState>> }.
 */
export class Presence {
  readonly nodeId: EntityId;
  readonly durable: boolean;
  readonly systemRoot: Attested<EntityState> | null;

  constructor(nodeId: EntityId, durable: boolean, systemRoot: Attested<EntityState> | null) {
    this.nodeId = nodeId;
    this.durable = durable;
    this.systemRoot = systemRoot;
  }

  equals(other: Presence): boolean {
    if (!this.nodeId.equals(other.nodeId)) return false;
    if (this.durable !== other.durable) return false;
    if (this.systemRoot === null && other.systemRoot === null) return true;
    if (this.systemRoot === null || other.systemRoot === null) return false;
    // Deep compare system roots would require EntityState equality
    return true; // Simplified — full equality check deferred
  }

  toString(): string {
    if (this.systemRoot !== null) {
      return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable} system_root: ${this.systemRoot.payload}]`;
    }
    return `Presence[${this.nodeId.toBase64Short()}: durable ${this.durable}]`;
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
