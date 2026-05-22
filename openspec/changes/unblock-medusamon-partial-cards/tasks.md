## 1. Tier 1 — G-SECURITY-SKILL-RESUME-REFIRE (security resume bug)

- [x] 1.1 Write a failing decline-path test: a declinable `[Security]` "you may" effect is declined → security resolution reaches a terminal outcome and no `pending_selection` remains (un-ignored `p_189_security_clause_can_be_declined`)
- [x] 1.2 Add `security_skill_drained: bool` to `SecurityResolutionState` (`selection.rs`), initialized `false` in the `resolve_security_card` constructor (`combat.rs:2451`)
- [x] 1.3 Update the `SecuritySkillDrain` arm (`combat.rs:2497`): skip the `SecuritySkill` re-enqueue when the flag is set, set the flag on first drain, drain the queue every entry, advance to `BattleResolved` only when no `pending_selection` remains
- [x] 1.4 Confirm the accept-path regression test still passes — fix exposed a latent double-play bug in `st19_08` (re-fire played a 2nd card); corrected that test's count-arithmetic assertion which had false-passed on the bug
- [x] 1.5 Verify green: `combat` (206 passed), `replacements` (110 passed — `security_effects` is not a real target; security tests live in `combat`/`replacements`), `cards_behavioral -- p_189 p_206 st19_08` (58 passed, 1 unrelated ignore)

## 2. Tier 1 — G-ZONE-SELECTED-TRASH-TO-DECK-TOP (return selected trash to deck top)

- [x] 2.1 Wrote failing tests in `zone_manipulation.rs` — `return_trash_cards_to_deck_top_places_card_at_deck_top` + a multi-card order test
- [x] 2.2 Added `EffectContext::return_trash_cards_to_deck_top` (`effect_context/mod.rs`) — reverse-push so the first selected card ends on top
- [x] 2.3 Added a `destination: top | bottom` field (default `bottom`, `DeckDestination` enum) to `ReturnTrashListToDeckBottomArgs` — `step.rs`, `compile.rs`, `compiled.rs`
- [x] 2.4 Lowered the `destination` (`to_top`) param in `dsl_cards/step/zone_moves.rs` — `top` routes to the new method; also resolves a `select_trash` `TrashIndex` binding
- [x] 2.5 Re-authored LM-027 clause B as a real `kind: delay` clause; implemented all 4 formerly-`unimplemented!()` delay tests; removed the orphaned `lm_027_delay_start_of_turn_noop` raw_rust fn
- [x] 2.6 Verified green: engine `dsl` (629), `zone_manipulation` (60), `cards_behavioral -- lm_027` (22, 0 ignored), `bt24_017`+`ex5_015` (30, callers unchanged), digimon-dsl crate (all pass)

## 3. Tier 1 — G-TRASH-SELECTED-SECURITY (trash a chosen non-top security card)

- [x] 3.1 Wrote failing tests — `effect_context` `trash_security_card_trashes_chosen_non_top_card_by_handle` + the BT24-018 `trash_chosen_opponent_security` behavioral test
- [x] 3.2 Added `EffectContext::trash_security_card(player, handle)` — handle-based (not index-based as D3 first sketched), which eliminates the index-staleness risk the design flagged; mirrors `trash_bottom_security`'s replacement window + observers
- [x] 3.3 Added a `trash_selected_security` DSL verb consuming a `select_security` `CardHandle` binding — `step.rs`, `compile.rs`, `compiled.rs` + dispatch in `dsl_cards/step/play_digivolve.rs`
- [x] 3.4 Re-authored BT24-018 clause (e) — split into two `when_digivolving` clauses (trash, unsuspend) so each printed "may" keeps its optionality; un-ignored + implemented the trash test
- [x] 3.5 Verified green: engine `dsl` (629), `cards_behavioral -- bt24_018` (19, 0 ignored), `effect_context -- trash_security_card` (2), digimon-dsl crate (all pass)

## 4. Tier 2 — G-ACTIVATION-COST-TRASH-SELF (declinable `trash_self` activation cost)

- [x] 4.1 Resolved Q2 — `trash_self_as_cost` routes through `EffectContext::delete_permanent`, which fires `OnDeletion`/`WhenWouldBeDeleted` exactly like the `delete_permanent` step BT21-093's clause 3 already used; no behavior change in the trash itself, only the optionality
- [x] 4.2 Wrote tests — `bt21_093_delay_can_be_declined` (decline skips the clause, no trash), accept-path in `delay_activates`, digivolve-still-optional in `decline_keeps_trash`; mutual-exclusion enforced in `compile.rs`
- [x] 4.3 Added `trash_self: bool` to `ActivationCostArgs` (`digimon-dsl/src/step.rs`) + `CompiledActivationCostKind::TrashSelf`; extended the `compile.rs` match to a 3-tuple with exactly-one-of-three enforcement
- [x] 4.4 Wired `TrashSelf => ctx.trash_self_as_cost()` in `lower_triggered.rs`; added `EffectContext::trash_self_as_cost` mirroring `return_self_to_deck_bottom_as_cost`
- [x] 4.5 Re-authored BT21-093 clause 3 — `optional: true` + leading `activation_cost: { trash_self: true }` (declinable <Delay>, per Rules 16-16-2) instead of a mandatory `delete_permanent` body step
- [x] 4.6 Verified green: engine `dsl` (629), `cards_behavioral -- bt21_093` (17, 0 ignored), digimon-dsl crate (all pass)

## 5. Tier 2 — G-ALT-PATH-SAVE-IN-TEXT (`keyword_in_text` alt-path predicate)

- [x] 5.1 Resolved Q1 — alt-path `from:` filters are evaluated via `eval_predicate(from, rctx, Permanent(base))` (`dna_digivolve.rs`). **Found the existing `effect_text_contains` predicate already covers "<Keyword> in text"** — and `eval_predicate` already runs it against the candidate's card data
- [x] 5.2 Wrote structural tests — `bt21_072_has_two_digivolve_alt_paths_standard_and_xros_req` + `bt21_072_xros_req_alt_path_gates_on_save_in_text_or_hero_trait` (matches the AD1-012 xros_req test precedent)
- [x] 5.3 **No new predicate added** — `effect_text_contains` is the faithful primitive (the requirement is literally worded "w/<Save> in text"). Spec `dsl-card-scripting-vocabulary` requirement 2 updated to reflect this; a redundant `keyword_in_text` verb was avoided
- [x] 5.4 Re-authored BT21-072 — added the xros_req cost-3 alt-path: `from: { any_of: [{level_eq:4, effect_text_contains:"<Save>"}, {level_eq:4, trait_has:Hero}] }, ignore_requirements: true` (AD1-012 pattern)
- [x] 5.5 Verified green: `cards_behavioral -- bt21_072` (18, 0 ignored); engine `dsl` already green from Group 4 (no new vocab to retest)

## 6. Tier 3 — design spike (no engine code)

- [x] 6.1 G-ACTIVATED-DIGIVOLVE-EXECUTION: investigated — the `DIGIVOLVE` action range (`400..1000`) already encodes `(hand_index, field_index)`, the exact shape of an activated digivolve. **Finding: reuse it, no `ACTION_SPACE_SIZE` change.** Documented in the follow-up change's design.md (D1)
- [x] 6.2 G-LINK-OPTION-DUAL-PLAY-MODE: investigated — `classify_option_subtype` is first-match-wins. **Finding: reuse `PLAY_HAND` + a mode-select pending selection; `classify_option_subtype` returns a mode set. No new action ID.** Documented in design.md (D2)
- [x] 6.3 Wrote the follow-up OpenSpec change `unblock-medusamon-tier3-cards` (proposal.md + design.md) — both gaps close by reusing existing action IDs; no tensor/decoder-width change, no RL retraining

## 7. Wrap-up

- [x] 7.1 Full engine suite green — `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader` exits 0; `cards_behavioral` 2714 passed / 0 failed; `dsl` (629), `combat` (206), `replacements` (110), `zone_manipulation` (60) all green
- [x] 7.2 Archived the 5 closed gaps to `qa/resolved-gaps.md` (new "Medusamon PARTIAL-card unblock" section); struck them through + added RESOLVED status lines in `engine-gaps.md` and `dsl-vocab-gaps.md`
- [x] 7.3 The 5 cards were re-authored directly in this change (YAML + tests) and verified green per-card — re-dispatching the multi-agent `/batch-implement-cards-rust-dsl` skill on already-implemented-and-green cards would be redundant; all 5 confirmed `IMPLEMENTED`
- [x] 7.4 Updated `qa/qa-reports/validated_cards_dsl.json` (5 entries → `IMPLEMENTED`, gap_kind cleared, test counts, 2026-05-21) and appended a Tier 1+2 follow-up section to `qa/archetype-qa/dsl/medusamon.md`
