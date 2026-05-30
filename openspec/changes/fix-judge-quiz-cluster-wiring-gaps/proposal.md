## Why

A pre-authoring substrate audit of the judge-quiz clusters (read-only probes against the Rust engine) surfaced three NEW engine-wiring gaps beyond the three already tracked by `fix-judge-quiz-engine-gaps`. One is a clean, card-agnostic wiring fix; the other two are real but card-shaped (their exact form depends on card text that the API ingest gets wrong, so they must be DCGO-verified at authoring time).

- **`G-BLAST-DIGIVOLVE-IMMUNITY` (Q18).** Neither `execute_blast_digivolve` (combat.rs) nor the Blast DNA field-target generator (`dna_digivolve.rs`) consulted the effect-immunity machinery. A Digimon immune to ALL Digimon effects including its own (Quantumon LM-020 — `CannotBeAffected` with `EffectControllerFilter::Any`) could still `<Blast Digivolve>` — but Blast Digivolve is itself a Digimon effect. This is a clean substrate wiring fix (reuse the existing `permanent_is_unaffected_by_effect`).
- **`G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` (Q25).** `take_digixros_material_origin` silently removes a battle-area Digimon consumed as a DigiXros material with NO `OnLeaveField` / `WhenWouldLeaveBattleArea` dispatch. The judge rule: a DigiXros departure counts as "leaving the battle area" (≠ battle).
- **`G-DIGIVOLVE-TARGET-RESTRICTION` (Q3).** No `ModifierType` expresses "can only digivolve INTO [X]" (only `CannotDigivolve` for the source itself). EX10-020's `[All Turns] this Digimon can only digivolve into [Apocalymon]` has no primitive. (The Q3-relevant breeding-area *inactivity* is already correct — continuous/aura effects enumerate `battle_area` sources only.)

## What Changes

- **Q18 — gate Blast Digivolve / Blast DNA Digivolve on effect-immunity (IMPLEMENTED).** Filter Blast counter-candidate collection (`try_enter_counter`) and the Blast DNA field-target generator on `permanent_is_unaffected_by_effect(base, base.player, EffectSourceKind::Digimon)`, with a defensive abort in `execute_blast_digivolve`. So a Digimon immune to its own controller's Digimon effects is never offered as a Blast base. Pinned by `counter_interrupt::blast_target_immune_to_own_effects_is_not_a_counter_candidate`.
- **Q3 — digivolve-target restriction modifier (ENGINE SUBSTRATE IMPLEMENTED).** Added `ModifierType::CanOnlyDigivolveInto` (allowed name in `ModifierPayload::Name`) + `Game::digivolve_target_blocked_by_restriction`, consulted in the central `normal_digivolve_route_for_card` (mask + Blast + hand-digivolve) and the arts path. A base carrying the restriction offers no digivolve route into a non-matching card; no-op when absent. Pinned by `tests_q3_digivolve_target_restriction::*`. The DSL-install vocab (declarative aura with a card-specific name) is deferred to EX10-020 authoring.

## Capabilities

### New Capabilities
- `judge-quiz-cluster-wiring`: (a) `<Blast Digivolve>` / `<Blast DNA Digivolve>` consult the effect-immunity machinery before digivolving the base Digimon, so a Digimon unaffected by its own controller's Digimon effects cannot blast-digivolve; (b) a digivolve-target restriction (`CanOnlyDigivolveInto`) so a base Digimon may digivolve only into a card whose name matches the allowed name, consulted at every digivolve route.

## Impact

- **Engine (Rust):** `code/digimon-engine/src/combat.rs` (`try_enter_counter` candidate gate + `execute_blast_digivolve` abort), `code/digimon-engine/src/dna_digivolve.rs` (`valid_blast_dna_field_targets_for_hand_card` filter).
- **Tests:** `code/digimon-engine/tests/combat/counter_interrupt.rs` (new immunity test; existing 13 stay green).
- **Trackers:** `G-BLAST-DIGIVOLVE-IMMUNITY` → `qa/resolved-gaps.md`; `qa/qa-reports/judge-quiz.md` notes Q18 substrate closed (still BLOCKED-CARD on LM-020).
- **No RL contract change** (no action-space/tensor change — gating only removes illegal Blast candidates from the existing counter mask).

## Non-Goals

- **`G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` (Q25)** — confirmed via probe + DCGO-verified design (in `qa/archetype-qa/engine-gaps.md`), NOT implemented here. DCGO shows BT17-095 keys off `when_would_leave_battle_area` with `none_of: [replacement_cause: battle]` and REDIRECTS WarGreymon into a DNA-digivolve mid-DigiXros — so the fix is DigiXros-transaction replacement-window surgery with per-material cancel/redirect/substitute handling, only safely validated with EX3-014 + BT17-095 as the integration oracle. Closed as the substrate step of EX3-014 authoring.
- **`G-DIGIVOLVE-TARGET-RESTRICTION` (Q3) DSL-install vocab** — the engine substrate IS implemented here (see What Changes), but the DSL step to INSTALL the restriction with a card-specific allowed name (a declarative aura) is deferred to EX10-020 authoring (cluster G), since the name is card-specific.
- **`G-ON-TRASH-OBSERVER-SYNCHRONOUS` (Q21/Q23)** — split out of `fix-judge-quiz-engine-gaps` (its spike showed the synchronous drain is load-bearing for EX10-036's sibling-clause pickup); its own follow-up change. Out of scope here.
- Authoring the BLOCKED-CARD scenarios (LM-020 for Q18, etc.) — that is `/batch-implement-cards-rust-dsl` work.
