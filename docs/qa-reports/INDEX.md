# QA Issue Resolution Index

**Last updated**: 2026-03-03

## Summary

| Report | Issues | Fixed | Won't Fix | Outstanding |
|--------|--------|-------|-----------|-------------|
| [medusa](2026-02-28-medusa.md) | 14 | 12 | 2 | 0 |
| [cs-hudiemon](2026-02-28-cs-hudiemon.md) | 12 | 9 | 3 | 0 |
| [retest-medusa-hudie](2026-02-28-retest-medusa-hudie.md) | 7 | 7 | 0 | 0 |
| [medusa-vs-hudie](2026-02-28-medusa-vs-hudie.md) | 11 | 9 | 2 | 0 |
| [medusa-hudie-coverage](2026-03-01-medusa-hudie-coverage.md) | 5 | 5 | 0 | 0 |
| [partial-fixes](2026-03-01-partial-fixes.md) | 28 | 28 | 0 | 0 |
| [medusa-v2](2026-03-01-medusa-v2.md) | 5 | 5 | 0 | 0 |
| [cs-hudiemon-partial-retest](2026-03-01-cs-hudiemon-partial-retest.md) | 5 | 2 | 0 | 3 |
| [medusa-partial-retest](2026-03-01-medusa-partial-retest.md) | 5 | 3 | 0 | 2 |
| [ts-neptune](2026-03-01-ts-neptune.md) | 8 | 5 | 0 | 3 |
| [rocks](2026-03-01-rocks.md) | 12 | 12 | 0 | 0 |
| [royal-knights](2026-03-01-royal-knights.md) | 12 | 12 | 0 | 0 |
| [diaboromon](2026-03-01-diaboromon.md) | 12 | 10 | 0 | 2 |
| [cs-mastemon](2026-03-01-cs-mastemon.md) | 5 | 5 | 0 | 0 |
| [millennium](2026-03-01-millennium.md) | 10 | 9 | 0 | 1 |
| [cross-archetype-matchups](2026-03-01-cross-archetype-matchups.md) | 1 | 0 | 0 | 1 |
| [royal-knights-retest](2026-03-02-royal-knights-retest.md) | 3 | 3 | 0 | 0 |
| [ts-neptune-retest](2026-03-02-ts-neptune-retest.md) | 1 | 1 | 0 | 0 |
| [rocks-retest](2026-03-02-rocks-retest.md) | 3 | 2 | 0 | 1 |
| [diaboromon-retest](2026-03-02-diaboromon-retest.md) | 0 | 0 | 0 | 0 |
| [cs-mastemon-retest](2026-03-02-cs-mastemon-retest.md) | 4 | 3 | 0 | 1 |
| [millennium-retest](2026-03-02-millennium-retest.md) | 0 | 0 | 0 | 0 |
| [cross-archetype-retest](2026-03-02-cross-archetype-retest.md) | 0 | 0 | 0 | 0 |
| [cross-archetype-replay](2026-03-03-cross-archetype-replay.md) | 1 | 0 | 0 | 1 |
| [millennium-retest-v2](2026-03-03-millennium-retest.md) | 4 | 2 | 0 | 2 |
| [diaboromon-retest-v2](2026-03-03-diaboromon-retest.md) | 3 | 3 | 0 | 0 |
| [royal-knights-gameplay](2026-03-03-royal-knights-gameplay.md) | 3 | 1 | 0 | 2 |
| [ts-neptune-gameplay](2026-03-03-ts-neptune-gameplay.md) | 12 | 3 | 0 | 9 |
| [royal-knights-script-audit](2026-03-03-royal-knights-script-audit.md) | 3 | 0 | 0 | 3 |
| **Total** | **189** | **151** | **7** | **31** |

---

## Report 1: Medusa (2026-02-28)

14 issues found across 20 cards. All critical/high resolved.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | Empty evo_costs for BT16-BT25 sets | crit | FIXED | `tools/ingest_cards.py` — infer missing fields |
| 2 | SYSTEMIC: effects fire at every timing | crit | FIXED | `_effect_matches_timing()` + 1837-script timing migration |
| 3 | BT24-018 alt digivolve onto tamer | high | FIXED | `digivolve_validator.py` — condition check |
| 4 | BT21-008 spurious trash pop | high | FIXED | Script rewrite |
| 5 | P-035 trash pop + memory anomaly | high | FIXED | Script rewrite + timing fix |
| 6 | BT24-012 protection unimplemented | high | FIXED | Script rewrite — added callback |
| 7 | BT24-016 When Attacking wrong player | high | FIXED | Script rewrite — corrected target |
| 8 | BT21-081 End-of-Turn targets opponent | med | FIXED | Script rewrite — own targeting |
| 9 | BT24-016 condition checks wrong object | med | FIXED | Script rewrite |
| 10 | BT24-017 DP flat not scaled | med | FIXED | Script rewrite — per-opponent scaling |
| 11 | BT24-008 filter ignores trait | med | FIXED | Script rewrite — trait check |
| 12 | SelectTarget recovery spurious | low | FIXED | Resolved by timing migration |
| 13 | Action descriptions wrong in SelectReveal | low | WONTFIX | Cosmetic — low priority |
| 14 | effects_on_field shows bottom card name | low | WONTFIX | Cosmetic — low priority |

## Report 2: CS Hudiemon (2026-02-28)

12 issues found across 15 cards. All critical/high resolved.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | SYSTEMIC: effects fire at every timing | crit | FIXED | Same as Medusa #2 |
| 2 | BT23-048 filter Tamers only, not Options | high | FIXED | Script rewrite — added Option check |
| 3 | Spurious trash pop (BT23-048, P-035, etc.) | high | FIXED | Script rewrites |
| 4 | BT16-082 Ukkomon reveal logic missing | high | FIXED | Script rewrite |
| 5 | BT23-090 protection targets opponent | high | FIXED | Script rewrite — `effect_select_own_permanent` |
| 6 | BT16-025 suspends 1 instead of all | high | FIXED | Script rewrite — loop pattern |
| 7 | Action descriptions wrong in SelectReveal | med | WONTFIX | Cosmetic |
| 8 | Spurious SelectTarget phases | med | FIXED | Resolved by timing migration |
| 9 | BT23-032 wrong timing flag | med | FIXED | Timing corrected |
| 10 | BT23-050 DP auto-select not player-chosen | low | WONTFIX | Engine auto-selects, acceptable |
| 11 | Game stuck in Draw phase | low | FIXED | Resolved by timing migration |
| 12 | BT23-017 Betamon deletes opponent not self | low | FIXED | Script rewrite |

## Report 3: Retest — Medusa + CS Hudiemon (2026-02-28)

Re-verified 83/90 effects (92%). Found 3 outstanding bugs, fixed 4 in-session.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| F1 | BT24-018 alt digivolve condition not checked | high | FIXED | `digivolve_validator.py` — `can_use_condition` call |
| F2 | BT23-090 `set_memory_3` dead code | high | FIXED | `bt23_090.py` — timing + callback |
| F3 | BT23-090 end-of-turn targets wrong player | high | FIXED | `bt23_090.py` — own Hudie targeting |
| F4 | BT18-087 `set_memory_3` dead code | high | FIXED | `bt18_087.py` — timing + callback |
| B1 | SYSTEMIC: `set_memory_3` factory (45 tamers) | med | FIXED | `generators.py` + patch 45 frozen scripts |
| B2 | BT23-032 WhenDigivolving no callback | med | FIXED | `bt23_032.py` — added process2 callback |
| B3 | DNA costs not populated | low | FIXED | `ingest_cards.py` + backfill 39 cards via API |

## Report 4: Medusa vs CS Hudiemon Matchup (2026-02-28)

Cross-archetype matchup test. 11 issues found across 2 games (~9 turns each).

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | DNA digivolve allowed with only 1 Digimon | high | FIXED | `effect_dna_digivolve_from_hand()` added to game.py; bt23_050, bt23_027 scripts use it |
| 2 | Shakkoumon [When Digivolving] fires on PLAY | high | FIXED | Fixed by #1 — DNA digivolve properly triggers WhenDigivolving only on digivolve |
| 3 | Shakkoumon force attack effect never expires | high | FIXED | FORCE_ATTACK modifier with condition+granting_player; `clear_opponent_turn_expiry()` |
| 4 | Digivolution source cards leaked on deletion | high | FIXED | Fixed by #1 — DNA digivolve handles sources correctly |
| 5 | Gotsumon trait-based evo cost missing | med | FIXED | Added `_alt_digi_trait="CS"` to bt23_048.py; player.py uses min of standard+alt costs |
| 6 | Hudiemon trait-based evo cost missing | med | FIXED | Added `_alt_digi_trait="CS"` to bt23_101.py; verified evo cost 4 onto CS Lv.3 |
| 7 | Chitose Imai OnTapped triggers for non-Hudie | med | FIXED | Rewrote condition1+process1 in bt23_081.py — Hudie trait check, suspend self as cost |
| 8 | Gotsumon reveal only allows 1 selection | med | FIXED | `_decode_selection` decline path now calls `on_decline` callback to chain multi-pass reveals |
| 9 | OnLoseSecurity digivolve shows Play actions | low | WONTFIX | Cosmetic — action labels don't affect gameplay |
| 10 | Owen Dreadnought displays piercing keyword | low | WONTFIX | Cosmetic — keyword display on tamer doesn't affect gameplay |
| 11 | Lamiamon condition may not be checked | low | FIXED | Added Reptile/Dragonkin trait check to condition1+condition2 in bt24_016.py |

## Report 5: Medusa + CS Hudiemon Full Coverage (2026-03-01)

100% card coverage pass. 7 new scripts created, 32 cards newly validated (2 PASS, 30 PARTIAL). 5 bugs fixed in-session.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | BT23-091 Delay repeats main delete instead of draw 2 | med | FIXED | Replaced process3 with `player.draw_cards(2)` |
| 2 | BT23-092 Delay repeats suspend-lock instead of draw 2 | med | FIXED | Replaced process3 with `player.draw_cards(2)` |
| 3 | BT23-095 Delay repeats bounce instead of draw 2 | med | FIXED | Replaced process3 with `player.draw_cards(2)` |
| 4 | BT23-095 bounces to hand instead of deck bottom + no filter | med | FIXED | Changed to `return_permanent_to_deck_bottom` + `is_suspended` filter |
| 5 | BT23-096 Delay repeats de-digivolve instead of draw 2 | low | FIXED | Replaced process3 with `player.draw_cards(2)` |

## Report 6: PARTIAL Script Fixes (2026-03-01)

28 PARTIAL scripts fixed to PASS. 1 engine change (game.py action mask for `_is_cannot_attack_digimon`). Final: 61 PASS / 10 PARTIAL (86% pass rate).

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | BT23-002 missing CS trait check | med | FIXED | Added CS trait check to inherited WhenAttacking |
| 2 | BT24-001 OnLoseSecurity gated by is_my_turn | med | FIXED | Removed turn guard |
| 3 | BT23-095/096 duplicate is_security_effect | low | FIXED | Removed duplicate flags |
| 4 | BT22-100 no CS filter on DP modifier | med | FIXED | Added CS trait filter + security level filter |
| 5 | EX11-054 self-suspend targets opponent | high | FIXED | Changed to self-suspend |
| 6 | P-225 wrong Delay + missing security | med | FIXED | Fixed Delay, added security effect |
| 7 | BT23-041 DP buff targets self not ally | high | FIXED | Moved into on_grant callback |
| 8 | BT23-084 Erika wrong targets throughout | high | FIXED | Full rewrite — correct targeting |
| 9 | BT23-085 Ryuji missing filters + wrong targets | high | FIXED | Hudie filter, self-suspend, CS Option |
| 10 | EX11-008 DP on self, no trait filter on Raid | high | FIXED | on_grant callback, trait filter |
| 11 | BT21-072 missing Piercing + attack mechanics | high | FIXED | Added Piercing, CAN_ATTACK_UNSUSPENDED, dynamic DP |
| 12 | BT23-017 On Play filter wrong zone | high | FIXED | Trash any card, recover CS from trash |
| 13 | BT23-037 cost reduction not CS-scoped | med | FIXED | Added CS trait check on digivolve_target |
| 14 | BT23-040 Erika placement not implemented | high | FIXED | Full implementation with cost_reduction=2 |
| 15 | BT22-094 spurious trash pop + missing self-removal | high | FIXED | Removed pop, added self-removal |
| 16 | BT23-091 no lowest-DP filter | med | FIXED | Added min_dp filter |
| 17 | BT21-093 no security<=3 check, no highest-DP | med | FIXED | Added both checks |
| 18 | BT23-092 missing Tamer target | med | FIXED | Added Tamer as second selection step |
| 19 | BT23-051 missing cant_attack_digimon | med | FIXED | Script + engine action mask change |
| 20 | BT20-102 missing Piercing, wrong wipe mechanic | high | FIXED | Full rewrite with deck bottom return |
| 21 | BT8-097 flat cost reduction, single delete | high | FIXED | Dynamic cost, batch delete, play restriction |
| 22 | BT16-077 plays from hand not trash | med | FIXED | Changed zone to trash |
| 23 | BT23-059 no Option cost, no play cost filter | high | FIXED | Added Option trash cost + lowest cost filter |
| 24 | BT23-100 wrong Delay + security filter | med | FIXED | CS Tamer Delay, CS+Lv.3 security |
| 25 | BT8-084 no WhenDigivolving callback | high | FIXED | Trash-to-evo-card + dynamic DP per color |
| 26 | BT10-042 no WhenDigivolving callback | high | FIXED | Security Attack -1 on all opp Digimon |
| 27 | EX10-010 no cost filter, no Tamer targets | high | FIXED | play_cost<=7 filter + Tamer in targets |
| 28 | Multiple hash collisions across scripts | low | FIXED | Unique hash strings per timing |

## Report 7: Meduamon vs TS Olympos v2 (2026-03-01)

5 user-reported bugs investigated. All 5 fixed.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | EX11-012 empty evo_costs prevents digivolve | high | FIXED | Added evo_costs to cards.json |
| 2 | Digimon can attack suspended Tamers | high | FIXED | Added `target.is_digimon` to attack mask |
| 3 | Petrification Tokens not implemented | med | FIXED | Token system implemented — `token_registry.py`, `effect_play_token()`, lifecycle intercepts |
| 4 | OnLoseSecurity inherited adds memory to wrong player | high | FIXED | Use `card.owner` instead of `ctx.get('player')` |
| 5 | Lamiamon WhenDigivolving incorrect condition | high | FIXED | Removed spurious Reptile/Dragonkin ally check |

## Report 8: CS Hudiemon PARTIAL Re-test (2026-03-01)

Re-tested 5 PARTIAL cards from CS Hudiemon archetype. 2 upgraded to PASS, 3 remain PARTIAL (engine limitations).

| # | Card | Previous | New Status | Reason |
|---|------|----------|------------|--------|
| 1 | BT1-090 Gravity Crush | PARTIAL | PASS | Core +2 memory works; end-of-turn -2 is rules contradiction (Option trashes before EOT) |
| 2 | BT22-099 Kuremi Detective Agency | PARTIAL | PASS | Reveal, CS filter, add-to-hand, Delay all work; cosmetic action label is systemic WONTFIX |
| 3 | BT3-103 Hidden Potential Discovered! | PARTIAL | PARTIAL | Conditional cost reduction (suspend-as-cost, green-only) not modelable in engine |
| 4 | EX1-068 Ice Wall! | PARTIAL | PARTIAL | Granting opponent WhenAttacking effects with turn expiry not supported by engine |
| 5 | EX1-071 Win Rate: 60%! | PARTIAL | PARTIAL | Conditional cost reduction (trash-as-cost, color-match) not modelable in engine |

## Report 9: Medusa PARTIAL Re-test (2026-03-01)

Re-tested 5 PARTIAL Medusa cards after token system implementation. 3 upgraded to PASS, 2 remain PARTIAL. Current totals: 66 PASS / 5 PARTIAL (93% pass rate).

| # | Card | Previous | New Status | Reason |
|---|------|----------|------------|--------|
| 1 | BT21-029 Medusamon | PARTIAL | PARTIAL | Token on deletion/security loss still stubbed (process callbacks are `pass`, not `game.effect_play_token()`) |
| 2 | BT24-017 Medusamon | PARTIAL | PASS | Token play working via `game.effect_play_token()`. DP scaling, Raid, Progress all verified. |
| 3 | BT24-018 Styracomon | PARTIAL | PASS | Armor Purge verified: engine trashes top digivolution card to survive deletion. All other effects functional. |
| 4 | BT5-008 Gaossmon | PARTIAL | PARTIAL | Unchanged. DP modifier over-applied to all Digimon; opponent cost block not modelable. |
| 5 | EX11-012 Medusamon | PARTIAL | PASS | Token play working for both WhenDigivolving and EndOfAttack. Rush, Progress verified. |

## Report 10: TS Neptune (2026-03-01)

Full archetype QA for TS Neptune (30 unique cards, 10 decklists). 16 PASS, 14 PARTIAL. 8 issues found, 5 fixed, 3 outstanding (2 deferred).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Persistent pending selection causes game deadlock | high | FIXED | `_decode_selection()` guard: if current phase is a selection phase but no `pending_selection` exists, fall back to `GamePhase.Main` |
| 2 | Play cost reduction ("When this card would be played") not applied | high | FIXED | Added `cost_reduction = 5` + board-state conditions + `card_source` guard to all 4 Olympos scripts |
| 3 | Homeros +1000 DP to TS Digimon not applied | med | OUTSTANDING | Engine lacks field-wide DP aura mechanism (deferred) |
| 4 | Lanamon When Digivolving skips hand-card placement | med | OUTSTANDING | `effect_place_from_hand_as_source()` engine helper needed (deferred) |
| 5 | Asuna Shiroki On Play trash-to-draw not triggered | med | FIXED | Broken `effect_source_permanent` condition replaced with standard `permanent_of_this_card()` guard + hand filter for TS/Three Musketeers |
| 6 | Tidal Stream Link mechanic not functional | med | FIXED | Rewrote [Main] effect: bounce all lowest-level, conditionally unsuspend TS, then `effect_link_to_permanent()`. Fixed [When Attacking] to target lowest-level. |
| 7 | Divermon has no DP in card database | low | OUTSTANDING | play_cost=0, dp=None for Lv5 Digimon — needs card data update |
| 8 | Davis Motomiya On Play reveal not triggered | med | FIXED | Condition guard already simplified in working copy |

## Report 11: Rocks (2026-03-01)

Full archetype QA for Rocks (28 unique cards, 8 decklists). All 12 issues resolved — verified through live debug-game gameplay (2026-03-02).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Empty evo_costs for 15 of 17 Digimon in EX7/EX8/EX10 sets | crit | FIXED | evo_costs data repaired via build_registry.py fix + API refresh. All 15 cards now digivolve correctly (verified via gameplay memory deltas). |
| 2 | Spurious trash_cards.pop() before reveal in 4 scripts | med | FIXED | Script rewrites removed spurious pop. Reveal effects fire without side effects. |
| 3 | EX10-025 On Play has no process callback | med | FIXED | Process callback added. EX10-025 digivolves from Lv2 at cost 0, draw bonus works. |
| 4 | EX8-070 Zofr Kabus crashes server on play | high | FIXED | No crash. Plays for cost 2, grants Collision+Piercing+Reboot+3K DP correctly. |
| 5 | EX8-070, EX10-032 missing Collision keyword from grant | med | FIXED | Collision keyword now granted. EX10-032 shows ['piercing', 'collision'] after digivolve. |
| 6 | EX8-048/EX10-028 play_filter too broad (no name/trait filter) | med | FIXED | Filters tightened. EX8-048 correctly plays Close; EX10-028 correctly filters Mineral/Rock. |
| 7 | EX10-033/EX10-036/EX8-055 trash wrong count (1 instead of 3) | med | FIXED | Trash counts corrected. Verified via trash contents after digivolve. |
| 8 | EX10-034 WhenAttacking trashes 1 (should be 2), no SecA+1 grant | med | FIXED | EX10-034 keywords: blocker, collision, fragment correct. |
| 9 | EX10-063/P-169 suspend targets opponent instead of self | med | FIXED | Close tamers correctly suspend self when granting memory. |
| 10 | BT20-055 effect order wrong (delete before de-digivolve) | med | FIXED | De-Digivolve 2 fires in correct order before delete. |
| 11 | P-206 Delay plays tamer free instead of cost-4 reduction | low | FIXED | Uses manual_reduction=4. Also fixed color ignore (new engine support). |
| 12 | EX10-033/EX8-055 place-from-trash effects missing process callbacks | med | FIXED | Callbacks implemented. Chain effects fire correctly. |

## Report 12: Royal Knights (2026-03-01)

Full archetype QA for Royal Knights (SPECIAL ATTENTION). 35 unique cards across 9 decklists. 21 PASS, 14 PARTIAL. 12 issues found, all fixed (Mar 2-3). 8 debug games run.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | BT13-007 King Drasil_7D6 breeding cost reduction not applied | high | FIXED | Engine play-cost path + script rewritten with BeforePayCost breeding reducer |
| 2 | BT20-017 Jesmon On Play token not created | high | FIXED | Token registered in token_registry.py; script calls `effect_play_token('atho_rene_por')` |
| 3 | BT6-082 Sistermon Blanc On Play Draw 1 not triggered | high | FIXED | On Play callback implemented with draw 1 |
| 4 | BT6-082 Sistermon Blanc continuous Blocker grant not working | high | FIXED | Conditional keyword support via `can_use_condition()` check |
| 5 | ST12-12 Sistermon Blanc Decoy granted without condition check | med | FIXED | Condition check added for Huckmon/Royal Knight in play |
| 6 | BT9-103 Kongou stays in battle area instead of trash | med | FIXED | Engine option lifecycle: non-Delay options trash after resolution |
| 7 | BT8-097 Crimson Blaze stays in battle area instead of trash | med | FIXED | Engine option lifecycle: non-Delay options trash after resolution |
| 8 | BT13-111 Gallantmon missing innate Rush keyword | med | FIXED | `_is_rush = True` added via NoTiming factory effect |
| 9 | BT23-047 Examon missing Piercing and Security A. +1 | med | FIXED | `_is_piercing = True` and `_security_attack_modifier = 1` added |
| 10 | BT23-072 King Drasil_7D6 Digimon grants keywords to self | med | FIXED | Now targets `played_permanent` instead of self |
| 11 | BT20-056 Alphamon missing Barrier keyword | low | FIXED | `_is_barrier = True` added via NoTiming factory effect |
| 12 | BT23-057 Gankoomon CS On Play Hinukamuy Token not created | med | FIXED | Token registered in token_registry.py; script calls `effect_play_token('hinukamuy')` |

## Report 13: Diaboromon (2026-03-01)

Full archetype QA for Diaboromon (Token/Swarm). 26 unique cards across 6 decklists. 7 PASS, 15 PARTIAL (+ 4 previously validated). 12 issues found, 10 fixed (Mar 2-3), 2 outstanding (deferred). 3 debug games run.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Diaboromon Token play callbacks stubbed in 8 scripts | high | FIXED | All 6 scripts now call `game.effect_play_token(player, 'diaboromon')`. Verified in Report 23 gameplay. |
| 2 | BT22-053 On Play process has spurious trash pop | high | FIXED | Reveal pattern corrected; uses `effect_reveal_and_select_multi()` properly |
| 3 | EX6-036 On Play condition incorrectly blocks effect | high | FIXED | Condition unblocked; reveal fires correctly (verified in Report 23) |
| 4 | EX6-039 cost reduction not functional | med | FIXED | Self-check added (systemic Issue 24 fix); deletion cost implemented |
| 5 | EX6-041 On Play/When Digivolving missing deletion cost | med | FIXED | Added deletion of own Diaboromon as cost before digivolve via `effect_select_own_permanent` |
| 6 | BT22-057 missing tamer count check | low | FIXED | Condition now checks `tamer_count <= 1` (verified in Report 23) |
| 7 | BT22-091 attack redirect not functional | med | OUTSTANDING | SwitchDefender mechanic not in engine (deferred) |
| 8 | Overclock keyword not triggering at end of turn | med | OUTSTANDING | _is_overclock flag present but no EOT attack occurs (deferred) |
| 9 | BT19-101 uses bounce to hand instead of deck-bottom return | med | FIXED | Changed to `return_permanent_to_deck_bottom()` |
| 10 | BT24-065 When Digivolving not scaled per own Digimon | med | FIXED | Now scales de-digivolve by own Digimon count (verified in Report 23) |
| 11 | BT5-085 cost reduction untested | low | FIXED | Systemic batch fix (Issue 24) added self-check to condition |
| 12 | EX6-043 Jamming/Blocker grant is self-only | low | FIXED | Replaced self-only keywords with continuous grant to all other Diaboromon-named Digimon via `grant_keyword()` |

## Report 14: Royal Knights Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Core Royal Knights engine and script fixes landed, but deterministic gameplay re-test was not executed in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | BT13-007 breeding cost reduction implementation landed | high | FIXED | Verified via Report 27 gameplay: cost reduction applies correctly (4 + digi cards) |
| 2 | BT23-072 keyword grant target corrected in code | med | FIXED | Verified via Report 27 gameplay: Rush/Raid/Reboot/Blocker granted to played Royal Knight |
| 3 | BT20-017 / BT23-057 token callbacks implemented | med | FIXED | Verified via Report 27 gameplay: Atho/Rene/Por and Hinukamuy tokens created correctly |

## Report 15: TS Neptune Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Shared engine play-cost and trigger-context fixes landed, but no live gameplay re-test was executed.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared play-cost engine path implemented | high | FIXED | Verified via Report 27 gameplay: BT13-007 breeding cost reduction, P-186 and BT13-111 play cost all correct |

## Report 16: Rocks Re-Test (2026-03-02)

Live gameplay verification of all March 1 Rocks fixes. 20 cards verified through debug games, 7 via static analysis. 2 new issues found and fixed, 1 cosmetic issue outstanding.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | OptionSkill effects re-fire from battle area for all option plays | med | FIXED | Added played_card identity check to OptionSkill→OnUseOption mapping |
| 2 | P-206 "ignore color requirement" not enforced | med | FIXED | Added match_color_requirement check in action mask + card property override |
| 3 | effect_reveal_and_select shows "Trash from hand" instead of revealed cards | low | OUTSTANDING | Cosmetic issue with action descriptions during reveal selection |

## Report 17: Diaboromon Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. No full Diaboromon script sweep or live gameplay re-test was executed in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared engine fixes applied | med | FIXED | Verified through Report 23 Diaboromon gameplay — tokens, cost reduction, alt-digi all working |

## Report 18: CS Mastemon Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. `_alt_digi_color` support landed in the validator. Verified through Report 21 CS Mastemon gameplay.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | `_alt_digi_color` engine support implemented | high | FIXED | Verified through Report 21 CS Mastemon gameplay verification |

## Report 19: Millennium Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Shared play-cost and option-lifecycle fixes verified through Report 22 Millennium gameplay.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared engine fixes applied | med | FIXED | Verified through Report 22 Millennium gameplay verification |

## Report 20: Cross-Archetype Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Trigger context handling was tightened, but the original Royal Knights vs Medusa replay was not run in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Trigger-context handling improved | med | OUTSTANDING | Cross-archetype replay confirms deadlock persists (Report 29 #42). Selection-phase decline leaves empty action mask. |

## Report 21: CS Mastemon Gameplay Verification (2026-03-02/03)

Live gameplay verification of CS Mastemon fixes. 20 cards newly promoted to PASS (48 total PASS, 17 PARTIAL). 4 new issues found (3 fixed, 1 systemic).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 16 | Digivolve-onto-Tamer bug | high | FIXED | `can_digivolve()` allowed digivolving onto Tamers. Added `is_digimon or is_digi_egg` check. |
| 17 | EX5-059 Dobermon X missing name constraint | high | FIXED | Alt-digi set cost=0 without name="Dobermon". Any Digimon could digivolve at cost 0. |
| 18 | 261 scripts with unconstrained alt-digi effects | crit | FIXED | Batch fix tool created and run. 234 scripts patched with constraints. Remaining have incomplete constraint detection (Issue 22). |
| 19 | BT23-102 Mastemon security-trash-to-3 not implemented | med | FIXED | Added same-level card detection and security-trashing logic to When Digivolving callback. |

## Report 22: Millennium Gameplay Verification (2026-03-03)

Live gameplay verification of Millennium fixes. 17 cards newly promoted to PASS (35 total PASS, 2 PARTIAL). 4 new issues found (2 fixed, 2 systemic).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 20 | BT13-083 unconditional cost_reduction = 4 | high | FIXED | Duplicate NoTiming effect removed. Condition now checks for Lv3 on field. |
| 21 | BT19-070 missing Composite trait constraint | high | FIXED | Added `_alt_digi_trait = "Composite"` to match xros_req. |
| 22 | Batch fix incomplete constraint detection | med | OUTSTANDING | Tool skips scripts with any existing constraint. Needs smarter re-run. |
| 23 | DNA digivolve + When Digivolving game crash | high | FIXED | `_decode_selection()` guard prevents orphaned selection phases after callbacks. |

## Report 23: Diaboromon Gameplay Verification (2026-03-03)

Live gameplay verification of Diaboromon fixes. 13 cards promoted to PASS (24 total PASS, 8 PARTIAL). 2 systemic issues found and fixed, 1 script fix.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 24 | SYSTEMIC: BeforePayCost cost_reduction leak (31 scripts) | high | FIXED | Scripts with "play THIS CARD" cost reduction leaked from field. Batch fix added self-check to 31 scripts. |
| 25 | SYSTEMIC: Alt-digi validator blocking 414 cards | crit | FIXED | `_check_alt_digivolve()` called `can_use_condition({})` which fails for hand-resident cards. Removed check; `_alt_digi_*` attributes encode constraints. |
| 26 | BT24-065 condition0 checks permanent in hand | med | FIXED | Alt-digi condition checked `permanent_of_this_card()` which is None in hand. Simplified to `return True`. |

## Report 27: Royal Knights Gameplay Verification (2026-03-03)

Live gameplay verification of Royal Knights fixes. 12 PARTIAL cards tested across 2 debug games. 10 promoted to PASS, 1 remains PARTIAL (DP issue), 1 remains PARTIAL (missing cost condition). 2 pre-test script fixes applied (BT13-111, P-186).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 27 | BT20-056 Alphamon DP displays 3000 instead of 11000 | med | OUTSTANDING | Database has 11000 but in-game permanent shows 3000. Other Lv.6 Digimon display correctly. |
| 28 | BT23-057 Gankoomon unconditional cost reduction (-5) | high | OUTSTANDING | Trash-return cost (3 Huckmon/Sistermon/Jesmon from trash) not implemented in condition. |
| 29 | BT13-111/P-186 cost reduction and delete effects stubbed | low | FIXED | Added BeforePayCost with dynamic cost reduction. Rewrote delete filters. |

## Systemic Issues (Backlog)

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| S1 | Transpiled scripts use `OnEnterFieldAnyone` + `is_when_digivolving` flag instead of `EffectTiming.WhenDigivolving` | low | OUTSTANDING | Transpiler generates roundabout timing pattern. Dedicated `WhenDigivolving` timing exists and works. Affects many frozen scripts — needs bulk migration. |

## Report 28: TS Neptune Gameplay Verification (2026-03-03)

Live gameplay verification of TS Neptune fixes. 14 PARTIAL cards tested across 3 debug games. 1 promoted to PASS (Merukimon), 11 remain PARTIAL, 2 FAIL (game crashes). 3 pre-test fixes applied (duplicate cost reduction x3, Neptunemon callbacks, Neptunemon unsuspend). 12 issues found (3 fixed, 9 outstanding).

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 30 | BT24-030/040/041 duplicate cost reduction (-10 instead of -5) | high | FIXED | Transpiler generated two identical BeforePayCost effects. Removed duplicate from all 3 scripts. |
| 31 | BT24-030 On Play/When Digivolving missing process callbacks | high | FIXED | Added `_neptunemon_bottom_deck()` helper and process callbacks for both effects. |
| 32 | BT24-030 unsuspend targets any permanent instead of self | med | FIXED | Changed to `perm.unsuspend()` directly on self. |
| 33 | BT24-031 Elecmon On Play trashes from hand instead of revealing from deck | high | OUTSTANDING | Script uses wrong zone — should reveal top 3 of deck, not prompt hand trash. |
| 34 | BT24-031 Elecmon inherited effect logic inverted | med | OUTSTANDING | Security-to-hand logic is backwards. |
| 35 | BT24-029 Whamon On Play applies wrong effect | high | OUTSTANDING | Applies CANNOT_BE_SELECTED on self instead of tucking + protection. |
| 36 | BT24-102 Homeros EOT effect is a stub | med | OUTSTANDING | Only suspends Homeros, doesn't reactivate Olympos XII effects. |
| 37 | BT24-090 Abyss Sanctuary security swap not implemented | med | OUTSTANDING | OptionSkill missing security swap mechanic. |
| 38 | BT24-088 Asuna On Play crashes game | crit | OUTSTANDING | Game crashes during SelectHand phase. Game state lost. |
| 39 | BT3-093 Davis On Play crashes game | crit | OUTSTANDING | Game crashes immediately upon play. Game state lost. |
| 40 | BT24-027/028/029 tucking cost not implemented | low | OUTSTANDING | Keywords granted unconditionally. Needs `effect_place_from_hand_as_source()` engine helper (deferred). |
| 41 | BT24-028 Divermon inherited wrong filter and zone | low | OUTSTANDING | Checks "Neptunemon" instead of Lv.4 TS, targets hand instead of digi cards. |

## Report 29: Cross-Archetype Replay (2026-03-03)

Royal Knights vs Medusa cross-archetype replay. Game stuck after selection phase decline — action mask returns empty despite Main phase and active Digimon. 1 outstanding issue confirms Report 20 deadlock persists in multi-archetype games.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 42 | Cross-archetype deadlock after selection phase decline | high | OUTSTANDING | After P2 plays Medusa Elizamon (triggers On Play selection), declining the selection leaves the game stuck with empty action mask. Reproduces across multiple games. Related to Report 20 #1. |

## Report 30: Royal Knights Script Audit (2026-03-03)

Full script audit of all 35 Royal Knights cards. Found 6 systemic bug patterns across 30 scripts. Fixed all scripts and verified key cards via headless API testing. 3 outstanding issues are engine-level limitations.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 43 | CANNOT_DIGIVOLVE modifier not checked in action mask | med | OUTSTANDING | BT13-007 registers modifier but digivolve actions still appear in mask. Engine gap. |
| 44 | BeforePayCost process callbacks never fire | med | OUTSTANDING | action_play_card() never calls execute_effects(BeforePayCost). Scripts with trash-return costs (BT23-057) don't execute. |
| 45 | CANNOT_ADD_SECURITY modifier not enforced | low | OUTSTANDING | BT9-103 registers modifier but engine recovery/add-security doesn't check it. |
