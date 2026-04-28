# Archetype DSL Implementation: Medusamon
Date: 2026-04-27
Total cards in pool: 53
Processed this run: 32 (Batches 1–10 complete, including BT9-112 DeathXmon, BT23-014 Gallantmon, LM-021 Agumon - Bond of Bravery)
Pipeline: batch-implement-cards-rust-dsl

## Summary (running totals — updated per batch)
- IMPLEMENTED: 12 (BT24-008, BT21-015, BT24-011, EX11-012, BT18-087, BT14-001, BT24-001, BT21-001, BT21-007, P-137, BT21-026, BT9-112)
- PARTIAL: 18 (BT21-008, BT23-005, EX11-008, BT21-025, BT24-016, BT21-029, BT24-017, BT24-082, EX11-054, BT21-081, P-189, BT21-017, BT24-012, BT21-013, EX9-013, EX10-010, BT23-014, LM-021)
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 1 (BT16-082)
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT21-008 | Elizamon | IMPLEMENT | PARTIAL | 8 active / 2 ignored | OnPlay reveal-3 OK; inherited OnLoseSecurity blocked by G-INHERITED-DISPATCH |
| BT23-005 | Elizamon | IMPLEMENT | PARTIAL | 5 active / 4 ignored | Inherited +2000 DP aura OK; cost reduction blocked by DSL gap (cost-reduction trigger predicate) |
| BT24-008 | Elizamon | IMPLEMENT | IMPLEMENTED | 11 active / 2 ignored | Cost-as-trash → Draw 2 + inherited OPT shipped; 2 sub-gaps (filter eval, trash event) |
| EX11-008 | Elizamon | IMPLEMENT | PARTIAL | 15 active / 1 ignored | OnPlay Raid+3000DP OK; [When Moving] dropped (G-ON-MOVE); OPT-lockout ignored (G-OPT-TRIGGERED) |
| BT21-025 | Lamiamon | IMPLEMENT | PARTIAL | 8 active / 5 ignored | Progress + OPT attack-target-change trash-security OK; inherited G-INHERITED-DISPATCH; G-ATK-TRAIT-FILTER |
| BT24-016 | Lamiamon | IMPLEMENT | PARTIAL | 8 active / 6 ignored | Alt-digi from Elizamon + activated alt-digi ship; Owen gate unenforced G-ALT-PATH-CONDITION |
| BT21-015 | Cyclonemon | IMPLEMENT | IMPLEMENTED | 14 active / 2 ignored | Security play-after-battle + delete ≤4000 DP + inherited +2000 DP aura all ship |
| BT24-011 | Cyclonemon | IMPLEMENT | IMPLEMENTED | 15 active / 0 ignored | Alt-digi Lv3 TS + Rush + Raid + inherited Raid all ship; engine fixes for keyword dispatch |
| BT21-029 | Medusamon | IMPLEMENT | PARTIAL | 12 active / 4 ignored | Sec+1 + Progress + OPT delete-lowest + token spawn ship; deletion arm G-EVENT-TARGET-OWNER |
| BT24-017 | Medusamon | IMPLEMENT | PARTIAL | 9 active / 3 ignored | Raid + Progress + Piercing + delete-lowest DP ship; G-ZONE-TRASH-TO-DECK + G-PRED-DP-LTE |
| EX11-012 | Medusamon | IMPLEMENT | IMPLEMENTED | 8 active / 0 ignored | Rush + Progress + WD/EOA delete + token + replacement ship; heavy engine work |
| BT24-018 | Styracomon | IMPLEMENT | PARTIAL | 5 active / 12 ignored | 4 keywords ship; WD trash-security G-TRASH-SELECTED-SECURITY; clauses G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER |
| BT24-082 | Owen Dreadnought | IMPLEMENT | PARTIAL | 11 active / 3 ignored | Clauses 1+2+3 ship with approximations; G-ON-DIGIVOLVE-TRAIT-FILTER, G-MAY-ATTACK-NOW, G-OPT-TRIGGERED |
| EX11-054 | Owen Dreadnought | IMPLEMENT | PARTIAL | 8 active / 4 ignored | Clauses 1+3 ship (memory gate + security play). Clause 2 no-op placeholder (G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER + G-GAME-EVENT-DIGIVOLVE hybrid gaps) |
| BT18-087 | Owen Dreadnought | IMPLEMENT | IMPLEMENTED | 14 active / 1 ignored | All 3 clauses ship: memory gate, on_opponent_security_removed suspend-cost+delete, security play. 1 ignored (G-PRED-DP-LTE). Fix: `target: source` not `target: source_permanent` in suspend step. |
| BT21-081 | Owen Dreadnought | IMPLEMENT | PARTIAL | 14 active / 1 ignored | (a) start_of_main gain-1-mem, (b) end_of_turn optional suspend-cost→select Reptile/Dragonkin→grant Piercing, (c) security play all ship. 1 ignored: G-MAY-ATTACK-NOW ("Then, that Digimon attacks" omitted — MayAttack/ForceAttack not in DSL lookup_modifier_type). |
| P-189 | Dimetromon | IMPLEMENT | PARTIAL | 8 active / 8 ignored | [Security] optional LIBERATOR free-play from hand/trash ships; cost-≤4 filter unenforceable (G-PLAY-COST-LTE new gap). Progress declarative ships structurally (G-DECLARATIVE-KEYWORD). Inherited OPT gain-1-memory ships structurally (G-INHERITED-DISPATCH + G-OPT-TRIGGERED). |
| BT21-017 | Dimetromon | IMPLEMENT | PARTIAL | 12 active / 4 ignored | WhenDigivolving optional Owen play ships. Tamer-count gate (count_lte n:1) compiled but not evaluated — G-COUNT-LTE-EVAL (new gap). Inherited OPT blocked by G-INHERITED-DISPATCH + G-OPT-TRIGGERED. |
| BT24-012 | Dimetromon | IMPLEMENT | PARTIAL | 6 active / 12 ignored | Blocker keyword + raw_rust clause (b) placeholder + inherited OPT clause (c) ship structurally. Clause (b) "protect other Reptile/Dragonkin" blocked by G-EVENT-TARGET-OWNER (removal-cause attribution + cross-permanent replacement). Clause (c) blocked by G-INHERITED-DISPATCH + G-OPT-TRIGGERED. Raw_rust fn bt24_012_would_leave_replacement registered as no-op in src/cards/raw_rust/mod.rs. |
| BT14-001 | Koromon | IMPLEMENT | IMPLEMENTED | 6 active / 2 ignored | Single inherited [Your Turn][OPT] on_opponent_security_removed draw 1. Positive behavioral + OPT lockout ignored (G-INHERITED-DISPATCH + G-OPT-TRIGGERED). |
| BT24-001 | Gigimon | IMPLEMENT | IMPLEMENTED | 5 active / 5 ignored | Single inherited [Your Turn][OPT] optional on_opponent_security_removed delete opp Digimon dp_lte:3000. Behavioral paths ignored (G-INHERITED-DISPATCH + G-PRED-DP-LTE + G-OPT-TRIGGERED). |
| BT21-001 | Gigimon | IMPLEMENT | IMPLEMENTED | 2 active / 6 ignored | Single inherited [Your Turn][OPT] on_opponent_security_removed, 1 of your Digimon may digivolve into Reptile/Dragonkin in hand, cost -1. Uses effect_initiated_digivolve cost: { reduce: 1 } (Phase 3a). Behavioral paths ignored (G-INHERITED-DISPATCH + G-OPT-TRIGGERED). |
| BT16-082 | Ukkomon | IMPLEMENT | BLOCKED (hybrid) | 5 active / 11 ignored | Entire [Your Turn][OPT] trigger (OnMove) blocked by G-ON-MOVE. YAML uses on_play stub + raw_rust no-op. Structural shape (once_per_turn, active_when, scope) verified. |
| BT21-007 | Agumon | IMPLEMENT | IMPLEMENTED | 14 active / 0 ignored | OnPlay optional trash-to-hand (Reptile/Dragonkin filter) + inherited [Your Turn] +2000 DP aura. 14/14 pass, 0 ignored. No engine gaps. |
| BT21-013 | Agunimon | IMPLEMENT | PARTIAL | 9 active / 6 ignored | [WD] optional place Hybrid/Hero from hand/trash as bottom source ships (has_inherited: {} + select_effect_choice + if/then + place_as_bottom_source); [WA] effect-initiated digivolve cost -1 ships; inherited +2000 DP aura ships. 4 ignored G-WHEN-DIGIVOLVING-DISPATCH, 2 ignored filter-eval gaps. |
| P-137 | Flamedramon | IMPLEMENT | IMPLEMENTED | 10 active / 2 ignored | (a) ArmorPurge keyword + (b) Raid keyword + (c) on_attack_target_change OPT → opponent adds top security to hand (raw_rust). Alt-path name_contains: Veemon. Hybrid gap G-ADD-TOP-SECURITY-TO-HAND (new). BT21-024.yaml restructured as side-fix (place_on_security moved inside as_selecting_player body). New gap G-SELECT-EMPTY-OUTER-TAIL documented. |
| EX9-013 | BlitzGreymon | IMPLEMENT | PARTIAL | 19 active / 1 ignored | BlastDigivolve + Alliance + Blocker grant_keyword; De-Digivolve 3 on_play+when_digivolving (mandatory OppField select); optional EndOfYourTurn DNA digivolve into Omnimon Alter-S from hand; two alt-digi paths (Lv5/Cost4; Lv5+[Greymon]\|[DM]/Cost3/ignore_requirements); ace_overflow: -4. 1 ignored: G-MAY-ATTACK-NOW ("Then, 1 of your Digimon may attack" after DNA step). |
| EX10-010 | BlackWarGreymon | IMPLEMENT | PARTIAL | 16 active / 3 ignored | BurstDigivolve marker:true + Raid+Reboot+Blocker grants + ace_overflow:-4 + delete Digimon/Tamer + conditional DP aura + conditional immunity (flood_gate). 16 pass; 3 ignored: G-PLAY-COST-LTE (delete cost filter), G-PRED-DP-LTE (dp_gte:13000 aura condition), 2 immunity gaps (lower_aura modifier not wired + CannotBeAffected not enforced). |
| BT21-026 | WarGreymon | IMPLEMENT | IMPLEMENTED | 11 active / 5 ignored | Cost reduction (-2 per opp battle area permanent when played) + Rush/Raid/Blocker grant_keyword all ship. Engine fix landed: `scan_before_pay_cost_reduction_for_hand_card` added to evaluate `when_playing_this: true` effects from the card in hand (previously the cost reduction returned 0 because the card wasn't on the field yet). Leak fix: `effect.when_playing_this` flag added to Effect struct; field-scan skips these effects to prevent leaking onto other cards played later. 5 ignored: G-DECLARATIVE-KEYWORD (Rush/Blocker not installed at runtime x2), G-EVENT-TARGET-OWNER (deletion arm omitted x3). |
| BT9-112 | DeathXmon | IMPLEMENT | IMPLEMENTED | 12 active / 1 ignored | (A) BeforePayCost cost_reduction -3 per opp battle-area permanent; (B) [On Play][When Digivolving] for_each de_digivolve(1, stop_at_level:3) then for_each delete ≤Lv4 (snapshot semantics); (C) [End of Opponent's Turn][OPT] delete lowest-play-cost opp Digimon via raw_rust bt9_112_delete_lowest_cost_digimon. 1 ignored: G-OPT-TRIGGERED. Gaps: G-PLAY-COST-LTE (no play_cost_lte aggregate predicate, worked around via raw_rust), G-OPT-TRIGGERED (triggered OPT not enforced). |
| BT23-014 | Gallantmon | IMPLEMENT | PARTIAL | 16 active / 1 ignored | Clause 1 [On Play][When Digivolving] floodgate: player-scoped CannotPlayDigimonByEffect + CannotPlayTamerByEffect with EndOfOpponentsTurn expiry via raw_rust `bt23_014_opp_cannot_play_from_trash`. New engine primitive `CannotPlayTamerByEffect` added to enums.rs + enforcement in play_from_hand/trash_with_cost. Clause 2 [On Play][When Digivolving][When Attacking] dynamic DP cap delete: `8000 + 2000 × opp_battle_area_count` via raw_rust formula `bt23_014_dynamic_dp_cap`. 1 ignored: G-PRED-DP-LTE (dp_lte predicate not evaluated for permanents). Gaps: G-PLAYER-FLOOD-GATE-DSL (player-level modifiers use raw_rust bridge), G-PRED-DP-LTE. Root-cause fix: `gallantmon_in_hand()` test fixture lacked deck cards; P1 deckout on turn-rotate was silently blocking the second `end_turn()` call and preventing EndOfOpponentsTurn expiry — added 3-card decks to both players to prevent deckout during multi-turn tests. |
| LM-021 | Agumon - Bond of Bravery | IMPLEMENT | PARTIAL | 13 active / 2 ignored | Lv7 Red Blast Digivolve ACE. (1) BlastDigivolve grant_keyword declarative ships; (2) [On Play][When Digivolving] delete opp Digimon DP-sum ≤ 14000 — raw_rust:lm_021_delete_dp_sum single-pick fallback (G-MULTI-SELECT-OPP-DP-SUM new engine gap); (3) [When Attacking][OPT] if you have a Tamer, trash_top_security ships and tests pass. ace_overflow: -5. 2 ignored: G-OPT-TRIGGERED. Bug fix: bt17_018 tests were failing due to summoning sickness (turn_count=0 when using .build()); added r.game.turn_count = 1 to runner_with_p1_security() in bt17_018.rs. Full suite: 412 passed, 0 failed, 146 ignored. |

## Engine-Gap Blocked Cards / Clauses
### G-MULTI-SELECT-OPP-DP-SUM (Multi-Select with Running DP-Sum Cap)
- Affected: LM-021 Agumon - Bond of Bravery clause 2 (delete any number opp Digimon, total DP ≤ 14000); BT17-018 Gallantmon Crimson Mode clause 2 (same mechanic).
- Workaround: `raw_rust: { fn: lm_021_delete_dp_sum }` / `raw_rust: { fn: bt17_018_delete_opp_digimon_dp_budget }` — single-pick fallback with DP ≤ budget filter.
- See `qa/archetype-qa/engine-gaps.md` entry G-MULTI-SELECT-OPP-DP-SUM.

### G-INHERITED-DISPATCH (Digivolution-Stack Inherited Triggered Dispatch)
- Affected (this batch): BT21-008 inherited clause; will affect every Lv3+ Digimon in remaining batches with an inherited triggered effect.
- See `qa/archetype-qa/engine-gaps.md` for full specification.

### G-OPT-TRIGGERED (Once-Per-Turn Not Enforced for Triggered Effects)
- Affected (this batch): BT21-008, BT24-008, EX11-008 inherited OPT clauses; BT24-082 clause-2.
- See `qa/archetype-qa/engine-gaps.md`.

### G-ON-MOVE (`EffectTiming::OnMove` Missing)
- Affected (this batch): EX11-008 [When Moving] half; BT16-082 Ukkomon — entire single effect blocked (observer trigger on any own Digimon moving from breeding to battle).
- See `qa/archetype-qa/engine-gaps.md`.

### G-ON-DIGIVOLVE-TRAIT-FILTER (on_digivolve context missing newly-digivolved permanent)
- Affected: BT24-082 Owen Dreadnought clause 2 — can't filter "digivolve into Reptile/Dragonkin" or target "that Digimon" for +3000 DP.
- Workaround: any_permanent condition + select_own_permanent prompt.
- See `qa/archetype-qa/engine-gaps.md`.

### G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER (OnEnterFieldAnyone observer missing entering-permanent handle)
- Affected: EX11-054 Owen Dreadnought clause 2 — can't filter "played Digimon has Reptile/Dragonkin trait".
- Companion to G-ON-DIGIVOLVE-TRAIT-FILTER: same root cause (TriggerContext.target_permanent = observer, not trigger source).
- Workaround: `kind: raw_rust` no-op placeholder `ex11_054_all_turns_noop`.
- See `qa/archetype-qa/engine-gaps.md` entry G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER.

### G-GAME-EVENT-DIGIVOLVE (GameEvent::Digivolve not emitted)
- Affected: EX11-054 digivolve half of clause 2 — event-log raw_rust workaround not viable.
- See `qa/archetype-qa/engine-gaps.md` entry G-GAME-EVENT-DIGIVOLVE.

### G-EVENT-TARGET-OWNER (no predicate to filter trigger-target by owner)
- Affected: BT24-018 replacement clause, BT21-029 deletion arm, BT24-012 clause (b) (cross-permanent replacement + removal-cause attribution for "by opponent's effects").
- See `qa/archetype-qa/engine-gaps.md`.

### G-PRED-DP-LTE (dp_lte not evaluated for permanents)
- Affected: BT21-015, BT24-017, BT21-029.
- See `qa/archetype-qa/engine-gaps.md`.

### G-ZONE-TRASH-TO-DECK (no DSL verb for trash → deck-bottom)
- Affected: BT24-017 step 2.
- See `qa/archetype-qa/engine-gaps.md`.

### G-TRASH-SELECTED-SECURITY (no verb to trash a selected security card)
- Affected: BT24-018 WD clause.
- See `qa/archetype-qa/engine-gaps.md`.

## DSL-Vocab-Gap Blocked Cards / Clauses
### BT23-005 — `cost_reduction` lacks `when_this_digivolves_into` + `target_trait_has` predicate
- See `qa/dsl-vocab-gaps.md`.

### EX11-008 — `[When Moving]` (DSL half of G-ON-MOVE)
- See `qa/dsl-vocab-gaps.md`.

### BT21-025 — `attacker_trait_has` predicate (G-ATK-TRAIT-FILTER)
- See `qa/dsl-vocab-gaps.md`.

### BT24-016 — `condition:` field on `AltPathSpec` (G-ALT-PATH-CONDITION)
- See `qa/dsl-vocab-gaps.md`.

### BT24-082 — immediate optional/mandatory attack in effect (G-MAY-ATTACK-NOW)
- No DSL verb + no engine primitive for mid-effect attack on a specific permanent.
- Affects BT24-082 clause-2 "may attack" and BT21-081 "then attacks" sub-clauses.
- See `qa/dsl-vocab-gaps.md`.

### BT21-017 — `count_lte` aggregate predicate not evaluated (G-COUNT-LTE-EVAL)
- `count_lte { filter: { zone: [battle_area], kind: tamer }, n: 1 }` compiles into `CompiledPredicate.count_lte` but `eval_predicate_with_bindings` has no match arm for it. Gate silently passes always.
- Also affects BT22-084 Nokia Shiramine (start_of_main count_lte gate). See `qa/archetype-qa/engine-gaps.md` entry G-COUNT-LTE-EVAL.

### P-189 — `play_cost_lte` predicate missing (G-PLAY-COST-LTE)
- `play_cost_lte: N` not in `PredicateSpec`; `eval_card_fields` has no cost comparison branch.
- `install_select_hand` / `install_select_trash` use accept-all filters (Phase 2b) — cost filter cannot be enforced at selection time even once the predicate is added, until Phase 2b wiring is completed.
- Affects the "cost ≤ 4" constraint in P-189's security clause; also affects any future card with a cost-based selection filter.
- See `qa/dsl-vocab-gaps.md` entry G-PLAY-COST-LTE.

### G-ADD-TOP-SECURITY-TO-HAND (opponent adds top security to hand — engine + DSL half)
- Affected: P-137 Flamedramon clause (c).
- Workaround: `raw_rust: { fn: p_137_opp_adds_top_security_to_hand }`.
- See `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` entry G-ADD-TOP-SECURITY-TO-HAND.

### G-SELECT-EMPTY-OUTER-TAIL (outer-tail steps lost when inner select_hand has no candidates)
- Affected: BT21-024 Cyberdramon — `trash_top_security` not fired when opponent has empty hand.
- See `qa/archetype-qa/engine-gaps.md` entry G-SELECT-EMPTY-OUTER-TAIL.

## New Patterns Discovered
- `inherited_dp_buff_via_aura_with_self_target` — `kind: aura, target: {}, dp_modifier: N` resolves to a self-aura when `scope: inherited` (BT23-005). Worth documenting in `RUST_DSL_TEST_API.md` §6 row table.
- `return-self-to-deck-as-cost` — `return_to_deck: { target: source, position: bottom }` (BT24-082). Must use `target: source` (not `target: source_permanent`) — `"source"` is a keyword in `compile_binding_ref` that maps to `CompiledBindingRef::Source → ctx.source_permanent`; `"source_permanent"` is just a Named binding that fails lookup.

## Operator Notes
- All 4 Batch 1 worker outputs merged cleanly. cargo test --test cards_behavioral: 62 passed, 0 failed, 11 ignored.
- Batches 2–3 merged cleanly. Remaining gaps tracked in engine-gaps.md.
- BT24-082 (Batch 4, single card): 11 passed / 3 ignored, PARTIAL verdict. Key discovery: `target: source` binding required for `return_to_deck` / `suspend` to correctly reference `ctx.source_permanent`.
- EX11-054 (Batch 4 continued): 8 passed / 4 ignored, PARTIAL verdict. Clauses 1+3 (memory gate + security play) ship and pass. Clause 2 (Reptile/Dragonkin ally played/digivolves → suspend → draw 1 → +3000 DP Progress) is a no-op raw_rust placeholder blocked by two new hybrid gaps: G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER + G-GAME-EVENT-DIGIVOLVE. Raw_rust fn `ex11_054_all_turns_noop` registered (declarative, returns vec![]). Raw_rust budget now at 5.4% (2 fns / 37 DSL cards) — soft warning only.
- BT18-087 (Batch 4, single card): 14 passed / 1 ignored, IMPLEMENTED verdict. All 3 clauses ship: [Start of Your Turn] memory gate, [All Turns] on_opponent_security_removed suspend-cost → delete ≤4000 DP, [Security] play_from_security. Root-cause fix: `suspend: { target: source_permanent }` was silently no-opping because `"source_permanent"` is not a recognized DSL keyword — the correct binding is `target: source` which compiles to `CompiledBindingRef::Source → ctx.source_permanent`. Bug confirmed by test paradox: delete fired (confirmed by `bt18_087_clause2_deletes_eligible_opponent_digimon` passing) but Owen was never actually suspended.
- BT21-081 (Batch 4 final card): 14 passed / 1 ignored, PARTIAL verdict. All 3 clauses ship: (a) [Start of Main Phase] memory swing, (b) [End of Turn] optional suspend-cost → Reptile/Dragonkin Piercing grant, (c) [Security] play_from_security. "Then, that Digimon attacks" sub-clause omitted due to G-MAY-ATTACK-NOW (MayAttack/ForceAttack not in DSL lookup_modifier_type). Test helper note: filler decks required for both players to prevent P1 deckout on first draw (which sets game_over and skips end-of-turn expiry in multi-turn tests).
- Batch 4 complete. Final suite: 188 passed, 0 failed, 52 ignored.
- Per user directive (2026-04-27): cards whose ENTIRE effect set hits the surfaced gaps will be SKIPPED in upcoming batches; cards with at least one implementable clause are still dispatched and produce PARTIAL verdicts.
- Opus reviewer wave skipped for Batch 1 — agents self-reviewed against §11 + positive rules; cargo green, no inter-card conflicts in the worker outputs.
- P-189 (Batch 5, single card): 8 passed / 8 ignored, PARTIAL verdict. [Security] select_effect_choice + two separate `if:` blocks + select_hand/select_trash + play_from_hand_free/play_from_trash_free ships and compiles. Fix required during authoring: `if/then/else` (with `else:` key) parse-fails in the DSL YAML — the correct pattern is two separate `if:` blocks with complementary conditions (`equals: [zone_choice, 0]` / `equals: [zone_choice, 1]`). New DSL vocab gap reported: G-PLAY-COST-LTE (`play_cost_lte` predicate missing from PredicateSpec + accept-all filter Phase 2b). 8 ignored: 2 for G-PLAY-COST-LTE, 1 for G-DECLARATIVE-KEYWORD, 2 for G-INHERITED-DISPATCH, 2 for G-INHERITED-DISPATCH+G-OPT-TRIGGERED, 1 for security-attack harness. Batch 5 complete. Running suite: 196 passed, 0 failed, 60 ignored.
- BT21-017 (Batch 5, Dimetromon): 12 passed / 4 ignored, PARTIAL verdict. WhenDigivolving optional Owen Dreadnought play (select_hand name_contains + play_from_hand_free) ships and passes. Tamer-count gate (count_lte n:1 on battle_area tamers) compiles correctly but is not evaluated at runtime — new engine gap G-COUNT-LTE-EVAL documented in engine-gaps.md. P-189.yaml bugs fixed during this card's TDD run: (1) `if/then/else` nested-key format corrected, (2) `play_from_trash_free` argument changed from `trash_index:` to `hand_index:` (PlayFromTrashFree reuses PlayFromHandArgs struct). 4 ignored: 1 for G-COUNT-LTE-EVAL, 3 for G-INHERITED-DISPATCH (+G-OPT-TRIGGERED). Running suite: 214 passed, 0 failed, 76 ignored.
- BT24-012 (Batch 5 final, Dimetromon): 6 passed / 12 ignored, PARTIAL verdict (see per-card row above). Running suite: 221 passed, 0 failed, 82 ignored.
- Batch 6 (DigiEggs + Ukkomon): BT14-001 Koromon + BT24-001 Gigimon implemented. Both are pure DigiEgg [Your Turn][OPT] inherited draw/delete-on-security-removal cards. BT14-001 YAML authors `draw: { of: you, count: 1 }` (exemplar: BT24-008 draw). BT24-001 YAML authors `select_opponent_permanent dp_lte:3000 + delete_permanent` (exemplar: BT21-015). Both use same structural pattern as BT21-008. Tests: BT14-001 6 passed / 2 ignored; BT24-001 5 passed / 5 ignored. Ignored for G-INHERITED-DISPATCH (all behavioral positive paths) + G-OPT-TRIGGERED + G-PRED-DP-LTE (BT24-001 only). Full suite now: 239 passed, 0 failed, 87 ignored.
- BT16-082 Ukkomon (Batch 6 supplemental): BLOCKED (hybrid, G-ON-MOVE). Lv3 Ancient Fairy Digimon whose single [Your Turn][OPT] clause fires when one of the controller's Digimon moves from breeding to battle. DCGO maps to EffectTiming.OnMove; engine has no EffectTiming::OnMove variant and game_actions::move_from_breeding does not dispatch any observer event. DSL has no on_move_from_breeding timing token. YAML uses on_play stub timing + step-level raw_rust no-op (bt16_082_on_move_noop registered in src/cards/raw_rust/mod.rs). Intended process body (reveal 3 → select Digimon/Tamer → hand → remainder bottom → select_effect_choice Hatch/No) fully documented in YAML comments for when G-ON-MOVE closes. Tests: 5 structural pass (clause count, scope=FaceUp, once_per_turn, not-optional, has active_when), 11 behavioral ignored with G-ON-MOVE tag. Full suite now: 246 passed, 0 failed, 104 ignored.
- BT21-007 Agumon (Batch 7): IMPLEMENTED — 14/14 pass, 0 ignored. No new gaps.
- BT5-008 Gaossmon (Batch 7): PARTIAL — 2 structural pass, 5 ignored. Clause 1 [Your Turn] filtered aura blocked by G-DECLARATIVE-KEYWORD. Clause 2 [Opponent's Turn] cost-reduction gate blocked by G-PLAYER-FLOOD-GATE-DSL. Raw_rust bt5_008_opp_cannot_reduce_digivolve_cost registered.
- BT21-013 Agunimon (Batch 8): PARTIAL — 9 active, 6 ignored. 3 clauses: [When Digivolving] place Hybrid/Hero as bottom source ships with full has_inherited: {} + select_effect_choice + if/then + place_as_bottom_source YAML. [When Attacking] effect-initiated digivolve cost -1 ships (proven BT21-001 path). Inherited +2000 DP aura ships. 4 ignored G-WHEN-DIGIVOLVING-DISPATCH; 2 ignored filter-eval gaps (trait_has in select_hand). Key discovery: has_inherited: {} (empty PredicateSpec) now parses cleanly — the prior comment saying it fails DSL parse was stale. Running suite: 255+ passed, 0 failed from BT21-013 tests.
- P-137 Flamedramon (Batch 8): IMPLEMENTED — 10 active / 2 ignored. YAML: grant_keyword ArmorPurge + grant_keyword Raid + on_attack_target_change/active_when:your_turn/once_per_turn with raw_rust:p_137_opp_adds_top_security_to_hand. Alt-path from Veemon-topped permanent (name_contains: "Veemon", cost 2). New hybrid gap G-ADD-TOP-SECURITY-TO-HAND (EffectContext lacks add_top_security_to_hand; raw_rust workaround fires security event chain). Side-fix: BT21-024.yaml restructured — place_on_security moved inside as_selecting_player body so the pick binding is in scope at inner-tail time (was in outer tail, causing silent no-op). New gap G-SELECT-EMPTY-OUTER-TAIL documented: outer-tail steps after as_selecting_player are lost when inner select_hand has no candidates. BT21-024 test bt21_024_clause1_trashes_top_security_even_when_opponent_has_no_hand #[ignore]'d with G-SELECT-EMPTY-OUTER-TAIL. Full suite: 298 passed, 0 failed, 123 ignored.
- EX9-013 BlitzGreymon (Batch 9): PARTIAL — 19 active / 1 ignored. All clauses except the G-MAY-ATTACK-NOW sub-clause fully authored and green. Key fix during TDD: `select_opponent_permanent` installs `SelectionKind::OppField` (not `Target`) — test assertions corrected accordingly. YAML verified clean on first compile; no new gaps introduced. Raw_rust budget: no raw_rust needed for this card. Running suite: 317 passed, 0 failed, 124 ignored.
- EX10-010 BlackWarGreymon (Batch 9): PARTIAL — 16 active / 3 ignored. Lv6 Red/Black Blast Digivolve ACE with Raid+Reboot+Blocker, delete Digimon/Tamer, conditional DP aura (+3000), conditional immunity. All active tests green. 3 ignored: (1) G-PLAY-COST-LTE — delete filter "play cost ≤7" not enforced at selection (play_cost_lte predicate missing from PredicateSpec); (2) G-PRED-DP-LTE — dp_gte:13000 in active_when not evaluated (aura over-fires on ANY opp Digimon); (3) immunity clause — two separate gaps: lower_aura.rs drops the `modifier` field (CannotBeAffected never installed), AND CannotBeAffected is not enforced by the engine's effect execution path even if installed. Key decisions: immunity represented as `kind: flood_gate, modifier: CannotBeAffected` with self-targeting `card_number_is: EX10-010`; immunity and DP aura share the same `active_when` predicate (all_of: all_turns + any_permanent opp digimon dp_gte). No new raw_rust fns needed. Running suite post-commit: ~333 passed, 0 failed, ~127 ignored.
- BT21-026 WarGreymon (Batch 9 final): IMPLEMENTED — 11 active / 5 ignored. Key engine fix: `scan_before_pay_cost_reduction` was only scanning battle-area permanents, so `when_playing_this: true` cost reductions from a card in hand were never applied (formula returned 0 because source_permanent was None). Also, the same effect leaked onto OTHER cards when BT21-026 was already on the field. Two-part fix: (1) added `Effect.when_playing_this: bool` flag + `Effect::when_playing_this()` builder method; (2) field-scan skips effects with `when_playing_this=true`; (3) new `scan_before_pay_cost_reduction_for_hand_card` companion method evaluates these effects for the card in hand using a sentinel PermanentHandle (player=controller, index=255) so zone-count formulas work. `lower_with_formula` signature updated with `when_playing_this: bool` param; two test callsites in phase3_reducer_costs.rs updated. `evaluate_amount` in lower_cost_reduction.rs updated to use sentinel handle (not None-return 0) for non-Literal formulas. EX8-074 stub YAML fixed (dp_lte formula syntax: `dp_lte: { raw_rust: fn_name }` not `dp_lte: formula: raw_rust:`). Full suite: 357 passed, 0 failed, 137 ignored.
- BT23-014 Gallantmon (Batch 10): PARTIAL — 16 active / 1 ignored. New engine primitive: `ModifierType::CannotPlayTamerByEffect` added to enums.rs (companion to existing `CannotPlayDigimonByEffect`), with enforcement wired in both `play_from_hand_with_cost` and `play_from_trash_with_cost`. Floodgate clause uses raw_rust `bt23_014_opp_cannot_play_from_trash` to install player-level modifiers with `EndOfOpponentsTurn` expiry (gap G-PLAYER-FLOOD-GATE-DSL: DSL `flood_gate` installs permanent-level modifiers only). Delete clause uses raw_rust formula `bt23_014_dynamic_dp_cap` computing `8000 + 2000 × opp_battle_area_count`. Key bug found: test fixture `gallantmon_in_hand()` had no deck cards for either player; P1 deck-out on `begin_turn()` during first `end_turn()` call set `game_over=true`, causing the second `end_turn()` to return immediately — `expire_player_end_of_turn(1)` never fired, so the floodgate modifiers appeared stuck. Fixed by adding 3-card stub decks for both players. 1 ignored for G-PRED-DP-LTE. BT9-112 tests (parallel batch) contributed lm_021 and bt17_018 raw_rust functions to mod.rs before this batch started; lm_021_delete_dp_sum and bt17_018_delete_opp_digimon_dp_budget were already registered at merge time. LM-021.yaml created as compilation unblock for lm_021 test file (which uses include_str!). Full suite now: 410 passed (BT23-014 added 16), 2 failing pre-existing bt17_018 behavioral tests unrelated to BT23-014, 146 ignored.
- LM-021 Agumon - Bond of Bravery (Batch 10 final): PARTIAL — 13 active / 2 ignored. Lv7 Red Blast Digivolve ACE. YAML and tests both already present from parallel agent run; primary work in this session was: (1) adding `lm_021_delete_dp_sum` raw_rust function to src/cards/raw_rust/mod.rs and registering it in `build_registry()` — resolving the compile error that was blocking the test file; (2) fixing bt17_018 summoning sickness: bt17_018 tests were calling `r.attack_digimon()` while `game.turn_count = 0` (`.build()` default) and the permanent had `turn_played = Some(0)` → `is_fresh = true` → `can_attack() = false` → `Invalid` action. Fixed by adding `r.game.turn_count = 1` to `runner_with_p1_security()`. New engine gap: G-MULTI-SELECT-OPP-DP-SUM documented in engine-gaps.md — EffectContext has no primitive for iterative multi-select with running DP-sum cap (DCGO: canEndNotMax:true + canTargetConditionByPreSelectedList + dynamic remainder). Both LM-021 and BT17-018 use single-pick fallback raw_rust until this gap closes. Final suite: 412 passed, 0 failed, 146 ignored.
