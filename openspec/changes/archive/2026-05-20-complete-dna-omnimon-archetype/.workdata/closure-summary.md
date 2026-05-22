# DNA Omnimon completion — closure facts (for tracker reconciliation)

Change: `complete-dna-omnimon-archetype`. Date: 2026-05-20.
Verdict ledger end state: 64 cards — 62 IMPLEMENTED, 2 PARTIAL, 0 BLOCKED
(baseline at Phase A sweep: 34 IMPLEMENTED / 25 PARTIAL / 5 BLOCKED).

## Substrate gaps CLOSED (new engine/DSL capability landed)

Small DSL gaps:
- G-DSL-PREDICATE-TEXT-CONTAINS — `effect_text_contains` predicate leaf
- G-EVENT-TARGET-NAME-CONTAINS — `event_target_name_contains` predicate leaf
- G-FORMULA-SOURCE-DP — `source_dp` FormulaSpec variant
- G-PLAY-COST-AGGREGATE — `LowestPlayCost` AggregateSelector/FieldSelector
- G-SELF-DIGIVOLUTION-CONTAINS-NAME — `self_digivolution_sources_contain_name` (sources-only) predicate
- G-ALT-PATH-DIRECTION-INTO companion — `distinct_tamer_colors_gte` predicate leaf

Medium DSL/engine gaps:
- G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH — `min:` on select_count_capped_multi + `return_trash_list_to_deck_bottom` step + atomic cost-then-cancel guard
- G-DSL-UNION-PLAY-FREE — `select_union_zone` widened to bind a zone-tagged index
- G-IGNORE-COLOR-MASK — from-hand color-requirement bypass via card-level `use_requirement`
- G-MULTI-SELECT-OPP-PLAY-COST-SUM — `SelectOpponentPlayCostBudget` step + `SelectionKind::PlayCostBudget`
- G-OPT-MULTI-TIMING-SHARED-LOCKOUT — `Effect::shared_opt_group` shared once-per-turn key
- G-OUTER-OPTIONAL-NOT-INSTALLED — outer accept/decline PendingSelection for a lone optional triggered effect
- G-PRED-NO-FACE-UP-SECURITY-NAMED — `no_face_up_security_named` predicate leaf
- G-SELECT-EMPTY-OUTER-TAIL — `select_hand` empty-candidate path drains the outer tail
- BT5-092 cost-reduction trigger — `when_any_ally_digivolves_into` CostReductionBody trigger

Deep engine gaps:
- G-DSL-DNA-FROM-HAND-PARTNER — `effect_initiated_dna_digivolve_with_hand_partner` engine API + DSL step
- G-FOR-EACH-DELETE-INDEX-SHIFT — `ForEach` re-resolves stable top-card identity per iteration
- AD1-012 defender-side effect-initiated DNA mid-attack-interrupt — closed (no engine redesign needed; authored on existing interrupt substrate)

Additional gaps closed during card authoring:
- G-AURA-GRANTED-SECURITY-KEYWORD — `Modifiers::granted_security_attack_keyword_bonus`
- G-FORMULA-COST-DELTA — formula-valued `CostDelta::ReduceFn` on play_from_hand
- G-EVENT-TARGET-COLOR — `event_target_color_any_of` predicate leaf
- G-PLAY-SELECTED-SECURITY-CARD — `play_security_card` step + `EffectContext::play_from_security_card`
- source_material_count formula; OnLeaveField timing wired to fire from deletion + return paths; binding_count_eq predicate; ResolvedBinding::SourceRefs + per_selected over source refs

## Gaps that were STALE (tracker said open / `#[ignore]` said pending, but substrate already existed and is now USED)

G-COUNT-AGGREGATE, G-COUNT-LTE-EVAL, G-DECLARATIVE-KEYWORD, G-DSL-EVENT-TARGET-IS-OTHER,
G-DSL-EVENT-TARGET-IS-SELF, G-OPT-TRIGGERED, G-PLAY-COST-GTE, G-PLAY-COST-LTE,
G-SECURITY-ZONE-AURA-SOURCE, G-DSL-SOURCE-NAME-CONTAINS, G-DSL-SELECT-OWN-SOURCES-FILTER,
G-DSL-DISTINCT-TAMER-COLORS-FORMULA, G-PLACE-SELF-AS-OPTION-PERMANENT,
G-ADD-OPTION-SELF-TO-HAND, G-EVENT-CARD-TAMER-PLAY, G-COLOR-MATCH-AGAINST-BOARD,
G-DSL-SELF-NAME-CONTAINS, G-EVENT-TARGET-NOT-SOURCE.
(All now have the DNA Omnimon card clauses authored and their behavioral tests re-enabled and passing.)

## Gaps STILL OPEN (verified open against code; filed, not closed by this change)

- G-DYNAMIC-NAME-ALIAS-FROM-STACK — BT17-102 `[All Turns]` material-name-alias clause. The DSL identity layer has only static `name_aliases`; there is NO engine consumer for a dynamic alias derived from the live digivolution-source stack. A faithful fix is a cross-cutting engine feature (Permanent-level effective-name-set query consulted by every name predicate). BT17-102 is otherwise IMPLEMENTED; this one clause is omitted, test `bt17_102_all_turns_aliases_low_level_material_names` left `#[ignore]`'d.
- G-DSL-DELAY-ON-ATTACK-EVENT (with G-DSL-ON-ALLY-ATTACK-TIMING / G-ATK-TRAIT-FILTER noted as already-present) — BT23-096 `<Delay>`-on-attack clause. 3-part engine blocker: `lower_delay.rs` does not map attack timings to `DelayTrigger::OnEvent`; `combat.rs` dispatches `OnAllyAttack` via `TriggerSource::PlayerBattleArea` which `effect_queue.rs` never fans out to event-gated delays; `attacker_trait_has` resolves the attacker only via `attack_target_change()` (unset for a plain attack). BT23-096 otherwise IMPLEMENTED; clause omitted, test left `#[ignore]`'d.

## raw_rust

DNA Omnimon now has 0 live `raw_rust` escapes (BT20-102 board-wipe migrated to pure DSL; AD1-025 body migrated; the unused `bt20_102_boardwipe_and_return` fn removed).
