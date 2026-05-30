## Context

Three recording formats and two replay paths exist today:

- **Native `GameRecorder` JSON** (self-play / training / eval) — fully deterministic, full information. Consumed by `ReplayRunner` (`code/digimon-engine/src/runners/replay.rs`), which is steppable (`step` / `seek` / `current_step`) and powers `LiveGame::from_recording` and the engine MCP's `load_recording`. It **trusts** the recorded action stream (`decode_action` with no mask check) and `seek`-backward **rebuilds from scratch and re-walks** (re-cloning ~4k `CardData`).
- **DCGO JSONL** (bot-vs-bot with both deck orders; PvP with opaque opponent + reveal stream) — consumed by `dcgo-replay`'s `replay_recording` (`code/tools/dcgo-replay/src/replay.rs`), a **batch, fire-and-forget** function operating on `Game` directly. It **checks** each step (actor match + mask-membership, with `sample_legal_ids`) and **aborts** on the first divergence (`ReplayOutcome::Fail{IllegalAction|ActorMismatch|WinnerMismatch|OpaqueRevealError|...}`). No cursor, no back-step, no reveal-cursor exposure.

The opaque-opponent engine mode it relies on is already built and tested by `add-dcgo-recording-parity-harness` (`Game::new_with_opaque_opponent`, `RevealQueue`/`RevealSource`, lazy security placeholders tagged `is_opaque_placeholder`, the `dcgo-replay` opaque dispatch). What is missing is an **interactive** surface: a single steppable core both paths share, snappy bidirectional stepping, a fat per-step view, and an agent workflow that turns "step to a divergence / judge an effect" into confirmed, localized bugs.

The two bug-hunting intents differ in their **oracle**:

- **Mode 1 — differential (DCGO source).** DCGO is battle-tested, so an accepted action is ground truth. Replaying through Rust and finding that Rust *masks* an action DCGO took (or reaches a different actor / winner / reveal alignment) is a mechanical Rust-bug signal. One-directional: we cannot detect "Rust offers an action DCGO would not" because the recording stores only the taken action, not DCGO's full mask.
- **Mode 2 — judge (self-play / eval source).** The recording came from the Rust engine itself, so replay reproduces it exactly — **no external oracle**. "Bugs" are faithfulness violations Claude judges by reading card text + `general_rule.pdf` + DCGO C#, using engine signals (panics, `EffectFizzled`, impossible states, mask anomalies) as leads.

## Goals / Non-Goals

**Goals:**

- One replay code path (`ReplaySession`) that serves both batch (parity harness) and interactive (MCP) callers and both recording families (native + DCGO), so they cannot drift.
- Snappy bidirectional stepping (`step_back` / `seek`) via checkpointing, working identically for deterministic and opaque games.
- A fat per-step view that supplies enough context for both the differential and judge modes from a single call.
- An agent skill (`/replay-bug-hunt`) with explicit Mode 1 and Mode 2 playbooks that drive the MCP and write confirmed findings to existing trackers.
- Keep the bug-hunting *judgment* in the skill (thick skill); keep the MCP a set of thin primitives plus a few cheap **mechanical** scanners.

**Non-Goals:**

- No change to the 2192-action layout, the DCGO JSONL schema, or the native `GameRecorder` JSON schema.
- No automated bug *fixing* — this ships the microscope, not the repairs it surfaces (mirrors the parity harness's stance).
- No detection of "Rust offers an action DCGO would not" (recording lacks DCGO's full mask). Out of scope; documented as a known limitation.
- No replacement of the batch parity report — `dcgo-replay`'s output stays byte-stable.
- No DCGO-side work (build, selection encoding, fuzzer loop) — that is the parity harness's domain.

## Decisions

### D1. One `ReplaySession` core, parameterized over a `RecordingSource` adapter

Promote `ReplayRunner` into a `ReplaySession` that owns `Game`, a cursor, a checkpoint ring, and a divergence log, and is parameterized over:

```
trait RecordingSource {
    fn build_initial_game(&self, card_data) -> Result<Game, ReplayError>;  // native restore | dcgo std/opaque
    fn steps(&self) -> &[StepSpec];        // normalized { actor, action_id, phase, source }
    fn reveal_feed(&self) -> Option<&[RevealEntry]>;  // None for native; ordered list for dcgo opaque
}
```

Two adapters: `NativeAdapter` (today's `initial_state` restore + Python-1/2 ID translation + mulligan filtering) and `DcgoAdapter` (today's `build_game` std/opaque dispatch). `ReplaySession` exposes `step_forward()`, `step_back()`, `seek(n)`, `step_view(n)`, `scan_divergences()`, and `run_to_completion()`.

**Alternative considered:** keep two runners and share only helper functions. Rejected — the parity oracle (batch) and the interactive hunter would drift; "one path that works for both" is the explicit requirement.

### D2. Per-step policy is a knob, divergence is non-fatal and pausing

A `StepPolicy { Trust, CheckThenApply }` selects behavior:

- **Trust** (native default): apply `decode_action` directly; still capture events + delta. Matches today's `ReplayRunner`.
- **CheckThenApply** (DCGO default): verify `actor == current_decision_player`, then mask-membership of `action_id`; on a miss, record a `Divergence{ kind, step, sample_legal_ids, ... }` and **pause** (do not apply, do not abort). The caller inspects and may `seek` elsewhere or stop.

Batch callers get identical semantics to today by running `run_to_completion()` and reading the divergence log — the first recorded divergence maps to the existing `ReplayOutcome::Fail` variants (`IllegalAction` / `ActorMismatch` / `WinnerMismatch` / `OpaqueRevealError`). This is the key to no-drift: the parity report is computed *from the same per-step checks* the interactive session uses.

**Alternative considered:** keep batch's abort-on-divergence and add a separate interactive mode. Rejected — duplicates the check logic and re-introduces drift.

### D3. Snapshot/restore via Arc-shared immutable state + checkpoint ring

**Revised after the task-1.1 audit** (see `notes/game-clone-audit.md`): full-state `Game` snapshots are infeasible. The mutable game graph is pervasively closure-bearing — `ModifierEntry` is explicitly `Not Clone` (`Box<dyn Fn>` condition), `pending_selection` carries a `Box<dyn FnOnce>` callback, and many parked continuations hold boxed closures — so the graph can neither be cloned nor serialized without an engine-wide refactor. Arc-wrapping the immutable data does **not** unblock it. So this change does **not** snapshot state; it uses **reset-and-replay**.

`step_back`/`seek`-backward today are slow because they clone all ~4085 `CardData` (`snapshot_card_data`) and rebuild every registry (`Game::new`), then re-walk. The fix: **reset the existing `Game` instance's mutable state in place** (reusing its already-built `card_data` + registries — no clone, no rebuild) to the recording's initial state, then replay forward to the target via the existing cheap `decode_action` path. Backward seek cost is O(target) cheap re-walk (tens of ms for realistic ~tens-of-steps games) with the expensive rebuild eliminated.

No Arc wrap is needed: reset-in-place reuses the one live `Game`, so `card_data` stays a plain `Vec<CardData>` and the hundreds of `game.card_data.push(...)` test sites are untouched. Robustness against missing a field reset is enforced by a guard test (reset-and-replay to N equals a freshly-constructed game replayed to N — byte-identical serialized views).

All user-facing capabilities ride this mechanism unchanged: `step_back` / `seek(n)` / `restore_checkpoint(n)` = reset + replay to n; counterfactual A/B = reset+replay to n, submit a different action (replay is deterministic, so the recorded line is reproducible by resetting again).

**Alternative considered:** Arc-wrap immutables + `#[derive(Clone)]`. Rejected — the closure-bearing mutable graph isn't cloneable regardless, and `Arc<Vec<CardData>>` forces churn across hundreds of test push-sites. **Also rejected:** full serde serializability of the mutable graph — multi-week engine-wide refactor; filed as possible future work if reset+replay is too slow for very long games.

### D4. Opaque reset re-attaches a fresh reveal queue at the cursor

`RevealSource` lives on `Game` as `Box<dyn RevealSource>` specifically to avoid a `Clone` bound (per parity-harness task 6.1) — another reason the graph can't be cloned. With reset-and-replay the `DcgoAdapter` owns the ordered reveal list, so an opaque reset re-attaches a **fresh `RevealQueue` positioned at the target cursor** and replays forward, consuming reveals in order. `OpaqueDeckState` (the multiset, on `Player`) is reset in place along with the rest of the mutable state. This keeps reset uniform across deterministic and opaque games.

### D5. The fat `step_view` is the single read surface for both modes

```
step_view(step_n) -> {
  recorded:   { action_id, decoded_label, card_id, actor, phase },   // what happened
  legal_now:  [ ActionExplanation, ... ],                            // what engine offers (decoded)
  divergence: { kind: mask_miss|actor|memory|phase|winner|reveal_kind|reveal_exhausted|null, ... },
  events:     [ ...events emitted by applying recorded action... ],  // includes EffectFizzled
  delta:      { memory, field_changes, security, hand_counts },      // before→after
  card_ids_in_play: [ ... ]                                          // → inspect_card
}
```

Mode 1 reads `divergence` + `legal_now`; Mode 2 reads `events` + `delta` + `recorded`. Reuses existing `explain_action` / `legal_decoded_actions` and the structured event format. Card text, rules PDF, and DCGO C# are **skill-level** context (Read tool / `inspect_card` / `$BASE_DCGO`), not MCP tools.

### D6. Partial observability surfaced via `is_opaque_placeholder`

For opaque games even the god view does not know unrevealed opponent cards; the substrate already tags them (`is_opaque_placeholder`, added in parity-harness 6.6e). The view layer (`PermanentView` / `SecurityView` / `HandView`) surfaces the flag so the agent reads "hidden" and does not mistake an unrevealed card for an engine bug. The reveal stream is the only legitimate filler.

### D7. `dcgo-replay` reduced to a batch driver; DCGO parser moves into the engine

The DCGO `RecordingV1` parser moves from `code/tools/dcgo-replay/` into the engine so the engine (and the MCP, which depends only on the engine) can build a `DcgoAdapter`. `dcgo-replay` becomes: parse → build `ReplaySession` with `DcgoAdapter` → `run_to_completion()` → map divergence log to the existing `ReplayOutcome` / parity report. The determinism test (`aggregate_is_deterministic_under_input_permutation`) must still pass byte-for-byte.

### D8. Thin MCP primitives + a few cheap mechanical scanners; thick skill

MCP gains: DCGO-aware `load_recording`, real `seek` / `step_forward` / `step_back` / `restore_checkpoint`, `replay_step_view`, and the mechanical scanners `scan_divergences` (run CheckThenApply to first/all divergences — Mode 1) and `scan_fizzles` / `scan_panics` (collect `EffectFizzled` events / recorded panics — Mode 2 leads). All scanners are deterministic and cheap, so they belong in the MCP; the *judgment* (is this faithful? localize to which card? confirm vs C#?) stays in the skill.

### D9. Findings routing

Confirmed faithfulness gaps → append to `docs/RUST_ENGINE_GAPS.md` (engine-primitive gaps) and `qa/archetype-qa/engine-gaps.md` (card-effect gaps), matching the existing campaign trackers and rule 28's "widen the substrate" flow. Mode 1 divergences that pin a specific card map naturally to the parity harness's per-card triage. The skill records: recording path + step, divergence/verdict, the card, and the source consulted (C# / PDF rule number).

### D10. Mode sequencing

Mode 2 rides the **existing native path** (eval/training recordings already load via `load_recording`), so it can ship on the fat step-view + snappy back-step + the judge skill without the DCGO loader. Mode 1 needs the `DcgoAdapter` in the engine. Tasks are ordered engine-core → MCP → tool reduction → skill, with Mode 2 reachable before the DCGO-adapter work fully lands.

## Risks / Trade-offs

- **In-place `Game::reset_for_replay` can miss a newly-added mutable field, leaking state across a reset.** → A guard test asserts reset-and-replay-to-N equals a freshly-constructed game replayed to N (byte-identical serialized views + key counters); any missed field fails it. Keep the reset method adjacent to the `Game` struct so it's the obvious place to update when fields are added.
- **Reset+replay is O(target) re-walk, not O(1) restore.** → Acceptable for realistic ~tens-of-steps games (the expensive `CardData` clone + registry rebuild is eliminated). If very long games make backward stepping sluggish, the true-snapshot serializability refactor is the documented escalation.
- **One-directional Mode 1 signal.** Cannot catch "Rust over-permits an action DCGO would reject." → Document as a known limitation; the judge mode (Mode 2) and existing card-text review partially cover over-permission.
- **Refactoring `dcgo-replay` could perturb the parity report.** → Lock it with the existing byte-identical determinism test plus the synthetic-recording integration tests before and after the refactor.
- **Replay non-determinism for RNG-consuming effects** (already noted in `replay.rs`). → Unchanged by this work; surface as a step-view note so the agent does not misread an RNG divergence as a faithfulness bug.
- **Dependency ordering.** Builds on `add-dcgo-recording-parity-harness`. → If that change is mid-flight, the engine-core + Mode 2 work (native path) can proceed first; the `DcgoAdapter` consumes the parser/opaque mode once they are stable.

## Migration Plan

1. Land engine core: in-place `Game` reset-for-replay, `ReplaySession` + `RecordingSource` + `NativeAdapter`, reset-and-replay seek (cheap backward). Native replay behavior preserved (regression-gated); guard test pins reset == fresh-construct.
2. Move DCGO parser into engine; add `DcgoAdapter`; reduce `dcgo-replay` to a driver (parity report byte-stable).
3. MCP: DCGO-aware `load_recording`, stepping/seek/restore tools, `replay_step_view`, scanners; register in `.mcp.json`.
4. Skill: `/replay-bug-hunt` with both playbooks; wire findings routing.
5. Docs: update `DEBUG_MCP.md` (lift "seek stubbed" / "no branching") and `RUST_ENGINE_API.md` (reset-and-replay contract).

Rollback: the change is additive at the API surface (new tools, new skill) plus an internal refactor; reverting restores the two-path state. The `.mcp.json` registration can be reverted independently.

## Open Questions

- Whether reset+replay needs sparse intermediate base-resets (a coarse checkpoint of *recording position* only, not state) if very long games make backward stepping sluggish; default is plain reset-to-initial + replay.
- Whether `scan_divergences` should stop at first divergence (cheapest, matches batch) or collect all (more agent context) — likely a parameter defaulting to first.
- Whether the skill should auto-spawn the parity harness as a pre-funnel (run `dcgo-replay` to pick a flagged game) or assume the user supplies a recording path.
