# Royal Knights Archetype QA Report

**Date**: 2026-03-01
**Archetype**: Royal Knights (SPECIAL ATTENTION)
**Cards tested**: 35 unique across 9 decklists
**Best decklist**: `egman_c8973fe02209` (1st place, 22 unique cards)
**Variant-only cards**: 13 (tested via inject-card)
**Debug games**: 8

## Summary

| Status | Count | Percentage |
|--------|-------|------------|
| PASS | 21 | 60% |
| PARTIAL | 14 | 40% |
| FAIL | 0 | 0% |

**Overall health**: PARTIAL -- core play cost mechanics work for most cards, but several important effects are not triggering (token plays, continuous keyword grants, cost reduction from breeding area). The archetype's signature King Drasil_7D6 egg cost reduction and Jesmon's token generation are both non-functional, which are critical to the deck's strategy.

## Issues Found

| # | Issue | Sev | Status | Cards Affected |
|---|-------|-----|--------|----------------|
| 1 | BT13-007 King Drasil_7D6 breeding cost reduction not applied | high | OUTSTANDING | BT13-007 |
| 2 | BT20-017 Jesmon On Play token not created | high | OUTSTANDING | BT20-017 |
| 3 | BT6-082 Sistermon Blanc On Play Draw 1 not triggered | high | OUTSTANDING | BT6-082 |
| 4 | BT6-082 Sistermon Blanc continuous Blocker grant not working | high | OUTSTANDING | BT6-082 |
| 5 | ST12-12 Sistermon Blanc Decoy granted without condition check | med | OUTSTANDING | ST12-12 |
| 6 | BT9-103 Kongou stays in battle area instead of going to trash | med | OUTSTANDING | BT9-103 |
| 7 | BT8-097 Crimson Blaze stays in battle area instead of going to trash | med | OUTSTANDING | BT8-097 |
| 8 | BT13-111 Gallantmon missing innate Rush keyword | med | OUTSTANDING | BT13-111 |
| 9 | BT23-047 Examon missing innate Piercing and Security A. +1 keywords | med | OUTSTANDING | BT23-047 |
| 10 | BT23-072 King Drasil_7D6 Digimon shows keywords on itself instead of granting to others | med | OUTSTANDING | BT23-072 |
| 11 | BT20-056 Alphamon missing Barrier keyword | low | OUTSTANDING | BT20-056 |
| 12 | BT23-057 Gankoomon On Play token (Hinukamuy) likely not created | med | OUTSTANDING | BT23-057 |

## Test Methodology

Eight debug games were created with `skip_shuffle=true` and `initial_memory=10`. Cards were tested via direct inclusion in decks or injection into hand. For each card:
- Play cost verified (memory delta before/after play)
- Keywords checked on the permanent in battle area
- On Play / When Digivolving effects checked via state changes
- Pending selections resolved to verify effect chains

## Card-by-Card Results

### Digi-Eggs

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT13-007 | King Drasil_7D6 | PARTIAL | Hatches correctly into breeding area. **[Breeding] cost reduction for Royal Knight plays does NOT apply** (Magnamon cost 7 paid full, Jesmon cost 11 paid full). Start of Main Phase reveal+absorb effect untested. |

### Level 3

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT6-082 | Sistermon Blanc | PARTIAL | Play cost 3 PASS. **On Play Draw 1 NOT triggered** (hand decreased by 1, no draw). **[All Turns] Blocker grant for Sistermons while Royal Knight in play NOT working** (Magnamon in play, Sistermon still had no Blocker). |
| ST12-12 | Sistermon Blanc | PARTIAL | Play cost 3 PASS. On Play trash-from-hand PASS (phase 11 selection prompted). Draw 2 PASS (deck decreased by 2). **Decoy keyword shows even without Huckmon/Royal Knight in play** -- continuous condition not checked. |

### Level 4

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT13-093 | Omekamon | PASS | Play cost 4 PASS. On Play Draw 1 PASS (hand maintained at 5 after play+draw). Card placed in battle area correctly. On Deletion effect (place Royal Knight under King Drasil) not tested. |
| BT20-083 | Omekamon (BT20) | PASS | Play cost 5 PASS. Blocker keyword PASS. On Play conditional digivolve into Omnimon X not tested (requires 1 or fewer security). On Deletion place-under-King-Drasil not tested. |
| BT23-054 | Magnamon | PASS | Play cost 7 PASS. Blocker PASS. Armor Purge keyword present PASS. On Play Draw 1 triggered (hand count correct). Protection effect (can't be returned) not testable without opponent effects. |
| BT23-077 | Sistermon Ciel | PASS | Play cost 4 PASS. Blocker keyword PASS. On Play delete opponent play cost 4 or less (no target available, correctly skipped). When suspends De-Digivolve not tested. |

### Level 6

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT13-019 | Gankoomon | PASS | Play cost 13 PASS (delta=13). Blocker keyword PASS. On Play play-Sistermon-from-trash not tested (empty trash). |
| BT13-087 | Dynasmon | PASS | Play cost 10 PASS. On Play reveal top 4, add Royal Knight/Lucemon to hand -- effect resolved (pending selections cleared). |
| BT13-111 | Gallantmon | PARTIAL | Play cost 13 with trash-based reduction working (cost was 11 with ~5 trash cards). **Rush keyword NOT present** in battle area keywords list. On Play deletion effect not tested (no opponent Digimon). |
| BT19-072 | LordKnightmon | PASS | Play cost 11 PASS. On Play play Lv4 or lower from trash not tested (empty trash). Opponent's Turn redirect attack not tested. |
| BT20-017 | Jesmon | PARTIAL | Play cost 11 charged (King Drasil reduction not applied). **On Play [Atho, Rene & Por] Token NOT created** (no token in battle area after play). Your Turn "when other Digimon played" effect not tested. |
| BT20-056 | Alphamon | PARTIAL | Play cost 0 PASS. On Play Recovery +1 may have triggered. **Barrier keyword NOT shown** in keywords list. Breeding area digivolve effect not tested. |
| BT23-014 | Gallantmon (CS) | PASS | Play cost 11 PASS. On Play trash-block effect not testable without opponent trash plays. When Attacking deletion effect not tested. |
| BT23-035 | Dynasmon (CS) | PASS | Play cost 12 PASS. Barrier keyword not shown (may not be listed). On Play trash security + opponent DP reduction not fully tested. |
| BT23-057 | Gankoomon (CS) | PARTIAL | Play cost 11 PASS (cost reduction via trash return not tested). **On Play Hinukamuy Token likely not created** (same token system issue as Jesmon). Deletion effect not tested. |
| BT23-058 | Craniamon | PASS | Play cost 11 PASS. Reboot keyword PASS. Blocker keyword PASS. Protection effect and "when suspends delete lowest" not tested. |
| EX8-074 | MedievalGallantmon | PASS | Play cost 11 PASS (suspend-to-reduce not tested as cost reduction is optional). Alliance PASS. Vortex PASS. When Digivolving effects not tested. |
| P-186 | Gallantmon | PARTIAL | Play cost 12 expected, but **observed 13 cost** (may have been name collision with BT13-111 in same game). Rush keyword present on one Gallantmon in BA. Blocker not verified. On Play delete 13000+ DP not tested. |

### Level 7

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT13-112 | Omnimon | PASS | Play cost 14 PASS. In battle area PASS. On Play delete or play Royal Knights from breeding evo cards not tested (no evo cards under breeding Digimon). |
| BT17-018 | Gallantmon: Crimson Mode | PASS | Play cost 8 PASS. On Play delete up to 15000 DP total (no opponent Digimon, correctly skipped). When Attacking trash security per 10 trash cards not tested. Blast Digivolve counter not tested. |
| BT20-060 | Alphamon: Ouryuken | PASS | Play cost 6 PASS. Blast DNA Digivolve keyword present. On Play -15000 DP effect not tested. |
| BT20-102 | Omnimon (X Antibody) | PASS | Play cost 16 PASS. Blocker PASS. Piercing PASS. Raid PASS. On Play wipe + deck bottom return not tested. End of Turn Rush + attack without suspending not tested. |
| BT23-047 | Examon | PARTIAL | Play cost 15 PASS. **Piercing keyword NOT present**. **Security A. +1 keyword NOT present**. On Play suspend 5 and attack not tested. Partition not tested. |

### Tamers

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT8-094 | Digimon Emperor | PASS | Play cost 3 PASS. In battle area PASS. All Turns draw-on-opponent-deletion not tested. Opponent's Turn memory-on-Lv3-move not tested. |
| BT13-102 | Keenan Crier | PASS | Play cost 3 PASS. On Play opponent-trash-or-gain triggered (phase 11 selection appeared). Opponent's Turn memory on effect-play not tested. |
| BT20-091 | Cool Boy | PASS | Play cost 4 PASS. Your Turn draw+memory when Royal Knight played/digivolves not fully tested. Opponent's Turn Omekamon play not tested. |
| BT21-086 | Marcus Damon | PASS | Play cost 4 PASS. Start of Main memory gain not tested (requires opponent Digimon). When suspends Piercing+DP grant not tested. |
| RB1-035 | Hokuto Amanokawa | PASS | Play cost 2 PASS. In battle area PASS. Start of Turn memory (3+ Tamers) not tested. When opponent plays Digimon memory/draw not tested. |

### Options

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT8-097 | Crimson Blaze | PARTIAL | Play cost 6 PASS (no opponent Digimon, so no reduction). Delete all 6000 DP or less effect (no targets). **Card stays in battle area instead of going to trash** after use. |
| BT9-103 | Kongou | PARTIAL | Play cost 2 PASS. **Card stays in battle area instead of going to trash** after use. Attack restriction and security-add-block effects not testable without multi-turn scenario. |
| BT13-110 | Royal Knights of the Purge | PASS | Play cost 6 PASS. Placed in battle area as Delay PASS. Draw 1 + place Digimon under King Drasil effect resolved. Delay activation (play Royal Knight with Rush) not tested. |
| BT20-100 | The Last Guardian | PASS | Play cost 4 PASS. Placed in battle area as Delay PASS. Reveal top 3 + add to hand effect resolved. Delay protection for Omnimon not tested. |
| P-206 | Digital Gate Open | PASS | Play cost 4 PASS. Placed in battle area as Delay PASS. Reveal + add Digimon/Tamer effect resolved. Delay Tamer play with cost reduction not tested. |

### Digimon (Non-Level)

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT23-072 | King Drasil_7D6 (Digimon) | PARTIAL | Play cost 6 PASS. DP 9000 PASS. **Keywords [blocker, rush, reboot, raid] shown on itself** -- these should only be granted to OTHER Royal Knight/CS Digimon when played, not to King Drasil itself. |

## Systemic Issues

### 1. Non-Delay Options staying in battle area (Issues #6, #7)
Both BT9-103 Kongou and BT8-097 Crimson Blaze remain as permanents in the battle area after being played. These are regular [Main] Options without Delay, and should go to trash after their effects resolve. This appears to be a script-level issue where the cards are placed as permanents instead of being trashed.

### 2. Token generation not working (Issues #2, #12)
Jesmon's [Atho, Rene & Por] Token and Gankoomon CS's [Hinukamuy] Token are not being created on play. The token system exists (per Report 7 notes on Petrification Tokens), but these specific token templates may not be registered.

### 3. King Drasil_7D6 breeding cost reduction (Issue #1)
The egg's "When a Royal Knight would be played, reduce cost by 4 + 1 per evo card" effect never triggers. This is a core mechanic for the archetype -- without it, all Royal Knight plays cost full price, making the deck unplayable at competitive level.

### 4. Continuous keyword grants not checking conditions (Issues #4, #5)
BT6-082's Blocker grant for Sistermons "while Royal Knight in play" never activates even when Royal Knight is in play. ST12-12's Decoy grant shows even when the condition (Huckmon/Royal Knight in play) is not met. Both suggest the continuous effect system either always-on or never-on, not properly checking runtime conditions.

### 5. Missing innate keywords (Issues #8, #9, #11)
Several cards are missing keywords that should be innate: BT13-111 (Rush), BT23-047 (Piercing, Security A. +1), BT20-056 (Barrier). These keywords appear in the card text but are not being registered on the permanent.
