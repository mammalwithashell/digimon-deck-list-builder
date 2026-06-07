## 1. Baseline & guardrails

- [x] 1.1 Capture baseline (see `BASELINE.md`): 7 pre-existing `cards_behavioral` failures (DP-aura), all else green. Bar = no NEW failures.
- [x] 1.2 Confirmed observation layer read-only (all `push` calls on local `Vec`/`String`; no `&mut Game`/`*_mut(`; all entry points take `&Game`).
- [x] 1.3 Inventoried pub-fn → mechanic buckets for `effect_context/mod.rs` (driver in `code/tools/archive/extract_facade_all.py`).

## 2. Phase A — Tier-3 facade: query split (pure movement)

- [ ] 2.1 Create `effect_context/query/` and move read accessors into `state.rs`, `event_ctx.rs`, `deletion_ctx.rs`, `source_ctx.rs` as `impl EffectContext` blocks.
- [ ] 2.2 Move facade infra (`new*`, `as_read`, `can_affect_permanent`, `override_selecting_player`, `refire_*`) into `effect_context/core.rs`.
- [ ] 2.3 Update `effect_context/mod.rs` to declare submodules and re-export; verify the full suite is green (no body edits expected).

## 3. Phase A — Tier-3 facade: action split (pure movement)

- [x] 3.1 Moved `trash` (19), `sources` (22), `play` (24), `digivolve` (12), `security` (15) into `effect_context/action/`.
- [x] 3.2 Moved `zones` (18), `combat` (15), `modifiers` (15), `digixros` (9), `scheduling` (8), `memory` (3), `suspend` (3), `replacement` (5), `refire` (3), `lifecycle` (5). 176 action methods total across 15 modules.
- [x] 3.3 Behavior gate PASS: `cargo test` → cards_behavioral 3548 passed / 7 failed (exactly the baseline set), all other binaries green. Behavior-preserving confirmed.

## 4. Phase A — Tier-2 operations: mechanic split (pure movement)

- [ ] 4.1 Create `game_actions/` and split `game_actions.rs` into `play.rs`, `digivolve.rs`, `trash.rs`, `sources.rs`, `zones.rs`, `security.rs`, `combat.rs`, `breeding.rs` mirroring the Tier-3 mechanic names; update `lib.rs` re-exports; suite-gated per file.
- [ ] 4.2 (Optional, narrowed — RQ3) Extract ONLY the `until_condition` machinery and the read-only query/aura-bonus helpers (`can_digivolve`, `has_keyword`, `*_aura_bonus`, `effects_for_card`) from `game.rs` into their own files; leave the state-machine lifecycle core intact. Full `game.rs` split is deferred to a follow-up. Suite-gated.

## 5. Phase A — Output ports (pure movement)

- [x] 5.1 Moved `observation.rs`→`observation/mod.rs` and the 5 `tensor*.rs` builders under `observation/`; `lib.rs` re-exports preserve crate-root paths (`crate::tensor`, …). `tensor_profiles/` (layout-spec) left at root. Engine + PyO3 (`cargo check`) both compile clean → external paths preserved.
- [x] 5.2 Added module-level doc on `observation/mod.rs` stating the read-only output-port invariant (entry points take `&Game`, core does not depend on observation).

## 6. Phase B1 — Shared source-trashing primitive (behavioral, gated)

- [ ] 6.1 Add/standardize a Tier-2 `game_actions` `trash_source` primitive (pop + trash-move + `fire_digivolution_card_trashed` with correct `EventCause`), reusing `trash_source_ref` / `remove_source_ref` semantics; byte-match the most common existing hand-rolled sequence.
- [ ] 6.2 Migrate the 9 facade source-trashing methods to delegate to the primitive, ONE at a time, running the relevant `cards_behavioral` + observer-order tests after each before proceeding.
- [ ] 6.3 Confirm no hand-rolled `pop()` + `trash.push()` + source-trashed-observer sequence remains in `effect_context/`; full suite + parity oracles green.

## 7. Phase B3 — Remove the de_digivolve inversion (behavioral, gated)

- [ ] 7.1 Move the `de_digivolve` pop-loop + `WhenWouldBeDeDigivolved` replacement handling from the facade into a Tier-2 `game_actions::de_digivolve`.
- [ ] 7.2 Reduce the facade `de_digivolve` to guard (`can_affect_permanent`) + delegate (matching `return_to_hand`'s shape); repoint `de_digivolve_from_effect` to call the Tier-2 fn directly (no upward `EffectContext` construction).
- [ ] 7.3 Gate on `dedigivolve-resolution-parity` + `permanent-deletion-semantics` tests passing unchanged; full suite + parity oracles green.

## 8. Placement rule & enforcement

- [ ] 8.1 Document the placement rule (rules machinery → Tier 2; facade → guards/identity/sugar/entry; Tier-3 exceptions need a doc comment) in `docs/RUST_ENGINE_API.md`.
- [ ] 8.2 Add the placement-rule lint (RQ1) asserting no `try_replace` / `self.game.fire_*` / `battle_area[..]`-write in `effect_context/` outside effect-entry points, with an explicit allowlist for documented Tier-3-only ops (digixros materials, `attach_tamer`, `play_token`). Ship in WARN / `continue-on-error` mode (deny before B3 lands would red-CI on the still-present de_digivolve inversion). Follow the `action-space-codegen-drift.yml` precedent; reuse the `code/tools/dsl-lint/` crate shape.
- [ ] 8.3 Update `CLAUDE.md` / engine docs to reference the new `<tier>/<mechanic>.rs` address scheme so new cards/mechanics land in the right module.

## 9. Verification & close-out

- [ ] 9.1 Re-run the full baseline suite (1.1) and confirm pass counts match the baseline exactly (behavior-preserving).
- [ ] 9.2 Confirm the PyO3 boundary (`digimon-engine-py`) builds and `maturin develop` + Python parity test pass (no public API drift).
- [ ] 9.3 Verify the new-capability spec scenarios (`engine-effect-context-layering`) are satisfied by the resulting structure.

## 10. Deferred follow-ups (record, do NOT implement here)

- [ ] 10.1 Record a follow-up: split `effect_context/selections.rs` (3,373 LOC, 35 `select_*` primitives) by selection-target — only if it keeps growing (RQ2).
- [ ] 10.2 Record a follow-up: full `game.rs` mechanic split beyond the narrow 4.2 extraction (RQ3).
- [ ] 10.3 Record a follow-up: promote the placement-rule lint from warn → deny/required once B1/B3 have landed and the Tier-3 exception allowlist is stable (RQ1).
