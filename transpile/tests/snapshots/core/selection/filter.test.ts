// MIRRORS: ankurah/core/src/selection/filter.rs (tests module)

import { describe, test, expect } from 'bun:test';
import { FilterIterator, FilterResult } from './filter';
import { Struct, dropOwned } from '@ankurah/base';
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
      return new Value('String', { _0: this.name.clone() });
    } else if (name === 'age') {
      return new Value('String', { _0: this.age.clone() });
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
    return `TestItem { name: ${JSON.stringify(this.name)}, age: ${JSON.stringify(this.age)} }`;
  }
}

describe('filter unit tests', () => {
  test('test_simple_equality', () => {
    const items = [TestItem.new('Alice', '30'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '35')];
    const selection = parseSelection('name = \'Alice\'').unwrap();
    try {
      const results = FilterIterator.new([...items], selection.takeField('predicate'));
      const _t0 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '30') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Skip', { _0: TestItem.new('Charlie', '35') })];
      try {
        expect(results).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_and_condition', () => {
    const items = [TestItem.new('Alice', '30'), TestItem.new('Bob', '30'), TestItem.new('Charlie', '35')];
    const selection = parseSelection('name = \'Alice\' AND age = \'30\'').unwrap();
    try {
      const results = FilterIterator.new([...items], selection.takeField('predicate'));
      const _t0 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '30') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '30') }), new FilterResult('Skip', { _0: TestItem.new('Charlie', '35') })];
      try {
        expect(results).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_complex_condition', () => {
    const items = [TestItem.new('Alice', '20'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '30'), TestItem.new('David', '35'), TestItem.new('Eve', '40')];
    const selection = parseSelection('(name = \'Alice\' OR name = \'Charlie\') AND age >= \'30\' AND age <= \'40\'').unwrap();
    try {
      const results = FilterIterator.new([...items], selection.takeField('predicate'));
      const _t0 = [new FilterResult('Skip', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Skip', { _0: TestItem.new('Eve', '40') })];
      try {
        expect(results).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
    } finally {
      selection.drop();
    }
  });

  test('test_in_operator', () => {
    const items = [TestItem.new('Alice', '20'), TestItem.new('Bob', '25'), TestItem.new('Charlie', '30'), TestItem.new('David', '35'), TestItem.new('Eve', '40')];
    const selection = parseSelection('name IN (\'Alice\', \'Charlie\', \'Eve\')').unwrap();
    try {
      const results = FilterIterator.new([...items.clone()], selection.takeField('predicate'));
      const _t0 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Pass', { _0: TestItem.new('Eve', '40') })];
      try {
        expect(results).toEqual(_t0);
      } finally {
        dropOwned(_t0);
      }
      const selection_1 = parseSelection('age IN (\'20\', \'30\', \'40\')').unwrap();
      try {
        const results_1 = FilterIterator.new([...items], selection_1.takeField('predicate'));
        const _t1 = [new FilterResult('Pass', { _0: TestItem.new('Alice', '20') }), new FilterResult('Skip', { _0: TestItem.new('Bob', '25') }), new FilterResult('Pass', { _0: TestItem.new('Charlie', '30') }), new FilterResult('Skip', { _0: TestItem.new('David', '35') }), new FilterResult('Pass', { _0: TestItem.new('Eve', '40') })];
        try {
          expect(results_1).toEqual(_t1);
        } finally {
          dropOwned(_t1);
        }
      } finally {
        selection_1.drop();
      }
    } finally {
      selection.drop();
    }
  });

});
