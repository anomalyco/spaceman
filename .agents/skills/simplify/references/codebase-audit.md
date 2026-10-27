# Whole-Codebase Audit

Use only when the user requests the entire codebase, not for ordinary PR cleanup.
Apply the main skill's behavior-preserving review and verification rules.

## Establish Evidence

Inventory runtime entry points, public exports, dynamically loaded code, subsystem
boundaries, existing checks, and dirty work. Keep live installations and services
outside the audit. Run independent read-only checks in parallel and save commands,
versions, raw reports, and exit statuses in the session's approved temporary area.

Use the relevant tools, not every tool on every run:

| Tool | Purpose | Interpretation |
| --- | --- | --- |
| Knip | Unused files, exports, dependencies | Model runtime/public/dynamic roots first. Normal and production-only graphs answer different questions. |
| TypeScript unused checks | Unused locals and parameters | Reuse the repository's compiler; retain its typecheck and test baseline. |
| jscpd | Repeated token sequences | Advisory candidates, not a zero-duplication target. |
| ast-grep | Specific repeated structures | Test each query on positive and negative examples before trusting matches. |
| Existing ESLint/Biome | Unreachable or redundant expressions | Reuse existing configuration. If a temporary audit is warranted, isolate its installation/config; do not introduce a competing permanent stack. |

Use appropriate equivalents outside JavaScript/TypeScript. Do not run bulk autofix.
For Knip production mode, mark production entry/project patterns with trailing `!`.
Test imports can conceal unused implementations in the normal graph, but a
production-only unused export can still have callers inside its own module.
Never delete public exports merely because the repository has no consumer.

Done when each subsystem has a coverage assignment or an explicit gap, and the
baseline distinguishes tool findings from command/configuration failures.

## Coordinate One Worksheet

Create one temporary Markdown worksheet and announce its path. Do not commit it
unless requested. The coordinator is its sole writer; reviewers submit findings.
Record coverage, raw-report paths, decisions, and these fields per finding:

```ts
id | axis | file/symbol | evidence | proposed deletion | preserved behavior
   | owner/files | status | verification

proposed -> verified -> claimed -> implemented -> checked
                    \-> rejected / deferred
```

Give one reviewer ownership of repo-wide automated reachability findings. Assign
other read-only reviewers bounded subsystems plus complementary perspectives from
the main skill: duplication, derivable state, resource ownership, or boundaries.
Choose only useful perspectives; do not create one agent per tool automatically.

Merge reports about the same underlying rule under one finding ID. Reviewers may
overlap in what they inspect; implementation writers may not overlap in files.
Check callers, receiver/argument behavior, evaluation order, errors, and resource
lifetimes before accepting a match. For example, replacing `x => f(x)` with `f`
can expose an array index as a meaningful second argument.

Done when every reported finding has one disposition and every proposed edit has
evidence, a verification plan, and an explicit file owner.

## Implement And Recheck

Present the consolidated worksheet for one scope approval unless the user already
authorized those changes. Then proceed in small, non-overlapping batches without
asking again for each approved fix. Recheck evidence against current source before
editing; earlier changes can invalidate later findings.

After integration, rerun affected checks and refresh audit reports. Have an
independent reviewer inspect nontrivial shared-state or lifecycle changes. Mark a
finding checked only with verification evidence; keep rejected/deferred findings
visible rather than suppressing them to make a report green.

Report net source reduction separately from added tests/tooling. State remaining
coverage gaps, expected advisory findings, and what was not tested. A clean scan
is not proof that all code is used; passing tests are not proof of equivalence.
Finish when approved changes are verified and every remaining finding is tracked.
