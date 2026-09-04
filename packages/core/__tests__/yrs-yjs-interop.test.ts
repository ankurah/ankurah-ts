// MIRRORS: ankurah/core/src/property/backend/yrs.rs
//
// The yrs_v2 fixtures are Yrs 0.24 documents in the lib0 v2 update format — the
// exact bytes `YrsBackend::to_state_buffer` and `to_operations` produce. Yjs is
// what the TypeScript port has in their place, so these tests are the proof that
// the two libraries agree: Yjs must reconstruct the same text and reach the same
// state vector from bytes Yrs wrote.
//
// Every fixture carries a sidecar naming the document state *after* the update is
// applied — its `text_fields` and its `state_vector`, and for a diff, the base it
// applies on top of. The suite walks that manifest rather than naming fixtures, so
// a fixture added on the Rust side is covered the next time this runs.
//
// The trap the sidecars exist to catch: yrs's default `OffsetKind::Bytes` positions
// text edits by UTF-8 byte offset, while Yjs positions by UTF-16 code unit. Any
// port that computes an insert position from `string.length` lands inside a
// multi-byte sequence. `unicode_text.bin` is where that shows.

import { describe, test, expect } from 'bun:test';
import * as Y from 'yjs';
import { existsSync } from 'fs';

import { fixturePath, listFixtureDir, readFixtureBytes, readSidecar, toHex } from '../../proto/__tests__/support/fixtures';

const YRS_DIR = 'proto/test_fixtures/yrs_v2';

interface YrsSidecar {
  fixture: string;
  total_len: number;
  applies_on_top_of: string | null;
  text_fields?: Record<string, string>;
  state_vector?: Record<string, number>;
  bytes?: number[];
}

function loadFixture(name: string): Uint8Array {
  return readFixtureBytes(YRS_DIR, name);
}

/** The doc's state vector as {clientId: clock}, the shape the sidecars use. */
function stateVectorOf(doc: Y.Doc): Record<string, number> {
  const sv = Y.decodeStateVector(Y.encodeStateVector(doc));
  const out: Record<string, number> = {};
  for (const [client, clock] of sv) out[String(client)] = clock;
  return out;
}

const fixtures = listFixtureDir(YRS_DIR)
  .filter((f) => f.endsWith('.bin'))
  .filter((f) => existsSync(fixturePath(YRS_DIR, f.replace(/\.bin$/, '.json'))));

if (fixtures.length === 0) {
  throw new Error(`No .bin/.json fixture pairs under ${fixturePath(YRS_DIR)}`);
}

describe('Yrs V2 fixtures applied by Yjs', () => {
  for (const binName of fixtures) {
    const bytes = loadFixture(binName);
    const sidecar = readSidecar(YRS_DIR, binName.replace(/\.bin$/, '.json')) as YrsSidecar;

    describe(binName, () => {
      test('file length matches the sidecar', () => {
        expect(bytes.length).toBe(sidecar.total_len);
      });

      if (sidecar.bytes) {
        test('bytes match the sidecar', () => {
          expect(toHex(bytes)).toBe(toHex(new Uint8Array(sidecar.bytes!)));
        });
      }

      test('text fields after applying', () => {
        const doc = new Y.Doc();
        if (sidecar.applies_on_top_of) Y.applyUpdateV2(doc, loadFixture(sidecar.applies_on_top_of));
        Y.applyUpdateV2(doc, bytes);

        const expected = sidecar.text_fields ?? {};
        // Read the roots before touching any of them: getText() creates a missing
        // root, which would hide a diff that failed to introduce a new field.
        expect(Array.from(doc.share.keys()).sort()).toEqual(Object.keys(expected).sort());
        for (const [field, text] of Object.entries(expected)) {
          expect(doc.getText(field).toString()).toBe(text);
        }
      });

      test('state vector after applying', () => {
        const doc = new Y.Doc();
        if (sidecar.applies_on_top_of) Y.applyUpdateV2(doc, loadFixture(sidecar.applies_on_top_of));
        Y.applyUpdateV2(doc, bytes);
        expect(stateVectorOf(doc)).toEqual(sidecar.state_vector ?? {});
      });
    });
  }
});

describe('Yrs V2 fixture invariants', () => {
  // `YrsBackend::to_operations` decides a backend produced nothing by comparing
  // its output against `Update::EMPTY_V2`, which is byte-identical to the encoding
  // of a document with no roots and no edits. That equality is the invariant.
  test('empty_update.bin is byte-identical to empty_doc.bin', () => {
    expect(toHex(loadFixture('empty_update.bin'))).toBe(toHex(loadFixture('empty_doc.bin')));
  });

  test('empty_update.bin applied to a populated doc is a no-op', () => {
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, loadFixture('simple_text.bin'));
    const before = doc.getText('content').toString();
    Y.applyUpdateV2(doc, loadFixture('empty_update.bin'));
    expect(doc.getText('content').toString()).toBe(before);
  });

  // Text indices are UTF-8 byte offsets, so the Rust edit positions are past where
  // a UTF-16-indexed port would put them. If Yjs disagreed with yrs about that, the
  // rendered string would differ — which is what the text_fields assertion above
  // checks. This pins the specific string so the intent is visible in the failure.
  test('unicode_text.bin renders the byte-offset result', () => {
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, loadFixture('unicode_text.bin'));
    expect(doc.getText('content').toString()).toBe('café 日本 🚀');
  });

  // Deleting all the text leaves the root and the delete set behind, so the bytes
  // are nothing like the empty document even though both render as "".
  test('fully_deleted_text.bin is not the empty document', () => {
    const deleted = loadFixture('fully_deleted_text.bin');
    expect(toHex(deleted)).not.toBe(toHex(loadFixture('empty_doc.bin')));
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, deleted);
    expect(doc.getText('content').toString()).toBe('');
    expect(Array.from(doc.share.keys())).toEqual(['content']);
  });

  // The merged string pins the conflict-resolution rule — order by client id —
  // not just the layout. Yrs resolves the two concurrent inserts at index 0 as
  // "HelloWorld"; a port that ties differently produces "WorldHello".
  test('concurrent_merge.bin resolves by client id', () => {
    const doc = new Y.Doc();
    Y.applyUpdateV2(doc, loadFixture('concurrent_merge.bin'));
    expect(doc.getText('content').toString()).toBe('HelloWorld');
  });
});

describe('Yjs reproduces the Yrs scenarios', () => {
  // Two clients insert at index 0 without seeing each other, then merge. Yjs must
  // land on the same merged string as the Yrs fixture, or the two libraries
  // disagree about conflict resolution and the port cannot interoperate.
  test('two Yjs docs merging concurrently match concurrent_merge.bin', () => {
    const docA = new Y.Doc(); docA.clientID = 10;
    const docB = new Y.Doc(); docB.clientID = 20;

    docA.getText('content').insert(0, 'Hello');
    docB.getText('content').insert(0, 'World');

    const stateA = Y.encodeStateAsUpdateV2(docA);
    const stateB = Y.encodeStateAsUpdateV2(docB);
    Y.applyUpdateV2(docA, stateB);
    Y.applyUpdateV2(docB, stateA);

    const fixtureDoc = new Y.Doc();
    Y.applyUpdateV2(fixtureDoc, loadFixture('concurrent_merge.bin'));
    const fixtureContent = fixtureDoc.getText('content').toString();

    expect(docA.getText('content').toString()).toBe(docB.getText('content').toString());
    expect(docA.getText('content').toString()).toBe(fixtureContent);
  });

  // `to_state_buffer` is a full-state encode and `to_operations` a diff against the
  // previously-seen state vector. This is that pair driven through Yjs, checked
  // against the Yrs fixture that recorded the same pair.
  test('a Yjs full state plus a Yjs diff reaches the incremental fixture content', () => {
    const doc = new Y.Doc();
    const text = doc.getText('content');

    text.insert(0, 'Hello');
    const baseState = Y.encodeStateAsUpdateV2(doc);
    const seen = Y.encodeStateVector(doc);

    text.insert(5, ', World!');
    const diff = Y.encodeStateAsUpdateV2(doc, seen);

    const replay = new Y.Doc();
    Y.applyUpdateV2(replay, baseState);
    expect(replay.getText('content').toString()).toBe('Hello');
    Y.applyUpdateV2(replay, diff);

    const fixtureDoc = new Y.Doc();
    Y.applyUpdateV2(fixtureDoc, loadFixture('incremental_base.bin'));
    Y.applyUpdateV2(fixtureDoc, loadFixture('incremental_diff.bin'));

    expect(replay.getText('content').toString()).toBe(fixtureDoc.getText('content').toString());
  });

  // Yjs and Yrs do not promise byte-identical v2 output for the same document, so
  // this asserts what a port actually depends on: re-encoding a Yrs document with
  // Yjs preserves both its content and its state vector.
  test('re-encoding a Yrs document with Yjs preserves content and state vector', () => {
    for (const binName of fixtures) {
      const sidecar = readSidecar(YRS_DIR, binName.replace(/\.bin$/, '.json')) as YrsSidecar;
      const doc = new Y.Doc();
      if (sidecar.applies_on_top_of) Y.applyUpdateV2(doc, loadFixture(sidecar.applies_on_top_of));
      Y.applyUpdateV2(doc, loadFixture(binName));

      const reEncoded = new Y.Doc();
      Y.applyUpdateV2(reEncoded, Y.encodeStateAsUpdateV2(doc));

      for (const [field, text] of Object.entries(sidecar.text_fields ?? {})) {
        expect(`${binName}:${field}=${reEncoded.getText(field).toString()}`).toBe(`${binName}:${field}=${text}`);
      }
      expect(stateVectorOf(reEncoded)).toEqual(stateVectorOf(doc));
    }
  });
});
