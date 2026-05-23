## 0. Prerequisites (DONE 2026-05-23)

- [x] 0.1 Fast-forward merged this worktree from `008386f1` to `claude/epic-perlman-29ac0f` (HEAD `7ffe8878`). No conflicts; OpenSpec files survived intact.
- [x] 0.2 Verified `code/digimon_gym/agents/training_recording.py:194-202` emits the panic line literally as `[recorder env={env_index} game={game_index} step={episode_steps}] ENGINE PANIC #{crash_count}: {exc_type}: {exc_msg[:200]}`. Regex contract: `^\[recorder env=(?P<env>\d+) game=(?P<game>\d+) step=(?P<step>\d+)\] ENGINE PANIC #(?P<count>\d+): (?P<exc_type>\S+): (?P<msg>.*)$`. Exception type is typically `PanicException` for engine panics.
- [x] 0.3 Verified `qa/archetype-qa/engine-gaps.md` lines 466, 487, 498 carry the three seed panic-family entries: `G-DSL-OUTER-TAIL-NESTED-PARK` (RESOLVED 2026-05-23), `G-OPTION-PLAY-REENTRANT` (RESOLVED 2026-05-23), `G-DELETION-RESUME-NESTED` (OPEN).
- [x] 0.4 User review on 2026-05-23 resolved all five open questions in design.md (panic-families.json → `qa/archetype-qa/`; rest per recommendations). Locked.
- [x] 0.5 Verified recording filename format from `training_recording.py:130-134` — `{source}_env_{env:03d}_game_{game:06d}_{result}_{reason}.json` — NO step in filename. Updated design.md §Decision 8 and spec.md §run_recordings accordingly.

## 1. Panic-family JSON contract

- [ ] 1.1 Create `qa/archetype-qa/panic-families.json` with the three seed entries (`G-DSL-OUTER-TAIL-NESTED-PARK`, `G-OPTION-PLAY-REENTRANT`, `G-DELETION-RESUME-NESTED`), each carrying `family_id`, `pattern` (regex string), `description`, `status`, `first_seen_at`, and optionally `resolved_at`.
- [ ] 1.2 Add a one-paragraph note to `qa/archetype-qa/engine-gaps.md` pointing to `panic-families.json` as the machine-readable index, and confirming the markdown remains the prose source-of-truth.
- [ ] 1.3 Hand-derive each `pattern` from prose evidence in the corresponding `engine-gaps.md` section (e.g. for `G-DSL-OUTER-TAIL-NESTED-PARK`, derive from "nested DSL outer-tail park" phrasing). Confirm each pattern matches its example panic message text and does NOT cross-match the other two families. Test against a synthetic console-log fixture with three known panic lines (one per family) plus one "unmatched" line.
- [ ] 1.4 Capture the verified panic-line regex from §0.2 into `panic_families.py::PANIC_LINE_RE` as a module-level constant.

## 2. Eval sidecar emission in pilot_training

- [ ] 2.1 Add `_eval_sidecar_path: Optional[Path]` field to the periodic-eval callback class in `code/digimon_gym/agents/pilot_training.py`. Resolve it from `tensorboard_log` (parent dir of `MaskablePPO_<N>/`).
- [ ] 2.2 At the end of `_run_evaluation` (after the existing `print(...)`), append a JSON row to `_eval_sidecar_path / "evals.jsonl"` per the patch shape in design.md §Decision 2. Use append-mode + line-buffered write.
- [ ] 2.3 Include `by_archetype: {<archetype>: {wins, games, draws, win_rate}}` nested dict in the sidecar row when `_archetype_games` is non-empty (resolved 2026-05-23).
- [ ] 2.4 Add a unit test under `code/tests/rl/` that runs the callback against a stub model with two synthetic evals and asserts `evals.jsonl` has exactly two lines with the required fields.
- [ ] 2.5 Confirm by running one short training session (`--timesteps 5000 --eval-freq 1000`) that the sidecar appears under `runs/<run>/evals.jsonl` and contains 5 rows.

## 3. Package scaffolding

- [ ] 3.1 Create directory `code/digimon-training-mcp/` with `pyproject.toml`, `README.md`, `src/digimon_training_mcp/__init__.py`, and `src/digimon_training_mcp/__main__.py`.
- [ ] 3.2 In `pyproject.toml`, declare the package name `digimon-training-mcp`, the importable name `digimon_training_mcp`, deps `mcp`, `tensorboard`, and the entrypoint `python -m digimon_training_mcp`.
- [ ] 3.3 Add `code/digimon-training-mcp` to the top-level `pyproject.toml` workspace members. Confirm `pip install -e code/digimon-training-mcp` works.
- [ ] 3.4 Create `requirements-mcp.txt` at repo root with `mcp>=<pinned>` and `tensorboard>=<pinned>`. Do NOT add to `requirements-training.txt` or `requirements.txt`.
- [ ] 3.5 Add a commented-out `digimon-training-mcp` entry to `.mcp.json` per design.md §Decision 7.

## 4. Server bootstrap + path resolution

- [ ] 4.1 Implement `src/digimon_training_mcp/__main__.py` as `argparse(--runs-dir, --models-dir) → asyncio.run(server.run())`.
- [ ] 4.2 Implement `src/digimon_training_mcp/paths.py`:
    - `resolve_runs_dir(arg: Optional[Path]) → Path` — explicit arg wins; else walk up 6 ancestors of cwd looking for `./runs`.
    - `resolve_models_dir(arg: Optional[Path]) → Path` — same shape for `./models`.
- [ ] 4.3 Implement `src/digimon_training_mcp/server.py` using the `mcp` SDK: `Server("digimon-training-mcp")`, `@server.list_tools()` enumerating the seven tools with their JSON schemas, `@server.call_tool()` dispatching to handlers.
- [ ] 4.4 Unit test: server starts, `tools/list` returns exactly seven tool names with schemas (no actual filesystem access required for this test).

## 5. Tool handlers

- [ ] 5.1 Implement `runs.py::list_runs(runs_dir) → List[Dict]` per spec §list_runs tool. Union mtime over `console.log` and latest `events.out.tfevents.*`.
- [ ] 5.2 Implement `summary.py`:
    - `_parse_header(console_log_path) → Dict` — regex over the header block (algo/opponent/steps/eval-freq/profile/hash/deck pool).
    - `_read_eval_sidecar(run_dir) → List[Dict]` — read `evals.jsonl`, return last N rows.
    - `_fallback_parse_eval_console(console_log_path) → List[Dict]` — regex over `[Eval @ N steps] ...` lines if sidecar absent.
    - `_count_panics(console_log_path, panic_families) → Dict` — line-scan, regex-match each panic line against `panic_families` patterns, count by family, roll up unmatched under `other`.
    - `_tail_console(console_log_path, n=50) → List[str]` — read last `n` lines.
    - `run_summary(name, tail_evals) → Dict` — compose the above per spec.
- [ ] 5.3 Implement `tb_metrics.py`:
    - `_AccumulatorCache` — `{run_name: EventAccumulator}` keyed by run name, invalidated when the TB event-file inode changes.
    - `run_metric(name, tag, since_step) → List[Dict] | Dict[str, List[Dict]]` — accept string or array tag, call `Reload()`, fetch scalar series, filter `since_step` server-side.
    - `run_tags(name) → List[str]` — `Reload()` then `.Tags()['scalars']`.
- [ ] 5.4 Implement `paths.py::resolve_model_run_dir(models_dir, name) → Optional[Tuple[Path, str]]` — return `(resolved_dir, model_run_id)`. Check `models_dir/name/` for direct `recordings|checkpoints|deck_pool_snapshot.json` markers; if absent, list child dirs by mtime desc and return the newest. Used by 5.5 / 5.6 / 5.7.
- [ ] 5.5 Implement `recordings.py::run_recordings(name, filter, limit) → Dict` per spec §run_recordings. Resolve model-run dir via 5.4, glob `recordings/*.json`, parse filename via the regex from spec, apply filter (`crash` = reason="crash"; `draw` = result="draw" AND reason!="crash"; `all` = no filter), sort by mtime desc, truncate. Return `{model_run_id, recordings: [...]}`.
- [ ] 5.6 Implement `checkpoints.py::run_checkpoints(name) → Dict` per spec §run_checkpoints. Resolve via 5.4, glob `checkpoints/step_*.zip`, parse 9-digit step, sort by step asc. Return `{model_run_id, checkpoints: [...]}`.
- [ ] 5.7 Implement `deck_pool.py::run_deck_pool(name) → Dict` per spec §run_deck_pool. Resolve via 5.4, read JSON, derive `deck_count` from `decks.length`, return verbatim with `model_run_id` field added.
- [ ] 5.8 Implement `panic_families.py`:
    - `PANIC_LINE_RE` module-level regex matching `[recorder env=N game=N step=N] ENGINE PANIC #N: <ExcType>: <msg>` per §0.2.
    - `load_panic_families(repo_root) → List[PanicFamily]` — locate `qa/archetype-qa/panic-families.json` relative to `repo_root` (the result of the ancestor-walk), parse, compile each `pattern` to `re.compile`.
    - `match_family(panic_msg, families) → Optional[str]` — return matching `family_id` or None (caller rolls up unmatched under `"other"`).

## 6. Per-tool tests

- [ ] 6.1 Test `list_runs` against a tempdir fixture with three synthetic runs of varying mtimes — asserts ordering and the `active` flag boundary.
- [ ] 6.2 Test `run_summary` against a synthetic run with: header block, 12 sidecar rows, 7 panic lines (2 families × 3 + 1 unmatched), 200-line console — asserts every field in spec §run_summary scenarios.
- [ ] 6.3 Test `run_summary` fallback path against a synthetic run with no sidecar but with `[Eval @ N steps]` console lines — asserts the regex-parsed `evals` rows.
- [ ] 6.4 Test `run_metric` (single + multi-tag) and `run_tags` against a synthetic TB event file written via `tf.compat.v1.summary.FileWriter` in the fixture.
- [ ] 6.5 Test `run_metric` `since_step` server-side filtering.
- [ ] 6.6 Test `run_metric` `Reload()` picks up new events when the event file is appended to between calls.
- [ ] 6.7 Test `run_recordings` filter modes (crash/draw/all) and limit truncation.
- [ ] 6.8 Test `run_checkpoints` step parsing and ordering.
- [ ] 6.9 Test `run_deck_pool` happy path + missing-file structured-error path.
- [ ] 6.10 Test path resolution: cwd two levels above runs/, no flags, server finds them; explicit `--runs-dir` overrides ancestor-walk.

## 7. Integration test

- [ ] 7.1 Add `code/digimon-training-mcp/tests/integration.py` that spawns the server over stdio, sends `initialize`, `tools/list`, then exercises each of the seven tools against the synthetic-run fixture. Mirror the shape of `code/digimon-engine-mcp/tests/integration.rs`.
- [ ] 7.2 Add a round-trip test confirming `run_recordings` returns absolute paths that the engine MCP's `load_recording` accepts. Run both servers; chain the call.

## 8. CI + docs

- [ ] 8.1 Add `python -m pytest code/digimon-training-mcp/tests -v` to CI (a new job — does not block the existing pytest run).
- [ ] 8.2 Add a smoke check that parses `panic-families.json` and confirms each `family_id` appears in `engine-gaps.md` (stretch goal per design.md §Risks).
- [ ] 8.3 Write `docs/TRAINING_MCP.md` — companion to `docs/DEBUG_MCP.md`. Sections: overview, tool reference, autodiscovery, the engine-MCP bridge example, troubleshooting.
- [ ] 8.4 Add a `docs/INDEX.md` entry.
- [ ] 8.5 Add a paragraph to `CLAUDE.md` under "Commands" introducing `python -m digimon_training_mcp` alongside the existing engine MCP commands.
- [ ] 8.6 Update `CLAUDE.md` Service Boundaries section to add the training MCP as a fourth deployable surface (read-only operator tool, local + dev only).

## 9. Acceptance gates

- [ ] 9.1 Activate `digimon-training-mcp` in `.mcp.json` (uncomment).
- [ ] 9.2 Manual smoke: against the live `runs/generalist_v2/` run, exercise each of the seven tools via the MCP client and confirm outputs make sense.
- [ ] 9.3 Chain test: `run_recordings(filter="crash", limit=1)` → grab path → pass to engine MCP's `load_recording` → step through the crash frame.
- [ ] 9.4 Confirm the training MCP **never** writes to `runs/` or `models/`. Run a `runs/` mtime audit before and after a 5-minute MCP exercise session and confirm no file under `runs/` changed because of the MCP.

## 10. Out of scope (v1 — track separately)

- [ ] 10.1 Lockfile / pid-based active-run detection (Decision 3 follow-up).
- [ ] 10.2 Run controls: start, stop, pause, resume, promote checkpoint (explicit Non-Goal in design.md).
- [ ] 10.3 HTTP / web UI surface.
- [ ] 10.4 Cross-run comparison tools (e.g. `compare_runs`, `metric_diff`).
- [ ] 10.5 Hosted-API integration (training MCP is local-only in v1).
- [ ] 10.6 Win-by-player filter modes for `run_recordings` (`"win:1"` / `"win:2"`).
