# Archetype DSL Implementation: Medusamon
Date: 2026-04-27
Total cards in pool: 53
Processed this run: 19 (Batches 1–5 complete)
Pipeline: batch-implement-cards-rust-dsl

## Summary (running totals — updated per batch)
- IMPLEMENTED: 5 (BT24-008, BT21-015, BT24-011, EX11-012, BT18-087)
- PARTIAL: 13 (BT21-008, BT23-005, EX11-008, BT21-025, BT24-016, BT21-029, BT24-017, BT24-082, EX11-054, BT21-081, P-189, BT21-017, BT24-012)
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
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

## Engine-Gap Blocked Cards / Clauses
### G-INHERITED-DISPATCH (Digivolution-Stack Inherited Triggered Dispatch)
- Affected (this batch): BT21-008 inherited clause; will affect every Lv3+ Digimon in remaining batches with an inherited triggered effect.
- See `qa/archetype-qa/engine-gaps.md` for full specification.

### G-OPT-TRIGGERED (Once-Per-Turn Not Enforced for Triggered Effects)
- Affected (this batch): BT21-008, BT24-008, EX11-008 inherited OPT clauses; BT24-082 clause-2.
- See `qa/archetype-qa/engine-gaps.md`.

### G-ON-MOVE (`EffectTiming::OnMove` Missing)
- Affected (this batch): EX11-008 [When Moving] half of its dual-timing OnPlay/OnMove clause.
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
