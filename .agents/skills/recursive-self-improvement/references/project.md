# Project Scope Reference

Covers a single repository, app, subsystem, or library: its dev loop, runtime performance, code and dependency seams, and its own observability.

## Ledger location

Keep the per-project ledger at `docs/rsi-ledger.md` in the repo (or under `## Project: <name>` in your central notes tool when running many parallel worktrees), while executable benchmarks live in `perf/` or `scripts/bench-*`.

## 1. Project visibility gate (Lane D first)

Before optimizing code, check whether the project can see itself:
1. **Dev-loop visibility:** can we read `p50`/`p95` times and failure rates for build, typecheck, targeted test, and CI on this project?
2. **Runtime visibility:** is there an automated, repeatable benchmark or trace for the project's critical runtime metric (startup ms, request/query `p95`, frame time/dropped frames, RSS/heap growth, or bundle size)?
3. **Failure attribution:** when a check or runtime path fails, does the output pinpoint the failing invariant without manual re-runs?

If the target metric has no automated measurement, **building the benchmark or structured trace (via the `autoresearch` skill) is the run's first lever**.

## 2. The four project lanes

Measure all four lanes for the target repo, subsystem, or library within the window:

### A. Project dev loop
- **Build, typecheck, and test (`p50`/`p95`):** cold vs. incremental compile/typecheck times, unit and integration test wall-clock, test flake rate, and preview/deploy startup time.
- **CI path:** critical-path duration on PRs in this repo, queue time before runners start, and checks that retry or fail on unrelated noise.
- **Agent turns in this repo:** count steps, tool errors, repeated commands, and wait time for sessions working in this repo's worktrees.

### B. Runtime behavior
- Run or inspect the project's existing benchmarks, frame/motion traces, memory snapshots, or bundle analyzers.
- Measure **median (`p50`) and tail (`p95`/worst)** across 7–9 runs after 1 warmup (`autoresearch` baseline). Watch for bimodal splits (cold vs. warm cache, small vs. large payload) and resource leaks across repeated operations.

### C. Code & dependencies
- **Hot paths:** profile or trace the dominant execution path for `O(n²)` scans, repeated parsing/serialization, synchronous I/O on the critical path, unnecessary clones/allocations, unbounded caches, or serialized steps that can batch or parallelize.
- **Libraries fighting the goal:** identify dependencies that dominate bundle size, startup, compile time, or runtime latency, or whose API forces awkward workarounds.
- **When to Rebuild (the `first-principles` skill):** when an external library or internal abstraction blocks performance or ergonomics, trace into its source to see what it actually does on our path. State the library to replace, what a purpose-built implementation drops or changes, and the line/hour budget for v1.

### D. The project's own visibility & guards
- Check whether the project carries its own regression guards: fast micro-benchmarks (`perf/<target>.md`), phase timing receipts, invariant assertions, and lint/type rules (`poka-yoke`) that catch the project's recurring bug classes at edit time.

## 3. Project-scope recursion (Step 6 audit)

At the end of every project run, audit the project's own verification and instructions:
- **Test earned-keep:** flag slow or flaky tests in this repo that add wall-clock drag without catching real regressions, or split slow full-simulation suites into a fast iteration contract plus CI-only deep runs.
- **Benchmark & doc hygiene:** ensure new benchmarks run in seconds, emit machine-readable `METRIC name=value` lines, and keep repo `AGENTS.md` / `perf/` notes free of stale architecture claims.
