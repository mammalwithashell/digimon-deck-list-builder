# QA Report: CS Mastemon Archetype

**Date:** 2026-03-01
**Archetype:** CS Mastemon (65 unique cards, 9 decklists)
**Best Decklist:** egman_b8fbc17d3cfb (9th place, 24 unique cards)
**Method:** Debug game API with inject-card, script review, action mask analysis
**Games Created:** 5 debug games

## Summary

- **Cards Tested:** 55 (previously untested)
- **PASS:** 18
- **PARTIAL:** 37
- **Previously Validated (by other reports):** 10

## Critical Findings

### 1. Digivolution Color Check Not Enforced (Engine-Level Bug)

The `_alt_digi_*` mechanism in `digivolve_validator.py` does not check color for alt digivolution requirements. Cards like BT23-031 Angewomon (Yellow Lv5 from Yellow Lv4) and BT23-067 LadyDevimon (Purple Lv5 from Purple Lv4) set `_alt_digi_level = 4` but no `_alt_digi_color`, allowing digivolution onto any color Lv4 base. This was confirmed in-game: LadyDevimon could digivolve onto Yellow Reppamon, and Angewomon could digivolve onto Purple Dobermon.

**Affected cards:** BT23-031, BT23-067, BT23-102, BT22-031, BT22-054, BT22-056, BT16-030, EX8-030

### 2. Systematic "Recovery/Trash/AddToHand" Transpiler Pattern Bug

Multiple scripts share an identical broken process callback pattern:
```python
player.recovery(1)  # unconditional recovery
if player.trash_cards:
    card_to_add = player.trash_cards.pop()
    player.hand_cards.append(card_to_add)
enemy.security_cards.pop(0)  # trash opponent security
```
This pattern appears in BT23-031, BT11-042, BT14-084, and appears to be a transpiler error that confuses "add security card to hand" with "recovery + add from trash + trash opponent security."

**Affected cards:** BT23-031 (On Play/When Digivolving), BT11-042 (When Digivolving), BT14-084 (On Play)

### 3. Missing Condition Checks in Triggered Effects

Many effects that should check what was played/digivolved trigger unconditionally:
- BT11-042, BT11-083: "+1 memory when playing LadyDevimon/Angewomon/Mirei" triggers on ANY play
- BT11-094: Mirei plays Angewomon/LadyDevimon on ANY digivolve, not just into those names
- EX6-074: Gains memory on any play, not just Holy Beast/Archangel/Fallen Angel traits

### 4. Self-Suspend vs Opponent-Suspend Confusion

Multiple Tamer scripts call `game.effect_select_opponent_permanent(player, on_suspend, ...)` when the card text says "by suspending this Tamer." The script suspends an opponent's permanent instead of self.

**Affected cards:** BT8-090, BT11-094, EX6-074

### 5. DNA Digivolve Check Missing

P-187 Mastemon and EX6-029 Mastemon effects that should only trigger "if DNA digivolving" always trigger on any digivolution.

---

## Card-by-Card Results

### DigiEggs (Lv.2)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT14-003 | Tokomon | PASS | Inherited OnAddSecurity Draw 1 with Once Per Turn. Correctly uses EffectTiming.OnAddSecurity and is_inherited_effect. |
| BT15-003 | Nyaromon | PARTIAL | Inherited attack effect gains 1 memory but does not actually trash a security card as cost. Process just adds memory without removing security. |
| BT6-006 | Tsunomon | PASS | Inherited OnDiscardHand Draw 1 with Once Per Turn. Correct timing and process. |
| BT22-004 | Wanyamon | PARTIAL | OnAddDigivolutionCards trigger present but digi_filter accepts ANY card, not just CS trait. |

### Rookies (Lv.3)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| ST10-02 | Salamon | PASS | Inherited End of Turn DNA digivolve from hand. Uses effect_dna_digivolve_from_hand with dna_filter checking dna_costs. Core archetype enabler works. |
| EX6-016 | Salamon | PASS | Start of Main Phase +1 memory. Condition checks permanent and turn. Inherited -2000 DP on attack auto-targets weakest opponent Digimon. |
| BT13-034 | Kudamon | PARTIAL | On Play reveal top 3 and add targets. Uses card_traits check for Vaccine but trait access pattern may fail on some card sources. |
| BT14-033 | Patamon | PARTIAL | Start of Main Phase digivolve from security is implemented as play_from_zone(hand) instead of searching security. Inherited OnAddSecurity +1 memory is correct. |
| BT14-070 | Goblimon | PASS | Inherited OnDiscardHand +1 memory. Correct timing and Once Per Turn. |
| BT16-030 | Salamon (YP) | PARTIAL | Alt digi has no level/color restriction (_alt_digi_cost=0 only). Start of Main Phase digivolve uses effect_digivolve_from_hand instead of trash. Trait check for Holy Beast/Free is present. |
| BT19-067 | Impmon | PARTIAL | On Play plays purple Tamer from trash. Filter should check play cost <= 4 and purple color, but uses unrestricted filter. Missing "1 or fewer Tamers" condition check. |
| BT5-033 | Cutemon | PARTIAL | "[Opponent's Turn] Opponent can't reduce evo costs" tagged as descriptive only. Engine lacks cost-lock mechanism. |
| BT8-035 | Candlemon | PARTIAL | Inherited effect says "When you play another purple Digimon" but condition doesn't check if played card is purple Digimon. Triggers on any play. |
| BT8-071 | Psychemon | PARTIAL | "[All Turns] Players can't reduce play costs" descriptive only, no enforcement. |
| BT9-033 | Pillomon | PARTIAL | "[All Turns] Players can't play Digimon by effects" descriptive only, no enforcement. |
| EX5-028 | Kudamon | PARTIAL | On Play plays yellow Tamer from hand if total security <= 6. Filter doesn't check for Tamer/yellow. Inherited -2000 DP correct. |
| EX5-057 | Labramon | PARTIAL | On Play trash from hand to return Dark Animal/Shaman from trash. No hand_filter for what to trash. Inherited OnEnterFieldAnyone +1 memory doesn't check if played card is a Digimon. |
| EX8-030 | Tapirmon | PARTIAL | "[All Turns] Opponent can't gain memory" descriptive only. Has unrelated alt_digi_trait="NSo" which is wrong for this card. |
| BT22-054 | Hagurumon | PARTIAL | Alt digi Lv2 for 0 cost (correct for egg -> Lv3 but no color check). OnAddDigivolutionCards -3000 DP no process callback. Inherited draw has condition check. |
| BT9-082 | Ordinemon (injected) | PARTIAL | Mass delete effect works but missing DNA digivolving check. Auto-selects first Lv6+ target. On Deletion revive reasonable. |

### Champions (Lv.4)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| ST10-04 | Gatomon (YP) | PASS | On Play reveal top 3 add yellow + purple Digimon. Cost reduction for Archangel/Fallen Angel digivolve. Inherited End of Turn DNA. No evo_costs in card data but has alt_digi via script. |
| EX6-020 | Gatomon (Y) | PASS | On Play reveal top 3 add Angel/Archangel/Fallen Angel + Mirei Mikagura. Inherited -2000 DP on attack. |
| BT15-037 | Gatomon (Y) | PASS | Barrier (own + inherited). OnLoseSecurity +1 memory with Once Per Turn. OnDiscardSecurity play self free present. |
| BT22-034 | Reppamon | PASS | On Play/When Digivolving -3000 DP to opponent. Plays correctly, costs correct. Digivolve onto Yellow Lv3 works. |
| ST20-05 | Gatomon (Y) | PASS | Security play free. On Play/When Digivolving Security A. -1 to 2 opponent Digimon. Inherited -2000 DP on attack. |
| BT14-073 | Ogremon | PASS | OnDiscardHand +1 memory (both own effect and inherited). Correct timing and Once Per Turn. |
| BT16-068 | Dobermon | PASS | On Play/When Digivolving grants Blocker to target. Target selection phase works. |
| BT22-031 | GoldNumemon | PARTIAL | Alt digi Lv4 for 2 cost (no color check). On Play/When Digivolving digivolve filter accepts any card, missing Security A. -2 application and same-level stack check. |
| BT22-056 | Guardromon | PARTIAL | Alt digi Lv3 for 2 cost (no color check). De-Digivolve mechanism present but missing -3000 DP application and same-level check. Inherited +2000 DP correct. |
| EX5-059 | Dobermon (X Antibody) | PARTIAL | On Play Retaliation grant and When Digivolving draw+trash. Filter for X Antibody trait recovery missing. |

### Ultimates (Lv.5)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT23-031 | Angewomon | PARTIAL | Alt digi Lv4 for 3 (missing Yellow color restriction). On Play/When Digivolving: completely wrong -- does Recovery+1 unconditionally, adds from trash, trashes opponent security. Should add top security to hand, then conditional recovery. Inherited Alliance marker correct. |
| BT23-067 | LadyDevimon | PARTIAL | Alt digi Lv4 for 3 (missing Purple color restriction). Delete effect filter missing Lv4-or-lower check. Play cost reduction condition doesn't check for Angewomon/Mirei. Blocker and inherited Scapegoat correct. |
| BT11-042 | Angewomon | PARTIAL | When Digivolving: same broken pattern as BT23-031 (recovery+trash+opponent security instead of searching security for Angel trait). Memory +1 on LadyDevimon/Mirei play doesn't check name. Inherited Blocker correct. |
| BT11-083 | LadyDevimon | PARTIAL | When Digivolving: trash filter wrong (requires Angel trait for what to trash, should be unrestricted). Return from trash filter missing. Memory +1 on Angewomon/Mirei play doesn't check name. Inherited Retaliation correct. |
| EX6-022 | Angewomon | PARTIAL | Barrier correct. On Play/When Digivolving: plays any card from hand instead of checking Mirei Mikagura condition for Security A. -2 vs playing Mirei. |
| BT18-082 | Lucemon: Chaos Mode | PASS | On Play/When Digivolving: opponent deletes or recovery+trash. Simplified (opponent auto-declines). All Turns leave prevention tagged. |
| BT19-039 | SkullBaluchimon | PARTIAL | On Play/When Digivolving: gains memory and deletes Lv4 or lower (filter correct). BUT also trashes OPPONENT's security instead of own security as cost. On Deletion Recovery not present. |
| BT16-075 | Cerberusmon | PASS | On Play/When Digivolving: return Dark Animal/Shaman from trash to hand. Inherited play-triggered Rush grant. |
| BT17-025 | Cerberusmon: WM | PARTIAL | When Digivolving: plays Lv3 blue/purple from trash. Filter doesn't check level or color. "Return to hand at end of opponent's turn" not implemented. |
| EX5-061 | Cerberusmon (XA) | PARTIAL | On Play: plays purple Lv3 from trash (filter doesn't check level/color). When Digivolving draw+trash present. Inherited attack unsuspend present. |

### Megas (Lv.6)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| P-187 | Mastemon | PARTIAL | When Digivolving: Recovery +1 works. But unconditionally trashes opponent security (should check DNA digivolving). Place to security doesn't include Tamers. On Attack plays from hand_or_trash but also unconditionally trashes opponent security. |
| BT23-102 | Mastemon | PARTIAL | Alt digi Lv5 for 5 (missing Yellow color restriction). Barrier and Partition markers correct. When Digivolving play filter missing yellow/purple color check. Missing same-level stack security trash. OnLoseSecurity has unexpected effect immunity modifier. |
| EX6-029 | Mastemon | PARTIAL | Blast DNA Digivolve marker correct. On Play/When Digivolving play filter missing Angel/Archangel/Fallen Angel trait check. Plays from hand only, not hand_or_trash. DNA check missing for security manipulation. Effect immunity modifier shouldn't be here. |

### Lv.7

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT9-082 | Ordinemon | PARTIAL | Mass delete works but no DNA digivolving check. On Deletion revive reasonable. |
| EX4-074 | ShineGreymon: RM | PARTIAL | When Digivolving/On Deletion -5000 DP applies but no duration tracking. End of Attack self-delete + opponent delete + recovery + tamer hatch works. Auto-selects targets. |

### Tamers

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT11-094 | Mirei Mikagura | PARTIAL | Start of Turn +1 memory correct. "When digivolving into Angewomon/LadyDevimon" effect: suspends opponent instead of self, plays any card instead of Angewomon/LadyDevimon, no name/count check. |
| EX6-074 | Mirei Mikagura | PARTIAL | Your Turn trait-play trigger: gains memory but suspends opponent instead of self. Digivolve from hand instead of trash. End of Turn DNA: uses play_from_zone instead of DNA digivolve. |
| BT8-090 | Kari Kamiya | PARTIAL | Set memory to 3 correct. OnAddSecurity +1 memory: gains memory but suspends opponent instead of self. |
| BT1-087 | T.K. Takaishi | PASS | Set memory to 3 correct. On Play search security correct (simplified first-card pick). Security play correct. |
| BT14-084 | T.K. Takaishi | PARTIAL | On Play: broken pattern (adds from trash, recovery, trashes opponent security) instead of returning top security to hand and placing Vaccine card. OnAddSecurity +1 memory correct. |
| BT22-093 | Ami Aiba | PARTIAL | Start of Main Phase +1 memory if opponent has Digimon. Missing condition check. CS trait digivolve effect not reviewed (truncated). |
| BT23-088 | K | PARTIAL | Start of Main Phase: trash from hand to gain memory. Filter should check Undead/Dark Animal/CS trait. End of Turn: self-delete to digivolve effects not fully reviewed. |

### Options

| Card | Name | Status | Notes |
|------|------|--------|-------|
| LM-035 | Amber Memory Boost! | PASS | Reveal top 3, add yellow/purple Digimon. Delay marker + gain 2 memory effect. Security place. All functional. |
| P-040 | Purple Memory Boost! | PASS | Reveal top 4, add purple Digimon. Delay + gain 2 memory. Security place. |
| P-108 | Wisdom Training | PASS | Reveal top 2, add purple card. Delay + gain 2 memory. Security place. |
| EX5-070 | X Antibody Proto Form | PARTIAL | Security add to hand present. Main digivolve effect uses generic filter. Inherited leave-field protection not implemented. |

## Engine-Level Issues Found

1. **Alt digi color check missing**: `_check_alt_digivolve()` in `digivolve_validator.py` checks `_alt_digi_level` and `_alt_digi_trait` but has no `_alt_digi_color` field support. All alt digi cards that should restrict by color are broken.

2. **Descriptive-only lock effects**: BT5-033, BT8-071, BT9-033, EX8-030 have continuous lock effects that are tagged as descriptive only. The engine lacks mechanisms to prevent play-by-effect, cost reduction, or memory gain.

3. **DP modifier duration**: Effects that apply "until end of opponent's next turn" (EX4-074) use `dp_modifier` directly without expiry tracking.

## Recommendations

1. **Add `_alt_digi_color` to digivolve_validator.py** -- highest priority, affects 8+ cards.
2. **Fix the systematic "Recovery/Trash/AddToHand" transpiler pattern** in BT23-031, BT11-042, BT14-084.
3. **Fix self-suspend vs opponent-suspend** in BT8-090, BT11-094, EX6-074 Tamer scripts.
4. **Add name-check conditions** to triggered effects (BT11-042, BT11-083, BT11-094).
5. **Add DNA-digivolving conditional checks** to P-187 and EX6-029 Mastemon effects.
