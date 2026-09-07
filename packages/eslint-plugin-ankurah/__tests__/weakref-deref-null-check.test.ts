// TS-ONLY: ESLint plugin enforcing Rust ownership semantics
import { RuleTester } from '@typescript-eslint/rule-tester';
import { rule } from '../src/rules/weakref-deref-null-check';

const ruleTester = new RuleTester({
  languageOptions: {
    parserOptions: {
      projectService: false,
    },
  },
});

ruleTester.run('weakref-deref-null-check', rule, {
  valid: [
    // Optional chaining on deref result
    {
      code: `const value = weakRef.deref()?.doSomething();`,
    },
    // Deref with null check before use
    {
      code: `
        const obj = weakRef.deref();
        if (obj !== undefined) {
          obj.doSomething();
        }
      `,
    },
    // Deref result not used (just checking existence)
    {
      code: `
        const obj = weakRef.deref();
        if (obj) {
          console.log('alive');
        }
      `,
    },
    // Not a deref call
    {
      code: `const result = someObj.getValue().name;`,
    },
    // Q2: a `deref` the file has not shown to be a `WeakRef`'s is left alone.
    // The port writes Rust's `Deref` impl under that name — `impl Deref for
    // AttestationSet` is `.deref()` here — and reading through one has no
    // `None` to handle. Matched by the method's name alone this was one error
    // on every clean run of `npx eslint packages/proto/src`.
    {
      code: `const count = this.attestations.deref().length;`,
    },
    {
      code: `
        const set = holder.deref();
        return set.length;
      `,
    },
    // And the same four spellings for something that is NOT a WeakRef: a field,
    // a parameter, a value alias and a type alias each leave the port's own
    // `Deref` accessor alone.
    {
      code: `
        class Holder {
          private set: AttestationSet;
          read() { return this.set.deref().length; }
        }
      `,
    },
    {
      code: `function read(s: AttestationSet) { return s.deref().length; }`,
    },
    {
      code: `
        const LocalSet = AttestationSet;
        const held = new LocalSet(items);
        const n = held.deref().length;
      `,
    },
    {
      code: `
        type Held = AttestationSet;
        const held: Held = made;
        const n = held.deref().length;
      `,
    },
    // Z9: the rule has no types to ask, so a class FIELD is evidence only where
    // the receiver is the instance the class body is about. Asked of the
    // property NAME alone it walked up to the enclosing class and reported
    // `other.ref.deref()` for a different object entirely.
    `
      class Holder {
        private ref: WeakRef<Target>;
        read(other: Other) {
          return other.ref.deref().name;
        }
      }
    `,
    // And a chain through another object of this instance's stays unrecognised,
    // which is the safe direction: a missed WeakRef is a warning nobody gets,
    // and a wrong one is a warning about code that is right.
    `
      class Holder {
        private ref: WeakRef<Target>;
        read() {
          return this.inner.ref.deref().name;
        }
      }
    `,
  ],
  invalid: [
    // Direct property access on deref without null check
    {
      code: `
        const weakRef = new WeakRef(target);
        const value = weakRef.deref().name;
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // Direct method call on deref without null check
    {
      code: `
        const weakRef: WeakRef<Target> = held;
        const value = weakRef.deref().doSomething();
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // And on the construction itself
    {
      code: `const value = new WeakRef(target).deref().name;`,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // T8/U6: a class FIELD annotated as one. The receiver is `this.ref`, a
    // member expression, and the evidence is where the class declares it.
    {
      code: `
        class Holder {
          private ref: WeakRef<Target>;
          read() { return this.ref.deref().name; }
        }
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // A PARAMETER annotated as one. The annotation is on the definition's NAME
    // and the rule read only its node, so this never fired.
    {
      code: `
        function read(r: WeakRef<Target>) { return r.deref().name; }
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // A VALUE alias of the constructor.
    {
      code: `
        const LocalWeakRef = WeakRef;
        const held = new LocalWeakRef(target);
        const value = held.deref().name;
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // A TYPE alias of the annotation.
    {
      code: `
        type Held = WeakRef<Target>;
        const held: Held = made;
        const value = held.deref().name;
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // The field the class really does declare, read through `this` — and
    // through a local the body assigned `this` to, which is how a closure keeps
    // hold of the receiver.
    {
      code: `
        class Holder {
          private ref: WeakRef<Target>;
          read() {
            return this.ref.deref().name;
          }
        }
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    {
      code: `
        class Holder {
          private ref: WeakRef<Target>;
          read() {
            const self = this;
            return self.ref.deref().name;
          }
        }
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
    // An IMPORTED alias of the constructor, which is how a shim would arrive.
    {
      code: `
        import { WeakRef as Ref } from './shim';
        const held = new Ref(target);
        const value = held.deref().name;
      `,
      errors: [{ messageId: 'directPropertyAccess' }],
    },
  ],
});
