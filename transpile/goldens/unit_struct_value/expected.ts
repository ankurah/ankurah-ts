// MIRRORS: ankurah/unit_struct_value/src/input.rs
import { Struct } from '@ankurah/base';

export class Mock extends Struct implements Greets {

  greeting(): string {
    return 'mock';
  }
}

export class Loud extends Struct implements Greets {

  greeting(): string {
    return 'LOUD';
  }
}

export interface Greets {
  greeting(): string;
}

export function aMock(): Mock {
  return new Mock();
}

export function greetWith<G extends Greets>(g: G): string {
  return g.greeting();
}

