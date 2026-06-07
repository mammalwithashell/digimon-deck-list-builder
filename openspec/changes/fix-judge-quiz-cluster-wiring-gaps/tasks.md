## 1. Q18 — `<Blast Digivolve>` / `<Blast DNA Digivolve>` consult effect-immunity — DONE 2026-05-29

- [x] 1.1 Probe confirmed (read-only audit): neither `execute_blast_digivolve` (combat.rs) nor the Blast DNA field-target generator consults `permanent_is_unaffected_by_effect`; immunity machinery (`permanent_is_unaffected_by_effect`, `EffectControllerFilter::{Any,OpponentOnly,OwnOnly}`) already exists.
- [x] 1.2 Gate Blast counter-candidate collection in `combat.rs::try_enter_counter`: skip a `(hand, field)` Blast candidate when `permanent_is_unaffected_by_effect(base, base.player, EffectSourceKind::Digimon)`.
- [x] 1.3 Mirror the gate in `dna_digivolve.rs::valid_blast_dna_field_targets_for_hand_card` so an immune field Digimon isn't a Blast DNA base.
- [x] 1.4 Defensive abort at the start of `execute_blast_digivolve` (covers effect-driven blast paths that bypass candidate collection).
- [x] 1.5 Test `combat::counter_interrupt::blast_target_immune_to_own_effects_is_not_a_counter_candidate` — an unconditional `CannotBeAffected` (Any) base suppresses the otherwise-valid Blast pair → no Counter prompt, attack resolves synchronously. Existing 13 `counter_interrupt` tests stay green.

## 1b. Q3 — digivolve-target restriction modifier (engine substrate) — DONE 2026-05-29

- [x] 1b.1 Added `ModifierType::CanOnlyDigivolveInto` (carries allowed name in `ModifierPayload::Name { value }`); registered in `modifier_map.rs` (lookup + exhaustiveness + all_variants), `validator::KNOWN_MODIFIER_KEYS`, and the `payload_matches_modifier` guard.
- [x] 1b.2 Added `Game::digivolve_target_blocked_by_restriction(base_handle, card)` — blocked iff a `CanOnlyDigivolveInto` entry is present whose allowed name matches none of the card's names (ANDs multiple entries); no-op when absent.
- [x] 1b.3 Wired the consult into the central `normal_digivolve_route_for_card` (feeds the digivolve action mask + Blast counter path + hand-digivolve execution → return `None` when blocked) AND the arts-digivolve path (`game_actions.rs`).
- [x] 1b.4 Tests `dna_digivolve::tests_q3_digivolve_target_restriction::{can_only_digivolve_into_blocks_nonmatching_name, no_restriction_is_a_noop}` — pass. Full suite regression-clean.
- [~] 1b.5 DSL-install vocab (a declarative aura installing `CanOnlyDigivolveInto` with a card-specific name) — DEFERRED to EX10-020 authoring (the allowed name is card-specific; `ChangeBaseCardName` Name-payload-aura lowering is the template). Cluster G — not first wave.

## 2. Reconcile

- [x] 2.1 Moved `G-BLAST-DIGIVOLVE-IMMUNITY` to `qa/resolved-gaps.md`; added the OPEN entries `G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` (Q25) and `G-DIGIVOLVE-TARGET-RESTRICTION` (Q3) to `qa/archetype-qa/engine-gaps.md` with fix shapes.
- [x] 2.2 Updated `qa/qa-reports/judge-quiz.md` (§0c Q18 substrate closed but BLOCKED-CARD on LM-020; §0d Q25/Q3 scoped-and-deferred).
- [x] 2.3 Full `cargo test --features dsl-yaml-loader --no-fail-fast` regression-clean (combined with `fix-judge-quiz-engine-gaps` Gap 1) — real test-failure set identical to baseline.

## 3. Out of scope (deferred — see proposal Non-Goals)

- [~] Q25 `G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` and Q3 `G-DIGIVOLVE-TARGET-RESTRICTION` — scoped in `engine-gaps.md`; closed at EX3-014 / EX10-020 authoring time (DCGO-verified). NOT implemented here.
- [~] Q21/Q23 `G-ON-TRASH-OBSERVER-SYNCHRONOUS` — split to its own follow-up change.
