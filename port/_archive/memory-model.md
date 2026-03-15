# Memory Model

This spec has been split into multiple files for digestibility:

- **[memory-model/overview.md](memory-model/overview.md)** — Quick-reference rulebook: mapping table, classification rules, checklists, inherent limitations
- **[memory-model/decisions.md](memory-model/decisions.md)** — Overarching architectural decisions about how major subsystems map to the rules. Narrow type-specific adjudications belong as annotations in the source code.
- **[memory-model/provided-types.md](memory-model/provided-types.md)** — API docs for the std-equivalent utility types: Disposable, DisposeGuard, PromiseMutex, Symbol.dispose polyfill
- **[memory-model/lint-rules.md](memory-model/lint-rules.md)** — Custom lint rules that enforce Rust-like ownership semantics at dev time
