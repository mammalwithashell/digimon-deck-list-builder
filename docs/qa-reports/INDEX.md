# QA Issue Resolution Index

**Last updated**: 2026-03-02

## Summary

| Report | Issues | Fixed | Won't Fix | Outstanding |
|--------|--------|-------|-----------|-------------|
| [medusa](2026-02-28-medusa.md) | 14 | 12 | 2 | 0 |
| [cs-hudiemon](2026-02-28-cs-hudiemon.md) | 12 | 9 | 3 | 0 |
| [retest-medusa-hudie](2026-02-28-retest-medusa-hudie.md) | 7 | 7 | 0 | 0 |
| [medusa-vs-hudie](2026-02-28-medusa-vs-hudie.md) | 11 | 8 | 2 | 1 |
| [medusa-hudie-coverage](2026-03-01-medusa-hudie-coverage.md) | 5 | 5 | 0 | 0 |
| [partial-fixes](2026-03-01-partial-fixes.md) | 28 | 28 | 0 | 0 |
| [medusa-v2](2026-03-01-medusa-v2.md) | 5 | 5 | 0 | 0 |
| [cs-hudiemon-partial-retest](2026-03-01-cs-hudiemon-partial-retest.md) | 5 | 2 | 0 | 3 |
| [medusa-partial-retest](2026-03-01-medusa-partial-retest.md) | 5 | 3 | 0 | 2 |
| [ts-neptune](2026-03-01-ts-neptune.md) | 8 | 0 | 0 | 8 |
| [rocks](2026-03-01-rocks.md) | 12 | 0 | 0 | 12 |
| [royal-knights](2026-03-01-royal-knights.md) | 12 | 0 | 0 | 12 |
| [diaboromon](2026-03-01-diaboromon.md) | 12 | 0 | 0 | 12 |
| [cs-mastemon](2026-03-01-cs-mastemon.md) | 5 | 0 | 0 | 5 |
| [millennium](2026-03-01-millennium.md) | 10 | 0 | 0 | 10 |
| [cross-archetype-matchups](2026-03-01-cross-archetype-matchups.md) | 1 | 0 | 0 | 1 |
| [royal-knights-retest](2026-03-02-royal-knights-retest.md) | 0 | 0 | 0 | 0 |
| [ts-neptune-retest](2026-03-02-ts-neptune-retest.md) | 0 | 0 | 0 | 0 |
| [rocks-retest](2026-03-02-rocks-retest.md) | 0 | 0 | 0 | 0 |
| [diaboromon-retest](2026-03-02-diaboromon-retest.md) | 0 | 0 | 0 | 0 |
| [cs-mastemon-retest](2026-03-02-cs-mastemon-retest.md) | 0 | 0 | 0 | 0 |
| [millennium-retest](2026-03-02-millennium-retest.md) | 0 | 0 | 0 | 0 |
| [cross-archetype-retest](2026-03-02-cross-archetype-retest.md) | 0 | 0 | 0 | 0 |
| **Total** | **152** | **79** | **7** | **66** |

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

Full archetype QA for TS Neptune (30 unique cards, 10 decklists). 16 PASS, 14 PARTIAL. 8 issues found, all outstanding.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Persistent pending selection causes game deadlock | high | OUTSTANDING | "Play from hand without cost" selection loops across turns, eventually deadlocks at phase 0 |
| 2 | Play cost reduction ("When this card would be played") not applied | high | OUTSTANDING | Affects Neptunemon, Venusmon, Merukimon, Minervamon -- all charge full 12 cost instead of reduced 7 |
| 3 | Homeros +1000 DP to TS Digimon not applied | med | OUTSTANDING | DP breakdown shows no modifier from Homeros All Turns effect |
| 4 | Lanamon When Digivolving skips hand-card placement | med | OUTSTANDING | "By placing" cost step not implemented |
| 5 | Asuna Shiroki On Play trash-to-draw not triggered | med | OUTSTANDING | No selection prompt for optional trash/draw effect |
| 6 | Tidal Stream Link mechanic not functional | med | OUTSTANDING | Card remains separate permanent; linkedCardIds empty on target |
| 7 | Divermon has no DP in card database | low | OUTSTANDING | play_cost=0, dp=None for Lv5 Digimon |
| 8 | Davis Motomiya On Play reveal not triggered | med | OUTSTANDING | Legacy card; reveal top 3 effect does not fire |

## Report 11: Rocks (2026-03-01)

Full archetype QA for Rocks (28 unique cards, 8 decklists). 15 PASS, 13 PARTIAL. 12 issues found, all outstanding.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Empty evo_costs for 15 of 17 Digimon in EX7/EX8/EX10 sets | crit | OUTSTANDING | Prevents digivolving for nearly all Rocks cards. Only P-167 and BT16-082 have valid evo_costs. |
| 2 | Spurious trash_cards.pop() before reveal in 4 scripts | med | OUTSTANDING | EX8-047, P-107, P-039, P-206 take last trash card to hand before reveal begins |
| 3 | EX10-025 On Play has no process callback | med | OUTSTANDING | Place-from-trash effect registered but no action code |
| 4 | EX8-070 Zofr Kabus crashes server on play | high | OUTSTANDING | JSON decode error when executing Option play |
| 5 | EX8-070, EX10-032 missing Collision keyword from grant | med | OUTSTANDING | grant_keyword calls omit _is_collision |
| 6 | EX8-048/EX10-028 play_filter too broad (no name/trait filter) | med | OUTSTANDING | Accepts all cards instead of filtering for Close/Mineral/Rock |
| 7 | EX10-033/EX10-036/EX8-055 trash wrong count (1 instead of 3) | med | OUTSTANDING | trash_digivolution_cards(1) should be trash_digivolution_cards(3) |
| 8 | EX10-034 WhenAttacking trashes 1 (should be 2), no SecA+1 grant | med | OUTSTANDING | Missing Security Attack +1 keyword grant |
| 9 | EX10-063/P-169 suspend targets opponent instead of self | med | OUTSTANDING | Uses effect_select_opponent_permanent instead of suspending own tamer |
| 10 | BT20-055 effect order wrong (delete before de-digivolve) | med | OUTSTANDING | Should de-digivolve first, then delete |
| 11 | P-206 Delay plays tamer free instead of cost-4 reduction | low | OUTSTANDING | Should reduce cost by 4, not play free |
| 12 | EX10-033/EX8-055 place-from-trash effects missing process callbacks | med | OUTSTANDING | Effects registered with no implementation code |

## Report 12: Royal Knights (2026-03-01)

Full archetype QA for Royal Knights (SPECIAL ATTENTION). 35 unique cards across 9 decklists. 21 PASS, 14 PARTIAL. 12 issues found, all outstanding. 8 debug games run.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | BT13-007 King Drasil_7D6 breeding cost reduction not applied | high | OUTSTANDING | "When Royal Knight would be played, reduce cost by 4 + 1 per evo card" never triggers |
| 2 | BT20-017 Jesmon On Play token not created | high | OUTSTANDING | [Atho, Rene & Por] Token not generated; token template may not be registered |
| 3 | BT6-082 Sistermon Blanc On Play Draw 1 not triggered | high | OUTSTANDING | Hand decreases by 1 on play with no draw |
| 4 | BT6-082 Sistermon Blanc continuous Blocker grant not working | high | OUTSTANDING | Blocker not granted to Sistermons even with Royal Knight in play |
| 5 | ST12-12 Sistermon Blanc Decoy granted without condition check | med | OUTSTANDING | Decoy shows without Huckmon/Royal Knight in play |
| 6 | BT9-103 Kongou stays in battle area instead of trash | med | OUTSTANDING | Non-Delay Option placed as permanent |
| 7 | BT8-097 Crimson Blaze stays in battle area instead of trash | med | OUTSTANDING | Non-Delay Option placed as permanent |
| 8 | BT13-111 Gallantmon missing innate Rush keyword | med | OUTSTANDING | Rush not in keywords list despite being in card text |
| 9 | BT23-047 Examon missing Piercing and Security A. +1 | med | OUTSTANDING | Both innate keywords absent |
| 10 | BT23-072 King Drasil_7D6 Digimon grants keywords to self | med | OUTSTANDING | Rush/Raid/Reboot/Blocker on self instead of played Digimon |
| 11 | BT20-056 Alphamon missing Barrier keyword | low | OUTSTANDING | Barrier not in keywords list |
| 12 | BT23-057 Gankoomon CS On Play Hinukamuy Token not created | med | OUTSTANDING | Same token system issue as Jesmon |

## Report 13: Diaboromon (2026-03-01)

Full archetype QA for Diaboromon (Token/Swarm). 26 unique cards across 6 decklists. 7 PASS, 15 PARTIAL (+ 4 previously validated). 12 issues found, all outstanding. 3 debug games run.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Diaboromon Token play callbacks stubbed in 8 scripts | high | OUTSTANDING | EX6-043, BT22-064, BT24-052, BT22-059, EX6-036, EX6-039 have `pass` instead of `game.effect_play_token(player, 'diaboromon')` |
| 2 | BT22-053 On Play process has spurious trash pop | high | OUTSTANDING | Steals card from trash before reveal; filter too broad |
| 3 | EX6-036 On Play condition incorrectly blocks effect | high | OUTSTANDING | Checks "Diaboromon" in own card text which fails; reveal never fires |
| 4 | EX6-039 cost reduction not functional | med | OUTSTANDING | Deletion cost not implemented; reduction property not consumed |
| 5 | EX6-041 On Play/When Digivolving missing deletion cost | med | OUTSTANDING | Free digivolve fires without deleting own Diaboromon |
| 6 | BT22-057 missing tamer count check | low | OUTSTANDING | Always allows Arata Sanada play regardless of tamer count |
| 7 | BT22-091 attack redirect not functional | med | OUTSTANDING | SwitchDefender mechanic not in engine |
| 8 | Overclock keyword not triggering at end of turn | med | OUTSTANDING | _is_overclock flag present but no EOT attack occurs |
| 9 | BT19-101 uses bounce to hand instead of deck-bottom return | med | OUTSTANDING | Also missing trash-to-deck cost |
| 10 | BT24-065 When Digivolving not scaled per own Digimon | med | OUTSTANDING | Single de-digivolve instead of N per own Digimon |
| 11 | BT5-085 cost reduction untested | low | OUTSTANDING | _temp_play_cost_reduction pattern may not be consumed by engine |
| 12 | EX6-043 Jamming/Blocker grant is self-only | low | OUTSTANDING | Should grant to all other Diaboromon-named Digimon |

## Report 14: Royal Knights Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Core Royal Knights engine and script fixes landed, but deterministic gameplay re-test was not executed in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | BT13-007 breeding cost reduction implementation landed | high | OUTSTANDING | Engine path and script were rewritten; runtime verification still pending |
| 2 | BT23-072 keyword grant target corrected in code | med | OUTSTANDING | Now targets `played_permanent`; live validation still pending |
| 3 | BT20-017 / BT23-057 token callbacks implemented | med | OUTSTANDING | Now call `game.effect_play_token(...)`; live validation still pending |

## Report 15: TS Neptune Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Shared engine play-cost and trigger-context fixes landed, but no live gameplay re-test was executed.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared play-cost engine path implemented | high | OUTSTANDING | Intended to address March 1 cost-reduction failures; runtime verification pending |

## Report 16: Rocks Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Shared engine changes landed, but the EX7 / EX8 / EX10 evo-cost data repair is still pending.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared engine fixes applied | med | OUTSTANDING | Runtime validation pending |
| 2 | Missing evo-cost data repair not completed in this session | crit | OUTSTANDING | Requires targeted ingestion/data pass |

## Report 17: Diaboromon Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. No full Diaboromon script sweep or live gameplay re-test was executed in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared engine fixes applied | med | OUTSTANDING | Diaboromon-specific runtime validation still pending |

## Report 18: CS Mastemon Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. `_alt_digi_color` support landed in the validator, but affected scripts were not fully migrated in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | `_alt_digi_color` engine support implemented | high | OUTSTANDING | Script migration and gameplay verification still pending |

## Report 19: Millennium Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Shared play-cost and option-lifecycle fixes landed, but live gameplay re-test was not executed.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Shared engine fixes applied | med | OUTSTANDING | Millennium-specific runtime verification still pending |

## Report 20: Cross-Archetype Re-Test Follow-Up (2026-03-02)

Implementation follow-up report only. Trigger context handling was tightened, but the original Royal Knights vs Medusa replay was not run in this session.

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| 1 | Trigger-context handling improved | med | OUTSTANDING | Cross-archetype replay still required |

## Systemic Issues (Backlog)

| # | Issue | Sev | Status | Notes |
|---|-------|-----|--------|-------|
| S1 | Transpiled scripts use `OnEnterFieldAnyone` + `is_when_digivolving` flag instead of `EffectTiming.WhenDigivolving` | low | OUTSTANDING | Transpiler generates roundabout timing pattern. Dedicated `WhenDigivolving` timing exists and works. Affects many frozen scripts — needs bulk migration. |
