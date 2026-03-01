# Gameplay QA Report — Medusa vs CS Hudiemon (Matchup)

## Test Setup
- **Date**: 2026-02-28
- **Archetypes**: Medusa (P1/P2) vs CS Hudiemon (P2/P1)
- **Game 1 ID**: cb0051ce-c333-4889-b359-8fc1ff079737 (Medusa P1 first)
- **Game 2 ID**: fcddce11-844e-4918-aa63-518a1b5cfeb6 (Hudie P1 first)
- **Total Turns Played**: ~9 per game
- **Focus Areas**: Cross-archetype interactions, Alliance keyword, security battles, DNA digivolve, trait-based evo costs, keyword mechanics

## Summary
- **Total Issues Found**: 11
- Critical: 0 | High: 4 | Medium: 4 | Low: 3

## Detailed Findings

### Issue 1: DNA digivolve allowed with only 1 Digimon on field
- **Card(s)**: BT23-050 (Ankylomon), BT23-032 (Shakkoumon)
- **Severity**: high
- **Category**: effect
- **Expected**: Ankylomon's On Play effect "2 of your Digimon may DNA digivolve into [Shakkoumon] in the hand" requires 2 Digimon on the battle area. With only 1 Digimon (Ankylomon), the DNA digivolve option should NOT appear.
- **Actual**: The engine offered "Play Shakkoumon from hand" as an action with only 1 Digimon on the field. Shakkoumon was placed as a standalone card (sourceCount=1, not DNA digivolved).
- **Steps to Reproduce**:
  1. Create game with P2 having BT23-050 and BT23-032 in hand
  2. Play Chitose Imai (BT23-081) to free-play Ankylomon
  3. Ankylomon On Play triggers, offers DNA digivolve with only 1 Digimon
- **Evidence**: Game cb0051ce, Shakkoumon placed with sourceCount=1, no DNA materials underneath
- **Rules Reference**: DNA digivolution (8-2) requires multiple field Digimon as materials

### Issue 2: Shakkoumon [When Digivolving] fires on PLAY
- **Card(s)**: BT23-032 (Shakkoumon)
- **Severity**: high
- **Category**: effect
- **Expected**: [When Digivolving] effects only trigger when a card is digivolved, not when played.
- **Actual**: Shakkoumon was placed via a bugged DNA-as-play action, but its [When Digivolving] effect ("give 1 opponent Digimon forced attack") still triggered.
- **Steps to Reproduce**: Same as Issue 1 - after Shakkoumon is placed, the force attack effect immediately activates.
- **Evidence**: Internal state shows "BT23-032 Attack with this Digimon" effect active on Shakkoumon despite being played, not digivolved.
- **Rules Reference**: [When Digivolving] is a trigger-type effect that only triggers on digivolution (15-5)

### Issue 3: Shakkoumon force attack effect never expires
- **Card(s)**: BT23-032 (Shakkoumon)
- **Severity**: high
- **Category**: effect
- **Expected**: The force attack effect "Until your opponent's turn ends, give 1 of their Digimon '[Start of Your Main Phase] This Digimon attacks'" should expire at the end of the opponent's next turn.
- **Actual**: The force attack persists permanently, triggering every turn for both players' main phases. It forces an attack action before any other main phase actions can be taken.
- **Steps to Reproduce**: Same as Issue 1-2. Observe force attack on subsequent turns.
- **Evidence**: Force attack triggered on P1's turn 3, P2's turn 4, P1's turn 5, all with the same effect.
- **Rules Reference**: "Until your opponent's turn ends" is a duration that should expire (15-7)

### Issue 4: Digivolution source cards leaked on deletion
- **Card(s)**: BT24-008 (Elizamon), BT21-001 (Gigimon)
- **Severity**: high
- **Category**: game_flow
- **Expected**: When a Digimon stack is deleted, the top card and all digivolution cards should go to trash (4-2-8).
- **Actual**: When Elizamon (with Gigimon underneath) was destroyed during the forced attack, BT24-008 went to trash but BT21-001 Gigimon remained on the battle area as a standalone Lv.2 egg with no DP.
- **Steps to Reproduce**: Create a digimon stack, delete it via the forced attack mechanic.
- **Evidence**: Game cb0051ce internal state: BT21-001 on field (sources=1), BT24-008 in trash. Repeated on turn 5 with Lamiamon -> BT23-005 Elizamon leaked.
- **Rules Reference**: "When a Digimon leaves the field, only the top card moves; all digivolution and link cards are trashed" (4-2-8)

### Issue 5: Gotsumon trait-based evo cost missing from card database
- **Card(s)**: BT23-048 (Gotsumon)
- **Severity**: medium
- **Category**: digivolution
- **Expected**: BT23-048 has evo requirement "Lv.2 w/[CS] trait: Cost 0" per the official card data API.
- **Actual**: Database only has "Black Lv.2: Cost 1". Digivolving Gotsumon onto Tsumemon (which has [CS] trait) charged 1 memory instead of 0.
- **Evidence**: Game cb0051ce, "Player 2 pays 1 memory to digivolve" for Gotsumon onto Tsumemon.
- **Rules Reference**: Card text takes priority (1-3-1)

### Issue 6: Hudiemon trait-based evo cost missing from card database
- **Card(s)**: BT23-101 (Hudiemon)
- **Severity**: medium
- **Category**: digivolution
- **Expected**: BT23-101 has evo requirement "Lv.3 w/[CS] trait: Cost 4" per the official card data API. Also has alt evo "While controlling 4+ [Hudie] trait Tamers and [Erika Mishima]: Cost 3".
- **Actual**: Database only has "Green Lv.3: Cost 5". Digivolving Hudiemon charged 5 memory instead of 4.
- **Evidence**: Game fcddce11, "Player 1 pays 5 memory to digivolve" for Hudiemon onto Gotsumon.

### Issue 7: Chitose Imai OnTapped triggers for non-Hudie Digimon
- **Card(s)**: BT23-081 (Chitose Imai)
- **Severity**: medium
- **Category**: effect
- **Expected**: Chitose's effect "[All Turns] When any of your [Hudie] trait Digimon suspend" should only trigger when a Digimon with [Hudie] trait suspends.
- **Actual**: The effect triggered when P2's Dimetromon (P-189, Reptile trait, NOT Hudie) suspended during an attack in Game 2.
- **Evidence**: Game fcddce11, log shows Chitose effect firing when Dimetromon attacks despite lacking [Hudie] trait.
- **Rules Reference**: Trigger conditions must be met (15-5)

### Issue 8: Gotsumon reveal only allows 1 selection instead of 2
- **Card(s)**: BT23-048 (Gotsumon)
- **Severity**: medium
- **Category**: effect
- **Expected**: On Play effect says "Add 1 card with the [Hudie] trait AND 1 Tamer card or Option card with the [CS] trait among them to the hand" - this should allow selecting 2 different cards.
- **Actual**: Only 1 card selection was offered before the reveal resolved.
- **Evidence**: Game fcddce11, Gotsumon reveal showed Hudiemon, Kuremi, Palmon but only allowed 1 pick.

### Issue 9: OnLoseSecurity digivolve shows Play actions
- **Card(s)**: BT21-001 (Gigimon)
- **Severity**: low
- **Category**: ui
- **Expected**: Gigimon inherited effect "1 of your Digimon may digivolve into a [Reptile]/[Dragonkin] card" should show Digivolve actions.
- **Actual**: The selection shows "Play X from hand" actions instead of "Digivolve X onto Y" actions.
- **Evidence**: Game fcddce11, after security loss, actions showed "Play Owen Dreadnought from hand" etc.

### Issue 10: Owen Dreadnought tamer displays 'piercing' keyword
- **Card(s)**: BT21-081 (Owen Dreadnought)
- **Severity**: low
- **Category**: ui
- **Expected**: Owen is a Tamer. The 'piercing' keyword is from his End-of-Turn grant effect, not an innate keyword. Tamers should not display combat keywords.
- **Actual**: Owen shows `kw:['piercing']` in battle area display.
- **Evidence**: Game fcddce11, P2 battle area shows Owen with piercing keyword.

### Issue 11: Lamiamon [When Digivolving] condition may not be checked
- **Card(s)**: BT24-016 (Lamiamon)
- **Severity**: low
- **Category**: effect
- **Expected**: Effect says "If you have another Digimon or Tamer with the [Reptile] or [Dragonkin] trait on the field" - with only Lamiamon on the field and no other qualifying cards, the condition should fail.
- **Actual**: The effect log shows "Add To Security, Destroy Security" fired, and BT23-027 appeared in P2's trash (possibly from security). Inconclusive whether condition was properly checked.
- **Evidence**: Game cb0051ce, Lamiamon WhenDigivolving fired with no other Reptile/Dragonkin on field.

## Cards Tested Successfully
- BT21-001 (Gigimon): Inherited evo cost reduction triggers on security loss. Breeding area suppresses effects correctly.
- BT23-005 (Elizamon): Inherited evo cost reduction (-1) verified across multiple digivolutions. Correct in both games.
- BT24-008 (Elizamon): Evo cost 0 from Lv.2 correct. Draw 2 not tested (breeding suppression correct).
- BT24-012 (Dimetromon): Blocker keyword displayed. Evo cost with reduction verified.
- BT24-016 (Lamiamon): Evo cost verified. WhenDigivolving triggers (condition check uncertain).
- BT22-043 (Terriermon): Play cost correct. On Play CS tamer play not tested (no eligible target).
- BT23-020 (Seadramon): Alliance keyword displayed and functional as alliance partner.
- BT23-081 (Chitose Imai): On Play free CS/Hudie play works. Tamer cost correct (4 memory).
- BT23-027 (Angemon): Free play via effect works. Barrier keyword displayed.
- BT23-101 (Hudiemon): Alliance keyword functional (DP addition + Security A. +1). WhenDigivolving free CS play works.
- P-035 (Red Memory Boost!): Delay mechanic works (trash from battle area, gain 2 memory). Main effect reveal triggers.
- P-103 (Offense Training): Delay placement works. Main effect reveal triggers.
- BT21-081 (Owen Dreadnought): Start-of-Main-Phase +1 memory works when opponent has Digimon. Tamer play cost correct (3).
- BT24-082 (Owen Dreadnought): Tamer timing verified via retest report.
- P-189 (Dimetromon): Progress keyword blocks security effects during attack. DP and combat correct.

## Areas Not Covered
- BT24-017 Medusamon (Raid keyword, token generation)
- BT24-018 Styracomon (Armor Purge, alt-digivolve onto tamer)
- BT21-029 Medusamon (token effects)
- BT22-099 Kuremi Detective Agency (full reveal logic)
- BT16-025 Paildramon (suspend all)
- BT16-082 Ukkomon (reveal logic in non-security context)
- BT22-044 Palmon (inherited memory gain)
- BT22-089 Mirei Mikagura (security effect)
- BT23-090 Keisuke Amasawa (end-of-turn targeting)
- Alliance attack with When Attacking return-tamer effect (Hudiemon)
- Counter timing
- Blocker timing (Ankylomon blocking attacks)
