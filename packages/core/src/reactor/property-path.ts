// MIRRORS: ankurah/core/src/reactor/property_path.rs

import type { PathExpr } from '@ankurah/ankql';
import type { Value } from '../value/index.ts';
import { extractAtPath } from '../value/index.ts';
import type { Entity } from '../entity.ts';

/**
 * A path to a property value, supporting both simple fields and JSON sub-paths.
 * Used by the watcher system to index and extract values for comparison.
 */
export class PropertyPath {
  /** The root property name (e.g., "context" for "context.task_id") */
  readonly root: string;
  /** The sub-path within the property (e.g., ["task_id"] for "context.task_id"), empty for simple fields */
  readonly subPath: string[];

  constructor(root: string, subPath: string[] = []) {
    this.root = root;
    this.subPath = subPath;
  }

  /** Create a PropertyPath from a PathExpr */
  static fromPath(path: PathExpr): PropertyPath {
    const steps = path.steps;
    return new PropertyPath(steps[0], steps.slice(1));
  }

  /** Create a PropertyPath from a dotted string (e.g., "context.task_id") */
  static fromString(val: string): PropertyPath {
    return new PropertyPath(val);
  }

  /** Check if this is a simple field (no sub-path) */
  isSimple(): boolean {
    return this.subPath.length === 0;
  }

  /**
   * Extract the value at this path from an entity.
   * For JSON paths, keeps the value wrapped to match index keys.
   */
  extractValue(entity: Entity): Value | null {
    const rootValue = entity.getPropertyValue(this.root);
    if (rootValue === null) {
      return null;
    }
    if (this.subPath.length === 0) {
      return rootValue;
    }
    // Extract nested value from JSON/Binary, delegating to extractAtPath
    return extractAtPath(rootValue, this.subPath);
  }

  toString(): string {
    if (this.subPath.length === 0) {
      return this.root;
    }
    return this.root + '.' + this.subPath.join('.');
  }
}
