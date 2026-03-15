// MIRRORS: ankurah/core/src/traits.rs

/**
 * Optional trait that allows storage operations to be scoped to a specific namespace.
 * For multitenancy or otherwise. Presumably the Context will implement this trait.
 * Storage engines may implement namespace-aware storage to partition data.
 *
 * Rust: `pub trait Namespace`
 */
export interface Namespace {
  /** Returns the namespace for this context, if any. */
  namespace(): string | null;
}
