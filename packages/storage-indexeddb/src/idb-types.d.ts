// DOM IndexedDB type declarations for environments without DOM lib.
// These are normally provided by TypeScript's "dom" lib, but the root
// tsconfig.json only includes "ES2021". This file provides the subset
// of IndexedDB types used by this package.

// Divergence: Rust uses wasm-bindgen + web_sys for these types. [E16]
// In TS, we declare the subset we need for non-DOM environments.

declare var indexedDB: IDBFactory;

interface IDBFactory {
  open(name: string, version?: number): IDBOpenDBRequest;
  deleteDatabase(name: string): IDBOpenDBRequest;
}

interface IDBDatabase extends EventTarget {
  readonly name: string;
  readonly version: number;
  readonly objectStoreNames: DOMStringList;
  close(): void;
  createObjectStore(name: string, optionalParameters?: IDBObjectStoreParameters): IDBObjectStore;
  transaction(storeNames: string | string[], mode?: IDBTransactionMode): IDBTransaction;
  onversionchange: ((this: IDBDatabase, ev: IDBVersionChangeEvent) => any) | null;
}

interface IDBObjectStoreParameters {
  keyPath?: string | string[] | null;
  autoIncrement?: boolean;
}

interface IDBTransaction extends EventTarget {
  readonly objectStoreNames: DOMStringList;
  objectStore(name: string): IDBObjectStore;
  abort(): void;
}

interface IDBObjectStore {
  readonly indexNames: DOMStringList;
  readonly name: string;
  add(value: any, key?: IDBValidKey): IDBRequest;
  clear(): IDBRequest;
  createIndex(name: string, keyPath: string | string[], optionalParameters?: IDBIndexParameters): IDBIndex;
  delete(key: IDBValidKey | IDBKeyRange): IDBRequest;
  get(key: IDBValidKey | IDBKeyRange): IDBRequest;
  index(name: string): IDBIndex;
  put(value: any, key?: IDBValidKey): IDBRequest;
  openCursor(query?: IDBValidKey | IDBKeyRange | null, direction?: IDBCursorDirection): IDBRequest<IDBCursorWithValue | null>;
}

interface IDBIndexParameters {
  multiEntry?: boolean;
  unique?: boolean;
}

interface IDBIndex {
  readonly name: string;
  openCursor(query?: IDBValidKey | IDBKeyRange | null, direction?: IDBCursorDirection): IDBRequest<IDBCursorWithValue | null>;
  openKeyCursor(query?: IDBValidKey | IDBKeyRange | null, direction?: IDBCursorDirection): IDBRequest<IDBCursor | null>;
  get(key: IDBValidKey | IDBKeyRange): IDBRequest;
}

interface IDBRequest<T = any> extends EventTarget {
  readonly error: DOMException | null;
  readonly result: T;
  readonly transaction: IDBTransaction | null;
  onsuccess: ((this: IDBRequest<T>, ev: Event) => any) | null;
  onerror: ((this: IDBRequest<T>, ev: Event) => any) | null;
}

interface IDBOpenDBRequest extends IDBRequest<IDBDatabase> {
  onupgradeneeded: ((this: IDBOpenDBRequest, ev: IDBVersionChangeEvent) => any) | null;
  onblocked: ((this: IDBOpenDBRequest, ev: Event) => any) | null;
}

interface IDBVersionChangeEvent extends Event {
  readonly oldVersion: number;
  readonly newVersion: number | null;
}

interface IDBCursor {
  readonly direction: IDBCursorDirection;
  readonly key: IDBValidKey;
  readonly primaryKey: IDBValidKey;
  advance(count: number): void;
  continue(key?: IDBValidKey): void;
  delete(): IDBRequest;
  update(value: any): IDBRequest;
}

interface IDBCursorWithValue extends IDBCursor {
  readonly value: any;
}

interface IDBKeyRange {
  readonly lower: any;
  readonly upper: any;
  readonly lowerOpen: boolean;
  readonly upperOpen: boolean;
  includes(key: any): boolean;
}

declare var IDBKeyRange: {
  prototype: IDBKeyRange;
  bound(lower: any, upper: any, lowerOpen?: boolean, upperOpen?: boolean): IDBKeyRange;
  lowerBound(lower: any, open?: boolean): IDBKeyRange;
  upperBound(upper: any, open?: boolean): IDBKeyRange;
  only(value: any): IDBKeyRange;
};

type IDBValidKey = number | string | Date | BufferSource | IDBValidKey[];
type IDBCursorDirection = 'next' | 'nextunique' | 'prev' | 'prevunique';
type IDBTransactionMode = 'readonly' | 'readwrite' | 'versionchange';

interface DOMStringList {
  readonly length: number;
  contains(string: string): boolean;
  item(index: number): string | null;
  [index: number]: string;
}

interface DOMException extends Error {
  readonly code: number;
  readonly name: string;
  readonly message: string;
}
