---
name: simplify
description: Simplify code by removing unnecessary complexity. Use when asked for a focused cleanup, a review of accumulated work or a PR, a whole-codebase simplification, or another simplify pass.
---

# Simplify

Take a fresh, critical pass over the target. Reduce unnecessary concepts, state, indirection, and work, not just line count. Fix what is worth fixing.

Preserve features, public interfaces, error behavior, and lifecycle guarantees. Removing a capability is a product decision, not a simplification finding.

## Workflow

### 1. Establish the scope

- Resolve the target from the request and conversation: specific code, accumulated work, a PR, a subsystem, or an entire codebase. Carry forward an established scope.
- Interpret "everything" within that context. A whole-codebase request includes existing code, not just recent changes. Version-control information can help inspect the target; working-tree state does not choose it.
- On repeat passes, reconsider the current state of the same target, not just the previous pass's edits.
- Briefly state the scope and proceed. Ask only when the intended target is genuinely unclear, not merely broad.
- For a PR or selected subsystem, keep review bounded to that target and relevant callers; do not expand into a historical repository-wide audit. Run existing CI checks normally.
- For an explicitly requested whole-codebase pass, read [the codebase audit workflow](references/codebase-audit.md). It adds automated discovery, coverage tracking, and a temporary shared worksheet to the workflow below.

### 2. Select perspectives and review

Inspect the target, then choose the smallest set of complementary perspectives that fits, usually one to three. Briefly name the choices and why they matter. Do not fill reviewer slots with irrelevant perspectives.

| Perspective | Question and focus |
| --- | --- |
| Necessity and structure | What can disappear, and which responsibilities belong together? Challenge dead code, speculative flexibility, obsolete compatibility, unnecessary wrappers, and scattered ownership. Consolidate concepts rather than moving complexity around. |
| Reuse and idioms | Are we reimplementing something the codebase or framework already expresses well? Check existing helpers and primitives, duplicate behavior, and near-identical paths. Do not extract helpers merely because code looks similar. |
| State and invariants | Can we eliminate bookkeeping or make invalid states impossible? Look for derivable state, redundant flags, synchronization, and lifecycle machinery. Preserve the ordering and ownership those mechanisms enforce. |
| API and call sites | Can callers express intent with fewer concepts and less ceremony? Inspect real consumers for parameter sprawl, leaky boundaries, awkward defaults, and weak type guarantees. |
| Tests and supporting material | Can we preserve evidence and guidance with less scaffolding? Look for repeated fixtures, overbuilt tests, stale docs, and redundant instructions. Preserve useful coverage and explanations. |
| Runtime efficiency | Can we avoid meaningful work or resource use without adding complexity? Select only for plausible runtime concerns: hot paths, I/O, startup, rendering, large loads, or resource lifecycles. Prefer removing work over speculative caching or bookkeeping. |

Use read-only review agents in parallel for independent perspectives or areas. Give each the established scope, its assignment, repository location, relevant code or diff, and constraints. Review directly when delegation would cost more than it adds.

For large scopes, partition coverage into explicit areas and select relevant perspectives per area. Inspect callers and shared code across boundaries, including tests and supporting docs within scope; do not mistake a sample or changed-file scan for a whole-codebase review.

Each reviewer returns the areas inspected, any coverage gaps, and its strongest actionable findings with file references, the complexity or waste removed, and a concrete fix direction. Correctness constrains every perspective; flag risks without substituting a bug hunt for simplification.

### 3. Apply worthwhile changes

- Wait for the reviewers and consolidate overlapping findings. Address coverage gaps or explicitly mark the review incomplete.
- Apply clear, behavior-preserving improvements within scope. Preserve unrelated user changes.
- Keep feature removal and public-contract redesign outside this pass unless explicitly requested. Do not offer narrower behavior as a substitute for finding internal redundancy.
- Reject speculative, cosmetic, or complexity-shifting suggestions. A clean pass is a valid result; do not manufacture edits to justify it.

### 4. Verify and report

- Run verification appropriate to the changes: affected tests, typechecks, runtime checks, or documentation checks. Report verification limits; a simplification review alone is not proof of correctness.
- Briefly summarize what became simpler, any significant rejected or deferred findings, and coverage gaps. If nothing needed changing, say so.
