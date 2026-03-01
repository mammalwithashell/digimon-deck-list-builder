# QA Issue Resolution Index

**Last updated**: 2026-03-01

## Summary

| Report | Issues | Fixed | Won't Fix | Outstanding |
|--------|--------|-------|-----------|-------------|
| [medusa](2026-02-28-medusa.md) | 14 | 12 | 2 | 0 |
| [cs-hudiemon](2026-02-28-cs-hudiemon.md) | 12 | 9 | 3 | 0 |
| [retest-medusa-hudie](2026-02-28-retest-medusa-hudie.md) | 7 | 7 | 0 | 0 |
| [medusa-vs-hudie](2026-02-28-medusa-vs-hudie.md) | 11 | 7 | 2 | 2 |
| [medusa-hudie-coverage](2026-03-01-medusa-hudie-coverage.md) | 5 | 5 | 0 | 0 |
| [partial-fixes](2026-03-01-partial-fixes.md) | 28 | 28 | 0 | 0 |
| [medusa-v2](2026-03-01-medusa-v2.md) | 5 | 4 | 1 | 0 |
| **Total** | **82** | **72** | **8** | **2** |

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
| 8 | Gotsumon reveal only allows 1 selection | med | OUTSTANDING | Code analysis shows 2-pass logic is correct; may be card availability issue |
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

5 user-reported bugs investigated. 4 fixed, 1 documented as engine limitation.

| # | Issue | Sev | Status | Fix |
|---|-------|-----|--------|-----|
| 1 | EX11-012 empty evo_costs prevents digivolve | high | FIXED | Added evo_costs to cards.json |
| 2 | Digimon can attack suspended Tamers | high | FIXED | Added `target.is_digimon` to attack mask |
| 3 | Petrification Tokens not implemented | med | WONTFIX | Engine limitation — no token creation API |
| 4 | OnLoseSecurity inherited adds memory to wrong player | high | FIXED | Use `card.owner` instead of `ctx.get('player')` |
| 5 | Lamiamon WhenDigivolving incorrect condition | high | FIXED | Removed spurious Reptile/Dragonkin ally check |
