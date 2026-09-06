// MIRRORS: ankurah/core/src/reactor/candidate_changes.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { CandidateChanges } from './candidate_changes';
import { Arc, HashMap } from '@ankurah/base';
import { QueryId } from '@ankurah/proto';

describe('candidate_changes unit tests', () => {
  test('test_candidate_changes_empty', () => {
    const changes = Arc.from([]);
    const candidates = CandidateChanges.new(changes);
    if (!(candidates.length === 0)) throw new Error('assertion failed');
    expect(candidates.queryCount()).toEqual(0);
  });

  test('test_candidate_changes_add_query', () => {
    const changes = Arc.new([10, 20, 30, 40, 50]);
    let candidates = CandidateChanges.new(changes);
    const q1 = QueryId.new();
    const q2 = QueryId.new();
    candidates.addQuery(q1, 1);
    candidates.addQuery(q1, 3);
    candidates.addQuery(q2, 0);
    expect(candidates.queryCount()).toEqual(2);
    if (!(!(candidates.length === 0))) throw new Error('assertion failed');
    let queryMap = new HashMap<QueryId, number[]>();
    for (const qc of candidates.queryIter()) {
      const values = [...qc].copied();
      queryMap.set(qc.queryId, values);
    }
    expect(queryMap[q1]).toEqual([20, 40]);
    expect(queryMap[q2]).toEqual([10]);
  });

  test('test_candidate_changes_entity_level', () => {
    const changes = Arc.new([10, 20, 30]);
    let candidates = CandidateChanges.new(changes);
    candidates.addEntity(0);
    candidates.addEntity(2);
    const entities = candidates.entityIter().copied();
    expect(entities).toEqual([10, 30]);
  });

});
