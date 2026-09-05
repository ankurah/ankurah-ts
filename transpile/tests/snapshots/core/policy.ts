// MIRRORS: ankurah/core/src/policy.rs
import { Struct, Enum, Result, tracing, HashMap, HashSet } from '@ankurah/base';
import { ParseError, Predicate } from '@ankurah/ankql';
import { Attested, Attestation, AuthData, CausalAssertion, CollectionId, EntityId, EntityState, Event, NodeRequest } from '@ankurah/proto';
import { Entity } from './entity';
import { ValidationError } from './error';
import { ContextData, Node, NodeInner } from './node';
import { PropertyError } from './property/traits';
import { State } from './reactor/subscription_state';
import { StorageEngine } from './storage';
import { Iterable_dispatch_iterable } from './util/iterable';

export class PermissiveAgent extends Struct implements PolicyAgent {

  static new(): PermissiveAgent {
    return new PermissiveAgent();
  }

  static default(): PermissiveAgent {
    return PermissiveAgent.new();
  }

  signRequest<SE extends StorageEngine, C>(_node: NodeInner<SE, PermissiveAgent>, cdata: C, _request: NodeRequest): Result<AuthData[], AccessDenied> {
    tracing.debug(`PermissiveAgent sign_request: ${_request.debug()}`);
    return Result.Ok(Iterable_dispatch_iterable(cdata).map((_) => proto.AuthData([])));
  }

  async checkRequest<SE extends StorageEngine, A>(_node: Node<SE, PermissiveAgent>, auth: A, _request: NodeRequest): Promise<Result<DefaultContext[], ValidationError>> {
    return Result.Ok(Iterable_dispatch_iterable(auth).map((_) => DEFAULT_CONTEXT));
  }

  checkEvent<SE extends StorageEngine>(_node: Node<SE, PermissiveAgent>, _cdata: DefaultContext, _entityBefore: Entity, _entityAfter: Entity, _event: Event): Result<Attestation | null, AccessDenied> {
    return Result.Ok(null);
  }

  validateReceivedEvent<SE extends StorageEngine>(_node: Node<SE, PermissiveAgent>, _fromNode: EntityId, _event: Attested<Event>): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  attestState<SE extends StorageEngine>(_node: Node<SE, PermissiveAgent>, _state: EntityState): Attestation | null {
    return null;
  }

  validateReceivedState<SE extends StorageEngine>(_node: Node<SE, PermissiveAgent>, _fromNode: EntityId, _state: Attested<EntityState>): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  canAccessCollection<C>(_data: C, _collection: CollectionId): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  checkRead<C>(_data: C, _id: EntityId, _collection: CollectionId, _state: State): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  checkReadEvent<C>(_data: C, _event: Attested<Event>): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  checkWrite(_context: DefaultContext, _entity: Entity, _event: Event | null): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  validateCausalAssertion<SE extends StorageEngine>(_node: Node<SE, PermissiveAgent>, _peerId: EntityId, _headRelation: CausalAssertion): Result<void, AccessDenied> {
    return Result.Ok([]);
  }

  filterPredicate<C>(_data: C, _collection: CollectionId, predicate: Predicate): Result<Predicate, AccessDenied> {
    return Result.Ok(predicate);
  }

  clone(): PermissiveAgent {
    return new PermissiveAgent();
  }
}

export class DefaultContext extends Struct implements ContextData {

  static new(): DefaultContext {
    return new DefaultContext();
  }

  static default(): DefaultContext {
    return DefaultContext.new();
  }

  equals(other: DefaultContext): boolean {
    return true;
  }

  /** The key hash `HashMap` and `HashSet` file this under. */
  hash(): string {
    return [].join('|');
  }

  clone(): DefaultContext {
    return new DefaultContext();
  }

  debug(): string {
    return 'DefaultContext';
  }
}

export type AccessDeniedV = {
  ByPolicy: { _0: string };
  CollectionDenied: { _0: CollectionId };
  PropertyError: { _0: PropertyError };
  ParseError: { _0: ParseError };
  InsufficientAttestation: {};
};

export class AccessDenied extends Enum<AccessDeniedV> {

  static fromPropertyError(error: PropertyError): AccessDenied {
    return new AccessDenied('PropertyError', { _0: error });
  }

  static fromParseError(error: ParseError): AccessDenied {
    return new AccessDenied('ParseError', { _0: error });
  }

  debug(): string {
    return this.match({
      ByPolicy: (v) => `ByPolicy(${JSON.stringify(v._0)})`,
      CollectionDenied: (v) => `CollectionDenied(${v._0.debug()})`,
      PropertyError: (v) => `PropertyError(${v._0.debug()})`,
      ParseError: (v) => `ParseError(${v._0.debug()})`,
      InsufficientAttestation: () => 'InsufficientAttestation',
    });
  }

  override toString(): string {
    return this.match({
      ByPolicy: (v) => `Access denied by policy: ${v._0}`,
      CollectionDenied: (v) => `Access denied by collection: ${v._0}`,
      PropertyError: (v) => `Access denied by property error: ${v._0}`,
      ParseError: (v) => `Access denied by parse error: ${v._0}`,
      InsufficientAttestation: () => 'Insufficient attestation',
    });
  }
}

export interface PolicyAgent {
  signRequest(node: NodeInner<SE, Self>, cdata: C, request: NodeRequest): Result<AuthData[], AccessDenied>;
  checkRequest(node: Node<SE, Self>, auth: A, request: NodeRequest): Promise<Result<ContextData[], ValidationError>>;
  checkEvent(node: Node<SE, Self>, cdata: ContextData, entityBefore: Entity, entityAfter: Entity, event: Event): Result<Attestation | null, AccessDenied>;
  validateReceivedEvent(node: Node<SE, Self>, receivedFromNode: EntityId, event: Attested<Event>): Result<void, AccessDenied>;
  attestState(node: Node<SE, Self>, state: EntityState): Attestation | null;
  validateReceivedState(node: Node<SE, Self>, receivedFromNode: EntityId, state: Attested<EntityState>): Result<void, AccessDenied>;
  canAccessCollection(data: C, collection: CollectionId): Result<void, AccessDenied>;
  filterPredicate(data: C, collection: CollectionId, predicate: Predicate): Result<Predicate, AccessDenied>;
  checkRead(data: C, id: EntityId, collection: CollectionId, state: State): Result<void, AccessDenied>;
  checkReadEvent(data: C, event: Attested<Event>): Result<void, AccessDenied>;
  checkWrite(data: ContextData, entity: Entity, event: Event | null): Result<void, AccessDenied>;
  validateCausalAssertion(node: Node<SE, Self>, peerId: EntityId, headRelation: CausalAssertion): Result<void, AccessDenied>;
}

export const DEFAULT_CONTEXT: DefaultContext = new DefaultContext();

