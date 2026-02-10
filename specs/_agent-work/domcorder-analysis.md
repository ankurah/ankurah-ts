# Domcorder Proto-TS: TypeScript Bincode Implementation Analysis

**Source**: `/Users/daniel/code/domcorder/` -- actual code read and analyzed
**Date**: 2026-02-10
**Purpose**: Reference implementation study for ankurah-ts bincode codec design

---

## 1. Project Overview

Domcorder is a DOM recording system. The `proto-ts` and `proto-rs` packages implement a shared binary protocol for serializing/deserializing DOM recording frames. The Rust side uses `bincode 1.3` with `serde` derive macros. The TypeScript side implements a **hand-rolled bincode-compatible codec** that produces byte-identical output to Rust's bincode.

### Critical Configuration: Big-Endian + Fixed Integers

The Rust writer (`/Users/daniel/code/domcorder/proto-rs/src/writer.rs` line 86-88) explicitly configures:

```rust
let config = bincode::DefaultOptions::new()
    .with_big_endian()
    .with_fixint_encoding();
```

This means:
- **Big-endian** byte order (NOT the bincode default of little-endian)
- **Fixed-size integer encoding** (NOT varint)
- Lengths are **u64** (8 bytes, big-endian)
- Enum variant indices are **u32** (4 bytes, big-endian)

**This is a non-default bincode configuration.** Standard bincode v1 defaults to little-endian. Domcorder chose big-endian, likely for network byte order convention.

**Ankurah uses default bincode v1 (little-endian, fixed integers).** The patterns are the same, just the endianness parameter differs.

---

## 2. Code Organization

### File Structure

```
proto-ts/
  src/
    index.ts       -- re-exports everything
    writer.ts      -- Writer class (binary encoding primitives)
    reader.ts      -- Reader class (binary decoding primitives)
    frames.ts      -- Frame enum + all frame type classes (encode/decode)
    vdom.ts        -- VNode, VElement, VDocument, etc. (encode/decode)
  test/
    writer-consolidated.test.ts   -- Writer unit tests
    reader-basic.test.ts          -- Reader unit tests
    reader-errors.test.ts         -- Error handling tests
    reader-roundtrip.test.ts      -- TS Writer->Reader round-trip tests
    frames.test.ts                -- Cross-language fixture comparison
    file-format.test.ts           -- .dcrr file format tests
    async-frames.test.ts          -- Async encoding tests
    stream-observer.test.ts       -- Stream utility tests
    sample-frames.ts              -- Shared test data (TS frame instances)
    stream-observer.ts            -- Test utility for consuming ReadableStreams
    util.js                       -- Binary file comparison utility
```

### Key Design Decisions

1. **No separate codec file per type.** Each type class (e.g., `Timestamp`, `VElement`) has its own `encode(w: Writer)` and `static decode(r: BufferReader)` methods. The codec is co-located with the type.

2. **Writer and Reader are the primitive layer.** They handle `u32`, `u64`, `byte`, `string`, `bytes` -- nothing domain-specific. All domain encoding logic lives in the type classes.

3. **Streaming architecture.** Writer outputs to a `ReadableStream<Uint8Array>`. Reader consumes a `ReadableStream<Uint8Array>` and outputs a `ReadableStream<Frame>`. This differs from a simple ArrayBuffer-in/ArrayBuffer-out approach.

4. **Enum dispatch via array lookup.** The `Frame.decode()` method uses an indexed array `DECODERS[]` for O(1) dispatch by frame type, rather than a switch statement.

---

## 3. Writer Implementation (`/Users/daniel/code/domcorder/proto-ts/src/writer.ts`)

### Class Structure

```typescript
export class Writer {
    private buf: Uint8Array;
    private bufLength: number = 0;
    private chunkSize: number;
    private controller!: ReadableStreamDefaultController<Uint8Array>;
    private stream: ReadableStream<Uint8Array>;
    private static enc = new TextEncoder();

    private constructor(chunkSize: number = 4096) { ... }

    static create(chunkSize: number = 4096): [Writer, ReadableStream<Uint8Array>] {
        const writer = new Writer(chunkSize);
        return [writer, writer.stream];
    }
```

**Factory pattern**: Private constructor, public static `create()` that returns `[Writer, ReadableStream]` tuple.

### Primitive Encoding Methods

```typescript
byte(n: number): void {
    if (this.bufLength >= this.buf.length) {
        this.growBuffer();
    }
    this.buf[this.bufLength] = n & 0xff;
    this.bufLength++;
    // Auto-flush if buffer reaches chunk size
    if (this.bufLength >= this.chunkSize) {
        this.flush();
    }
}

u32(n: number): void {
    // big-endian (bincode configured)
    this.byte(n >>> 24); this.byte(n >>> 16); this.byte(n >>> 8); this.byte(n);
}

u64(n: bigint): void {
    // big-endian, manual byte extraction
    for (let i = 7; i >= 0; i--) this.byte(Number((n >> (BigInt(8 * i))) & 0xffn));
}
```

**Key observations:**
- **u32**: Written as 4 individual bytes in big-endian order using bitwise shifts. Does NOT use DataView.
- **u64**: Uses BigInt arithmetic to extract bytes. Written as 8 individual bytes in big-endian order.
- **No u8/u16/i16/i32/i64/f32/f64 primitives**: The protocol only needs `byte`, `u32`, `u64`, and `string`. Domain-specific types are composed from these.
- **Auto-flush**: The buffer automatically flushes to the stream when it reaches `chunkSize`.

### String Encoding

```typescript
/** Write UTF-8 string as: u64 length (BE) + bytes (bincode style). */
strUtf8(s: string): void {
    const bytes = Writer.enc.encode(s);
    this.u64(BigInt(bytes.length));
    this.bytes(bytes);
}
```

**Pattern**: Length prefix is a `u64` (8 bytes, big-endian), followed by raw UTF-8 bytes. The length is the **byte count**, not the character count.

### Raw Bytes

```typescript
bytes(b: Uint8Array): void {
    let offset = 0;
    while (offset < b.length) {
        const remainingChunk = this.chunkSize - this.bufLength;
        const remainingData = b.length - offset;
        const writeSize = Math.min(remainingChunk, remainingData);

        while (this.bufLength + writeSize > this.buf.length) {
            this.growBuffer();
        }

        this.buf.set(b.subarray(offset, offset + writeSize), this.bufLength);
        this.bufLength += writeSize;
        offset += writeSize;

        if (this.bufLength >= this.chunkSize) {
            this.flush();
        }
    }
}

/** Write Vec<u8>-like: u64 length (BE) + raw bytes. */
bytesPrefixed(b: Uint8Array): void {
    this.u64(BigInt(b.length));
    this.bytes(b);
}
```

### Buffer Management

```typescript
private growBuffer(): void {
    const newSize = this.buf.length * 2;
    const newBuf = new Uint8Array(newSize);
    newBuf.set(this.buf, 0);
    this.buf = newBuf;
}

flush(): void {
    if (this.bufLength > 0) {
        const chunk = this.buf.slice(0, this.bufLength);
        this.controller.enqueue(chunk);
        this.bufLength = 0;
    }
}
```

**Pattern**: Growable `Uint8Array` with doubling strategy. `flush()` copies the used portion and enqueues it to the stream, then resets `bufLength` to 0 (reuses the buffer).

### Frame Boundary

```typescript
async endFrame(): Promise<void> {
    this.frameNumber++;
    this.flush();
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
}
```

Every frame's `encode()` method calls `await w.endFrame()` at the end. This flushes the buffer and yields to the event loop, allowing consumers to process chunks.

---

## 4. Reader Implementation (`/Users/daniel/code/domcorder/proto-ts/src/reader.ts`)

### Interface

```typescript
interface BufferReader {
    readU32(): number;
    readU64(): bigint;
    readString(): string;
    readBytes(length: number): Uint8Array;
    readByte(): number;
}
```

This is the interface that all decode methods consume. The `Reader` class implements it.

### Class Structure

```typescript
export class Reader implements BufferReader {
    private buffer: Uint8Array;
    private bufferOffset: number = 0;
    private controller?: ReadableStreamDefaultController<Frame>;
    private stream: ReadableStream<Frame>;
    private static dec = new TextDecoder();

    private constructor(inputStream: ReadableStream<Uint8Array>, expectHeader: boolean) { ... }

    static create(
        inputStream: ReadableStream<Uint8Array>,
        expectHeader: boolean
    ): [Reader, ReadableStream<Frame>] { ... }
```

**Same factory pattern** as Writer. Returns `[Reader, ReadableStream<Frame>]`.

### Primitive Decoding Methods

```typescript
readByte(): number {
    if (this.availableBytes() < 1) {
        throw new Error("Not enough data for byte");
    }
    const value = this.buffer[this.bufferOffset];
    this.bufferOffset += 1;
    return value;
}

readU32(): number {
    if (this.availableBytes() < 4) {
        throw new Error("Not enough data for u32");
    }
    const view = new DataView(this.buffer.buffer, this.buffer.byteOffset + this.bufferOffset, 4);
    const value = view.getUint32(0, false); // big-endian
    this.bufferOffset += 4;
    return value;
}

readU64(): bigint {
    if (this.availableBytes() < 8) {
        throw new Error("Not enough data for u64");
    }
    const view = new DataView(this.buffer.buffer, this.buffer.byteOffset + this.bufferOffset, 8);
    const value = view.getBigUint64(0, false); // big-endian
    this.bufferOffset += 8;
    return value;
}

readString(): string {
    const length = Number(this.readU64());  // u64 length prefix
    if (this.availableBytes() < length) {
        throw new Error("Not enough data for string");
    }
    const bytes = this.buffer.slice(this.bufferOffset, this.bufferOffset + length);
    this.bufferOffset += length;
    return Reader.dec.decode(bytes);
}
```

**Key observations:**
- **DataView for reading**: Unlike the Writer (which uses manual byte shifts), the Reader uses `DataView.getUint32()` and `DataView.getBigUint64()` with `false` for big-endian.
- **Bounds checking on every read**: Every method checks `availableBytes()` first and throws if insufficient data.
- **Error messages start with "Not enough data"**: This is how the streaming parser distinguishes between "need more data" (recoverable) and actual errors.

### Streaming Buffer Management

```typescript
private appendToBuffer(newData: Uint8Array): void {
    const newBuffer = new Uint8Array(this.buffer.length + newData.length);
    newBuffer.set(this.buffer);
    newBuffer.set(newData, this.buffer.length);
    this.buffer = newBuffer;
}

private compactBuffer(): void {
    if (this.bufferOffset > 0) {
        const remaining = this.buffer.slice(this.bufferOffset);
        this.buffer = new Uint8Array(remaining.length);
        this.buffer.set(remaining);
        this.bufferOffset = 0;
    }
}
```

### Incremental Parsing with Backtracking

```typescript
private tryParseFrame(): boolean {
    if (this.availableBytes() < 4) return false;

    const startOffset = this.bufferOffset;

    try {
        this.frameNumber++;
        const frame = Frame.decode(this);
        if (frame === null) {
            throw new Error("Failed to decode frame - unknown or invalid frame type");
        }
        this.controller?.enqueue(frame);
        this.compactBuffer();
        return true;
    } catch (error) {
        // Restore offset on failure
        this.bufferOffset = startOffset;
        this.frameNumber--;

        if (error instanceof Error && error.message.startsWith("Not enough data")) {
            return false; // Recoverable: wait for more data
        }
        throw error; // Real parsing error: propagate
    }
}
```

**Critical pattern**: The reader attempts to parse a frame. If any read method throws "Not enough data", the offset is rolled back and the reader waits for more data from the stream. This enables parsing frames that arrive split across multiple chunks.

---

## 5. Type Encoding Patterns

### 5.1 Enum Variants (Frames)

Rust definition (`/Users/daniel/code/domcorder/proto-rs/src/frame.rs`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum Frame {
    Timestamp(TimestampData) = 0,
    Keyframe(KeyframeData) = 1,
    ViewportResized(ViewportResizedData) = 2,
    // ... etc
}
```

TypeScript encoding pattern -- each variant class writes its own variant index:

```typescript
// TS enum mirrors Rust variant indices
export enum FrameType {
    Timestamp = 0,
    Keyframe = 1,
    ViewportResized = 2,
    // ...
}

// Each frame class writes: u32(variant_index) + payload fields
export class Timestamp extends Frame {
    constructor(public timestamp: number | bigint) { super(); }

    async encode(w: Writer): Promise<void> {
        w.u32(FrameType.Timestamp);         // enum variant index as u32
        w.u64(toU64(this.timestamp));       // payload field
        await w.endFrame();
    }

    static decode(reader: BufferReader): Timestamp {
        if (reader.readU32() !== FrameType.Timestamp) throw new Error(`Expected Timestamp`);
        const timestamp = reader.readU64();
        return new Timestamp(timestamp);
    }
}
```

**Pattern**: `u32 variant index` + fields in declaration order. This matches bincode's serialization of `#[repr(u32)]` enums with `with_fixint_encoding()`.

### 5.2 Enum Dispatch (Decoding)

```typescript
type DecoderFn = (r: BufferReader) => Frame | null;
const DECODERS: (DecoderFn | undefined)[] = [];

// Populated at module level:
DECODERS[FrameType.Timestamp] = Timestamp.decode;
DECODERS[FrameType.Keyframe] = Keyframe.decode;
// ... etc

export abstract class Frame {
    static decode(reader: BufferReader): Frame | null {
        const t = reader.peekU32();       // Peek at variant index
        const dec = DECODERS[t];          // O(1) lookup
        if (!dec) return null;
        return dec(reader);               // Concrete decoder reads its own variant index
    }
}
```

**Pattern**: `peekU32()` to look at the variant index without consuming it, then dispatch to the concrete decoder, which reads and validates the variant index itself.

### 5.3 VNode (Nested Enum / Tagged Union)

Rust:
```rust
pub enum VNode {
    Element(VElement),                             // 0
    Text(VTextNode),                               // 1
    CData(VCDATASection),                          // 2
    Comment(VComment),                             // 3
    DocType(VDocumentType),                        // 4
    ProcessingInstruction(VProcessingInstruction), // 5
}
```

TypeScript:
```typescript
export enum DomNodeType {
    Element = 0, Text = 1, CData = 2, Comment = 3, DocType = 4, ProcessingInstruction = 5
}

export abstract class VNode {
    static decode(r: BufferReader): VNode {
        const nodeType = r.readU32();  // Read variant index
        switch (nodeType) {
            case DomNodeType.Element: return VElement.decode(r);
            case DomNodeType.Text: return VTextNode.decode(r);
            case DomNodeType.CData: return VCDATASection.decode(r);
            case DomNodeType.Comment: return VComment.decode(r);
            case DomNodeType.DocType: return VDocumentType.decode(r);
            case DomNodeType.ProcessingInstruction: return VProcessingInstruction.decode(r);
            default: throw new Error(`Unknown DOM node type: ${nodeType}`);
        }
    }
}
```

**Note**: Unlike `Frame.decode()` which uses `peekU32()` + concrete decoder reads its own index, `VNode.decode()` uses `readU32()` and the concrete decoders do NOT re-read the variant index. Both patterns work; the choice is about who "owns" reading the variant index.

### 5.4 Structs (Fields in Order)

Rust:
```rust
pub struct VElement {
    pub id: u32,
    pub tag: String,
    pub ns: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<VNode>,
}
```

TypeScript encode:
```typescript
encode(w: Writer): void {
    w.u32(DomNodeType.Element);   // enum variant index (written by parent)
    w.u32(this.id);               // field 1: u32
    w.strUtf8(this.tag.toLowerCase()); // field 2: String
    // field 3: Option<String>
    if (this.ns) {
        w.byte(1);               // Some tag
        w.strUtf8(this.ns);
    } else {
        w.byte(0);               // None tag
    }
    // field 4: Vec<(String, String)> -- encoded as length + pairs
    const attrEntries = Object.entries(this.attrs || {});
    w.u64(BigInt(attrEntries.length));
    for (const [name, value] of attrEntries) {
        w.strUtf8(name);
        w.strUtf8(value);
    }
    // field 5: Vec<VNode> -- length + recursive encoding
    const children = this.children || [];
    w.u64(BigInt(children.length));
    for (const child of children) {
        child.encode(w);
    }
}
```

TypeScript decode:
```typescript
static decode(r: BufferReader): VElement {
    const id = r.readU32();
    const tag = r.readString();
    // Option<String>
    const hasNamespace = r.readByte();
    let ns: string | undefined;
    if (hasNamespace === 1) {
        ns = r.readString();
    }
    // Vec<(String, String)>
    const attributeCount = Number(r.readU64());
    const attrs: Record<string, string> = {};
    for (let i = 0; i < attributeCount; i++) {
        const name = r.readString();
        const value = r.readString();
        attrs[name] = value;
    }
    // Vec<VNode>
    const childCount = Number(r.readU64());
    const children: VNode[] = [];
    for (let i = 0; i < childCount; i++) {
        children.push(VNode.decode(r));
    }
    return new VElement(id, tag, ns, attrs, children);
}
```

### 5.5 Option<T>

**Encoding**: 1 byte tag (0 = None, 1 = Some) + optional value.

```typescript
// Encode
if (this.mime) {
    w.byte(1); // Some flag
    w.strUtf8(this.mime);
} else {
    w.byte(0); // None flag
}

// Decode
const hasFlag = reader.readByte();
const mime = hasFlag === 1 ? reader.readString() : undefined;
```

**Important**: The tag is `readByte()` / `w.byte()`, which is a **single u8** (1 byte). This matches bincode's Option encoding.

### 5.6 Vec<T> (Arrays)

**Encoding**: `u64` length prefix (8 bytes) + elements in sequence.

```typescript
// Encode Vec<u32>
w.u64(BigInt(this.styleSheetIds.length));
for (const id of this.styleSheetIds) {
    w.u32(id);
}

// Decode Vec<u32>
const idsLength = Number(reader.readU64());
const styleSheetIds: number[] = [];
for (let i = 0; i < idsLength; i++) {
    styleSheetIds.push(reader.readU32());
}
```

### 5.7 Vec<u8> (Byte Buffers)

```typescript
// Encode: u64 length + raw bytes
const bytes = new Uint8Array(this.buf);
w.u64(BigInt(bytes.length));
w.bytes(bytes);

// Decode
const length = Number(reader.readU64());
const bytes = reader.readBytes(length);
const buf = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
```

### 5.8 Vec<(String, String)> (Attribute Maps)

Rust type `Vec<(String, String)>` is used for element attributes. In TypeScript, this maps to `Record<string, string>`, which is encoded/decoded as a Vec of pairs:

```typescript
// Encode
const attrEntries = Object.entries(attrs);
w.u64(BigInt(attrEntries.length));
for (const [name, value] of attrEntries) {
    w.strUtf8(name);
    w.strUtf8(value);
}

// Decode
const attributeCount = Number(r.readU64());
const attrs: Record<string, string> = {};
for (let i = 0; i < attributeCount; i++) {
    const name = r.readString();
    const value = r.readString();
    attrs[name] = value;
}
```

### 5.9 Bool

```typescript
// Encode
w.byte(this.altKey ? 1 : 0);

// Decode
const altKey = reader.readByte() === 1;
```

**One byte**: 0 = false, 1 = true. Same as bincode.

### 5.10 Nested Enum (TextOperationData)

Rust:
```rust
#[repr(u32)]
pub enum TextOperationData {
    Insert(TextInsertOperationData) = 0,
    Remove(TextRemoveOperationData) = 1,
}
```

TypeScript encode/decode:
```typescript
// Encode
for (const op of this.operations) {
    if (op.op === 'insert') {
        w.u32(0); // Insert variant = 0
        w.u32(op.index);
        w.strUtf8(op.text);
    } else { // 'remove'
        w.u32(1); // Remove variant = 1
        w.u32(op.index);
        w.u32(op.length);
    }
}

// Decode
const opType = reader.readU32();
if (opType === 0) { // Insert
    const index = reader.readU32();
    const text = reader.readString();
    operations.push({ op: 'insert', index, text });
} else { // Remove
    const index = reader.readU32();
    const length = reader.readU32();
    operations.push({ op: 'remove', index, length });
}
```

### 5.11 Unit Structs / Empty Frames

```typescript
// WindowFocused has no fields -- just the variant index
export class WindowFocused extends Frame {
    async encode(w: Writer): Promise<void> {
        w.u32(FrameType.WindowFocused);
        await w.endFrame();
    }

    static decode(reader: BufferReader): WindowFocused {
        if (reader.readU32() !== FrameType.WindowFocused) throw new Error(`Expected WindowFocused`);
        return new WindowFocused();
    }
}
```

---

## 6. Rust Side (`/Users/daniel/code/domcorder/proto-rs/`)

### Serde Derive + Bincode Configuration

The Rust side relies entirely on `#[derive(Serialize, Deserialize)]` and lets bincode handle the encoding. The Cargo.toml uses `bincode = "1.3"`.

From `/Users/daniel/code/domcorder/proto-rs/src/writer.rs`:
```rust
pub fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
    let config = bincode::DefaultOptions::new()
        .with_big_endian()
        .with_fixint_encoding();

    let encoded = config
        .serialize(frame)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    self.writer.write_all(&encoded)?;
    Ok(())
}
```

From `/Users/daniel/code/domcorder/proto-rs/src/reader.rs`:
```rust
let config = bincode::DefaultOptions::new()
    .with_big_endian()
    .with_fixint_encoding();

let mut cursor = std::io::Cursor::new(&self.buffer);
match config.deserialize_from(&mut cursor) {
    Ok(frame) => {
        let consumed = cursor.position() as usize;
        self.buffer.drain(..consumed);
        return Ok(Some(frame));
    }
    Err(e) => { ... }
}
```

### Type Definitions

Rust struct fields map 1:1 to TypeScript encode/decode order:

```rust
// Rust
pub struct KeyPressedData {
    pub code: String,
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}
```

```typescript
// TypeScript -- must match field order exactly
async encode(w: Writer): Promise<void> {
    w.u32(FrameType.KeyPressed);
    w.strUtf8(this.code);
    w.byte(this.altKey ? 1 : 0);
    w.byte(this.ctrlKey ? 1 : 0);
    w.byte(this.metaKey ? 1 : 0);
    w.byte(this.shiftKey ? 1 : 0);
    await w.endFrame();
}
```

### Rust VNode Enum Matches TS DomNodeType Enum

```rust
pub enum VNode {
    Element(VElement),                             // 0
    Text(VTextNode),                               // 1
    CData(VCDATASection),                          // 2
    Comment(VComment),                             // 3
    DocType(VDocumentType),                        // 4
    ProcessingInstruction(VProcessingInstruction), // 5
}
```

```typescript
export enum DomNodeType {
    Element = 0, Text = 1, CData = 2, Comment = 3, DocType = 4, ProcessingInstruction = 5
}
```

---

## 7. Error Handling

### Reader Error Strategy

The Reader uses a **try-parse-with-backtracking** approach:

1. Save current offset
2. Attempt to parse a complete frame
3. If any `read*()` method throws "Not enough data" -> restore offset, wait for more data
4. If any other error -> propagate as fatal

```typescript
readByte(): number {
    if (this.availableBytes() < 1) {
        throw new Error("Not enough data for byte");
    }
    // ...
}

readU32(): number {
    if (this.availableBytes() < 4) {
        throw new Error("Not enough data for u32");
    }
    // ...
}
```

The frame parser catches these:
```typescript
} catch (error) {
    this.bufferOffset = startOffset;
    this.frameNumber--;

    if (error instanceof Error && error.message.startsWith("Not enough data")) {
        return false; // Recoverable
    }
    throw error; // Fatal
}
```

### Error Types Found in Tests

From `/Users/daniel/code/domcorder/proto-ts/test/reader-errors.test.ts`:

- **Invalid magic bytes**: `"Invalid magic bytes: expected DCRR, got XXXX"`
- **Truncated stream**: `"Unexpected end of stream: incomplete frame data"`
- **Invalid frame type**: `"Failed to decode frame - unknown or invalid frame type"`
- **String with invalid length** (length exceeds available data): Caught by "Not enough data" -> eventually becomes "Unexpected end of stream"

---

## 8. Buffer Management Details

### Writer Buffer

- **Type**: `Uint8Array` with manual `bufLength` tracking
- **Growth**: Doubling strategy (`this.buf.length * 2`)
- **Flush**: Copies `buf.slice(0, bufLength)` to stream, resets `bufLength` to 0
- **Auto-flush**: When `bufLength >= chunkSize`, automatically flushes
- **No DataView for writing**: The writer uses manual byte shifts for u32/u64, and `Uint8Array.set()` for bulk bytes

### Reader Buffer

- **Type**: `Uint8Array` with `bufferOffset` tracking
- **Growth**: Concatenation (creates new buffer of combined size)
- **Compaction**: After parsing a frame, shifts remaining bytes to start of buffer
- **DataView for reading**: Creates transient `DataView` objects for each read operation:
  ```typescript
  const view = new DataView(this.buffer.buffer, this.buffer.byteOffset + this.bufferOffset, 4);
  const value = view.getUint32(0, false);
  ```
- **Endianness**: `false` = big-endian in DataView API

### Why Writer Doesn't Use DataView

The Writer builds bytes one at a time via `byte()` calls, which handles auto-flush at byte granularity. Using DataView would require ensuring the buffer has space for the full value before writing, and wouldn't work well with the streaming flush model. The Reader doesn't have this concern since it's always reading from a complete buffer.

---

## 9. Cross-Language Test Infrastructure

### Fixture-Based Testing

The project uses **blessed binary fixtures** stored at `/Users/daniel/code/domcorder/.sample_data/proto/`:
- `frames-basic.bin` -- Frame stream without header
- `file-basic.dcrr` -- Complete file with DCRR header + frames

### Workflow

1. **TypeScript generates binary**: `frames.test.ts` encodes sample frames, compares against blessed fixture
2. **Rust reads TypeScript binary**: `frames_test.rs` reads the same `.bin` file and validates all frames
3. **Rust also writes and reads back**: Round-trip test within Rust
4. **TypeScript also writes and reads back**: Round-trip test within TS

### Blessing System

From `/Users/daniel/code/domcorder/proto-ts/test/util.js`:

```javascript
export function compareBinaryFile(filename, actualBuffer, testName) {
    const expectedFile = join(projectRoot, ".sample_data", "proto", filename);
    const shouldUpdate = process.env.PROTO_TEST_UPDATE === testName;

    if (existsSync(expectedFile)) {
        // Byte-by-byte comparison with hex dump on mismatch
        // ...
    }

    if (shouldUpdate) {
        writeFileSync(expectedFile, actualBuffer);
        console.log(`Updated expected file (${actualBuffer.length} bytes)`);
        return true;
    }
}
```

Usage: `PROTO_TEST_UPDATE=frames-basic bun test` to update the blessed file.

### Shared Test Data

Both Rust and TS define identical test data structures:

**TypeScript** (`/Users/daniel/code/domcorder/proto-ts/test/sample-frames.ts`):
```typescript
export const testVDocument = new VDocument(0, [], [
    new VDocumentType(1, "html", undefined, undefined),
    new VElement(2, "html", undefined, {}, [
        new VElement(3, "head", undefined, {}, [
            new VTextNode(4, "\n    "),
            new VElement(5, "meta", undefined, { "charset": "utf-8" }, []),
            // ...
        ]),
        // ...
    ])
]);
```

**Rust** (`/Users/daniel/code/domcorder/proto-rs/tests/common.rs`):
```rust
pub fn sample_frames() -> Vec<Frame> {
    vec![
        Frame::Timestamp(TimestampData { timestamp: 1722550000000 }),
        Frame::Keyframe(KeyframeData {
            document: VDocument {
                id: 0,
                adopted_style_sheets: vec![],
                children: vec![
                    VNode::DocType(VDocumentType { id: 1, name: "html".to_string(), ... }),
                    VNode::Element(VElement { id: 2, tag: "html".to_string(), ... }),
                ],
            },
            viewport_width: 1920,
            viewport_height: 1080,
        }),
        // ... 20+ more frames covering every type
    ]
}
```

---

## 10. Complete Encoding Format Summary

| Type | Encoding | Bytes |
|------|----------|-------|
| `u8` / `bool` / `Option` tag | Single byte | 1 |
| `u32` / enum variant index | 4 bytes, big-endian (in domcorder) | 4 |
| `u64` / length prefix | 8 bytes, big-endian (in domcorder) | 8 |
| `String` | u64 byte-length + UTF-8 bytes | 8 + N |
| `Vec<T>` | u64 element-count + elements | 8 + sum(elements) |
| `Vec<u8>` | u64 byte-length + raw bytes | 8 + N |
| `Option<T>` | u8 tag (0/1) + optional value | 1 or 1+sizeof(T) |
| `enum Foo { A(X) = 0, B(Y) = 1 }` | u32 variant + payload | 4 + payload |
| struct fields | Fields in declaration order, no separators | sum(fields) |
| Tuple `(A, B)` | A then B, no length prefix | sizeof(A) + sizeof(B) |
| `bool` | u8 (0 or 1) | 1 |

---

## 11. Key Differences from ankurah's Needs

| Aspect | Domcorder | ankurah |
|--------|-----------|---------|
| **Endianness** | Big-endian (`with_big_endian()`) | Little-endian (default) |
| **Integer types used** | Only u8, u32, u64 | u8, u16, u32, u64, i16, i32, i64, f64 |
| **Container types** | Vec, Option | Vec, Option, BTreeMap |
| **Fixed arrays** | Not used | `[u8; 16]`, `[u8; 32]` (no length prefix) |
| **Streaming** | Yes (ReadableStream-based) | Probably not needed for wire messages |
| **Custom serde** | None (all derive) | EntityId (raw bytes), json_as_bytes |
| **Recursive types** | VNode tree (limited depth) | Predicate/Expr AST (arbitrary depth) |

### What to Adopt from Domcorder

1. **Hand-rolled per-type encode/decode** co-located with type classes
2. **BincodeReader/BincodeWriter as primitive layer** with bounds-checked reads
3. **Error pattern**: "Not enough data" errors for streaming, propagated errors for structural issues
4. **Fixture-based cross-language testing** with blessing system
5. **Shared test data** in both Rust and TS
6. **u64 length prefix** pattern: `Number(reader.readU64())` for array/string lengths

### What to Change for ankurah

1. **Little-endian**: Use `DataView` with `true` for little-endian parameter
2. **Add primitive methods**: `readU16()`, `readI16()`, `readI32()`, `readI64()`, `readF64()`, etc.
3. **Add `readFixedBytes(n)`**: For `[u8; N]` types (EntityId, EventId) -- no length prefix
4. **Add `readMap()`**: For `BTreeMap<K, V>` -- u64 count + key-value pairs
5. **Non-streaming Reader**: For wire messages, a simple cursor-over-ArrayBuffer is sufficient (no need for ReadableStream chunking)
6. **BTreeMap write ordering**: Must sort keys lexicographically for byte-level compatibility

---

## 12. Full Source Reference

All source files read in this analysis:

- `/Users/daniel/code/domcorder/package.json`
- `/Users/daniel/code/domcorder/Cargo.toml`
- `/Users/daniel/code/domcorder/proto-ts/package.json`
- `/Users/daniel/code/domcorder/proto-ts/tsconfig.json`
- `/Users/daniel/code/domcorder/proto-ts/src/index.ts`
- `/Users/daniel/code/domcorder/proto-ts/src/writer.ts`
- `/Users/daniel/code/domcorder/proto-ts/src/reader.ts`
- `/Users/daniel/code/domcorder/proto-ts/src/frames.ts`
- `/Users/daniel/code/domcorder/proto-ts/src/vdom.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/sample-frames.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/stream-observer.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/util.js`
- `/Users/daniel/code/domcorder/proto-ts/test/frames.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/file-format.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/reader-basic.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/reader-errors.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/reader-roundtrip.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/async-frames.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/stream-observer.test.ts`
- `/Users/daniel/code/domcorder/proto-ts/test/TASKS.md`
- `/Users/daniel/code/domcorder/proto-rs/Cargo.toml`
- `/Users/daniel/code/domcorder/proto-rs/src/lib.rs`
- `/Users/daniel/code/domcorder/proto-rs/src/frame.rs`
- `/Users/daniel/code/domcorder/proto-rs/src/vdom.rs`
- `/Users/daniel/code/domcorder/proto-rs/src/reader.rs`
- `/Users/daniel/code/domcorder/proto-rs/src/writer.rs`
- `/Users/daniel/code/domcorder/proto-rs/tests/common.rs`
- `/Users/daniel/code/domcorder/proto-rs/tests/frames_test.rs`
