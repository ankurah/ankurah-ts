// MIRRORS: ankurah/core/src/indexing/key_spec.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { IndexDirection, IndexKeyPart, IndexSpecMatch, KeySpec } from './key_spec';
import { dropOwned } from '@ankurah/base';
import { ValueType } from '../value/index';

describe('key_spec unit tests', () => {
  test('test_exact_match', () => {
    const spec1 = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const spec2 = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
      try {
        const _t0 = spec1.matches(spec2);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
      } finally {
        spec2.drop();
      }
    } finally {
      spec1.drop();
    }
  });

  test('test_prefix_match', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_inverse_exact_match', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.desc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Inverse', {}));
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_inverse_prefix_match', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.desc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Inverse', {}));
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_user_example', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec1 = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec1);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
        const indexSpec2 = new KeySpec([IndexKeyPart.desc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {})), IndexKeyPart.desc('c', new ValueType('String', {}))]);
        try {
          const _t1 = querySpec.matches(indexSpec2);
          try {
            expect(_t1).toEqual(new IndexSpecMatch('Inverse', {}));
          } finally {
            dropOwned(_t1);
          }
        } finally {
          indexSpec2.drop();
        }
      } finally {
        indexSpec1.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_no_match_different_fields', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.asc('x', new ValueType('String', {})), IndexKeyPart.desc('y', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(null);
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_no_match_partial_field_overlap', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(null);
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_no_match_query_longer_than_index', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
    try {
      const indexSpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec);
        try {
          expect(_t0).toEqual(null);
        } finally {
          dropOwned(_t0);
        }
      } finally {
        indexSpec.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_empty_specs', () => {
    const emptySpec = new KeySpec([]);
    try {
      const nonEmptySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {}))]);
      try {
        const _t0 = emptySpec.matches(nonEmptySpec);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
        const _t1 = emptySpec.matches(emptySpec);
        try {
          expect(_t1).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t1);
        }
        const _t2 = nonEmptySpec.matches(emptySpec);
        try {
          expect(_t2).toEqual(null);
        } finally {
          dropOwned(_t2);
        }
      } finally {
        nonEmptySpec.drop();
      }
    } finally {
      emptySpec.drop();
    }
  });

  test('test_single_field_cases', () => {
    const ascSpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {}))]);
    try {
      const descSpec = new KeySpec([IndexKeyPart.desc('a', new ValueType('String', {}))]);
      try {
        const _t0 = ascSpec.matches(ascSpec);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
        const _t1 = ascSpec.matches(descSpec);
        try {
          expect(_t1).toEqual(new IndexSpecMatch('Inverse', {}));
        } finally {
          dropOwned(_t1);
        }
        const _t2 = descSpec.matches(ascSpec);
        try {
          expect(_t2).toEqual(new IndexSpecMatch('Inverse', {}));
        } finally {
          dropOwned(_t2);
        }
      } finally {
        descSpec.drop();
      }
    } finally {
      ascSpec.drop();
    }
  });

  test('test_complex_multi_field_scenarios', () => {
    const querySpec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
    try {
      const indexSpec1 = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {})), IndexKeyPart.desc('d', new ValueType('String', {}))]);
      try {
        const _t0 = querySpec.matches(indexSpec1);
        try {
          expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t0);
        }
        const indexSpec2 = new KeySpec([IndexKeyPart.desc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {})), IndexKeyPart.desc('c', new ValueType('String', {})), IndexKeyPart.asc('d', new ValueType('String', {}))]);
        try {
          const _t1 = querySpec.matches(indexSpec2);
          try {
            expect(_t1).toEqual(new IndexSpecMatch('Inverse', {}));
          } finally {
            dropOwned(_t1);
          }
          const indexSpec3 = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.asc('b', new ValueType('String', {})), IndexKeyPart.desc('c', new ValueType('String', {}))]);
          try {
            const _t2 = querySpec.matches(indexSpec3);
            try {
              expect(_t2).toEqual(null);
            } finally {
              dropOwned(_t2);
            }
          } finally {
            indexSpec3.drop();
          }
        } finally {
          indexSpec2.drop();
        }
      } finally {
        indexSpec1.drop();
      }
    } finally {
      querySpec.drop();
    }
  });

  test('test_helper_methods', () => {
    const ascKeypart = IndexKeyPart.asc('test', new ValueType('String', {}));
    try {
      expect(ascKeypart.column).toEqual('test');
      expect(ascKeypart.direction).toEqual(new IndexDirection('Asc', {}));
      expect(ascKeypart.nulls).toEqual(null);
      expect(ascKeypart.collation).toEqual(null);
      const descKeypart = IndexKeyPart.desc('test', new ValueType('String', {}));
      try {
        expect(descKeypart.column).toEqual('test');
        expect(descKeypart.direction).toEqual(new IndexDirection('Desc', {}));
        expect(descKeypart.nulls).toEqual(null);
        expect(descKeypart.collation).toEqual(null);
      } finally {
        descKeypart.drop();
      }
    } finally {
      ascKeypart.drop();
    }
  });

  test('test_edge_case_behaviors', () => {
    const spec = new KeySpec([IndexKeyPart.asc('a', new ValueType('String', {})), IndexKeyPart.desc('b', new ValueType('String', {})), IndexKeyPart.asc('c', new ValueType('String', {}))]);
    try {
      const _t0 = spec.matches(spec);
      try {
        expect(_t0).toEqual(new IndexSpecMatch('Match', {}));
      } finally {
        dropOwned(_t0);
      }
      const empty = new KeySpec([]);
      try {
        const _t1 = empty.matches(spec);
        try {
          expect(_t1).toEqual(new IndexSpecMatch('Match', {}));
        } finally {
          dropOwned(_t1);
        }
      } finally {
        empty.drop();
      }
    } finally {
      spec.drop();
    }
  });

});
