---
name: autoresearch
description: Run a benchmark-driven optimization loop for performance, reliability, or quality targets. Use when the user wants to systematically improve startup time, runtime speed, test wall clock, bundle size, query latency, memory use, or another measurable target by forming hypotheses, making small reversible changes, measuring immediately, and keeping only clear wins.
---

# Autoresearch

Run a disciplined keep-or-discard loop. Do not optimize by feel.

## Workflow

### 1. Define the target

- Pick one primary metric the user actually cares about.
- Prefer a user-visible metric: startup time, test runtime, query latency, memory, bundle size.
- If there is no benchmark yet, create one before optimizing.

Good examples:

- daemon startup median ms
- `bun run test` wall clock
- API p95 latency
- bundle size kb

### 2. Establish a stable benchmark

- Prefer `1` warmup and `7-9` measured runs.
- Use median as the primary metric.
- Also record spread if possible, such as MAD, best, worst, or mean.
- Emit machine-readable lines like `METRIC name=value`.

If the benchmark does not exist, add the smallest useful one.

Suggested script shape:

```text
run target N times
discard warmup
compute median + spread
print summary
print METRIC lines
```

### 2a. Bootstrap the repo when needed

If the repo has no perf scaffolding yet, add the smallest useful set of files. This is straightforward work, not magic.

Prefer this layout:

- `scripts/bench-<target>.ts` or `scripts/bench-<target>.js`
- `perf/<target>.md`
- optional `perf/results.jsonl`
- optional package script like `bench:<target>`

What each file is for:

- benchmark script: produce the metric reliably
- perf note: record goal, files in scope, hypotheses, and dead ends
- results jsonl: append experiment outcomes across sessions

Do not overbuild on the first pass. The benchmark script plus a short perf note is usually enough.

Suggested bootstrap order:

1. add benchmark script
2. add package script alias if the repo has a script runner
3. add `perf/<target>.md` with goal, command, and metric names
4. only add `perf/results.jsonl` if the work will span multiple sessions or many experiments

Suggested perf note sections:

- goal
- benchmark command
- primary metric
- secondary metrics
- files in scope
- signals to watch
- hypothesis loop
- dead ends

Suggested JSONL shape:

```json
{"target":"daemon-startup","hypothesis":"startup poll interval dominates latency","metric":"daemon_startup_ms","before":190.7,"after":185.0,"decision":"discard","notes":"improvement was not durable across reruns"}
```

### 3. Capture the baseline

- Run the benchmark before editing code.
- Record the current baseline in the conversation and in a repo note or file.
- If the run is noisy, increase iterations before making code changes.

### 4. Form one hypothesis

- State one specific reason the metric might improve.
- Tie it to a real boundary: startup probe, worker init, import graph, query plan, cache miss, I/O, lock contention.
- Keep the hypothesis falsifiable.

Good example:

- startup is blocked on a deep readiness path instead of a cheap startup signal

Bad example:

- maybe we can make it faster somehow

### 5. Make the smallest change that tests the hypothesis

- Change one thing at a time.
- Prefer reversible edits.
- Avoid stacked experiments that mix multiple ideas.

Good experiments:

- reduce one polling interval
- lazy-init one worker
- move one repair step off the critical startup path
- swap one query or one import boundary

### 6. Measure immediately

- Re-run the same benchmark.
- Compare against the baseline, not intuition.
- Watch both the primary metric and the spread.

Decision rule:

- Keep only clear wins.
- Discard regressions.
- Discard results inside the noise floor unless the code got simpler too.

### 7. Check correctness

- Run the smallest focused verification that protects the changed area.
- Prefer targeted tests first, then typecheck/build if warranted.
- Do not keep a performance change that quietly breaks correctness.

### 8. Record the result

- Write down:
  - hypothesis
  - change made
  - before metric
  - after metric
  - keep or discard
  - notable side effects
- Preserve dead ends. They stop future sessions from repeating the same work.

Good storage options:

- `perf/<target>.md`
- `perf/results.jsonl`
- project note or engineering diary

Suggested JSONL shape:

```json
{"target":"daemon-startup","hypothesis":"deep health check dominates startup","metric":"daemon_startup_ms","before":310.3,"after":185.0,"decision":"keep","notes":"moved startup acceptance to cheap signal"}
```

### 9. Repeat until the curve flattens

Stop when:

- wins are smaller than the observed noise floor
- new changes add complexity without durable improvement
- the next plausible gains require architecture, not micro-tuning

At that point, switch from small tuning to larger bets.

## Debugging Layer

Use measurement plus diagnosis together.

- Use logs, traces, and timing markers to form hypotheses.
- Use a root-cause-analysis workflow when the bottleneck is unclear.
- Use the benchmark to validate or reject the hypothesis.

The benchmark answers whether a change helped.
Logs and traces help explain why.

## Guardrails

- Do not optimize before the benchmark exists.
- Do not trust a single run.
- Do not stack multiple hypotheses in one experiment.
- Do not keep clever code without a measured win.
- Do not hide real failures behind broad retries or catches just to improve the benchmark.
- Prefer explicit startup/readiness state over brittle heuristics when working on startup paths.

## Useful Outputs

When reporting progress, include:

```text
Target
- metric:
- baseline:

Hypothesis
-

Experiment
- change:
- before:
- after:
- decision:

Checks
-

Next best hypothesis
-
```

## Practical Pattern

Use this loop:

1. add benchmark
2. record baseline
3. inspect logs or traces
4. pick one hypothesis
5. make one small change
6. rerun benchmark
7. run checks
8. keep or revert
9. record the result

The point is not automation for its own sake. The point is to make performance work disciplined, resumable, and hard to fool yourself about.
