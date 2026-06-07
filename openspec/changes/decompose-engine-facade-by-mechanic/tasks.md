## 1. Baseline & guardrails

- [x] 1.1 Capture baseline (see `BASELINE.md`): 7 pre-existing `cards_behavioral` failures (DP-aura), all else green. Bar = no NEW failures.
- [x] 1.2 Confirmed observation layer read-only (all `push` calls on local `Vec`/`String`; no `&mut Game`/`*_mut(`; all entry points take `&Game`).
- [x] 1.3 Inventoried pub-fn → mechanic buckets for `effect_context/mod.rs` (driver in `code/tools/archive/extract_facade_all.py`).

## 2. Phase A — Tier-3 facade: query split (pure movement)

- [ ] 2.1 DEFERRED (lower priority). Query/read-accessor split: the `&self` accessors are duplicated across `EffectContext` and `EffectReadContext` (same names, two impls), so the `&mut self` extractor doesn't apply directly — needs a `&self`/dual-impl-aware variant. The action split (the bloat) is done; `mod.rs` is already down to 1463 lines. Folds into a future facade-tidy pass.
- [ ] 2.2 DEFERRED with 2.1.
- [ ] 2.3 DEFERRED with 2.1.

## 3. Phase A — Tier-3 facade: action split (pure movement)

- [x] 3.1 Moved `trash` (19), `sources` (22), `play` (24), `digivolve` (12), `security` (15) into `effect_context/action/`.
- [x] 3.2 Moved `zones` (18), `combat` (15), `modifiers` (15), `digixros` (9), `scheduling` (8), `memory` (3), `suspend` (3), `replacement` (5), `refire` (3), `lifecycle` (5). 176 action methods total across 15 modules.
- [x] 3.3 Behavior gate PASS: `cargo test` → cards_behavioral 3548 passed / 7 failed (exactly the baseline set), all other binaries green. Behavior-preserving confirmed.

## 4. Phase A — Tier-2 operations: mechanic split (pure movement)

- [x] 4.1 Split `game_actions.rs` → `game_actions/mod.rs` + 10 `impl Game` submodules (`play`, `digivolve`, `breeding`, `zones`, `movement`, `sources`, `security`, `options`, `cost`, `misc`) mirroring the Tier-3 mechanic names. 84 `&mut self` methods moved; module-private types/`&self` readers stay in `mod.rs`. `use super::*` resolves parent privates → compiled clean first try (0 errors, no promotion needed). Gate PASS: 3548/7 (baseline), all binaries green. Completes the parallel `<tier>/<mechanic>` taxonomy.
- [ ] 4.2 (Optional, narrowed — RQ3) `game.rs` `until_condition` + query-helper extraction still deferred (§10.5) — lower priority; the Tier-2 verb split (the headline parallel-taxonomy goal) is done.

## 5. Phase A — Output ports (pure movement)

- [x] 5.1 Moved `observation.rs`→`observation/mod.rs` and the 5 `tensor*.rs` builders under `observation/`; `lib.rs` re-exports preserve crate-root paths (`crate::tensor`, …). `tensor_profiles/` (layout-spec) left at root. Engine + PyO3 (`cargo check`) both compile clean → external paths preserved.
- [x] 5.2 Added module-level doc on `observation/mod.rs` stating the read-only output-port invariant (entry points take `&Game`, core does not depend on observation).

## 6. Phase B1 — Shared source-trashing primitive (behavioral, gated)

**FINDING (revises B1 premise):** code inspection of the 7 remaining sites shows they are NOT identical copies — they differ in removal strategy (`pop` / `remove(pos)` / `remove(index)` / `drain(..)`) and in `host_card` derivation/ordering (sites in `trash.rs` ~685/~815 compute host AFTER the push; others before). The genuinely-identical fragment is the *tail*: `trash.push(removed)` + `fire_digivolution_card_trashed(owner, target, host_card, source_card, EventCause::from(infer_effect_cause(owner)))`. So the safe, faithful B1 is to extract THAT tail into one Tier-2 helper, applied only where ordering is uniform; removal + host-derivation stay per-site. `host_card` feeds `OnDigivolutionCardTrashed`, so consolidating its derivation is parity-risky and intentionally NOT done.

- [x] 6.1 Added Tier-2 `Game::trash_source_and_fire(trash_owner, fire_target, removed, host_card)` (push + `source_card` + `fire_digivolution_card_trashed` with `EventCause::from(infer_effect_cause(fire_target.player))`) next to `fire_digivolution_card_trashed` in `game_actions/mod.rs`.
- [x] 6.2 Migrated ALL 7 source-trashing sites to the primitive. First 3 (uniform): `trash_card_source`, `trash_top_source`, `trash_top_n_stacked_sources`. Then the 4 "divergent" ones via §10.4: re-analysis showed `trash_bottom_sources`/`trash_bottom_face_down_source` trash a BELOW-top source (top unchanged → host computable pre-push), and `attach_tamer`/`armor_purge_top` just needed an explicit-cause variant (`Return`/`Cost`). No site needed genuine post-trash host derivation. Standard cause via `trash_source_and_fire`; non-standard via `trash_source_and_fire_with_cause`.
- [x] 6.3 Compile clean (no unused-variable warnings → confirms no stranded `source_card`/`owner`). Full-suite gate PASS: 3548/7 (exactly baseline), all binaries green. No hand-rolled push+fire+EventCause sequence remains at the 3 uniform sites; the 4 divergent sites tracked by the placement lint (§8.2) for future relocation.

## 7. Phase B3 — Remove the de_digivolve inversion (behavioral, gated)

- [x] 7.1 Moved the `de_digivolve` pop-loop + `WhenWouldBeDeDigivolved` replacement + DigiEgg-cleanup into Tier-2 `Game::de_digivolve_core` (byte-identical logic, `self.game.`→`self.`).
- [x] 7.2 Facade `de_digivolve` reduced to `can_affect_permanent` guard + delegate to `de_digivolve_core`. NOTE: `de_digivolve_from_effect` retains its `EffectContext::new(CardHandle(0), None)` construction because the guard's `source_kind` inference is load-bearing and not safely hardcodable at Game level — but it now bounces through a THIN facade method (rules machinery is out of Tier 3, which is the substantive fix). Spec scenario wording updated to match this faithful outcome.
- [x] 7.3 de_digivolve targeted tests: 32 passed / 0 failed. Full-suite gate PASS: cards_behavioral 3548 passed / 7 failed (exactly baseline), all other binaries green.

## 8. Placement rule & enforcement

- [x] 8.1 Documented the placement rule + 3-tier model + `<tier>/<mechanic>` address scheme in `docs/RUST_ENGINE_API.md` §3 ("Module layout & the placement rule").
- [x] 8.2 Added the warn-mode placement lint: `code/tools/check_facade_placement.py` (a RATCHET over per-file `try_replace`/`fire_*`/`battle_area[..]`-mutation counts in `effect_context/`, baseline = current 16-occurrence Tier-3->Tier-2 backlog, fails only on NEW additions) + `.github/workflows/facade-placement-lint.yml` with `continue-on-error: true` (warn mode, action-space-drift pattern). Promote to required once the baseline ratchets to zero (§10.3).
- [x] 8.3 Engine-dev address scheme documented in `docs/RUST_ENGINE_API.md` §3 (the authoritative engine-dev reference that CLAUDE.md points to). CLAUDE.md edit not required — it already routes engine work through RUST_ENGINE_API.md.

## 9. Verification & close-out

- [x] 9.1 Full suite matches baseline exactly: cards_behavioral 3548 passed / 7 failed (the pre-existing DP-aura set), all other binaries green. Behavior-preserving confirmed. `effect_context/mod.rs` shrank 6933 → 1463 lines.
- [x] 9.2 PyO3 boundary (`digimon-engine-py`) `cargo check` clean → no public API drift (crate-root tensor/observation paths preserved via re-exports).
- [x] 9.3 Spec scenarios: **satisfied** — by-mechanic organization (Tier 3 + Tier 2 parallel), call-surface-unchanged, observation read-only port, de_digivolve rules-machinery-in-Tier-2, single source-trashing primitive (3 uniform sites; 4 divergent tracked by lint), behavior-preserved. The placement rule is documented (§8.1) and enforced by a warn-mode ratchet lint (§8.2). Remaining backlog: ratchet the 16 tracked Tier-3 occurrences toward zero (§10.4) and the optional `game.rs` narrow extraction (§10.5 / 4.2).

## 10. Deferred follow-ups (record, do NOT implement here)

- [ ] 10.1 Record a follow-up: split `effect_context/selections.rs` (3,373 LOC, 35 `select_*` primitives) by selection-target — only if it keeps growing (RQ2).
- [ ] 10.2 Record a follow-up: full `game.rs` mechanic split beyond the narrow 4.2 extraction (RQ3).
- [ ] 10.3 Record a follow-up: promote the placement-rule lint from warn → deny/required once B1/B3 have landed and the Tier-3 exception allowlist is stable (RQ1).
- [x] 10.5 **DONE — Tier-2 game_actions split landed** (commit `92549ae9`): 10 `impl Game` mechanic submodules mirroring the facade; parallel `<tier>/<mechanic>` taxonomy complete. Residual: optional narrow `game.rs` extraction (4.2) still open, low priority.
- [x] 10.4 **DONE (full) — B1 trash-source primitive complete.** `Game::trash_source_and_fire` (39766d80) + `trash_source_and_fire_with_cause` variant. ALL 7 `fire_digivolution_card_trashed` sites now route through the Tier-2 primitive: 3 uniform (39766d80) + the 4 divergent — `trash_bottom_sources`/`trash_bottom_face_down_source` (below-top source → host unchanged, computed pre-push), `attach_tamer_to_digimon` drain (`EventCause::Return` variant), `armor_purge_top` (`EventCause::Cost` variant, host = post-pop promoted top). Zero trash-source fire calls remain in the facade. Lint ratcheted 16 → 12. Gate: 3548/7 (baseline), all binaries green.
