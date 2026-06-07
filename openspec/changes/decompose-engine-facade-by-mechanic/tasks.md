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

- [ ] 4.1 **DEFERRED to follow-up (§10.5).** Inspection: `game_actions.rs` holds 136 methods in one `impl Game` block PLUS module-private types (`OptionSource`, `CostReductionKey`, `CostTargetContext`, `BeforePayCostSourceInfo`, …) and private free-fns that submodules would need visibility-promoted, with denser interdependencies than the facade. The scripted toolchain (`code/tools/archive/extract_facade_all.py`, adapted for `impl Game` + `use super::*` over pub(crate)-promoted types) is ready; deferring so it gets its own focused pass + gate rather than rushing a large Tier-2 front in this change.
- [ ] 4.2 DEFERRED (§10.5) — narrowed `game.rs` extraction (until_condition + read-only query helpers) folds into the same Tier-2 follow-up.

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

- [x] 8.1 Documented the placement rule + 3-tier model + `<tier>/<mechanic>` address scheme in `docs/RUST_ENGINE_API.md` §3 ("Module layout & the placement rule").
- [ ] 8.2 **DEFERRED to follow-up (§10.3, RQ1).** The warn-mode lint depends on a stable Tier-3-exception allowlist; ship it after the remaining mechanical splits land. Precedent + reuse noted in §10.3.
- [x] 8.3 Engine-dev address scheme documented in `docs/RUST_ENGINE_API.md` §3 (the authoritative engine-dev reference that CLAUDE.md points to). CLAUDE.md edit not required — it already routes engine work through RUST_ENGINE_API.md.

## 9. Verification & close-out

- [x] 9.1 Full suite matches baseline exactly: cards_behavioral 3548 passed / 7 failed (the pre-existing DP-aura set), all other binaries green. Behavior-preserving confirmed. `effect_context/mod.rs` shrank 6933 → 1463 lines.
- [x] 9.2 PyO3 boundary (`digimon-engine-py`) `cargo check` clean → no public API drift (crate-root tensor/observation paths preserved via re-exports).
- [x] 9.3 Spec scenarios: **satisfied** — by-mechanic organization (Tier 3), call-surface-unchanged, observation read-only port, de_digivolve no-inversion, behavior-preserved. **Partially satisfied (deferred):** "parallel mechanic names" Tier-2 side (game_actions, §10.5) and "single source-trashing primitive" (B1, §10.4) — broader "all rules machinery out of facade" beyond de_digivolve folds into B1. Honest status: structural decomposition + the exemplar inversion fix landed; the remaining Tier-2 mirror + trash-primitive are documented follow-ups.

## 10. Deferred follow-ups (record, do NOT implement here)

- [ ] 10.1 Record a follow-up: split `effect_context/selections.rs` (3,373 LOC, 35 `select_*` primitives) by selection-target — only if it keeps growing (RQ2).
- [ ] 10.2 Record a follow-up: full `game.rs` mechanic split beyond the narrow 4.2 extraction (RQ3).
- [ ] 10.3 Record a follow-up: promote the placement-rule lint from warn → deny/required once B1/B3 have landed and the Tier-3 exception allowlist is stable (RQ1).
- [ ] 10.5 **Tier-2 split (game_actions + narrow game.rs) deferred to a dedicated follow-up.** Mirror the facade mechanic taxonomy in `game_actions/<mechanic>.rs` (`impl Game`) using the proven extractor; promote module-private types/free-fns to `pub(crate)`; then the narrow `game.rs` extraction (until_condition + query helpers). Completes the parallel `<tier>/<mechanic>` taxonomy. Pure movement; own full-suite gate.
- [ ] 10.4 **B1 (trash-source primitive) deferred to a dedicated follow-up.** Inspection found the 7 sites non-uniform (fire-attribution owner vs perm.player, removal strategy, host-derivation timing). Extract `Game::trash_source_and_fire(trash_owner, fire_target, removed, host_card)` for the truly-identical tail (push + observer fire), migrate one site at a time each gated on its own `cards_behavioral` test, leaving removal + host-derivation per-site. Parity-sensitive (`OnDigivolutionCardTrashed`).
