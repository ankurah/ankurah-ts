# Bun Workspaces as Monorepo Tool for ankurah-ts

Research date: 2026-02-10
Bun latest stable: v1.3.9 (as of February 2026)
Bun acquired by Anthropic: December 2025 (remains MIT-licensed, open source)

## 1. Bun Workspaces + Expo / React Native

### Official Support

Expo officially supports bun workspaces. Since **SDK 52**, Expo's Metro config auto-detects monorepos for bun, npm, pnpm, and Yarn. No manual Metro configuration is needed when using `expo/metro-config`.

Expo's ["Using Bun" guide](https://docs.expo.dev/guides/using-bun/) and [monorepo guide](https://docs.expo.dev/guides/monorepos/) both document bun as a first-class package manager.

### The Metro Transitive Dependency Problem (CRITICAL)

**This is the single biggest issue with bun + React Native.**

Bun 1.3 defaults to **isolated installs** for workspaces. Packages are stored in `node_modules/.bun/` with symlinks at the top level. Metro bundler cannot resolve transitive dependencies in this layout because it expects npm/yarn-style hoisted `node_modules/`.

- [oven-sh/bun#25870](https://github.com/oven-sh/bun/issues/25870) - Metro bundler can't resolve transitive dependencies
- [facebook/metro#1636](https://github.com/facebook/metro/issues/1636) - Same issue reported on Metro side

**Workarounds:**

1. **Use `--linker hoisted`**: Forces traditional flat `node_modules/` layout.
   ```bash
   bun install --linker hoisted
   ```
   Or in `bunfig.toml`:
   ```toml
   [install]
   linker = "hoisted"
   ```

2. **Selective hoisting** (bun >= 1.3.1): Keep isolated installs but hoist packages Metro needs.
   In `bunfig.toml`:
   ```toml
   [install]
   publicHoistPattern = ["*"]
   ```
   Or in `.npmrc`:
   ```
   public-hoist-pattern[]=*
   ```
   Note: `publicHoistPattern = ["*"]` effectively makes it behave like hoisted anyway.

3. **Add transitive deps as direct deps**: Manually list the packages Metro fails to resolve in your app's `package.json`. Fragile and not recommended.

### EAS Build Issues

Reported issues with EAS Cloud builds not detecting bun in monorepos:
- [expo/eas-cli#2658](https://github.com/expo/eas-cli/issues/2658) - EAS Build not detecting bun in monorepo
- [expo/eas-cli#3238](https://github.com/expo/eas-cli/issues/3238) - Local builds always use bun 1.3 even when a different version is specified
- [expo/eas-cli#3118](https://github.com/expo/eas-cli/issues/3118) - Cannot use bun for EAS local builds

These are primarily tooling detection issues, not fundamental incompatibilities.

### Verdict: Usable with Caveats

Bun workspaces work with Expo/Metro **if you use hoisted installs**. The default isolated install mode is incompatible with Metro. This eliminates one of bun's differentiating features (strict dependency isolation), making it behave more like npm/yarn in practice.

---

## 2. Bun Workspaces vs pnpm Workspaces

| Feature | bun workspaces | pnpm workspaces |
|---------|---------------|-----------------|
| **Install speed** | ~4x faster than pnpm (clean install) | Fast with global content-addressable store |
| **Disk usage** | ~234 MB (test project) | ~205 MB (same project), up to 70% less than npm |
| **Hoisting** | Isolated by default (1.3+), `--linker hoisted` available | Strict by default, configurable `node-linker` |
| **Workspace filtering** | `--filter` flag (improved in 1.3+) | `--filter` flag (mature, advanced patterns) |
| **Dependency catalogs** | Supported (1.3+, inspired by pnpm) | Mature catalog protocol |
| **Lockfile** | `bun.lock` (binary, switchable to text) | `pnpm-lock.yaml` (text, git-friendly) |
| **Metro compatibility** | Requires hoisted linker or publicHoistPattern | Requires `node-linker=hoisted` in `.npmrc` |
| **Expo monorepo reference** | Expo docs mention bun | [byCedric/expo-monorepo-example](https://github.com/byCedric/expo-monorepo-example) uses pnpm |
| **Maturity for monorepos** | Rapidly improving, some rough edges | Industry standard for JS monorepos |
| **Overrides/patches** | Supported | Supported with `pnpm patch` |

### Key Difference: Both Need Hoisting for Metro

Both bun and pnpm default to non-hoisted (strict/isolated) installs. Both require switching to hoisted mode for Metro/React Native compatibility. This means neither gives you strict dependency enforcement in a React Native monorepo -- Metro's resolution model requires the flat `node_modules/` layout.

### pnpm Advantage: Ecosystem Maturity

pnpm has the larger ecosystem of monorepo tooling, better-documented patterns for Expo (byCedric's example repo), and more predictable behavior. Its text-based lockfile is easier to review in PRs.

### bun Advantage: Speed and All-in-One

bun is significantly faster for installs and offers a unified runtime (package manager + runtime + test runner + bundler). For a developer already using bun (like the domcorder project), staying in one ecosystem reduces friction.

---

## 3. Bun Test Runner

### Feature Summary

The `bun:test` module provides a built-in, Jest-compatible test runner:

- **TypeScript/JSX**: Native support, no configuration needed
- **API**: `describe`, `it`/`test`, `expect`, `beforeAll`, `beforeEach`, `afterEach`, `afterAll`
- **Assertions**: Full `expect()` matcher API (`.toBe`, `.toEqual`, `.toMatchSnapshot`, `.toThrow`, etc.)
- **Mocking**: `mock()`, `spyOn()`, `mock.module()` for module mocking
- **Snapshots**: Supported
- **Watch mode**: Supported with HMR-style re-execution
- **Coverage**: Built-in (`--coverage`)
- **Performance**: 10-50x faster than Jest

### What Works for ankurah-ts

- **Fixture loading**: `Bun.file(path).text()` / `.json()` / `.arrayBuffer()` for reading test fixtures
- **TypeScript**: Runs `.ts` files directly, no compilation step
- **Lifecycle hooks**: Full support for setup/teardown at describe and file scope
- **Module mocking**: `mock.module()` works but without hoisting (differs from Jest)

### Limitations / Missing Features

- **Fake timers**: `jest.useFakeTimers()` now implemented (as of 1.3.x), but `setSystemTime` does not impact timer scheduling yet -- only `Date.now()` is affected. This is a partial implementation.
- **Module mock hoisting**: Unlike Jest's `jest.mock()` which is hoisted to the top of the file, bun's `mock.module()` patches the module cache at runtime. Side effects from the original module still execute.
- **Some Jest matchers**: Not 100% complete coverage of all Jest matchers.
- **`--pathIgnorePatterns`**: Not yet supported ([oven-sh/bun#21395](https://github.com/oven-sh/bun/issues/21395)).

### better-sqlite3 Compatibility (CRITICAL for ankurah-ts)

**better-sqlite3 has known crashes with bun.** Multiple issues reported:

- [oven-sh/bun#23757](https://github.com/oven-sh/bun/issues/23757) - Segfault loading better_sqlite3.node on macOS
- [oven-sh/bun#24956](https://github.com/oven-sh/bun/issues/24956) - Crashes with better-sqlite3
- [oven-sh/bun#16050](https://github.com/oven-sh/bun/issues/16050) - ABI version mismatch (NODE_MODULE_VERSION 131 vs 127)

**Alternative**: Bun has a built-in `bun:sqlite` module that is 3-6x faster than better-sqlite3, with an API inspired by (but not identical to) better-sqlite3. A ~100-line compatibility shim exists.

**Impact on ankurah-ts**: The `@ankurah/storage-better-sqlite3` package is designed for Node.js testing. If using bun as the test runner, you would need to either:
1. Use `bun:sqlite` with a thin adapter (new package or adapter layer)
2. Accept that tests run under Node.js (via `node --test` or Vitest/Jest) while bun handles package management only
3. Abstract the sqlite interface so `bun:sqlite` and `better-sqlite3` are interchangeable behind a common interface (this aligns with ankurah's `StorageEngine` trait pattern)

### Verdict

Bun's test runner is viable for TypeScript unit tests. The better-sqlite3 incompatibility is a real concern but solvable via `bun:sqlite` or by keeping Node.js as the test runtime.

---

## 4. Bun Build

### Current Capabilities

`bun build` can:
- Bundle TypeScript/JavaScript to ESM or CJS
- Tree-shake, minify, target different environments (browser, bun, node)
- Handle CSS
- Zero-config frontend dev server (1.3+)

### Declaration File Generation: NOT SUPPORTED

**`bun build` cannot generate `.d.ts` declaration files.** This is a long-standing feature request:

- [oven-sh/bun#5141](https://github.com/oven-sh/bun/issues/5141) - "Generate type declarations during `bun build`" (open since 2023)

### Workarounds

1. **`tsc --emitDeclarationOnly`**: Run TypeScript compiler alongside bun build. This is the standard approach.
   ```json
   {
     "scripts": {
       "build": "bun build src/index.ts --outdir dist && tsc --emitDeclarationOnly --outDir dist"
     }
   }
   ```

2. **[bunup](https://bunup.dev/)**: A third-party build tool for TypeScript libraries powered by bun. It has its own high-performance `.d.ts` bundler and handles both JS output and declaration files. Designed for monorepo use.

3. **tsdown**: Another option with `.d.ts` generation, compatible with bun.

### For ankurah-ts

The ankurah-ts monorepo has ~12 packages that need to export types. Options:
- Use `tsc` for declarations + `bun build` for JS (two-step build per package)
- Use `bunup` which wraps both steps
- Skip bundling entirely: since packages are consumed within the monorepo and by the Expo app (which uses Metro for bundling), you may only need `tsc` for type checking and let Metro/the consuming bundler handle the rest. TypeScript project references with `"composite": true` can provide incremental type checking across packages.

### Verdict

bun build is not sufficient for library packages that need `.d.ts` output. You need `tsc` or a wrapper tool like bunup regardless.

---

## 5. Turborepo Compatibility

### Status: Works, With Known Issues

Turborepo officially supports bun workspaces (beta since late 2023, now more stable).

**What works:**
- Task orchestration (`turbo run build test lint`)
- Caching (local and remote)
- Dependency graph resolution across bun workspaces
- `--filter` for running tasks in specific packages

**Known issues:**
- [vercel/turborepo#11007](https://github.com/vercel/turborepo/issues/11007) - `turbo prune` generates different `bun.lock` files, breaking `bun i --frozen-lockfile`
- [vercel/turborepo#7456](https://github.com/vercel/turborepo/discussions/7456) - `prune` support for bun workspaces still maturing
- Workspace filtering requires `--cwd` flag workarounds in some cases

**Configuration:**
```json
{
  "packageManager": "bun@1.3.9"
}
```

### Does Bun Have Its Own Task Orchestration?

**No.** Bun has no built-in equivalent to Turborepo's task graph, caching, or parallel task execution. Bun's `--filter` flag is for dependency installation only, not task running. You need Turborepo, Nx, or moon for task orchestration in a bun monorepo.

However, bun's `bun run` is a faster script runner than `npm run` / `pnpm run`, so simple `bun run build` in each package is fast even without caching.

### Verdict

Turborepo + bun works for the common case. The `prune` issue matters for Docker/CI but not for local development. For ankurah-ts's initial development phase, Turborepo is optional -- `bun run` scripts in root `package.json` (as domcorder does) may be sufficient.

---

## 6. Current Maturity

### Version Timeline

| Version | Date | Notable |
|---------|------|---------|
| 1.0 | Sep 2023 | Initial stable release |
| 1.1 | Apr 2024 | Windows support, module mocking |
| 1.2 | Jan 2025 | Node.js compat improvements, text lockfile option |
| 1.3 | Oct 2025 | Isolated installs default, dependency catalogs, zero-config frontend, Bun.SQL, Redis client, fake timers |
| 1.3.9 | Feb 2026 | Latest patch (current) |

### Anthropic Acquisition (December 2025)

Anthropic acquired Oven (the company behind bun) in December 2025. Bun powers Claude Code CLI, which reached $1B run-rate revenue. Key implications:
- Bun has strong financial backing and a major production user (Anthropic)
- Remains MIT-licensed and open source
- Same core team continues development
- Strategic alignment with AI coding tools ecosystem

Sources:
- [Bun is joining Anthropic](https://bun.com/blog/bun-joins-anthropic)
- [Anthropic acquires Bun](https://www.anthropic.com/news/anthropic-acquires-bun-as-claude-code-reaches-usd1b-milestone)

### Production Readiness Assessment

**Strengths:**
- Package management: Mature, fast, compatible with npm registry
- Runtime: Stable for most Node.js patterns
- Test runner: Functional for typical test suites
- Anthropic backing ensures long-term viability

**Weaknesses:**
- Native module compatibility: ~34% of native dependencies work without issues
- Some Node.js API edge cases (buffer handling, crypto)
- Metro/React Native transitive dependency resolution needs hoisted linker
- `bun build` lacks declaration file generation
- EAS Build detection issues in monorepos
- Fewer production monorepo success stories than pnpm

---

## 7. Practical Recommendation for ankurah-ts

### ankurah-ts Requirements Recap

1. **Monorepo**: ~12 packages under `@ankurah/*` scope
2. **Expo Go target**: Metro bundler, Hermes engine, no native modules
3. **Node.js testing**: `better-sqlite3` for server-side / test storage engine
4. **TypeScript**: All packages are TS, need type checking and declaration files
5. **CRDT**: Yjs (pure JS, no bun-specific concerns)
6. **Cross-platform**: Must work in both Expo Go (production) and Node.js (testing/server)

### Option A: Bun Workspaces (All-In)

Use bun as package manager, runtime, test runner, and build tool.

```
Pros:
+ Fastest install times
+ Single tool for most operations
+ bun:sqlite replaces better-sqlite3 (faster, no native module issues)
+ Consistent with domcorder project
+ Anthropic backing

Cons:
- Must use --linker hoisted (loses strict dependency isolation)
- better-sqlite3 crashes under bun runtime
- Need tsc or bunup for .d.ts generation
- EAS Build detection issues in monorepos
- Less proven for Expo monorepos than pnpm
- bun:sqlite only works under bun runtime, not Node.js
```

### Option B: pnpm Workspaces (Conservative)

Use pnpm as package manager, Vitest or Jest for testing, tsc for builds.

```
Pros:
+ Most proven Expo monorepo tooling (byCedric example)
+ Strict dependency management (even if Metro needs hoisting)
+ better-sqlite3 works perfectly under Node.js
+ Text lockfile (git-friendly)
+ Largest monorepo ecosystem

Cons:
- Slower installs than bun
- More tools to configure (pnpm + Vitest + tsc)
- Different tool than domcorder
```

### Option C: Hybrid (RECOMMENDED)

Use **bun as package manager** (with hoisted linker) and **bun as test runner** for pure-TS packages. Use **Node.js** for packages that need better-sqlite3. Use **tsc** for type checking and declaration files.

```toml
# bunfig.toml
[install]
linker = "hoisted"
```

```
Package manager:     bun (--linker hoisted)
Test runner:         bun:test for most packages
                     Node.js + vitest for storage-better-sqlite3 tests
SQLite in tests:     bun:sqlite via StorageEngine adapter
Type checking:       tsc (with project references)
Declaration files:   tsc --emitDeclarationOnly
Task orchestration:  bun run scripts (Turborepo optional later)
Expo bundling:       Metro (standard Expo setup)
```

**Storage engine abstraction**: Define a `SqliteDatabase` interface that both `bun:sqlite` and `better-sqlite3` implement. The `StorageEngine` interface in ankurah's architecture already provides this separation. For testing, use `bun:sqlite` under bun, `better-sqlite3` under Node.js. The in-memory storage engine covers pure unit tests regardless of runtime.

### Why Hybrid

1. **Speed**: bun installs are 4x faster than pnpm, which matters during development
2. **Simplicity**: One package manager, consistent with domcorder
3. **Compatibility**: Hoisted linker avoids Metro issues
4. **Flexibility**: Can still run Node.js for specific packages that need it
5. **Future-proof**: Bun's Anthropic backing means rapid improvement on current gaps
6. **Pragmatic**: Doesn't force bun:sqlite as the only test SQLite -- keeps better-sqlite3 as an option under Node.js if needed

### Migration Path

If bun proves problematic during development, switching from bun workspaces to pnpm workspaces is straightforward:
1. Delete `bun.lock` and `node_modules/`
2. Add `pnpm-workspace.yaml`
3. Run `pnpm install`
4. Package.json workspace configs are identical between bun and pnpm

The `package.json` `"workspaces"` field is the same format for both, so the switchover cost is low.

---

## Sources

- [Expo: Work with monorepos](https://docs.expo.dev/guides/monorepos/)
- [Expo: Using Bun](https://docs.expo.dev/guides/using-bun/)
- [Bun: Workspaces docs](https://bun.com/docs/pm/workspaces)
- [Bun: Isolated installs](https://bun.com/docs/pm/isolated-installs)
- [Bun: Test runner](https://bun.com/docs/test)
- [Bun: SQLite](https://bun.com/docs/runtime/sqlite)
- [Metro + bun transitive deps issue](https://github.com/oven-sh/bun/issues/25870)
- [Metro issue #1636](https://github.com/facebook/metro/issues/1636)
- [bun build .d.ts request](https://github.com/oven-sh/bun/issues/5141)
- [better-sqlite3 crash on macOS](https://github.com/oven-sh/bun/issues/23757)
- [Turborepo + bun prune issue](https://github.com/vercel/turborepo/issues/11007)
- [byCedric expo-monorepo-example](https://github.com/byCedric/expo-monorepo-example)
- [bunup - TS library build tool](https://bunup.dev/)
- [Bun is joining Anthropic](https://bun.com/blog/bun-joins-anthropic)
- [JavaScript Package Managers in 2026](https://vibepanda.io/resources/guide/javascript-package-managers)
- [PNPM vs Bun vs Yarn Berry](https://betterstack.com/community/guides/scaling-nodejs/pnpm-vs-bun-install-vs-yarn/)
- [Bun Package Manager Reality Check 2026](https://vocal.media/01/bun-package-manager-reality-check-2026)
- [Dealing with Monorepo Hell with Bun](https://www.fgbyte.com/blog/02-bun-turborepo-hell/)
