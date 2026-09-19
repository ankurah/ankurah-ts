// MIRRORS: ankurah/a_capture_named_only_inside_a_macro/src/input.rs
import { Struct, Arc, OwnedClosure, invokeRef, Invocable } from '@ankurah/base';

export class Name extends Struct {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }
}

export class Greeter extends Struct {
  readonly f: Invocable<[], string>;

  constructor(f: Invocable<[], string>) {
    super();
    this.f = f;
  }

  static new(f: Invocable<[], string>): Greeter {
    return new Greeter(f);
  }

  greet(): string {
    return invokeRef(this.f);
  }
}

export function greeter(firstText: string, lastText: string): Greeter {
  const held = (() => {
    const first = Arc.new(new Name(firstText));
    const last = Arc.new(new Name(lastText));
    return Greeter.new(new OwnedClosure([first, last], () => `${first.value.text} ${last.value.text}`));
  })();
  return held;
}

