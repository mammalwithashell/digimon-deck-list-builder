# Tasks — fix DSL substrate rot and bugs

> **Scope note (discovered during apply):** the hard-error loader guard surfaced
> that `build.rs` compiles `_examples/` into the PRODUCTION pack, and 4 deck-legal
> cards (BT11-042, BT7-107, EX11-027, EX6-072) live ONLY there with raw_rust fns
> registered only in the test harness (`tests/dsl/phase0_exit.rs`) — silent no-ops
> in production, same class as BT13-007. Per user decision, those 4 are being
> MIGRATED to pure DSL (new §2.5) so the guard can ship as a hard error. Until
> then the guard runs in WARN mode (no regression).

## 1. Loader integrity guard
- [x] 1.1 Shipped in WARN mode (`cards.rs`): the engine load path validates raw_rust refs via `from_embedded_with_raw_registry`, logs a `WARNING` naming any unregistered refs, and falls back to a full load (no regression). Verified the pack loads and warns about EXACTLY the 2 remaining refs (`ex11_027_link_requirements`, `ex11_027_optional_link_maquinamon`). **Promote to hard-error (panic) once EX11-027 migrates** — that's the only remaining offender (BT13-007/EX8-070/BT7-107/BT11-042/EX6-072 are all cleared).
- [ ] 1.2 Engine test: a card referencing an unregistered `raw_rust` formula/step fails load with a message naming the card + reference. (The dsl-crate `registry.rs` tests already cover the validator; add an engine-level test once the guard is promoted.)
- [x] 1.3 Confirmed the guard catches the real case: it listed 8 unregistered refs across 5 cards (BT13-007 + the 4 `_examples` cards) before fixes.

## 2. Fix BT13-007 (silent no-op cost reduction) — DONE
- [x] 2.1 Replaced `amount_fn: { raw_rust: … }` with `amount_fn: { base: 4, per: material_count, delta: 1 }` (verified `per: material_count` resolves against the reducer source = King Drasil).
- [x] 2.2 Un-ignored + implemented `bt13_007_cost_reduction_counts_sources_under_king_drasil` (embedded card, asserts 4 + N scaling) + a behavioral-parity test in `cost_hooks` (`drasil_production_structured_formula_matches_raw_rust`). All 3 bt13_007 + 4 drasil tests pass.
- [x] 2.3 Kept `bt13_007_royal_knight_cost_reduction` (test-harness only) — still used by the `_examples` roundtrip + cost_hooks YAML test; production no longer references it.

## 2.5 Migrate the 4 `_examples` deck-legal cards to pure DSL (precondition for hard-error guard)
- [x] 2.5.0 Researched/drafted faithful pure-DSL migrations (parallel workflow, DCGO-verified, adversarially reviewed for faithfulness + vocab-existence).
- [x] 2.5.1 BT11-042 Angewomon — migrated `when_digivolving` (search_own_security_stack + on_select recover + shuffle_security, with clause `optional: true` + `outer_prompt: true` for DCGO's two-level optionality) + `on_ally_played` (gain_memory gated on your_turn/OPT/name). Pure DSL.
- [x] 2.5.2 BT7-107 Calling From the Darkness — migrated `on_security` → `add_this_option_to_hand` (review verdict: ready). Pure DSL.
- [BLOCKED] 2.5.3 EX11-027 Maquinamon — link clauses need 4 NEW substrate primitives (filed: G-DSL-LINK-RELINK-STANDING-PERMANENT, G-DSL-LINK-HETEROGENEOUS-CHOICE, G-DSL-LINK-HOST-FILTER, G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE). Cannot go pure-DSL now; left on test-only raw_rust. **This blocks the hard-error guard.** → user decision pending.
- [x] 2.5.4 EX6-072 Mega Digimon Assembly! — migrated to the EX3-008/BT17-095 precedent: `effect_initiated_dna_digivolve_hand_partner { cost: printed }` + `on_security` `add_this_option_to_hand` + top-level `use_requirement` color bypass. Pure DSL. Two PRE-EXISTING shared DNA engine gaps filed (G-ENGINE-DNA-PRINTED-COST, G-ENGINE-DNA-RECIPE-ENFORCEMENT) — not specific to this card.
- [x] 2.5.5 Verified dependent tests stay green: `dsl` target (766 pass — phase0_exit/roundtrip/embedded_registry incl. cross-check vs cards.json) + `judge_quiz` (42 pass — Q19/Q20 drive real BT7-107). NO test changes needed: phase0/roundtrip `StubRegistry` extra entries are harmless; `_examples` stays packed so embedded lookups + judge_quiz still resolve.
- [x] 2.5.6 Substrate gaps filed precisely (4 link in `qa/dsl-vocab-gaps.md`, 2 DNA in `docs/RUST_ENGINE_GAPS.md`); no approximation shipped (EX11-027 left on raw_rust rather than stubbed; EX6-072 matches accepted precedent).

## 3. Fix EX8-070 (no-approximations tie auto-pick) — DONE (cleanup pending)
- [x] 3.1 Replaced the `[Security]` raw_rust step with `select_opponent_permanent { filter: { kind: digimon }, selector: lowest_play_cost }` + `delete_permanent`.
- [x] 3.2 Reworked `ex8_070_security_deletes_lowest_cost_digimon` (asserts the correct target dies, not auto-index) + added `ex8_070_security_tie_exposes_player_choice` (tie surfaces a `pending_selection`). All 22 ex8_070 tests pass.
- [x] 3.3 Remove `ex8_070_delete_lowest_cost_digimon` from `raw_rust/mod.rs` `build_registry` (done together with the §4.3 BT24-062 removal — `build_registry` now registers neither `ex8_070_*` nor `bt24_062_*`; verified `mod.rs:209-213`).

## 4. Retire BT24-062 raw_rust — DONE (substrate widened, not routed around)
- [x] 4.1 Added the `is_source_permanent` predicate (DSL spec + compiled + compile + engine eval) — a reusable `target: self` filter (audit theme 10), and taught the flood_gate dispatch to expand `scope: both` into face_up + inherited emissions. No board scan / aux predicate needed: in face_up the carrier resolves to self, in inherited to the host — both top-level permanents the gate's scan covers.
- [x] 4.2 Migrated BT24-062's `CannotSwitchAttackTarget` lock to `kind: flood_gate { scope: both, target: { is_source_permanent: true }, active_when: { your_turn: true } }`. All 4 BT24-062 tests pass (compiles to 1 clause; lock on the carrier; inactive on opponent's turn; doesn't affect other attackers). Verified only BT24-062 uses `scope: both`, so no other card's behavior changes.
- [x] 4.3 Removed `bt24_062_attack_target_lock` (and §3.3 `ex8_070_delete_lowest_cost_digimon`) from `mod.rs` + `build_registry`. Remaining registered card fns: `bt24_012`, `lm_027`, `bt21_093`, `bt13_040` (+ EX11-027's two test-only fns, deferred to the link change).
- [x] (4.x) Regenerated the vocab doc (146 predicates now; `is_source_permanent` listed) + drift gate green; dsl suite (766) + ex8_070 (22) green.

## 5. Retire dead vocabulary — AUDITED → retire none (roadmap over-estimated)
- [x] 5.1 Full-repo reference audit (cards + tests + src, not just card-corpus): of the 9 candidates, **8 are referenced by behavioral/unit TESTS** — `bounce_self` (4), `place_self_at_security` (4), `source_is_tamer` (2), `mark_security_face_up`/`lose_memory_fn`/`add_digixros_cost_delta`/`add_digixros_wildcard_to_pending_transaction`/`event_target_same_level_as_previous` (1 each). These are **wired, tested completeness API** (the "keep" category in the original §5.2), not dead weight. Only `form_is` is 0 cards / 0 tests.
- [x] 5.2 Decision: **retire none.** Removing the 8 tested verbs would delete working, tested functionality. `form_is` is unwired but a legitimate card-attribute predicate (kept for parity with `attribute_is`, which IS used). The roadmap's "~10 dead verbs" conflated card-usage (the generated `unused` tag) with true deadness; a reference audit corrects it. (`lose_memory_fn` is separately removed by `unify-dsl-scalar-and-comparators` as part of the FormulaSpec scalar unification — the right place for it.)
- [x] 5.3 No vocab removed → no regen needed; drift gate already green.

## 6. Documentation + gap-tracker reconciliation
- [x] 6.1 Added a `link_card_to_self` deprecation note (superseded by `link_cards`; usage growing) to the §5 banner of `docs/RUST_DSL_AGENT_GUIDE.md` (outside the generated block, so the drift gate is unaffected). Migration + deletion tracked in `collapse-dsl-step-idioms`.
- [~] 6.2 Stale-header scrub: the §7 check surfaces **17 verified-stale citations** across ~10 cards (AD1-001, BT15-020, BT15-101, BT20-102, BT21-072, BT22-005/013/026, BT23-018, EX1-021, LM-027, P-123, P-182, ST20-11) — each cites a gap that IS resolved (verified all 10 distinct gap IDs against the open trackers). NOT scrubbed here: most are YAML comments *describing* `#[ignore = "BLOCKED: G-X"]` test states, so an accurate correction needs per-card verification (is the clause now authored, or only authorable?) + the paired test `.rs` ignore reasons — that's card-by-card work, not a blanket comment edit. Surfaced by the warn-first gate (§7) for incremental cleanup → hard-fail promotion. [remaining — per-card follow-up]
- [x] 6.3 Closed the stale `G-DSL-SELECT-OPP-SOURCES-DYNAMIC-CROSS-PERMANENT` entry (`dsl-vocab-gaps.md`): verified `SelectOpponentSourcesArgs` now has `max: CountBound` (FormulaSpec) + `clamp_to_available` + cross-permanent (omit `target`); struck through + flagged BT25-103 for re-assessment (its BLOCKED verdict is stale).
- [x] 6.4 De-duplicated `OnDiscardHand` (was DOUBLE-logged, not triple — the dsl-vocab-gaps.md "hit" was a false `discard…hand` span match). Consolidated to one canonical home `docs/RUST_ENGINE_GAPS.md` (`[G-ENGINE-ON-DISCARD-HAND]`, engine-primitive gaps live there per the filing rules) — merged BT25-084 in with BT25-080/029; replaced the `qa/archetype-qa/engine-gaps.md` duplicate with a cross-ref stub + the BT25-084 PARTIAL verdict.

## 7. CI guard against header rot — DONE (warn-first)
- [x] 7.1 Added `code/tools/check_resolved_gap_citations.py` + `.github/workflows/resolved-gap-citation-drift.yml`. Conservative heuristic: flags a card-header line only when it cites a RESOLVED gap id (a `## … [G-X]` header in `qa/resolved-gaps.md`) AND carries a staleness marker (pending/blocked/until-closes/workaround/raw_rust/…) AND no resolved-keyword in the ±3-line context. Verified: 63 resolved ids, 207 legit citations NOT flagged, 2 false positives (BT22-099, P-182) correctly excluded by the context window, **17 genuinely-stale flagged**.
- [x] 7.2 Shipped `continue-on-error: true` (warn-first), matching the `action-space-codegen-drift` phase-1 precedent + this change's loader-guard warn-mode. PROMOTE to required once §6.2's 17 are scrubbed.

## 8. Verification
- [x] 8.1 Full cards_behavioral suite (5248 passed / 0 failed / 64 ignored) + targeted BT13-007 (3), EX8-070 (22), BT24-062 (4) + dsl (766) + judge_quiz (42) — all green.
- [x] 8.2 Production set-dir cards reference zero raw_rust (BT13-007/EX8-070/BT24-062 migrated); `build_registry` registers no `ex8_070`/`bt24_062` fn. Remaining raw_rust is `_examples/` worked-examples + the blocked EX11-027 (folded into `collapse-dsl-step-idioms`).
- [x] 8.3 Vocab-doc drift gate green; resolved-gap-citation CI shipped (warn-first).
