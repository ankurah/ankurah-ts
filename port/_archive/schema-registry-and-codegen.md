# Schema Registry and Code Generation

## The Central Challenge

In Rust ankurah, the **Rust struct IS the schema**. The `#[derive(Model)]` macro reads struct fields at compile time and generates View/Mutable wrappers. There is no separate schema definition - the Rust type system is the source of truth.

For a TypeScript port, we need a **language-agnostic schema representation** that can generate typed wrappers for both Rust and TypeScript from a single source.

## PR #236: Property Registration

[PR #236](https://github.com/ankurah/ankurah/pull/236) introduces exactly this: a property registration system where Models and Properties become first-class entities in the `_ankurah_system` collection.

### What PR #236 adds:

1. **`sys::Item::Model`** - A registered model (name only)
2. **`sys::Item::Property`** - A registered property with:
   - `name: String` (e.g., "title", "email")
   - `model: EntityId` (reference to parent Model)
   - `backend: BackendKind` (Lww, YrsText, YrsMap, YrsArray)
   - `value_type: ValueType` (I16, I32, I64, F64, Bool, String, EntityId, Object, Binary, Json)
   - `optional: bool`
   - `target_model: Option<EntityId>` (for Ref types)
3. **`BackendKind` enum** - Language-agnostic backend identifiers
4. **`ValueType` enum** - Language-agnostic primitive types
5. **Bidirectional `From` conversions** between `core::ValueType` and `proto::sys::ValueType`

### What remains to complete in PR #236:

- [ ] Backend refactoring to use property entity IDs instead of string names
- [ ] Registration flow (triggered on first `trx.create::<Model>()`)
- [ ] Read path updates (empty string fix via registered defaults)
- [ ] Derive macro updates to emit registration code
- [ ] SystemManager CRUD methods for Model/Property entities

## Schema Flow: Rust → Schema Registry → TypeScript

```
┌──────────────────────┐
│  Rust Model Struct    │
│  #[derive(Model)]     │
│  pub struct Album {   │
│    title: String,     │
│    artist: Ref<Artist>│
│    year: i32,         │
│  }                    │
└──────────┬───────────┘
           │ (derive macro emits registration)
           ▼
┌──────────────────────┐
│  Schema Registry      │  (stored in _ankurah_system as entities)
│  Model: "Album"       │
│  Properties:          │
│    title:  YrsText/String/required     │
│    artist: Lww/EntityId/required/→Artist│
│    year:   Lww/I32/required            │
└──────────┬───────────┘
           │ (CLI reads from registry OR schema file)
           ▼
┌──────────────────────┐
│  @ankurah/cli         │
│  ankurah-codegen      │
│  generate --lang ts   │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│  Generated TypeScript │
│  AlbumView class      │
│  AlbumMutable class   │
│  AlbumInput interface │
│  AlbumRef class       │
│  AlbumLiveQuery class │
│  AlbumOps class       │
└──────────────────────┘
```

## Schema Definition Format

Rather than requiring the Rust node to be running to read the schema from the system collection, we should support a **static schema definition file** that both the Rust derive macro and the TS codegen can consume.

### Option A: Schema file as source of truth (Recommended)

```yaml
# ankurah-schema.yaml
models:
  Album:
    collection: album
    properties:
      title:
        backend: yrs_text
        value_type: string
        optional: false
      artist:
        backend: lww
        value_type: entity_id
        optional: false
        target_model: Artist
      year:
        backend: lww
        value_type: i32
        optional: false

  Artist:
    collection: artist
    properties:
      name:
        backend: yrs_text
        value_type: string
        optional: false
      genre:
        backend: lww
        value_type: string
        optional: false
```

**Workflow**:
1. Developer defines models in `ankurah-schema.yaml`
2. Rust `#[derive(Model)]` reads the schema file (or validates against it)
3. `ankurah-codegen generate --lang ts --schema ankurah-schema.yaml` generates TS wrappers
4. Both Rust and TS share the exact same schema

### Option B: Rust struct remains source of truth, CLI extracts schema

```
cargo run --bin ankurah-schema-export > schema.json
ankurah-codegen generate --lang ts --schema schema.json
```

**Workflow**:
1. Developer defines models as Rust structs with `#[derive(Model)]`
2. A Rust binary reads the derive macro output and emits a schema JSON
3. `ankurah-codegen` reads the JSON and generates TS wrappers

This is simpler but requires a Rust build step before TS codegen.

### Option C: Runtime schema extraction from running node

The Rust node starts, registers models in `_ankurah_system`, and the TS CLI connects to read the schema. This is the most dynamic but requires a running node.

### Recommendation

**Option B** for the initial implementation, with a path to Option A for ongoing maintenance:

1. Complete PR #236 to establish the schema registry
2. Add a Rust CLI command: `ankurah schema export` that reads the derive macro metadata and outputs `schema.json`
3. Build `@ankurah/cli` that reads `schema.json` and generates TS wrappers
4. Future: Support a YAML/TOML schema file as the shared source of truth

## Generated TypeScript Code

For each model in the schema, the CLI generates:

### ModelView (read-only typed accessor)

```typescript
// Generated: AlbumView.ts
import { Entity, PropertyError, EntityId, Ref } from '@ankurah/core';
import { YjsBackend, LWWBackend } from '@ankurah/core';
import { Signal, Listener, Subscription } from '@ankurah/signals';
import { ArtistView } from './ArtistView';

export class AlbumView implements Signal {
  private readonly entity: Entity;

  constructor(entity: Entity) {
    this.entity = entity;
  }

  get id(): EntityId { return this.entity.id; }

  // Typed getter - projects from YrsString backend to string
  get title(): string {
    return this.entity.getPropertyValue('title') as string ?? '';
  }

  // Ref getter - returns typed reference
  get artist(): Ref<ArtistView> {
    const id = this.entity.getPropertyValue('artist') as EntityId;
    return new Ref<ArtistView>(id);
  }

  // LWW getter - projects from LWW backend to number
  get year(): number {
    return this.entity.getPropertyValue('year') as number ?? 0;
  }

  // Signal implementation
  listen(listener: Listener): Subscription {
    return this.entity.broadcast.listen(listener);
  }

  // Edit - returns mutable handle within a transaction
  edit(trx: Transaction): AlbumMutable {
    return trx.edit<Album>(this.entity);
  }

  // Conversion
  static fromEntity(entity: Entity): AlbumView {
    return new AlbumView(entity);
  }
}
```

### ModelMutable (CRDT wrapper accessor)

```typescript
// Generated: AlbumMutable.ts
import { Entity, Transaction, EntityId } from '@ankurah/core';
import { YjsText, LWW, Ref } from '@ankurah/core';
import { ArtistView } from './ArtistView';

export class AlbumMutable {
  private readonly entity: Entity;

  constructor(entity: Entity) {
    this.entity = entity;
  }

  get id(): EntityId { return this.entity.id; }

  // Returns CRDT wrapper for collaborative editing
  get title(): YjsText {
    return this.entity.getActiveType<YjsText>('title');
  }

  // Returns LWW wrapper for atomic set
  get artist(): LWW<Ref<ArtistView>> {
    return this.entity.getActiveType<LWW<Ref<ArtistView>>>('artist');
  }

  get year(): LWW<number> {
    return this.entity.getActiveType<LWW<number>>('year');
  }

  // Read the current state
  read(): AlbumView {
    return AlbumView.fromEntity(this.entity);
  }
}
```

### ModelInput (creation data)

```typescript
// Generated: AlbumInput.ts
export interface AlbumInput {
  title: string;
  artist: EntityId | string;  // Accept EntityId or base64 string
  year: number;
}
```

### ModelOps (CRUD operations)

```typescript
// Generated: AlbumOps.ts
export class AlbumOps {
  private ctx: Context;

  constructor(ctx: Context) {
    this.ctx = ctx;
  }

  async get(id: EntityId): Promise<AlbumView> {
    return this.ctx.get<AlbumView>(AlbumView, id);
  }

  async fetch(query: string): Promise<AlbumView[]> {
    return this.ctx.fetch<AlbumView>(AlbumView, query);
  }

  query(query: string): LiveQuery<AlbumView> {
    return this.ctx.query<AlbumView>(AlbumView, query);
  }

  async create(trx: Transaction, input: AlbumInput): Promise<AlbumMutable> {
    return trx.create(Album, input);
  }
}
```

## Type Mappings

| Schema `value_type` | Schema `backend` | TypeScript View Type | TypeScript Mutable Type |
|---------------------|------------------|---------------------|------------------------|
| `string` | `yrs_text` | `string` | `YjsText` |
| `string` | `lww` | `string` | `LWW<string>` |
| `i16` | `lww` | `number` | `LWW<number>` |
| `i32` | `lww` | `number` | `LWW<number>` |
| `i64` | `lww` | `bigint` | `LWW<bigint>` |
| `f64` | `lww` | `number` | `LWW<number>` |
| `bool` | `lww` | `boolean` | `LWW<boolean>` |
| `entity_id` | `lww` | `Ref<T>` | `LWW<Ref<T>>` |
| `json` | `lww` | `JsonValue` | `LWW<JsonValue>` |
| `binary` | `lww` | `Uint8Array` | `LWW<Uint8Array>` |
| `object` | `lww` | `Uint8Array` | `LWW<Uint8Array>` |

## Minimal Rust Changes Required

1. **Complete PR #236** - Property registration with Model/Property as system entities
2. **Add schema export command** - `ankurah schema export --format json` that reads all registered models and outputs the schema
3. **Ensure the derive macro emits schema metadata** - The macro should populate the schema registry entries
4. **Wire format documentation** - Document the exact binary encoding of NodeMessage, Event, State for TS interop

## Ongoing Sync: When Rust Models Change

When a developer modifies a Rust model struct:

1. The derive macro updates the schema registry metadata
2. Run `ankurah schema export > schema.json`
3. Run `ankurah-codegen generate --lang ts --schema schema.json --output src/generated/`
4. TypeScript compiler catches any type mismatches in consuming code

This can be automated via:
- A pre-commit hook
- A cargo build script that triggers the export
- A watch mode in the CLI
- A CI step that validates TS generated code matches Rust schema
