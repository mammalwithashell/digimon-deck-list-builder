# DCGO PvP replay hunt, round 2 (2026-11-19)

Integration report for the 17 round-2 lanes merged onto `claude/replay-hunt-integrated` (round-1 tip 1154a6eee) as `claude/replay-hunt-round2-integrated`.

## How the pass was run

Round 2 re-diagnosed the games that still stopped after round 1, with focus on the EX13 deck pool (core pool cards first, then the union pool, then opponent-seat cards
  that block a replay). Each of the 17 lanes ran in its own git worktree under E:/wt/ with its own cargo target dir under E:/cargo-target/, cut from the round-1 tip, with
  a fixer and two reviewers per lane (a verification review, then a repair pass for blocking findings). Several lanes resumed an earlier interrupted pass and re-verified
  its commit instead of redoing it. The lanes were merged here one merge commit each, in a fixed order; generated artifacts were regenerated after the merges rather than
  merged by hand. No raw or converted recordings and no deck or hand contents are committed: only card ids, rule citations (CR 4.3) and recording keys.

## Lane table

| lane | status | tests added |
|---|---|---|
| frame-model-unlogged | fixed | test_dcgo_frame_model.py (14) |
| ci-invariant-fuzz | fixed | bt25_035 digest test |
| refire-source-identity | fixed | bt11_112 test |
| replay-int-and-order-folds | fixed | replay_dcgo_int_and_order_folds (14) |
| replay-bool-answer-maps | fixed | replay_dcgo_bool_answer_maps (10) |
| replay-multipleskills-index | fixed | replay_dcgo_filtered_order_entries (7) |
| homeros-effect-pick | fixed | bt24_102 (9), refire_duplicate_pick (7), dsl refire_target_eligibility (5), py (3) |
| dedigivolve-count | partial | replay_dcgo_h2_dedigivolve_count (8), ex13_066, dsl parse |
| trigger-order-bundle | partial | bt20_014, ex12_021, bt25_059, bt26_103 |
| engine-prompt-shape-parity | partial | ex13_053, ex12_056, ex12_003, dsl group7 |
| trait-substring-audit | partial | bt25_010, bt24_043, ex13_042, bt11_089 and more |
| digitama-inherited-effects | partial | 37 tests in 5 modules |
| unimpl-digimon-misc | partial | per-card modules (6) |
| memory-tamer-start-main | partial | bt22_093, bt17_080, bt22_056 |
| alt-digivolve-cost-cards | partial | 6 per-card modules |
| reveal-bucket-cards | fixed | per-card modules (12) |
| ex8-nso-opponent | partial | ex8 modules, dsl phase2f1 |

- **frame-model-unlogged** (fixed). Fixed: Converter frame model explains an Option placed by its own effect (no recorded play row), so the seat is no longer killed.
  Review: no review requested.
- **ci-invariant-fuzz** (fixed). Fixed: Verification digest hashes the multi-card face-down payment pick state; invariant_fuzz no longer sees a no-op step (pre-existing
  on main). Review: approved; minor: field also applied to the archived V3 digest.
- **refire-source-identity** (fixed). Fixed: Refired effects keep the borrowed-from permanent identity ("this Digimon" in BT12-029 refired by BT11-112). Review: no review
  requested.
- **replay-int-and-order-folds** (fixed). Fixed: Replay folds: Chirinmon EX13-032 int menu at the outer gate, zone-loop trigger entry, placement-order row. Review:
  approved; minor notes on the fold evidence checks.
- **replay-bool-answer-maps** (fixed). Fixed: Four generic_bool maps: stamped yes then MultipleSkills row, Ascension yes/no, lone cost-reducer gate, BT23-013 mode bools.
  Review: approved; minor: doc cites 16-1, should be 16-43-3.
- **replay-multipleskills-index** (fixed). Fixed: Replay: card-less/same-card MultipleSkills index against filtered TriggerOrder entries (EX9-032, EX12-044). Review:
  BLOCKING: a cancel row was accepted as proof in the lone-entry path; fixed, plus one further regression.
- **homeros-effect-pick** (fixed). Fixed: Homeros BT24-102 duplicate-candidate effect pick folded via Game::last_collapsed_refire; has_refireable_effect leaf; BT24-081
  trait data fix. Review: approved; minor: refire record lingers if no pick row comes.
- **dedigivolve-count** (partial). Fixed: EX13-066 De-Digivolve is one declared count (de_digivolve.declared); replay answers the count rows. Review: approved; minor:
  unit-count budget is an upper bound.
- **trigger-order-bundle** (partial). Fixed: Mandatory trigger with no target is not a TriggerOrder entry (Effect::activation_guard, BT20-014); EX12-021 processing
  condition stays orderable. Review: BLOCKING: stale dsl_impact_index; regenerated (and lowering_coverage).
- **engine-prompt-shape-parity** (partial). Fixed: EX13-053 one capped select_count (returned_card_count formula); DigiXros no empty trailing prompt; DNA replacement
  subject prompt. Review: BLOCKING: stale generated artifacts after the EX13-053 rewrite; regenerated.
- **trait-substring-audit** (partial). Fixed: "in any of its traits" lowered to trait_contains (BT25-010 and family); "other than [Sea Animal]" shape fixed in 7 cards.
  Review: BLOCKING: stale lowering_coverage/admissibility; regenerated.
- **digitama-inherited-effects** (partial). Fixed: Five JSON-only Digi-Egg inherited effects authored (BT9-006, BT12-006, P-247, EX8-006, EX11-001); rules_text_contains
  reads the deletion snapshot. Review: approved.
- **unimpl-digimon-misc** (partial). Fixed: Six Digimon authored (BT17-012, EX11-018, EX10-009, BT24-073, BT19-024, EX11-013); play_to_breeding_from_trash step and
  predicate. Review: BLOCKING: stale verb map/guide/index; repaired.
- **memory-tamer-start-main** (partial). Fixed: BT22-093 Ami Aiba, BT17-080 Takato Matsuki (and BT22-056) authored; 4 DSL leaves (same-level source, stack count,
  is_source_card, card_in_zone). Review: BLOCKING: stale generated artifacts; regenerated.
- **alt-digivolve-cost-cards** (partial). Fixed: Six stub-only alt-route cards authored (EX5-018, EX10-042, BT21-069, BT21-080, BT24-065, BT25-074); highest_play_cost
  aggregate. Review: approved.
- **reveal-bucket-cards** (fixed). Fixed: Eleven stub-only cards authored (BT11-046, P-230/232/227, P-181 and more); {Security} cost reducers; event_batch_card_trait_has.
  Review: BLOCKING: PACK_FORMAT_VERSION not bumped; bumped 0.35.0 to 0.36.0.
- **ex8-nso-opponent** (partial). Fixed: Opponent NSo cards authored (EX8-071/033/060/062/064/006/032/008/057, EX11-059, BT5-107, P-174); DNA bind_as. Review: approved.

Status is the lane's own report (partial = the targeted divergence is fixed but the game now stops at a later, different defect, listed under Residuals). 'approved' =
  both reviewers approved, with minor notes only. Tests are synthetic-board DebugRunner or replay-binary tests; none embed recording data.

## Integration conflicts and how they were resolved

- replay-bool-answer-maps vs the int/order folds: both added a helper in `runners/replay.rs` at the same spot (`outer_gate_int_menu` and `is_lone_cost_reduction_gate`);
  both kept.
- trigger-order-bundle vs replay-multipleskills-index: both added a method in `effect_queue.rs` at the same spot (`parked_trigger_order_filtered_siblings` and
  `queued_effect_activation_guard_fails`); both kept, the guard stays in the non-firing filter.
- ex8-nso-opponent vs digitama-inherited-effects: both authored EX8-006 DemiMeramon. Kept the digitama YAML and `ex8_006.rs` (it carries DCGO's always-shown yes/no,
  `outer_prompt: true`, EX8_006.cs isOptional TRUE, a hand >= 1 condition, and `form`).
  The nso lane's test file is kept as `ex8_006_nso.rs`, each test accepting that outer gate first; the duplicate `mod ex8_006;` that the textual merge left in
  `ex8/mod.rs` was removed. `ex11/mod.rs` and `p/mod.rs` keep both lanes' entries.
- Generated artifacts (card_admissibility, lowering_coverage, RUST_DSL_AGENT_GUIDE) conflicted for trait-substring-audit, unimpl-digimon-misc and memory-tamer-start-main:
  took the base side and regenerated at the end.
- Semantic conflict found by `cargo check --tests`: the dedigivolve lane added `declared` to `CompiledStep::DeDigivolve`; two literals in
  `tests/dsl/phase2c_permanent_mutations.rs` (another lane) needed `declared: false`.
- `PACK_FORMAT_VERSION`: only the reveal-bucket lane bumped it (0.35.0 to 0.36.0), which is one above round 1; no further bump (several lanes changed compiled structs,
  all covered by that one step).

## Homeros BT24-102 x [Olympos XII] Digimon

Homeros (BT24-102): at end of your turn, suspend it, pick one of your [Olympos XII] Digimon with a refireable [On Play]/[When Digivolving] effect, and activate that
  effect with Homeros as the source. DCGO builds one effect object per timing tag of a printed [On Play] [When Digivolving] effect
  (CardEffectFactory.ActivateClassesForSharedEffects), so its 'Select 1 effect to activate' list holds duplicates that the engine does not park (one collapsed clause, CR
  4.3 15-15-7-2). The lane records the collapse in `Game::last_collapsed_refire` (replay evidence only) and folds DCGO's duplicate pick row in the replay layer only when
  that record proves it.

Shapes tested. The lane's own shape labels were not among the inputs I integrated from, so these are the shapes the tests actually exercise: S = DCGO lists exactly one
  candidate (engine runs it, nothing collapsed, no prompt); C2 = one printed [On Play][When Digivolving] effect split into 2 DCGO objects (engine: one clause, records
  dcgo_entries 2, no prompt); OC = a real multi-effect choice (engine keeps its own prompt). The per-card check is
  `bt24_102_refire_matches_dcgo_candidate_shape_for_every_olympos_xii_digimon` (real BT24-102 YAML plus real card YAML, End-of-Your-Turn trigger, suspend cost, the pick
  offers exactly that Digimon, carrier and grantor checked).

| card | name | DCGO shape | result (lane-run; not re-run after merge) |
|---|---|---|---|
| BT10-042 | Venusmon | S ([WD] only) | pass |
| BT24-030 | Neptunemon | C2 | pass |
| BT24-040 | Venusmon | C2 | pass |
| BT24-041 | Minervamon | C2 ([On Deletion] not offered) | pass |
| BT24-051 | Merukimon | OC: DCGO lists 3, engine 2 | pass for the engine shape; replay card pick is ambiguous and stops honestly (open) |
| BT24-101 | Jupitermon | C2 | pass |
| BT25-018 | Apollomon | C2 | pass |
| BT25-020 | Marsmon | C2 ([WA] not offered) | pass |
| BT25-028 | Dianamon | C2 ([All Turns] not offered) | pass |
| BT25-044 | Junomon | C2 | pass |
| BT25-059 | Ceresmon | C2 | pass |
| BT25-075 | Vulcanusmon | C2 | pass |
| BT25-077 | Bacchusmon | C2 | pass |
| BT25-103 | GraceNovamon | S ([WD][WA]; only [WD] qualifies) | pass |
| BT26-032 | Ceresmon (DUAL) | S | pass |
| BT26-033 | Jupitermon (DUAL) | S | pass |
| BT26-080 | Bacchusmon (DUAL) | S | pass |
| BT26-081 | Mervamon | C2 | pass |
| BT26-083 | Junomon: Hysteric Mode | C2 ([On Deletion] not offered) | pass |
| BT26-103 | Jupitermon: Wrath Mode | S ([WD][Counter]) | pass |
| P-207 | Minervamon | C2 | pass |
| BT24-102 | Homeros itself | grantor | gate offered only with an eligible target; aura installs on play and on move from breeding (9 tests) |
| BT24-081 | Titamon + SkullBaluchimon | not Olympos XII | data fix: wrong [Olympos XII] trait removed (printed Shaman/Titan/TS); not offered |

No YAML, so Homeros cannot offer them (none in the EX13 core or union pool): BT11-017 Marsmon S; BT22-077 Dianamon S; EX5-014 Apollomon S; EX5-025 Dianamon S; EX5-042
  Merukimon C2; P-191 Apollomon C2; P-208 Merukimon S; ST10-13 Junomon S. BT10-083, BT3-056, BT5-030 and BT8-018 have no [On Play]/[When Digivolving] effect and are
  correctly not offered.

Other Homeros tests: `bt24_102_pick_offers_only_olympos_xii_digimon_with_a_refireable_effect` (attack-only, trait-less and plain Digimon not offered; none eligible opens
  no prompt, cost still paid); `bt24_102_refires_minervamons_single_effect_with_homeros_as_source` (optional pick stays optional);
  `bt24_102_end_of_turn_refire_surfaces_target_then_effect_choice` (real two-effect choice); `bt24_102_refire_combined_on_play_when_digivolving_is_one_choice`;
  `replay_dcgo_refire_duplicate_pick.rs` (7: consumed at either list position, honest refusals, real two-effect choice not swallowed); 5 DSL construct tests for
  `has_refireable_effect`.

Games: two recordings pass their Homeros rows (steps 12 and 41 of one, step 41 of another); each then stops at a different defect (an optional triggered-effect yes/no
  count row; an EX5-061 digivolve with a Lv4 requirement onto a Lv5). Open: BT24-051 index map; `has_refireable_effect` ignores the clause activation condition (no pool
  card found where it differs).

## Residuals (per lane, as reported)

### homeros-effect-pick

(1) BT24-051 Merukimon (union pool): DCGO lists 3 candidate objects under Homeros ([On Play] and [When Digivolving] of the shared effect, plus the [When Digivolving] of
  the separate [WD][WA] effect), the engine has a 2-way choice (shared clause vs [WD][WA]) with the same source card. The replay card pick resolves to 'ambiguous among
  effect choices' and stops honestly; mapping DCGO position 0/1 to the shared clause and 2 to [WD][WA] is possible via the new `indexes` field but needs a recording of
  that case to verify DCGO's list order, so it was not built. (2) has_refireable_effect checks refireable timing + once-per-turn slot but NOT the effect's activation
  condition; DCGO's CanBeEffectCandidate uses CanUse (CanTrigger and CanActivate). For the pool cards read (BT10-042, BT24-030/040/041/101, P-207 and the shared-effect
  cards) DCGO's activate gate is only 'on battle area', so offers match; a Digimon whose YAML clause carries a real activation condition could be offered by the engine
  where DCGO would not offer it (not found in the pool). (3) 12 Olympos XII Digimon have no YAML, so Homeros cannot offer them (none are in the EX13 core or union pool):
  [OP]/[WD] ones are BT11-017 Marsmon ([WD] Blitz, single), BT22-077 Dianamon ([WD], single), EX5-014 Apollomon ([WD] Blitz, single), EX5-025 Dianamon ([WD][WA][OPT],
  single), EX5-042 Merukimon ([OP][WD], collapsed 2), P-191 Apollomon ([OP][WD], collapsed 2), P-208 Merukimon ([WD][OD], single), ST10-13 Junomon ([WD], single); no
  [OP]/[WD] effect, so correctly not offered: BT10-083 Minervamon, BT3-056 Ceresmon, BT5-030 Neptunemon, BT8-018 Marsmon. (4) The five related games are blocked by
  unimplemented cards (BT11-046, P-230, P-232, EX10-042) or the separate BT25-063 count-row gap; g6 step 44 and g3 step 45 are separate defects. (5) dcgo-replay dev build
  took 87 min under machine contention; full-suite runs (lib, dsl, other replay binaries) were not executed here.

### ci-invariant-fuzz

Three open items. (1) Confirm the new bt25_035 regression test is green once the cards_behavioral build finishes: cargo test --manifest-path code/digimon-
  engine/Cargo.toml --test cards_behavioral -- bt25_035. (2) Check no golden verification-digest fixture covers a TrashBottomFaceDownSourcesPick pick state; the hash is
  skipped for LegacyV2 only. (3) Optionally run seed 20260708001 on origin/main (86652b985) in E:/wt/h2-fuzz-base to turn "pre-existing on main" from a code-reading
  conclusion into a proof. No other defects found. Stale background cargo/rustc processes from my runs may still hold the lock in E:\cargo-target\h2-ci-invariant-fuzz.

### replay-int-and-order-folds

None of the three games replays to the end yet, but each now gets past the defect in this cluster and stops at a different defect.
1. g3 step 52 (pool: TS Jupitermon, BT24-041 and BT24-101 are core). The legal-action mask in src/action/mask.rs (~line 150-177) checks affordability against the printed
  play cost (available_play_assignment_base) and ignores a card's own before_pay_cost play-cost reduction. Minervamon (BT24-041) costs 12 and is reduced by 5 with an
  Iliad Digimon or Tamer in play, so it really costs 7. I confirmed this with a throwaway probe (deleted, not committed): at memory 0 the mask hides Minervamon and at
  memory 3 it offers it. At memory 0 the real cost 7 leaves -7, which is within the -10 floor, so the card should be offered. The existing bt24_041 tests all use memory
  20, so they cannot see this. Suggested fix: have the mask use the same effective cost the play path pays, and add a memory-0 test to bt24_041.rs.
2. g4 step 101: BT25-092's hand/digivolution-card zone menu answered with generic_bool false is not folded. This belongs to the replay-bool-answer-maps lane. Rows 10 and
  91 also note an unstamped generic_bool consumed with no effect at the UnionZone 'Place 1 [Appmon] or [Three Musketeers]' prompt (informational).
3. g1 step 69: AD1-022 (not in core; it is in the opponent seat) digivolve with a printed cost of 1 matches no legal route. This looks like the alt-digivolve-cost-cards
  family (lane h2-alt-digivolve-cost-cards).
4. g3 row 25 note: an unstamped generic_bool true for a hand-digivolve optional effect ('digivolve into a Digimon card with [Kentaurosmon] in its name or the [Holy Beast]
  trait', which appears to be a [Hand] effect) is consumed with no effect because no gate is parked. This is not blocking so far, but it is a possible bool-map gap.
5. Two older review tests (replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short and
  replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer) are red on claude/replay-hunt-integrated itself, apparently from an
  earlier merge into that branch. They are not caused by this lane. The nightly engine-binary-rot EXCLUDE list or a separate fix commit should cover them.
The integrator should know the lane was not re-run from scratch: I verified commit 0b7d2370c and did not re-author it. I did not separately reconfirm that the new tests
  fail without the fix (the earlier pass presumably did the red step). I did re-run the full replay_dcgo_* family with the fix in place and compared the two red tests
  against the base.

### engine-prompt-shape-parity

(1) g8 now stops at step 59 on EX12-071 Saneiketsu Invitation (delay) after EX12-056's On Play. The engine resolves the queued EX12-071 trigger BEFORE the effect-
  initiated alliance attack that EX12-056 starts. DCGO runs the whole attack (rows SelectCountEffect 59, alliance pick 60, SelectAttackEffect 61) first, and only then
  asks EX12-071's 'Select 1 Digimon that will digivolve' (SelectPermanentEffect with canNoSelect:true, row 62). The engine also skips that SW-Digimon pick and goes
  straight to the hand pick. Diagnosis: queued trigger effects must wait until an effect-initiated attack has fully resolved, and the delay's optional SW-Digimon
  SelectPermanentEffect must be kept even when only one candidate exists. Different defect; not fixed.\n\n(2) g1 stops at step 53 on an ambiguous EX13-053 copy (two
  copies on the board). The recording carries ids only, so this is a stale-recording / recorder-identity limit. A newer 0.2.0 recording with identity fields would resolve
  it.\n\n(3) Other chained-optional-pick ladders with the same prompt-shape risk, not touched: EX10-036 and EX11-044 (Choose 1st/2nd [Mineral] or [Rock] (1/3), (2/3)),
  EX7-043 (Three Musketeers return 3, 1/3 and 2/3), and BT7-107 (2/2). Check each against its DCGO C# for a single multi-select.\n\n(4) Pre-existing reds on the
  integrated base, not caused by this lane: replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short and
  replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer (a clone-safety panic on the closure-based AD1-016 fixture). Both also
  fail on other lanes' builds (h2-dedigivolve-count, h2-replay-bool-answer-maps). Add them to the nightly EXCLUDE list or fix them separately.\n\n(5) The first three
  diagnosis notes in the task also named EX12-016, EX12-064 and BT26-033 Jupitermon. The g17 replay is clean, so no further change was needed for them.

### replay-bool-answer-maps

Remaining blockers per game: g2 BT24-102 count rows vs the optional-trigger prompt; g5 ambiguous digivolution-cost count (two cost-2 routes); g4 BT22-031 unimplemented
  (plus BT22-093 Ami Aiba unimplemented, memory drift); g7 duplicate-copy target ambiguity (needs card_index in the recorder board entries). Audit of other card-authored
  two-mode generic_bool menus found by scanning DCGO SetBoolSelection with a non-decline false button: BT25-078 Gazimon (core pool card, Add to hand = true / Place on
  digivolution cards = false) is NOT mappable by label because DCGO asks the card pick first and the bool afterwards while the engine asks the mode first; it needs an
  order fold or an engine prompt-shape change, left unmapped so it still halts. Others not added (not in any pool deck or game, YAML and C# unread): BT13-112, BT17-102,
  BT19-090, BT24-058 (union), P-167, EX3-008, P-128, ST18-14. The zone-style menus (From hand / From trash and similar) are covered by the existing hand_zone_bool path.
  Two pre-existing red tests on the integrated branch (replay_dcgo_review_r4 unfollowable_order_branch..., replay_dcgo_review_r4b cardless_index... which panics on a
  closure-callback clone in selection.rs) fail on the baseline too; the nightly EXCLUDE list should name them.

### trigger-order-bundle

1) g11 step 29, EX12-069 Virus Busters (core pool, Option card with a Security attack trigger). DCGO CanActivateCondition requires a same-level playable [VB] Digimon in
  hand, read against the live attacker level. At the 2-entry TriggerOrder the attacker is EX12-044 and the hand matches, so DCGO lists it. EX12-044's effect then
  digivolves the attacker to EX12-018 (level 6, DUAL), the hand has no level-6 playable [VB] card, and DCGO skips the entry with no prompt. Our YAML tests that hand match
  inside the body (`if count_gte ... level_eq_binding: attacker_level`) after `bind_permanent_property`, so it cannot be a clause `condition`, and the optional outer
  prompt shows anyway. Fix needs new DSL vocabulary: a predicate comparing a card's level to the live event target's level (a `level_eq_event_target` sibling of
  level_eq_binding), then moving the hand-match plus affordable_with_cost into the clause `condition` of the EX12-069 security clause. Touches predicate.rs, compile.rs,
  compiled.rs, the implementation_inventory reference walks, validator.rs and engine dsl_cards/predicate.rs. Same activatability-versus-processing-condition family as
  this lane, not done because of the vocabulary breadth.
2) g16 step 66: DCGO BT23_013 (Jesmon) uses SetBoolSelection (true = play the Atho/Rene/Por token, false = play a Sistermon from hand or trash). Our engine models it as
  EffectChoice 'Choose what Jesmon plays'. The replayer needs a verified bool to EffectChoice mapping for BT23-013 in runners/dcgo_bool_yes_no_cards.json or
  selection_resolve.rs. This is the h2-replay-bool-answer-maps family.
3) The wider audit was not done: other pool cards that author a DCGO activation condition (CanActivateCondition with HasMatchCondition*) or a processing condition
  (ActivateCoroutine plus RemoveUse) as a clause `condition` are not yet flagged. Candidates are mandatory leading-select clauses (delete, bounce, de-digivolve) and OPT
  draw/gain clauses with an in-coroutine test. Each needs a per-card DCGO check, because DCGO is inconsistent (BT25-059 does not gate).

### dedigivolve-count

Remaining blockers; neither game passes yet, and neither is a De-Digivolve defect:
1. 20261113T025510Z-g19, step 57 of 65. This game's owner seat is a pool deck: its EX13-071 and EX13-036 are pool cards. Richard Sampson (EX13-071) plays for 4 at memory
  3, leaving -1, and its On Play "if your opponent has a Digimon, gain 1 memory" takes memory to 0, but DCGO passes the turn. Find where P0's memory drifts or where DCGO
  skips or reorders the gain, using the DCGO source (Assets/Scripts/CardEffect/EX13/Yellow/EX13_071.cs, SharedActivateCoroutine) and the memory trace of rows 47-56.
2. 20261114T142653Z-g1, step 43 of 107. The recording has an unmodelled SelectCardEffect (BT24-041, index 73) between the BT24-102 target pick and the [Iliad] hand pick.
  Check the DCGO BT24_102.cs flow for how it chooses which of the Digimon's effects to activate, and add the prompt or a verified adapter for it. The adapter is
  legitimate only when the engine state proves it.
3. Two test failures that are not caused by this change, since they also fail on an older integrate build (D:\cargo-target\hunt-integrate):
   - replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short: "expected one divergence at row 0, got []".
   - replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer: a panic at selection.rs:360, "cloned PendingSelection has no closure
  callback".
   The integrator should triage them in their own commit.
4. Optional hardening, not done: emit `candidates` for SetCount in Core.cs CountPostfix, so future recordings carry the count set and the evidence-based unit-count
  adapter is needed less.
5. EX13-036, ST12-13, BT24-003 and BT26-073 (all cards in this cluster's game list) show no divergence in this lane's windows once the count rows pass.
6. The rest of this check was not run: the full cards_behavioral suite, and a full cargo test for digimon-dsl beyond lib plus the parse_scalar_unification test.

### replay-multipleskills-index

1. g2, step 46 (new first divergence; a different defect, NOT fixed, an EX13-pool engine/DSL gap in EX12-069 Virus Busters). After MetalGarurumon (EX12-035) resolves, the
  engine parks an optional "You may activate EX12-069's triggered effect" Replacement prompt for P1. DCGO never asks it, and the recording moves on to P0's next turn
  (Breeding), hence actor_mismatch. DCGO EX12_069.cs:44-47 puts HasMatchConditionOwnersHand(CanSelectCardCondition) in CanActivateCondition, so the effect is not
  activatable when the hand holds no same-level [VB] Digimon that is playable at cost -3. Our YAML wraps that check in an `if: count_gte` inside the process, after
  bind_permanent_property binds the attacker level. The engine's outer_optional_guard (first_step_candidate_guard) and the optional_condition_guard (CostGuard) only
  derive from a leading select_hand with a literal filter, so neither recognises it and the useless prompt is offered. Likely fix: add a DSL predicate "same level as the
  event target / attacker" (e.g. level_eq_event_target) so the select_hand can be the first step and the existing candidate guard suppresses the prompt. EX12-069 already
  has a test module (cards_behavioral/ex12/ex12_069.rs), which would need a regression test. Also check the card image for the printed "[Once Per Turn]": DCGO's
  description says it, while the YAML and an existing test say it is not once per turn.\n\n2. g4, step 58 (new first divergence; a different defect, NOT fixed). Replay-
  layer duplicate-copy identity: P0's SelectPermanentEffect names BT24-031 by id at board position 2 of [BT24-031, BT24-041, BT24-031, BT24-084]. Two engine copies answer
  the row but lead to different games, and the next 8 rows do not tell them apart. The recorded boards name cards by id only. This is most likely a STALE-RECORDING
  artefact: the raw log predates the recorder's board card_index fields. Re-recording with recorder 0.2.0 would make it exact.\n\n3. Two tests are red on base, in
  replay_dcgo_review_r4 (unfollowable_order_branch_does_not_cut_the_other_branches_short) and replay_dcgo_review_r4b
  (cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer). They also fail in the sibling lanes' pre-built binaries (h2-homeros-effect-pick,
  h2-dedigivolve-count), whose replay.rs diffs do not touch the trigger-order lookahead. Evidence: r4 now gets no divergence where it expects a halt "is ambiguous ...
  bottommost ... 1 entry fits", which fits the integrated bottom-order lane making the sweep branch followable. r4b panics at selection.rs:360, a hand-built closure
  prompt being cloned and resolved in probe_branch_for_row. My diff does not reach either code path (r4b's EffectChoice entries carry no source_card). I did not rebuild
  base to prove it, so this inference is not proven. Treat them as stale-on-main tests to fix in their own commit.

### unimpl-digimon-misc

(1) 20261114T012101Z-g3, REPLAY-LAYER defect in code/digimon-engine/src/runners/replay.rs remote_trigger_order_choice + accept_outer_gate_for_pick: for a card-less
  MultipleSkills index whose two entries are [optional outer-gated effect (BT17-012), mandatory effect (EX12-061)], evidence row = the mandatory effect's pick
  (SelectHandEffect trash). The probe lets the optional entry qualify through the implied-accept fold, and at step() the pick row phantom-accepts BT17-012's outer gate,
  consuming DCGO's later decline row's slot. Honest fix: when the actor's row after the evidence row is an unstamped generic_bool false by the same actor and the
  optional-gated entry would have to have been accepted for the evidence to fit, the entry must be refuted (or the decline must be consumed as the outer-gate PASS). Needs
  a synthetic-recording regression test in replay_dcgo_*; not attempted because each engine lib rebuild took ~55 min under contention. (2) 20261113T025510Z-g10: implement
  EX10-005 Pagumon (inherited [Your Turn][OPT] 'when your opponent's deck is trashed from, <Draw 1>') -- no DSL timing exists for 'deck trashed from' (grep of digimon-dsl
  found none); and EX2-071 Death Slinger 'when this card is trashed from your deck, gain 1 memory'. Other stubs in that deck still .json-only: BT24-078 Creepymon (X
  Antibody), BT24-096 Seventh Graviton, P-219 Flame Inferno, BT16-071, plus EX2-071, EX10-005. (3) 20261114T034011Z-g1: new stop at step 40 (P1 MultipleSkills index 1, no
  trigger order queued) needs diagnosis; likely another unimplemented/misordered card on the P1 board. (4) Rule-34 note: none of the six authored cards is in the ex13
  core/union pool sets, but each has its own behavioural module so they are admissible if a deck uses them; regenerate data/card_admissibility.json,
  qa/impact/lowering_coverage.json, validated_cards_dsl.json and the DSL vocab docs (inventory walkers for the new step/predicate were updated in the commit). (5) Not
  run: full cards_behavioral, dsl crate suite, lib tests, replay_dcgo_* binaries, python scripts/verify --tier 0, dsl-lint -- the machine was starved (46 concurrent
  rustc, <3 GB free RAM); the integrator should run them.

### frame-model-unlogged

20261114T012101Z-g4 now stops at step 18, a replay-layer ambiguity. DCGO's card-less MultipleSkills index 1 must pick between two identical ST24-14 (DATA SQUAD Tamer)
  mandatory entries, one per copy. This is the same family as lane h2-replay-multipleskills-index (commit bc42f904e, same-card trigger entries), so I did not duplicate
  it. Re-run this game after that lane merges. If it still refuses, the adapter would need to prove from board state that the order between two same-card mandatory
  entries cannot change the game. Separately, the converter still kills a seat in a few games for other reasons: 20261114T012101Z-g4 seat 0 at row 97
  (QueueMainPhaseAction_Internal), 20261113T025510Z-g4 seat 1 at row 41, 20261113T025510Z-g2 seat 0 at row 53, and 20261113T011922Z-g3 / 20261114T142653Z-g6 at PlayCard
  rows 72 / 76. Not diagnosed, as they are outside this cluster. Replays that still fail for other reasons: 20261113T011922Z-g1, 20261113T025510Z-g2, g5, g14, g18,
  20261114T034011Z-g1, 20261115T153822Z-g7.

### ex8-nso-opponent

g10 now stops at a replay-layer or recording gap, not an engine bug. The opponent (the remote seat) triggers EX8-060's DNA digivolve: Piedmon EX8-062 + Myotismon EX8-060
  into Boltboutamon EX8-064.
DCGO's DNADigivolvePermanentsIntoHandOrTrashCard records the hand pick, and only the local or AI path records the two root rows via
  SelectJogressEffect.SelectDigivolutionRoots. For the remote human seat the raw log (recorder 0.2.0) has no root rows: the stream goes hand pick (EX8-064), then the
  PlayCard row (EX8-064, root Hand), then boards already showing [EX8-064, ...].
fold_effect_dna_root_rows needs the two root rows, so the following SelectAttackEffect row ([{player 1, frame -1}], the may-attack target) is paired with the engine's
  AnyField pair prompt and halts.
An honest adapter would apply when the engine's pair prompt has exactly one legal pair (here exactly two own Digimon in the battle area) AND the recording shows the
  PlayCard of the chosen result with the new board. Then the pair is forced and the row is DCGO's missing duplicate. I did not build it: it touches the replay layer,
  which the diagnosis said to leave alone.
Possible recorder-side alternative: hook the remote branch of SelectDigivolutionRoots in Core.cs.
Remaining steps 36-133 are unverified, so more NSo-card gaps may sit behind this one.
Also stale: data/card_admissibility.json, qa/impact/lowering_coverage.json and data/mechanics_readiness.json need regenerating by the integrator after the new YAML cards.

### reveal-bucket-cards

Not part of this cluster, so not fixed here. These look like other lanes' families (the h2-memory-tamer-start-main, h2-unimpl-digimon-misc, h2-alt-digivolve-cost-cards,
  h2-engine-prompt-shape-parity and h2-replay-bool-answer-maps branches), but I did not confirm any of them.
1. g2 (step 44): EX11-056 (Red/Green Tamer, LIBERATOR) has no YAML, only a .json stub. Printed text: [Start of Your Turn] set memory to 3 if <= 2; [All Turns] on a
  digivolve into a Lv5+ [Tyrannomon]/[Dinosaur] Digimon, suspend this Tamer to hatch, then digivolve a breeding-area Digimon for free; [Security] play this card without
  paying the cost.
2. g8 (step 20): EX7-055 Punkmon is a stub. Its [When Digivolving] is 'if you have 1 or fewer Tamers, may play 1 [Yuuki] from hand free', its inherited effect is [Your
  Turn] +2000 DP, and its alt-digivolve is Lv.3 w/[Evil], cost 2.
3. g7 (step 32): replay-layer ambiguity in 'recorded printed digivolution cost N matches 2 legal routes' when two routes at the same reduced cost are legal. The two
  routes appear to be 'cost 0' and one other (the choice labels are cut off in the trace), so this needs a verified-from-state disambiguation rule or a recorder candidate
  field.
4. g6 (step 24): the recorder emits SelectCountEffect 0/1 (and an unstamped generic_bool) as DCGO's 'use this optional effect?' gate, but the engine parks the selection
  directly. The adapter must prove that the gate is DCGO's duplicate prompt, and that the recorded count cannot change the game, before consuming it. The source is an
  EX12-039/EX12-056 [SW] Alliance effect.
5. g18 (step 53): the same count-as-optional-gate shape, here a SelectCountEffect with no candidates against a <Barrier> 'May accept replacement' prompt. The recorder
  would need to emit candidates, or the replayer must map the count 0/1 verifiably onto accept/decline.
6. The ch18_pending_overwrite timeout-guard flake under heavy load: raise or scale the limit, or exclude it from loaded runs.
7. Builds in this lane took 88 minutes cold under cross-lane contention.

### alt-digivolve-cost-cards

None of the five games replays to PASS. Each stops at a defect outside the alt-digivolve cluster, found by comparing the engine's parked prompt with the recorded rows:
1. g2: AD1-023 (pool-core) inherited replacement versus DCGO battle-trigger ordering, with P1's BT21-072 attack triggers (MultipleSkills and discard counts). Needs an
  interactive trace.
2. g5: EX12-069 (pool-core Option) trigger the engine parks for P0 that DCGO does not. Check its DCGO .cs trigger conditions.
3. g7: BT21-077 Regulusmon is unimplemented. Its alt route (Lv.4 [Gammamon]-in-text, cost 3, cost-choice) is the same family. Its other clauses need:
   - a granted <Collision> plus 'start of your main phase this Digimon attacks'
   - On Deletion plays of [Canoweissmon] or a Lv4-or-lower [Gammamon]-text Digimon from trash
   - an inherited On Deletion play
4. g13: replay-layer gap in `runners/replay.rs` / `selection_resolve.rs` (`forced_count_evidence` and the `actor_mismatch` branch). DCGO's IDegeneration asks one forced
  SelectCountEffect per <De-Digivolve 1> instance. When an effect contains N separate instances followed by a deletion of that permanent, and another seat's trigger is
  parked, the single-instance evidence rule refuses. A verified multi-instance rule is feasible: N forced rows, top-N of one permanent, remainder deleted.
5. g1: BT24-102 refire chooser. DCGO's 'Select 1 effect to activate' SelectCardEffect over the OP and WD ActivateClasses of BT24-101 has no engine counterpart, because
  the YAML is a combined clause. Either split the clauses or add a verified adapter for the identical-body case.
Also not run: `cargo test -p digimon-dsl`, the replay_dcgo_* binaries and `python scripts/verify --tier 0`. Runners/ was not touched, but the machine was heavily
  contended, with a dev build taking 30-60 minutes.

### memory-tamer-start-main

1) g7 needs the h2-dedigivolve-count lane (or my removed fold) to clear step 44; the Guardromon De-Digivolve forced count is a recorder-0.2.0 candidate-less count.
  Patch/test for my version saved at
  C:/Users/james/AppData/Local/Temp/claude/fee206060164b783/3a8d0070-6ea1-45e1-80c6-f4f64494dbbf/scratchpad/replay_dcgo_h2_count_no_candidates.rs.bak (is_forced_count_row
  would accept count==1 with no candidates; evidence via forced_count_evidence). 2) g8 now stops at step 26: two AD1-021 copies (P1 slots 0 and 2, one just played) each
  queue an End-of-Turn trigger; DCGO's MultipleSkills row names only the card, lookahead and interchangeable_copies cannot separate them (copy-identity limitation, needs
  card_index per board entry or a verified symmetry proof). Different defect family, not fixed. 3) The BT22-094, BT22-046, EX13-015, EX13-010, AD1-003, LM-061 and EX8-073
  cards in the cluster list were not the blocker for these two games and were not touched. 4) A repo-wide scripts/verify tier 0 needs the integrator to regenerate
  qa/impact/dsl_impact_index.json, qa/impact/lowering_coverage.json and data/card_admissibility.json. 5) Machine was saturated by 14 lanes: each engine build took 45-50
  min, which is why the full replay_dcgo_* sweep on the final tree was not completed.

### trait-substring-audit

g14 now stops at step 41 with an actor_mismatch, a different defect and not trait-related. P0 attacks security and takes the top security card, which is P1's LM-059. The
  engine then goes straight to P1's LM-059 prompt, "Add 1 yellow or red card to your hand" (a Reveal for P1). The recording has an extra P0 MultipleSkills(-1) trigger-
  choice row (recorded step 43) between P0's security take (step 42 generic_bool true) and P1's SelectCardEffect (step 44). Diagnosis: either a replay-layer prompt-shape
  gap (a DCGO-only optional trigger-order prompt for P0) or a missing P0 trigger in the engine at the security-check point. The P0 board there is BT26-029 (Aegiochusmon:
  Holy), BT24-031 and BT24-093, and a TriggerOrder of BT24-003 and BT24-093 was already answered at step 40. I did not dig further because it is not the same family. Also
  note the earlier steps 33-34: DCGO chose digivolve cost 3 via the Aegiomon alt path; the engine's cost-choice prompt (cost 3 or 4) matched, so that part is fine.

### digitama-inherited-effects

g5: not fixed. Shademon (BT19-068) and Nene Amano (BT19-087) have no YAML. Shademon has a DigiXros requirement on Nene Amano plus an On Play top-3 add and an On Deletion
  replay-and-Save. Nene Amano has a Start of Your Turn memory reset and an All Turns DigiXros-substitution effect. Both are a different card-implementation family from
  this lane, in the opponent's non-pool deck. g1: not fixed. 11 JSON-only cards on the opponent seat (list withheld: recording-derived deck content). Without
  ChuuChuumon's Save the engine cannot produce the DCGO Save plus Monimon double trigger. The step-29 DigiXros material mismatch is likely tied to these missing cards,
  but I did not isolate it. Digi-Egg audit: 182 JSON-only Digi-Eggs with an inherited effect still have no YAML (per cards.json, kind 3). None are in the 21-deck pool
  core set; the largest remaining group is the opponent-side BT1 to BT26, EX, ST and P eggs. I did not author them. The previous pass only authored the five that the two
  games or the union set touched.

### refire-source-identity

No blocking residual for this cluster. Not run: cargo test --lib, dsl suite, and the replay_dcgo_* binaries, because the change is confined to one function in
  effect_queue.rs and each build took 1-2 h under contention; the integrator's CI should cover them. Two non-blocking notes from the replay of g6 (row 18 generic_bool
  with no parked optional gate for P1, and row 49 position-only pairing of 2 copies of P-104) are replayer-layer guesses that do not cause a divergence here.

## Census after round 2

Census run with the release `dcgo-replay` built from this branch (539fa1687, before the repair commit) over the recorder folder. The folder now holds 85 games: the 65 from round 1 plus 20 recorded on 2026-11-19. Counts and card IDs only; no deck or hand contents.

**Replay-clean games.** On the 65 shared games the clean count went 18 to 20 (the round-1 report's "6 of 65" counted only a narrower set; the triage JSON shows 18 raw passes, 10 of them real games of 4+ turns). Real-game passes went 10 to 12. 0 regressions: every round-1 pass still passes. Newly clean: `20261115T153822Z#g7` and `20261116T045756Z#g6`. Across all 85 games: 20 pass plus 4 prefix-clean truncated recordings. None of the 20 new games replays clean.

**Movement among the 39 shared games that fail in both rounds:** 35 first divergences moved later in the game, 4 are unchanged, 0 moved earlier; 37 changed defect kind. So the round-2 fixes did advance most games, but the next defect in line stopped them before the end.

**EX13 pool seat** (a seat deck with at least 80% of its cards in the 264-card training-pool core): 72 of 85 games have one. Of the 56 shared pool-seat games, 18 replay clean (16 in round 1), 12 of them real games. 0 of the 16 new pool-seat games are clean.

**Training gate.** All 21 distinct EX13 training lists are fully admissible in `data/card_admissibility.json` on this branch and on the round-1 branch, with 0 blocking cards in either. No pool card's admissibility entry changed. Round 2 modified the pool cards BT24-030, BT24-102 and EX12-021 and newly authored none. Replay failures do not gate training; they are faithfulness findings.

### Round-3 list (62 failing games with a first divergence; 3 more are not games)

36 fall in known residual families, 20 are new defects, 6 are on the recording or converter side.

1. Duplicate-copy / same-card MultipleSkills identity, 15 games (EX13-053, BT24-034, BT24-031, ST24-14, ST24-13, BT25-051, AD1-021, BT16-084, BT17-023, EX13-023, BT24-084 with BT25-030). The raw logs do carry `board_indexes`/`card_index` keys, so the converted rows may be losing the copy identity. Investigate the converter first.
2. EX12-069 activation-condition prompt: the engine offers it, DCGO skips it. Needs a `level_eq_event_target` predicate.
3. BT24-041 play mask ignores the `before_pay_cost` reduction. The illegal-digivolve/play shape recurs in two further games.
4. Deck-order prompt shape: the engine parks "Place remaining cards on deck in any order" where DCGO records a 2-card SelectCardEffect row (BT13-007, EX10-046). New, and cheap to fix in one place.
5. BT25-092 zone-menu bool fold.
6. Digivolution cost-route ambiguity and alt-cost (AD1-022, plus two more games).
7. Count-as-optional-gate rows (SelectCountEffect) against an engine parked prompt, including BT24-034 where the recorder does not log candidates.
8. New engine-offers-extra or misses-effect findings: the declare-a-blocker prompt, BT21-069 hand pick, Kurisarimon play optional effect never offered, EX5-054 name filter missing BT14-097, P-249 trigger not queued, the Cerberusmon: Werewolf Mode trigger order, RevealBucket two-bucket order (BT24-068, EX10-049), and the TriggerOrder seat/extra prompt.

## Integration build status

See the integration result returned with this branch (tier-0 verify, `cargo check --tests`, `cargo check -p dcgo-replay`, lib tests). Gap entries for every lane are in
  `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` under "DCGO PvP replay hunt, round 2".
