// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs #[cfg(test)] mod tests
import { describe, test, expect } from 'bun:test';
import { QueryId } from '@ankurah/proto';
import { CandidateChanges } from './candidate_changes.ts';

// ── Tests ──

describe('candidate_changes', () => {
  // Rust: fn test_candidate_changes_empty()
  test('candidate changes empty', () => {
    const changes: number[] = [];
    const candidates = new CandidateChanges(changes);
    expect(candidates.isEmpty()).toBe(true);
    expect(candidates.queryCount()).toBe(0);
  });

  // Rust: fn test_candidate_changes_add_query()
  test('candidate changes add query', () => {
    const changes = [10, 20, 30, 40, 50];
    const candidates = new CandidateChanges(changes);

    const q1 = QueryId.new();
    const q2 = QueryId.new();

    candidates.addQuery(q1, 1); // 20
    candidates.addQuery(q1, 3); // 40
    candidates.addQuery(q2, 0); // 10

    expect(candidates.queryCount()).toBe(2);
    expect(candidates.isEmpty()).toBe(false);

    const queryMap = new Map<string, number[]>();
    for (const qc of candidates.queryIter()) {
      queryMap.set(qc.queryId.toUlidString(), qc.iter());
    }

    expect(queryMap.get(q1.toUlidString())).toEqual([20, 40]);
    expect(queryMap.get(q2.toUlidString())).toEqual([10]);
  });

  // Rust: fn test_candidate_changes_entity_level()
  test('candidate changes entity level', () => {
    const changes = [10, 20, 30];
    const candidates = new CandidateChanges(changes);

    candidates.addEntity(0);
    candidates.addEntity(2);

    const entities = candidates.entityIter();
    expect(entities).toEqual([10, 30]);
  });
});
