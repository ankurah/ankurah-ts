// MIRRORS: ankurah/core/src/reactor/property_path.rs

import type { PathExpr } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import { extractAtPath } from '../value/index.ts';
import type { Entity } from '../entity.ts';

// Rust: pub struct PropertyPath { root: String, sub_path: Vec<String> }
// Derives: Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash

export class PropertyPath {
  // Rust: root: String
  readonly root: string;
  // Rust: sub_path: Vec<String>
  readonly subPath: string[];

  constructor(root: string, subPath: string[] = []) {
    this.root = root;
    this.subPath = subPath;
  }

  // Rust: pub fn from_path(path: &ankql::ast::PathExpr) -> Self
  static fromPath(path: PathExpr): PropertyPath {
    const steps = path.steps;
    return new PropertyPath(steps[0], steps.slice(1));
  }

  // Rust: impl From<&str> for PropertyPath
  static fromString(val: string): PropertyPath {
    return new PropertyPath(val);
  }

  // Rust: pub fn root(&self) -> &str
  getRoot(): string {
    return this.root;
  }

  // Rust: pub fn is_simple(&self) -> bool
  isSimple(): boolean {
    return this.subPath.length === 0;
  }

  // Rust: pub fn extract_value<E: super::AbstractEntity>(&self, entity: &E) -> Option<Value>
  // Divergence: Concrete Entity instead of generic E: AbstractEntity [E8].
  extractValue(entity: Entity): Value | null {
    const rootValue = entity.getPropertyValue(this.root);
    if (rootValue === null) {
      return null;
    }
    if (this.subPath.length === 0) {
      return rootValue;
    }
    // Extract nested value from JSON/Binary, keeping it wrapped as Value::Json to match index keys
    return extractAtPath(rootValue, this.subPath);
  }

  toString(): string {
    if (this.subPath.length === 0) {
      return this.root;
    }
    return this.root + '.' + this.subPath.join('.');
  }
}
