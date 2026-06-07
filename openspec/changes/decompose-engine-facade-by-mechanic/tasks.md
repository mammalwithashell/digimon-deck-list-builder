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

**FINDING (revises B1 premise):** code inspection of the 7 remaining sites shows they are NOT identical copies — they differ in removal strategy (`pop` / `remove(pos)` / `remove(index)` / `drain(..)`) and in `host_card` derivation/ordering (sites in `trash.rs` ~685/~815 compute host AFTER the push; others before). The genuinely-identical fragment is the *tail*: `trash.push(removed)` + `fire_digivolution_card_trashed(owner, target, host_card, source_card, EventCause::from(infer_effect_cause(owner)))`. So the safe, faithful B1 is to extract THAT tail into one Tier-2 helper, applied only where ordering is uniform; removal + host-derivation stay per-site. `host_card` feeds `OnDigivolutionCardTrashed`, so consolidating its derivation is parity-risky and intentionally NOT done.

- [ ] 6.1 **DEFERRED to follow-up** (see §10.4). The 7 sites differ in fire-attribution (`removed.owner` for the trash-push vs `perm.player` for the observer fire — these can differ under control-transfer), removal strategy, and host-derivation timing. Safe consolidation needs per-site parity verification of `OnDigivolutionCardTrashed` attribution; rushing it risks silent divergence. Analysis recorded; implementation deferred so each site gets individual parity attention.
- [ ] 6.2 DEFERRED (see 6.1 / §10.4).
- [ ] 6.3 DEFERRED (see 6.1 / §10.4).

## 7. Phase B3 — Remove the de_digivolve inversion (behavioral, gated)

- [x] 7.1 Moved the `de_digivolve` pop-loop + `WhenWouldBeDeDigivolved` replacement + DigiEgg-cleanup into Tier-2 `Game::de_digivolve_core` (byte-identical logic, `self.game.`→`self.`).
- [x] 7.2 Facade `de_digivolve` reduced to `can_affect_permanent` guard + delegate to `de_digivolve_core`. NOTE: `de_digivolve_from_effect` retains its `EffectContext::new(CardHandle(0), None)` construction because the guard's `source_kind` inference is load-bearing and not safely hardcodable at Game level — but it now bounces through a THIN facade method (rules machinery is out of Tier 3, which is the substantive fix). Spec scenario wording updated to match this faithful outcome.
- [x] 7.3 de_digivolve targeted tests: 32 passed / 0 failed. Full-suite gate PASS: cards_behavioral 3548 passed / 7 failed (exactly baseline), all other binaries green.

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
- [ ] 10.4 **B1 (trash-source primitive) deferred to a dedicated follow-up.** Inspection found the 7 sites non-uniform (fire-attribution owner vs perm.player, removal strategy, host-derivation timing). Extract `Game::trash_source_and_fire(trash_owner, fire_target, removed, host_card)` for the truly-identical tail (push + observer fire), migrate one site at a time each gated on its own `cards_behavioral` test, leaving removal + host-derivation per-site. Parity-sensitive (`OnDigivolutionCardTrashed`).
