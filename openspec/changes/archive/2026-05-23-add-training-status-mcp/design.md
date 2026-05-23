## Context

Two surfaces already exist in this domain. The Rust **engine debug MCP** (`code/digimon-engine-mcp/`, PR #519) exposes per-game `LiveGame` over JSON-RPC for forensics on individual games and recordings — 22 tools, hand-rolled JSON-RPC, `data/cards.json` autodiscovery. The **training CLI** (`code/digimon_gym/agents/pilot_training.py`) produces, per run, three artifact streams:

1. `runs/<run>/console.log` — a header block followed by `[Eval @ N steps] Win rate: X% | Mean reward: Y | Games played: Z` lines (verified at `pilot_training.py:548-554`) and, during crashes, `[recorder env=N game=M step=K] ENGINE PANIC #C: PanicException: <msg>` lines emitted by the crash-resilient wrapper.
2. `runs/<run>/MaskablePPO_1/events.out.tfevents.*` — TensorBoard proto3 event files carrying `time/fps`, `rollout/ep_rew_mean`, `rollout/ep_len_mean`, `train/loss`, `train/value_loss`, `train/policy_gradient_loss`, and the `pilot/*` tags written from `_run_evaluation`.
3. `models/<run>/<run_id>/{deck_pool_snapshot.json, checkpoints/step_NNNNNNNNN.zip, recordings/<env>_<game>_<step>_<verdict>.json}` — model artifacts and crash-resilient recordings.

No agent-facing way exists to ask "which run, what step, what shape" across this surface. Operators tail logs and crack open TensorBoard manually. The same agent that has the engine MCP for per-game forensics has no equivalent for the training side. The design below carves out a small, read-only Python MCP that sits adjacent to the engine MCP and hands off recording paths when a per-game drill-down is needed.

### Worktree-branch caveat

The brief says to branch from `claude/epic-perlman-29ac0f` (HEAD `7ffe8878`). The chip's worktree (`angry-davinci-d1869c`) was actually branched from main at `008386f1` ("Fix EOT loops and clarify training eval metrics") — earlier than the brief expects. Consequences:

- The eval print format (`pilot_training.py:548-554`) **is** present here — `[Eval @ N steps] Win rate: X% | Mean reward: Y | Games played: Z`.
- The `[recorder env=N game=M step=K] ENGINE PANIC #C: PanicException: <msg>` log line and the three named gap entries (`G-DSL-OUTER-TAIL-NESTED-PARK`, `G-OPTION-PLAY-REENTRANT`, `G-DELETION-RESUME-NESTED`) are **not** in this worktree's `training_recording.py` or `qa/archetype-qa/engine-gaps.md`.
- Live `runs/generalist_v2/` artifacts are **not** present.

This proposal can be authored against the brief's stipulated contracts (they're the canonical format going forward), but **implementation must wait for** `claude/epic-perlman-29ac0f` (or its merged equivalent) to land on this branch — that's the source of truth for the panic log regex and the seed panic-family table. We mark this as a blocker in `tasks.md` Phase 0.

## Goals / Non-Goals

**Goals:**
- One Python MCP stdio server with seven read-only tools, exposing run inspection over filesystem artifacts.
- Clean domain split: engine MCP owns per-game state; training MCP owns cross-game and time-series surfaces. Cooperation through a path handoff — training MCP returns a recording path, agent calls engine MCP's `load_recording`.
- Stable parse contracts: structured sidecars where formats are subject to drift, regex over log lines only where the producer side is already structured.
- Same operator ergonomics as the engine MCP: stdio JSON-RPC, autodiscovery of artifact roots from the working directory, override flags for non-standard layouts.

**Non-Goals:**
- **Any** mutation of training state — no start, stop, pause, resume, checkpoint promotion, or model deletion. Read-only in v1.
- Engine instantiation or game simulation. The training MCP never loads a `LiveGame` or runs a step. That's the engine MCP's job.
- DB access. The hosted API owns persistence; the training MCP operates on local filesystem artifacts.
- Per-game inspection (state, hand, field, legal actions). Already covered by the engine MCP.
- A web UI, dashboard, or HTTP server. MCP stdio only.
- Hosted-API integration. The training MCP runs locally next to where the run was produced.

## Decisions

### Decision 1 — TensorBoard parsing strategy

**Choice**: `tensorboard.backend.event_processing.event_accumulator.EventAccumulator`.

**Alternatives considered**: raw `tensorboard.compat.proto.event_pb2` over the event file stream.

**Rationale**: `EventAccumulator` handles tag discovery, scalar/histogram unification, file rotation, and partial-file safety (truncated final entry on an active run is its happy path, not an exception). Its `Reload()` method picks up new events without re-reading, perfect for our active-run case. Raw proto would only win if we needed sub-second tail latency or wanted to stream events as they're written; at MCP call frequency (human-driven, seconds-apart), `Reload()` is fine. The cost is one heavy dep (`tensorboard`) — but that dep is already in the training environment, and v1 makes it explicit via `requirements-mcp.txt`.

**Behaviour on active runs**: instantiate one `EventAccumulator` per run name and cache it; call `Reload()` on every metric/tag tool invocation. Cache invalidation is mtime-based — if the event file rotates (new `MaskablePPO_<N>` directory) we drop and re-instantiate.

### Decision 2 — Eval log contract (the structured sidecar)

**Choice**: Add an append-only `runs/<run>/evals.jsonl` sidecar; emit one row per eval from `pilot_training.py:_run_evaluation`. `run_summary` reads the sidecar; do not regex-parse the console print.

**Alternatives considered**:
- (a) Regex-parse the `[Eval @ N steps] Win rate: X% | Mean reward: Y | Games played: Z` console line.
- (b) Read the TB scalar tags (`pilot/win_rate`, `pilot/mean_eval_reward`, etc.) and reconstruct eval rows from synchronized timestamps.

**Rationale**: (a) is brittle — the print format is user-visible, has changed twice in the past quarter, and SB3 buffers stdout so an active-run tail can miss the latest line. (b) is reconstructive — we'd have to join multiple TB tags on `step` and trust they were all logged in the same eval; the `games_played` cumulative counter would also need to be diffed. The sidecar is a single line of code per eval, an explicit contract, and self-describing.

**Patch shape** (target: end of `_run_evaluation` after the existing `print(...)`):

```python
# pilot_training.py — additive: emit structured eval row for the training MCP
eval_row = {
    "step": int(self.num_timesteps),
    "wall_time": time.time(),
    "win_rate": win_rate,
    "mean_reward": mean_reward,
    "draw_rate": draw_rate,
    "mean_terminal_score": mean_terminal_score,
    "mean_dense_reward": mean_dense_reward,
    "mean_eval_episode_length": mean_length,
    "games_played": self.games_played,
}
if self._eval_sidecar_path is not None:
    with open(self._eval_sidecar_path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(eval_row, separators=(",", ":")) + "\n")
```

Run-dir resolution: the callback already gets a `tensorboard_log` path through `MaskablePPO(tensorboard_log=...)`. Add an `_eval_sidecar_path: Optional[Path]` field to the callback, populated from the parent run dir (one level up from `tensorboard_log`'s `MaskablePPO_1/`). Atomicity: line-buffered append + JSON-encoded value gives us safe interleaving without locks — one writer, one reader at MCP call time.

### Decision 3 — Active-run detection

**Choice**: `console.log` mtime within the last 60 seconds counts as active.

**Alternatives considered**: a `runs/<run>/run.pid` lockfile written by `pilot_training` on start, removed on graceful shutdown; or a Unix socket / advisory file lock.

**Rationale**: mtime is zero-coordination — works for any future training entrypoint without requiring it to participate. Limitations: an idle eval phase between rollouts could push mtime past the 60s window and falsely report inactive (mitigated: a 60s window is generous given training emits multiple lines per minute under normal operation, and `pilot_training` writes every rollout to TB which also touches the event file — we can union mtimes of console.log and the latest events.out.tfevents.*); a crashed-but-not-cleaned process leaves no signal (acceptable for v1 — operator runs `list_runs` once and sees `active=false` on the stale dir). Lockfile is a v2 enhancement; flag it as an open question (see below).

### Decision 4 — Panic family table location

**Choice**: A small JSON file `qa/archetype-qa/panic-families.json`, machine-readable companion to `engine-gaps.md`. Schema:

```json
[
  {
    "family_id": "G-DSL-OUTER-TAIL-NESTED-PARK",
    "pattern": "outer effect tail.*nested park",
    "description": "DSL outer tail dispatched while a nested park is still pending",
    "status": "resolved",
    "first_seen_at": "2026-04-29",
    "resolved_at": "2026-05-20"
  },
  ...
]
```

`run_summary` loads this once per server start, compiles `pattern` to regex, and counts panic-line matches per family. Unmatched panics roll up under `other`.

**Alternatives considered**:
- (a) Hardcoded in MCP source — fine for v1 but couples MCP releases to panic-table updates.
- (b) Parse `engine-gaps.md` directly — markdown tables are unstable parse surfaces; adding a new family becomes a markdown-table contract.

**Rationale**: a tiny JSON file is the smallest stable contract. `engine-gaps.md` remains the prose source-of-truth; the JSON is the index that points back to it. The MCP stays decoupled from markdown changes; new families need only one file edit. Sourced from `engine-gaps.md`'s last three entries (seed entries: `G-DSL-OUTER-TAIL-NESTED-PARK` resolved, `G-OPTION-PLAY-REENTRANT` resolved, `G-DELETION-RESUME-NESTED` open) — written as part of Phase 1.

### Decision 5 — Package layout

**Choice**: New directory `code/digimon-training-mcp/` with its own `pyproject.toml`. Installable as a workspace member (added to the top-level `pyproject.toml`'s package list — same shape as the engine MCP's Cargo workspace integration). Runnable as `python -m digimon_training_mcp`.

```
code/digimon-training-mcp/
├── pyproject.toml
├── README.md
├── src/digimon_training_mcp/
│   ├── __init__.py
│   ├── __main__.py            # entrypoint — `python -m digimon_training_mcp`
│   ├── server.py              # MCP server bootstrap + tool registration
│   ├── runs.py                # list_runs / active-detection
│   ├── summary.py             # run_summary — header parse, eval sidecar, panic counts
│   ├── tb_metrics.py          # run_metric / run_tags — EventAccumulator cache
│   ├── recordings.py          # run_recordings — directory walk + metadata parse
│   ├── checkpoints.py         # run_checkpoints — directory walk
│   ├── deck_pool.py           # run_deck_pool — JSON read
│   ├── paths.py               # --runs-dir / --models-dir resolution
│   └── panic_families.py      # panic-families.json loader + matcher
└── tests/
    ├── conftest.py            # synthetic-run fixture (tempdir + skeleton)
    ├── test_runs.py
    ├── test_summary.py
    ├── test_tb_metrics.py
    └── test_recordings.py
```

**Alternatives considered**: tuck under `code/digimon_gym/mcp/`. Rejected because `digimon_gym` is the RL/training package; the MCP is a separate deployable surface (CLAUDE.md Service Boundaries §4) and shouldn't be importable from training code.

**Rationale**: parallel to `code/digimon-engine-mcp/`, satisfies CLAUDE.md Working Rule #24 (everything under `code/`, no new top-level dirs). Package directory uses hyphens (matches engine MCP); import name uses underscores (Python convention).

### Decision 6 — MCP protocol implementation

**Choice**: Official `mcp` Python SDK (`pip install mcp`).

**Alternatives considered**: hand-rolled JSON-RPC over stdio mirroring the engine MCP's pattern.

**Rationale**: the engine MCP was hand-rolled because the Rust MCP ecosystem was thin and the surface (initialize, tools/list, tools/call, notifications) was small enough to handwrite cleanly. Python's `mcp` SDK is mature, official, decorator-driven (`@server.list_tools`, `@server.call_tool`), and handles the JSON-RPC framing, the initialize/notifications dance, and the per-spec `{ content: [{ type: "text", text: ... }] }` wrap. Cost is one well-maintained dep. We'd reinvent ~200 lines of boilerplate to save it — not worth it.

### Decision 7 — `.mcp.json` activation

**Choice**: Add the server entry, commented out / `disabled: true`. Operators uncomment when v1 ships.

```jsonc
{
  "mcpServers": {
    // "digimon-training-mcp": {
    //   "command": "python",
    //   "args": ["-m", "digimon_training_mcp", "--runs-dir", "./runs", "--models-dir", "./models"],
    //   "disabled": false
    // }
  }
}
```

**Rationale**: zero-friction enablement once v1 lands; no risk of an unfinished server being auto-loaded during development.

### Decision 8 — Recording filename + runs/models path resolution

**The brief's stated filename `<env>_<game>_<step>_<verdict>.json` is wrong.** Verified against `code/digimon_gym/agents/training_recording.py:130-134`, the real format is:

```
{source_slug}_env_{env_index:03d}_game_{game_index:06d}_{result}_{reason}.json
```

— five fields, no `step`, separated by underscores. `source` is a free-form slug (e.g. `train`, `eval`), `env_index` is 3-digit zero-padded, `game_index` is 6-digit zero-padded, `result` is one of `win` / `loss` / `draw` / `unknown`, `reason` is `crash` / `step_limit` / `decked_out` / `defeat` / `unknown` / etc. Example matching `engine-gaps.md`'s `G-DELETION-RESUME-NESTED` entry: `train_env_000_game_000034_draw_crash.json`. Per-step count lives in the file body (`outcome.step_count`), not the filename.

**Filter semantics**:
- `"crash"` → `reason == "crash"` (crashes always produce `result=draw`, so this captures them by reason).
- `"draw"` → `result == "draw"` AND `reason != "crash"` (drawn games that weren't crashes — e.g. step_limit timeouts).
- `"all"` → no filter.
- `"win:1"` / `"win:2"` left for v2 (would need to read the file body to disambiguate winner).

**Path resolution** — the brief's described layout `runs/generalist_v2/...` paired with `models/generalist_v2/pilot_ppo_20260523_100355/...` reflects how the user actually invokes training:

- `--tensorboard-log=runs/generalist_v2` produces `runs/generalist_v2/MaskablePPO_1/events.out.tfevents.*` and the user pipes console to `runs/generalist_v2/console.log`.
- `--models-dir=models/generalist_v2` produces `models/generalist_v2/<run_name>/{checkpoints/, recordings/, deck_pool_snapshot.json}` where `<run_name>` defaults to `pilot_ppo_<timestamp>` (see `pilot_training.py:949-951`).

So `<runs-dir>` and `<models-dir>` share a logical run-name prefix (e.g. `generalist_v2`), but `<models-dir>/<name>/` has an additional timestamped subdirectory that `<runs-dir>/<name>/` does not. The MCP resolves model-side tools as:

1. For run `<name>`, look at `<models-dir>/<name>/`. If `recordings/`, `checkpoints/`, or `deck_pool_snapshot.json` exist directly under it, use them.
2. Otherwise, list child directories under `<models-dir>/<name>/`. Pick the most-recently-modified one (typically `pilot_ppo_<timestamp>`). Use its `recordings/` / `checkpoints/` / `deck_pool_snapshot.json`.
3. If `<models-dir>/<name>/` does not exist at all, model-side tools return a structured `{ ok: false, error: "no model artifacts found for run '<name>'" }`.

The disambiguation result (which timestamped subdir was chosen) is surfaced in the response as `model_run_id` so the user knows which one was inspected. A future `model_run_id` argument (v2) lets callers pick a specific one when multiple exist.

## Risks / Trade-offs

- **TensorBoard dependency weight** → Mitigation: isolated in `requirements-mcp.txt`. Hosted API and Tauri desktop don't pull it. Training runs already include it as a transitive dep of SB3.
- **Eval sidecar deployment** → Mitigation: `run_summary` falls back to regex-parsing the console line if the sidecar doesn't exist (e.g. for runs started before the sidecar landed). Both paths produce the same row shape; sidecar is preferred when present.
- **mtime active-detection false negatives** → Mitigation: union over `console.log` mtime and latest TB event-file mtime; 60s window. Document the limitation. v2 adds lockfile signal.
- **Panic-family table drift** → Mitigation: a one-line CI smoke check parses `panic-families.json` and confirms every `family_id` mentioned in `engine-gaps.md` appears in the JSON. Stretch goal; flagged in `tasks.md` Phase 5.
- **Worktree branch missing source contracts** → Mitigation: explicit Phase 0 blocker in `tasks.md` — rebase onto a branch that includes `7ffe8878` before any implementation work. Proposal/design/spec authoring can proceed against stipulated contracts.
- **MCP SDK churn** → Mitigation: pin `mcp` to a tested minor version range in `requirements-mcp.txt`; cover the tool surface with integration tests that spin up the server over stdio.
- **Recording-path handoff staleness** → If a recording file is rewritten/rotated between `run_recordings` returning a path and the engine MCP loading it, the engine MCP sees the newer content. Acceptable: recordings are append-once.
- **Server discovery of `runs/` when invoked from arbitrary cwd** → Mitigation: same ancestor-walk pattern as engine MCP's `default_cards_json_path` (walk up 6 levels looking for `./runs`). Override via `--runs-dir`.

## Resolved Open Questions

User review on 2026-05-23 resolved every remaining open question. All five lock-ins:

1. **Per-archetype in `evals.jsonl`** — **YES.** When `_archetype_games` is non-empty, the sidecar row includes a nested `by_archetype: {<archetype>: {wins, games, draws, win_rate}}` dict; absent for non-gauntlet runs.

2. **Multi-tag `run_metric`** — **YES.** `run_metric(name, tag)` accepts either a string or an array; returns a list when string, dict-of-lists when array. Spec already encodes this.

3. **`list_runs` recursion** — **NO.** v1 is single-level only; documented in `run_summary` / `list_runs` scenarios. Nesting is a v2 follow-up.

4. **`panic-families.json` location** — **`qa/archetype-qa/panic-families.json`.** Lives next to `engine-gaps.md` as the auth-of-truth-adjacent index. The MCP loads it relative to the discovered repo root (same ancestor-walk that finds `runs/`).

5. **Tool naming** — **snake_case** matching the engine MCP (`list_runs`, `run_summary`, …). Locked.
