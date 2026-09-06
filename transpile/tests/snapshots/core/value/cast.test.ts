// MIRRORS: ankurah/core/src/value/cast.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { Value_castTo } from './cast';
import { Json } from '../property/value/json';
import { Value, ValueType } from './index';
import { EntityId } from '@ankurah/proto';

describe('cast unit tests', () => {
  test('test_string_to_entity_id', () => {
    const entityId = EntityId.new();
    const base64Str = entityId.toBase64();
    const value = new Value('String', { _0: base64Str });
    try {
      const result = Value_castTo(value, new ValueType('EntityId', {})).unwrap();
      try {
        return result.match({
          EntityId: (v) => {
            const parsedId = v._0;
            expect(parsedId).toEqual(entityId);
          },
          I16: () => {
            throw new Error('Expected EntityId variant');
          },
          I32: () => {
            throw new Error('Expected EntityId variant');
          },
          I64: () => {
            throw new Error('Expected EntityId variant');
          },
          F64: () => {
            throw new Error('Expected EntityId variant');
          },
          Bool: () => {
            throw new Error('Expected EntityId variant');
          },
          String: () => {
            throw new Error('Expected EntityId variant');
          },
          Object: () => {
            throw new Error('Expected EntityId variant');
          },
          Binary: () => {
            throw new Error('Expected EntityId variant');
          },
          Json: () => {
            throw new Error('Expected EntityId variant');
          },
        });
      } finally {
        result.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_entity_id_to_string', () => {
    const entityId = EntityId.new();
    const value = new Value('EntityId', { _0: entityId.clone() });
    try {
      const result = Value_castTo(value, new ValueType('String', {})).unwrap();
      try {
        return result.match({
          String: (v) => {
            const s = v._0;
            expect(s).toEqual(entityId.toBase64());
          },
          I16: () => {
            throw new Error('Expected String variant');
          },
          I32: () => {
            throw new Error('Expected String variant');
          },
          I64: () => {
            throw new Error('Expected String variant');
          },
          F64: () => {
            throw new Error('Expected String variant');
          },
          Bool: () => {
            throw new Error('Expected String variant');
          },
          EntityId: () => {
            throw new Error('Expected String variant');
          },
          Object: () => {
            throw new Error('Expected String variant');
          },
          Binary: () => {
            throw new Error('Expected String variant');
          },
          Json: () => {
            throw new Error('Expected String variant');
          },
        });
      } finally {
        result.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_invalid_entity_id_string', () => {
    const value = new Value('String', { _0: 'invalid-entity-id' });
    try {
      const result = Value_castTo(value, new ValueType('EntityId', {}));
      try {
        if (!(((result) => {
          if (!(result.isErr())) return false;
          const _v = result.unwrapErr();
          return true;
        })(result))) throw new Error('assertion failed');
      } finally {
        result.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_numeric_upcasting', () => {
    const value = new Value('I16', { _0: 42 });
    try {
      const _t0 = Value_castTo(value, new ValueType('I32', {})).unwrap();
      try {
        const _t1 = new Value('I32', { _0: 42 });
        try {
          expect(_t0).toEqual(_t1);
        } finally {
          _t1.drop();
        }
      } finally {
        _t0.drop();
      }
      const _t2 = Value_castTo(value, new ValueType('I64', {})).unwrap();
      try {
        const _t3 = new Value('I64', { _0: 42n });
        try {
          expect(_t2).toEqual(_t3);
        } finally {
          _t3.drop();
        }
      } finally {
        _t2.drop();
      }
      const _t4 = Value_castTo(value, new ValueType('F64', {})).unwrap();
      try {
        const _t5 = new Value('F64', { _0: 42.0 });
        try {
          expect(_t4).toEqual(_t5);
        } finally {
          _t5.drop();
        }
      } finally {
        _t4.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_numeric_downcasting', () => {
    const value = new Value('I32', { _0: 42 });
    try {
      const _t0 = Value_castTo(value, new ValueType('I16', {})).unwrap();
      try {
        const _t1 = new Value('I16', { _0: 42 });
        try {
          expect(_t0).toEqual(_t1);
        } finally {
          _t1.drop();
        }
      } finally {
        _t0.drop();
      }
      const largeValue = new Value('I32', { _0: 100000 });
      try {
        if (!(((_v) => {
          if (!(_v.isErr())) return false;
          const _v1 = _v.unwrapErr();
          return true;
        })(Value_castTo(largeValue, new ValueType('I16', {}))))) throw new Error('assertion failed');
      } finally {
        largeValue.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_string_to_numeric', () => {
    const value = new Value('String', { _0: '42' });
    try {
      const _t0 = Value_castTo(value, new ValueType('I16', {})).unwrap();
      try {
        const _t1 = new Value('I16', { _0: 42 });
        try {
          expect(_t0).toEqual(_t1);
        } finally {
          _t1.drop();
        }
      } finally {
        _t0.drop();
      }
      const _t2 = Value_castTo(value, new ValueType('I32', {})).unwrap();
      try {
        const _t3 = new Value('I32', { _0: 42 });
        try {
          expect(_t2).toEqual(_t3);
        } finally {
          _t3.drop();
        }
      } finally {
        _t2.drop();
      }
      const _t4 = Value_castTo(value, new ValueType('I64', {})).unwrap();
      try {
        const _t5 = new Value('I64', { _0: 42n });
        try {
          expect(_t4).toEqual(_t5);
        } finally {
          _t5.drop();
        }
      } finally {
        _t4.drop();
      }
      const _t6 = Value_castTo(value, new ValueType('F64', {})).unwrap();
      try {
        const _t7 = new Value('F64', { _0: 42.0 });
        try {
          expect(_t6).toEqual(_t7);
        } finally {
          _t7.drop();
        }
      } finally {
        _t6.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_string_to_bool', () => {
    const _t0 = new Value('String', { _0: 'true' });
    try {
      const _t1 = Value_castTo(_t0, new ValueType('Bool', {})).unwrap();
      try {
        const _t2 = new Value('Bool', { _0: true });
        try {
          expect(_t1).toEqual(_t2);
        } finally {
          _t2.drop();
        }
      } finally {
        _t1.drop();
      }
    } finally {
      _t0.drop();
    }
    const _t3 = new Value('String', { _0: 'false' });
    try {
      const _t4 = Value_castTo(_t3, new ValueType('Bool', {})).unwrap();
      try {
        const _t5 = new Value('Bool', { _0: false });
        try {
          expect(_t4).toEqual(_t5);
        } finally {
          _t5.drop();
        }
      } finally {
        _t4.drop();
      }
    } finally {
      _t3.drop();
    }
    const _t6 = new Value('String', { _0: '1' });
    try {
      const _t7 = Value_castTo(_t6, new ValueType('Bool', {})).unwrap();
      try {
        const _t8 = new Value('Bool', { _0: true });
        try {
          expect(_t7).toEqual(_t8);
        } finally {
          _t8.drop();
        }
      } finally {
        _t7.drop();
      }
    } finally {
      _t6.drop();
    }
    const _t9 = new Value('String', { _0: '0' });
    try {
      const _t10 = Value_castTo(_t9, new ValueType('Bool', {})).unwrap();
      try {
        const _t11 = new Value('Bool', { _0: false });
        try {
          expect(_t10).toEqual(_t11);
        } finally {
          _t11.drop();
        }
      } finally {
        _t10.drop();
      }
    } finally {
      _t9.drop();
    }
    const _t12 = new Value('String', { _0: 'maybe' });
    try {
      if (!(((_v) => {
        if (!(_v.isErr())) return false;
        const _v1 = _v.unwrapErr();
        return true;
      })(Value_castTo(_t12, new ValueType('Bool', {}))))) throw new Error('assertion failed');
    } finally {
      _t12.drop();
    }
  });

  test('test_incompatible_types', () => {
    const value = new Value('Binary', { _0: new Uint8Array([1, 2, 3]) });
    try {
      const result = Value_castTo(value, new ValueType('I32', {}));
      try {
        if (!(((result) => {
          if (!(result.isErr())) return false;
          const _v = result.unwrapErr();
          return true;
        })(result))) throw new Error('assertion failed');
      } finally {
        result.drop();
      }
    } finally {
      value.drop();
    }
  });

  test('test_same_type_cast', () => {
    const value = new Value('I32', { _0: 42 });
    try {
      const result = Value_castTo(value, new ValueType('I32', {})).unwrap();
      try {
        expect(result).toEqual(value);
      } finally {
        result.drop();
      }
    } finally {
      value.drop();
    }
  });

});
