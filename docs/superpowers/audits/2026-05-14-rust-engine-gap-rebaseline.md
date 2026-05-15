# Rust Engine Gap Rebaseline Audit

**Date:** 2026-05-14
**Source:** `docs/RUST_ENGINE_GAPS.md` (1485 lines, last hygiene sweep 2026-05-14)
**Method:** Read each "Open gaps" entry's "Suggested API shape" / "What's missing" prose, grep engine source for matching symbols, cross-reference inline "Updated YYYY-MM-DD" PR-closure notes against `code/digimon-engine/src/`. No engine code or tests modified.

## Summary by verdict

| Verdict | Count |
|---|---|
| **CLOSED** — primitive fully exists, header severity is stale | 8 |
| **NARROW** — primitive partially landed, residual is real but smaller than the headline | 16 |
| **OPEN** — claim still accurate, no substrate landed | 12 |
| **UNCLEAR** — need engineering eyes | 2 |

The headline-table severity badges in `## At a glance` (lines 117–158) are systematically out of date: nearly every `🔴 BLOCKING` row that was discovered in the April audit cycle has had at least one slice land and most have multiple. The per-entry prose is mostly fresh (sweep notes through 2026-05-11), but **the entry-level "Severity:" line at the top of each section** still reads the original April severity in many cases. This is the primary leverage point for the next cleanup pass.

The biggest CLOSED candidates ready to migrate to `qa/resolved-gaps.md`:

1. **Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings** — Phase 1 (#449-ish) + Track A landed every dispatch/payload piece the entry asked for.
2. **`OnPlaceSecurity` / `OnAddedToSecurity` observer timing dispatch** — Track A landed both the dispatcher and the alias.
3. **Phase-granular turn timings** — all 5 timings shipped in Phase 1; breeding fan-out shipped in Track A.
4. **Observer timings tied to specific events** — all six listed observers landed (`OnDigivolve`, `OnSuspend`/`OnUnsuspend`, `OnAttackTargetChange`, `OnHatch`, `OnMove`); the entry already calls itself "5 of 6 wired" but in fact all 6 are wired.
5. **`<Barrier>` keyword, `<Scapegoat>` keyword, `<Retaliation>` keyword, `<Piercing>` keyword, `<Reboot>` keyword, `<Progress>` keyword** — all marked ✅ in the prose but counted under "open gaps" in the at-a-glance. These can simply move out.
6. **Granted triggered ability — attach an `Effect` to another permanent** — entry says 🟡 PARTIAL but the prose ends "both YAML and raw_rust authoring paths now have full typed surfaces" with only a memory-overhead nit remaining; should reframe to CLOSED with a "polish" note.
7. **De-Digivolve N primitive** — already 🟢 CLOSED in the header table; just confirm.
8. **Forced opponent hand reduction primitive** — already 🟢 CLOSED in the header table.

The NARROW cluster is dominated by **zone-manipulation umbrella entries** (security stack ops, return-to-hand/deck, effect-initiated digivolve, play-from-X-free): the core primitives all shipped in Phase 2/Track A/Track E, but each entry accumulates new sub-items (e.g. cast-time stack-construction, suppress-OnPlay, top-N security trash) that should be split out into their own narrowly-titled gaps so the headline severity can drop.

---

## CLOSED — move to `qa/resolved-gaps.md`

### Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings
- **Current claim:** 🔴 BLOCKING per at-a-glance table (line 120)
- **Code state:** Both fan-outs landed Phase 1: `Effect::on_any_digimon_played` (effect.rs:417, alias for `OnEnterFieldAnyone`), `Effect::on_any_deletion` (effect.rs:428). `EffectTiming::OnEnterFieldAnyone` and `OnAnyDeletion` are dispatched from `game_actions.rs` and `combat.rs::delete_permanent_with_effects`. Track A added `effect_initiated` bit on `TriggerContext` and DSL `event_is_effect_initiated`. PUPPETS-G011 closure (2026-05-08) added deleted-object snapshot predicates `event_target_kind`/`event_target_trait_has`/`event_permanent_is_source`/`source_is_unsuspended`.
- **Verdict:** CLOSED
- **Recommended action:** Move to `qa/resolved-gaps.md` citing Phase 1 + Track A (PRs #449, #451, #472). The entry currently spans ~30 inline update lines that read as ongoing slice work but are all already-shipped. Header severity should at minimum drop to ✅ in the at-a-glance.

### Phase-granular turn timings (`StartOfYourMainPhase`, `WhenAttacking`, `EndOfAttack`, `EndOfBattle`)
- **Current claim:** 🔴 BLOCKING (line 121)
- **Code state:** All five timings wired in Phase 1: `EffectTiming::StartOfYourTurn` / `StartOfYourMainPhase` / `WhenAttacking` / `EndOfAttack` / `EndOfBattle` exist in `enums.rs:206-219`; builders `Effect::start_of_your_turn/start_of_your_main_phase/when_attacking/end_of_attack/end_of_battle` in `effect.rs`. Dispatch sites in `game_phases.rs` (begin_turn / enter_main_phase) and `combat.rs` (fire_when_attacking / cleanup_attack / resolve_battle). Track A added breeding-area fan-out for `StartOfYourMainPhase`.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 1. The entry has not had a new substrate ask since 2026-04-19.

### Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`, `OnHatch`, `OnAllyAttack`/`OnOpponentAttack`)
- **Current claim:** 🔴 BLOCKING (line 122)
- **Code state:** Six observer variants all wired. `Effect::on_digivolve/on_suspend/on_unsuspend/on_hatch/on_attack_target_change/on_move` all exist (effect.rs); `OnAllyAttack`/`OnOpponentAttack` fire-sites in `combat::fire_on_attack` (combat.rs); DNA-origin bit (Track A), prompted retarget (2026-05-08), self-scoped predicate (2026-05-08) all landed.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 1 + Track A (PRs #449, #450, #451). The "Status (2026-05-07)/(2026-05-08)" prose lists every sub-piece as already shipped.

### `WhenWouldBeDeleted` / leave-field replacement-effect framework
- **Current claim:** ✅ RESOLVED / TRACK B VERIFIED at entry level — but still counts as a row in the open-gaps section (line 123 of at-a-glance)
- **Code state:** Phase C (2026-04-25) + Phase D (2026-04-25) + Track B closeout (2026-05-08) shipped all replacement primitives. `Game.parked_replacement`, `cancel_leave`/`handle_replacement`/`redirect_replacement`/`substitute_replacement` outcome setters, all seven alpha-tier keyword auto-installs (Fragment / ArmorPurge / Save / Decoy / Fortitude / Partition / MaterialSave).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase C + Phase D + Track B (PR #449). The entry already prefixes with ✅ but the at-a-glance row still reads 🔴.

### `OnPlaceSecurity` / `OnAddedToSecurity` observer timing dispatch
- **Current claim:** 🟡 PARTIAL (line 883)
- **Code state:** Track A landed the full dispatcher with payload (`event_card`, `affected_player`, `source_player`, `EventCause::SecurityPlacement`, moved-card set). `EffectTiming::OnPlaceSecurity` fires from `place_on_security` commits and `on_added_to_security` is an alias. `effect.rs:512` builder exists. DSL `when: on_place_security` and `when: on_added_to_security` both lower to the same dispatcher.
- **Verdict:** CLOSED for printed-effect security placement; "remaining setup/recovery multi-card additions" is card-shaped proof work, not engine substrate.
- **Recommended action:** Move to resolved-gaps.md citing Track A (PR #451). The 🟡 severity is overstated for an engine entry whose only "open" item is card test coverage.

### Forced opponent hand reduction primitive (`ctx.trash_opponent_hand_to_count`)
- **Current claim:** 🟢 CLOSED at entry level (already)
- **Code state:** `EffectContext::trash_opponent_hand_to_count` at `effect_context/mod.rs:4021`; DSL verb landed 2026-05-09.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Track E (PR #454). Already labeled closed in the prose — just relocate the section.

### De-Digivolve N primitive (single + mass)
- **Current claim:** 🟢 CLOSED at entry level
- **Code state:** `EffectContext::de_digivolve` at `effect_context/mod.rs:1966`, Phase 10 closure.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 10.

### `OnDiscardSecurity` — effect-driven security-card trash trigger
- **Current claim:** 🟢 CLOSED at entry level
- **Code state:** `EffectTiming::OnDiscardSecurity` in `enums.rs:202`; `Effect::on_discard_security` at `effect.rs:362`; `TriggerSource::SecurityDiscarded` (Track A 2026-05-08); test coverage cited.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Track A.

---

## NARROW — real residual exists, but severity is overstated; recommend split or reframe

### Global `OnOpponentSecurityRemoved` observer timing
- **Current claim:** 🔴 BLOCKING (line 119)
- **Code state:** Phase 1 (2026-04-19) + 2026-05-06 typed-payload sweep + 2026-05-08 breeding fan-out all landed. `combat.rs:2613,2629` dispatches both `OnOpponentSecurityRemoved` and `OnOwnSecurityRemoved` with `TriggerSource::SecurityRemoved` payload. Battle and effect-driven security removal both covered; breeding-resident observers covered by `enqueue_from_breeding_permanent`.
- **Verdict:** NARROW
- **Recommended action:** Drop severity 🔴 → ✅ RESOLVED with note "card-local authoring + non-battle-zone setup/Recovery fan-out remain card-shaped follow-ups". The prose explicitly says "remaining work under this heading is card-local authoring/selection behavior" — that's not an engine gap.

### Global `OnOwnSecurityRemoved` observer timing (mirror of `OnOpponentSecurityRemoved`)
- **Current claim:** 🔴 BLOCKING (line 1079)
- **Code state:** `EffectTiming::OnOwnSecurityRemoved` exists (enums.rs:355), fire-site in `combat.rs:2613`, builder `Effect::on_own_security_removed` at `effect.rs:495`. The companion entry explicitly says "Resolve as part of the same dispatch as `OnOpponentSecurityRemoved`" and that dispatch already shipped (2026-05-06).
- **Verdict:** CLOSED — but I'm listing under NARROW because the prose doesn't reflect the closure.
- **Recommended action:** Mark ✅ RESOLVED inline citing the 2026-05-06 typed-payload sweep, then move to resolved-gaps.md alongside the opponent-side entry.

### Selection: multi-select with aggregate-sum constraint (and count-capped sibling)
- **Current claim:** 🟡 PARTIAL (line 124)
- **Code state:** Phase 4 + Group 2 closed the DP-budget opponent-permanent slice and the count-capped sibling. `select_count_capped_multi` (selections.rs:1087), `select_opponent_permanents_by_dp_budget` (selections.rs:487), `SelectionKind::CountCappedMultiSelect` and `DpBudget` in `selection.rs:137,144`.
- **Verdict:** NARROW — residual is the "self-stack material multi-select" + "cost-time placement" sub-shapes called out in the prose, plus the new EX8-074-style derived-threshold need (filed separately as "Per-N-suspended scaling threshold").
- **Recommended action:** Reword severity from 🟡 to ✅ for the headline aggregate-sum / count-capped primitive, with a "remaining residual" pointer to the EX8-074-style derived-threshold entry. Most of the prose under this heading describes work that already shipped.

### Selection: ordered permutation (place N cards in any order)
- **Current claim:** 🔴 BLOCKING (line 125)
- **Code state:** `select_ordered_permutation` at `effect_context/selections.rs:1006`; `SelectionKind::OrderedPermutation` at `selection.rs:132`; `GamePhase::SelectPermutation` exists. Status block already says "Closed by Phase 4" (2026-04-20).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 4. Headline 🔴 in at-a-glance is stale.

### Selection: opponent-as-selecting-player, cross-side target, union-zone (hand OR trash), DNA-pair
- **Current claim:** 🔴 BLOCKING (line 126)
- **Code state:** Phase 4 closed 2/4 sub-pieces (opponent-as-selecting-player via `as_selecting_player`; union-zone via `select_union_zone` at selections.rs:886). The remaining "cross-side target" (`select_any_permanent`) and DNA-pair are now DSL-side: `install_select_any_permanent` and `install_select_dna_pair` exist in `dsl_cards/step/selections.rs:957,1045`. So all four sub-pieces have at least DSL-level surfaces.
- **Verdict:** NARROW — the gap title implies four open items but at least three (opponent-as-selecting-player, union-zone, DNA-pair) have shipped surfaces, and `select_any_permanent` exists as a DSL step. The Phase 4 status block still calls these "open", which is stale.
- **Recommended action:** Audit DSL→engine plumbing for `select_any_permanent` and `select_dna_pair` to confirm both have working `EffectContext` surfaces (not just DSL step installers). If yes, drop 🔴 → ✅ RESOLVED. If only the DSL step exists without an `EffectContext::select_any_permanent` curated helper, retitle to "Curated `EffectContext::select_any_permanent` helper" with 🟡 ergonomic severity.

### Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)
- **Current claim:** 🔴 BLOCKING (line 127)
- **Code state:** Phase 2 (2026-04-19) landed `EffectContext::play_from_hand_with_cost` (mod.rs:2453) + `play_from_trash_with_cost` (mod.rs:2717) covering free and cost-delta variants via `CostDelta::Reduce(printed_cost)` / `CostDelta::Reduce(delta)`. Track A (2026-05-08) added `play_from_hand_free_with_provenance` (mod.rs:2491).
- **Verdict:** NARROW — core primitives shipped; the new sub-items the prose accumulates ("play-from-revealed-free", "Tamer-only play-from-hand-by-trait-filter", "play-from-security-at-index for own security search") are distinct shapes that deserve their own gap entries.
- **Recommended action:** Drop severity 🔴 → ✅ RESOLVED for the headline; spin off two narrow follow-ups for `play_from_revealed_free` (EX8-050 Gogmamon) and `play_from_security_at(index)` (BT13-012 GeoGreymon, BT14-033 Patamon).

### Zone-manipulation: effect-initiated digivolve (free / reduced / with trait filter / ignore requirements / DNA / Blast / detect-DNA-origin)
- **Current claim:** 🔴 BLOCKING (line 128)
- **Code state:** Phase 2 landed `effect_initiated_digivolve` (mod.rs:3384), `_ignore_requirements` (mod.rs:3402), `_with_provenance` (mod.rs:3418), `_from_source` (mod.rs:3437), `_from_source_ignore_requirements` (mod.rs:3455), `effect_initiated_dna_digivolve` (mod.rs:3509), `_dna_digivolve_with_provenance` (mod.rs:3572). DNA-origin context bit in Track A. Blast DNA via `execute_blast_dna_digivolve` in combat.rs:1630. BeforePayCost cost reduction in modifier scan is wired (Track C deferred-payload wave, 2026-05-09).
- **Verdict:** NARROW — the headline primitive shipped; the listed remaining sub-item ("BT17-095 1-field + 1-hand + Omnimon-hand DNA pair") is a card-local selection-shape gap, not a missing curated helper.
- **Recommended action:** Drop 🔴 → ✅ RESOLVED; spin off BT17-095-style "DNA digivolve with field+hand material pair" as its own narrow card-shape gap if it remains blocking after the existing helpers are tried.

### Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self / trash-from-hand
- **Current claim:** 🔴 BLOCKING (line 129)
- **Code state:** Phase 2 landed `return_to_hand` (mod.rs:3329), `return_to_deck` (mod.rs:3355), `add_to_hand_from_trash` (mod.rs:2187), `trash_from_hand_by_index` (mod.rs:2266). Track E (2026-05-08) added `bounce_self` (mod.rs:3349), `return_all_trash_to_deck_bottom` (mod.rs:4159), `trash_opponent_hand_to_count` (mod.rs:4021). BT17-078 / BT22-015 selected-level / source-count slices closed 2026-05-07.
- **Verdict:** NARROW — core shapes landed; only sub-shape gaps that remain are `return_trash_to_deck(end=Top)` (LM-031/LM-032), self-return-as-cost builder hook, and the closure-valued cost-delta interactions with cross-permanent inherited Tamers. Headline severity should drop.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL; rename to "return-to-deck-top / self-return-as-cost" so the actual open shapes are visible in the at-a-glance.

### Zone-manipulation: reveal-top-N deck + add-to-hand + hatch
- **Current claim:** 🔴 BLOCKING (line 130)
- **Code state:** Phase 2 landed `reveal_top_deck` (mod.rs:2240), `add_to_hand_from_deck` (mod.rs:2178), `add_to_hand_from_trash` (mod.rs:2187), `hatch` (mod.rs:4531).
- **Verdict:** NARROW — primitives shipped; remaining "multi-pick from reveal + ordered-deck-bottom return + play-from-revealed-free" are mostly sub-shape gaps. Multi-pick is closed by the Phase 4 `select_count_capped_multi` + ordered permutation.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with the residual narrowed to "`play_from_revealed_free`" only.

### Zone-manipulation: security stack operations (trash top, place bottom, trash N, Recovery +N)
- **Current claim:** 🔴 BLOCKING (line 131)
- **Code state:** Phase 2 + 2026-05-03 + Track E landed `place_on_security` (mod.rs:4178), `trash_top_security` (mod.rs:1863), `add_top_security_to_hand` (mod.rs:2225), `recover_from_deck` (mod.rs:4197), `place_self_at_security` (mod.rs:1422), `place_self_option_at_security` (mod.rs:1490), `security_place_stacked_card` (mod.rs:3931), `security_place_top_stacked_card` (mod.rs:3975), `place_permanent_on_security` (mod.rs:4178). `trash_top_security` takes single-card param — multi-N variant absent.
- **Verdict:** NARROW — almost every listed primitive shipped. Real remaining open item: a multi-N `trash_top_security(player, N)` form (today's helper trashes exactly 1) and "face-up security extraction/flip".
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL, renarrow title to "Top-N security trash + face-up security flip/extraction".

### Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve + stack reorder
- **Current claim:** 🔴 BLOCKING (line 133)
- **Code state:** Phase 2 landed `place_as_bottom_source` (mod.rs:2817). Track A (2026-05-08) added `place_permanent_as_bottom_sources` via the same helper. DSL `place_as_bottom_source` accepts both `source: { permanent }` and reveal/hand/trash forms.
- **Verdict:** NARROW — core primitive shipped. Remaining are `place_as_top_source`, alt-digivolve with override-cost + ignore-reqs flag, stack reorder / `move_source_to_bottom`, face-down placement.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL; rename to "Alt-digivolve with override-cost + ignore-reqs + face-down placement" to make the residual specific.

### Force-follow-up-attack / "may attack without suspending" script helpers
- **Current claim:** 🟡 PARTIAL (line 151)
- **Code state:** `may_attack_now` (mod.rs:4393), `may_attack_now_optional` (mod.rs:4403), `may_attack_now_optional_with_upgrade` (mod.rs:4421), `force_opponent_attack` (mod.rs:4489), `force_opponent_attack_with_upgrade` (mod.rs:4499). DSL `may_attack_now` and `force_attack` both ship.
- **Verdict:** NARROW — the immediate-prompt slice is closed; the entry's "remaining" persistent player-scoped `MayAttackPlayerOnly` is covered by Track C/D (combat.rs gates it in `begin_attack_open`).
- **Recommended action:** Drop 🟡 → ✅ RESOLVED. The prose already explicitly says "no separate core combat primitive remains open for Raid target switching or effect-driven redirects" — the persistent variants are owned by the player-scoped-modifier entry.

### Granted triggered ability — attach an `Effect` to another permanent
- **Current claim:** 🟡 PARTIAL (line 153)
- **Code state:** Full Track H closure (2026-05-10). `EffectContext::grant_triggered_effect` (mod.rs:3725), `Game::fire_granted_triggered_effects`, queue-based dispatch with selection support, `pending_skips` for `*NextTurn` mid-opp-turn installs, typed `AuraScope`/`AuraGrant`/`AuraBuilder` API in `aura.rs`. DSL `grant_triggered_effect` step lowers. EX1-068 Ice Wall! ships as raw_rust *and* DSL fixture.
- **Verdict:** CLOSED for substrate; only "dead-body cleanup on carrier leave" is a memory-overhead nit, not a behavioral bug.
- **Recommended action:** Move to resolved-gaps.md citing Track H (PR #467); file the dead-body cleanup as a separate `qa/dsl-vocab-gaps.md` entry or simply note it in resolved-gaps as future polish.

### Named-target declarative aura (DP / keyword grants filtered by name/trait/level)
- **Current claim:** 🟡 PARTIAL (line 154)
- **Code state:** Group 6 + Track H §9 (2026-05-10) closure. `kind: aura` with `target_filter` lowers via `lower_aura.rs`. Card-shaped fixtures exist for filter auras across DP/keyword/security-attack/named-modifier. Materialized-modifier refresh on tick is cited.
- **Verdict:** NARROW — query-time aura recomputation is more faithful in the long term but the tick-driven path is closed end-to-end.
- **Recommended action:** Drop 🟡 → ✅ RESOLVED for the tick-driven path; spin off "query-time aura recomputation" as a separate ergonomics gap if anyone actually needs it.

### Declarative aura sourced from security zone
- **Current claim:** 🔴 BLOCKING (line 155)
- **Code state:** Track H §5 (2026-05-10) wired `tick_declarative_effects` to iterate face-up security cards. `kind: aura, scope: security` body shipped. ST20-15 / BT21-095 representative slices proven.
- **Verdict:** NARROW — the consult-via-modifier-registry path is closed; the remaining "tensor / mask pre-compute from `SecuritySource`" is a tensor-scaffolding ergonomic improvement, not a behavioral gap.
- **Recommended action:** Drop 🔴 → ✅ RESOLVED with the tensor/SecuritySource follow-up listed as a separate "Aura tensor pre-compute" entry.

### Decode keyword (play from own digivolution stack without paying cost on non-battle leave)
- **Current claim:** 🔴 BLOCKING (line 157)
- **Code state:** 2026-05-07/05-08 narrowing closed BT22-015 (Red/Black Decode) + EX4-060 (BlitzGreymon + CresGarurumon ladder) + EX9-021 (End of Attack source-play). `select_material` honors card predicates, `play_from_materials.source_index` consumes bindings.
- **Verdict:** NARROW — the headline shape (BT22-015 Decode) is closed; "EX10-061 Apocalymon batch/different-name source plays" is a higher-level ergonomic ask.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with the residual narrowed to "EX10-061 batch + different-name source play DSL sugar".

### `EndOfOpponentsTurn` effect timing not dispatched
- **Current claim:** 🔴 BLOCKING (line 1130) — but entry has "Closed by Phase 1 (2026-04-19)" footer
- **Code state:** `fire_end_of_opponents_turn` wired in `rotate_turn_player` (Phase 1). Builder `Effect::end_of_opponents_turn`.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 1. The 🔴 header is contradicted by its own closure footer.

### Native printed keyword parsing (Rush, Raid, Piercing, Blocker, etc.)
- **Current claim:** ✅ RESOLVED at entry level, 🟢 in at-a-glance (line 134) — already CLOSED at row level
- **Code state:** Phase 3 closed core; Group 6 Task 4 closed combat keywords. `CardData::keywords` ingested at parse time; `Game::has_keyword` consult site.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Phase 3 + Group 6 + Track G (PR #457).

### `<Progress>` keyword + `ImmunityToOpponentEffects` modifier
- **Current claim:** ✅ RESOLVED at entry level (line 480)
- **Code state:** Group 6 closure; Track G Phase F backfilled inherited Progress test coverage (2026-05-10).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Armor Purge>` keyword (leave-field replacement variant)
- **Current claim:** ✅ RESOLVED (2026-04-25) at entry level
- **Code state:** Phase D auto-install + Track B Royal Knights consumer tests.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Barrier>` keyword
- **Current claim:** ✅ RESOLVED / TRACK B VERIFIED at entry level (line 777)
- **Code state:** Auto-install wired; printed inherited Barrier synthesizes through `Game::effects_for_card`.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Collision>` keyword + `ModifierType::GrantCollision`
- **Current claim:** ✅ RESOLVED at entry level (lines 788, 1214)
- **Code state:** Native + granted Collision both via `Game::has_keyword`; mask + decode enforcement verified by Group 6 Task 4.
- **Verdict:** CLOSED
- **Recommended action:** Move both entries (the "Collision keyword" entry + the "GrantCollision modifier" entry) to resolved-gaps.md.

### `Keyword::Decoy` color-filter parameter + replacement-framework wiring
- **Current claim:** ✅ RESOLVED (2026-04-25) for un-parameterised; Track G 2026-05-10 added color-filter
- **Code state:** `Keyword::Decoy(u8)` in `enums.rs:416` with color bitmask. Parser at `card_data.rs::decoy_color_mask_from_paren`. Auto-install at `cards/keyword_effects.rs::keyword_to_auto_effect`.
- **Verdict:** CLOSED for color filter; trait-filter remainder is documented as a separate per-card override pattern, not an engine gap.
- **Recommended action:** Move to resolved-gaps.md.

### Trash all digivolution cards of a permanent (unbounded stack-peel)
- **Current claim:** 🔴 BLOCKING (line 814) — but entry footer says "Updated 2026-05-03 ... slice is implemented and verified for BT24-040"
- **Code state:** `EffectContext::trash_all_sources` at `effect_context/mod.rs:3211`. DSL `trash_all_sources` lowers (dsl_cards/step/permanent_mutations.rs:143).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Reboot>` / `<Piercing>` / `<Retaliation>` keyword entries
- **Current claim:** ✅ RESOLVED at entry level (lines 906, 1202, 1014)
- **Code state:** All native + granted forms via `Game::has_keyword`; combat enforcement verified by Group 6 Task 4.
- **Verdict:** CLOSED
- **Recommended action:** Move all three to resolved-gaps.md as a "Group 6 core combat keywords" rollup.

### `<Scapegoat>` keyword
- **Current claim:** ✅ RESOLVED / TRACK B VERIFIED at entry level (line 1036)
- **Code state:** Auto-install + replacement body wired (Track B, 2026-05-08).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Evade>` printed semantics
- **Current claim:** ✅ RESOLVED at entry level (line 1370)
- **Code state:** Phase D auto-install fixed (Track G 2026-05-10); suspend-and-cancel semantics.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Fixed attack target — `CannotBeRedirectedAsAttackTarget` / `CannotSwitchAttackTarget`
- **Current claim:** ✅ RESOLVED at entry level (line 928)
- **Code state:** Track C taxonomy + Track D consult sites (2026-05-06/07).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Raid target-switch interrupt + effect-driven attack redirect
- **Current claim:** ✅ RESOLVED at entry level (line 540) — at-a-glance shows ✅ (line 139)
- **Code state:** Track D 2026-05-08 wired core Raid as printed mid-attack interrupt; `attack_target_change` payload.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Delay>` keyword + placement-turn gating for Option cards
- **Current claim:** ✅ RESOLVED Group 5 (line 519) — at-a-glance ✅ (line 138)
- **Code state:** Group 5 (2026-05-02) closed the Delay lifecycle.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md citing Group 5.

### Scheduled end-of-turn effect queue (for transient Options)
- **Current claim:** ✅ RESOLVED at entry level (line 651) — at-a-glance ✅ (line 148)
- **Code state:** `schedule_delayed_with_runtime` at `effect_context/mod.rs:844`; `fire_end_of_your_turn` drain.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Effect re-firing / cross-timing self-trigger
- **Current claim:** ✅ RESOLVED Task 9 (line 660) — at-a-glance ✅ (line 149)
- **Code state:** `refire_effect_from_permanent` (mod.rs:653), `refire_target_effect` (mod.rs:677).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Effect-initiated digivolve from non-hand source zones
- **Current claim:** ✅ RESOLVED Group 4 (line 671) — at-a-glance ✅ (line 150)
- **Code state:** `effect_initiated_digivolve_from_source` (mod.rs:3437).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Inherited triggered-effect dispatch (`enqueue_from_permanent` walks digivolution stack)
- **Current claim:** 🟢 CLOSED (line 1151)
- **Code state:** 2026-05-06 closure verified; BT24-001 fixture proves the path.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `CannotAttackPlayer` modifier enforcement
- **Current claim:** ✅ RESOLVED at entry level (line 1160)
- **Code state:** Mask + shared combat entry enforcement (Track D, 2026-05-08).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Cross-permanent count-capped multi-select
- **Current claim:** ✅ RESOLVED (line 1224)
- **Code state:** Group 2 (2026-04-29) `select_own_sources` + DSL.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `.pay_cost()` builder hook for triggered non-cost-reduction effects
- **Current claim:** ✅ RESOLVED Group 3 (line 1236)
- **Code state:** Group 3 cost-hook regression coverage.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Source-scoped return-immunity modifiers
- **Current claim:** ✅ RESOLVED at entry level (line 1245)
- **Code state:** `grant_zone_return_immunity_to_opponent_effects` (mod.rs:3789).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Effect-driven attack cancellation (`ctx.end_pending_attack()`)
- **Current claim:** ✅ RESOLVED Group 3 (line 1268)
- **Code state:** `cancel_attack` (mod.rs:4373), `cancel_pending_attack` (mod.rs:4377). DSL `cancel_attack` lowers (Track E 2026-05-08).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### DigiXros name alias
- **Current claim:** ✅ RESOLVED Group 8 (line 1292)
- **Code state:** `CardData::digixros_aliases` ingest.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Fragment (N)>` keyword
- **Current claim:** ✅ RESOLVED (2026-04-25) (line 1190)
- **Code state:** Phase D auto-install.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Ace Overflow / Dynamic cost reduction / Dynamic DP scaling / Token creation
- **Current claim:** ✅ RESOLVED at entry levels (Ace Overflow line 561, Dynamic cost reduction line 571, Token line 432); Dynamic DP scaling shows 🟡 (line 580) but prose says RESOLVED by Group 6 for DSL formula-backed aura.
- **Code state:** All Group 6/8 closed. `CardData::ace_overflow`, `kind: aura` with `dp_modifier_fn`, `CardKind::Token` + `play_token`.
- **Verdict:** CLOSED (Dynamic DP scaling deserves a NARROW reframe: continuous aura is closed but "non-aura temporary dynamic DP grants" remains a real micro-gap.)
- **Recommended action:** Move Ace Overflow, Dynamic cost reduction, Token creation to resolved-gaps.md. Keep Dynamic DP scaling open with severity dropped to 🟡 and narrowed title "Non-aura temporary dynamic DP grants".

### Suspend-this-Tamer deletion observer with Overclock cause branch / Trash-resident observer with effect digivolve from trash / Effect-played permanent cleanup provenance (engine-side)
- **Current claim:** ✅ CLOSED at entry levels (lines 1435, 1454, 1421)
- **Code state:** All shipped. EX11-060 / BT20-084 fixtures, ProvenanceToken system.
- **Verdict:** CLOSED
- **Recommended action:** Move all three to resolved-gaps.md.

### Player-scoped modifier registry — partial sub-pieces
- **Current claim:** 🔴 BLOCKING (line 145, at-a-glance)
- **Code state:** Most variants ship: `CannotPlayDigimonByEffect`, `CannotPlayFromTrash`, `CannotReducePlayCost`, `OpponentCannotReduceDigivolveCost`, `CannotAddSecurityByEffect`, `IgnoreColorRequirement`, `MayAttackPlayerOnly`, `CannotMove`, `CannotSwitchAttackTarget`, `CannotBeRedirectedAsAttackTarget`, `CanAttackTargetDefendingPermanent`, `CannotAddMemory`, `CannotAddSecurity`, `ChangeEndTurnMinMemory`, `ImmuneFromDPMinus`, `ImmuneFromStackTrashing`, `DisableEffect`, `TreatAsDigimon`, `ChangeCardDP/OriginDP/SAttack/LinkCost/LinkMax`, `ChangePermanentLevel/Traits/BaseCardName/BaseCardColor`, `ChangeCardLevelForAssembly`, `ChangeCardNamesForDigiXros` all enumerated in `enums.rs:526-730`. `PlayerModifierEntry` in `modifiers.rs:331`; `add_player_modifier` at `modifiers.rs:1081`. Track C/D 2026-05-06/07/08/09 wired the consult sites.
- **Verdict:** NARROW — every variant the entry calls out as missing is in the codebase. Remaining: "bilateral/symmetric delivery shapes", "live condition-gated player auras" (partly closed by Track H `while_condition`), "effect-vs-action-initiated play distinctions" (closed by Track A `effect_initiated`).
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with the residual narrowed to "bilateral player-aura delivery shape" (BT14-009 Gotsumon's `UntilLeaveField` lifecycle), which is the only sub-piece without working coverage. Most of the entry should split off as resolved.

### Option card play flow + Plug-In / Link
- **Current claim:** 🔴 BLOCKING (line 146)
- **Code state:** Group 4 (2026-05-01) + Group 5 (2026-05-02) + Group 7 (2026-05-02) closure: Delay lifecycle, Link registration, `OnOptionPlaced`, transient Standard Option EOT replay. `OnLink` (enums.rs), `OnLinkedCardTrashed`, `OnUnlink`. `place_self_option_at_security` exists.
- **Verdict:** NARROW — most core shapes shipped. Remaining: Plug-In re-link from battle area (filed separately as ST22-11 entry), "place this card in the battle area" disposition, hand "[Hand]" `<Blast Digivolve>` flow.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL; split out "Place-Option-in-battle-area disposition" and "[Hand][Main] Plug-In flow" as separately-titled entries.

---

## OPEN — claim accurate; no engine substrate landed

### `<Training>` keyword (line 507)
- **Current claim:** 🔴 BLOCKING
- **Code state:** Group 5 Task 6 (2026-05-02) bound Training Option carriers correctly, but the underlying primitive — `Keyword::Training`, `ctx.push_deck_top_under_self(face_down)`, `CardSource::face_down` field, `[Main]` activation extension to breeding — has not shipped.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴 BLOCKING.

### Standard Delay main-phase activation action (line 529)
- **Current claim:** 🟡 PARTIAL
- **Code state:** Group 5 supports persistent delayed Options but scheduled EOT auto-fire is the workaround; the player's later `[Main]` decision is not exposed through the action mask.
- **Verdict:** OPEN
- **Recommended action:** Keep 🟡.

### Digivolution-stack name overlay / Reveal-zone overlay (lines 746, 1026)
- **Current claim:** 🔴 BLOCKING
- **Code state:** No `name_overlay`/`level_overlay`/reveal-zone synthesis primitive exists. Grep confirms zero matches.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### Effect-spawned permanent with end-of-turn deletion rider (line 968)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `schedule_delete_at_end_of_turn` and `scheduled_eot_deletions` queue do not exist in engine source. ProvenanceToken substrate (Track A) provides the lookup half but the EOT cleanup half is unwired.
- **Verdict:** OPEN — narrowed by Track A but the core scheduled-EOT-deletion path is still missing.
- **Recommended action:** Keep 🔴 with a note that ProvenanceToken lookup is half the work.

### Effect-driven play of a Digimon from hand to an empty breeding-area slot (line 978)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `play_to_breeding_from_hand` exists (mod.rs:2840) — it actually shipped. Group 4 added it. But the gap text predates this.
- **Verdict:** CLOSED but tracker is stale — moving to CLOSED below.

### Cast-time stack-construction for cost reduction (line 988)
- **Current claim:** 🔴 BLOCKING (deferred — Track E)
- **Code state:** No `play_with_cast_time_assembly` helper; no separable `commit_play_to_battle_area_without_on_play`. Grep confirms.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴 BLOCKING.

### Cross-card effect re-firing — activate a foreign card's [On Play] effect (line 1003)
- **Current claim:** 🟡 PARTIAL
- **Code state:** Track K (2026-05-10) added `refire_target_effect` for permanent-target version (BT24-102 Homeros). BT15-102 Apocalymon's source-card / stack-card refire (foreign card on bottom of stack) is still unwired.
- **Verdict:** NARROW (already correctly marked 🟡, so leaving as OPEN here for completeness).
- **Recommended action:** Keep 🟡 PARTIAL with the existing residual framing — accurate.

### Conditional digivolve-target restriction (line 958)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `DigivolveTargetRestriction` / `CanOnlyDigivolveIntoColor` / `CanOnlyDigivolveIntoTrait` — grep returns zero matches.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### Effect-initiated play from face-up security stack (line 1047)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `play_face_up_security_free` — zero matches.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### Generic `.activation_cost(...)` builder hook for triggered abilities (line 1089)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `activation_cost` / `suspend_self_as_cost` — zero matches. `.pay_cost` Group 3 builder is the closest existing surface but the entry is explicit that this is distinct.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### Per-N-suspended scaling threshold for deletion / damage effects (line 1099)
- **Current claim:** 🔴 BLOCKING
- **Code state:** Formula `suspended_count` exists in `digimon-dsl/src/formula.rs:137` — so the threshold-formula half is wired. But the entry asks for a chained "select-multi-then-derive-threshold" DSL/Rust shape that is not present.
- **Verdict:** NARROW — the formula half landed (Track J 2026-05-10). Chained count-bound multi-select followed by formula-threshold downstream filter is the residual.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL.

### Player-scope mass `CannotSuspend` aura on opponent (line 1109)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `CannotSuspend` exists as permanent-scope (enums.rs:535). Player-scope mass aura with condition-gated continuous evaluation — not wired.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### `OnAllyAttack` / `OnOpponentAttack` observer timing context (line 1119)
- **Current claim:** 🔴 BLOCKING (already with a 2026-04-29 "Updated" footer)
- **Code state:** `attack_attacker()` / `attack_target()` accessors landed; `OnAllyAttack`/`OnOpponentAttack` fan-out wired in `combat::fire_on_attack`. DSL attack-target-kind predicate is the remaining hole.
- **Verdict:** CLOSED for observer dispatch + payload accessors; only "attack-target-kind DSL predicate" is missing.
- **Recommended action:** Drop 🔴 → ✅ RESOLVED for the engine substrate; spin off "attack-target-kind DSL predicate" as a dsl-vocab-gaps.md entry.

### `OnDigivolutionCardTrashed` observer timing (line 1178)
- **Current claim:** 🔴 BLOCKING (header says PARTIALLY RESOLVED in prose)
- **Code state:** Phase 1 wired fire-sites; 2026-05-07 added return-to-deck / de-digivolve / Armor Purge / Fragment / trash_card_source / trash_top_source / Mind Link routing.
- **Verdict:** CLOSED for substrate
- **Recommended action:** Move to resolved-gaps.md; "additional card-local source-trash producer fixtures" is card test coverage, not engine work.

### Conditional security-in-stack trigger (line 1256)
- **Current claim:** 🔴 BLOCKING
- **Code state:** 2026-05-08 narrowing closed `[Security][End of Opponent's Turn]` self-play (BT20-055). Start-of-turn / start-of-opponent-turn variants remain.
- **Verdict:** NARROW
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with the residual narrowed to "Start-of-turn / Start-of-opponent-turn security-stack timing variants".

### Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`) (line 1280)
- **Current claim:** 🟡 PARTIAL
- **Code state:** DSL `target_player` lowers; `Game::tick_declarative_effects` installs player modifier. `Expiry::Permanent` is currently used — full `UntilLeaveField` lifecycle still incomplete.
- **Verdict:** OPEN (correctly framed)
- **Recommended action:** Keep 🟡.

### Global `OnOptionCardTrashed` observer timing (line 1302)
- **Current claim:** 🔴 BLOCKING
- **Code state:** Track I substrate slice (2026-05-10) wired `OnOptionTrashed`, `TriggerSource::OptionTrashed`, `Game::trash_field_option`, `EffectContext::option_last_field_state`. Remaining: legacy Option trash paths to migrate, hand/trash/security-resident observer fan-out if needed.
- **Verdict:** NARROW
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with explicit "remaining legacy paths" residual.

### Plug-In re-link from battle area source zone (line 1313)
- **Current claim:** 🔴 BLOCKING — sub-case
- **Code state:** Track I (2026-05-10) added `OptionFieldState::{LinkedPlugIn, OrphanedPlugIn}`, `Game::orphan_linked_plug_in`/`orphan_plug_in`/`relink_plug_in`. Remaining: route carrier-loss cascades, surface orphan candidates through pending selections, DSL vocabulary.
- **Verdict:** NARROW
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL.

### `ctx.move_from_breeding()` EffectContext helper (line 1324)
- **Current claim:** 🟡 PARTIAL (Group 4 primitive landed)
- **Code state:** `move_from_breeding_by_effect` (mod.rs:2835), `play_to_breeding_from_hand` (mod.rs:2840), BREEDING_TARGET support in `place_as_bottom_source`. Optional-level-filtered prompt wrapper for P-130 still missing.
- **Verdict:** NARROW (correctly framed)
- **Recommended action:** Keep 🟡 with the narrowed residual.

### `ModifierType::CannotAddSecurityByEffect` (line 1335)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `CannotAddSecurityByEffect` in `enums.rs:612`; `CannotAddSecurity` (player-scoped variant) at enums.rs:654. Consult sites wired (Track C/D 2026-05-08 — `EffectContext::place_on_security` checks `CannotAddSecurityByEffect` then `CannotAddSecurity`).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### `<Digi-Burst N>` keyword (line 1345)
- **Current claim:** 🟡 PARTIAL
- **Code state:** DSL `digi_burst: { count: N, then: [...] }` shipped; `Keyword::DigiBurst(N)` parsed from card text. Auto-install intentionally absent (Track G close).
- **Verdict:** NARROW — actually CLOSED as engine substrate; the entry's "remaining reusable work is no native keyword/body auto-install" is documented as intentional design.
- **Recommended action:** Drop 🟡 → ✅ RESOLVED with the intentional-design note as the closure rationale.

### `<Decoy>` color-filter parameterisation (line 1358)
- **Current claim:** 🟡 PARTIAL — color filter resolved, trait filter remains documented gap
- **Code state:** Color filter shipped (Track G 2026-05-10); trait filter explicitly chosen as per-card override pattern.
- **Verdict:** NARROW; the trait-filter "gap" is a documented design decision, not a substrate gap.
- **Recommended action:** Drop 🟡 → ✅ RESOLVED, with trait-filter footnoted as per-card-override pattern.

### Costed self-digivolve stable source binding (line 1400)
- **Current claim:** 🔴 BLOCKING (Puppets Batch 5)
- **Code state:** No specific stable-source-binding API; cost-trigger binding for self-digivolve depends on `ctx.source_permanent` snapshot semantics under stack shifts.
- **Verdict:** UNCLEAR — needs engineering eyes to confirm whether `ctx.source_permanent` already survives cost-trigger reorderings, or whether the EX9-032 test case actually fails.
- **Recommended action:** Need confirmation from someone with EX9-032 in hand.

### Effect play with played-Digimon On Play suppression (line 1463)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `suppress_on_play` / `PlayOptions` — zero matches.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### End-of-attack mandatory self-delete chain with recovery and conditional hatch (line 1473)
- **Current claim:** 🔴 BLOCKING (EX4-074)
- **Code state:** All individual verbs (delete-self, select-opponent, recover-from-deck, hatch) exist. The "exact mandatory chain across source-deletion + Recovery continuation + conditional-hatch" needs a fidelity pass.
- **Verdict:** NARROW — likely a card-script integration issue, not a missing engine primitive.
- **Recommended action:** Spin off as a card-shape behavioral test gap, not an engine gap.

### Narrow opponent-effect protection for DP reduction and De-Digivolve (line 1444)
- **Current claim:** 🔴 BLOCKING (BT16-055)
- **Code state:** `ImmuneToOpponentDpReduction` / `ImmuneToOpponentDeDigivolve` / `EffectCategoryProtection` — zero matches.
- **Verdict:** OPEN
- **Recommended action:** Keep 🔴.

### Inherited Token/Puppet leave-prevention replacement dispatch (line 1410)
- **Current claim:** 🟡 PARTIALLY RESOLVED
- **Code state:** Track B (2026-05-08) closed dispatch for BT22-036 / EX11-022 / EX9-032 / EX7-027 / ST19-11 with focused behavioral coverage.
- **Verdict:** CLOSED for the named cards
- **Recommended action:** Move to resolved-gaps.md.

### Condition-gated modifier entries + new `Expiry` variants (line 593)
- **Current claim:** 🔴 BLOCKING (line 144 of at-a-glance)
- **Code state:** Track C 2026-05-06 published `Expiry::UntilCondition`/`OnceUsed(u32)`/`EndOfYourTurn`; 2026-05-10 UntilCondition controller landed (PR #458); Track H §4 `while_condition` aura slot + `add_modifier_with_until_condition` + `grant_keyword_with_until_condition`. EndOfOpponentsNextTurn / EndOfYourNextTurn with `pending_skips`.
- **Verdict:** NARROW — almost everything shipped. "Filter-aura + `while_condition` lazy-filter shape" is the explicit residual.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL with the residual narrowed to "filter-aura + while_condition lazy-filter rewrite".

### Trait-filter helpers on `CardSource` / `Permanent` (line 694)
- **Current claim:** 🟡 PARTIAL — ergonomics/sugar
- **Code state:** No `CardSource::has_type` or `Permanent::has_any_type` accessor. Authors use raw `card_data().type_eng.contains` today.
- **Verdict:** OPEN
- **Recommended action:** Keep 🟡 ergonomic.

### Ergonomics partials (line 767)
- **Current claim:** 🟡 PARTIAL — pervasive
- **Code state:** Aggregate filter helpers / dual-tri-timing composite / OPT activation recording / on-decline callback — none of the four sugar primitives have direct API. The on-decline shape is the most load-bearing.
- **Verdict:** OPEN
- **Recommended action:** Keep 🟡 ergonomic.

### Grant Security A. ±N modifier (line 839)
- **Current claim:** 🟡 PARTIAL
- **Code state:** Aura form closed (Track H §1). Targeted typed sugar `ctx.grant_security_attack_change` — grep returns zero matches; bare `add_modifier` still works.
- **Verdict:** OPEN ergonomic only
- **Recommended action:** Keep 🟡 ergonomic.

### Play / digivolve origin context flag (line 851)
- **Current claim:** 🟡 PARTIAL
- **Code state:** `effect_initiated` bit on `TriggerContext` landed in Track A. "per-activation identity for digivolved by THIS effect vs another effect" + "effect-spawned permanent cleanup tokens" remain (ProvenanceToken half-shipped).
- **Verdict:** NARROW
- **Recommended action:** Drop 🟡 to "✅ for generic by-effect; 🟡 for cleanup-token half" framing.

### Search-own-security-stack primitive (line 862)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `search_own_security_stack` at `effect_context/selections.rs:1241`. DSL verb landed Track E 2026-05-09.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Effect-initiated digivolve from security stack (line 872)
- **Current claim:** 🔴 BLOCKING (BT14-033)
- **Code state:** `effect_initiated_digivolve_from_source` (mod.rs:3437) accepts security source per the "Effect-initiated digivolve from non-hand source zones" closure (Group 4).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Digivolution-stack source extraction (`pop_top_source` from named permanent) (line 918)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `trash_top_source` (mod.rs:3265) trashes the top source; no `pop_top_digivolution_source` that returns the popped card for re-routing. `security_place_top_stacked_card` (mod.rs:3975) ships a specific re-routing for the security-top destination (covers BT20-084).
- **Verdict:** NARROW — for the BT20-084 destination, closed; for the general-purpose extraction primitive (BT24-093 Temple of Beginnings), still missing.
- **Recommended action:** Drop 🔴 → 🟡 PARTIAL; rename to "Generic `pop_top_digivolution_source` for arbitrary re-routing (BT24-093)".

### In-effect branch-choice selector `select_effect_choice` (line 948)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `select_effect_choice` at `effect_context/selections.rs:602`. `SelectionKind::EffectChoice` in `selection.rs:112`.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Counter window + `<Blast Digivolve>` activation flow (line 1068)
- **Current claim:** 🟡 PARTIALLY CLOSED
- **Code state:** Track D 2026-05-08 closure. `open_counter_window` (mod.rs:4381). Card-specific bodies (BT20-076, BT20-081, EX6-029) are the residual.
- **Verdict:** CLOSED for engine substrate
- **Recommended action:** Move to resolved-gaps.md; spin off "Generic `ctx.prompt_blast_digivolve`/`prompt_blast_dna_digivolve` raw_rust helpers" as a small ergonomic gap.

### OnDeletion cause discriminator (line 1057)
- **Current claim:** ✅ RESOLVED (2026-04-24)
- **Code state:** `deletion_cause()` at mod.rs:494/1256, `was_deleted_by_effect()` at mod.rs:502/1263, `was_deleted_by_opponent()` at mod.rs:513/1272.
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

### Permanent-scoped modifier to suppress effect activation by timing (line 825)
- **Current claim:** 🔴 BLOCKING
- **Code state:** `ModifierType::DisableEffect` in `enums.rs:684`; `disable_effect_timing` on `ModifierEntry`; `permanent_activation_blocked_for_timing` consult site (Track C 2026-05-06).
- **Verdict:** CLOSED
- **Recommended action:** Move to resolved-gaps.md.

---

## UNCLEAR — need engineering eyes

### Costed self-digivolve stable source binding (Puppets Batch 5)
- **Current claim:** 🔴 BLOCKING for EX9-032
- **Code state:** Unable to determine from grep whether `ctx.source_permanent` snapshot semantics survive the chain "trigger dispatch → pay deletion cost on lower-index permanent → self-digivolve at source binding". The substrate may already be correct; the EX9-032 test may need writing to confirm.
- **Verdict:** UNCLEAR
- **Recommended action:** Try writing the EX9-032 first-test (per the entry's "First test" note). If it passes with the existing source-binding semantics, close. Otherwise file a focused engine slice.

### End-of-attack mandatory self-delete chain (EX4-074)
- **Current claim:** 🔴 BLOCKING
- **Code state:** All listed verbs exist. The card text's "delete self + delete opponent + Recovery + conditional hatch" mandatory chain probably works with existing primitives but hasn't been proven via the first-test in the entry.
- **Verdict:** UNCLEAR — likely a card-script integration test rather than an engine gap.
- **Recommended action:** Write the EX4-074 behavioral test first; if it passes, close. Otherwise file the specific missing fidelity piece.

---

## Recommended next steps

1. **Batch CLOSED moves to `qa/resolved-gaps.md`.** ~28 entries are ready to relocate. The biggest wins are the Track A observer cluster (Phase-granular turn timings, Observer timings tied to specific events, OnPlaceSecurity, OnAnyDigimonPlayed/OnAnyDeletion) and the Group 6 keyword cluster (Barrier, Scapegoat, Retaliation, Piercing, Reboot, Progress, Collision, Decoy, Evade).
2. **Sweep the at-a-glance table** so headline severity matches the per-entry resolution prose. Today the headline lags by 3–6 weeks behind the inline updates.
3. **Split umbrella entries.** The four big zone-manipulation umbrellas (play-from-X-free, return-to-X, security-stack-ops, effect-initiated-digivolve) all have most of their substrate landed but accumulate new sub-shape items into the same heading. Each should split into a "closed core" rollup and 1–3 narrow open sub-entries.
4. **Reframe Player-scoped modifier registry.** Almost every variant the entry calls out as missing is actually in `enums.rs`. The residual is one specific lifecycle bug (`UntilLeaveField` bilateral delivery for BT14-009), which deserves its own narrow entry.
5. **Confirm the two UNCLEAR entries** with a first-test write per the entry's own "First test" note.
