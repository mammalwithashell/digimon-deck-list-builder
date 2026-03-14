# Archetype QA: Jesmon
Date: 2026-03-14
Total cards: 118

## Summary
- Frozen: 87 (QA pending)
- Unfrozen (prior reviewed): 16
- IMPLEMENTED: 15 new scripts (13 with C#, 2 from API)
- FIXED: 8 scripts with bugs found during spot-check
- BLOCKED: 0

## Implemented Cards

### Batch 1 (8 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT6-009 | BaoHuckmon | On Play: reveal 5, add up to 2 Huckmon/Jesmon/Sistermon |
| BT6-011 | SaviorHuckmon | Inherited: [When Attacking] OPT delete opp <=5000 DP if Sistermon in play |
| BT6-015 | Jesmon | When Digi: play Sistermon free. Inherited: unsuspend if Sistermon in play |
| BT7-082 | Sistermon Blanc (Awakened) | On Play: place Sistermon Blanc under + Recovery +1. On Deletion: return Jesmon/Huckmon/Sistermon from trash |
| BT9-092 | Hina Kurihara | Tamer. On Play: reveal 3 for X Antibody. Suspend on same-level X digi for +1 memory + Draw 1 |
| BT9-109 | X Antibody | Option. Place under Digimon as digi-card. Inherited: protect X digi-cards + digi into X Antibody on attack |
| BT4-001 | Sakuttomon | Digi-Egg. Inherited: [When Attacking] OPT if Lv7, +1 memory |
| ST12-03 | Solarmon | [All Turns] Players can't reduce play costs |

### Batch 2 (7 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT12-001 | Gigimon | Digi-Egg. Inherited: +1000 to DP deletion threshold |
| BT18-009 | Shamanmon | [All Turns] Opponent can't gain memory from Digimon effects |
| BT3-097 | A Delicate Plan | Option. Grant security-option-immunity. Security: add to hand |
| BT5-086 | Omnimon | Blitz. When Digi: unsuspend. Prevent deletion by trashing Lv6 from digi-stack |
| EX2-064 | Alice McCoy | Tamer. BeforePayCost: delete own Digimon for evo cost -3 (Lv5->Lv6). Security: play free |
| LM-033 | Garnet Memory Boost! | Reveal 3, add red/black Digimon. Delay +2 memory. Security: place in BA |
| ST16-14 | Matt Ishida | Tamer. Start turn: memory 3. On hand trash: suspend for +1 memory. Security: play free |

## Fixes Applied (2026-03-14)

### Stub Fixes (3 cards)

| Card | Name | Issue | Fix |
|------|------|-------|-----|
| BT19-072 | LordKnightmon | Redirect effect was stub (pass body) with wrong timing (OnAllyAttack) | Rewrote to use `_is_when_attacked_observer` + `switch_attack_target()` + `effect_select_own_permanent` for Royal Knight target selection |
| BT23-047 | Examon | Suspend auto-selected first 5 without player choice; "then may attack" was stub | Rewrote suspend to use chained `effect_select_opponent_permanent` callbacks for proper player selection; added `FORCE_ATTACK` modifier for "then this Digimon may attack" |
| BT23-077 | Sistermon Ciel | Misleading "stub" comment on name aliasing | Removed stale stub comment; `card.also_treated_as_names` was already correctly set. All effects (Blocker, On Play delete, De-Digivolve on suspend) verified correct |

### Spot-Check Fixes (5 cards)

| Card | Name | Issue | Fix |
|------|------|-------|-----|
| BT7-082 | Sistermon Blanc (Awakened) | On Deletion auto-selected first qualifying card from trash | Replaced with `request_selection` using `SEL_TRASH_START` indices for proper player choice |
| ST12-10 | Jesmon | Effect2 ("when you play another Digimon by effect") used `is_on_play=True` which only triggers on self-play | Rewrote to use `_is_play_observer` pattern which correctly fires for other Digimon entering; checks played Digimon is_digimon and owner's turn |
| ST12-13 | Sistermon Ciel | Reveal effect had spurious `trash_cards.pop()` before reveal; remaining cards went to deck bottom instead of trash; Reboot aura checked source permanent instead of target | Fixed reveal to use `effect_reveal_and_select_multi` with `remaining_placement='trash'`; rewrote Reboot as single aura effect with `_keyword_permanent_condition` filter for Huckmon/Royal Knight targets |
| ST12-14 | Aus Generics | DP+2000 and Piercing selections were sequential (second overwrote first); Security effect popped random card from trash | Fixed DP grant to chain Piercing selection inside callback; fixed security to add THIS card to hand |
| BT22-043 | Terriermon | Play filter accepted all cards instead of CS Tamers; [Main] cost (place top stacked to bottom) not paid; CS trait not checked on permanent | Fixed filter to CS Tamer only + 1-or-fewer-Tamers check; added card rearrangement cost; added CS trait condition |

### Spot-Check PASS (10 cards)

| Card | Verdict | Notes |
|------|---------|-------|
| BT6-009 | PASS | Reveal & multi-select correct |
| BT6-011 | PASS | Inherited delete <=5000 DP correct |
| BT6-015 | PASS | When Digi play Sistermon + inherited unsuspend correct (docstring has wrong name, cosmetic) |
| BT6-082 | PASS | Blocker aura with Huckmon/RK condition correct |
| BT6-084 | PASS | +2000 DP aura to RK/Huckmon correct |
| BT13-019 | PASS | Complex branch choice (Sistermon from trash OR RK from breeding) well implemented |
| BT9-109 | PASS | X Antibody placement, protection modifier, digivolve-on-attack all correct |
| BT20-083 | PASS | Name alias + Blocker + On Play digivolve into Omnimon X + On Deletion place under King Drasil correct |
| EX10-068 | PASS | Name alias + Start Main memory gain + On Play delete chain correct |
| ST12-12 | PASS | Trash-to-draw + conditional Decoy correct (color restriction noted as engine gap) |

### Engine Gaps Identified
- **Attack redirect selection**: BT19-072 needs player to choose which Royal Knight to redirect to. Implemented using `effect_select_own_permanent` + `switch_attack_target`, which works but fires synchronously (no queued multi-step for redirect window).
- **"By an effect" play detection**: ST12-10 cannot distinguish between normal plays and effect plays. The engine doesn't propagate `played_by_effect` context flag. Used `_is_play_observer` which fires for all plays of other Digimon. In Jesmon decks, most mid-combat plays ARE by effect (Sistermon from When Attacking).
- **FORCE_ATTACK optionality**: BT23-047 "this Digimon may attack" uses FORCE_ATTACK which restricts action mask to attack only. The "may" (optional) aspect is lost, but if the Digimon can't attack (suspended), the mask falls through to normal.

## Smoke Test
- 50/50 mirror games completed (3 different Jesmon decklists, varied matchups)
