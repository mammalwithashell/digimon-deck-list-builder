# Pre-Scaling Cleanup Batch (Owner-Routing Coverage + Tracker Hygiene + Failure-Mode Audit)

You are landing the final tightening pass on the Rust game engine at `code/digimon-engine/` before the next archetype-scaling wave (Track L production YAML migration). Three items, one PR (or three small focused PRs if any individual one balloons). All three are insurance work — none unblocks a card on its own, but together they put the engine in the right state to scale without serial breakages.

## The three items

1. **Owner-routing live-coverage harness** — Track E's owner-routing fix (PR #453) is dormant; no card mechanic produces `owner != controller` today. Add an end-to-end harness that exercises the routing through a real card flow.
2. **Tracker hygiene sweep** — PRs #449, #452, #453, #454, #455, #456, #458 collectively skipped `qa/archetype-qa/engine-gaps.md` and most `qa/archetype-qa/dsl/*.md` rollups. Migration agents read these rollups to know what's gap-blocked vs. ready; stale rollups produce wrong `raw_rust` carve-outs during scaling.
3. **`cards_behavioral` failure-mode audit** — PR #456's 67 fixes were surgical (+89 LOC). Sample two tests from each failure cluster and add explicit edge-case coverage for failure modes that would have failed but happened not to. Catches "regression fixed by accident" before scaling re-exposes it.

## Why this matters

Card scaling through Track L will produce a high rate of new YAML cards, each consuming the foundation tier (Tracks A–E, G, I, H, UntilCondition controller) at high cardinality. Three concrete risks if cleanup is skipped:

* Owner-routing: when a control-transfer card finally lands during scaling, latent routing bugs surface only then. The dormant fix has no live coverage to catch the regression.
* Trackers: agents authoring cards consult the per-archetype gap rollups to decide which mechanics are expressible in YAML vs. needing `raw_rust`. Stale rollups (e.g. claiming a verb is missing when PR #454 landed it; claiming a modifier is missing when PR #455 wired it) cause wrong-shape PRs that need rework.
* Failure modes: PR #456 fixed exactly the 67 failing tests. Adjacent failure modes — same code path, slightly different state — may still be broken and not have tests yet. Card scaling exercises the same paths at higher cardinality and surfaces them.

Together, an hour or two of cleanup avoids days of debug during scaling.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules 17–22 (no-approximations, TDD via DebugRunner, parity tracker check). Working Rule 4 (no `ACTION_SPACE_SIZE` / tensor / mask / PyO3 / frontend / RL contract changes).
2. `docs/RUST_ENGINE_API.md` — current `EffectContext` / `Effect` builder / `CardEffect` trait. Confirm the canonical `Permanent` shape and the owner / controller distinction.
3. `code/digimon-engine/src/permanent.rs` — `Permanent` and `PermanentHandle`. `PermanentHandle.player` is the controller; `CardSource.owner` (on the top card and each source) is the owner. The Track E owner-routing fix (PR #453) made `Game::return_to_hand` and `Game::return_to_deck_inner` consult `CardSource.owner` — confirm by reading the implementation.
4. `code/digimon-engine/src/card_source.rs` — `CardSource::new(data_idx, owner, idx)` constructor. The owner-routing tests at `tests/zone_manipulation.rs:2077,2125` use this directly to seed `owner != controller` state.
5. `code/digimon-engine/tests/zone_manipulation.rs:2070-2165` — the existing direct-mutation owner-routing tests. The harness in §1 wraps these into a card flow.
6. `code/digimon-engine/tests/cards_behavioral/` — existing failure-cluster examples for §3:
   * `lm_029.rs`, `lm_030.rs`, `lm_034.rs`, `lm_035.rs`, `lm_037.rs`, `lm_054.rs`, `lm_055.rs` — reveal-and-bottom cluster.
   * `bt17_097.rs`, `lm_054.rs`, `p_037.rs`, `p_105.rs` — Delay-option placement cluster.
   * `bt8_097.rs` — `CannotPlayDigimonByEffect` floodgate cluster.
   * `bt1_090.rs` — scheduled-effect-queue cluster.
   * `st2_13.rs`, `bt4_104.rs` — pure memory ±N main activation cluster.
   * `bt5_106.rs`, `bt24_089.rs` — effect-driven play cluster.
7. `qa/archetype-qa/engine-gaps.md` — read end-to-end. Many entries are stale; cross-reference against `docs/RUST_ENGINE_GAPS.md` (which IS current) to identify what to mark closed.
8. `qa/archetype-qa/dsl/` directory — every `*.md` rollup. Search each for entries marked `BLOCKED` or `🔴` or `🟡` and cross-reference against landed PRs.
9. PR bodies for tracker provenance (use `gh pr view <num> --json body`):
   * PR #449 (Track B replacement framework)
   * PR #450 (Track D combat centralization)
   * PR #451 (Track A event payload)
   * PR #452 (Track C foundation)
   * PR #453 (Track E zone movement)
   * PR #454 (Track E DSL verbs)
   * PR #455 (Track C deferred modifier variants)
   * PR #456 (67 regression fixes)
   * PR #457 (Track G keyword library close)
   * PR #458 (UntilCondition controller)
   Each PR's body lists what landed, which tracker entries it closed, and what it deferred. Use these as the authoritative diff between "PR claimed" and "tracker says".
10. DCGO C# reference — printed text wins on disagreements; DCGO is for what owner-routing should look like end-to-end, which observer firings are canonical, and which failure modes adjacent to the fixed ones are real.

## Work to be done

### 1. Owner-routing live-coverage harness

Add a `DebugRunner` helper that constructs an `owner != controller` state via a synthetic control-transfer fixture, then add end-to-end tests exercising every Track E helper that consumes `CardSource.owner` for routing.

#### 1a. Synthetic control-transfer helper

In `code/digimon-engine/src/debug_runner.rs` (or a sibling test-helper file), add a public test-only helper:

```rust
impl DebugRunner {
    /// Test-only: simulates a control-transfer effect by relocating an existing
    /// permanent from its owner's battle area to the opposite player's battle area
    /// while preserving `CardSource.owner`. The engine has no control-transfer
    /// effect today (no card prints one); this helper exists to seed the state
    /// for owner-routing regression tests. Removed once a real control-transfer
    /// card lands.
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn transfer_control(
        &mut self,
        from_handle: PermanentHandle,
        to_player: PlayerId,
    ) -> PermanentHandle {
        // pop from from_handle.player's battle_area at from_handle.index,
        // push into to_player's battle_area, return the new handle.
        // Asserts: source's CardSource.owner is unchanged.
    }
}
```

#### 1b. End-to-end tests

Add `code/digimon-engine/tests/owner_routing_live.rs` exercising each Track E helper through a real card effect with the synthetic transfer applied first.

Helpers to cover:
* `EffectContext::return_to_hand`
* `EffectContext::return_to_deck` (top + bottom positions)
* `EffectContext::trash`
* `EffectContext::bounce_self`
* `EffectContext::return_all_trash_to_deck_bottom(player)`
* `Game::place_permanent_on_security_observed`
* `EffectContext::security_place_stacked_card` and `_top_stacked_card`

#### 1c. Negative-case coverage

Add a test confirming the routing DOES NOT trigger when `owner == controller` (the common case).

### 2. Tracker hygiene sweep

Cross-reference each per-archetype rollup and the legacy gap tracker against landed PR bodies. Mark closed entries closed; demote workarounds where the substrate now exists.

* 2a. Build the closure index (PR bodies → entry mapping)
* 2b. Update `qa/archetype-qa/engine-gaps.md`
* 2c. Update each `qa/archetype-qa/dsl/*.md` rollup
* 2d. Update `qa/dsl-vocab-gaps.md`
* 2e. Update `docs/RUST_ENGINE_GAPS.md`
* 2f. Update `docs/RUST_PYTHON_PARITY.md`

### 3. `cards_behavioral` failure-mode audit

For each failure cluster from PR #456, add 2–3 adjacent edge-case tests covering failure modes that the surgical fix may not have caught.

* 3a. Reveal-and-bottom cluster (12 LM/P/BT22 tests)
* 3b. Delay-option placement cluster (4 BT17/LM/P tests)
* 3c. `CannotPlayDigimonByEffect` floodgate cluster (4 BT8-097 tests)
* 3d. Scheduled-effect-queue cluster (5 BT1-090 tests)
* 3e. Pure memory ±N main activation cluster (4 ST2-13 / BT4-104 tests)
* 3f. Effect-driven play cluster (7 BT5-106 / BT24-089 tests)

### 4. Documentation

Document the cleanup discipline in `docs/RUST_ENGINE_API.md`:
* The owner-routing live-coverage harness pattern
* The tracker-hygiene sweep cadence
* The failure-mode audit pattern

## Acceptance gates

* `cargo test --manifest-path code/digimon-engine/Cargo.toml --test owner_routing_live` passes
* The synthetic `transfer_control` helper is gated behind `#[cfg(any(test, feature = "test-helpers"))]`
* Every entry in `qa/archetype-qa/engine-gaps.md` is either explicitly open with a current note or marked closed with the closing PR + test command
* Every `qa/archetype-qa/dsl/*.md` rollup is walked
* `qa/dsl-vocab-gaps.md` and `docs/RUST_ENGINE_GAPS.md` audit-table summary are consistent
* Each failure cluster from PR #456 has 2–3 adjacent edge-case tests; total `cards_behavioral` count grows by ~15–25 tests
* Every player-visible choice introduced by the new tests surfaces through `pending_selection` and the action mask

## Constraints

* No-approximations: every player-visible choice surfaces through `pending_selection` and the action mask
* Working Rule 4: no `ACTION_SPACE_SIZE`, tensor profile, PyO3 export, frontend, or RL wrapper changes
* Source priority: printed text > Rules Manual > fandom wiki > DCGO C#
* TDD discipline: failing test before implementation
* No new Python-side card scripts (Working Rule 21)
* `code/engine_py_legacy/` is sunset reference (Working Rule 22)
* Synthetic `transfer_control` helper is test-only
* Tracker sweep is annotation only — no code changes; if a missing primitive is found, file a separate gap PR
* Failure-mode audit: any failing test on landing is a real regression; file as a separate bug-fix follow-up, do not patch inline
* Cross-track coordination: do not modify upstream surfaces; consume them
* Decoy trait-filter (deferred from PR #457) is NOT in scope

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
cargo test --manifest-path code/digimon-engine/Cargo.toml --test owner_routing_live
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_manipulation
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```

`cards_behavioral` baseline: 2067 passing. Target after this batch: ~2082–2092 passing (15–25 new audit tests). No regressions.

## Land order

1. Owner-routing live-coverage harness (§1)
2. Failure-mode audit (§3)
3. Tracker hygiene sweep (§2)
4. Documentation (§4)

If §1 surfaces a real owner-routing bug, file as its own PR rather than rolling into the cleanup batch. Same for §3: any adjacent-edge-case test that fails on landing is a real bug; file separately.
