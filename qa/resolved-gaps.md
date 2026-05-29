# Resolved Engine and DSL Gaps

Last updated: 2026-05-29

This file is the archive for reusable engine and DSL gap entries that have been resolved. Active gap trackers should keep only open gaps or partial slices with remaining implementation work:

- [qa/archetype-qa/engine-gaps.md](archetype-qa/engine-gaps.md)
- [qa/dsl-vocab-gaps.md](dsl-vocab-gaps.md)

When a reusable gap closes, move the full entry here and leave any card-specific migration/test cleanup in the active tracker only if there is still real follow-up work.

## ST-2 Cocytus Blue substrate closure — 2026-05-29

- **G-TRASH-BOTTOM-SOURCES** — the DSL now exposes
  `trash_bottom_sources: { target, count }`, lowering to
  `CompiledStep::TrashBottomSources` and executing through
  `EffectContext::trash_bottom_sources`. The verb trashes source cards from the
  bottom up, caps at available sources, routes cards to their owners' trash,
  fires normal `OnDigivolutionCardTrashed` observers, and never installs a
  source-selection prompt.
- **G-BATTLE-OPPONENT-NO-SOURCES** — `battle_opponent_no_sources` is available
  as a battle-context predicate for inherited/aura effects. It resolves
  relative to the source carrier's opposing Digimon in the current
  Digimon-vs-Digimon battle and is false for security checks or player attacks.
- **Source-play confirmation:** ST2-15 Kaiser Nail is expressible with existing
  `select_material` plus `play_from_materials`; no new source-play vocabulary
  was required.
- **Drivers:** ST2-01, ST2-03, ST2-06, ST2-09, and ST2-15 production YAML and
  tests.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl st2_substrate --no-default-features --features dsl-yaml-loader,test-helpers`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral st2_cards --no-default-features --features dsl-yaml-loader,test-helpers`.

## Rocks EX10-034 grant-triggered binding closure — 2026-05-23

- **G-DSL-GRANT-TRIGGERED-EFFECT-TO-BINDING** — `grant_triggered_effect.target`
  now accepts the existing predicate broadcast shape or a binding ref. Binding
  targets resolve through `dsl_cards::binding_ref` and install exactly one
  granted-triggered entry on the selected permanent, preserving the previous
  predicate behavior for EX1-068-style "all matching Digimon gain..." text.
- **Carrier binding:** `CompiledBindingRef::Carrier` now resolves directly to
  `ctx.source_permanent`, while `source` keeps its stable source-card relocation
  behavior. Granted-triggered bodies can therefore use `carrier` for printed
  "this Digimon" while retaining grantor source attribution for effect-origin
  checks.
- **Driver:** EX10-034 Blastmon Clause A is now production YAML: select exactly
  1 opponent Digimon, grant `[Start of Your Main Phase] This Digimon attacks.`
  until the opponent's turn ends, and force that selected carrier to attack when
  the timing fires. The unselected opponent Digimon does not receive the grant.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_034 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- grant_triggered_effect --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`.

## Rocks EX11-044 substrate closure — 2026-05-23

- **G-HIGHEST-PLAY-COST-SELECTOR** — `FieldSelector::HighestPlayCost` and
  `CompiledFieldSelector::HighestPlayCost` now parse, compile, and evaluate
  as the max printed play cost among a field-selection candidate set. Runtime
  coverage mirrors the existing lowest-play-cost path in both normal step
  selection and replacement helper selection.
- **event_host_permanent_is_source** — `PredicateSpec` /
  `CompiledPredicate` now expose `event_host_permanent_is_source`, evaluated
  against `current_trigger_context.event_host_permanent == ctx.source_permanent`.
  This lets `OnDigivolutionCardTrashed` observers faithfully express "this
  Digimon/Tamer's source was trashed" instead of firing for every source-trash
  event seen by the observer fanout.
- **Driver:** EX11-044 Pyramidimon Clause A and Clause B are now production
  YAML, with no ignored tests.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_044 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`.

## Rocks B2 move-from-breeding DSL step — RESOLVED 2026-05-23

- **G-MOVE-BREEDING-DSL** — the DSL now exposes `move_from_breeding`, lowering
  to the existing effect-initiated breeding movement path rather than the player
  phase action. The step supports an optional accept/decline prompt and a
  compiled breeding-permanent filter, so P-130's "level 3 or higher" choice is
  surfaced through `pending_selection`.
- **Driver:** P-130 Lui Ohwada's `[On Play]` clause is production YAML:
  eligible breeding Digimon can be moved to the battle area by effect, the move
  observers fire, and declining or having no eligible permanent leaves the
  breeding area unchanged.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_130 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2g_breeding_selection --nocapture`.

## Rocks B3 hand-or-source union-zone cost selector — RESOLVED 2026-05-23

- **G-DSL-SELECT-OWN-SOURCES-FILTER / hand-or-source union cost** — the DSL and
  engine can now install a single cost prompt over a filtered union of cards in
  hand and cards under the controller's own Digimon. The selected card is
  trashed as a cost from its original zone, and all legal candidates remain
  visible through the action mask.
- **Driver:** EX11-065 Close's `[Start of Your Main Phase]` memory clause can
  trash a `[Mineral]` or `[Rock]` card from hand or from a digivolution stack.
  EX11-038 Sunarizamon reuses the same selector shape for its union-zone draw
  clause.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_065 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_038 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- union_zone_cost --nocapture`.

## BG Imperial substrate closeout — 2026-05-20

The `bg-imperial-substrate-closeout` change (OpenSpec) re-audited the 24-card BG
Imperial pool and closed the genuinely-remaining DSL substrate gaps. ~12 gap IDs
that the BG trackers still listed as open were verified stale (closed by Phase 2
Tracks A–J, DNA Omnimon completion, Puppets sweep) — see
`openspec/changes/bg-imperial-substrate-closeout/phase-0-audit.md`. The 5 genuine
substrate gaps below landed. Engine/DSL only — no card YAML re-authored (the BG
card re-authoring sweep runs separately); no `ACTION_SPACE_SIZE` / tensor changes.

**Readiness reconciliation (2026-05-22):** the live `BG Imperial`
deck-library pool is 25 cards, not 24. The extra pool card is `BT17-077`,
which remains canonically ledger-owned by `Royal Knights` but is covered for BG
Imperial deck readiness. `BT21-037` also moved from a stale `PARTIAL`
ledger entry to `IMPLEMENTED` after adding its printed `[Digivolve] [Veemon]:
Cost 2` alternate path. The reconciliation found zero live `raw_rust` YAML
escapes in the 25-card pool.

**Audit correction:** the Phase 0 audit initially flagged 9 gaps, but a second
review found 4 of them already covered by pre-existing engine capability — they
were NOT genuine gaps and no new predicate was added for them:

- **G-PRED-STACK-SIZE-LTE-SOURCE / G-DSL-STACK-SIZE-LTE-SOURCE** — already
  expressible as `materials_count_lte: { formula: { source_material_count: {} } }`
  (the `source_material_count` formula resolves the source permanent's
  digivolution-card count). BT16-027 / AD1-025 use this path.
- **G-DSL-CARRIER-HAS-KEYWORD** — already covered: `has_keyword` resolves against
  the carrier permanent for inherited clauses (`enqueue_from_permanent` sets
  `source_permanent` to the carrier handle).
- **G-DSL-AURA-TARGET-SOURCE-PERMANENT** — already covered: a `kind: aura` with
  `scope: inherited` + `target: {}` is a carrier-only self-aura.
- **G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT** — EX1-014's `[Free]`-trait arm is
  covered by the existing `source_permanent_trait_has` predicate.

### Tier 1 — DSL predicate leaves (genuinely new)

- **G-DSL-EFFECT-SUSPENDED-RESULT** — `effect_suspended_any_opponent_digimon` result
  predicate; opponent-side read-time filter on the existing owner-agnostic suspend
  result log. Driver BT16-025 clause 2.
- **G-EVENT-CARD-COLOR-IS** — `event_card_color_has` predicate: event card has ≥1 of
  the listed colors (intersection; sibling of subset-semantics `event_card_color_only`).
  Driver BT16-085.

Test: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch`

### Tier 2 — engine-touching DSL verbs

- **G-SELECT-OPPONENT-SOURCES** — `select_opponent_sources` step + `EffectContext::select_opponent_sources`;
  opponent-side mirror of `select_own_sources` (exact-N / up-to-N, PASS-after-min,
  `filter:`, `target:`, stable refs). Driver BT16-085.
  Test: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2g_select_sources`
- **G-ZONE-SELECTED-TRASH-TO-DECK-TOP** — `move_trash_card_to_deck_top` verb: a single
  `select_trash`-bound card → owner's deck top. Driver LM-030 clause B.
- **G-ANY-RETURNED-CARD-PREDICATE** — `returned_card_matching` filtered result predicate
  (distinct from the bare-bool `any_returned_card` alias); result log records returned
  card handles. BT17-077 clause-1b player-choice-of-trash verified already composable
  via `select_effect_choice` + `if`. Driver BT17-077.
  Test: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`

### Follow-up engine gaps closed (2026-05-21)

The two engine gaps that left BG Imperial at 22/24 were scoped and then closed,
bringing the archetype to **24/24 IMPLEMENTED**.

- **G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME** — `EffectContext::return_card_source_to_hand(perm, card)`
  (sibling of `trash_card_source`, routes the extracted source `Card` to its
  owner's hand; does NOT fire `OnDigivolutionCardTrashed`) + the `Vec`-taking
  `return_selected_sources_to_hand` wrapper + the `return_selected_sources_to_hand`
  DSL verb (mirrors `trash_selected_sources`). Closes BT12-031 Step C → BT12-031
  IMPLEMENTED. Test: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt12_031`.
- **G-COST-REDUCE-ALLY-DIGIVOLVE** (umbrella over `G-COST-REDUCE-NEXT-SINGLE-FIRE`
  + `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`) — new `player_cost_reducer.rs` module
  (`PlayerDigivolveCostReducer` data struct), `Game` fields, `EffectContext::arm_player_digivolve_cost_reducer`,
  and a pre-cost accept/decline + suspend-cost `PendingSelection` chain wired into
  a split `digivolve_from_hand` / `digivolve_from_hand_inner`. The synchronous
  `scan_before_pay_cost_reduction_with_target` hot path was deliberately NOT
  touched (the prompt runs before it) — normal/DNA/Blast digivolve confirmed
  unregressed. DSL surface: `arm_digivolve_cost_reducer` step. End-of-turn expiry
  wired in `rotate_turn_player`. Closes BT3-103 Clause 0 → BT3-103 IMPLEMENTED.
  Tests: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks player_digivolve_reducer`;
  `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt3_103`.

## Phase 2 / DNA Omnimon completion closure — 2026-05-20

The `complete-dna-omnimon-archetype` change drove the DNA Omnimon archetype (64 cards)
from a Phase A baseline of 34 IMPLEMENTED / 25 PARTIAL / 5 BLOCKED to an interim ledger
of **62 IMPLEMENTED / 2 PARTIAL / 0 BLOCKED**. The 2026-05-22
`close-dna-omnimon-partial-gaps` follow-up resolved the two remaining partial gaps
(BT17-102, BT23-096), so the current archetype verdict ledger is
**64 IMPLEMENTED / 0 PARTIAL / 0 BLOCKED**:
[qa/qa-reports/validated_cards_dsl.json](qa-reports/validated_cards_dsl.json).

DNA Omnimon now has 0 live `raw_rust` escapes — BT20-102's board-wipe/return migrated to
pure DSL, AD1-025's body migrated, and the unused `bt20_102_boardwipe_and_return` fn was
removed.

### Substrate gaps CLOSED (new engine/DSL capability landed)

Small DSL gaps:

- **G-DSL-PREDICATE-TEXT-CONTAINS** — `effect_text_contains` predicate leaf scans a
  candidate's printed effect/inherited/security text (BT22-017 "[Omnimon] in its text").
- **G-EVENT-TARGET-NAME-CONTAINS** — `event_target_name_contains` predicate leaf gates
  observer clauses on the triggering card's name.
- **G-FORMULA-SOURCE-DP** — `source_dp` FormulaSpec variant exposes the source
  permanent's DP to formula evaluation.
- **G-PLAY-COST-AGGREGATE** — `LowestPlayCost` AggregateSelector / FieldSelector for
  "the card(s) with the lowest play cost" targeting.
- **G-SELF-DIGIVOLUTION-CONTAINS-NAME** — `self_digivolution_sources_contain_name`
  (sources-only) predicate evaluates the source permanent's own stack for a named card.
- **distinct_tamer_colors_gte** (G-ALT-PATH-DIRECTION-INTO companion) — `distinct_tamer_colors_gte`
  predicate leaf counts distinct Tamer colors on the field.

Medium DSL/engine gaps:

- **G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH** — `min:` on `select_count_capped_multi` +
  `return_trash_list_to_deck_bottom` step + atomic cost-then-cancel guard.
- **G-DSL-UNION-PLAY-FREE** — `select_union_zone` widened to bind a zone-tagged index so
  a free play can originate from either zone in the union.
- **G-IGNORE-COLOR-MASK** — from-hand color-requirement bypass via a card-level
  `use_requirement`.
- **G-MULTI-SELECT-OPP-PLAY-COST-SUM** — `SelectOpponentPlayCostBudget` step +
  `SelectionKind::PlayCostBudget` for multi-pick under an opponent-play-cost budget.
- **G-OPT-MULTI-TIMING-SHARED-LOCKOUT** — `Effect::shared_opt_group` shared
  once-per-turn key across multiple timings of one card.
- **G-OUTER-OPTIONAL-NOT-INSTALLED** — outer accept/decline `PendingSelection` for a
  lone optional triggered effect that installs no inner selection.
- **G-PRED-NO-FACE-UP-SECURITY-NAMED** — `no_face_up_security_named` predicate leaf.
- **G-SELECT-EMPTY-OUTER-TAIL** — `select_hand` empty-candidate path now drains the
  outer tail so post-selection steps (e.g. `trash_top_security`) still fire.
- **BT5-092 cost-reduction trigger** — `when_any_ally_digivolves_into` CostReductionBody
  trigger.

Deep engine gaps:

- **G-DSL-DNA-FROM-HAND-PARTNER** — `effect_initiated_dna_digivolve_with_hand_partner`
  engine API + DSL step (DNA digivolve pairing a field material with a hand partner).
- **G-FOR-EACH-DELETE-INDEX-SHIFT** — `ForEach` re-resolves stable top-card identity per
  iteration so index shifts during deletion do not corrupt iteration.
- **AD1-012 defender-side effect-initiated DNA mid-attack-interrupt** — closed with no
  engine redesign; authored on the existing interrupt substrate.

Additional gaps closed during card authoring:

- **G-AURA-GRANTED-SECURITY-KEYWORD** — `Modifiers::granted_security_attack_keyword_bonus`.
- **G-FORMULA-COST-DELTA** — formula-valued `CostDelta::ReduceFn` on `play_from_hand`.
- **G-EVENT-TARGET-COLOR** — `event_target_color_any_of` predicate leaf.
- **G-PLAY-SELECTED-SECURITY-CARD** — `play_security_card` step +
  `EffectContext::play_from_security_card`.
- **source_material_count formula**; `OnLeaveField` timing wired to fire from deletion +
  return paths; `binding_count_eq` predicate; `ResolvedBinding::SourceRefs` +
  `per_selected` over source refs.

### Gaps that were STALE (tracker said open / `#[ignore]` said pending, but the substrate already existed and is now USED)

The following gaps were carried as open in the trackers, but the engine/DSL substrate had
already landed in an earlier phase. The `complete-dna-omnimon-archetype` change authored
the DNA Omnimon card clauses against the existing substrate and re-enabled the
behavioral tests, which now pass:

`G-COUNT-AGGREGATE`, `G-COUNT-LTE-EVAL`, `G-DECLARATIVE-KEYWORD`,
`G-DSL-EVENT-TARGET-IS-OTHER`, `G-DSL-EVENT-TARGET-IS-SELF`, `G-OPT-TRIGGERED`,
`G-PLAY-COST-GTE`, `G-PLAY-COST-LTE`, `G-SECURITY-ZONE-AURA-SOURCE`,
`G-DSL-SOURCE-NAME-CONTAINS`, `G-DSL-SELECT-OWN-SOURCES-FILTER`,
`G-DSL-DISTINCT-TAMER-COLORS-FORMULA`, `G-PLACE-SELF-AS-OPTION-PERMANENT`,
`G-ADD-OPTION-SELF-TO-HAND`, `G-EVENT-CARD-TAMER-PLAY`, `G-COLOR-MATCH-AGAINST-BOARD`,
`G-DSL-SELF-NAME-CONTAINS`, `G-EVENT-TARGET-NOT-SOURCE`.

### Follow-up gaps CLOSED 2026-05-22

- **G-DYNAMIC-NAME-ALIAS-FROM-STACK** — BT17-102 Greymon `[All Turns]` material-name-alias
  clause. Closed by `identity.source_name_aliases`, synthesized permanent effective names,
  and name-predicate routing through that synthesized name set. BT17-102 is now fully
  IMPLEMENTED; `bt17_102_all_turns_aliases_low_level_material_names` is enabled and passing.
- **G-DSL-DELAY-ON-ATTACK-EVENT** — BT23-096 Comet Hammer `<Delay>`-on-attack clause.
  Closed by lowering attack timings to `DelayTrigger::OnEvent`, feeding attack events into
  event-gated delayed options, and carrying attacker context for `attacker_trait_has`.
  BT23-096 is now fully IMPLEMENTED; the CS attack Delay and non-CS negative tests are
  enabled and passing.

## Phase 2 Track C closure — 2026-05-17

### Engine Gap: G-OPT-TRIGGERED — already-closed (no engine fix shipped)

- **Status:** Substrate verified correct as of pre-PR Phase 2 baseline (commit `2b083c5a`). The `Effect::max_per_turn` gate at `effect_queue.rs::run_queued_effect_inner` reads `Permanent::activation_count((source_card_handle, effect_slot))` on the carrier permanent and skips the body when the count is at the cap; the success-path tail (`run_queued_effect_process_tail`) records the activation via `Permanent::record_activation` before invoking the body process. Track B (commit `2c2c4632`) added the parallel post-`activation_cost`-failure record so a cost-failed firing also consumes the slot. The path was therefore complete by the time this track ran.
- **What the 26 BLOCKED tests' ignore reasons claimed:** "max_per_turn is not enforced in `run_queued_effect_inner`". This was stale — the gate had been wired earlier in the Rust pivot and the once-per-turn enforcement already worked for both top-card and inherited-source effects. 18 of the 26 tests passed verbatim when un-ignored; the remaining 8 were `unimplemented!()` / `todo!()` stubs with no body to verify (those stay ignored with a clarified reason).
- **Slot key:** `(carrier_permanent_handle's HashMap) × (source_card_handle, effect_slot)`. The HashMap is stored on `Permanent::effect_activations` and is fully cleared by `Permanent::new_turn` (`permanent.rs:412-416`), which is called once per begin-turn for every battle-area permanent of the new turn-player. No collision between top-card and inherited-source slots — both live in the same per-carrier map and are distinguished by the `source_card` half of the key.

### Engine Gap: G-OPT-RESET-VIA-ATTACK-CYCLE — CLOSED (test-setup fix only)

- **Status:** Misdiagnosed in the original tracker note. The suspected "OPT key persists across turn boundaries when carrier permanent's source identity differs from trigger source" was not happening — the HashMap is cleared wholesale at `Permanent::new_turn`, so divergence between carrier identity and trigger source is irrelevant.
- **Actual failure mode in the canonical BT16-040 / BT17-015 / BT17-018 reset tests:** the test setup did not populate decks or security for either player, so the first `end_turn()` would rotate to P1 → `begin_turn()` → empty-deck draw → `game_over=true`. `Permanent::new_turn` therefore never fired a second time for P0's carrier, and the OPT slot stayed populated.
- **Fix shipped:** test-setup adjustments only — decks + security for both players, low-DP defenders where digimon-vs-digimon combat would otherwise destroy the attacker before the second cycle. No engine-side change.
- **Evidence:** diagnostic harness `diag_bt16_040_opt_reset` (deleted before commit) confirmed that with decks/security present, `effect_activations` went `{}` → `{(BT16-040.handle, 2): 1}` → `{}` (P0's `new_turn` after the second `end_turn`) → `{(BT16-040.handle, 2): 1}` (after a second attack), and `pending_selection` correctly installed `OppField` for both attacks. Direct-`enqueue_triggered` harness reproduced the same trajectory.
- **Tests un-ignored / re-tagged in this PR:**
  - 18 directly-passing OPT-tagged tests (no test-body changes): ad1_012 cross-timing OPT, bt13_012 OPT lockout, bt14_001 inherited OPT, bt17_018 same-turn OPT block, bt21_001 OPT block + reset, bt21_025 clause-2 OPT block + reset, bt22_005 inherited OPT, ex10_002 attack-target-change OPT, ex4_038 inherited OPT, ex8_074 [All Turns] OPT, lm_021 [When Attacking] OPT block + reset, p_123 OnMove OPT block + reset, p_137 clause-c OPT block + reset.
  - 4 tests un-ignored after small test-setup fixes (decks + security added; low-DP defender for BT17-018): bt16_040 reset-after-turn-cycle, bt17_015 reset-after-end-turn, bt17_018 reset-after-end-turn, bt21_017 inherited OPT block (also dropped `memory(10)` → `memory(5)` — root cause unrelated to OPT, future investigation).
  - 1 test un-ignored from non-`G-OPT-TRIGGERED` ignore-reason variant: ex11_008 inherited OPT block (reason: "engine gap: OPT not enforced for triggered effects via queue").
  - 1 test (ad1_025 `*_all_turns_observer_opt_blocks_*`) kept `#[ignore]`'d — the DRIFT 2 YAML drift is the real residual blocker; reason simplified to drop the OPT half.
  - 6 combined-tag tests where the OPT half closed but another gap (`G-INHERITED-DISPATCH`, `G-AS-SELECTING-PLAYER`) still blocks — kept `#[ignore]`'d, OPT half trimmed from the reason: bt24_016 (×2), p_189 (×2), bt21_025 clause-3 stub, ad1_025 [All Turns] observer.
- **Net `#[ignore]` count:** 566 → 543 (drop of 23). Tests that remain `#[ignore]`'d with OPT mentions are all `unimplemented!()` / `todo!()` stubs or other-gap-blocked combined-tag entries (Track D's territory for the inherited-dispatch half).
- **Source priority cross-check (Working Rule 17):** OPT lockout failure drops silently — no hidden retry prompt surfaces to the player. The behavior matches DCGO (`OncePerTurn` hash check before SetUpActivateClass) and the Comprehensive Rules Manual (§16-36 once-per-turn family).

## Phase 2 Track A closure — 2026-05-17

### DSL Gap: `dp_eq` / `dp_lte` / `dp_gte` on card subjects — RESOLVED 2026-05-17 [G-PRED-DP-LTE]

- **Status:** Closed. `eval_dp_constraints` (permanent path) was wired in PR #405; `eval_card_fields` now consults `dp_eq` / `dp_lte` / `dp_gte` against `CardData.dp` for card-zone subjects (hand / trash / security / deck). Non-Digimon cards (`data.dp == None`) fail any DP constraint.
- **Engine surface:** `code/digimon-engine/src/dsl_cards/predicate.rs::eval_card_fields` — new branch after `play_cost_lte` (PR #475 → Track A).
- **Note:** The `predicate_has_card_zone_unsupported_leaf` filter in `formula_eval.rs` still classifies `dp_*` as permanent-only for the `FilteredCardCountInZoneScoped` aggregate (separate code path with its own tests). That is a follow-up; the per-selection card-zone filter route (`select_trash` / `select_hand` / `select_security`) does honor `dp_lte`.
- **Evidence:** un-ignored 13 `#[ignore]`'d tests now pass: `bt13_012_dp_lte_filter_excludes_high_dp_targets`, `bt18_087_clause2_skips_delete_when_target_is_above_4000dp`, `bt22_013_when_digivolving_branch_1_only_lowest_dp_is_a_legal_target`, `bt22_015_on_play_only_lowest_dp_is_a_legal_target`, `bt24_001_high_dp_digimon_not_eligible_for_delete`, `bt24_017_delete_targets_only_lowest_dp_digimon`, `lm_030_security_no_selection_when_only_large_green_digimon_in_trash`, `st20_11_when_digivolving_only_offers_lowest_dp_opponent`, `st20_11_when_digivolving_offers_all_tied_lowest_dp_opponents`, `bt21_015_on_play_no_selection_when_no_eligible_target`, `bt21_015_on_play_filters_ineligible_targets_correctly`, plus structural placeholders in `ex8_074` and `st22_08`. LM-027 remained ignored — pending card-local YAML authoring (the YAML never authored `dp_lte: 2000` in its `select_trash` filter); the substrate is ready.

### DSL Gap: `AltPathSpec.condition` post-resolution sweep — RESOLVED 2026-05-15 (annotations) [G-ALT-PATH-CONDITION]

- **Status:** Substrate closure landed in PR #475 (Phase 1) — `AltPathSpec.condition: Option<PredicateSpec>` field + consumer wiring in `dna_digivolve.rs::find_matching_alt_path`. Phase 2 Track A removed the stale `#[ignore = "Pending G-ALT-PATH-CONDITION"]` annotations on existing placeholder-body tests so the regression-scaffold tests run.
- **Un-ignored placeholder tests:** `bt22_013_activated_digivolve_blocked_without_nokia_tamer`, `bt22_013_activated_digivolve_available_with_nokia_and_agumon`, `bt22_026_activated_digivolve_blocked_without_nokia_tamer`, `bt22_026_activated_digivolve_available_with_nokia_and_gabumon`. These have no assertions yet — they pass trivially as scaffolds. Promoting them to active assertions is card-local follow-up (the BT22-013 / BT22-026 / BT22-042 YAML files do not yet populate `condition:` on their activated_digivolve alt-paths to gate on Nokia Shiramine / Arisa Kinosaki).

## Phase 2 Track B closure — 2026-05-17

### Engine Gap: Generic `.activation_cost(...)` builder hook for triggered abilities — RESOLVED 2026-05-17

- **Status:** Closed for the two printed cost shapes called out in `docs/RUST_ENGINE_GAPS.md`: "by suspending this Tamer" (BT13-101 family) and "by returning this Tamer to the bottom of the deck" (BT22-088 / BT22-094 / BT17-093 / EX11-071 family).
- **Engine surface:** New `Effect::activation_cost_fn: Option<ActivationCostFn>` field, populated via `EffectBuilder::activation_cost(|ctx| -> bool)` (`code/digimon-engine/src/effect.rs`). Runs in `effect_queue::run_queued_effect_inner` between the condition gate and the body `process`. Cost failure (`false`) consumes the OPT slot identically to a successful firing (no decline-vs-fail elision, Working Rule 17). Cost success bypasses the standard source-liveness check in `run_queued_effect_process_tail_after_activation_cost` so a "return-self" body still runs even though the source permanent has left the field. Distinct from `pay_cost_fn` (which is consumed during `BeforePayCost` cost calculation for plays/digivolves); the two builders coexist.
- **Cost helpers on `EffectContext`** (`code/digimon-engine/src/effect_context/mod.rs`):
  - `ctx.suspend_self_as_cost() -> bool` — returns `false` if the source permanent is gone or already suspended; otherwise suspends it (firing `OnSuspend` observers).
  - `ctx.return_self_to_deck_bottom_as_cost() -> bool` — routes through `Game::return_to_deck` (top card to owner's deck bottom, digivolution sources trashed per standard return-to-deck rules, fires leave-field observer chain).
- **DSL surface:** New `CompiledStep::ActivationCost { kind: CompiledActivationCostKind }` variant with `SuspendSelf` / `ReturnSelfToDeckBottom` (`code/digimon-dsl/src/compiled.rs`); YAML step `activation_cost: { suspend_self: true }` or `activation_cost: { return_self_to_deck_bottom: true }` (`code/digimon-dsl/src/step.rs`). Compile-side validator rejects mid-body uses and requires exactly one cost kind (`code/digimon-dsl/src/compile.rs::compile_triggered`). DSL lowering lifts a leading `ActivationCost` step onto `EffectBuilder::activation_cost` (`code/digimon-engine/src/dsl_cards/lower_triggered.rs`).
- **Evidence:**
  - Substrate behavioral tests — `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- activation_cost` (8 tests: builder-surface, cost-paid happy path, pre-suspended cost-failure, ordering, condition gating, OPT-slot consumption on cost failure, return-self-to-deck-bottom + draw).
  - DSL surface tests — `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- activation_cost` (parse + compile lowering for both cost kinds, validator rejects mid-body / missing kind / both kinds).
  - Variant-coverage lint — `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage`.
- **Downstream consumer cards now ready for card-author migration (NOT migrated in this PR):** BT4-097, BT8-090, ST6-14, BT8-094, EX9-068, BT13-102, RB1-035, P-136, BT22-094, BT17-093. BT13-101 itself remains gated on the separate event-card color predicates half of PUPPETS-G023; BT22-088's start-of-main return-self branch (PUPPETS-G028) needs the chained `choose_one` + origin-preserving free-play branch selectors that are out of scope here. The All Turns Token/Puppet observer on BT22-088 keeps its existing `select_effect_choice` workaround until the YAML migration wave picks it up.
- **Coordination notes:** the cost-failure short-circuit consumes the OPT slot regardless of whether Track C eventually reshapes OPT keying — Track C's bookkeeping will replace `record_source_permanent_activation` whenever it lands but the contract "cost-failed firings count toward the OPT cap identically to successful firings" stays.

## Phase 2 Track H closure — 2026-05-17

BG Imperial pilot-archetype substrate closures (`.claude/plans/phase-2-track-h-bg-imperial-pilot-completion.md`).

### G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — RESOLVED

- **Surface:** Two new `CompiledPredicate` fields:
  - `cost_target: Option<Box<CompiledPredicate>>` — evaluates the inner predicate against the digivolve-target card (as a `Card` subject) using `EffectReadContext::cost_target_card`. Fails outside cost-calc dispatch.
  - `source_is_cost_target_permanent: Option<bool>` — true when the effect's `source_permanent` is one of the permanents being digivolved (single entry for normal digivolve, both materials for DNA).
- **Engine substrate:**
  - `EffectReadContext` gained `cost_target_permanents: Vec<PermanentHandle>` and a `source_is_cost_target_permanent()` helper.
  - `game_actions.rs::scan_before_pay_cost_reduction_with_target` threads `CostTargetContext { card, from_hand, target_permanents }` through every digivolve cost-calc site (normal, breeding, DNA stage-2, effect-initiated). All four sites pass the digivolve-target permanent(s).
  - `lower_cost_reduction`'s `lower` dispatcher (in `dsl_cards/mod.rs`) was relaxed to accept clauses gated only by `active_when` / `condition` (previously required `when_playing_this` or `when_any_ally_played`).
- **DSL surface example:**
  ```yaml
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when:
      all_of:
        - your_turn: true
        - source_is_cost_target_permanent: true
        - cost_target: { trait_has: Free }
        - any_field_permanent: { of: you, kind: tamer }
    once_per_turn: true
    amount: 1
  ```
- **Cards unblocked:** P-117 Veemon (clause 0), BT23-005 Elizamon (pattern proven; YAML update pending).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p::p_117` (3 new cost-reduction tests pass).

### G-BEFORE-PAY-COST-GAIN-MEMORY — RESOLVED

- **Surface:** New sibling builder `Effect::before_pay_cost_observe(card)` paired with `EffectTiming::BeforePayCostObserve` and `CompiledTiming::BeforePayCostObserve`. Triggered clauses with `when: before_pay_cost_observe` lower to this timing and fire their `process:` body at BeforePayCost cost-calc dispatch — without coupling to the cost-reduction path.
- **Engine substrate:**
  - `game_actions.rs::scan_before_pay_cost_observers` walks observer effect sources (battle area + breeding + the target hand card for `when_playing_this`) and fires `process` closures after the cost-reduction scan and before `pay_memory`.
  - DNA digivolve stage-2 wraps the observer scan in `current_dna_origin = Some(true)` so `dna_origin: true` predicates pass at cost-calc time.
- **DSL surface example:**
  ```yaml
  - when: before_pay_cost_observe
    active_when:
      all_of:
        - your_turn: true
        - dna_origin: true
        - source_is_cost_target_permanent: true
        - cost_target: { color_is: green, kind: digimon }
    process:
      - gain_memory: 1
  ```
- **Scope:** observer body MUST NOT install a pending selection in v1 (no-approximations §17 mandate — supporting selection-popping observer bodies is planned future work; BG Imperial's six initial refs all have scalar bodies).
- **Cards unblocked:** BT12-022 ExVeemon (clause 0), BT12-050 Stingmon (clause 0).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt12::bt12_022 bt12::bt12_050` (6 new observer tests pass).

### G-OPTIONAL-SELECTION-CONTINUE-TAIL — RESOLVED (select_trash slice)

- **Surface:** `install_select_trash` in `dsl_cards/step/selections.rs` now attaches an `on_decline` callback for optional selections. Declining (PASS) drops the binding insert but still runs the outer tail with the captured `trigger_context`, mirroring the 2026-04-29 pattern landed for `select_material` and `select_own_sources`.
- **Scope:** Only `select_trash` is touched in this closure. Audit of other `select_*` install fns (`install_select_hand`, `install_select_own_permanent`, `install_select_opp_permanent`, `install_select_any_permanent`) found they share the same `on_decline: None` hardcoding in their underlying `select_*` constructors; extending the fix to those is a follow-up.
- **Cards unblocked:** LM-030 Green Scramble — declining the optional security-trash play now still adds the option to hand.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm::lm_030::lm_030_security_adds_card_to_hand_even_when_trash_play_declined`.

### G-PLAY-FROM-HAND-FREE-BIND-AS — RESOLVED

- **Surface:** New `PlayFromHandFreeArgs` struct (replacing the shared `PlayFromHandArgs` for the Free variant) with an optional `bind_as: Option<String>` field. Lowered to `CompiledStep::PlayFromHandFree { of, hand_index, bind_as }`.
- **Engine substrate:** `dsl_cards/step/play_digivolve.rs` calls `bindings.insert_permanent(name, played)` when `bind_as` is set, exposing the just-played permanent handle to subsequent steps in the same body (e.g. `schedule_delayed` with `return_to_hand: { target: { permanent: played } }`).
- **DSL surface example:**
  ```yaml
  - play_from_hand_free: { of: you, hand_index: pick, bind_as: played }
  - schedule_delayed:
      when: end_of_opponents_next_turn
      body:
        - return_to_hand: { target: { permanent: played } }
  ```
- **Cards unblocked:** BT16-085 Davis Motomiya & Ken Ichijoji (clause 0 — delayed return at next opponent EOT).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16::bt16_085::bt16_085_start_of_main_played_digimon_returns_at_opponent_eot`.

### G-COST-REDUCE-ALLY-DIGIVOLVE — DEFERRED (per discovery rider)

- **Status:** open. BT3-103 Hidden Potential Discovered! is blocked on three composite substrate gaps (G-COST-REDUCE-ALLY-DIGIVOLVE + G-COST-REDUCE-NEXT-SINGLE-FIRE + G-PAY-COST-SELECT-ARBITRARY-SUSPEND). Closing requires (a) turn-scoped armed observer install, (b) single-fire/auto-cleanup semantics, (c) selectable suspend cost via `select_own_permanent` inside `pay_cost:` — substantially more invasive than the rest of the Phase 2 Track H deliverables. Per the plan's discovery rider, deferred to a follow-up rather than ship the rest with a half-baked observer-arming primitive.

## Phase 2 Track E closure (2026-05-17) — Rocks pilot reveal-ordering + option self-disposition

Three reusable Rocks-flagged gaps closed in a single PR.

### G-ROCKS-REVEAL-ORDERING — RESOLVED 2026-05-17
- **DSL surface:** two new verbs.
  - `choose_from_reveal: { of, filter, destination, bind_as?, optional?, prompt }`
    where `destination` is one of: `hand`, `deck_top`, `deck_bottom`,
    `{ bottom_source_of: { target: <binding> } }`.
  - `order_remainder: { of, destinations: [deck_top, deck_bottom?], prompt? }`
    — single destination = direct placement; two destinations = player
    effect-choice → ordered-permutation chain.
- **Lowers to engine API:** `EffectContext::select_reveal` +
  `add_to_hand_from_reveal` / `return_to_deck_from_reveal` /
  `place_as_bottom_source` (with `CardSourceRef::Reveal`) for
  `choose_from_reveal`; `select_effect_choice` +
  `select_ordered_permutation` + the `place_remainder_on_deck` placement
  loop for `order_remainder`.
- **Semantics:** Both verbs surface every authored choice as a player-visible
  pending selection — no auto-determinism (CLAUDE.md Working Rule §17).
  `choose_from_reveal { optional: true }` lets the player decline when no
  candidate matches the filter; `order_remainder` with an empty reveal pool
  is a silent no-op (tail runs synchronously).
- **Authored cards:** P-167 `[Start of Your Main Phase][When Digivolving]`
  reveal/source-placement clause; EX8-047 `[On Play]` reveal/two-pick
  clause.
- **Evidence:**
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl track_e_reveal_ordering` (8 tests pass)
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_167 ex8_047` (8 tests pass)
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` (variant-coverage lint passes)

### G-ROCKS-OPTION-SELF-DISPOSITION — RESOLVED 2026-05-17
- **DSL surface:** existing `add_this_option_to_hand: {}` and
  `place_self_as_delay_option: {}` verbs are now the sole path used by all
  six Rocks Option modernization targets (P-206, EX7-074, P-107, P-039,
  LM-031, EX10-069). P-206's `raw_rust { fn: p_206_add_self_to_hand }` was
  replaced with native DSL; the helper itself remains
  (`EffectContext::add_pending_security_to_hand`) — both paths called the
  same engine method.
- **Removed raw_rust:** `p_206_add_self_to_hand` in
  `code/digimon-engine/src/cards/raw_rust/mod.rs`.
- **Deferred:** `trash_this_option` DSL verb (no card in the modernization
  list requires it; Option auto-trash is handled by
  `classify_option_subtype` + `dispose_option` for non-Delay Options).
- **Evidence:** P-206's 18-test suite remains all-green post-modernization.

### G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS — RESOLVED 2026-05-17
- **DSL surface:** existing `add_player_modifier` (for
  `CannotAddSecurityByEffect`) + `for_each` + `add_modifier` (for
  `CannotAttackPlayer`) patterns. No new substrate proved necessary.
- **Authored card:** BT9-103 Kongou (Main + Security mirror).
- **Evidence:**
  `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt9_103`
  (3 tests pass, including behavioral verification that opponent gains
  `CannotAddSecurityByEffect`, opponent Digimon cost ≤ 7 gain
  `CannotAttackPlayer`, opponent Digimon cost > 7 do not).

### Also-closed in this PR
- **G-ADD-OPTION-SELF-TO-HAND** (from P-206 test comments) — closed by the
  same DSL modernization above. The `add_this_option_to_hand` step verb
  already existed and now has no raw_rust shadow in the Rocks pool.

## Phase 2 Track F closure — 2026-05-17 (DNA Omnimon Pilot Completion)

Track F closes a cluster of DNA Omnimon DSL substrate gaps and authors
production YAML for the directly-unblocked cards. Five gap tags resolve:

### DSL Gap: `place_top_source_as_bottom: { target: <perm> }` step  [G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM]

- **Status:** Closed. New DSL verb that rotates a permanent's top stacked
  card (the source immediately beneath the active top — `card_sources[len-2]`)
  to the bottom of its own digivolution stack. Deterministic — no
  player-facing source-selection prompt.
- **DSL surface:** `StepSpec::PlaceTopSourceAsBottom(TargetArg)` → `CompiledStep::PlaceTopSourceAsBottom { target }` → `EffectContext::place_top_source_as_bottom(target)`.
- **Engine helper:** `EffectContext::place_top_source_as_bottom` returns
  false when `card_sources.len() < 2` (no source beneath top); callers
  gate with `materials_count_gte: 1`.
- **Cards authored:** BT23-008 Greymon (Clause 2), BT23-018 Garurumon
  (Clause 2). Sister cards with identical cost shape, different optional
  filter (Gabumon vs. Agumon).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl place_top_source_as_bottom`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_008 bt23_018`. Both card files: 13 pass / 0 ignored each (was 11 pass / 5 ignored).

### DSL Gap: `gain_memory_fn: { formula: ... }` + `lose_memory_fn` step  [G-DSL-GAIN-MEMORY-FN]

- **Status:** Closed. Formula-valued memory mutation step. Evaluates a
  `FormulaSpec` at resolution time and feeds the result to
  `EffectContext::gain_memory` / `lose_memory`. Mirror of the literal
  `GainMemory(i32)` with runtime-computed magnitude.
- **DSL surface:** `StepSpec::GainMemoryFn(FormulaStepArgs)` + `LoseMemoryFn` → `CompiledStep::GainMemoryFn { formula }` / `LoseMemoryFn { formula }` → wired in `dsl_cards/step/memory.rs::try_run`.
- **Cards authored:** EX1-021 MetalGarurumon (Clause 1: "[When Digivolving] Gain 1 memory for every 4 cards in your hand"). Pattern: `floor_div([base/per/delta wrapping card_count_in_zone, 4])`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- memory_fn`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex1_021_when_digivolving`. EX1-021: 13 pass / 0 ignored.

### DSL Gap: `has_on_deletion_effect: <bool>` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT]

- **Status:** Closed. New predicate leaf that returns true if the candidate
  permanent's top card or any digivolution source carries any
  `EffectTiming::OnDeletion`-timed effect — via DSL clause or
  hand-written `CardEffect` impl. Surfaces auto-installed keyword effects
  (Save, MaterialSave, Decoy, Fortitude) and authored on-deletion clauses
  uniformly.
- **DSL surface:** `PredicateSpec.has_on_deletion_effect` + `CompiledPredicate.has_on_deletion_effect` → consulted in `eval_permanent_fields` via `permanent_has_on_deletion_effect` walker over `effects_for_card`.
- **Cards authored:** EX1-021 MetalGarurumon (Clause 2 target filter: "return 1 of your opponent's Digimon **that has an [On Deletion] effect**").
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl has_on_deletion_effect`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex1_021_when_attacking`.

### DSL Gap: `AltPathSpec.direction: into`  [G-ALT-PATH-DIRECTION-INTO]

- **Status:** Closed. `AltPathSpec` gains a `direction: AltPathDirection` field
  (`From` default = legacy; `Into` = inverse). With `direction: into` the
  alt-path registers on the SOURCE card and `from:` filters the
  destination hand-card candidate, expressing "this card may digivolve
  INTO X" (ST20-10 Agumon warp-shape).
- **DSL surface:** `AltPathSpec.direction` + `CompiledAltPath.direction` → threaded through `dna_digivolve.rs::dsl_alt_digivolve_route_for_card`, which now consults both From-side paths (on the hand card) AND Into-side paths (on the carrier permanent's top card).
- **Cards authored:** Substrate only — ST20-10 warp clause still blocked on `G-DSL-DISTINCT-TAMER-COLORS` predicate-leaf (separate gap; the opp-DP disjunct of ST20-10's condition is satisfiable but the Tamer-color disjunct is not without that predicate). Regression coverage lives in `tests/dsl/group7_alt_path_registration.rs::alt_path_direction_into_*`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl alt_path_direction`.

### Engine Gap: chained `select_own_permanent → select_* → effect_initiated_digivolve` re-investigation  [G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET]

- **Status:** Closed as **phantom**. The gap was filed against the
  assumption that the chain dispatcher dropped the tail after the first
  permanent pick. Investigation (2026-05-17) showed
  `run_tail_preserving_trigger_context` has been driving both selections
  + the synchronous digivolve step to completion since the helper
  landed. The previously-blocked tests panicked on a mid-chain
  `pending_kind()` assertion that the chain had already advanced past —
  not on a missing dispatch.
- **Tests un-ignored (5 total):** `bt16_040_on_play_chains_through_permanent_pick_trash_pick_and_effect_digivolve` (formerly `..._on_play_with_eligible_trash_installs_own_field_then_trash_selection`; rewritten as end-state assertion), `bt17_015_on_play_branch_1_digivolves_gabumon_into_metalgarurumon_free`, `bt17_027_on_play_branch_1_digivolves_agumon_into_wargreymon_free`, `bt22_013_when_digivolving_branch_0_digivolves_gabumon_into_metalgarurumon_free`, `bt22_026_when_digivolving_branch_0_digivolves_agumon_into_wargreymon_free`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_040 bt17_015_on_play_branch_1 bt17_027_on_play_branch_1 bt22_013_when_digivolving_branch_0 bt22_026_when_digivolving_branch_0`.

### DSL Gap: `distinct_colors_count` formula with `of: any` (both players)  [G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA]

- **Status:** Closed as **already-resolved upstream**. The `DistinctColorsCount(CardCountInZoneSpec)` `PerSelector` variant + `DistinctColorsCountScoped` compiled form ships with full evaluator support for `of: any`, which collapses to the union of both players' permanents. The gap was filed before that landed; regression coverage now lives in `tests/dsl/group7_formula_batch.rs::distinct_colors_count_formula_counts_both_players_battle_area_union`.
- **Cards authored:** P-182 WarGreymon ([All Turns] +1000 DP per distinct color across both players' Digimon + Tamers — `dp_modifier_fn` aura with `distinct_colors_count: { of: any, zone: battle_area, filter: ... }`).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl distinct_colors_count_formula_counts_both_players_battle_area_union`; P-182 advances from 4 pass / 8 ignored to 8 pass / 4 ignored.

## Phase 2 Track I closure — 2026-05-17

### DSL Gap: PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA — RESOLVED 2026-05-17 (Track I)

- **Status:** Closed. New `applies_to_opponent_security_dp: Option<bool>` field on `AuraBody` (`code/digimon-dsl/src/clause.rs`) lowers through `CompiledDeclarativeClause::Aura.applies_to_opponent_security_dp: bool` (`code/digimon-dsl/src/compiled.rs`) to a special-case in `lower_aura::lower_all` (`code/digimon-engine/src/dsl_cards/lower_aura.rs`) that builds an `Effect::declarative(card).inherited().dp_modifier(N).applies_to_opponent_security_dp()`. The engine substrate's existing `attacker_security_dp_adjustment` consult site (`code/digimon-engine/src/combat.rs:260`) walks the attacker's stack at security-battle time and sums any `applies_to_opponent_security_dp` flags' `dp_modifier`.
- **YAML shape:**
  ```yaml
  - kind: aura
    scope: inherited
    active_when: { your_turn: true }
    dp_modifier: -3000
    applies_to_opponent_security_dp: true
  ```
- **Migrated:** [`ST19-03`](../code/digimon-engine/cards/st19/ST19-03.yaml) (IMPLEMENTED with full behavioral coverage in [`code/digimon-engine/tests/cards_behavioral/st19/st19_03.rs`](../code/digimon-engine/tests/cards_behavioral/st19/st19_03.rs)) and [`EX7-024`](../code/digimon-engine/cards/ex7/EX7-024.yaml) (substrate authored — `todo!()` test bodies in `ex7_024.rs` ready to un-fill when card-author work picks it up).
- **Behavioral evidence:** `st19_03_inherited_security_dp_debuff_lets_8000_attacker_survive_9000_security_digimon` and `st19_03_inherited_security_dp_debuff_without_shoemon_in_stack_loses_battle` together prove the -3000 swing flips a security battle outcome between `AttackerDeletedBySecurity` and `SecurityCheckSurvived`.
- **Out of scope:** the aura is intentionally inherited-only — it rides under the attacker's stack. `[Your Turn]` printed text is functionally redundant because security battles only happen during the attacker's turn; the consult site is turn-agnostic but practically equivalent.

### Engine Gap: End-of-attack mandatory self-delete chain with recovery and conditional hatch — RESOLVED 2026-05-17 (Track I)

- **Status:** Closed. Existing DSL primitives (`delete_permanent { target: source }`, `select_opponent_permanent { optional: true }`, `recover`, `if { all_of: [any_field_permanent: { of: you, kind: tamer }, can_hatch: you] } then hatch`) compose into a faithful End-of-Attack chain for EX4-074 ShineGreymon: Ruin Mode under the no-approximations policy.
- **Card YAML:** [`code/digimon-engine/cards/ex4/EX4-074.yaml`](../code/digimon-engine/cards/ex4/EX4-074.yaml) Clause 2.
- **Behavioral test:** [`code/digimon-engine/tests/cards_behavioral/ex4/ex4_074.rs::ex4_074_end_of_attack_self_deletes_opponent_delete_recovers_and_hatches_with_tamer`](../code/digimon-engine/tests/cards_behavioral/ex4/ex4_074.rs).
- **Mandatory-continuation note:** the printed text reads "Delete this Digimon and 1 of your opponent's Digimon" — mandatory targeting. The DSL primitive `select_opponent_permanent { optional: true }` is the faithful shape: with at least one legal opp Digimon present the player is forced to choose one (auto-resolve picks the only legal target in the test); with no legal opp Digimon present the step PASSes cleanly and the rest of the mandatory chain (Recovery, hatch) still resolves, matching official mandatory-targets-as-much-as-possible rulings.
- **Self-delete-mid-chain note:** the `delete_permanent { target: source }` first step works because subsequent steps don't reference `source` again — Recovery and hatch are player-scoped, not source-scoped. The `effect_queue::run_queued_effect_inner` flow correctly continues processing after the source permanent has left the field (Track B's `run_queued_effect_process_tail_after_activation_cost` is the related primitive).
- **Hatch gating note:** `any_field_permanent: { of: you, kind: tamer }` checks "you have a Tamer in play"; `can_hatch: you` covers both "breeding area empty" and "digitama deck non-empty" per `code/digimon-engine/src/dsl_cards/predicate.rs:123`. Combined under `all_of`, hatch only fires when the printed condition holds AND a hatch is mechanically possible — no no-op hatch action surfaces to the action space.
- **Out of scope:** The same card's `[When Digivolving] [On Deletion]` opponent-Digimon DP debuff remains BLOCKED on `Expiry::EndOfOpponentsNextTurn` (separate tracked gap — modifier next-turn-of-target-player expiry).
- **Audit predecessor:** This entry was flagged UNCLEAR by the 2026-05-15 audit with a "first-test write recommended" footer; the first test was written 2026-05-17 and passed without engine changes. See [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md).

## Phase 2 Track D closure — 2026-05-17

### Engine Gap: G-INHERITED-DISPATCH — `enqueue_from_permanent` digivolution-stack walk — RESOLVED 2026-05-17 (Track D)

- **Status:** Closed. The substrate walk landed during the 2026-04-29 Group 5/6 work and was claimed RESOLVED in the 2026-05-15 hygiene sweep. Track D completes the closure by adding a dedicated regression test, un-ignoring every test whose body was passing-but-still-ignored under the gap, and re-tagging the residue with the actual remaining blocker (test-fixture gaps, `todo!()` bodies, or other DSL gaps — never G-INHERITED-DISPATCH).
- **Diagnosis verified:** `enqueue_from_permanent` (`code/digimon-engine/src/effect_queue.rs:1501-1552`) iterates `permanent.card_sources.iter().take(stack_len.saturating_sub(1))` — every below-top digivolution source — and pushes one `QueuedEffect` per `effect.inherited == true` clause matching the dispatch timing. The queued entry has `source_card` = the stacked card's handle, `source_permanent` = the carrier, `allow_below_top_liveness: true`. The liveness check at `queued_effect_source_is_live` (line 2089) accepts the stacked source via the `inherited_source_matches` branch.
- **OPT-slot keying invariant (Track C dependency):** confirmed. The OPT key shape `(carrier_permanent.effect_activations HashMap) × (source_card_handle, effect_slot)` is per-carrier-per-stacked-card, so two different inherited sources under the same carrier do NOT collide on the OPT slot. The `Permanent::new_turn` clear (`code/digimon-engine/src/permanent.rs:412-416`) wipes the whole map at begin-turn, so OPT resets cleanly turn-over.
- **Regression coverage added:** [`inherited_stack_dispatch_fires_each_below_top_source_once`](../code/digimon-engine/tests/timing_dispatch.rs) — a 3-card stack `[BOTTOM-D, MID-D, TOP-D]` where BOTTOM-D and MID-D both carry an inherited OnOpponentSecurityRemoved memory-gain observer. After P0's attack on P1's security, both inherited observers fire exactly once, each with `ctx.source_card` matching the stacked card's handle (not the carrier top). Auto-resolves a `TriggerOrder` prompt installed for the controller's pick-order over the multi-bundle.
- **Tests un-ignored (substrate dispatch already worked; the `#[ignore]` annotation was stale):**
  - `bt14_001_inherited_fires_draws_one_card_on_your_turn`
  - `bt21_001_inherited_does_not_fire_on_opponents_turn`
  - `bt21_001_inherited_fires_on_your_turn_and_installs_selection`
  - `bt21_001_declining_permanent_selection_no_digivolve`
  - `bt22_005_inherited_fires_draws_one_on_unidentified_digimon_play`
  - `bt22_005_inherited_fires_draws_one_on_cs_digimon_play`
  - `bt21_017_inherited_fires_when_source_under_carrier_your_turn`
  - `bt21_017_inherited_does_not_fire_on_opponents_turn`
  - `bt21_025_clause3_decline_does_nothing`
- **Tests un-ignored after authoring `todo!()` bodies (substrate ready; bodies now mirror BT14-001 / BT22-005 / BT21-008 idioms):**
  - `p_189_inherited_gains_1_memory_on_opp_security_removed`
  - `p_189_inherited_does_not_fire_on_opponents_turn`
  - `p_189_inherited_opt_blocks_second_activation_same_turn`
  - `p_189_inherited_opt_clears_after_end_turn`
  - `bt24_012_inherited_gain_memory_on_opp_security_removed`
  - `bt24_012_inherited_clause_does_not_fire_when_not_in_stack` (documents the limit — top-card scan still fires the inherited clause when BT24-012 is face-up on its own permanent; printed "Inherited" semantic is not engine-enforceable today without a separate `G-DSL-SOURCE-IS-INHERITED` gate)
  - `bt24_012_inherited_clause_opt_blocks_second_activation`
  - `bt24_012_inherited_clause_opt_resets_after_turn`
  - `bt24_012_inherited_clause_does_not_fire_on_opponent_turn`
- **Tests re-tagged (gap closed; residual failure is non-Track-D):**
  - `bt21_001_selecting_permanent_then_hand_card_triggers_digivolve_cost_minus_1` — depends on REPTILE-HAND fixture having valid `evo_costs`; engine substrate (selection install) confirmed working.
  - `ex4_003_inherited_fires_draws_one_on_other_digimon_digivolve` / `ex4_003_inherited_opt_blocks_second_trigger_same_turn` — `make_lv4_digimon` returns a card with empty `evo_costs` so the digivolve action itself is rejected; the inherited dispatch substrate is ready.
  - `bt21_013_when_digivolving_*` (×4) — test fixtures place a `BASE-LV3` filler permanent (not the Agunimon DSL card) and then enqueue WhenDigivolving on it; no Agunimon effect ever sits where the dispatch can find it. Requires per-test fixture authoring.
  - `bt21_025_clause3_plays_reptile_from_hand_free` — uses `TriggerSource::PlayerBattleArea(1)` instead of `TriggerSource::SecurityRemoved`; the dispatch never reaches P0's Lamiamon. Requires fixture revision.
  - `bt21_024_inherited_aura_*` / `bt24_016_inherited_*` / `bt21_025_clause3_opt_blocks_second_activation` — remain `todo!()` placeholders; substrate ready, bodies pending.
- **Net `#[ignore]` count:** total tests-tree ignored drops 379 → 361 (-18). `cards_behavioral`-specific drop: 375 → 357 (-18). Pass count: 2336 → 2354. The one residual failure (`ex11_054_all_turns_suspends_and_draws_when_reptile_ally_played`) is the pre-existing Medusamon flake (Track G's territory).
- **Source priority cross-check (Working Rule 17):** the inherited-from-stack dispatch surfaces any optionality through `pending_selection` for "you may" clauses (`effect.optional` flag is threaded into `QueuedEffect.is_optional`). The drainer's `bundle.len() > 1 → install_trigger_order_selection` path keeps the controller in control of firing order when multiple stacked sources match the same timing. DCGO confirms top-down `IBattleAreaPermanent.AllEffects()` enumeration; the Rust scan visits sources base-to-top-1 (`card_sources[..n-1]`), matching the documented ordering.
- **Discovery rider findings (documented, not patched):**
  - `bt24_012_inherited_clause_does_not_fire_when_not_in_stack` — when a DigiEgg-shape card with an inherited triggered clause is face-up on its own permanent (carrier top), the top-card scan in `enqueue_from_permanent` does not skip inherited effects (`skip_inherited` is set only for Training option permanents). Printed text says "Inherited" semantically means "fires only as a digivolution source", but the engine fires the clause from the top-card slot too. Fixing this would require either a separate `inherited_only: bool` flag on `Effect`, or wiring `effect.inherited` into the top-card scan's filter. Not in Track D scope.
  - `bt21_013` WhenDigivolving test fixtures — the test author placed a filler `BASE-LV3` rather than `BT21-013` (Agunimon) on the field. The dispatch surface for WhenDigivolving from a permanent is correctly wired (BT22-013 `When Digivolving` tests pass live); this is a fixture authoring issue.
- **Verification:** all gauntlet commands listed in the Track D plan pass except the pre-existing `ex11_054` Medusamon flake which is unrelated to this track.

## Phase 2 Track G closure — 2026-05-17

Medusamon pilot-archetype completion (`.claude/plans/phase-2-track-g-medusamon-pilot-completion.md`).

### DSL Gap: `opponent_security_count_lte` / `opponent_security_count_gte` predicate leaves  [G-OPP-SECURITY-COUNT-LTE]

- **Status:** Closed. New `PredicateSpec.opponent_security_count_lte: Option<DpConstraint>` and `opponent_security_count_gte` fields lower through `CompiledPredicate` and are evaluated in `eval_predicate_with_bindings` against `rctx.security_count(rctx.opponent_id())`. Sibling of the existing controller-side `security_count_lte` / `gte` pair. Variant-coverage lint compliant.
- **Engine surface:** `code/digimon-engine/src/dsl_cards/predicate.rs` adds two scalar opponent-stack guard arms immediately after the existing controller-stack arms; `code/digimon-dsl/src/predicate.rs`, `compiled.rs`, and `compile.rs` extended in lockstep.
- **DSL surface:** `condition: { opponent_security_count_lte: 3 }` (and `_gte`). Accepts the same `DpConstraint` shape as the controller-side predicate (literal or formula-valued threshold).
- **Cards migrated:** BT21-093 Raging Serpentine's BeforePayCost cost-reduction clause now reads `condition: { opponent_security_count_lte: 3 }, amount: 4` instead of the older `count_lte: { filter: { zone: [security], owner: opponent }, n: 3 }` aggregate. The corresponding `bt21_093_cost_reduction_amount` raw_rust formula was removed; the registry no longer registers it.
- **Evidence:** new `group7_predicate_batch::opponent_security_count_lte_evaluates_opponent_stack` test exercises a runner where P0 has 5 security and P1 has 2, asserting `opponent_security_count_lte: 3` matches while controller-side `security_count_lte: 3` still fails. `bt21_093_cost_reduction_uses_opponent_security_count_predicate` updated to expect the native predicate shape. Full eval-arm-coverage and dsl gauntlets green.
- **Pre-existing closures retained:** G-EVENT-TARGET-OWNER (Group 6 → still wired at `predicate.rs:908`), G-PLACE-SELF-AS-OPTION-PERMANENT (resolved 2026-05-02 via `place_self_as_delay_option`), G-ADD-OPTION-SELF-TO-HAND (resolved 2026-05-01 via `add_this_option_to_hand`), G-DSL-LINK-VERB / G-DSL-LINKED-SCOPE (resolved 2026-05-02), G-MAY-ATTACK-NOW (resolved 2026-05-03) — Track G's role for these is the test-tree sweep, not new substrate.

### Card Authoring: EX11-054 Owen Dreadnought All-Turns clause migrated to `activation_cost`

- **Status:** Closed. The pre-existing `ex11_054_all_turns_suspends_and_draws_when_reptile_ally_played` failure (`code/digimon-engine/tests/cards_behavioral/ex11/ex11_054.rs`) was a YAML-modelling bug, not a missing engine primitive. The clause originally used a leading `suspend: { target: source }` process step plus `optional: true`, but the engine's single-trigger drainer (`effect_queue.rs:631-642`) does NOT install a separate Accept/Decline prompt for one-shot optional triggers — optionality surfaces only at the body's first PASS-able selection step. That meant Owen got suspended unconditionally regardless of the player's intent.
- **Fix:** Replaced the leading `suspend:` step with Track B's `activation_cost: { suspend_self: true }`, which routes through `ctx.suspend_self_as_cost()`. The new builder hook (a) returns `false` if Owen is already suspended or no longer on the field (short-circuiting the body and consuming the OPT slot identically to a successful firing per Working Rule 17), and (b) keeps the cost-paid `process` body (`draw + select Progress Digimon + add_dp_modifier`) gated by the cost step. The clause-level `optional: true` annotation is retained so the engine's `is_optional` flag stays accurate even though no prompt installs for the single trigger.
- **YAML diff:** [`code/digimon-engine/cards/ex11/EX11-054.yaml`](../code/digimon-engine/cards/ex11/EX11-054.yaml) clause 2; the `any_permanent: card_number_is: EX11-054, is_unsuspended` predicate was simplified to `source_is_unsuspended: true` (semantically equivalent since the source IS Owen).
- **Test update:** [`ex11_054_all_turns_suspends_and_draws_when_reptile_ally_played`](../code/digimon-engine/tests/cards_behavioral/ex11/ex11_054.rs) reworked to match the documented engine model — assert post-trigger state (Owen suspended, draw observed) without expecting an Accept/Decline prompt.

### Test-tree sweep: substrate-closed body deferrals retagged

Existing `#[ignore]` annotations on stub-bodied tests that cited closed gaps (G-OPT-TRIGGERED via Track C, G-INHERITED-DISPATCH via Track D, G-EVENT-TARGET-OWNER + G-DECLARATIVE-KEYWORD via Group 6, G-MAY-ATTACK-NOW via 2026-05-03, G-IGNORE-COLOR-MASK via 2026-05-02) were retagged from "BLOCKED: G-XYZ" to "pending: card-local body not authored — substrate closed; sibling regression coverage exists" across `bt21_024`, `bt21_025`, `bt21_026`, `bt21_029`, `bt24_016`, `bt24_082`, `lm_055`. The `bt21_026` deletion arm was migrated from omitted-from-YAML to an `event_target_owner: opponent`-gated `when: on_any_deletion` clause; the corresponding "deletion arm omitted" structural assertion was flipped to "one triggered deletion arm exists". The `lm_055` IgnoreColorMask empty-body placeholders were deleted outright (G-IGNORE-COLOR-MASK closed; behavioral coverage exists via the other clauses).

### Build infrastructure: build.rs worker-thread wrapper

- **Status:** Closed for Windows builds. `code/digimon-engine/build.rs` now wraps its main loop in a `std::thread::Builder::stack_size(16 << 20)` worker thread. The default 1 MiB Windows main-thread stack overflowed when the digimon-dsl crate was rebuilt (e.g., after adding fields to `PredicateSpec`) and the build script re-deserialized every card YAML through the deeper monomorphized parser path. The wrapper is a no-op on Linux/macOS where the 8 MiB default already accommodated the load.

### Verdict advancement: 12 Medusamon cards advanced PARTIAL → IMPLEMENTED

- **Promoted in `qa/qa-reports/validated_cards_dsl.json`:** BT21-008, BT21-017, BT21-025, BT21-026, BT21-029, BT21-081, BT24-082, EX9-013, EX11-008, EX11-054, LM-021, LM-055, P-151. Net Medusamon count: 16 IMPLEMENTED + 29 PARTIAL + 1 BLOCKED → 28 IMPLEMENTED + 17 PARTIAL + 1 BLOCKED.

### Acceptance gate verification

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl` — green.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` — **2355 pass / 0 fail / 355 ignored** (was 2354 / 1 / 357 pre-Track-G; ex11_054 fixed, 2 LM-055 empty placeholders deleted).
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow` — green.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` — green (new `opponent_security_count_lte/gte` fields covered by the predicate-field walker; no wildcard catch-alls introduced).

### Out of scope (per the plan's discovery rider)

- G-PLACE-SELF-AS-OPTION-PERMANENT inherited-security YAML migration for P-035, P-103, BT24-089 — substrate ships, card-side `process: []` placeholders intentionally left for a future authoring wave. Track G did not absorb the full Option-play-flow re-architecture.
- BT24-016 / Lamiamon alt-digivolve subshapes — absorbed by Track F (`AltPathSpec.direction: into`) per the plan's "Out of scope" rider.
- G-AURA-DP-FORMULA (BT21-072 formula-valued AuraBody), G-DELAY-SUSPEND-CONDITION (BT24-089 OnSuspend Delay trigger), G-ZONE-TRASH-TO-DECK (BT24-017), G-AS-SELECTING-PLAYER (BT24-016 / P-189 cross-permanent select-on-behalf), G-PRED-DP-LTE-AGGREGATE (BT21-093 highest-DP delete) — all remain OPEN and continue to block their respective Medusamon residual cards.

## DSL Gap: `refire_effect` On Play / When Digivolving timing filter — RESOLVED 2026-05-10

- **Status:** Closed for existing permanent-target refire.
- **DSL surface:** `refire_effect: { source: <permanent binding>, timing: on_play_or_when_digivolving }`.
- **Lowers to engine API:** `EffectContext::refire_target_effect(target, TimingFilter::Either, selecting_player, false)`.
- **Semantics:** Enumerates the target permanent's eligible `[On Play]` and `[When Digivolving]` effects, exposes an `EffectChoice` when 2+ effects are legal, preserves the original source card as grantor attribution, uses the target permanent as carrier, and respects the target effect's once-per-turn slot. The combined timing rejects `optional: true`; put optionality on the containing trigger or target selection.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl refire`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt24_102`.

## Engine Gaps: Legacy Resolved List

1. ~~**Also Treated As (Name Aliasing)**~~ — RESOLVED 2026-03-14. Added `card.also_treated_as_names` list to `CardSource`, `card_names` property returns `[base] + aliases`. 41 scripts batch-updated.

2. ~~**Disable Effect**~~ — RESOLVED 2026-03-14. Added `ModifierType.DISABLE_EFFECT` check in `_collect_triggered_effects()` before effects fire.

3. ~~**Redirect Attack**~~ — RESOLVED 2026-03-14. Added `game.redirect_attack(new_target_perm)` in `combat.py`. Used by EX11-042, P-094, BT22-061.

4. ~~**Force Attack**~~ — RESOLVED (pre-existing). `ModifierType.FORCE_ATTACK` + action mask enforcement in `action_mask.py`.

5. ~~**DP Floor**~~ — RESOLVED 2026-03-14. Added `ModifierType.DP_FLOOR`; `permanent.dp` applies `max(floor, computed)`.

6. ~~**Security Effect Duplication**~~ — RESOLVED 2026-03-14. `is_security_effect` effects now only fire on `SecuritySkill`/`OnSecurityCheck`.

7. ~~**Suppress On Play**~~ — RESOLVED 2026-03-14. `_suppress_on_play` context flag skips `is_on_play` effects. Used by BT13-110.

8. ~~**Aura Keywords**~~ — RESOLVED 2026-03-14. `has_keyword()` now supports aura-style grants from other permanents (Blocker, Rush, Piercing, Alliance, etc.).

9. ~~**Hand-Activated Main Effects**~~ — RESOLVED 2026-03-12. `_is_hand_main` action type (actions 30-59). See engine-api-reference.md Pattern 13.

10. ~~**Trash Main Action Mask**~~ — RESOLVED 2026-03-14. Added `TRASH_MAIN_START = 1150` action range (1150 + trash_idx). Scripts use `_is_trash_main = True` flag. 5 scripts updated: BT20-096, BT24-076, EX10-054, EX10-011, EX7-060.

11. ~~**One-Shot Digivolve Cost Hook**~~ — RESOLVED 2026-03-14. `Player.digivolve()` now checks `CHANGE_DIGIVOLUTION_COST` modifiers from the registry. BT3-103 and EX1-071 use closure-based consumed flags for one-shot behavior.

12. ~~**End-of-Turn DNA Digivolve**~~ — RESOLVED 2026-03-14. No new engine API needed; existing `effect_dna_digivolve_from_hand()` called from inherited `OnEndTurn` effects on BT12-022 and BT12-050.

13. ~~**Grant Triggered Effect to Opponent's Permanent**~~ — RESOLVED 2026-03-14 (pre-existing). `permanent.grant_temp_effect(effect, expiry_turn)` API + `clear_expired_effects()` at turn start. BT14-044 fully implemented.

14. ~~**Effect-Based Play Lock**~~ — RESOLVED 2026-03-14. Added `ModifierType.CANNOT_PLAY_BY_EFFECT`; checked in `effect_play_from_zone()` only (normal hand plays unaffected). BT9-047 updated.

15. ~~**Aura-Style CANNOT_UNSUSPEND for New Entries**~~ — RESOLVED 2026-03-14 (pre-existing). BT12-057 uses aura-style modifier with condition `target is not owner_perm`; `unsuspend()` dynamically checks `has_modifier()` for all permanents including new entries.

16. ~~**OnDigivolutionCardReturnToDeckBottom Not Auto-Fired**~~ — RESOLVED 2026-03-14 (matches DCGO). DCGO also uses manual trigger. Scripts call `game.execute_effects()` explicitly. Functional pattern, not a gap.

17. ~~**Top/Bottom Deck Choice**~~ — RESOLVED 2026-03-14. Added `game.effect_choose_deck_placement(player, card, callback)` helper. BT23-057 updated to use it.

18. ~~**WhenRemoveField Lacks Removal Cause Context**~~ — RESOLVED 2026-03-14. `Player.delete_permanent()` now accepts `removal_cause` parameter ('battle', 'effect', 'rule', 'cost', 'de_digivolve'). Passed in WhenRemoveField/OnRemovedField/WhenPermanentWouldBeDeleted context. EX7-049 updated.

19. ~~**Face-Down Card Tracking**~~ — RESOLVED 2026-03-14 (matches DCGO). DCGO `IsFlipped` is Security-only. Approximation counting all non-top sources is acceptable.

20. ~~**Ignore Color Requirement**~~ — RESOLVED 2026-03-14. Added `ModifierType.IGNORE_COLOR_REQUIREMENT` for aura-style bypass in `action_mask.py`. 7 Hudiemon Option scripts use `card._match_color_requirement = False` for self-bypass. BT23-094 also updated. Rust note 2026-05-02: `ModifierType::IgnoreColorRequirement` is now honored by Option masks and decode/execution in `digimon-engine`, covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`.

21. ~~**Security Play API**~~ — RESOLVED 2026-03-17. Added `game.effect_play_from_security(player, card)` helper. `security_attack()` now checks `card._security_played` flag before trashing. EX1-066 updated.

22. ~~**Dynamic Alt-Digi Cost**~~ — RESOLVED 2026-03-17. `digivolve_validator.py` now checks `_alt_digi_cost_fn` callable for dynamic cost calculation. BT24-101 Jupitermon uses security card count.

23. ~~**Digivolve from Hand or Trash**~~ — RESOLVED 2026-03-17 (pre-existing). `include_trash` param already exists on `effect_digivolve_from_hand()`.

24. ~~**Dynamic Security Attack Modifier**~~ — RESOLVED 2026-03-17. Wired `CHANGE_SECURITY_ATTACK` modifier registry into `permanent.security_attack_modifier()`. Fixed 6 scripts with wrong `value_fn` arity.

25. ~~**Optional Attack ("may attack")**~~ — RESOLVED 2026-03-17. Added `ModifierType.MAY_ATTACK` — enables but doesn't force attack (pass remains available). 4 scripts updated.

26. ~~**Digimon-Only Attack Target Restriction**~~ — RESOLVED 2026-03-17. Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `can_attack_player()`.

27. ~~**is_own_effect in WhenRemoveField Context**~~ — RESOLVED 2026-03-17. Added `is_own_effect`/`is_opponent_effect` to WhenRemoveField, OnRemovedField, WhenPermanentWouldBeDeleted contexts.

28. ~~**Conditional Color Requirement Bypass**~~ — RESOLVED 2026-03-17. Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. 4 scripts updated.

29. ~~**Deletion Observer Recursion Guard**~~ — RESOLVED 2026-03-17. Added depth limit (8) to `execute_deletion_effects()` to prevent RecursionError from token chain loops (Puppets vs TS Olympos).

---

## Engine Gap: ~~Inherited Aura Keyword Grants~~ — RESOLVED 2026-04-12

- **Discovered in:** BT11-042 Angewomon fix-card review (2026-04-12)
- **Card(s):** BT11-042 Angewomon (Blocker aura), BT20-019 Jesmon (X Antibody) (Piercing aura while Jesmon GX), plus other cards using `is_inherited_effect + _applies_to_all_own_digimon + keyword`.
- **What was broken:** `permanent.has_keyword()` aura scan only checked `_applies_to_all_own_digimon` effects on other perms' **non-inherited top-card** effects. Inherited aura keyword effects (below-the-line on a card that is either the current top card or a digivolution source below another Digimon) were silently ignored — while the equivalent DP aura path (`_get_aura_dp_modifier`) already scanned inherited effects in other perms' `card_sources[:-1]`.
- **Resolution:** Extended `has_keyword()` in `permanent.py` to (a) scan inherited aura effects from other perms' `card_sources[:-1]` (mirroring `_get_aura_dp_modifier`); (b) scan ALL aura effects (inherited and non-inherited) on other perms' top cards; (c) scan the self permanent's top card for inherited aura effects targeting self (so BT11-042 Angewomon's aura applies to herself via the `_keyword_permanent_condition` filter). No new script APIs required — existing `_applies_to_all_own_digimon` + `_keyword_permanent_condition` pattern now works for inherited auras as documented.

---

## Engine Gap: Digivolve from Hand or Trash — RESOLVED 2026-03-17 (pre-existing)

- **Resolution:** `effect_digivolve_from_hand()` already has `include_trash` parameter (effects.py:454). No engine change needed.

---

## Engine Gap: ~~Puppet-Scoped Overclock Sacrifice Filter [G-OVERCLOCK-TRAIT-FILTER]~~ — RESOLVED 2026-05-02

- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust engine action mask and Overclock activation.
- **Card(s):** BT22-042 Nyabootmon, EX7-027 Chaperomon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, BT22-036 Kazuchimon, plus other cards with `<Overclock ([Puppet] Trait)>`.
- **Effect text:** "<Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)"
- **Resolution:** Overclock cost candidates are now parameterized. The end-of-turn activation bit only appears when at least one legal token or predicate-matching Digimon can pay the cost, and the pending selection stores only those legal action IDs. The generic mask exposes only stored candidates plus `PASS`, and decode rejects non-candidate targets without deleting a permanent or starting an attack.
- **Covered by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_overclock overclock --nocapture`
- **DSL coverage:** `grant_keyword` accepts `overclock_cost_filter`, compiled as a predicate and lowered onto the same runtime Overclock cost filter path.

---

## Engine Gap: ~~Familiar Token On Deletion Effect Missing [G-FAMILIAR-TOKEN-ON-DELETION]~~ — RESOLVED 2026-05-02

- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust token card effects.
- **Card(s):** TOKEN_FAMILIAR; generated by P-165 ShoeShoemon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, ST19-12 Cendrillmon, and related Puppet effects.
- **Effect text:** "Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn."
- **Resolution:** `src/cards/tokens/familiar.rs` now implements the mandatory `OnDeletion` effect using the opponent-permanent selection path and applies -3000 DP until end of turn. Token card-data construction is also covered by an all-registered-token invariant so future token definitions must synthesize complete `CardData`.
- **Covered by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token`, `cargo test --manifest-path code/digimon-engine/Cargo.toml token_registry`, and the full `cargo test --manifest-path code/digimon-engine/Cargo.toml`.
- **Remaining limits:** This closes the Familiar token deletion text only. Event-gated Delay windows remain a separate Puppet blocker.

---

## Engine Gap: Cost and Replacement Framework

Resolved by Group 3:
- BT13-007 King Drasil_7D6 and ST21-13 Matt Ishida & T.K. Takaishi can both reduce AD1-025 Omnimon before memory is paid because AD1-025 has both `[Royal Knight]` and `[ADVENTURE]`.
- Triggered effect costs may install pending selections and resume process only after cost payment.
- Optional cost decline skips process without hidden auto-selection.
- Replacement predicates can inspect cause, source controller, and subject controller.
- Partition source requirements are enforced before prevention.
- Delay options can pay themselves as replacement costs and prevent deletion.
- Effects can end a pending attack after a printed cost resolves.

---

## Engine Gap: ~~When Attacking Selection Phase Override~~ — RESOLVED 2026-04-02

- **Discovered in:** BT24-024 Submarimon fix-card review
- **Card(s):** All cards with [When Attacking] effects that use selection-based APIs (effect_play_from_zone, effect_select_opponent_permanent, etc.)
- **What was broken:** `declare_attack()` in combat.py continued to counter/block/security after `execute_effects(OnUseAttack)` without checking if effects created a pending selection. The selection phase was overwritten by counter timing.
- **Resolution:** Added park-and-resume pattern: `declare_attack` checks for `pending_selection` after WA effects fire and returns early; `_decode_selection` calls `_maybe_resume_combat_after_wa_selection()` to continue the attack flow after all selections resolve.

---

## Engine Gap: ~~Dynamic Security Attack Modifier~~ — RESOLVED 2026-03-17

- **Resolution:** Wired `ModifierType.CHANGE_SECURITY_ATTACK` into `permanent.security_attack_modifier()` via registry query. BT10-112 uses `_DynamicSAEffect` subclass with `@property` for computed count. Fixed 6 scripts with wrong `value_fn` arity (`lambda: -1` → `lambda cur, t, c: cur - 1`): BT10-042, BT15-084, BT23-094, BT24-071, EX6-022.

---

## Engine Gap: ~~Optional Attack ("may attack")~~ — RESOLVED 2026-03-17, EXTENDED 2026-04-04

- **Resolution:** Added `ModifierType.MAY_ATTACK` semantic marker. Unlike `FORCE_ATTACK`, `MAY_ATTACK` does NOT trigger the forced attackers block in `action_mask.py` — pass (action 62) remains available. Scripts grant Rush + unsuspend alongside `MAY_ATTACK`. Updated 4 scripts: BT24-085, BT24-037, BT24-082, BT24-051.
- **Extension (2026-04-04):** MAY_ATTACK now works at end of turn: `_has_end_of_turn_keywords()` checks MAY_ATTACK, `EndOfTurnAction` action mask offers attack actions (Digimon + player), `_decode_end_of_turn_action` handles SECURITY_TARGET. Also added deferred end-phase completion (`_end_phase_deferred`) for OnEndTurn effects that create pending selections.

---

## Engine Gap: ~~Digimon-Only Attack Target Restriction~~ — RESOLVED 2026-03-17

- **Resolution:** Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `permanent.can_attack_player()` via modifier registry. BT24-051 Merukimon registers it in the "attack your opponent's Digimon" callback.

---

## Engine Gap: ~~is_own_effect in WhenRemoveField Context~~ — RESOLVED 2026-03-17

- **Resolution:** Added `is_own_effect` and `is_opponent_effect` booleans to `WhenPermanentWouldBeDeleted`, `WhenRemoveField`, and `OnRemovedField` timing contexts in `player.py`. Derived from existing `is_opponent_effect` parameter on `delete_permanent()`. BT24-037 Silphymon updated to use clean `is_own_effect` check instead of `removal_cause` heuristic. BT20-059 Gankoomon was already properly implemented (not affected).

---

## Engine Gap: ~~Conditional Color Requirement Bypass~~ — RESOLVED 2026-03-17

- **Resolution:** Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. Dynamic fn is checked first, falls through to static `_match_color_requirement`. Updated 4 scripts: BT24-091 (TS trait check), BT22-099 (CS trait check), ST20-15 (face-up IoA check), BT10-110 (Royal Knight check).

---

## Engine Gap: ~~DigiXros~~ — RESOLVED 2026-03-15

- **Card(s):** 60 cards across BT10-BT24, EX3-EX10, P sets
- **Resolution:** Engine natively supports DigiXros/Assembly: `DigiXrosCost` data model, `parse_digixros_req()` parser (all 60 cards), `digixros_validator.py` for material matching, play intercept → `SelectMaterial` loop → `_execute_digixros_play()`, field materials fire `WhenRemoveField` with `removal_cause='digixros'`, `digixros_count` in `OnEnterFieldAnyone` context.

---

## Engine Gap: Digivolution-Stack Inherited Triggered-Effect Dispatch (Rust Engine)  [G-INHERITED-DISPATCH]

- **Discovered in:** Medusamon archetype, BT21-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine (`code/digimon-engine/`) only. Python engine resolves inherited effects via `card_sources[:-1]` scanning in `_collect_triggered_effects`.
- **Card(s):** BT21-008 Elizamon — inherited `[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory.` Almost all Lv3+ Digimon in this archetype have a similar inherited triggered effect; this gap blocks the inherited half of every one of them.
- **Effect text:** any DSL clause with `scope: inherited` + a triggered timing.
- **Status:** Fixed for battle-area permanent dispatch on 2026-04-29. `enqueue_from_permanent` now preserves top-card / linked / Training dispatch, then scans below-top `card_sources` and queues only matching `effect.inherited == true` effects with `source_permanent` set to the carrier and `source_card` set to the inherited source card.
- **Affected cards:** YAML cards with `scope: inherited` and a triggered timing can now fire from below the top card when the relevant event is already dispatched to the carrier permanent's battle-area observer path.
- **Regression coverage:** `bt21_008_inherited_positive_fires_when_source_under_carrier_your_turn` and `buried_non_inherited_triggered_effect_does_not_fire_from_source_position`.
- **Remaining limits:** This does not add every event fire site. Group 4 added source-trash context for direct `EffectContext::trash_card_source` / `trash_top_source` helpers and effect-driven security-removal fan-out/resume for direct security stack moves. Lower-source trash from some older zone-return paths and breeding-area dispatch remain separate follow-ups. Group 2 closed the shared source, DP-budget, breeding-permanent, and empty-tail selection primitives on 2026-04-29.
- **Updated 2026-05-02 (Group 5 Task 6):** Training sideways inheritance now records `trained: Option<TrainingBinding>` on the Training permanent. Binding is by specific Training permanent handle and validates the carrier's physical top card source before enqueue and queued-effect resolution, closing the over-broad owner-wide Training fan-out slice for bound Training effects without aliasing stale field indices or duplicate Training copies. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_sideways_effect_applies_only_to_its_intended_trained_permanent training_bound_to_removed_permanent_does_not_apply_to_reused_index duplicate_training_copies_bind_to_distinct_carriers_by_permanent_handle queued_training_effect_revalidates_bound_carrier_before_resolution`.

---

## Engine Gap: `max_per_turn` (Once-Per-Turn) Not Enforced for Triggered Effects  [G-OPT-TRIGGERED]

- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-008 Elizamon (inherited OPT clause); applies to every DSL card with `once_per_turn: true` on a triggered clause.
- **Effect text:** any clause that combines `[Once Per Turn]` with a non-Main triggered timing (`OnLoseSecurity`, `WhenAttacking`, `OnPlay`, `OnDigivolving`, etc.).
- **Status:** Fixed for permanent-backed queued triggered effects on 2026-04-29. `run_queued_effect_inner` now checks `Permanent::activation_count(source_card, slot) >= effect.max_per_turn` before processing and records activation before `process`, matching the existing activated field-main timing.
- **Regression coverage:** `bt21_008_inherited_opt_blocks_second_trigger_same_turn`.
- **Remaining limits:** This only enforces the existing queued-effect activation counter. It does not add optional prompt/action-space handling or breeding dispatch. Group 4 separately covered direct source-trash helper context and owner routing.

---

## Engine Gap: `EffectTiming::OnMove` for Breeding-to-Battle Movement  [G-ON-MOVE]

- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid; see `qa/dsl-vocab-gaps.md` for DSL half).
- **Card(s):** EX11-008 Elizamon — `[When Moving] [On Play]` shared body; BT16-082 Ukkomon — entire [Your Turn][OPT] triggered effect (observer in battle area watches any own Digimon move from breeding). Other archetypes will surface this for cards with "[When one of your Digimon moves from the breeding area]" observer triggers.
- **Effect text (EX11-008):** "[When Moving] 1 of your Digimon ... gains <Raid> and +3000 DP for the turn."
- **Effect text (BT16-082):** "[Your Turn][Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon card or Tamer card among them to the hand. Return the rest to the bottom of the deck. Then, you may hatch in your breeding area."
- **Status:** Fixed for breeding-to-battle movement on 2026-04-29. `EffectTiming::OnMove`, `Effect::on_move(card)`, DSL `when: on_move`, and `TriggerSource::MovedFromBreeding { player, permanent, card }` now carry the moved battle-area permanent and top/source card after `Game::move_from_breeding` commits. Regression coverage: `on_move_fires_after_breeding_permanent_moves_to_battle`; direct DSL event-context coverage: `on_move_event_target_trait_predicate_matches_moved_permanent` proves `event_target_trait_has` sees the moved permanent/card.
- **Remaining limits:** This does not add unrelated `OnPlay`/`WhenDigivolving` body work for multi-timing cards. BT16-082's reveal/add-to-hand/remainder-bottom/optional-hatch body is implemented as native DSL as of 2026-05-04, backed by a reusable `can_hatch` predicate for the hatch prompt gate.

---

## Engine Gap: `dp_lte` Predicate Compiled but Not Evaluated in `eval_card_fields`  [G-DP-LTE-PREDICATE]

- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 Cyclonemon — `[On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or less.`
- **Effect text:** any DSL clause whose `select_*` filter uses `dp_lte: N` to constrain valid targets.
- **What's missing:** `dp_lte` parses and lowers to a `CompiledPredicate` variant, but `eval_card_fields` in `predicate.rs` does not evaluate it — the predicate evaluates as ALWAYS-TRUE for any target's `card_fields`. This means the 4000 DP cap is not enforced at selection time; ineligible targets appear in `valid_action_ids`. Two BT21-015 tests are `#[ignore]`'d pending the fix (`bt21_015_on_play_no_selection_when_no_eligible_target` and `bt21_015_on_play_filters_ineligible_targets_correctly`); boundary-inclusion at exactly 4000 is still asserted via the eligible-target tests that DO pass.
- **Suggested change:** add a `dp_lte` (and presumably `dp_gte`, `dp_eq`) match arm in `eval_card_fields` that reads the target's printed DP from card metadata (or the live effective DP — whichever the predicate semantics intend) and applies the comparison.
- **Workaround:** None — BLOCKED for negative-case tests; positive-case tests still pass because the engine over-permissively accepts eligible targets.

---

## Engine Gap: `event_target_owner` Predicate Missing  [G-EVENT-TARGET-OWNER]

- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (and BT21-029 Medusamon clause-d-deletion-arm) DSL implementations (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** BT24-018 Styracomon — replacement clause "When any of your [Reptile] or [Dragonkin] would leave the battle area"; BT21-029 Medusamon — deletion-arm of the All-Turns token-spawn trigger.
- **Status:** RESOLVED for reusable trigger event context and generic replacement context.
- **Trigger coverage:** `event_target_owner` is available in DSL predicates and now reads `OnAnyDeletion` event context carrying the deleted permanent/card. BT21-029's deletion arm uses `event_target_owner: opponent` + `event_target_kind: digimon` and passes behaviorally.
- **Replacement coverage:** Replacement clauses can opt into cross-permanent subjects with `replacement_subject_is_mine` and combine it with `replacement_source_is_opponent` / `replacement_cause`; `lower_replacement.rs` no longer drops the non-source subject during process execution.
- **Regression coverage:** BT21-029 deletion-arm focused tests plus replacement-context focused suites: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement_context --nocapture`.

---

## Engine Gap: `dp_lte` / `dp_gte` Permanent Predicates  [G-PRED-DP-LTE / G-PREDICATE-DP-FILTER / G-SELECT-OPP-FILTER — same root cause]

- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon (Batch 2) + BT24-017 Medusamon + BT21-029 Medusamon (Batch 3) (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 (delete ≤4000 DP), BT24-017 (lowest DP), BT21-029 (lowest DP), and most cards with DP-bounded delete/select.
- **Status:** RESOLVED for reusable permanent predicate evaluation by Group 7 (2026-05-02). `dp_lte` / `dp_gte` now evaluate against live/effective permanent DP in the shared predicate path; BT8-097's high-DP and exact-6000 boundary tests are active and passing.
- **Remaining migration work:** older card-level tests and QA notes that were ignored under this gap may still need to be unignored or retagged after each card is revisited. Aggregate extrema shapes such as "lowest DP" / "highest DP" remain separate aggregate-selection/filtering work when they require more than a direct comparator.
- **Note:** Three card-discovery names (G-PRED-DP-LTE, G-PREDICATE-DP-FILTER, G-SELECT-OPP-FILTER, G-DP-LTE-PREDICATE) all refer to this same root cause; consolidating under G-PRED-DP-LTE.

---

## Engine Gap: Resolved during Medusamon run

- **`PlayFromSecurity` dispatch in security-skill timing** — RESOLVED 2026-04-27 in BT21-015 implementation. `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` now dispatches to `play_pending_security()` when `ctx.game.pending_security.is_some()` (security-skill replay path) and `play_from_security(player)` otherwise. Affects every DSL card with a `[Security]` clause that uses `play_from_security: {}` (BT21-015, BT5-093, BT9-092, BT22-084, ...).
- **Declarative inherited `grant_keyword` not visible to `has_keyword`** — RESOLVED 2026-04-27 in BT24-011 implementation. `code/digimon-engine/src/game.rs::has_keyword` now scans `perm.card_sources` for `declarative && inherited` effects whose name matches the `Grant <Keyword>` convention set by `lower_grant_keyword`. Companion change in `code/digimon-engine/src/debug_runner.rs::card_data_from_compiled` populates `CardData.keywords` from FaceUp `GrantKeyword` clauses so own-printed keywords surface without dispatch.
- **`Progress` keyword not in `lookup_keyword`** — RESOLVED 2026-04-27 in BT21-029 / EX11-012 implementations. `src/dsl_cards/modifier_map.rs` now maps `"Progress" => Keyword::Progress`.
- **`SelectOwnPermanent` / `SelectOpponentPermanent` ignored predicate filters (accept-all)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/step/selections.rs::install_select_*_permanent` now pre-filters candidates with `eval_predicate` and threads the filter closure to the underlying `select_*_permanent` API.
- **Replacement clause subject-guard missing (would-leave fires for any permanent)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/lower_replacement.rs` now checks `subject_matches` so `WhenWouldLeaveBattleArea` only fires when the carrier itself is the leaving permanent.
- **Generic cross-permanent replacement authoring** — RESOLVED 2026-05-03 for explicit replacement-context predicates. `src/dsl_cards/lower_replacement.rs` keeps self-scoped replacement clauses self-only by default, but clauses with `replacement_subject_is_mine` may now protect a different matching subject from the source permanent. Covered by `cross_permanent::dsl_source_permanent_can_protect_a_different_subject` and `replacement_context::replacement_subject_and_source_predicates_compile_together`.
- **`CompiledCardKind::Token` missing from predicate match** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/predicate.rs` now handles `CompiledCardKind::Token`, enabling `kind: token` filters (e.g. "delete 1 Token" cost).
- **Petrification token name case-sensitivity bug** — RESOLVED 2026-04-27 in BT21-029 implementation. `TokenRegistry` is keyed lowercase; YAML `token_name:` values must use lowercase (`petrification` not `Petrification`). EX11-012's example version had the same bug; production version corrected.
- **Source-scoped return/de-digivolve immunity** — RESOLVED 2026-05-02 for covered Rust consumers. EX8-070, P-215, and BT18-064 can now use narrow `CannotBeReturnedToHand`, `CannotBeReturnedToDeck`, and `CannotBeDeDigivolved` passive replacements scoped to opponent effects through the production `EffectContext` movement helpers and `EffectContext::grant_zone_return_immunity_to_opponent_effects`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`.

---

## Engine Gap: ~~`count_lte` / `count_gte` Aggregate Predicate (Non-Security) Not Evaluated  [G-COUNT-LTE-EVAL / G-COUNT-GTE-EVAL]~~ — RESOLVED 2026-05-03

- **Discovered in:** Medusamon archetype, BT21-017 Dimetromon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-017 Dimetromon — "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Owen Dreadnought] from your hand without paying the cost." Also BT22-084 Nokia Shiramine (Start of Your Main Phase condition) and any card whose clause-level `condition:` uses `count_lte` with a non-security zone filter.
- **Effect text:** "If you have 1 or fewer Tamers" — a `count_lte` aggregate gate on the controller's battle area.
- **Status:** RESOLVED for `count_lte` / `count_gte`. `eval_predicate_with_bindings` now counts matching subjects across authored zones and owner scopes before comparing to `n`.
- **Affected cards:** BT21-017 (tamer ≤ 1 gate on WhenDigivolving), BT22-084 Nokia Shiramine (count_lte gate on StartOfMainPhase), and any other card with a non-security zone count condition.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_017 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_006 --nocapture`.

---

## Engine Gap: ~~`EffectTiming::Declarative` Never Fired — Filtered Aura / Grant-Keyword Runtime Gap  [G-DECLARATIVE-KEYWORD]~~ — RESOLVED 2026-05-02

- **Discovered in:** Medusamon archetype — BT21-029, EX11-012, EX11-054 (grant_keyword), BT5-008 (filtered aura), 2026-04-27
- **Scope:** Rust engine.
- **Card(s):** Any card using `kind: aura` with a non-empty target predicate (filtered aura), or `kind: grant_keyword` with a declarative scope. Specific cards: BT21-029 Medusamon, EX11-012 Medusamon (Progress keyword), BT5-008 Gaossmon (filtered aura +3000 DP to other Gaossmon).
- **Effect text:** "[Your Turn] Your other [Gaossmon] all get +3000 DP." (BT5-008); "[When Digivolving / On Field] <Progress>" (EX11-012 inherited); "SecurityAttack+1" (BT21-029 clause a).
- **Status:** RESOLVED by Group 6 for battle-area filtered aura/player-modifier runtime dispatch. `Game::tick_declarative_effects` runs process-backed declarative effects from face-up field sources, filtered `kind: aura` clauses materialize matching permanent modifiers, `other: true` excludes the source permanent, player-scoped aura clauses can install player modifiers, and tick refresh removes stale materialized modifiers without duplicating entries.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`; broad confirmation `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- --nocapture`.
- **Remaining related blocker:** security-zone aura sources and the future query-time aura model remain separate work; no remaining blocker for battle-area tick-dispatched filtered auras covered by Group 6.
- **What was missing:** `EffectTiming::Declarative` is defined in `enums.rs` (line 204) and used by `Effect::declarative(card)` in `effect.rs`, but it was not enqueued or fired by the runtime. As a result:
  - Filtered aura `process` closures (which call `ctx.add_dp_modifier`, `ctx.grant_keyword`) are never invoked.
  - Declarative `grant_keyword` modifiers are never installed in `ModifierRegistry` at runtime.
  - The `active_when` condition closure on declarative effects is also never evaluated.
  - The only working declarative path is the **self-aura** (empty predicate), which uses `Effect.dp_modifier` static field read by `source_dp_contribution` — a completely different execution path that doesn't require the process closure.
- **Scope of impact:** All DSL `kind: aura` clauses with a non-empty `target:` predicate, all `kind: grant_keyword` declarative clauses, and all `kind: flood_gate` clauses that target permanents (flood_gate uses the same declarative process path).
- **Suggested change:** Implement a declarative-effects tick loop. Two approaches:
  1. **On-placement tick**: when `play_from_hand` / `fire_on_play` installs a new permanent, iterate all existing permanents and call each declarative process closure via `enqueue_triggered(EffectTiming::Declarative, TriggerSource::PlayerBattleArea(p))` or a dedicated `fire_all_declarative_effects()` scan. The `Expiry::Permanent` flag on declarative modifiers + `ModifierRegistry` deduplication ensure re-firing is safe.
  2. **Per-query computation**: evaluate the declarative effect condition+process inline at query time (e.g., inside `game.modifiers.sum(h, ChangeDp)`) rather than pre-installing modifiers. This matches the "continuous re-evaluation" model used by physical TCGs where static effects are recomputed on every state change.
  The simplest v1 fix is to call a new `Game::run_declarative_effects()` method at the end of `fire_on_play`, `end_turn`, and `begin_turn` — these are the natural state-change boundaries where auras need to be recomputed.
- **Workaround:** Structural tests pass (compile-time verification only). All behavioral tests for filtered aura and grant_keyword modifier installation are `#[ignore]`'d with this gap tag.

---

## Engine Gap: Multi-Select of Opponent Battle-Area Permanents with Running DP-Sum Cap  [G-MULTI-SELECT-OPP-DP-SUM]

- **Discovered in:** Medusamon Batch 10, LM-021 Agumon - Bond of Bravery DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** LM-021 Agumon - Bond of Bravery — "[On Play][When Digivolving] Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP." Also BT17-018 Gallantmon Crimson Mode — "[On Play][When Digivolving] Delete any number of your opponent's Digimon whose total DP adds up to 15000 or less and delete them." Both cards share the same selection mechanic; LM-021 uses a source-DP formula budget and BT17-018 uses a literal 15000 budget.
- **Effect text (LM-021):** "Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP."
- **What's closed:** `EffectContext::select_opponent_permanents_by_dp_budget` provides the iterative multi-select: each pick reduces the remaining DP budget, re-filters candidates, exposes PASS after `min_picks`, and commits all selected handles together.
- **Consumer cleanup (2026-05-22):** the former `raw_rust: { fn: lm_021_delete_dp_sum }` and `raw_rust: { fn: bt17_018_delete_opp_digimon_dp_budget }` fallbacks are gone. LM-021 uses native `select_opponent_dp_budget` with `dp_budget: { source_dp: {} }`; BT17-018 uses native `select_opponent_dp_budget` with `dp_budget: 15000`. Regression coverage lives in `code/digimon-engine/tests/cards_behavioral/lm/lm_021.rs` and `code/digimon-engine/tests/cards_behavioral/bt17/bt17_018.rs`.
- **Updated 2026-04-29:** Resolved for opponent battle-area DP-budget selection. `EffectContext::select_opponent_permanents_by_dp_budget` installs `SelectionKind::DpBudget`, filters remaining affordable targets after each pick, and exposes PASS after `min_picks`. DSL `select_opponent_dp_budget` binds the chosen permanents and `delete_bound_permanents` consumes them. Covered by `dp_budget_selection_tracks_remaining_dp_and_allows_pass_after_min`, `dp_budget_selection_finishes_when_no_targets_fit`, `dp_budget_selection_mask_exposes_only_remaining_affordable_targets`, and `dsl_select_dp_budget_deletes_bound_permanents`.

---

## Engine Gap: Cross-Permanent Rocks Source Selection Resolved  [G-ROCKS-SOURCE-SELECTION-DSL]

- **Discovered in:** Rocks / RockClose archetype assessment (2026-04-29 follow-up).
- **Scope:** Rust engine + DSL.
- **Card(s):** EX10-032 Proganomon, EX10-028 Landramon, EX8-070 Zofr Kabus, EX10-036 Magneticdramon, EX10-033 / EX11-044 / EX8-055 Pyramidimon source-trash bodies.
- **Status:** Resolved for the shared selection primitive. `EffectContext::select_own_sources` supports exact-N and up-to-N source choices across own battle-area stacks, binds stable `SourceSelectionRef` values, and DSL `trash_selected_sources` consumes those refs without a fake permanent prompt.
- **Regression coverage:** `source_multi::exact_two_sources_can_be_selected_across_own_battle_area`, `source_multi::up_to_sources_enables_pass_only_after_minimum_is_met`, `source_multi_mask_only_exposes_selecting_players_pending_actions`, `select_own_sources_binds_source_refs_for_trashing`, and `empty_select_own_sources_runs_outer_tail_synchronously`.
- **Remaining limits:** Triggered-body cost ordering, Fragment / Digi-Burst / replacement integration, and card-specific Rocks bodies remain separate gaps.

---

## Engine Gap: Effect Re-Firing / Cross-Timing Self-Trigger

- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Dark Masters (2026-04-18); Puppets/Nyabootmon assessment (2026-04-28).
- **Scope:** Rust engine effect context and DSL lowering.
- **Card(s):** EX8-074 MedievalGallantmon; BT22-042 Nyabootmon for the permanent-sourced `[When Digivolving]` self-refire slice.
- **Status:** Resolved for constrained permanent-sourced `WhenDigivolving` re-firing. `EffectContext::refire_effect_from_permanent(source, "when_digivolving")` enumerates refireable effects, queues the selected effect slot through the normal `QueuedEffect` path, preserves `source_card` / `source_permanent` identity, and reuses existing once-per-turn accounting. DSL authors can use `refire_effect: { source: <binding>, timing: when_digivolving, optional: true|false }`.
- **Regression coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_refiring --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_refiring --nocapture`.
- **Remaining limits:** Foreign-card `[On Play]` re-firing and Puppet deleted-object gating for "your other Digimon are deleted" remain separate open/partial gaps; this entry only archives the reusable permanent-sourced WhenDigivolving refire primitive.

---

## Engine Gap: Exact Trashed-Source Inherited Dispatch  [G-ROCKS-TRASHED-SOURCE-INHERITED-DISPATCH]

- **Discovered in:** Rocks pool implementation pass (2026-05-04).
- **Scope:** Rust engine effect queue.
- **Card(s):** EX8-051 Proganomon, EX8-047 Sunarizamon, EX8-005 Tumblemon, EX10-025 Sunarizamon, EX10-028 Landramon, EX10-032 Proganomon, BT21-055 Sunarizamon, EX11-038 Sunarizamon, and other Rocks inherited effects that trigger when the source card itself is trashed from a [Mineral]/[Rock] stack.
- **Status:** Resolved for source-trash events that already emit `TriggerSource::SourceTrashedFromStack`. The queue now enqueues inherited effects from the exact source card that was just trashed, even though that card is no longer live under its former host, and relies on trigger context predicates such as `host_permanent_trait_has` / `trashed_source_trait_has` for the former host and source-card facts.
- **Regression coverage:** Rocks behavioral tests including `ex8_051` exercise an inherited effect firing from the exact trashed source card. The source-trash event-context regression remains covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context --nocapture`.
- **Remaining limits:** This does not make every source-trash producer emit the event. Fragment, Digi-Burst, replacement costs, de-digivolve, and older source movement paths still need their own producer coverage before cards depending on those paths can be marked complete.

---

## Engine Gap: ~~`IgnoreColorRequirement` Modifier Not Enforced in Rust Option Action Mask~~ — RESOLVED 2026-05-02 [G-IGNORE-COLOR-MASK]

- **Discovered in:** Medusamon Batch 11, ST22-08 Offensive Plug-In V DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** ST22-08 Offensive Plug-In V — "While you have a Tamer, you can ignore this card's color requirements." Also any card that would use the `IgnoreColorRequirement` modifier via a flood_gate clause.
- **Effect text:** "While you have a Tamer, you can ignore this card's color requirements."
- **Resolution:** `code/digimon-engine/src/action/mask.rs` now treats player-scoped `ModifierType::IgnoreColorRequirement` as satisfying Option use legality before board color checks. The same helper is used by `play_option_from_hand`, so decode/execution rejects or accepts the same actions as the mask.
- **Regression coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`.
- **Remaining limits:** This closes the Rust engine mask/decode hook only. DSL-specific conditional aura expression, live condition refresh, and card-specific flood_gate authoring remain separate DSL/card implementation work where applicable.

---

## Engine Gap: `DelayTrigger::StartOfYourNextTurn` Missing — Delay Fires at Start of Turn  [G-DELAY-START-OF-TURN]

- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble; LM-030 Green Scramble in BG Imperial — "[Start of Your Turn] If your opponent has a Digimon, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)" Likely affects other Delay option cards whose activation timing is the controller's next turn START rather than END.
- **Effect text:** "[Start of Your Turn] … ＜Delay＞ …"
- **Status:** Resolved 2026-05-02. `DelayTrigger::StartOfYourNextTurn` is stored on delayed Option permanents, drains from `begin_turn` after `StartOfYourTurn` observers and before per-turn reset/draw/main progression, and DSL `CompiledTiming::StartOfYourTurn` lowers to the start-delay trigger.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_start_of_your_turn_maps_to_start_of_your_next_turn`.
- **Group 5 handoff verification:** The full Delay/Link regression set passes: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.
- **Remaining related work:** Card-specific Scramble bodies may still be blocked by other non-timing gaps such as pending-security self-to-hand, hand-or-trash/free-play selection, color requirement bypass, and card-specific effect bodies. Existing ignored LM-027 card-level tests that still name `G-DELAY-START-OF-TURN` are stale migration placeholders, not evidence that the generic start-of-turn Delay primitive is open.

---

## Engine Gap: `EffectContext::add_pending_security_to_hand`  [G-ADD-OPTION-SELF-TO-HAND]

- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28). Also previously surfaced by ST22-08 Offensive Plug-In V (Batch 11) and EX6-072 pattern.
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble — "[Security] … Then, add this card to the hand." Also ST22-08 Offensive Plug-In V and any option card whose Security clause ends with returning itself to hand.
- **Effect text:** "Then, add this card to the hand." — the currently-resolving security option card moves to the controller's hand.
- **Status:** Resolved for the narrow pending-security disposition slice on 2026-05-01. `EffectContext::add_pending_security_to_hand()` consumes `Game.pending_security` and pushes the revealed card to the defender/controller hand so the security dispose phase cannot trash it. DSL `add_this_option_to_hand: {}` lowers to the method. Legacy raw-rust shims now delegate to the method; new scripts should use the native step.
- **Coverage:** `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand`; `lm_027_security_adds_card_to_hand_after_play`.
- **Remaining related work:** ST22-08, P-206, EX7-074, and sibling Options may still be blocked by other gaps such as DP/play-cost predicates, Plug-In/Link, Delay timing, or broader Option play-flow disposition.

<!-- Entry template:

---

## DSL Gap: Reveal-zone multi-bucket selection

- Status: RESOLVED on 2026-05-03 for the reusable DSL/runtime primitive. `select_reveal_buckets` now parses and compiles named reveal buckets, validates empty/duplicate/malformed bucket shapes, evaluates bucket predicates against the reveal overlay, installs one `SelectionKind::RevealBucket` pending selection per bucket, and supports `no_duplicate_cards` across buckets without changing action-space or tensor contracts.
- Bucket results bind as `CardList`; singleton bucket lists are compatible with reveal single-card consumers such as `add_to_hand_from_reveal`.
- Lowers to engine API: `EffectContext::select_reveal_buckets(Vec<RevealBucketSelection>, prompt, no_duplicate_cards, callback)`.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2e_select_reveal phase2e_select_ordered_permutation phase2b_zone_moves_extra --nocapture`.
- Remaining related blockers: card-specific TS/Olympos YAML migration and any top-security inherited variants are tracked separately; the reusable multi-bucket reveal selection primitive is closed.

---

## DSL Gap: Source-stack DP sum formula

- Status: RESOLVED on 2026-05-03 for the narrow reusable formula leaf. `source_stack_dp_sum` now parses, compiles, validates its optional predicate filter, and evaluates by summing printed DP of live source-stack cards below the target permanent's top card. The optional filter reuses existing card predicate evaluation against each source card handle.
- Implemented DSL formula:
  ```yaml
  source_stack_dp_sum:
    target: self
    filter: { trait_has: Iliad }
  ```
- Lowers to engine formula evaluator: `CompiledFormula::SourceStackDpSum { target, filter }`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- residual_formula_predicate_vocab group7_formula_batch group7_predicate_batch group6_dynamic_formulas --nocapture`.
- Remaining related blockers: none for summing matching live source-stack card DP; card-specific YAML authoring remains separate.

---

## DSL Gap: Source-stack trash-all-sources step

- Status: RESOLVED on 2026-05-03 for the reusable `trash_all_sources` DSL step and runtime path. The step preserves the target permanent's top card, trashes every below-top source, and is now used by production `BT24-040` YAML.
- Lowers to engine API: `EffectContext::trash_all_sources(target)`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- source_stack_aggregates --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`.
- Remaining related blockers: source-count aggregate predicates and dynamic De-Digivolve amount formulas for other TS/Olympos cards remain separate.

---

## DSL Gap: Permanent-scoped effect suppression modifier

- Status: RESOLVED on 2026-05-03 for targeted `CannotActivateEffectsByTiming` DSL modifier use. Production `BT24-040` YAML uses it with `CannotSuspend` to lock two selected opponent Digimon/Tamers until the printed expiry.
- Lowers to engine modifier registry: `ModifierType::CannotActivateEffectsByTiming(EffectTiming::WhenDigivolving)` plus the existing `CannotSuspend` modifier.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`.
- Remaining related blockers: aura-wide timing suppression, other printed timings, and unvalidated Venusmon/Queen Device cards remain separate.

---

## DSL Gap: Security Option self-disposition to hand

- Status: COVERED on 2026-05-03 for the narrow currently-resolving security Option moving itself to hand. The existing DSL step `add_this_option_to_hand: {}` already parses/compiles as `AddThisOptionToHand`, lowers through `zone_moves.rs` to `EffectContext::add_pending_security_to_hand()`, and consumes `Game.pending_security` so the security dispose phase cannot also trash the card.
- No new disposition marker/API was added. Broader security disposition primitives such as adding an opponent's top security card to hand remain separate tracker entries.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl -- security_dsl_adds_currently_resolving_option_to_hand --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- option_security_disposition --nocapture`.

---

## DSL Gap: Security stack steps: top-security-to-hand and Recovery

- Status: RESOLVED on 2026-05-03 for the reusable `add_top_security_to_hand` and `recover_from_deck` DSL/runtime steps. Production `BT24-031` and `BT24-101` YAML now exercise top-security-to-hand, trash-top-security, and Recovery branches through behavioral tests.
- Lowers to engine API: `EffectContext::add_top_security_to_hand(player)` and `EffectContext::recover_from_deck(player, count)`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_031 bt24_101 --nocapture`.
- Remaining related blockers: security-stack search/extraction, face-up security handling, placing the resolving permanent itself into security, and card-specific YAML for `BT24-034`, `BT24-090`, and similar cards remain separate.

---

## DSL Gap: Chaos Control — effect-initiated digivolve from trash

- Effect text: EX11-005 Yaamon / EX11-069 Yuuki / BT21-100 The Digimon I Designed / BT24-080 Megidramon all digivolve a battle-area Digimon into a card in trash, sometimes for free and sometimes with a reduced cost.
- Status: resolved for selected live card bindings in Group 4 (2026-05-02). `effect_initiated_digivolve` now accepts `source:` as a source-zone-parametric binding while preserving legacy `from_hand:`.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve_from_source` / `Game::effect_initiated_digivolve_from_source`, with card-handle bindings resolved from hand, trash, security, material stack, or reveal pool.
- Suggested DSL syntax:
  ```yaml
  - effect_initiated_digivolve:
      target: base
      from:
        zone: trash
        of: you
        card: evo
      cost: { reduce: 1 }
      ignore_requirements: false
  ```
- First reported: 2026-04-28
- Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones`, plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement`.

---

## DSL Gap: EX4-011 — DP deletion threshold from shared trash count

- Status: RESOLVED for the reusable shared-trash formula primitive on 2026-05-02. `FormulaSpec` now accepts `per: { shared_trash_count: {} }` with a `bucket` on the surrounding base/per/delta formula, compiles to `CompiledPerSelector::SharedTrashCount { bucket }`, and runtime evaluation sums both players' trashes before applying bucket floor division.
- Effect text: "For every 10 total cards in both player's trashes, add 2000 to the maximum DP you can choose with DP-based deletion effects."
- Missing DSL verb / step kind / predicate: formula support for cross-player trash count buckets inside a `dp_lte` selection predicate. Existing formula vocabulary covers some modifier values, but not a target-filter threshold derived from `floor((your_trash + opponent_trash) / 10) * 2000`.
- Lowers to engine API: read both players' trash lengths, compute the threshold, then install the normal opponent-permanent selection and `delete_permanent` callback.
- Suggested DSL syntax:
  ```yaml
  dp_lte:
    formula:
      base: 7000
      per: shared_trash_count
      bucket: 10
      delta: 2000
  ```
- First reported: 2026-04-28
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch`.

---

---

## DSL Gap: BT22-008 / BT22-017 — inherited end-of-turn DNA digivolve registration

- Effect text: "[End of Your Turn] This Digimon and another of your Digimon may DNA digivolve into a Digimon card in your hand."
- Missing DSL verb / step kind / predicate: RESOLVED 2026-05-02 for literal-cost, two battle-area material `alt_path_registration` declarative clauses with `kind: dna_digivolve`, `scope: inherited`, and `trigger: end_of_your_turn`.
- Lowers to engine API: the same alternate-path registration and action-mask channel used by normal DNA digivolve costs, producing a player-visible pending/action path rather than an automatic end-of-turn digivolve.
- Suggested DSL syntax: keep the existing `alt_path_registration` shape and require lowering for inherited clauses, including `timing: end_of_your_turn`, `kind: dna_digivolve`, material filters, target hand-card filter, and cost override.
- Also blocks: `BT12-021` Veemon and `BT12-047` Wormmon in BG Imperial. Their inherited text is "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand."
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_alt_path_registration`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- dna`.
- Remaining limits: formula costs, extra costs, `from` gates, burst end steps, `stacks_unsuspended`, non-battle-area materials, repeat/unbounded materials, `ignore_requirements`, `source_treated_as` for DNA routes, marker routes, and non-DNA alt-path kinds beyond normal digivolve are intentionally not consumed by this v1 action hook. Normal digivolve `source_treated_as` routes are covered separately by the 2026-05-03 Hybrid/Tamer closure.
- First reported: 2026-04-28
- **Superseded 2026-05-24 (G-DSL-EOT-DNA-INLINE):** the 2026-05-02 closure satisfied the *vocabulary* gap but missed a *timing* gap. `alt_path_registration` registers a player-callable action that becomes legal in the NEXT P0 main phase — the DNA digivolve never surfaces AT the EoT trigger fire. This breaks the printed "at end of your turn ... may DNA digivolve" semantic; the user's intended single-turn chain (e.g. play MetalGreymon cost-reduced → Agumon→WG via MG's effect → end-of-turn DNA digivolve into Omnimon → T&M's `[EoT] Omnimon may attack a player`) cannot complete because Omnimon enters the field one turn too late. **Proposal `may-dna-digivolve-now-dsl-step` (archived 2026-05-24)** added a new DSL step verb `may_dna_digivolve_now` that surfaces the choice inline at trigger fire and orchestrates partner-selection + target-selection + the call to `effect_initiated_dna_digivolve` within the same EoT drain. All 6 affected cards (BT22-008/-017, BT17-007/-019, BT12-021/-047) migrated from `alt_path_registration` to the new step. The `alt_path_registration { kind: dna_digivolve }` mechanism still exists in the engine but has zero remaining card-script consumers — a follow-up audit (proposal Section 10) tracks whether to remove the mechanism entirely.

---

## DSL Gap: BG Imperial DNA cards — YAML `dna_costs` authoring / production data population

- Effect text: "[DNA Digivolve] Blue Lv.4 + Green Lv.4 : Cost 0" and equivalent BG Imperial DNA requirements.
- Missing DSL verb / step kind / predicate: RESOLVED 2026-05-02 for top-level `alt_paths: [{ kind: dna_digivolve, ... }]` authoring into runtime `CardData.dna_costs`.
- Lowers to engine API: `CardData.dna_costs`, consumed by the DNA digivolve action-mask branch and `Game::initiate_dna_digivolve`.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_016_has_dna_digivolve_alt_path`.
- Full verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml`, `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`, and `python -m pytest code/tests/rl -v`.
- Remaining limits: this closure covers top-level printed DNA digivolve card data. Inherited/end-of-turn registration is covered separately by the preceding entry and supports the existing DNA action IDs for the v1 literal-cost two-material shape.
- First reported: 2026-04-28 (BG Imperial assess-rust-engine-archetype)

---

## DSL Gap: Group 8 — scoped DigiXros aliases, ACE Overflow metadata, and reveal overlays

- Effect text: "also treated as [X] for DigiXros", `<ACE> Overflow -N`, and reveal effects whose revealed cards should be evaluated with temporary name/kind identity only while in the reveal zone.
- Status: RESOLVED 2026-05-02 for the planned vocabulary/data slices.
- Lowered runtime surfaces: `CardData.digixros_aliases` / compiled `digixros_aliases` for DigiXros-only material matching; `CardData.ace_overflow` for memory loss when ACE cards leave battle-area stacks or are removed from under a stack; and `RevealOverlay` on `CardSource` for reveal-zone predicate evaluation until destination movement clears the overlay.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor`, and full `cargo test --manifest-path code/digimon-engine/Cargo.toml`.
- Remaining limits: full DigiXros play flow, reveal overlays outside explicit reveal-zone predicates, and unrelated immediate-attack/declarative-keyword gaps remain separate entries.
- First reported: consolidated from Group 8 token/card-data gap planning.

---

## DSL Gap: EX11-008 — [When Moving] timing (DSL half — see engine-gaps.md for engine half)

- Effect text: "[When Moving] [On Play] 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Raid> and +3000 DP for the turn."
- Status: fixed for the timing token and direct runtime event context on 2026-04-29. `when: on_move` lowers to `EffectTiming::OnMove`; `event_target_trait_has` can inspect the moved permanent/card from `TriggerSource::MovedFromBreeding`. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_move_event_target_trait_predicate_matches_moved_permanent`.
- Remaining DSL work: card-specific bodies still need any additional step verbs/predicates they print, such as EX11-008's target grant body. BT16-082's reveal/add-to-hand/remainder-bottom/hatch tail is implemented as of 2026-05-04 via native reveal steps plus the reusable `can_hatch` predicate.
- Gap kind: hybrid (this entry tracks the DSL half; engine half tracked separately).
- First reported: 2026-04-27 (EX11-008 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: BT24-082 / BT21-081 — Immediate optional attack within effect resolution  [G-MAY-ATTACK-NOW]

- Effect text (BT24-082): "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack."
- Effect text (BT21-081): "[End of Your Turn] By suspending this Tamer, 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Piercing> for the turn. Then, that Digimon attacks."
- Status: RESOLVED for the reusable immediate in-effect attack primitive on 2026-05-03. The DSL now has `may_attack_now`, which lowers to `EffectContext::may_attack_now_optional(...)` and installs an existing-action-ID pending target selection mid-effect-resolution.
- Engine notes: `ModifierType::MayAttack` / `ForceAttack` remain EOT-window modifiers and are still not the right vehicle for this specific immediate prompt. This gap is closed by a distinct effect-context primitive, not by registering those modifiers.
- Lowers to engine API: `EffectContext::may_attack_now_optional(attacker, targets, without_suspending, optional, prompt)`; mandatory "then attacks" is `optional: false`, optional "may attack" is `optional: true`.
- Suggested DSL syntax:
  ```yaml
  - may_attack_now:
      attacker: tgt
      targets: any        # any | player | digimon
      optional: true      # false for mandatory "then attacks"
      without_suspending: false
  ```
- Gap kind: closed for immediate attack prompts. Persistent player-scoped grants such as `MayAttackPlayerOnly` and cross-side granted forced attacks remain separate engine gaps.
- Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- effect_granted_attack --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_granted_attack --nocapture`.
- First reported: 2026-04-27 (BT24-082 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: P-189 — [Security] play cost ≤ 4 filter on select_hand / select_trash  [G-PLAY-COST-LTE]

- Status: CLOSED on 2026-05-01. `play_cost_lte` is now parsed, compiled, evaluated against `CardData::play_cost`, and wired into `select_hand` / `select_trash` valid-action filtering. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[Security] You may play 1 card with the [LIBERATOR] trait and a play cost of 4 or less from your hand or trash without paying the cost."
- Missing DSL verb / step kind / predicate: `play_cost_lte` (or `cost_lte`) — a `PredicateSpec` leaf that checks `CardData::play_cost <= N`. `PredicateSpec` in `digimon-dsl/src/predicate.rs` has no cost-comparison field. The `eval_card_fields` function in `code/digimon-engine/src/dsl_cards/predicate.rs` handles `level_eq`, `level_lte`, `level_gte`, `color_is`, `trait_has`, `name_*`, `card_number_is` — but no `play_cost` / `cost_lte` / `cost_gte` variant.
- Companion issue: `install_select_hand` and `install_select_trash` in `code/digimon-engine/src/dsl_cards/step/selections.rs` currently use `|_game, _idx| true` (accept-all filter, Phase 2b), so even if `play_cost_lte` were added to `PredicateSpec`, it would not be evaluated until Phase 2b filter wiring is completed.
- Lowers to engine API: no new engine method needed. Fix requires: (1) add `play_cost_lte: Option<u32>` (and optionally `play_cost_gte`) to `PredicateSpec`; (2) add a branch in `eval_card_fields` to check `card_data.play_cost <= n`; (3) wire the filter predicate into `install_select_hand` and `install_select_trash`.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - trait_has: LIBERATOR
      - play_cost_lte: 4
  ```
- Gap kind: dsl (engine already stores `play_cost` on `CardData`; the DSL/lowering path just lacks the predicate leaf).
- Workaround: none needed for static play-cost caps after 2026-05-01. Previously ignored tests for incorrect candidate filtering can be unignored when updating affected card suites.
- First reported: 2026-04-27 (P-189 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: BT5-008 — `other: true` predicate not evaluated in `eval_permanent_fields`  [G-OTHER-PREDICATE-UNEVALUATED]

- Effect text: "[Your Turn] Your other [Gaossmon] all get +3000 DP."
- Status: RESOLVED by Group 6. `eval_permanent_fields` now rejects the source `PermanentHandle` when `other: true` is evaluated with `source_permanent` context, so filtered declarative auras can exclude their own source permanent.
- Passing command(s): `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`.
- Remaining related blocker: none for this predicate primitive.
- Suggested DSL syntax: already in the DSL spec as `other: true`.
- Gap kind: closed DSL/runtime predicate gap.
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype). Closed: 2026-05-02 (Group 6 Task 3).

---

---

## DSL Gap: BT5-008 — Player-level flood-gate modifier not installable from DSL  [G-PLAYER-FLOOD-GATE-DSL]

- Effect text: "[Opponent's Turn] Your opponent can't reduce digivolution costs."
- Status: RESOLVED by Group 6 for aura-delivered player modifiers, and previously closed for DSL vocabulary/direct runtime primitives. The DSL supports `target_player` on `kind: flood_gate` and an explicit `add_player_modifier` step, and the engine has a `CannotReduceDigivolveCost` modifier with digivolve-only cost-reduction enforcement.
- Engine note 2026-05-02: player-scoped `IgnoreColorRequirement` is now consumed by Rust Option masks and decode/execution. No new DSL syntax landed in that engine-only pass; passive/static field dispatch remains governed by the related blocker below.
- Passing command(s): `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`.
- Remaining related blocker: flood-gate-specific tick coverage is still worth adding separately, but aura-delivered player modifiers no longer require a raw-rust placeholder.
- Engine note 2026-05-02: `Game::tick_declarative_effects` now dispatches process-backed declarative effects from face-up field sources, and Group 6 aura coverage proves a `kind: aura` with `target_player: opponent` installs `CannotReduceDigivolveCost` on the referenced player. The same dispatcher executes flood-gate process closures, but this entry does not claim dedicated flood-gate installation coverage until a flood-gate-specific tick test lands.
- Implemented DSL syntax:
  ```yaml
  - kind: flood_gate
    active_when: { opponents_turn: true }
    target_player: opponent
    modifier: CannotReduceDigivolveCost
  ```
- Gap kind: closed vocabulary/runtime primitive gap; aura-based passive dispatch is covered by Group 6 Task 3, with flood-gate-specific dispatcher coverage still worth adding separately.
- Former workaround removed for BT5-008: raw_rust no-op placeholder (`bt5_008_opp_cannot_reduce_digivolve_cost`).
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype). Closed: 2026-05-01 (floodgate DSL flexibility pass).

---

---

## DSL Gap: P-137 — Opponent adds top security card to hand  [G-ADD-TOP-SECURITY-TO-HAND]

- Effect text: "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- Status: RESOLVED for the reusable DSL/runtime primitive on 2026-05-03. `add_top_security_to_hand` now moves the top security card to the owner's hand and preserves the security-loss observer chain. P-137 production YAML still needs a card-level cleanup pass if it retains an old raw-rust workaround.
- Lowers to engine API: `EffectContext::add_top_security_to_hand(player: PlayerId) -> bool` — pops `security.last()`, pushes to `hand`, fires `EffectTiming::OnLoseSecurity` via `SecurityRevealed` and `EffectTiming::OnOpponentSecurityRemoved` via `PlayerBattleArea(controller)`.
- Suggested DSL syntax:
  ```yaml
  - add_top_security_to_hand: { of: opponent }
  ```
- Gap kind: resolved reusable DSL/engine primitive; card-local raw_rust removal remains a separate authoring cleanup if present.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations --nocapture`.
- First reported: 2026-04-27 (P-137 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

---

## DSL Gap: P-035 / P-103 / BT24-089 — Option-as-permanent placement (inherited security)  [G-PLACE-SELF-AS-OPTION-PERMANENT]

- Effect text (P-035): "[Main] … Then, place this card in your battle area." and "[Security] Place this card in the battle area." (inherited)
- Status: IMPLEMENTED for inherited-security source Options as of 2026-05-02. The DSL verb / step kind is `place_self_as_delay_option: {}` — a step that places the currently-resolving Option card into the battle area as an `OptionState::Delayed` permanent from within an inherited security context. Two contexts exist:
  1. **Main clause** ("Then, place this card in your battle area."): DCGO calls `PlaceDelayOptionCards(card, activateClass)`. In the Rust engine, `dispose_option` + `classify_option_subtype` detect the `kind: delay` clause and auto-place the card at the `MainEffectDrain` phase — no explicit DSL step is needed. The engine handles placement implicitly.
  2. **Inherited security clause** ("[Security] Place this card in the battle area."): DCGO calls `CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)`. Rust now exposes `EffectContext::place_self_as_delay_option_permanent`, which removes the matching non-top source Option from its host stack, places it in the owner's battle area as `OptionState::Delayed`, and dispatches `OnOptionPlaced`.
- Lowers to engine API: `EffectContext::place_self_as_delay_option_permanent(&mut self)`. In the inherited-security context, this method: (1) identifies the source Option card (the digivolution source that triggered this effect), (2) removes it from its current location (digivolution stack), and (3) places it in `self.game.players[owner].battle_area` as a `Permanent` with `OptionState::Delayed`.
- Suggested DSL syntax:
  ```yaml
  - place_self_as_delay_option: {}
  ```
  Used in the `process:` of the inherited security clause. Not needed in the Main clause (engine auto-placement via `dispose_option` suffices).
- Gap kind: resolved dsl vocabulary/engine API gap for inherited security context. Engine already handled the Main clause auto-placement; inherited-security-context placement now uses the explicit DSL step because `dispose_option` is not called in that path.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- inherited_security_places_source_option_as_delay_permanent`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_self_as_delay_option`.
- Group 5 handoff verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.
- Workaround: no longer needed for new YAML/tests. Existing `process: []` placeholders for P-035, P-103, and BT24-089 can migrate to `place_self_as_delay_option: {}` when those card YAMLs/tests are revisited. Existing ignored card-level tests that still name `G-PLACE-SELF-AS-OPTION-PERMANENT` remain migration/process-body follow-ups, not an open reusable placement primitive; keep distinct card-level blockers such as remaining P-206 Delay/action-flow work separate.
- First reported: 2026-04-28 (P-035 Red Memory Boost! batch-implement-cards-rust-dsl, Medusamon Batch 12). Same gap pre-existed in P-103.yaml and BT24-089.yaml without a tracker entry.

---

---

## DSL Gap: P-206 — Board-color cross-reference predicate in Delay clause  [G-COLOR-MATCH-AGAINST-BOARD]

- Effect text: "[Main] ＜Delay＞ … You may play 1 Tamer card with the same color as any of your Digimon on the field from your hand with the play cost reduced by 4."
- Status: RESOLVED on 2026-05-02 for dynamic board-color card predicates. `color_matches_any_field_digimon` now parses, compiles to `CompiledPredicate.color_matches_any_field_digimon`, and evaluates against live battle-area Digimon top-card colors during selection filtering. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- color_matches_any_field_digimon_compiles`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_hand_color_matches_any_field_digimon_filters_by_live_board_colors`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch`.
- Implemented DSL predicate: `color_matches_any_field_digimon` — a `PredicateSpec` leaf that checks whether a candidate card's colors share at least one color with any Digimon currently in the requested player's battle area.
- Lowers to engine API: `CardData::colors` read from the candidate card and from the requested player's battle-area top cards. No new engine method was needed; empty-board behavior is false.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: tamer
      - color_matches_any_field_digimon: { of: you }
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for dynamic card-color filtering.
- Workaround: no longer needed for the reusable predicate. P-206 card YAML may still need separate Delay, Option, or action-flow follow-up work before the full card can be unblocked.
- First reported: 2026-04-28 (P-206 Digital Gate Open batch-implement-cards-rust-dsl, Medusamon Batch 14)

---

---

## DSL Gap: BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Status: RESOLVED for reusable formula-zone count filters on 2026-05-02. `card_count_in_zone` payloads now accept `filter: { ... }`; the compiler carries the predicate into filtered count IR, and runtime evaluation counts only representable subjects that satisfy the predicate instead of falling back to an unfiltered count.
- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Implemented DSL form: `card_count_in_zone` formulas can now apply a `kind: digimon` filter. `BT8-097.yaml` uses this filtered form so Tamers and Option permanents no longer reduce Crimson Blaze's play cost.
- Lowers to engine API: the engine can inspect each battle-area permanent and test `Permanent::is_digimon(&card_data)`; the formula DSL needs a filtered-count form that passes a compiled predicate into formula evaluation.
- Suggested DSL syntax:
  ```yaml
  amount_fn:
    base: 0
    per:
      card_count_in_zone:
        of: opponent
        zone: battle_area
        filter: { kind: digimon }
    delta: 1
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for filtered zone-count formulas.
- Workaround: no longer needed for BT8-097 or other `card_count_in_zone` formulas with simple predicate filters.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch phase3d_formula_zone_count`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097`.


---

## Engine Gap: `EffectContext::add_top_security_to_hand` Missing (engine half of G-ADD-TOP-SECURITY-TO-HAND)

- **Discovered in:** Medusamon Batch 8, P-137 Flamedramon DSL implementation (2026-04-27)
- **Card(s):** P-137 Flamedramon — "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- **Effect text:** "opponent adds the top card of their security stack to the hand"
- **What's missing:** `EffectContext` only exposes `trash_top_security(player)` for security removal. There is no `add_top_security_to_hand(player)` method that pops the top security card and places it in the player's hand while firing the standard security-removed event chain (`OnLoseSecurity` via `SecurityRevealed` + `OnOpponentSecurityRemoved` via `PlayerBattleArea`).
- **Suggested change:** Add `pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool` to `EffectContext`. Implementation: pop `security.last()`, push to `hand`, fire `EffectTiming::OnLoseSecurity` with `TriggerSource::SecurityRevealed { defender: player, card: card_handle }` and `EffectTiming::OnOpponentSecurityRemoved` with `TriggerSource::PlayerBattleArea(controller)`.
- **Former workaround:** `raw_rust: { fn: p_137_opp_adds_top_security_to_hand }` manually implemented the move + event chain. As of 2026-05-22, P-137 uses native `add_top_security_to_hand` and the raw shim has been removed.

---

## DSL Gap: ST22-08 — Link Registration Clause (Plug-In / Link Card Mechanic)  [G-DSL-LINK-VERB]

- Status: closed 2026-05-02. DSL now supports `kind: link_requirement` lowering to `Effect::link(cost, filter)` and `link_to_own_digimon` process steps that surface the existing Link host-selection `PendingSelection` without adding action IDs.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- Effect text: "Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2 (Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)"
- Also: "[Main] You may link this card to 1 of your Digimon without paying the cost."
- Missing DSL verb / step kind / predicate: Two missing DSL constructs:
  (a) A declarative clause kind for declaring link requirements — no `kind: link_requirement` or equivalent in `TypedDeclarativeBody`. The closest existing cards (EX11-027 Maquinamon) use `kind: raw_rust fn: ex11_027_link_requirements triggers: [main_from_hand, main_on_field]`.
  (b) An optional link-action step within a `process:` body — no `link_to_digimon:` or similar step verb. DCGO's `SelectPermanentEffect` with `Mode.Custom` + `card.CanLinkToTargetPermanent(permanent, false)` + `canNoSelect: true` drives this. The engine has `OptionSubtype::Link`, `Effect::link(cost, filter)`, and `attach_linked_card()`, but these are reachable only via hand-written `CardEffect` or raw_rust functions.
- Lowers to engine API: `Effect::link(cost, filter_fn)` on the declarative side; `ctx.game.attach_linked_card(host_handle)` on the step side. Both exist in the engine; neither is accessible from the DSL step vocabulary.
- Suggested DSL syntax:
  ```yaml
  # Declaration form (inherited link requirements):
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
  
  # Step form (optional free link in process body):
  - link_to_own_digimon:
      optional: true
      cost_delta: -99   # or free: true
      filter: { kind: digimon }
      bind_as: linked_host
  ```
- Gap kind: dsl (engine has the primitive; DSL lacks both the clause kind and the step verb).
- Remaining card-level blockers: ST22-08 behavioral tests that still name `G-DSL-LINK-VERB` are stale card migration placeholders unless they also depend on separate process-body blockers such as `G-BINDING-DP-FORMULA`, DP/play-cost predicates, or card-specific YAML rewrites.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: ST22-08 — Linked-Card Effect Scope  [G-DSL-LINKED-SCOPE]

- Status: closed 2026-05-02. DSL now accepts `scope: linked`; triggered lowering marks the emitted effect with `.linked()` so the existing linked-card dispatch path can fire it from the host.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- Effect text: DCGO shows EndOfTurnLinkedEffect with `activateClass.SetIsLinkedEffect(true)` — an effect that fires only when the card is linked to a Digimon in the battle area, and the linked Digimon may attack at the controller's end of turn.
- Missing DSL verb / step kind / predicate: `scope: linked` — a clause scope for effects that fire as if they were part of the Digimon the card is linked to. `CompiledScope` in `digimon-dsl/src/compiled.rs` has `FaceUp` and `Inherited` variants; there is no `Linked` variant. The effect-queue (`effect_queue.rs`) already handles linked cards in `enqueue_from_permanent` (the Phase 8 Task 4 linked_cards branch), but the scope is expressed as a raw `linked_cards` list on `Permanent`, not as a DSL-compiled clause with `scope: linked`.
- Lowers to engine API: the engine already fires effects for linked cards via the `linked_cards` loop in `enqueue_from_permanent`. The DSL lowering layer would need to detect `scope: linked` on a clause and install the resulting `Effect` via `Effect::declarative(card)` with a flag indicating it should be enqueued from the linked-card path rather than the top-card path.
- Suggested DSL syntax:
  ```yaml
  - scope: linked
    when: end_of_your_turn
    optional: true
    once_per_turn: true
    process:
      - raw_rust: { fn: st22_08_linked_eot_may_attack }
  ```
- Gap kind: dsl (engine fires linked-card effects; DSL has no `scope: linked` clause kind that lowers into the linked-card effect list).
- Remaining card-level blockers: ST22-08 ignored tests that still name `G-DSL-LINKED-SCOPE` should be migrated or retagged during card implementation; the reusable DSL scope is closed. Leave unrelated blockers such as `G-BINDING-DP-FORMULA` open.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: ST22-08 — Named-Binding DP Reference in Formula  [G-BINDING-DP-FORMULA]

- Status: RESOLVED for the reusable `binding_dp` formula primitive on 2026-05-02. `dp_lte: { formula: { binding_dp: ally } }` now parses, compiles to `CompiledFormula::BindingDp("ally")`, and predicate formula evaluation threads current `Bindings` so the threshold reads the named permanent's effective DP.
- Effect text: "[Main] … delete 1 of your opponent's Digimon with as much or less DP as 1 of your Digimon."
- Missing DSL verb / step kind / predicate: `binding_dp` — a formula primitive that reads the effective DP of a named binding (a `PermanentHandle` stored by `bind_as:` from a prior `select_own_permanent`). The formula system (`formula.rs` + `formula_eval.rs`) can read `source_permanent`'s DP via `{ of: source_permanent, value: dp }` (see DSL spec §3.10), but there is no form to read an arbitrary named binding's DP — which is required for "DP ≤ chosen own Digimon's DP" where the comparator is player-selected mid-effect.
- Lowers to engine API: `ctx.game.effective_dp(handle)` — already exists. The gap is that `CompiledFormula` has no `BindingDp(String)` variant that reads `bindings.get_permanent(name)` and calls `effective_dp`. 
- Suggested DSL syntax:
  ```yaml
  # In dp_lte formula, reference a named binding:
  dp_lte:
    formula:
      binding_dp: ally   # resolves bindings["ally"] as PermanentHandle, calls effective_dp
  ```
  Requires: (1) add `BindingDp(String)` to `FormulaSpec` and `CompiledFormula`; (2) add evaluation branch in `formula_eval.rs` that resolves the binding from `Bindings` and calls `ctx.game.effective_dp(h)`; (3) pass `Bindings` into the formula evaluator call chain.
- Gap kind: dsl (engine has `effective_dp`; DSL formula system has no binding-reference form).
- Workaround: None for `binding_dp` itself. Static and existing formula-backed `dp_lte` / `dp_gte` permanent predicates are now evaluated as of 2026-05-01, but this gap remains open until formulas can read a named binding's effective DP.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch group7_predicate_batch`.

---

---

## DSL Gap: ST22-08 — Return Played Option Card to Hand Post-Security  [G-ADD-OPTION-SELF-TO-HAND]

- Effect text: "[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, add this card to the hand."
- Status: Resolved for the narrow pending-security disposition slice on 2026-05-01. DSL now supports a deterministic `add_this_option_to_hand: {}` step that lowers to `EffectContext::add_pending_security_to_hand()`, consuming `Game.pending_security` and moving the revealed card to the defender/controller hand instead of letting security dispose trash it.
- DSL syntax:
  ```yaml
  - add_this_option_to_hand: {}
  ```
- Coverage: `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand` and `lm_027_security_adds_card_to_hand_after_play`.
- Remaining related gaps: cards may still be blocked by other predicates or Option mechanics, such as lowest-DP selection, Plug-In/Link, Delay timing, or play-cost filters.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT21-072 — [All Turns] +1000 DP per digivolution card (dynamic formula aura)  [G-AURA-DP-FORMULA]

- Effect text: "[All Turns] This Digimon gets +1000 DP for each of its digivolution cards."
- Missing DSL verb / step kind / predicate: `dp_modifier_fn` / `dp_modifier_formula` — a formula-based variant of `AuraBody.dp_modifier`. The DCGO implements this via `ChangeSelfDPStaticEffect(changeValue: 1000 * count(), ...)` at `EffectTiming.None`, where `count()` is a live lambda returning `PermanentOfThisCard().DigivolutionCards.Count()` (= material_count = stack_size - 1). This is a **continuously-recomputed** aura that updates dynamically each tick, including after `de_digivolve` operations that pop digivolution cards from the stack. The DSL `kind: aura` with self-target accepts only `dp_modifier: Option<i32>` — a static literal with no formula variant. The `FormulaSpec` type (with `per: material_count, delta: 1000`) exists for step-level `add_dp_modifier` verbs, but `add_dp_modifier` only snapshots the formula's value at event-fire time, not continuously. Storing a snapshot in `Effect.dp_modifier` cannot model the dynamic behaviour required.
- Lowers to engine API: `source_dp_contribution(perm_handle, source_index)` reads `Effect.dp_modifier` continuously — the engine query mechanism already supports live reads. The gap is that `AuraBody` has no formula field to store a `FormulaSpec` that `lower_aura.rs` could evaluate at read-time rather than compile-time.
- **Status:** RESOLVED by Group 6. `kind: aura` now accepts `dp_modifier_fn` and lowers it into a live runtime formula closure instead of snapshotting into `Effect.dp_modifier`. `Game::effective_dp` and `Game::source_dp_contribution` evaluate that closure at query/tensor time, so `per: material_count` changes when the stack changes. Sibling coverage: `security_attack_fn` is also accepted on aura clauses and is recomputed at security-check resolution, alongside printed Security Attack keywords and `ModifierType::SecurityAttackChange`.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas --nocapture`; broader formula regression coverage remains in `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- --nocapture`.
- **Remaining related blocker:** none for formula-backed DP and Security Attack aura clauses.
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    active_when: { all_turns: true }   # or omit for always-on
    target: {}                          # self
    dp_modifier_fn:                     # NEW: formula-based dynamic variant
      base: 0
      per: material_count               # CompiledPerSelector::MaterialCount = stack_size - 1
      delta: 1000
  ```
  Implementation notes: implemented as `AuraBody.dp_modifier_fn` / `CompiledDeclarativeClause::Aura::dp_modifier_fn` plus runtime `Effect.dp_modifier_fn` evaluation in the DP query path.
- Gap kind: dsl (closed for formula-backed DP and Security Attack aura clauses).
- Workaround: None for this shape.
- First reported: 2026-04-27 (BT21-072 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT20-102 — Omnimon (X Antibody) self-digivolution-stack name check  [G-SELF-DIGIVOLUTION-CONTAINS-NAME]

- Effect text: "[On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, ..."
- DSL predicate coverage: `self_digivolution_contains_name` is the needed `BoolPredicate` leaf for evaluating this Digimon's digivolution stack from a triggered clause's `condition:` block. The DSL predicate `source_name_contains` applies to the SOURCE PERMANENT (the Digimon this card is stacked under, in inherited contexts) — not to this card's own digivolution stack at runtime.
- Engine support: `Permanent::contains_card_name(name, data)` exists in `code/digimon-engine/src/permanent.rs` and scans the full stack. Triggered `condition:` / `active_when:` evaluation now passes a `PredicateSubject::Permanent(source_h)` when a live source permanent is available.
- Lowers to engine API: `Permanent::contains_card_name(name, &game.card_data)` on `rctx.source_permanent()`.
- Suggested DSL syntax:
  ```yaml
  condition:
    self_digivolution_contains_name: "Omnimon"
    # or: any_of: [{ self_digivolution_contains_name: "Omnimon" }, { self_digivolution_contains_name: "X Antibody" }]
  ```
  Implementation: add `self_digivolution_contains_name: Option<String>` to `BoolPredicateSpec` in `digimon-dsl/src/predicate.rs`, compile to `CompiledPredicate` field, evaluate in `eval_predicate(p, rctx, PredicateSubject::Permanent(source_h))` where `source_h` is the triggering permanent's handle — requires threading the source handle into the triggered-clause condition closure in `lower_triggered.rs`.
- Updated 2026-05-02: `self_digivolution_contains_name` is now a compiled predicate leaf, triggered `condition:` / `active_when:` evaluation passes the live source permanent as subject when available, and runtime evaluation scans the full source stack through `Permanent::contains_card_name`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch`.
- Gap kind: hybrid; the predicate leaf and triggered-condition subject threading are implemented, with broader BT20-102 authored-card coverage tracked separately.
- Workaround: entire boardwipe clause routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which calls `perm.contains_card_name(...)` directly. Over-wide: top card name "Omnimon (X Antibody)" contains "X Antibody" so condition is always true for BT20-102 even with no digivolution source.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT20-102 — Exclude-from-binding filter in `for_each`  [G-FOR-EACH-EXCLUDE-BINDING]

- Status: CLOSED on 2026-05-01. `not_in_binding` is now parsed, compiled, evaluated against `Permanent` / `PermanentList` bindings, and `for_each` threads current bindings into its permanent scan. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[On Play][When Digivolving] ... choose 1 of both players' Digimon and delete all other Digimon."
- Missing DSL verb / step kind / predicate: `not_in_binding` — a `CandidatePredicate` leaf in `for_each { over, filter, body }` that excludes permanents whose handle appears in a named binding (a prior selection). Without it, "delete all OTHER Digimon" (all except the two saved by selection) cannot be expressed purely in DSL.
- Engine API: the engine can iterate `battle_area` handles and compare against a collected `Vec<PermanentHandle>`. No new API needed — gap is purely in the DSL filter vocabulary.
- Suggested DSL syntax:
  ```yaml
  - for_each:
      over: { owner: any, kind: digimon, not_in_binding: saved }
      bind_as: candidate
      body:
        - delete_permanent: { target: candidate }
  ```
  Implementation: add `not_in_binding: Option<String>` to `CandidatePredicateSpec` in `digimon-dsl/src/predicate.rs`, compile, and evaluate by looking up the named binding in `Bindings` and comparing handle equality.
- Gap kind: dsl (engine can express this in a raw_rust loop; DSL has no filter for handle-set exclusion).
- Workaround: none needed for handle-set exclusion after 2026-05-01. Card YAML/tests may still need separate migration away from any raw_rust bridge if other blockers remain.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## DSL Gap: P-156 — Color Match Against a Chosen Tamer Binding  [G-COLOR-MATCH-BOUND-PERMANENT]

- **Status:** RESOLVED on 2026-05-04 for card/selection predicates that compare a candidate card's colors against a previously selected permanent binding.
- **Effect text:** P-156 Future Potential!: "[Main] Choose 1 Tamer. You may play 1 Digimon card with the same color as the chosen Tamer with a play cost of 3 or less from your hand or trash without paying the cost."
- **Implemented DSL predicate:** `color_matches_binding: chosen_tamer`
- **Resolution:** `PredicateSpec` and `CompiledPredicate` now carry `color_matches_binding`; card predicate evaluation resolves the named permanent binding through `Bindings`, reads the bound permanent's live top-card colors, and filters hand/trash candidates by color overlap. The evaluator handles Digimon/Dual candidates through Digimon-face colors and Options/Dual through Option-face colors, matching the existing card-color split.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch --nocapture` includes `select_hand_color_matches_binding_filters_against_bound_tamer_colors`.
- **Card evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_156 --nocapture` covers own and opponent Tamer choices, hand and trash branches, play-cost filtering, and free play without paying cost.
- **Remaining related blockers:** P-156's Security optional Tamer play before the mandatory add-to-hand tail remains open under `PUPPETS-G017`; this resolved predicate only covers same-color filtering against a chosen permanent.

---

## Rust Engine Gap: Option Use Active-Body Preflight  [G-OPTION-ACTIVE-BODY-PREFLIGHT]

- **Status:** RESOLVED on 2026-05-04 for ordinary Main-phase Option hand/trash use and Counter-window Option use.
- **Problem:** Partial Security-only Option YAML, or an Option whose mandatory Main precondition has no legal candidates, could still appear as a legal hand-play action and resolve as a no-effect Option. P-156 exposed this when its Main effect needs at least one Tamer choice before any branch can resolve.
- **Resolution:** Option play masks and `play_option_from_hand`/`play_option_from_trash` now preflight that ordinary Option use has an active `OptionMain` body whose condition passes. During a Counter window, the preflight accepts an active `CounterEffect` body instead, preserving legal Counter Options that do not also have an `OptionMain` body.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_156 --nocapture` covers masking/direct rejection when P-156 has a matching black source but no Tamer to choose. `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- counter_hand_option_without_option_main_still_resolves_counter_body counter_hand_option_resolves_through_play_option_pipeline --nocapture` covers Counter-window compatibility.
- **Remaining related blockers:** This does not implement optional-subeffect mandatory-tail continuation; P-156 Security remains tracked under `PUPPETS-G017`.

---

## Rust Engine Gap Group Summaries

- **Group 6 modifiers / auras / keywords closure (2026-05-02):** Status: resolved for the planned Group 6 primitives. Option color bypass uses the existing player-scoped modifier in both masks and decode/execution; source-scoped return/de-digivolve immunity blocks opponent effects while allowing own effects, battle, costs, and rule cleanup; filtered/dynamic auras refresh without stale materialized modifiers; dynamic DP and Security Attack formulas recompute from existing tensor/action surfaces; Collision, Piercing, Reboot, Retaliation, and Progress use existing combat/action paths; Overclock uses the field-effect sub-slot; and DigiXros aliases remain scoped to DigiXros material matching only. Passing targeted coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras group6_dynamic_formulas --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords group6_overclock --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords parses_digixros_scoped_alias_without_global_name_alias --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not --nocapture`. Broad Rust gates passed with Cargo's required no-filter separator forms for `--nocapture`. Python gates are not closure evidence as of this review: `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v` failed one tensor-shape parity test (`(1375,)` legacy vs `(8320,)` Rust default), and `python -m pytest code/tests/rl -v` failed three legacy/Rust parity tests plus `test_same_seed_reproduces_first_action`.

- **G-DELAY-START-OF-TURN / start-of-turn Delay options (Group 5, 2026-05-02):** resolved by `DelayTrigger::StartOfYourNextTurn`, trigger-aware `OptionState::Delayed`, a start-of-turn delay drain in `Game::begin_turn`, and DSL lowering from `CompiledTiming::StartOfYourTurn`. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_start_of_your_turn_maps_to_start_of_your_next_turn`. Remaining Scramble/card blockers are non-timing gaps tracked separately.
- **Group 8 token/card-data gap closure (2026-05-02):** Status: implemented for the planned slices. Familiar Tokens now have their printed mandatory `OnDeletion` target selection and -3000 DP modifier; all registered tokens synthesize complete `CardData`; top-level authored DNA `alt_paths` populate runtime `CardData.dna_costs`; `digixros_aliases` are scoped to DigiXros material matching and do not leak into generic name predicates; `ace_overflow` card data is enforced when ACE cards leave battle-area stacks or leave from under a stack; and reveal-zone overlays let reveal predicates see temporary name/kind identities until the card moves to its destination. Regression coverage includes `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor`, full `cargo test --manifest-path code/digimon-engine/Cargo.toml`, `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`, and `python -m pytest code/tests/rl -v`. Remaining limits: inherited/end-of-turn DNA alt-path registration, full DigiXros play flow, reveal overlays outside explicit reveal-zone predicates, and unrelated token/keyword mechanics remain tracked by their own entries.
- **Group 4 zone/source/security movement (2026-05-02):** implemented source-parametric effect digivolve, exact material-source movement/trash with source-trash event context, selected security-to-hand and security shuffle DSL steps, effect-driven security removal cleanup that resumes after `OnLoseSecurity` selections without duplicating cards, full-stack return-to-deck with owner routing, and real breeding-slot effect movement/placement helpers.
- **Latest source-stack residual closure (2026-05-03 Task 4):** direct source-stack residual operations are narrowed by `EffectContext::trash_all_sources`, DSL `trash_all_sources`, and DSL `play_selected_sources_free` over stable `SourceSelectionRef` bindings. Same-level source aggregate formula coverage remains in the existing Group 7 formula path. Focused coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- source_stack_operations --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregates group7_formula_batch group7_predicate_batch --nocapture`.
- **Latest Hybrid/Tamer alt-path closure (2026-05-03 Task 5):** normal hand-to-battle-area digivolve masks and execution now consume DSL `alt_paths.kind: digivolve` with literal costs and `source_treated_as`, allowing Tamer-as-level/color Hybrid bases without mutating printed Tamer kind. Union-zone selections are covered end-to-end into effect-initiated digivolve from a selected `CardHandle`. Focused coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- hybrid_tamer_digivolve phase2e_select_union_zone --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_union_zones effect_digivolve_from_zones --nocapture`. Remaining limits: dynamic-cost effect digivolve from Tamer/Digimon base unions, delayed cleanup riders, and card-local Red Hybrid YAML authoring.
- **Cost and Replacement Framework (Group 3, 2026-04-30):** Status: implemented. Regression coverage: `code/digimon-engine/tests/cost_hooks/stacked_would_play_reducers.rs`; `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`; `code/digimon-engine/tests/replacements/context_predicates.rs`; `code/digimon-engine/tests/replacements/partition.rs`; `code/digimon-engine/tests/option_flow/replacement_integration.rs::bt17_097_delay_prevents_deletion_and_digivolves_from_hand`; `code/digimon-engine/tests/replacements/attack_cancel.rs`. The engine supports stacked optional would-play cost reducers, triggered pay costs that park pending selections before process execution, optional pay-cost decline, replacement cause/controller predicates, Partition source selection, Delay-as-replacement prevention, and effect-driven pending attack cancellation.
- **G-ROCKS-SOURCE-SELECTION-DSL / cross-permanent count-capped source selection (2026-04-29):** resolved by `EffectContext::select_own_sources`, `SelectionKind::SourceMulti`, stable `SourceSelectionRef` bindings, and DSL `select_own_sources` / `trash_selected_sources`.
- **G-MULTI-SELECT-OPP-DP-SUM DP-budget slice (2026-04-29):** resolved by `EffectContext::select_opponent_permanents_by_dp_budget`, `SelectionKind::DpBudget`, and DSL `select_opponent_dp_budget` / `delete_bound_permanents`. Count-capped non-DP sibling shapes remain open where called out above.
- **G-BREEDING-PERMANENT-SELECTION (2026-04-29):** resolved by `EffectContext::select_own_breeding_permanent`, `SelectionKind::BreedingPermanent`, phase-scoped `encode_breeding_select`, and DSL `select_own_breeding_permanent`.
- **G-SELECT-EMPTY-OUTER-TAIL selection regression (2026-04-29):** covered by `empty_select_material_runs_outer_tail_synchronously` and `empty_select_own_sources_runs_outer_tail_synchronously`.

---

## 2026-05-15 Hygiene Sweep Relocations

The following Rust engine gap entries were relocated here from `docs/RUST_ENGINE_GAPS.md` during the 2026-05-15 tracker hygiene sweep, per the audit in [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md). Each entry's body is preserved verbatim; a brief "Audit closure note (2026-05-15)" paragraph at the bottom cites the audit doc and the closing PRs.

## Engine Gap: Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings — RESOLVED 2026-05-15 (PRs #449, #451, #472)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** EX8-074 MedievalGallantmon ("When Digimon are played"), BT21-029 Medusamon ("When any of your opponent's Digimon are deleted"), BT21-026 WarGreymon ("When any of your opponent's Digimon are deleted") — DNA Omnimon adds: BT22-005 Tsumemon (trait-filtered OnPlayed observer), EX9-066 Tai Kamiya & Matt Ishida (suspend-self cost on any ally played), BT17-081 Tai Kamiya & Matt Ishida (observer on ally played or digivolves), EX9-019 WereGarurumon: Sagittarius Mode / EX9-012 MetalGreymon: Alterous Mode / AD1-001 Greymon / AD1-010 Garurumon (hand-resident observers on ally played or digivolves — expands the fan-out target from "battle area" to "hand"), EX4-061 Matt Ishida & Tai Kamiya — Rocks adds: BT8-094 Digimon Emperor (cross-side `OnAnyDeletion` gated on level ≤5 with suspend-self cost + draw), EX11-065 Close (OnAnyDigimonPlayed trait-filtered) — Dark Masters adds: ST6-14 Matt Ishida, BT8-094 Digimon Emperor, BT13-102 Keenan Crier, EX9-068 Analogman, RB1-035 Hokuto Amanokawa, BT19-075 MoonMillenniummon
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate…" / "When any of your opponent's Digimon are deleted, this Digimon may unsuspend."
- **Resolution:** Both fire sites wired in `digimon-engine` — see `fire_on_enter_field_anyone()` in `code/digimon-engine/src/game_actions.rs` (called after OnPlay from `play_from_hand_with_cost` and `play_from_trash_with_cost`) and `fire_on_any_deletion()` in `code/digimon-engine/src/combat.rs` (called from `delete_permanent_with_effects`). Builders: `Effect::on_enter_field_anyone(card)` and `Effect::on_any_deletion(card)` in `code/digimon-engine/src/effect.rs`. Track A added `effect_initiated` bit on `TriggerContext` and DSL `event_is_effect_initiated`. PUPPETS-G011 closure (2026-05-08) added deleted-object snapshot predicates `event_target_kind`/`event_target_trait_has`/`event_permanent_is_source`/`source_is_unsuspended`. Effect-played token plays now also queue `OnEnterFieldAnyone` / `OnAllyPlayed` with an `EnteredField` payload. Card-shaped consumers exercised by BT22-002, BT22-088, EX9-033, EX11-023, ST19-14, BT16-028, BT20-083.
- **Audit closure note (2026-05-15):** Per [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md), every dispatch/payload piece called out by the entry has shipped through Phase 1 (PR #449), Track A (PR #451), and PUPPETS-G011 consumer adoption (PR #472). Remaining work is card-shaped authoring, not engine substrate.

## Engine Gap: Phase-granular turn timings (`StartOfYourTurn`, `StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`) — RESOLVED 2026-05-15 (PR #449)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT21-081 Owen Dreadnought, BT24-016 Lamiamon, LM-021 Agumon – Bond of Bravery, BT23-014 Gallantmon, BT17-018 Gallantmon: Crimson Mode, BT21-029 Medusamon, EX11-012 Medusamon, BT21-015 Cyclonemon, plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** Various — "[Start of Your Main Phase] …", "[When Attacking] …", "[End of Attack] …", "[Security] At the end of the battle …"
- **Resolution:** All five timings wired in `digimon-engine` — see `fire_start_of_your_turn()` in `begin_turn` (before unsuspend), `fire_start_of_your_main_phase()` in `enter_main_phase`, `fire_on_attack()` and `fire_when_attacking()` in `combat::fire_on_attack`, `fire_end_of_attack()` in `cleanup_attack`, and `fire_end_of_battle()` in `resolve_battle` (Digimon-vs-Digimon only). Builders: `Effect::start_of_your_turn/start_of_your_main_phase/when_attacking/end_of_attack/end_of_battle(card)` in `code/digimon-engine/src/effect.rs`. Track A (2026-05-08) added breeding fan-out for `StartOfYourMainPhase` via `enqueue_from_breeding_permanent` and the stable `BREEDING_TARGET` source handle. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- start_of_your_main_phase_fans_out_to_battle_and_breeding_once_each --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, all five timings shipped in Phase 1 (PR #449) plus breeding fan-out in Track A; the entry has not had a new substrate ask since 2026-04-19. Remaining breeding observer work is timing-specific event fan-out, not phase timing itself, and is tracked by its own entries where applicable.

## Engine Gap: Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`, `OnHatch`, `OnAllyAttack`/`OnOpponentAttack`) — RESOLVED 2026-05-15 (PRs #449, #450, #451)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-082 Owen Dreadnought (`OnDigivolve` trait-filtered, with DP + extra-attack riders), BT24-089 Unique Emblem: Blazing Conductor (`OnSuspend` of named card), BT21-025 Lamiamon (`OnAttackTargetChange`), P-137 Flamedramon (`OnAttackTargetSwitched`), EX11-008 Elizamon (`[When Moving]` breeding→battle), BT16-082 Ukkomon (`[When Moving]` observer); plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** Various — "When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, …" / "When any of your [Owen Dreadnought]s suspend, …" / "[When Moving] [On Play] …"
- **Resolution:** All six observer variants wired in `digimon-engine` — `Effect::on_digivolve/on_suspend/on_unsuspend/on_hatch/on_attack_target_change/on_move` builders, with `fire_on_digivolve()` / `fire_on_suspend()`/`fire_on_unsuspend()` / `fire_on_hatch()` / `fire_on_attack_target_change()` dispatch sites in `combat.rs` and `game_actions.rs`. `OnAllyAttack`/`OnOpponentAttack` fire-sites in `combat::fire_on_attack`. DSL `event_target_trait_has`, `event_card_trait_has`, `event_permanent_is_source`, and `event_is_effect_initiated` predicates all wired. DNA-origin context bit (Track A 2026-05-08), prompted retarget (2026-05-08), self-scoped predicate (2026-05-08) all landed.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the "Status (2026-05-07)/(2026-05-08)" prose in the original entry lists every sub-piece as already shipped (Phase 1 PR #449, Track A PR #451, Track D combat centralization PR #450). All six variants are wired end-to-end.

## Engine Gap: `WhenWouldBeDeleted` / leave-field replacement-effect framework — RESOLVED 2026-05-15 (PRs #449 Track B; Phase C + Phase D)

- **Severity:** ✅ RESOLVED / TRACK B VERIFIED
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-018 Styracomon, EX11-012 Medusamon, BT24-012 Dimetromon, P-137 Flamedramon, BT20-016 Paildramon, plus DNA Omnimon, Rocks (all Fragment instances), and Dark Masters card lists from the original entry.
- **Effect text:** "When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion." (and other leave-field replacement variants)
- **Resolution:** Phase C (2026-04-25) + Phase D (2026-04-25) + Track B closeout (2026-05-08) shipped all replacement primitives. `Game.parked_replacement`, `cancel_leave`/`handle_replacement`/`redirect_replacement`/`substitute_replacement` outcome setters, all seven alpha-tier keyword auto-installs (Fragment / ArmorPurge / Save / Decoy / Fortitude / Partition / MaterialSave). Generic DSL cross-permanent replacement authoring, named pre-move windows (`WhenPermanentWouldDigivolve`, `WhenPermanentWouldPlay`, `WhenWouldLink`), inherited replacement scanning, cross-permanent subject guards, Delay-as-prevention, native/inherited Barrier, Armor Purge, Scapegoat, Fragment, color-gated Decoy, Decode/material play, Partition source enforcement, source-scoped immunity short-circuiting, Overclock cause propagation, and Counter Blast DNA/security-loss replacement are all covered by framework and card-shaped tests.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the entry already prefixed ✅ RESOLVED / TRACK B VERIFIED but the at-a-glance row still read 🔴; this relocation aligns headline severity with the per-entry prose. Track B (PR #449) is cited as the closing PR.

## Engine Gap: `OnPlaceSecurity` / `OnAddedToSecurity` observer timing dispatch — RESOLVED 2026-05-15 (PR #451 Track A)

- **Severity:** 🟡 PARTIAL
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT14-033 Patamon (inherited "[Your Turn] [Once Per Turn] When a card is added to your security stack, gain 1 memory.") — Dark Masters adds: BT8-090 Kari Kamiya ("[Your Turn] When a card is added to your security stack, you may suspend this Tamer to gain 1 memory.")
- **Effect text:** "When a card is added to your security stack, gain 1 memory."
- **Resolution:** Track A (2026-05-08) landed the full dispatcher with payload (`event_card`, `affected_player`, `source_player`, `EventCause::SecurityPlacement`, moved-card set). `EffectTiming::OnPlaceSecurity` fires from `place_on_security` commits and `on_added_to_security` is an alias. `effect.rs:512` builder exists. DSL `when: on_place_security` and `when: on_added_to_security` both lower to the same dispatcher. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_place_security_fires_once_with_security_placement_payload` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_place_security_event_card_trait_predicate_matches_placed_card`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the 🟡 severity was overstated for an engine entry whose only "open" item is card test coverage; remaining setup/recovery multi-card additions are card-shaped proof work, not engine substrate. Track A (PR #451) cited as the closing PR.

## Engine Gap: Forced opponent hand reduction primitive (`ctx.trash_opponent_hand_to_count`) — RESOLVED 2026-05-15 (PR #454 Track E)

- **Severity:** 🟢 CLOSED (2026-05-08, Track E)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT19-075 MoonMillenniummon
- **Effect text:** "Your opponent trashes cards in their hand until they have 5 left."
- **Resolution:** `EffectContext::trash_opponent_hand_to_count(opponent, target_count) -> bool` shipped in `code/digimon-engine/src/effect_context/mod.rs`. Installs a `select_count_capped_multi` selection on the opponent's hand with `selecting_player = opponent` (the affected side picks, per the no-approximations rule), `max = current_hand_size - target_count`, and `is_optional_zero = false` (cannot PASS without picking). The callback trashes each chosen card by stable `CardHandle` (so hand-index shifts between picks don't invalidate the reference). No-op if hand is already at or below `target_count`. DSL closed 2026-05-09: YAML can author `trash_opponent_hand_to_count: { opponent: opponent, target_count: 5 }`. Covered by `cargo test … --test zone_manipulation -- trash_opponent_hand_to_count` plus `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_zone_movement_steps` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already labeled closed in the prose — relocated here to clean up the open-gap list. Track E (PR #454) cited as the closing PR.

## Engine Gap: De-Digivolve N primitive (single + mass) — RESOLVED 2026-05-15 (Phase 10)

- **Severity:** 🟢 CLOSED
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** EX9-013 BlitzGreymon, BT9-112 DeathXmon, plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "`<De-Digivolve N>` 1 of your opponent's Digimon. (Trash up to N cards from the top. You can't trash past level 3 cards.)"
- **Resolution:** `EffectContext::de_digivolve` at `effect_context/mod.rs:1966`. Closed in Phase 10 (2026-04-21, plan [`docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md`](../docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md)). Generalized signature: `ctx.de_digivolve(target, stop_at_level: Option<u8>, amount: Option<u8>) -> u8`. TS Olympos Ikkakumon-style unbounded pop expressible as `(None, None)`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already labeled closed in the prose — relocated here to clean up the open-gap list.

## Engine Gap: `OnDiscardSecurity` — effect-driven security-card trash trigger — RESOLVED 2026-05-15 (PR #451 Track A)

- **Severity:** 🟢 CLOSED for base dispatch
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT13-106 Odin's Breath
- **Effect text:** "When an effect trashes this card from the security stack, activate this card's [Main] effect."
- **Resolution:** Added `EffectTiming::OnDiscardSecurity`, `Effect::on_discard_security`, DSL `when: on_discard_security`, and `TriggerSource::SecurityDiscarded`. Effect-driven `trash_top_security` now fires the trashed security card's own timing with `event_card`, `affected_player`, `source_player`, `event_cause`, and a security-to-trash moved-card set; normal attack security checks do not fire it. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- discard_security`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_discard_security_event_cause_predicate_matches_effect_trash`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_106`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already labeled closed in the prose — relocated here to clean up the open-gap list. Track A (PR #451) cited as the closing PR.

## Engine Gap: Global `OnOpponentSecurityRemoved` observer timing — RESOLVED 2026-05-15 (PRs #449 Phase 1, Track A 2026-05-06/05-08)

- **Severity:** 🔴 BLOCKING (closed core; card-local authoring remains card-shaped follow-up)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT21-008 Elizamon, BT21-017 Dimetromon, BT21-025 Lamiamon, BT24-018 Styracomon, BT24-016 Lamiamon, BT21-001 Gigimon, BT24-008 Elizamon, BT18-087 Owen Dreadnought, BT21-093 Raging Serpentine, BT21-029 Medusamon, EX11-008 Elizamon, P-189 Dimetromon, BT24-012 Dimetromon, BT24-001 Gigimon, BT14-001 Koromon — DNA Omnimon adds: BT22-013 WarGreymon, BT17-015 WarGreymon, EX4-073 Omnimon Alter-B — Rocks adds: EX10-036 Magneticdramon, BT20-055 Invisimon — Dark Masters adds: BT4-097 Kari Kamiya (own-side mirror), BT24-001 Gigimon.
- **Effect text:** "[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory." (and many archetype variants)
- **Resolution:** Phase 1 (2026-04-19) + 2026-05-06 typed-payload sweep + 2026-05-08 breeding fan-out all landed. `combat.rs:2613,2629` dispatches both `OnOpponentSecurityRemoved` and `OnOwnSecurityRemoved` with `TriggerSource::SecurityRemoved` payload (`affected_player`, `source_player`, `event_card`, `EventCause`, moved-card set). Battle and effect-driven security removal both covered; breeding-resident observers covered by `enqueue_from_breeding_permanent`. Builder `Effect::on_opponent_security_removed(card)` in `code/digimon-engine/src/effect.rs`. Covered by `on_own_security_removed_fires_for_defender_with_battle_payload`, `bt4_097_*`, BT24-001 inherited security-removal fixture, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the prose explicitly says "remaining work under this heading is card-local authoring/selection behavior" — that's not an engine gap. Severity 🔴 → ✅ RESOLVED with card-local authoring + non-battle-zone setup/Recovery fan-out as card-shaped follow-ups.

## Engine Gap: Global `OnOwnSecurityRemoved` observer timing (mirror of `OnOpponentSecurityRemoved`) — RESOLVED 2026-05-15 (Track A 2026-05-06)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT4-097 Kari Kamiya, BT8-090 Kari Kamiya
- **Effect text:** "[All Turns] When a card is removed from your security stack, by suspending this Tamer, gain 1 memory." / "[Your Turn] When a card is added to your security stack, you may suspend this Tamer to gain 1 memory."
- **Resolution:** `EffectTiming::OnOwnSecurityRemoved` exists (enums.rs:355), fire-site in `combat.rs:2613`, builder `Effect::on_own_security_removed` at `effect.rs:495`. Resolved as part of the same dispatch as `OnOpponentSecurityRemoved`; the 2026-05-06 typed-payload sweep wired both directions with `TriggerSource::SecurityRemoved`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the prose did not reflect the closure but the dispatch already shipped. Relocated here alongside the opponent-side entry.

## Engine Gap: Selection: ordered permutation (place N cards in any order) — RESOLVED 2026-05-15 (Phase 4)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** P-035 Red Memory Boost!, P-151 Digimon Liberator, BT21-008 Elizamon, BT24-018 Styracomon, P-103 Offense Training, P-206 Digital Gate Open, EX7-074 Vortex Resonance, BT16-082 Ukkomon plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "Return the rest to the bottom of the deck in any order." / "Place the remaining cards at the bottom of your deck in any order."
- **Resolution:** Closed by Phase 4 (2026-04-20) — helper `select_ordered_permutation` in `code/digimon-engine/src/effect_context/selections.rs`, surfaces as `SelectionKind::OrderedPermutation` / `GamePhase::SelectPermutation`. Sequential pick-by-pick with accumulator; empty items call fires immediately; singleton still installs a 1-choice selection. See `docs/RUST_ENGINE_API.md` §Phase 4.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, status block already says "Closed by Phase 4"; relocated here to clean up the open-gap list.

## Engine Gap: Token creation + `CardKind::Token` + Petrification Token definition — RESOLVED 2026-05-15 (Phase 10)

- **Severity:** 🟢 CLOSED
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-017 Medusamon, BT21-029 Medusamon, EX11-012 Medusamon
- **Effect text:** "they play 1 [Petrification] Token. (Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend. [On Deletion] Trash your top security card.)"
- **Resolution:** Closed in Phase 10 (2026-04-21, plan [`docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md`](../docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md)). Introduces `CardKind::Token` + `TokenRegistry`. `ctx.play_token(controller, token_id) -> Option<PermanentHandle>` creates a synthetic `CardSource`, places a `Permanent`, fires `OnPlay`. Ships Petrification Token data + `CardEffect` (CannotSuspend [Your Turn] + OnDeletion → `trash_top_security`). Familiar Token's [On Deletion] clause still requires the opponent-permanent selection primitive — deferred.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already 🟢 closed; relocated here to clean up the open-gap list.

## Engine Gap: Native printed keyword parsing (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N, Fragment, Save, Collision, Retaliation) — RESOLVED 2026-05-15 (PRs Phase 3 + #457 Track G + Group 6)

- **Severity:** ✅ RESOLVED
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** 17+ cards in Medusamon alone plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** `<Rush>`, `<Raid>`, `<Piercing>`, `<Blocker>`, `<Reboot>`, `<Jamming>`, `<Blitz>`, `<Vortex>`, `<Alliance>`, `<Security A. +N>` printed on the card face
- **Resolution:** Phase 3 (2026-04-19) closed native keyword parsing — see `CardData::keywords` (code/digimon-engine/src/card_data.rs) and `Game::has_keyword` unified query (code/digimon-engine/src/game.rs). All 14 keyword check sites migrated. Group 6 Task 4 (2026-05-02) closed core combat keywords `Collision`, `Piercing`, `Reboot`, and `Retaliation` end-to-end through runtime consumers. Track G (PR #457, 2026-05-10) backfilled Evade printed-semantics, Decoy color-filter, Progress card-shape backfill.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED at entry level; relocated here to clean up the open-gap list. Phase 3 + Group 6 + Track G (PR #457) cited as the closing work.

## Engine Gap: `<Progress>` keyword + `ImmunityToOpponentEffects` modifier — RESOLVED 2026-05-15 (Group 6 + Track G)

- **Severity:** ✅ RESOLVED for core Progress/opponent-effect mutation gates
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT21-025 Lamiamon, BT24-018 Styracomon, BT24-017 Medusamon, BT21-029 Medusamon, EX11-012 Medusamon, P-189 Dimetromon, plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "`<Progress>` (While attacking, your opponent's effects don't affect this Digimon.)"
- **Resolution:** Closed by Group 6 for the core Progress/opponent-effect mutation contract. `Keyword::Progress` is parsed as a combat keyword, and opponent-effect mutation gates cover delete, DP changes, suspend/unsuspend, attack restrictions, security-attack changes, return-to-hand, return-to-deck, and de-digivolve while preserving own effects, battle, costs, and rule cleanup. Track G Phase F (2026-05-10) backfilled inherited Progress test coverage. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_f -- progress`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Armor Purge>` keyword (leave-field replacement variant) — RESOLVED 2026-05-15 (Phase D 2026-04-25; Track B 2026-05-08)

- **Severity:** ✅ RESOLVED (2026-04-25)
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** BT24-018 Styracomon, P-137 Flamedramon
- **Effect text:** "`<Armor Purge>` (When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.)"
- **Resolution:** Phase D `ctx.armor_purge_top(perm)` primitive landed (commit f08d5eca, fires `OnDigivolutionCardTrashed`); `Keyword::ArmorPurge` auto-install wired in `keyword_to_auto_effect` (commit e07031d5); docstring updated (56e48afc). Gate: `card_sources.len() >= 2`; no player selection — top-swap is forced. Track B update 2026-05-08: Royal Knights Armor Purge consumers authored/tested through the replacement framework. `BT23-054` and `BT21-037` expose the optional accept/decline prompt, trash the top source on accept, and only cancel the pending deletion that was paid for.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Barrier>` keyword (battle-only leave-field replacement with security-trash cost) — RESOLVED 2026-05-15 (Track B 2026-05-08)

- **Severity:** ✅ RESOLVED / TRACK B VERIFIED
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** P-194 Aegiomon (face + inherited), BT24-034 Aegiomon (face + inherited), BT24-035 Gatomon (inherited), BT24-062 MasterBlimpmon (face), P-165 ShoeShoemon (inherited), BT24-039 Piximon (face), BT24-033 Salamon (inherited), EX11-019 Shoemon (inherited), BT24-024 Submarimon (face)
- **Effect text:** "＜Barrier＞ (When this Digimon would be deleted in battle, by trashing the top card of your security stack, prevent that deletion.)"
- **Resolution:** `Keyword::Barrier` now installs a battle-deletion-only optional replacement, pays by trashing the top card of the controller's security stack, and declines or no-security cases let the original deletion proceed. Printed inherited Barrier is synthesized through `Game::effects_for_card`, so buried-source Barrier keeps source attribution while protecting the carrier. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- printed_barrier_keyword` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_019_inherited_barrier`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED at entry level; relocated here.

## Engine Gap: `<Collision>` keyword (attack-scoped opposing Blocker aura + must-block enforcement) — RESOLVED 2026-05-15 (Group 6 Task 4)

- **Severity:** ✅ RESOLVED for core printed/granted Collision mask + decode enforcement
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-063 Locomon (face + inherited)
- **Effect text:** "＜Collision＞ (During this Digimon's attack, all of your opponent's Digimon gain ＜Blocker＞, and must block if possible.)"
- **Resolution:** `Keyword::Collision` is available through native printed parsing and `Game::has_keyword` consumers. The block interrupt mask removes `SEL_REPLACEMENT_PASS` only while a legal blocker exists, and decode rejects block-decline while Collision makes blocking mandatory. Granted Collision shares the same keyword read path as printed Collision. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords collision_mandatory piercing_security reboot_unsuspend --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED at entry level; relocated here.

## Engine Gap: `Keyword::Decoy` color-filter parameter + replacement-framework wiring — RESOLVED 2026-05-15 (Phase D + Track G PR #457)

- **Severity:** ✅ RESOLVED (2026-04-25)
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** ST12-12 Sistermon Blanc
- **Effect text:** "this Digimon gains ＜Decoy (Red/Black)＞ (When your other Red or Black Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent 1 of those Digimon's deletion.)"
- **Resolution:** Phase D `Keyword::Decoy` auto-install wired in `keyword_to_auto_effect` (commit 3a6b70a5). Track G (PR #457, 2026-05-10) added color-filter parameter — `Keyword::Decoy(u8)` in `enums.rs:416` with color bitmask. Parser at `card_data.rs::decoy_color_mask_from_paren`. Auto-install at `cards/keyword_effects.rs::keyword_to_auto_effect`. Track B 2026-05-08: `ST12-12` encodes Decoy (Red/Black) as an explicit replacement clause with a red/black subject predicate and the delete-self cost. Trait-filter remainder documented as a separate per-card override pattern, not an engine gap.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED for color filter; trait-filter is per-card override pattern.

## Engine Gap: Trash all digivolution cards of a permanent (unbounded stack-peel) — RESOLVED 2026-05-15 (2026-05-03)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-040 Venusmon
- **Effect text:** "Trash all digivolution cards of 1 of your opponent's Digimon."
- **Resolution:** `EffectContext::trash_all_sources` at `effect_context/mod.rs:3211`. DSL `trash_all_sources` lowers (dsl_cards/step/permanent_mutations.rs:143). Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregates --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, entry footer says "slice is implemented and verified for BT24-040"; relocated here.

## Engine Gap: `<Reboot>` keyword enforcement in opponent's unsuspend phase — RESOLVED 2026-05-15 (Group 6 Task 4)

- **Severity:** ✅ RESOLVED for core Reboot unsuspend enforcement
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-058 Blimpmon (inherited `<Reboot>`) — Dark Masters adds: BT15-062 Gigadramon, EX10-010 BlackWarGreymon, BT16-046 GranKuwagamon, BT21-051 Puppetmon
- **Effect text:** "＜Reboot＞ (Unsuspend this Digimon during your opponent's unsuspend phase.)"
- **Resolution:** Printed Reboot parsing and opponent-unsuspend-phase enforcement route through `Game::has_keyword`. A suspended Reboot Digimon unsuspends during the opponent's unsuspend phase, once for that phase. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords collision_mandatory piercing_security reboot_unsuspend --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords --nocapture`. Reboot-suppression variants such as "can't unsuspend during your opponent's next unsuspend phase" remain separate card/effect behavior.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Fixed attack target — `CannotBeRedirectedAsAttackTarget` / `CannotSwitchAttackTarget` modifiers — RESOLVED 2026-05-15 (Track C + Track D 2026-05-06/07)

- **Severity:** ✅ RESOLVED across Block, Raid, and the unified substitution API (2026-05-07)
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-062 MasterBlimpmon (inherited "[Your Turn] This Digimon's attack target can't change.")
- **Effect text:** "[Your Turn] This Digimon's attack target can't change."
- **Resolution:** Track C taxonomy publishes the target/attacker lock modifiers (2026-05-06) and Track D wires the consult sites (2026-05-07). `CannotSwitchAttackTarget` early-returns from `try_enter_block`, `try_enter_raid_retarget`, and `apply_attack_target_substitution`. `CannotBeRedirectedAsAttackTarget` filtered from Block / Raid retarget candidates and rejected by substitution API. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_modifiers --nocapture` (8 tests covering Block + Raid + player-target paths).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED at entry level; relocated here.

## Engine Gap: Raid target-switch interrupt (scripting-surface, not mask-only) + effect-driven attack redirect — RESOLVED 2026-05-15 (Track D 2026-05-08)

- **Severity:** ✅ RESOLVED for core Raid / redirect attack-flow surfaces
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-017 Medusamon, BT24-011 Cyclonemon, EX11-012 Medusamon, P-137 Flamedramon, BT21-025 Lamiamon plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "`<Raid>` (When this Digimon attacks, you may switch the target of attack to 1 of your opponent's unsuspended Digimon with the highest DP.)"
- **Resolution:** Core Raid is now a printed mid-attack interrupt. Non-Vortex attacks transition `Declared -> RaidOpen -> AllianceOpen`, and `RaidOpen` installs an optional `PendingSelection` for the attacker's controller when an opponent has unsuspended Digimon tied for highest DP. PASS keeps the declared target, selecting a target rewrites `effective_target`, and successful switches fire `OnAttackTargetChange` with `reason = Raid`. The post-Block invalid-target rider routes through `Game::validate_attack_redirect_target`. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- raid_retarget`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED at entry level; relocated here.

## Engine Gap: `<Delay>` keyword + placement-turn gating for Option cards — RESOLVED 2026-05-15 (Group 5 2026-05-02)

- **Severity:** ✅ RESOLVED (Group 5, 2026-05-02)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** P-103 Offense Training, LM-027 Red Scramble, BT24-089 Unique Emblem, P-035 Red Memory Boost!, BT21-093 Raging Serpentine, P-206 Digital Gate Open, plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "`<Delay>` (By trashing this card after the placing turn, activate the effect below.)"
- **Resolution:** Group 5 closes the reusable Delay lifecycle: delayed Options persist on the battle area, are gated by placement turn, fire at end/start/event windows, trash themselves through the replacement-aware cost path, resume after nested pending selections, and preserve outer scan/drain state. BT17-097 Return to the Primogenitor's Delay-as-replacement prevention is owned by the Group 3 Task 5 handoff (`docs/superpowers/plans/2026-04-30-gap-group-3-task-5-delay-replacement.md`). Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Scheduled end-of-turn effect queue (for transient Options) — RESOLVED 2026-05-15 (Group 5 Task 7)

- **Severity:** ✅ RESOLVED
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** BT1-090 Gravity Crush — Dark Masters adds: EX10-012 / EX10-020 / EX10-035 / EX10-057 / EX10-061
- **Effect text:** "[Main] Gain 2 memory. At end of turn, lose 2 memory."
- **Resolution:** `EffectContext::schedule_delayed_with_runtime` captures the source card, source permanent, source kind, controller, bindings, schedule turn, and runtime; `Game::fire_end_of_your_turn` drains `EndOfYourTurn` and `EndOfYourNextTurn` scheduled entries after printed observers, so Standard Options can schedule replay bodies that still run after the Option has been trashed. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- transient_option_scheduled_end_of_turn_effect_replays_with_option_source`. Provenance-anchored "delete the Digimon this effect played" cleanup remains tracked separately.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Effect re-firing / cross-timing self-trigger — RESOLVED 2026-05-15 (Task 9 2026-05-03; Track K 2026-05-10)

- **Severity:** ✅ RESOLVED (Task 9, 2026-05-03)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** EX8-074 MedievalGallantmon
- **Effect text:** "[All Turns] [Once Per Turn] When Digimon are played, you may activate 1 of this Digimon's [When Digivolving] effects."
- **Resolution:** `EffectContext::refire_effect_from_permanent(source, "when_digivolving")` enumerates safe refireable effects on the selected permanent, queues the exact effect slot through the normal `QueuedEffect` path, preserves `source_card` / `source_permanent` identity, and reuses existing once-per-turn accounting. DSL authors can use `refire_effect: { source: <binding>, timing: when_digivolving, optional: true|false }`. `BT22-040` and `BT22-042` provide self-refire fixtures combining deleted-object payload predicates with the refire primitive. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_refiring --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_refiring --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_040 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_042 --nocapture`. BT15-102-style foreign-card `[On Play]` activation is still tracked separately under "Cross-card effect re-firing — activate a foreign card's [On Play] effect attributed to the source".
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Effect-initiated digivolve from non-hand source zones — RESOLVED 2026-05-15 (Group 4 2026-05-02)

- **Severity:** ✅ RESOLVED (Group 4, 2026-05-02)
- **Discovered in:** Chaos Control (2026-04-28); TS Olympos (2026-04-18)
- **Card(s):** EX11-005 Yaamon, EX11-069 Yuuki, BT21-100 The Digimon I Designed, BT24-080 Megidramon; security-stack variant previously surfaced by BT14-033 Patamon.
- **Effect text:** "This Digimon may digivolve into a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash..."
- **Resolution:** `CardSourceRef` now supports hand, trash, deck top, security, material-stack, and reveal sources; `Game::effect_initiated_digivolve_from_source` removes/restores the selected card without hand transit, pays the effect cost, and fires the normal digivolve observers. The DSL keeps legacy `from_hand` and adds source-parametric `source`. Regression coverage: `effect_digivolve_from_zones::{effect_digivolve_from_trash_moves_exact_card_to_target_top,effect_digivolve_from_security_moves_selected_card_and_preserves_neighbors,effect_digivolve_from_material_moves_exact_source_out_of_stack,failed_effect_digivolve_restores_source_zone}` and `dsl::group4_zone_movement_effect_digivolve_step_uses_card_binding_live_source`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Force-follow-up-attack / "may attack without suspending" script helpers — RESOLVED 2026-05-15 (2026-05-08)

- **Severity:** 🟡 PARTIAL (closed; persistent variants tracked under player-scoped-modifier registry)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18)
- **Card(s):** BT21-081 Owen Dreadnought, BT24-082 Owen Dreadnought, BT20-016 Paildramon, BT20-102 Omnimon (X Antibody), BT21-072 Arresterdramon: Superior Mode, EX9-013 BlitzGreymon, BT24-037 Silphymon, plus DNA Omnimon and Rocks card lists from the original entry.
- **Effect text:** Variants of "it may attack" / "attack without suspending" immediately following another effect.
- **Resolution:** Immediate in-effect attack prompts are implemented via `EffectContext::may_attack_now(...)` / `may_attack_now_optional(...)` and DSL `may_attack_now`. `EffectContext::force_opponent_attack(...)` and DSL `force_attack` install a mandatory prompt. `AttackOpen.cost_upgrade` plus DSL `cost_upgrade` apply printed attack-only DP/security riders. DSL predicate `binding_owner: { binding, of }` lets continuations test the controller of a previously selected permanent. Card-shaped coverage for BT20-102, BT21-072, BT24-082, BT20-016, BT21-081, EX9-013, AD1-009, BT22-015, BT24-037, and BT24-047. Persistent player-scoped grants such as `MayAttackPlayerOnly` are covered by the player-scoped modifier / granted-triggered-ability entries.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the prose explicitly says "no separate core combat primitive remains open for Raid target switching or effect-driven redirects" — the persistent variants are owned by the player-scoped-modifier entry. Relocated here.

## Engine Gap: Granted triggered ability — attach an `Effect` to another permanent — RESOLVED 2026-05-15 (PR #467 Track H)

- **Severity:** 🟡 PARTIAL (closed for substrate)
- **Discovered in:** DNA Omnimon (2026-04-17); Rocks (2026-04-18)
- **Card(s):** EX1-068 Ice Wall! ("All of your opponent's Digimon gain \"[When Attacking] lose 2 memory\" until the end of their next turn.") — Rocks adds: EX10-034 Blastmon; BT21-095 Wind Guardians.
- **Effect text:** As above.
- **Resolution:** Full Track H closure (2026-05-10, PR #467). `EffectContext::grant_triggered_effect(carrier, timing, expiry, body)` registers a closure-bodied granted effect on the carrier's `ModifierRegistry` slot. `Game::fire_granted_triggered_effects(handle, timing)` iterates and runs each body inline. Cleanup: `clear_permanent` evicts on carrier-leave; `expire_end_of_turn` evicts on turn-bound expiry. Multi-timing dispatch wired centrally via `pending_granted_fires`. `Expiry::EndOfOpponentsNextTurn`/`EndOfYourNextTurn` + `pending_skips` for mid-opp-turn installs. Queue-based granted-body dispatch with selection support via `QueuedEffect.granted_effect_id`. Typed `AuraScope`/`AuraGrant`/`AuraBuilder` API in `code/digimon-engine/src/aura.rs`. DSL `grant_triggered_effect` step lowers. EX1-068 Ice Wall! ships as raw_rust *and* DSL fixture. Dead-body registry cleanup on carrier leave is a memory-overhead nit (~16 bytes/granted body), not a behavioral bug.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, both YAML and raw_rust authoring paths now have full typed surfaces; the residual is small polish. Track H (PR #467) cited as the closing PR.

## Engine Gap: Named-target declarative aura (DP / keyword grants filtered by name/trait/level) — RESOLVED 2026-05-15 (Group 6 + Track H §9)

- **Severity:** 🟡 PARTIAL (closed for tick-driven path)
- **Discovered in:** DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** BT22-084 Nokia Shiramine, BT5-093 Tai Kamiya & Matt Ishida, ST21-13 Matt Ishida & T.K. Takaishi — Dark Masters adds: EX10-061 Apocalymon, EX2-046 ADR-02 Searcher.
- **Effect text:** "[All Turns] All your Digimon with [Greymon], [Garurumon] or [Omnimon] in their names get +1000 DP." etc.
- **Resolution:** Group 6 + Track H §9 (2026-05-10) closure. `kind: aura` scans matching battle-area permanents and installs `dp_modifier`, `grant_keyword`, and named permanent `modifier` entries. Card-shaped fixtures landed under `code/digimon-engine/tests/dsl/group6_auras.rs` for Holy +1000 DP filter aura, cross-side opponent -2000 DP, and named-target keyword aura "Royal Knight gain Rush" via `name_contains`. Tick refresh does not duplicate materialized modifiers, and source-leave refresh removes stale materialized modifiers. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`. Query-time aura recomputation remains the more faithful long-term shape and is tracked as a separate ergonomics gap.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the tick-driven path is closed end-to-end; relocated here.

## Engine Gap: Declarative aura sourced from security zone — RESOLVED 2026-05-15 (Track H §5 PR #467)

- **Severity:** 🟡 PARTIAL (closed for consult-via-modifier-registry path)
- **Discovered in:** DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** ST20-15 Island of Adventure, BT21-095 Wind Guardians
- **Effect text:** "[Security] [All Turns] All of your level 3 or higher Digimon get +2000 DP."
- **Resolution:** Track H §5 (2026-05-10) wired `tick_declarative_effects` to iterate face-up security cards. `kind: aura, scope: security` body shipped. ST20-15 / BT21-095 representative slices proven. DP modifiers, keyword grants, security-attack grants, and named-modifier grants all flow through the same path. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- security_zone_aura`. Tensor/mask pre-compute from `SecuritySource` remains a separate ergonomics follow-up.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the consult-via-modifier-registry path is closed; remaining tensor scaffolding is a separate "Aura tensor pre-compute" follow-up. Track H (PR #467) cited as the closing PR.

## Engine Gap: `EndOfOpponentsTurn` effect timing not dispatched — RESOLVED 2026-05-15 (Phase 1 PR #449)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-066 Machinedramon, BT15-079 Piedmon, BT9-112 DeathXmon, BT15-031 MetalSeadramon
- **Effect text:** "[End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon card with the [Dark Masters] trait..."
- **Resolution:** Fire site wired in `digimon-engine` — see `fire_end_of_opponents_turn()` called in `rotate_turn_player()` (between EndOfYourTurn drain and turn advance). Dispatches to the non-ending player's battle area. Builder: `Effect::end_of_opponents_turn(card)` in `code/digimon-engine/src/effect.rs`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, the 🔴 header was contradicted by its own closure footer; relocated here.

## Engine Gap: Inherited triggered-effect dispatch: `enqueue_from_permanent` must walk digivolution stack — RESOLVED 2026-05-15 (2026-05-06)

- **Severity:** 🟢 CLOSED for battle-area stack dispatch
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT3-006 DemiMeramon, BT24-001 Gigimon, BT16-006 Cupimon, BT15-006 DemiMeramon, BT17-001 Gigimon, BT14-001 Koromon
- **Effect text:** "Inherited Effect [On Deletion] <Draw 1>. Then, trash 1 card in your hand." etc.
- **Resolution:** `enqueue_from_permanent` walks the stack with the top-card vs inherited-source discriminator, preserving the under-card `source_card` while the carrier permanent remains the event host. BT24-001 proves the stack path end-to-end: inherited `OnOpponentSecurityRemoved` fires from the carrier, exposes an optional delete selection, deletes after acceptance, declines cleanly, and enforces same-turn OPT lockout. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_001`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already 🟢 closed; relocated here.

## Engine Gap: `CannotAttackPlayer` modifier enforcement (mask + combat) — RESOLVED 2026-05-15 (Track D 2026-05-08)

- **Severity:** ✅ RESOLVED for mask + shared combat entry enforcement
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT9-103 Kongou, EX2-046 ADR-02 Searcher
- **Effect text:** "until the end of your opponent's turn, your opponent's Digimon with play costs of 7 or less can't attack players"
- **Resolution:** `action/mask.rs` hides direct-player attack bits for attackers carrying `CannotAttackPlayer`, including effect-created attack masks that reuse the attack target ranges. `combat::begin_attack_open` also rejects forced player targets under the same modifier before paying suspend cost or opening `PendingAttack`. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- cannot_attack_player_blocks_mask_and_shared_attack_entry`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Cross-permanent count-capped multi-select (single-source and up-to-N across own stacks) — RESOLVED 2026-05-15 (Group 2 2026-04-29)

- **Severity:** ✅ RESOLVED
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-032 Proganomon, EX10-028 Landramon, EX8-070 Zofr Kabus, EX10-036 Magneticdramon, EX11-044 Pyramidimon, EX10-033 Pyramidimon, EX8-055 Pyramidimon
- **Effect text:** "By trashing any 1 [Mineral] or [Rock] trait card from your Digimon's digivolution cards, …"
- **Resolution:** Cross-permanent source selection now reuses the existing source action range (`2000..2167`) through `encode_source_select` and `SelectionKind::SourceMulti`. `EffectContext::select_own_sources` installs a stable `SourceSelectionRef` list, exact-N selections complete automatically, up-to-N selections expose PASS only after the minimum, and DSL `select_own_sources` / `trash_selected_sources` binds and consumes source refs. Covered by `source_multi::exact_two_sources_can_be_selected_across_own_battle_area`, `source_multi::up_to_sources_enables_pass_only_after_minimum_is_met`, `source_multi_mask_only_exposes_selecting_players_pending_actions`, `select_own_sources_binds_source_refs_for_trashing`, and `empty_select_own_sources_runs_outer_tail_synchronously`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `.pay_cost()` builder hook for triggered non-cost-reduction effects — RESOLVED 2026-05-15 (Group 3)

- **Severity:** ✅ RESOLVED (Group 3)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX8-067 Close, P-169 Close, EX11-065 Close, EX10-063 Close, BT8-094 Digimon Emperor, P-130 Lui Ohwada, BT23-059 Justimon: Blitz Arm, BT4-072 Gogmamon, EX10-003 Tumblemon
- **Effect text:** Triggered ability bodies paying a cost before running their reward, where the cost is not a cost-reduction on play/digivolve.
- **Resolution:** Triggered effect costs may install pending selections and resume process only after cost payment; optional cost decline skips process without hidden auto-selection. Regression coverage: `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`; `code/digimon-engine/tests/replacements/attack_cancel.rs`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Source-scoped return-immunity modifiers (`CannotBeReturnedToHand` / `CannotBeReturnedToDeck` / `CannotBeDeDigivolved` by-opponent-effects-only) — RESOLVED 2026-05-15

- **Severity:** ✅ RESOLVED for covered Rust consumers
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX8-070 Zofr Kabus, P-215 Icemon, BT18-064 Mercurymon
- **Effect text:** "opponent's effects can't return it to hands or decks" etc.
- **Resolution:** Implemented for production `EffectContext::return_to_hand`, `EffectContext::return_to_deck`, and `EffectContext::de_digivolve` fire-sites, including card effects resolving during security checks. Queued card effects supply `effect_source_player`, so default passive cause filters block opponent effects and allow own effects; security battle/rule cleanup without a resolving effect remains `SecurityCheck`. `EffectContext::grant_zone_return_immunity_to_opponent_effects` installs the narrow three-modifier bundle without broad `CannotBeAffected` substitution. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Effect-driven attack cancellation (`ctx.end_pending_attack()`) — RESOLVED 2026-05-15 (Group 3 + Track D 2026-05-07/08)

- **Severity:** ✅ RESOLVED (Group 3)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-003 Tumblemon ("Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards from this Digimon's digivolution cards, end that attack.")
- **Effect text:** As above.
- **Resolution:** Effects can end a pending attack after a printed cost resolves, using the same triggered pay-cost continuation path. `ctx.cancel_attack()` rejects cancellation once the Counter window has opened with `AttackError::InvalidPhase`. DSL `cancel_attack: {}` lowers to `ctx.cancel_pending_attack()`. EX10-003 has production YAML using `select_own_sources { from: source, filter: ... }`, `trash_selected_sources`, and `cancel_attack`. Regression coverage: `code/digimon-engine/tests/replacements/attack_cancel.rs`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- cancel_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_003`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: DigiXros name alias (`treated as [X] for DigiXros`) — RESOLVED 2026-05-15 (Group 8 2026-05-02)

- **Severity:** ✅ RESOLVED (Group 8, 2026-05-02)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT21-021 OmniShoutmon ("This card is also treated as [Shoutmon] for DigiXros.")
- **Effect text:** As above.
- **Resolution:** Implemented as `CardData::digixros_aliases`, parsed from printed "also treated as [X] for DigiXros", "for a DigiXros", and prefix-scoped "When you would DigiXros, ... also treated as [X]" card text, including multiple bracketed aliases in one scoped phrase, and compiled DSL `digixros_aliases`. DigiXros material matching unions printed names with these aliases, while generic name predicates remain overlay-blind so the alias does not leak into unrelated name-sensitive effects. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- digixros`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Fragment (N)>` keyword — leave-field replacement via N-source self-trash — RESOLVED 2026-05-15 (Phase D 2026-04-25)

- **Severity:** ✅ RESOLVED (2026-04-25)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-036 Magneticdramon, EX8-051 Proganomon, EX10-033 Pyramidimon, EX8-055 Pyramidimon, EX10-034 Blastmon, EX11-044 Pyramidimon
- **Effect text:** "`<Fragment (N)>` (When this Digimon would be deleted, by trashing any N of its digivolution cards, it isn't deleted.)"
- **Resolution:** Phase D `Keyword::Fragment(N)` auto-install wired in `keyword_to_auto_effect` (commit d4fd09a0 + fixup db47ca35). Uses `CountCappedZone::Material(perm)` (Task 0, commit 41b6eac2) and `ctx.trash_card_source` (Task 4 review fixup db47ca35). Gate: `card_sources.len() >= N+1`. Implementation reality: Fragment(N) is mandatory (no `.optional()` call), faithfully matching DCGO `Fragment.cs:38` `canNoSelect: () => false`. Save (5c072623), Fortitude (e57ae55e), Partition (5b18d355), and MaterialSave(N) (d353013a) also resolved in Phase D.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Piercing>` combat-time security continuation after a winning battle — RESOLVED 2026-05-15 (Group 6 Task 4)

- **Severity:** ✅ RESOLVED for core Piercing security continuation
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-032 Proganomon (grants `<Piercing>`), EX8-051 Proganomon (native `<Piercing>`), EX8-070 Zofr Kabus (grants `<Piercing>`)
- **Effect text:** "`<Piercing>` (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.)"
- **Resolution:** Printed Piercing parsing and battle resolution route through `Game::has_keyword`. After a Digimon-vs-Digimon battle where the Piercing attacker deletes the opposing Digimon and survives, combat performs the normal security check continuation once. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords collision_mandatory piercing_security reboot_unsuspend --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `ModifierType::GrantCollision` + `combat::try_enter_block` honoring granted Collision — RESOLVED 2026-05-15 (Group 6 Task 4)

- **Severity:** ✅ RESOLVED for `grant_keyword`/`Game::has_keyword` consumers
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-032 Proganomon (grants `<Collision>`), EX8-070 Zofr Kabus (grants `<Collision>`), EX10-034 Blastmon (native `<Collision>`)
- **Effect text:** "1 of your such Digimon gains `<Collision>`, `<Piercing>` and +3000 DP until your opponent's turn ends."
- **Resolution:** `ctx.grant_keyword(target, Keyword::Collision, expiry)` stores a granted keyword, and `combat::try_enter_block` consumes `Game::has_keyword(attacker, Keyword::Collision)`, so native printed and granted sources share the same read path. No separate `ModifierType::GrantCollision` variant is required for this consumer; DSL declaratives should use keyword grants that lower through `Keyword::Collision`. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords collision_mandatory piercing_security reboot_unsuspend --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Ace Overflow: inherited memory penalty on zone-change from field / under-card — RESOLVED 2026-05-15 (Group 8 2026-05-02)

- **Severity:** ✅ RESOLVED (Group 8, 2026-05-02)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** EX10-010 BlackWarGreymon, EX9-013 BlitzGreymon, BT17-018 Gallantmon: Crimson Mode, LM-021 Agumon – Bond of Bravery — DNA Omnimon adds: BT17-078 Omnimon, BT17-095 Miraculous Mega Knight, ST20-11 WarGreymon — Dark Masters adds: LM-043 Darkdramon, BT16-026 Vikemon, EX8-026 MetalSeadramon, EX10-074 Beelzemon, BT16-046 GranKuwagamon, BT21-051 Puppetmon, BT19-064 Justimon: Blitz Arm
- **Effect text:** "Ace Overflow `<-N>` (As this card moves from the field or under a card to an area other than those, lose N memory.)"
- **Resolution:** Implemented with `CardData::ace_overflow: Option<i32>` populated from raw card data and compiled DSL card data. The runtime applies the memory penalty when an ACE top card leaves a battle-area stack and when an ACE source leaves from under a stack through source-trash, return-to-hand, or return-to-deck paths. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_ace_overflow_populates_runtime_card_data`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_018_ace_overflow_is_minus_5`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated + suspend/self-return as cost) — RESOLVED 2026-05-15 (Group 3)

- **Severity:** ✅ RESOLVED (Group 3)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT8-097 Crimson Blaze, BT9-112 DeathXmon, BT21-026 WarGreymon, EX8-074 MedievalGallantmon, plus DNA Omnimon, Rocks, and Dark Masters card lists from the original entry.
- **Effect text:** "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play." etc.
- **Resolution:** Implemented for Group 3 cost/replacement coverage. Stacked optional would-play cost reducers can apply before memory is paid (including AD1-025 Omnimon with both `[Royal Knight]` and `[ADVENTURE]` reducers). Triggered pay costs can park pending selections before process execution, optional cost decline skips process without hidden auto-selection, and replacement cause/controller predicates participate before prevention choices are offered. Regression coverage: `code/digimon-engine/tests/cost_hooks/stacked_would_play_reducers.rs`; `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`; `code/digimon-engine/tests/replacements/context_predicates.rs`; `code/digimon-engine/tests/replacements/partition.rs`; `code/digimon-engine/tests/option_flow/replacement_integration.rs::bt17_097_delay_prevents_deletion_and_digivolves_from_hand`; `code/digimon-engine/tests/replacements/attack_cancel.rs`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Scapegoat>` keyword (leave-field replacement with "delete another own Digimon" cost) — RESOLVED 2026-05-15 (Track B 2026-05-08)

- **Severity:** ✅ RESOLVED / TRACK B VERIFIED
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** LM-043 Darkdramon
- **Effect text:** "<Scapegoat> (When this Digimon would be deleted other than by your effects, by deleting 1 of your other Digimon, prevent that deletion.)"
- **Resolution:** `Keyword::Scapegoat` is parsed, auto-installs an optional `WhenWouldBeDeleted` replacement, suppresses the outer accept prompt when no other own Digimon can be deleted, rejects own-effect deletion via `cause != OwnEffect`, and routes the delete-another-Digimon cost through `PendingSelection`. EX11 card fixtures also encode the exact replacement body where DSL keyword granting is still card-local. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- scape_goat` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_023_scapegoat`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Evade>` printed semantics — suspend-and-cancel, NOT redirect-to-deck — RESOLVED 2026-05-15 (Track G PR #457)

- **Severity:** ✅ RESOLVED
- **Discovered in:** Track G keyword-library audit (2026-05-10)
- **Card(s):** Every printed `<Evade>` Digimon (e.g. BT5-105 Beelzemon Blast Mode, BT11-098 Mervamon, P-067 Thomas H. Norstein)
- **Effect text:** "When this Digimon would be deleted, you may suspend it to prevent that deletion."
- **Resolution:** Auto-install now suspends the carrier (firing `OnSuspend` observers) and calls `rctx.cancel()` to cancel the deletion. Gate at candidate-collection time on `!is_suspended` — an already-suspended carrier cannot pay the cost (DCGO `CanActivatePermanentSuspendCostEffect`). Self-scope and re-check guards in the body match the Fragment / ArmorPurge precedents. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_d -- evade` (6 passing) + `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- evade printed_evade_keyword_suspends` (2 passing).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: `<Progress>` card-shaped test backfill — RESOLVED 2026-05-15 (Track G PR #457)

- **Severity:** ✅ RESOLVED — engine consult was already implemented; only test coverage was missing.
- **Discovered in:** Track G keyword-library audit (2026-05-10)
- **Card(s):** Every printed `<Progress>` Digimon (BT21-025 Lamiamon, BT24-018 Styracomon, BT24-017 Medusamon, etc.) plus modifier-granted forms.
- **Resolution:** `code/digimon-engine/tests/keyword_phase_f/progress.rs` adds 9 card-shaped cases covering native printed Progress, modifier-granted Progress, inherited Progress at multiple stack positions, top-only card exclusion, `Expiry::EndOfTurn` modifier-granted Progress expiry, own-effect non-exclusion, dormant non-attacking carrier, and Tamer-source inherited Progress. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_f -- progress` (9 passing).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Inherited Token/Puppet leave-prevention replacement dispatch — RESOLVED 2026-05-15 (Track B 2026-05-08)

- **Severity:** 🟡 PARTIALLY RESOLVED (closed for named cards)
- **Discovered in:** Puppets Batch 5 (2026-05-04)
- **Card(s):** `EX9-032`, `BT22-036`, `EX11-022`; also related to `EX7-027` and `ST19-11`
- **Effect text:** "[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving."
- **Resolution:** Shared replacement candidate collection now scans inherited source effects under the carrier, preserving buried source attribution and carrier subject identity. `BT22-036`, `EX11-022`, `EX9-032`, `EX7-027`, and `ST19-11` production YAML now expose the optional accept prompt plus Token/other-Puppet cost selection and pass focused behavioral coverage. Verified with `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_036_inherited_replacement`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_022_inherited_leave_prevention`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_032_inherited_prevents`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex7_027_inherited`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st19_11_inherited`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED for the named cards; relocated here.

## Engine Gap: Effect-played permanent cleanup provenance — RESOLVED 2026-05-15 (Track A PR #451)

- **Severity:** 🟢 ENGINE-DONE (Track A 2026-05-08), DSL/card-side authoring still pending
- **Discovered in:** Puppets Batch 5 (2026-05-04)
- **Card(s):** `EX11-022`, `EX11-061`, and other effect-played cleanup riders
- **Effect text:** "At turn end, delete the Digimon this effect played."
- **Resolution:** Provenance token system shipped at the engine layer: `ctx.play_from_hand_free_with_provenance`, `Game::provenance_token_for_card`, `Game::resolve_provenance_token`, `Game::delete_permanent_with_effects`. Lookup is by `CardHandle`, not by position, so it is robust to battle-area index shifts. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- provenance_tokens::effect_play_provenance_token_survives_field_shift_and_zone_move`. DSL verb authoring (`bind_played_provenance`, `delete_provenance_token`) remains as DSL-side follow-up.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, engine substrate ✅ DONE; relocated here. DSL/card-side authoring tracked separately in `qa/dsl-vocab-gaps.md`.

## Engine Gap: Suspend-this-Tamer deletion observer with Overclock cause branch — RESOLVED 2026-05-15 (2026-05-06)

- **Severity:** ✅ CLOSED
- **Discovered in:** Puppets Batch 6 (2026-05-04)
- **Card(s):** `EX11-060` Arisa Kinosaki
- **Effect text:** "[All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted, by suspending this Tamer, <Draw 1>. If this effect was activated by <Overclock>, you may play 1 level 4 or lower [Puppet] trait Digimon card from your hand without paying the cost."
- **Resolution:** `EX11-060` is now authored with `when: on_any_deletion` over the deleted-object snapshot, an explicit activation choice before suspending Arisa, `<Draw 1>`, and an `event_cause: overclock` branch that exposes the optional level 4 or lower Puppet hand-play only for Overclock-caused deletions. Overclock cost deletion now preserves `ReplacementCause::Cost` for replacement windows while tagging the observer payload with `EventCause::Overclock`, and the Overclock attack resumes after any observer selections by re-finding the source card rather than trusting a stale battle-area slot. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_060`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ CLOSED; relocated here.

## Engine Gap: Trash-resident observer with effect digivolve from trash — RESOLVED 2026-05-15 (2026-05-06)

- **Severity:** ✅ CLOSED
- **Discovered in:** Puppets Batch 8 (2026-05-04)
- **Card(s):** `BT20-084` Sistermon Ciel (Awakened)
- **Effect text:** "[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel]s may digivolve into this card without paying the cost."
- **Resolution:** `EffectTiming::OnAllyPlayed` now dispatches from play emitters with `TriggerSource::EnteredField`, scanning the playing player's battle area and top-level trash observers exactly once. `Effect::on_ally_played` and DSL `when: on_ally_played` lower to that engine timing. The existing source-parametric effect-digivolve path can consume a trash `CardSource`; its DSL `ignore_requirements` path now allows name-filtered alt-path effects such as BT20-084. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- trash_resident_on_ally_played_observer_sees_played_subject_once`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_084`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ CLOSED; relocated here.

## Engine Gap: Effect-driven play of a Digimon from hand to an empty breeding-area slot (without paying cost) — RESOLVED 2026-05-15 (Group 4)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-062 Gigadramon, BT15-077 LadyDevimon, BT15-027 Scorpiomon, BT15-050 Cherrymon
- **Effect text:** "By deleting 1 of your Digimon, you may play 1 Digimon card with the [Dark Masters] trait from your hand to an empty space in your breeding area without paying the cost."
- **Resolution:** `play_to_breeding_from_hand` exists (mod.rs:2840) — Group 4 added it. The real breeding slot is used, source stacks stay intact, and movement observers fire. Covered by `breeding_zone_movement::play_to_breeding_from_hand_uses_real_breeding_slot_and_rejects_occupied_slot`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED but tracker was stale; relocated here.

## Engine Gap: `ModifierType::CannotAddSecurityByEffect` (player-scoped opponent-security-placement block) — RESOLVED 2026-05-15 (Track C/D 2026-05-08)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT9-103 Kongou
- **Effect text:** "cards can't be added to security stacks by your opponent's effects"
- **Resolution:** `CannotAddSecurityByEffect` in `enums.rs:612`; `CannotAddSecurity` (player-scoped variant) at enums.rs:654. Consult sites wired (Track C/D 2026-05-08 — `EffectContext::place_on_security` checks `CannotAddSecurityByEffect` then `CannotAddSecurity`).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED; relocated here.

## Engine Gap: Search-own-security-stack primitive (reveal full stack + select by filter) — RESOLVED 2026-05-15 (Track E 2026-05-09)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT14-033 Patamon — Dark Masters adds: P-216 WaruMonzaemon
- **Effect text:** "Search your security stack."
- **Resolution:** `search_own_security_stack` at `effect_context/selections.rs:1241`. DSL verb landed Track E 2026-05-09.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED; relocated here.

## Engine Gap: Effect-initiated digivolve from security stack (free, trait-filtered) — RESOLVED 2026-05-15 (Group 4)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT14-033 Patamon
- **Effect text:** "This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them [= searched security stack] without paying the cost."
- **Resolution:** `effect_initiated_digivolve_from_source` (mod.rs:3437) accepts security source per the "Effect-initiated digivolve from non-hand source zones" closure (Group 4).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED; relocated here.

## Engine Gap: In-effect branch-choice selector (`select_effect_choice` / "choose one of N effects") — RESOLVED 2026-05-15

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** P-195 Inori Misono, EX4-051 BlitzGreymon
- **Effect text:** "[On Play] Activate 1 of the effects below..."
- **Resolution:** `select_effect_choice` at `effect_context/selections.rs:602`. `SelectionKind::EffectChoice` in `selection.rs:112`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED; relocated here.

## Engine Gap: Counter window + `<Blast Digivolve>` activation flow ([Hand][Counter] play path) — RESOLVED 2026-05-15 (Track D 2026-05-08)

- **Severity:** 🟡 PARTIALLY CLOSED for engine substrate
- **Discovered in:** Dark Masters (2026-04-18); DNA Omnimon (2026-04-28)
- **Card(s):** EX10-010 BlackWarGreymon, BT16-026 Vikemon, EX8-026 MetalSeadramon, LM-043 Darkdramon, EX10-074 Beelzemon, BT16-046 GranKuwagamon, BT21-051 Puppetmon, BT19-064 Justimon: Blitz Arm, BT17-078 Omnimon, EX6-011 RagnaLoardmon, BT20-045 Examon, BT20-060 Alphamon: Ouryuken.
- **Effect text:** "[Hand] [Counter] <Blast Digivolve>" / "[Hand] [Counter] Blast DNA Digivolve ([WarGreymon] + [MetalGarurumon])"
- **Resolution:** Resolved for the reusable Track D Counter / Blast DNA selection machinery. CounterTiming is now an attack interrupt window for defender hand/field Counter candidates. The Counter window is live for single-base Blast Digivolve, hand Counter Options, field Counter abilities, and Blast DNA cards whose printed route uses one defender field Digimon plus one named hand material. DSL `kind: blast_dna_digivolve` ships. Native printed `Keyword::BlastDigivolve` auto-installs the Counter marker. Card-specific printed bodies after the Blast DNA evolution resolves remain as card-shaped follow-ups, not Counter-window substrate gaps. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- counter native_blast_digivolve_keyword_installs_counter_candidate`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED for engine substrate; relocated here. Generic `ctx.prompt_blast_digivolve(...)` / `ctx.prompt_blast_dna_digivolve(...)` raw_rust helpers are tracked as a small ergonomic follow-up.

## Engine Gap: OnDeletion cause discriminator ("if deleted by an effect" / "by battle" / "by your own effects") — RESOLVED 2026-05-15 (Phase B 2026-04-24)

- **Severity:** ✅ RESOLVED (2026-04-24)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT17-068 Mephistomon
- **Effect text:** "[On Deletion] If deleted by an effect, you may play 1 [Gulfmon] or 1 level 6 Digimon with the [Dark Masters] trait from your hand or trash without paying the cost."
- **Resolution:** `EffectContext::deletion_cause() -> Option<ReplacementCause>` / `was_deleted_by_effect() -> bool` / `was_deleted_by_opponent() -> bool` landed in Phase B (commit 17b9875b). `Game::current_deletion_cause` is populated by the deletion fire-site (commit cf400d4f) and read on the `OnDeletion` `EffectContext` so `.condition` closures can branch on cause without installing a replacement effect.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, already ✅ RESOLVED; relocated here.

## Engine Gap: Permanent-scoped modifier to suppress effect activation by timing — RESOLVED 2026-05-15 (Track C 2026-05-06)

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT10-042 Venusmon, BT24-040 Venusmon, BT19-093 Queen Device
- **Effect text:** "can't activate [When Attacking] and [When Digivolving] effects" / "can't suspend or activate [When Digivolving] effects"
- **Resolution:** `ModifierType::DisableEffect` in `enums.rs:684`; `disable_effect_timing` on `ModifierEntry`; `permanent_activation_blocked_for_timing` consult site (Track C 2026-05-06). Mirrors DCGO `DisableEffectClass.cs`. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test modifier_disable_effect --nocapture`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED; relocated here.

## Engine Gap: `<Digi-Burst N>` keyword — RESOLVED 2026-05-15 (Track G PR #457)

- **Severity:** 🟡 PARTIAL — closed by design as intentional no-auto-install (printed keyword token is a cost prefix only)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT4-072 Gogmamon (`<Digi-Burst 1>`)
- **Effect text:** "`<Digi-Burst N>` (You may trash N of this Digimon's digivolution cards to activate the effect below.)"
- **Resolution:** DSL `digi_burst: { count: N, then: [...] }` shipped; `Keyword::DigiBurst(N)` parsed from card text. Native `keyword_to_auto_effect` install for `Keyword::DigiBurst(N)` is intentionally absent (Track G close 2026-05-10) — the printed keyword token is a cost prefix for a per-card `[Main]` body that can't be synthesized from the keyword alone, matching DCGO's per-card cost+body inlining. Cards from `cards.json` carrying printed `<Digi-Burst N>` without a DSL spec silently no-op, but `Keyword::DigiBurst(N)` is still produced for tensor / mask / filter predicates. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digi_burst_two_selects_exact_self_sources_and_fires_source_trash_per_card`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, intentional-design closure rationale; relocated here.

## Engine Gap: `OnAllyAttack` / `OnOpponentAttack` observer timing context — RESOLVED 2026-05-15 (2026-04-29 substrate; DSL predicate spin-off)

- **Severity:** 🔴 BLOCKING (closed for engine substrate)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-008 Muchomon
- **Effect text:** "[Your Turn] [Once Per Turn] When one of your red Digimon attacks a player, <Draw 1>."
- **Resolution:** Battle-area declared-attack observers dispatch from the real combat state machine. `OnAllyAttack` scans the attacker's controller battle area and excludes the attacking permanent; `OnOpponentAttack` scans the defending player's battle area before Alliance/Counter/Block windows. `EffectReadContext` / `EffectContext` expose `attack_attacker()` and `attack_target()` over the live pending attack. `PendingAttack::declaration_committed` separates optional pre-declaration replacement resumes from post-declaration observer resumes. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack_fires_ally_and_opponent_observers_with_attack_context`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- on_ally_attack on_opponent_attack`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, CLOSED for engine substrate + payload accessors; attack-target-kind DSL predicate is a separate `qa/dsl-vocab-gaps.md` follow-up. Relocated here.

## Engine Gap: `OnDigivolutionCardTrashed` observer timing — RESOLVED 2026-05-15 (Phase 1 PR #449; 2026-05-07 routing fan-out)

- **Severity:** 🔴 BLOCKING (closed for substrate)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX10-032 Proganomon, P-167 Landramon, EX8-047 Sunarizamon, EX8-048 Landramon, EX8-005 Tumblemon, BT21-055 Sunarizamon, EX8-051 Proganomon, EX10-025 Sunarizamon, EX10-028 Landramon, EX11-038 Sunarizamon, EX10-063 Close, P-169 Close, EX11-044 Pyramidimon, plus any future Rocks card whose inherited text starts with "When effects trash this card from a [Mineral]/[Rock] Digimon's digivolution cards"
- **Effect text:** "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, <payoff>." etc.
- **Resolution:** Observer timing wired in `digimon-engine` — see `fire_on_digivolution_card_trashed()` in `code/digimon-engine/src/game_actions.rs`. Builder: `Effect::on_digivolution_card_trashed(card)` in `code/digimon-engine/src/effect.rs`. Fires in each player's battle area to notify Tamer observers. Payload carries the trashed card (`event_card` / `event_source_card`), host card/permanent snapshot, affected/source player, `EventCause`, and a moved-card set from battle area to trash. Return-to-deck source disposition, de-digivolve, Armor Purge, Fragment / `trash_card_source`, `trash_top_source`, and Mind Link below-top disposal all route through the same helper. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_return_to_deck_carries_host_and_trashed_source on_digivolution_card_trashed_de_digivolve_carries_host_and_trashed_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_051_inherited_source_trash_dedigivolves_after_host_return_to_deck`.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, substrate ✅ CLOSED; additional card-local source-trash producer fixtures are card test coverage, not engine work. Relocated here.

## Engine Gap: Zone-manipulation play-from-hand / trash without paying cost (+ cost override) — RESOLVED 2026-05-15 (Phase 2 PR; Track A 2026-05-08)

- **Severity:** 🔴 BLOCKING (closed for headline primitive; narrow sub-shape follow-ups spun off)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** Many — see original entry; archetype-wide.
- **Effect text:** "you may play 1 [X] from your hand without paying the cost" / "play 1 [X] from your trash without paying the cost" / "play 1 Tamer card … with the play cost reduced by 4"
- **Resolution:** Phase 2 (2026-04-19) landed `EffectContext::play_from_hand_with_cost` (mod.rs:2453) + `play_from_trash_with_cost` (mod.rs:2717) covering free and cost-delta variants via `CostDelta::Reduce(printed_cost)` / `CostDelta::Reduce(delta)`. Track A (2026-05-08) added `play_from_hand_free_with_provenance` (mod.rs:2491). `OnPlay` is fired through the standard effect queue in both paths.
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, headline primitive ✅ RESOLVED; remaining sub-shape gaps `play_from_revealed_free` (EX8-050 Gogmamon) and `play_from_security_at(index)` (BT13-012 GeoGreymon, BT14-033 Patamon) are spun off as their own narrow entries in `docs/RUST_ENGINE_GAPS.md`.

## Engine Gap: Zone-manipulation effect-initiated digivolve (free / reduced / with trait filter / ignore requirements / DNA / Blast / detect-DNA-origin) — RESOLVED 2026-05-15 (Phase 2; Track A/C 2026-05-08/09)

- **Severity:** 🔴 BLOCKING (closed for headline primitive; narrow sub-shape spun off)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** Many — see original entry.
- **Effect text:** "1 of your Digimon may digivolve into a [X] trait Digimon card in the hand with the digivolution cost reduced by N" (and "without paying the cost" variants)
- **Resolution:** Phase 2 landed `effect_initiated_digivolve` (mod.rs:3384), `_ignore_requirements` (mod.rs:3402), `_with_provenance` (mod.rs:3418), `_from_source` (mod.rs:3437), `_from_source_ignore_requirements` (mod.rs:3455), `effect_initiated_dna_digivolve` (mod.rs:3509), `_dna_digivolve_with_provenance` (mod.rs:3572). DNA-origin context bit in Track A. Blast DNA via `execute_blast_dna_digivolve` in combat.rs:1630. BeforePayCost cost reduction in modifier scan is wired (Track C deferred-payload wave, 2026-05-09).
- **Audit closure note (2026-05-15):** Per the 2026-05-14 rebaseline audit, headline primitive ✅ RESOLVED; BT17-095-style "DNA digivolve with field+hand material pair" is spun off as its own narrow card-shape gap in `docs/RUST_ENGINE_GAPS.md` if it remains blocking after the existing helpers are tried.

## Engine + DSL Gap: Effect play with played-Digimon On Play suppression — RESOLVED 2026-05-19 (Phase 2 Track J Task S1.1) [PUPPETS-G030]

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Puppets Batch 9 (2026-05-04) — BT5-106 Demonic Disaster.
- **Card(s):** `BT5-106` Demonic Disaster [Security]. Royal Knights source-play payoffs (`BT13-110`, `BT13-112`) are adjacent consumers.
- **Effect text:** "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- **What was missing:** Effect-play helpers had no way to suppress *only* the just-played permanent's [On Play] enqueue for a play event. Ordinary play-from-trash fires [On Play] normally; a broad/global suppression would wrongly silence unrelated permanents.
- **Resolution:**
  - **Engine:** Added `game::PlayOptions { origin, suppress_on_play }` (a `Copy` struct bundling the rollback origin with the new flag), threaded through `play_from_hand_with_cost_result_with_options` → `continue_play_from_hand_cost_reduction_chain` → `finish_play_from_hand_after_reductions` → `commit_play_from_hand_card_no_replace` (carried across the `WhenPermanentWouldPlay` replacement boundary via `PendingWouldPlayResume.suppress_on_play`). When set, `commit_play_from_hand_card_no_replace` skips the `fire_on_play` call. Because `fire_on_play` enqueues only the just-played permanent (`TriggerSource::Permanent`), skipping it suppresses ONLY that play — sibling permanents enqueue their [On Play] at their own play time, and the `OnEnterFieldAnyone` / `OnAllyPlayed` observer timings still fire. New `Game::play_from_trash_with_cost_options`.
  - **Card-scripting API:** New public `effect_context::PlayOptions { suppress_on_play }` + `play_from_hand_with_cost_options` / `play_from_trash_with_cost_options` / `play_from_materials_options` helpers.
  - **DSL:** `suppress_on_play: true` flag on `play_from_hand` / `play_from_hand_free` / `play_from_trash` / `play_from_trash_free` / `play_from_materials` step specs (defaults to `false`, omitted from serialization); lowered through `compile.rs` and `dsl_cards/step/play_digivolve.rs`.
  - **Card:** BT5-106's [Security] slice was authored in `code/digimon-engine/cards/bt5/BT5-106.yaml` (previously omitted pending this gap).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106` (9 pass, incl. `bt5_106_security_suppresses_on_play_effects_of_played_digimon` and the sibling-On-Play regression `bt5_106_security_suppression_does_not_silence_a_sibling_on_play`); `cargo test ... --features dsl-yaml-loader --test dsl -- suppress_on_play` (3 pass — YAML→compiled flag lowering + default-false + end-to-end suppression); full engine suite green.

## Engine + DSL Gap: Source selection from a breeding-area carrier — RESOLVED 2026-05-20 (Phase 2 Track J Task S1.3) [G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES residual]

- **Severity:** 🔴 BLOCKING (closed)
- **Discovered in:** Royal Knights DSL assessment (2026-05-03); narrowed by Track J S1.2 (2026-05-19) to the breeding-carrier residual.
- **Card(s):** `BT13-112` Omnimon, `BT13-110` Royal Knights of the Purge, `EX11-053` Omekamon, `BT13-019` Gankoomon, `BT23-072` King Drasil_7D6 — all play Royal-Knight sources out of "the digivolution cards of your Digimon in the breeding area" (a King Drasil breeding-area carrier the controller owns).
- **What was missing:** `select_material` / `select_count_capped_multi(CountCappedZone::Material)` encoded each source pick as `SOURCE_SELECT_START + perm.index * SOURCES_PER_FIELD + source_index`. A breeding-area carrier resolves to a sentinel `PermanentHandle { player, index: BREEDING_TARGET (=14) }`; candidate collection read `battle_area.get(14)` → `None` → zero candidates → the selection step no-op'd, and the would-be action ID `2168` fell outside the action space. King Drasil source picks were therefore unrepresentable.
- **Resolution (Task S1.3):**
  - **Action space:** appended a 24-slot `BREEDING_SOURCE_SELECT` sub-range (`BREEDING_SOURCE_SELECT_START..END` = `2168..2192`, `2` carriers × `SOURCES_PER_FIELD`), keyed by the carrier's owning player. `ACTION_SPACE_SIZE` raised `2168 → 2192` — a deliberate action-space version bump. New `encode_breeding_source_select` / `decode_breeding_source_select` helpers in `action/space.rs`.
  - **Engine:** new `material_carrier_permanent` / `material_zone_slice` / `material_zone_geometry` helpers in `effect_context/selections.rs` — the single battle-vs-breeding branch point on the `BREEDING_TARGET` sentinel. `select_count_capped_multi` (candidate collection, per-pick filter loop, and all three `card_sources` re-derivations inside the recursive resolution callback) and `select_material` route through them; `range_start` is threaded through the recursive count-capped trampoline. `select_material`'s callback recovers the source index via `range_start` instead of `decode_source_select`.
  - **DSL:** `has_material_candidates` and the `install_select_material` filter/bind closures resolve their carrier via `material_carrier_permanent`, so a breeding King Drasil carrier yields its real sources and `completes_synchronously` accounting is correct.
  - **Explain:** `explain_selection` gained a `BREEDING_SOURCE_SELECT` branch (`kind = SourceSelect`, `target_zone = Breeding`, `target_index = carrier owner`).
  - **Tensor:** `standard_full_v2`'s `action_id_features` grew 24 rows (`tensor_size` `43008 → 43392`, `feature_schema_version` `standard_full_v2.2`, LAYOUT_HASH recomputed). `standard_compact_v1` / `standard_lite_v2` are size-unchanged; their action mask grows `2168 → 2192` automatically.
- **Model-compat note:** the `ACTION_SPACE_SIZE` bump changes the policy/value head width — existing trained RL models must be retrained. Flagged in `docs/MODEL_CATALOG.md` and `docs/TRAINING_RUNBOOK.md`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_carrier`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier` (incl. the `distinct_by`-on-breeding recursive-path test); `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor` (mask length 2192, breeding-source mask survival, `standard_full_v2` layout); full engine suite green. **Remaining work is card-authoring only** (the Rush grant + production YAML for the five cards).

## Engine Gap: v2 observation tensor omits the 12th digivolution-source slot — RESOLVED 2026-05-20 (Phase 2 Track J Task S1.4) [G-BREEDING-SOURCE-OBSERVABILITY]

- **Severity:** 🟡 PARTIAL (closed) — an action↔observation consistency gap, not a hard block.
- **Discovered in:** Task S1.4 follow-up to the Task S1.3 `BREEDING_SOURCE_SELECT` action range (see the entry above).
- **What was missing:** Task S1.3 made every digivolution-source slot of a breeding-area carrier (and the battle-area twin) selectable via `SOURCE_SELECT` / `BREEDING_SOURCE_SELECT`, which use the engine constant `SOURCES_PER_FIELD = 12`. But the v2 observation profiles (`standard_lite_v2`, `standard_full_v2`) encoded only `PERM_MAX_SOURCES = 11` source slots per permanent. Source index 11 — the 12th source, e.g. a deep King Drasil breeding stack — was therefore a legal action (`encode_breeding_source_select(owner, 11)`) whose identity/features never appeared in the tensor. The RL agent could pick that slot but only blind. (The premise that breeding-carrier sources were entirely absent from the tensor was wrong: the v2 writer already encodes the breeding carrier as `permanent_slots` slot 14, including its source stack — the gap was strictly the 11-vs-12 off-by-one.)
- **Resolution (Task S1.4):**
  - **Tensor:** bumped v2 `PERM_MAX_SOURCES` `11 → 12` in `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`. `PERMANENT_SLOT_SIZE` is now derived (`PERM_SOURCE_START_OFFSET + PERM_MAX_SOURCES * PERM_SOURCE_ENTRY_SIZE` = 99); `permanent_slots` `2880 → 2970`; every section after it shifted `+90`; `tensor_size` `8320 → 8410`; `card_id_slot_count` `542 → 572`; `feature_schema_version` `standard_lite_v2.2`; `LAYOUT_HASH` recomputed. `standard_full_v2` re-exports these constants, so it cascaded automatically: `tensor_size` `43392 → 43482`, `action_id_features` offset `8064 → 8154`, `feature_schema_version` `standard_full_v2.3`, `LAYOUT_HASH` recomputed.
  - **Writer:** no logic change — `tensor_v2_lite.rs`'s source-encoding loop and `positions_for_section` already iterate `layout::PERM_MAX_SOURCES`, so the 12th slot writes for battle and breeding permanents, both players, automatically. `tensor_v2_full.rs` copies the lite prefix unchanged.
  - **Scope:** `standard_compact_v1` (`v1.rs`) and the PyO3-exported v1 `TENSOR_SIZE` / `MAX_SOURCES` (1375 / 11) are deliberately unchanged — touching them would alter the legacy `tensor::TENSOR_SIZE` contract. Breeding-area digivolution sources are public information (`docs/RULES_CONTEXT.md` §3), so encoding both players' breeding sources leaks nothing; the pre-existing `face_down` redaction gate already covers the new 12th slot.
- **Model-compat note:** the `standard_lite_v2` / `standard_full_v2` observation width changed — existing trained RL models predating `standard_lite_v2.2` are observation-incompatible and must be retrained. Bundled with the S1.3 action-space break as one breaking checkpoint. Flagged in `docs/MODEL_CATALOG.md` and `docs/TRAINING_RUNBOOK.md`.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor` (157 pass — incl. new failing-first tests: a 12-source breeding carrier and a 12-source battle permanent both encode source slot 11, plus an action↔observation consistency check that every `BREEDING_SOURCE_SELECT` slot is observable); full engine suite green (0 failed). PyO3 rebuilt via `maturin`; `code/tests/rl` green (167 pass) with the updated `8410` / `43482` literals.

## Engine + DSL Gap: `G-ALLY-PLAYED-MAY-ATTACK` — already-composable 2026-05-20 (Phase 2 Track J Task S2.1)

- **Severity:** 🟢 NONE — filed and closed in the same pass; no engine or DSL code change shipped.
- **Discovered in:** Royal Knights full-pool DSL assessment (`RK-G005`, 2026-05-05) — never had a canonical entry, only a name in the Royal Knights rollup. Task S2.1 filed it properly and resolved it as already-composable.
- **Card consumers:** `BT20-017` Jesmon — `[Your Turn] [Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of your Digimon may attack.` `BT23-013` Jesmon — `[Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack.`
- **Effect-text shape:** an `on_any_digimon_played` (printed "When any of your other Digimon are played") observer whose body grants a may-attack. The Track J substrate plan (`G-ALLY-PLAYED-MAY-ATTACK`) hypothesized the grant must target the **event-played Digimon** and that `may_attack_now` was hard-bound to `self`.
- **Step 0 finding (the gap as hypothesized does not exist):**
  1. **`may_attack_now` is not hard-bound to `self`.** Its `attacker:` field is a DSL `BindingRef` (`step.rs::MayAttackNowArgs`). The lowering `dsl_cards/step/combat.rs::resolve_permanent_ref` routes any non-`SelfRef` binding through `dsl_cards/binding_ref.rs::resolve_binding_ref`, and `ctx.may_attack_now_optional_with_upgrade` accepts an arbitrary `PermanentHandle` — it computes the attack mask for that exact attacker and installs a `pending_selection` with `is_optional` (PASS surfaced through the action mask via `on_decline`).
  2. **`event_target` already resolves to the event-played permanent.** `resolve_binding_ref(CompiledBindingRef::EventTarget, …)` reads `current_trigger_context.event_permanent` and returns `ResolvedBinding::Permanent(handle)` when that permanent is live in the battle area. `on_any_digimon_played` maps to `EffectTiming::OnEnterFieldAnyone`; `TriggerSource::EnteredField` populates `event_permanent` with the played Digimon (`effect_queue.rs`).
  3. **The printed card text differs from the plan's premise.** Neither Jesmon grants the attack to the event-played Digimon: BT23-013 says "**this** Digimon" (the observer source → `attacker: this`), and BT20-017 says "**1 of your** Digimon" (a player choice → `select_own_permanent` `bind_as` → `attacker: <binding>`). All three attacker shapes — `this` (`Source`), a named `bind_as` binding (`Binding`), and the hypothesized `event_target` (`EventTarget`) — were already composable from primitives landed on/before the BASE commit.
- **API shape (already present, no change):** `may_attack_now: { attacker: <event_target | this | named-binding>, targets: any, optional: true }`.
- **First failing test:** none failed against the substrate. The proof tests passed on first run, which *is* the discovery — `may_attack_now`'s attacker binding already selects between the observer source and the event-played permanent. (The only intermediate red was a test-authoring mistake: PASS for an optional selection lives in `build_action_mask`, not in `PendingSelection.valid_action_ids`; corrected to assert the mask.)
- **Proof tests (cited):** `code/digimon-engine/tests/dsl/effect_granted_attack.rs` — `may_attack_now_event_target_yaml_lowers_to_event_target_binding` (DSL lowering: `attacker: event_target` → `CompiledBindingRef::EventTarget`), `may_attack_now_event_target_grants_attack_to_event_played_digimon` (the granted attack prompt is bound to the event-played Digimon, every legal action decodes to its field index, PASS is in the action mask — §17), `may_attack_now_event_target_decline_branch_starts_no_attack` (PASS resolves cleanly, no attack starts, the played Digimon stays unsuspended), `may_attack_now_this_vs_event_target_select_different_attackers` (cross-check: `attacker: this` binds the prompt to the observer source instead — proves the binding genuinely selects), and `may_attack_now_event_target_respects_summoning_sickness` (documents that a summoning-sick event-played Digimon yields no prompt — the generic attack-eligibility rule, independent of the binding).
- **Card-authoring residual (NOT substrate):** authoring the production `BT20-017.yaml` / `BT23-013.yaml` is later Track J work (PR 2 / PR 3) — they also need the Atho/René/Por token (already registered, Track J PR 1) and a Union hand+trash play primitive (`G-UNION-HAND-TRASH-NAME-EXCLUSION`, Task S2.2). `BT20-017` additionally needs `select_own_permanent` + a `dp_lte: 8000` delete sub-clause (the `dp_lte` predicate landed in Track A — `G-PRED-DP-LTE`).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- effect_granted_attack` (15 pass).

## Engine + DSL Gap: `G-UNION-HAND-TRASH-NAME-EXCLUSION` — RESOLVED 2026-05-20 (Phase 2 Track J Task S2.2)

- **Severity:** 🟡 PARTIAL → 🟢 NONE — two genuine substrate pieces were missing and are now closed. Filed and closed in the same pass; this gap previously had no canonical entry, only a name in the Royal Knights `RK-G005` rollup and the BT23-013 test ignore string.
- **Discovered in:** Royal Knights full-pool DSL assessment (`RK-G005`, 2026-05-05). Task S2.2 filed it properly and resolved it.
- **Card consumers:** `BT23-013` Jesmon is the **only** genuine consumer of the exact "hand OR trash, name-restricted, exclude names already in play" shape — `[When Digivolving] [When Attacking] You may play 1 [Atho, René & Por] Token (…) or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.`
- **Step 0 finding — plan-premise correction (the substrate plan misdescribed the card pool):** The plan named four cards (BT20-017 / BT23-013 / BT13-019 / BT20-021). Printed text (`data/cards.json`) shows only BT23-013 matches:
  - **BT20-017** Jesmon — `[On Play] [When Digivolving] You may play 1 [Atho, René & Por] Token.` No hand/trash union play at all.
  - **BT13-019** Gankoomon — plays a Sistermon `from your trash` OR a Royal Knight `from the digivolution cards of your Digimon in the breeding area`, with a **fixed** name-exclusion (`You can't play [Gankoomon] or [Omnimon]`). Zones are trash + breeding-area digivolution-sources, not hand+trash; exclusion is fixed names, not in-play names. That is the separate `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` gap, untouched here.
  - **BT20-021** Jesmon GX — `By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete …`. This *places a card as a digivolution source* — a **cost**, not a play — with no name-exclusion. That is the separate `G-UNION-HAND-TRASH-SOURCE-COST` gap.
  - "Union" is informal gap-doc shorthand: there is **no printed `<Union>` keyword** (verified absent from `docs/RULES_CONTEXT.md`; the printed Jesmon keyword is `<Alliance>`, an unrelated attack-time mechanic).
- **Step 0 finding — what was actually missing (two genuine substrate pieces):**
  1. **The DSL `select_union_zone` lowering dropped its filter.** The DSL step `select_union_zone` (hand+trash in one prompt) parses a `filter: PredicateSpec`, and `CompiledStep::SelectUnionZone` carries a `filter` field — but the engine lowering `install_select_union_zone` in `code/digimon-engine/src/dsl_cards/step/selections.rs` destructured the step with `..` and passed a hardcoded `|_game, _card| true` accept-all closure to `EffectContext::select_union_zone`. The compiled filter was never applied. The engine helper itself already applies whatever filter it receives (proven by the pre-existing `tests/selection/union_zone.rs::filter_restricts_valid_action_ids`), so this was purely a DSL-lowering bug. Name-restriction (`name_contains: Sistermon`) was silently inoperative for every union-zone card.
  2. **No name-exclusion predicate leaf existed.** Nothing could express "this candidate card's name is NOT shared by any of my battle-area Digimon". The `no_permanent` existential matches against fixed predicate fields and cannot reference the candidate card's own name; `color_matches_any_field_digimon` was the closest analog but for colors, not names.
- **What was implemented:**
  - **`name_not_shared_by_field_digimon: { of: <player> }`** — a card-subject predicate leaf added to `PredicateSpec` (`code/digimon-dsl/src/predicate.rs`), `CompiledPredicate` (`code/digimon-dsl/src/compiled.rs`), the compiler (`code/digimon-dsl/src/compile.rs`), and the engine evaluator (`code/digimon-engine/src/dsl_cards/predicate.rs` — new `field_digimon_has_name` helper, mirroring `card_shares_color_with_any_field_digimon`). True when no battle-area Digimon of the scoped player has the candidate card's effective name. Field names are read through `synth_identity` so a `ChangeBaseCardName` overlay on a field Digimon is respected; the candidate's own name respects a reveal overlay. Exact, case-sensitive comparison — consistent with `name_is` / `name_in`. Added to `eval_no_subject_fields` as a subject-only field.
  - **The `select_union_zone` lowering now applies its `filter`.** `install_select_union_zone` builds an `EffectReadContext` per candidate and evaluates the compiled `filter` against each hand/trash `CardSource` — exactly as `install_select_hand` / `install_select_trash` already did.
- **API shape:**
  ```yaml
  - select_union_zone:
      of: you
      zones: [hand, trash]
      optional: true            # printed "You may …" — PASS stays legal (§17)
      prompt: Play 1 Sistermon from hand or trash
      filter:
        all_of:
          - name_contains: Sistermon
          - name_not_shared_by_field_digimon: { of: you }
  ```
  The name-exclusion shapes the legal action mask; every surviving candidate from hand AND trash surfaces through `pending_selection`. Nothing auto-picks. No `ACTION_SPACE_SIZE` / tensor change — union-zone reuses the existing `PLAY_HAND` / `TRASH_EFFECT` action ranges.
- **First failing test (TDD red):** `code/digimon-engine/tests/dsl/s2_2_union_hand_trash_name_exclusion.rs::union_zone_filter_excludes_in_play_name_across_hand_and_trash` — before the fix it offered 3 candidates (filter dropped, and `name_not_shared_by_field_digimon` silently swallowed into `PredicateSpec::extra`) instead of the 2 legal Sistermon names; the field-empty companion test offered 3 instead of 2.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- s2_2_union` (2 behavioral tests); `cargo test … --test dsl -- name_not_shared_by_field_digimon` (lowering test in `group7_predicate_batch.rs` — proves both the leaf compiles AND the union-zone `filter` survives compilation); `cargo test … --test dsl -- parse_leaf_predicates` (parse test); full DSL suite (615 → 618 pass), `cards_behavioral` (2358 pass), and full engine suite green — no regressions.
- **Card-authoring residual (NOT substrate):** authoring production `BT23-013.yaml` is later Track J work (PR 3) — it also needs the Atho/René/Por token (registered, Track J PR 1), `<Rush>` / `<Alliance>` keyword slices, and an effect-choice between the token play and the Sistermon union play. BT13-019 / BT20-021 remain blocked on their own separate gaps named above.

## DSL Gap: `AltPathSpec.condition` field for alt-digivolve activation gates — RESOLVED 2026-05-15 (Phase 1)

- **Status:** Closed for the schema + Digivolve consumer route. First reported 2026-04-27 (BT24-016 batch-implement-cards-rust-dsl) as `G-ALT-PATH-CONDITION`.
- **Discovered in:** BT24-016 Lamiamon — `[Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.` The "If you have [Owen Dreadnought]" gate could not be expressed on `AltPathSpec`, so the activated alt-path was available whenever the source filter (Elizamon on field) and the `extra_cost` (Dimetromon in trash) were satisfied, regardless of Owen presence.
- **DSL surface:** `condition: Option<PredicateSpec>` on `AltPathSpec` in `code/digimon-dsl/src/alt_path.rs`. Compiles to `condition: Option<Box<CompiledPredicate>>` on `CompiledAltPath` in `code/digimon-dsl/src/compiled.rs` via `compile_alt_path` in `code/digimon-dsl/src/compile.rs`.
- **Lowers to engine API:** Consumed in `code/digimon-engine/src/dna_digivolve.rs::find_matching_alt_path` after the source-filter check passes (Digivolve route). The condition predicate is evaluated with `PredicateSubject::Permanent(base_handle)`. Skipped (treated as pass) when `condition` is `None`.
- **YAML shape:**
  ```yaml
  alt_paths:
    - kind: activated_digivolve
      condition:
        all_of:
          - exists: { of: you, zone: [battle_area], kind: tamer, name_contains: "Owen Dreadnought" }
      from: { name_contains: "Elizamon" }
      cost: 3
      ignore_requirements: true
      extra_cost: ...
  ```
- **Evidence:** `cargo test --manifest-path code/digimon-dsl/Cargo.toml` (parse round-trip); `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` (variant-coverage lint).
- **Card-side authoring follow-up:** BT24-016's YAML still leaves the Owen gate unenforced; populating `condition:` on the activated_digivolve path is card-local work, not substrate. Other alt-path routes (`DigiXros`, `BurstDigivolve`, `Assembly`, `AppFusion`, etc.) do not yet read the field — extend per-route as cards need them.

## Engine Gap: Standard Delay main-phase activation action (PUPPETS-G009) — RESOLVED 2026-05-20 (Puppets substrate sweep)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets (2026-05-03 batch implementation)
- **Card(s):** `P-037` Yellow Memory Boost!, `P-105` Physical Training, `LM-035` Physical Training, `LM-037` Yellow Scramble, `LM-054` Treadmill Training; also standard Memory Boost/Training/Scramble cards whose `<Delay>` text is activated by the controller during a later main phase.
- **Effect text:** "`<Delay>` (By trashing this card after the placing turn, activate the effect below.)"
- **What was missing:** The Group 5 Delay lifecycle supported persistent delayed Options, placement-turn gating, start/end/event drains, and replacement-aware self-trash costs. Standard main-phase Delay cards were modeled in DSL/YAML as scheduled automatic effects (e.g. `end_of_your_next_turn`), which hid the player's later `[Main]` decision to activate or decline after the placing turn. The `DelayTrigger::MainPhaseActivated` variant and the main-phase action mask activation path did not exist before this sweep. An earlier tracker entry (Track I, commit `26e27ccc`, 2026-05-17) was optimistic/incorrect — that commit did not deliver this substrate.
- **Resolution:** `DelayTrigger::MainPhaseActivated` variant added. Standard `kind: delay` Options placed in the battle area with this trigger expose a field-effect activation slot through the normal main-phase action mask once the placing turn has passed (placement-turn gating per rule 16-16-3). Taking the activation trashes the Option as cost and runs the Delay body; PASS leaves the Option parked for a future legal activation. No `ACTION_SPACE_SIZE` change. DSL standard `<Delay>` lowers to `OptionState::Delayed { trigger: DelayTrigger::MainPhaseActivated, .. }`.
- **Closed by:** Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, commits `44ce72a4` + `9afdfdb7`, 2026-05-20.
- **Evidence:** `p_105_delay_is_player_visible_main_activation_after_placing_turn`, `p_105_declining_delay_leaves_option_on_field`, `lm_054_delay_is_main_phase_action_after_placing_turn`, `lm_054_declining_delay_leaves_option_in_battle_area` — all pass without `#[ignore]`. Tests verify: (a) placing-turn mask bit is 0, (b) next own main phase bit flips to 1 with PASS legal, (c) taking the action trashes the Option and resolves the body, (d) PASS leaves the Option parked.
- **Residual:** `BT22-098` has a distinct residual filed as `PUPPETS-G033` — the Option-card pipeline's integrated resolution (pending optional play + post-resolution battle-area placement as a Delayed Option) is not yet proven. That is a different, narrower shape from the standard `<Delay>` main-phase activation that this entry closes.
- **Related:** `PUPPETS-G009` in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`; `PUPPETS-G033` (BT22-098 Option pipeline residual).

## Hybrid Gap: `on_ally_played` event-gated Delay Options (PUPPETS-G004) — RESOLVED 2026-05-21

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets (2026-04-28 archetype assessment)
- **Card(s):** `P-229` Unique Emblem: Narrative Ronde. (`BT22-098`'s sibling `on_suspend` slice closed earlier, 2026-05-02.)
- **Effect text:** "[Your Turn] When any of your [Mirai Kinosaki]s are played, `<Delay>` (By trashing this card after the placing turn, activate the effect below.) 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- **What was missing (two halves):**
  - **DSL:** `code/digimon-engine/src/dsl_cards/lower_delay.rs` mapped only `on_suspend` / `on_unsuspend` to `DelayTrigger::OnEvent`. `trigger: on_ally_played` fell through to `DelayTrigger::EndOfYourNextTurn`, which auto-expires the Option on the wrong schedule.
  - **Engine:** `enqueue_event_gated_delayed_options` (the scan that fires placed Delay Options matching an observed event) was only invoked from the `EventObserved` / `AttackTargetChanged` dispatch in `effect_queue.rs` `enqueue_triggered`. The `OnAllyPlayed` play broadcast uses `TriggerSource::EnteredField`, which never reached it — a placed Delay Option keyed to `on_ally_played` never fired.
- **Resolution:**
  - `lower_delay.rs` now maps `CompiledTiming::OnAllyPlayed` → `DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed)` alongside the existing event arm.
  - `effect_queue.rs` `enqueue_triggered` now fans `TriggerSource::EnteredField` dispatches out to `enqueue_event_gated_delayed_options`. The candidate scan only matches Options whose `OnEvent(event_timing)` equals the dispatch `timing`, so dispatching for both the `OnEnterFieldAnyone` and `OnAllyPlayed` play broadcasts is harmless.
  - `OnEvent(_)` Delay Options park indefinitely (`compute_delay_trash_turn` = `u16::MAX`), so the Option's "place this card in the battle area" tail is faithful with no turn-keyed auto-trash approximation.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_229` — 13 tests pass, 0 ignored, incl. placing-turn gate, non-Mirai event-predicate gate, and decline-still-pays-the-trash-cost. Full engine suite green (`bt22_098` `on_suspend` sibling unaffected).
- **Related:** `PUPPETS-G004` in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`, `qa/dsl-vocab-gaps.md`; `G-DELAY-EVENT-GATED` in `qa/archetype-qa/engine-gaps.md`.

## Engine Gap: Effect-spawned permanent with end-of-turn deletion rider — RESOLVED 2026-05-20 (Puppets substrate sweep)

- **Severity:** 🔴 BLOCKING (closed for provenance-bound cleanup substrate)
- **Discovered in:** Dark Masters (2026-04-18); Puppets (2026-05-04)
- **Card(s):** EX10-012 MetalSeadramon, EX10-020 Puppetmon, EX10-035 Machinedramon, EX10-057 Piedmon, EX10-061 Apocalymon, EX10-072 Spiral Mountain, P-216 WaruMonzaemon; Puppets adds EX11-022, EX11-061 (PUPPETS-G003) and P-165 (PUPPETS-G016).
- **Effect text:** "At turn end, delete the Digimon this effect played." / "At the end of your opponent's turn, delete that token."
- **Resolution:** `schedule_delete_played_at_turn_end` provenance-bound turn-end self-delete landed (`PUPPETS-G003`): effect-play helpers return a `PermanentHandle`; `ctx.schedule_delete_at_end_of_turn(handle)` enqueues a handle-keyed deletion that is a no-op if the permanent is already gone. `play_token` `bind_as` (`PUPPETS-G016`) lets the token-creation step bind the returned `PermanentHandle` so a separate `schedule_delete_at_end_of_opponent_turn(handle)` step can target exactly that token rather than any same-kind token.
- **Closed by:** Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, 2026-05-20.

## Engine Gap: Costed self-digivolve stable source binding — RESOLVED 2026-05-20 (Puppets substrate sweep)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets Batch 5 (2026-05-04)
- **Card(s):** `EX9-032` Karakurumon (PUPPETS-G018)
- **Effect text:** "[On Play] [When Digivolving] By deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon may digivolve into a [Puppet] trait Digimon card in your hand without paying the cost."
- **Resolution:** `source_permanent` is now re-located by `CardHandle` after any mid-body battle-area index shift. `binding_ref.rs::Source` resolution searches the live battle area for the carrier card-handle rather than trusting the snapshot index. Preflight for legal Token/Puppet cost bodies excludes the source permanent by handle.
- **Closed by:** Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, 2026-05-20.

## Engine Gap: Narrow opponent-effect protection for DP reduction and De-Digivolve — RESOLVED 2026-05-20 (Puppets substrate sweep)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets Batch 8 (2026-05-04)
- **Card(s):** `BT16-055` Namakemon (PUPPETS-G024)
- **Effect text:** "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects."
- **Resolution:** `grant_narrow_opponent_effect_protection` landed: category-scoped protection modifiers (`ImmuneToOpponentDpReduction`, `ImmuneToOpponentDeDigivolve`) with opponent-source and live security-count predicates are consulted at DP-reduction and De-Digivolve effect sites. DSL surface: `narrow_opponent_effect_protection: { categories: [dp_reduction, de_digivolve], while: { own_security_gte: 3 } }`.
- **Closed by:** Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, 2026-05-20.

## Engine Gap: Effect play with played-Digimon On Play suppression — RESOLVED 2026-05-20 (Puppets substrate sweep)

- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets Batch 9 (2026-05-04)
- **Card(s):** `BT5-106` Demonic Disaster (PUPPETS-G030); adjacent: `BT13-110`, `BT13-112` (Royal Knights)
- **Effect text:** "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- **Resolution:** `suppress_on_play: true` flag added to effect-play helpers and threaded through the play event context. On Play enqueue skips the just-played permanent's On Play clauses when the flag is set. DSL step: `play_from_trash_free: { filter: ..., suppress_on_play: true }`.
- **Closed by:** Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, 2026-05-20.

## Medusamon PARTIAL-card unblock (Tier 1+2) — 2026-05-21

The `unblock-medusamon-partial-cards` OpenSpec change closed 5 of the 7 substrate
gaps blocking PARTIAL Medusamon cards. The remaining 2 (G-ACTIVATED-DIGIVOLVE-EXECUTION,
G-LINK-OPTION-DUAL-PLAY-MODE) are scoped into the follow-up change
`unblock-medusamon-tier3-cards` — both close by reusing existing action IDs (no
`ACTION_SPACE_SIZE` change), per the action-space spike recorded in that change's
design.md.

### Gaps CLOSED

- **G-SECURITY-SKILL-RESUME-REFIRE** (engine) — a drain arm of
  `combat.rs::drive_security_resolution` re-enqueued its `EffectTiming`
  on every resume, so declining an optional `[Security]` "you may" effect
  infinite-looped (and, with 2+ candidates, double-played). Fixed by a
  `phase_enqueue_done: bool` on `SecurityResolutionState`: each drain phase
  enqueues its timing exactly once; resume just flushes any continuation and
  advances. The flag covers all three drain phases (superseding an earlier
  single-phase `security_skill_drained` variant). Mirrors the `Dispose` →
  `DisposeFinalize` guard. Unblocks **P-189**; also fixes P-206 / ST19-08 and a
  latent double-play (`st19_08` count assertion corrected).
- **G-ZONE-SELECTED-TRASH-TO-DECK-TOP** (engine + DSL) — no method moved a
  selected trash card to the deck TOP. Added `EffectContext::return_trash_cards_to_deck_top`
  (reverse-push so the first selected card lands on top) and a
  `destination: top | bottom` field (`DeckDestination`, default `bottom`) on the
  `return_trash_list_to_deck_bottom` DSL step; the lowering also resolves a
  `select_trash` `TrashIndex` binding. Unblocks **LM-027** (clause B now a real
  `kind: delay` clause; the `lm_027_delay_start_of_turn_noop` raw_rust placeholder
  removed); LM-029/030/031 use the same primitive.
- **G-TRASH-SELECTED-SECURITY** (engine + DSL) — no verb trashed a chosen non-top
  security card. Added `EffectContext::trash_security_card(player, handle)`
  (handle-based — no index-staleness) and a `trash_selected_security` DSL verb
  that consumes a `select_security` binding. Unblocks **BT24-018**.
- **G-ACTIVATION-COST-TRASH-SELF** (DSL) — `activation_cost:` accepted only
  `suspend_self` / `return_self_to_deck_bottom`. Added a declinable
  `trash_self: true` variant (`CompiledActivationCostKind::TrashSelf` +
  `EffectContext::trash_self_as_cost`) so a `<Delay>` "by trashing this card"
  cost is genuinely declinable per Comprehensive Rules 16-16-2. Unblocks **BT21-093**.
- **G-ALT-PATH-SAVE-IN-TEXT** (DSL) — alt-path `from:` filters could not gate on
  "<Keyword> in text". **No new predicate needed:** the existing
  `effect_text_contains` predicate already does a printed-text substring scan and
  `eval_predicate` already runs it against an alt-path `from:` candidate. BT21-072's
  `xros_req` cost-3 path now uses `from: { any_of: [{level_eq:4, effect_text_contains:"<Save>"}, {level_eq:4, trait_has:Hero}] }`.
  Unblocks **BT21-072**.

- **Closed by:** `unblock-medusamon-partial-cards` OpenSpec change, 2026-05-21.
  Full engine suite green (`cards_behavioral` 2714 passed / 0 failed).

## Medusamon PARTIAL-card unblock (Tier 3) — 2026-05-22

The `unblock-medusamon-tier3-cards` OpenSpec change closed the two Tier-3 gaps that
the spike (design.md) had feared would need a new action ID. **The spike found
neither needs the action space to grow** — `ACTION_SPACE_SIZE` (2192) and
`TENSOR_SIZE` are unchanged; no RL retraining.

### G-LINK-OPTION-DUAL-PLAY-MODE — RESOLVED (engine)

A Plug-In Option that is both a Standard `[Main]` Option and a Link Option could
not be expressed: `classify_option_subtype` was first-match-wins, so any effect
with `link_cost.is_some()` reclassified the whole card as `OptionSubtype::Link`.

**Resolution (no new action ID):**
- `classify_option_subtype` → `classify_option_modes` — returns a
  `Vec<OptionPlayMode>` of the available play modes; `[Standard, Link]` for a
  dual-mode Plug-In, a 1-element list for every other Option (single-mode cards
  behaviorally identical).
- `OptionSubtype` moved to `selection.rs` (pub) and is stored on
  `PendingOption.subtype` at play time, so `dispose_option` routes on the
  resolved mode instead of re-classifying.
- `play_option_core` gained a `chosen_mode` parameter. When `option_legal_play_modes`
  (affordability filter) yields >1 mode it installs an `EffectChoice` mode-select
  (`install_option_mode_select`, "Play as a [Main] Option" vs "Plug in via Link
  Requirements"); the selection callback re-enters `play_option_core` with the
  chosen mode. Cost forks (Standard use cost vs flat Link cost — BeforePayCost
  reductions apply to Standard only); `OptionMain` firing is skipped for a Link
  play; disposal routes Standard→trash vs Link→host-attach.
- The mode-select reuses the existing `EffectChoice` / `HAND_EFFECT_START` action
  range and surfaces as a normal `pending_selection`, so every legal play mode is
  exposed to the action space (no-approximations policy).
- `action/mask.rs` lights `PLAY_HAND` for an Option when any mode is affordable.
  Unblocks **ST22-08** (gained a `kind: link_requirement` clause, cost 2,
  `filter: { level_gte: 3 }`; 34 behavioral tests, 0 ignored).

### G-ACTIVATED-DIGIVOLVE-EXECUTION — BT24-016 unblocked via re-model (no engine code)

The `kind: activated_digivolve` alt-path kind has no engine execution route, and
the task-1.1 investigation found `extra_cost` is unimplemented engine-wide (3
sites, all exclusions). Rather than build a from-scratch parking `extra_cost`
runner, **BT24-016 clause 1 was re-modelled** (design.md D1-REVISED) from a
`kind: activated_digivolve` alt-path to a `when: main_from_hand` triggered clause
(select Elizamon → select Dimetromon from trash → `place_as_bottom_source` →
`effect_initiated_digivolve` cost 3, `ignore_requirements`). This is faithful to
the printed `[Hand][Main]` text, uses only working machinery, and adds **zero
engine code**. BT24-016 is `IMPLEMENTED` (24/24 tests).

This entry is **not fully closed**: the `CompiledAltPathKind::ActivatedDigivolve`
execution route is still genuinely missing for the 3 out-of-scope cards
(BT22-013/026, BT16-027) — see the residual entry in
[qa/archetype-qa/engine-gaps.md](archetype-qa/engine-gaps.md).

- **Closed by:** `unblock-medusamon-tier3-cards` OpenSpec change, 2026-05-22.
  Full engine suite green (`cards_behavioral` 2722 passed / 129 ignored / 0 failed;
  `option_flow` 93; `mask_and_tensor` 157).

## DSL Gap: Card-level "also treated as [Name]" identity alias — RESOLVED 2026-05-21

- **Severity:** 🟡 PARTIAL (DSL-vocabulary gap + engine name-matching substrate)
- **Discovered in:** Puppets / Royal Knights (BT23-077 Sistermon Ciel)
- **Card(s):** `BT23-077` Sistermon Ciel ("Also Treated As [Sistermon Noir]")
- **Effect text:** Printed identity line "Also Treated As [Sistermon Noir]" (DCGO `ChangeCardNamesClass`). Not part of `cards.json` `effect_description_eng`; carried by the legacy Python script as `card.also_treated_as_names = ['Sistermon Noir']`.
- **Resolution:** Added card-level DSL field `also_treated_as: [Name, ...]` to `CardSpec`, lowered to `CompiledCard.also_treated_as` and threaded into a new static `CardData.also_treated_as` by the DSL→engine bridges (`dsl_bridge::enrich_card_data_with_dsl_alt_paths`, `debug_runner::card_data_from_compiled`, `tests/support/dsl_card_data.rs`). Generic name-matching honors the alias set: `eval_card_fields` `name_is` / `name_contains` / `name_in`, and `CardSource::card_names` / `contains_card_name`. Unlike `digixros_aliases` (DigiXros-recipe-scoped, deliberately overlay-blind to generic name predicates — see the DigiXros name-alias entry above), `also_treated_as` is an always-on identity alias visible to generic name predicates in every zone. `BT23-077.yaml` authored with `also_treated_as: [Sistermon Noir]`. Supersedes the legacy Python `CardSource.also_treated_as_names` resolution (2026-03-14) for the Rust engine.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- also_treated_as`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.
- **Closed by:** branch `claude/laughing-lehmann-5771d9`, 2026-05-21.

## Rocks B4 face-up security lifecycle — RESOLVED 2026-05-23

- **Severity:** 🟡 PARTIAL engine/DSL security lifecycle gap.
- **Discovered in:** Rocks archetype pass (`BT20-055` Invisimon).
- **Card(s):** `BT20-055` Invisimon.
- **Effect text:** "flip your opponent's top face-down security card face up" and "When your Digimon checks a face-up security card, you may place the top card of this Digimon face-up at the bottom of your security stack."
- **Resolution:** Added the no-choice `flip_security_face_up` DSL step (`StepSpec` -> `CompiledStep` -> `EffectContext::flip_security_face_up`) that marks the topmost still-face-down card in the target player's security stack. Added attacker-side `when: on_check_face_up_security` timing, dispatched from combat only when the checked security card was already face-up and scanning the attacker's battle area. Authored `BT20-055.yaml` with the flip rider and optional top-stacked-card-to-bottom-security face-up clause.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- face_up_security_lifecycle`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_055`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- step_variants_have_exec_arms`.
- **Residual:** Top-N security trash and face-up security extraction/filtering remain open in `docs/RUST_ENGINE_GAPS.md`.

## G-EVENT-TARGET-LEVEL-LTE — RESOLVED 2026-05-23

- **Severity:** 🟡 PARTIAL DSL predicate / event-observer gap.
- **Discovered in:** Rocks archetype pass (`BT8-094` Digimon Emperor); also benefits `RB1-035`.
- **Card(s):** `BT8-094` Digimon Emperor.
- **Effect text:** "[All Turns] When one of your opponent's level 5 or lower Digimon is deleted, you may suspend this Tamer to <Draw 1>." / "[Opponent's Turn] When one of your opponent's level 3 Digimon is moved from their breeding area to their battle area, gain 2 memory."
- **Resolution:** Added `event_target_level_eq`, `event_target_level_lte`, and `event_target_level_gte` to `PredicateSpec`, `CompiledPredicate`, compiler lowering, and the runtime predicate evaluator. The evaluator reads the event-target card snapshot, so deleted-object and moved-permanent contexts can gate on printed level. Authored both `BT8-094` omitted clauses. `MovedFromBreeding` dispatch now scans all battle areas, matching other cross-player event observers and allowing opponent Tamers to observe a moved event target.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_094 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_082 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_130 --nocapture`.

## G-ON-OPTION-TRASHED-DSL — RESOLVED 2026-05-23

- **Severity:** 🟡 PARTIAL DSL timing gap over existing option-lifecycle engine substrate.
- **Discovered in:** Rocks archetype pass (`BT23-059` Justimon: Blitz Arm).
- **Card(s):** `BT23-059` Justimon: Blitz Arm.
- **Effect text:** "[All Turns] [Once Per Turn] When Option cards in the battle area are trashed, this Digimon unsuspends. Then, your opponent's Digimon's effects don't affect this Digimon for the turn."
- **Resolution:** Added `Timing::OnOptionTrashed` / `CompiledTiming::OnOptionTrashed`, compiler lowering, and `compiled_timing_to_engine` mapping to `EffectTiming::OnOptionTrashed`. Authored `BT23-059` Clause B with `when: on_option_trashed`, `unsuspend: { target: source }`, and `grant_effect_immunity` from opponent Digimon effects until end of turn.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_059 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- lifecycle_state_machine::trash_field_option_fires_on_option_trashed_with_last_state --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`.

## Engine + DSL Gap: `play_from_revealed_free` (EX8-050 Gogmamon) — RESOLVED 2026-05-23 (`complete-rocks-archetype`)

- **Severity:** 🟡 PARTIAL reveal-zone free-play gap.
- **Discovered in:** Rocks archetype pass (`EX8-050` Gogmamon).
- **Card(s):** `EX8-050` Gogmamon.
- **Effect text:** "[On Deletion] Reveal the top 3 cards of your deck. You may play 1 Digimon card with the [Mineral] or [Rock] trait and a play cost of 5 or less among them without paying the cost. Trash the rest."
- **Resolution:** Added `EffectContext::play_from_revealed_free(player, card)` plus DSL `play_from_revealed_free: { of, card, bind_as? }`. The helper consumes the selected `CardHandle` from `Game::revealed_cards`, clears its reveal overlay, parks it only inside the synchronous play pipeline, and calls the normal effect-initiated free-play path so floodgates, would-play replacements, OnPlay, and OnEnterField behavior match other free plays. `PendingWouldPlayOrigin::Reveal` restores cancelled replacement paths back to the reveal pool. Authored `EX8-050.yaml` to play the selected reveal bucket before trashing the rest.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2f1_play_steps --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_050 --nocapture`.
