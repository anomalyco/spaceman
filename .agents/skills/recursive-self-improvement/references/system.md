# System Scope Reference

Covers the whole engineering system: parallel AI agents, the build/test fleet, machines, human attention, and this loop's own tools.

**Ledgers:** Read the user's system ledger note and the instrumentation inventory, creating them if missing.

## 1. Self-knowledge gate

Before hunting bottlenecks, verify the system can answer four questions from data for the window:
1. **What ran?** (sessions, subagents, build/test jobs, scheduled automations)
2. **How long did each phase take?** (`p50`/`p95` across model think, tool run, queue/sync, build/test, and CI)
3. **Who waited on what?** (agents idle waiting on human approval/pickup, jobs waiting on fleet slots, PRs waiting on CI or review)
4. **What did it cost per outcome?** (lead time, rework rounds, and spend per merged PR vs. abandoned work)

If any of the four cannot be answered from telemetry, **visibility is the run's first lever**: build or repair that collector (using §3–§4) before proposing workflow changes.

## 2. The four system lanes

1. **Human wait & attention:** recent agent sessions (repeated asks, corrections, long idle gaps before user replies), daily logs or friction notes, and stalled todos.
2. **Agent turns & models:** agent session database or transcripts (turns per task, empty stops, compactions, tool-error rates, TTFT, cost per session tree).
3. **Fleet & CI queues:** build/test runner logs (`queue_ms`, `sync_ms`, `build_ms`, `test_ms`, flake rate) and bounded `gh api` REST calls for PR lead time and CI queue wait.
4. **Machines & hot paths:** host CPU, memory pressure, swap, free disk, alert history, and hot code paths touched across sessions.

## 3. Deriving human-wait and cost per merged PR (zero runtime overhead)

Both metrics derive read-only from the OpenCode SQLite DB and GitHub REST API without extra daemons.

- **Safe SQLite rules:** open read-only (`sqlite3 'file:/path/to/opencode.db?mode=ro'` or `new Database(path, { readonly: true })`). Filter `session_message` by `WHERE time_created >= :since_ms AND time_created < :until_ms` and always write `GROUP BY +session_id` (the unary `+` prevents SQLite from choosing `(session_id, seq)` and scanning 20 GB of history). Never call `GET /api/experimental/session/stats`.
- **Fast form:** read `session_message` with `FROM session_message m INDEXED BY session_message_session_type_seq_idx WHERE m.type = 'assistant'` (swap in the type you need) over small `time_created` windows. Size blobs with `octet_length(m.data)`, which reads the record header; `length(m.data)` reads every overflow page and stalls for minutes.
- **Human vs wake prompts:** top-level `user` rows with `displayText` null are synthetic wakes (subagent completions, `skulk`, routines, alerts). Count only rows with `displayText` set as the user's prompts and report the two wait distributions separately. Skip `idle`/`system` rows when pairing an assistant `stop` with the next `user` row.
- **Cost:** `session_v2.cost` is lifetime; sum assistant `$.cost` within the window for window spend.

```sql
SELECT +m.session_id AS session_id, s.parent_id, s.title,
       COUNT(*) FILTER (WHERE m.type = 'assistant') AS assistant_turns,
       ROUND(SUM(CASE WHEN m.type = 'assistant' THEN COALESCE(json_extract(m.data, '$.cost'), 0) ELSE 0 END), 4) AS cost_usd,
       MIN(m.time_created) AS first_ms, MAX(m.time_updated) AS last_ms
FROM session_message m INDEXED BY session_message_session_type_seq_idx JOIN session_v2 s ON s.id = m.session_id
WHERE m.type = 'assistant' AND m.time_created >= :since_ms AND m.time_created < :until_ms
GROUP BY +m.session_id ORDER BY cost_usd DESC LIMIT 50;
```

- **Human-wait time (`wait`) and task time split:** in top-level sessions (`parent_id IS NULL`), order messages by `seq` and measure `next_user.time_created - assistant.time_updated` after each assistant turn (`json_extract(data, '$.finish') = 'stop'` or pending tool approval). Split into **approval/question wait** (`< 4h` where an agent asked or blocked on permission) and **pickup/review wait** (subagent or PR finished, awaiting human action). Stack `think_ms + tool_ms + queue_ms + human_wait_ms`; the largest slice is the primary bottleneck candidate.
- **Cost per merged PR (`pr` joined with `turn`):** fetch closed PRs via one bounded REST call (`gh api 'repos/<owner>/<repo>/pulls?state=closed&sort=updated&direction=desc&per_page=50'` — never `gh pr checks --watch`). Join PR branches/numbers to author + child sessions (`session_v2.parent_id`) to compute **PR lead time** (`first_prompt → merged_at` `p50`/`p95`), **cost per merged PR** (`SUM(cost_usd) ÷ merged_prs`, a diagnostic for churn), and **waste share** (spend on abandoned PRs or `review_rounds > 1`).

## 4. Minimal wide-event schema & collector hygiene

When Step 1 builds missing visibility, emit append-only JSONL rows sharing one envelope:
`{"ts","host","kind","id","parent_id","session","worktree","repo","model","dur_ms","ok","attrs":{}}` (keep high-cardinality IDs like `session`, `worktree`, `commit`, `test`; omit prompts and file contents).

| Kind | Source | Required `attrs` | Answers |
| --- | --- | --- | --- |
| `job` | Build/test wrapper (`hivemind` or shim) | `queue_ms`, `sync_ms`, `run_ms`, `exit`, `first_failure`, `cache_hit` | Time-to-signal, queue wait, flake rate |
| `turn` | Session DB extractor | `tokens_in`, `tokens_out`, `cache_read`, `cost_usd`, `think_ms`, `tool_ms`, `tool_errors`, `stop_reason` | Turns/task, TTFT, empty stops, provider RED |
| `wait` | Session DB extractor | `human_wait_ms`, `reason` (`approval` · `question` · `pickup`) | Human bottleneck, Little's Law WIP |
| `pr` | Daily `gh api` REST poll | `opened_at`, `first_ci_green_at`, `merged_at`, `review_rounds`, `reverted` | Lead time, rework rate, cost per merged PR |
| `sample` | 60 s host sampler | `cpu_pct`, `mem_pressure`, `swap_gib`, `disk_free_gib`, `slots_busy`, `slots_total`, `active_agents` | Saturation `ρ = busy/total`, WIP |

**Collector hygiene (Step 6):** register every collector in the instrumentation inventory. Enforce `<1%` avg CPU, `<50 MB` RSS, `nice 10+`, poll `≥60 s`, wrapper `<20 ms`, and `≤5 MB/day` rotated at 14–30 days. Propose retiring any collector unread across two runs or duplicating another stream.
