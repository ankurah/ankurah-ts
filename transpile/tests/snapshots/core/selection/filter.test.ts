// MIRRORS: ankurah/core/src/selection/filter.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { FilterIterator, FilterResult } from './filter';
import { Struct, debugString, dropOwned } from '@ankurah/base';
import { parseSelection } from '@ankurah/ankql';

class TestItem extends Struct implements Filterable {
  name: string;
  age: string;

  constructor(name: string, age: string) {
    super();
    this.name = name;
    this.age = age;
  }

  static new(name: string, age: string): TestItem {
    return new TestItem(name, age);
  }

  collection(): string {
    return 'users';
  }

  value(name: string): Value | null {
    if (name === 'name') {
      return new Value('String', { _0: this.name });
    } else if (name === 'age') {
      return new Value('String', { _0: this.age });
    } else {
      return null;
    }
  }

  equals(other: TestItem): boolean {
    if (this.name !== other.name) return false;
    if (this.age !== other.age) return false;
    return true;
  }

  clone(): TestItem {
    return new TestItem(this.name, this.age);
  }

  debug(): string {
    return `TestItem { name: ${debugString(this.name)}, age: ${debugString(this.age)} }`;
  }
}

describe('filter unit tests', () => {
  test('test_simple_equality', () => {
    const items = [TestItem.new('Alice', '30'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '35')];
    const selection = parseSelection('name = \'Alice\'').unwrap();
    try {
      let _moved1 = false;
      const _b0 = [...items];
      try {
        const _b2 = selection.takeField('predicate');
        _moved1 = true;
        const results = FilterIterator.new(_b0, _b2);
        const _t3 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '30') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Skip', { _0: TestItem.new('Charlie', '35') })];
        try {
          expect(results).toEqual(_t3);
        } finally {
          dropOwned(_t3);
        }
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_and_condition', () => {
    const items = [TestItem.new('Alice', '30'), TestItem.new('Bob', '30'), TestItem.new('Charlie', '35')];
    const selection = parseSelection('name = \'Alice\' AND age = \'30\'').unwrap();
    try {
      let _moved1 = false;
      const _b0 = [...items];
      try {
        const _b2 = selection.takeField('predicate');
        _moved1 = true;
        const results = FilterIterator.new(_b0, _b2);
        const _t3 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '30') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '30') }), new FilterResult('Skip', { _0: TestItem.new('Charlie', '35') })];
        try {
          expect(results).toEqual(_t3);
        } finally {
          dropOwned(_t3);
        }
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_complex_condition', () => {
    const items = [TestItem.new('Alice', '20'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '30'), TestItem.new('David', '35'), TestItem.new('Eve', '40')];
    const selection = parseSelection('(name = \'Alice\' OR name = \'Charlie\') AND age >= \'30\' AND age <= \'40\'').unwrap();
    try {
      let _moved1 = false;
      const _b0 = [...items];
      try {
        const _b2 = selection.takeField('predicate');
        _moved1 = true;
        const results = FilterIterator.new(_b0, _b2);
        const _t3 = [new FilterResult('Skip', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Skip', { _0: TestItem.new('Eve', '40') })];
        try {
          expect(results).toEqual(_t3);
        } finally {
          dropOwned(_t3);
        }
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_in_operator', () => {
    const items = [TestItem.new('Alice', '20'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '30'), TestItem.new('David', '35'), TestItem.new('Eve', '40')];
    const selection = parseSelection('name IN (\'Alice\', \'Charlie\', \'Eve\')').unwrap();
    try {
      let _moved1 = false;
      const _b0 = [...items.map((e) => e.clone())];
      try {
        const _b2 = selection.takeField('predicate');
        _moved1 = true;
        const results = FilterIterator.new(_b0, _b2);
        const _t3 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Pass', { _0: TestItem.new('Eve', '40') })];
        try {
          expect(results).toEqual(_t3);
        } finally {
          dropOwned(_t3);
        }
        const selection_1 = parseSelection('age IN (\'20\', \'30\', \'40\')').unwrap();
        try {
          let _moved5 = false;
          const _b4 = [...items];
          try {
            const _b6 = selection_1.takeField('predicate');
            _moved5 = true;
            const results_1 = FilterIterator.new(_b4, _b6);
            const _t7 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Pass', { _0: TestItem.new('Eve', '40') })];
            try {
              expect(results_1).toEqual(_t7);
            } finally {
              dropOwned(_t7);
            }
          } finally {
            if (!_moved5) dropOwned(_b4);
          }
        } finally {
          selection_1.drop();
        }
      } finally {
        if (!_moved1) dropOwned(_b0);
      }
    } finally {
      selection.drop();
    }
  });

});
