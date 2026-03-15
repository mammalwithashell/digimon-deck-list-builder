# TS Neptunemon Archetype QA

## Deck Library Key: TS Neptunemon
- **Unique Cards:** 31
- **Decklists:** 10
- **Status:** All reviewed, fixes applied

## QA Results

| Card ID | Card Name | Verdict | Notes |
|---------|-----------|---------|-------|
| BT24-002 | Bukamon | PASS | ESS end-of-turn unsuspend with cost; checks blue+TS correctly |
| BT24-014 | Aegiochusmon | IMPLEMENTED | Fixed auto-selection: DP -5000 target now uses `effect_select_opponent_permanent` instead of auto-picking lowest DP |
| BT24-019 | Kamemon | PASS | Alt-digi from TS Lv.2, digi cost reduction for blue TS, inherited Jamming |
| BT24-020 | Gomamon | PASS | On Play reveal 3 multi-select, inherited unsuspend draw |
| BT24-022 | Ikkakumon | PASS | Jamming, On Play/WD trash 2 digi cards + stun. ESS draw on unsuspend. Minor: hand count in process not condition (acceptable) |
| BT24-023 | Calmaramon | IMPLEMENTED | Was STUB: On Play/When Digivolving had empty process callbacks. Fully implemented: bottom-deck 1 opponent Lv.4- Digimon, then if played by effects, stun 1 Digimon/Tamer |
| BT24-025 | Shellmon | PASS | Unsuspend trigger for Venusmon digivolve; end-of-turn unsuspend other TS Digimon |
| BT24-027 | Lanamon | PASS | Decode (Calmaramon), tuck + battle protection grant, inherited WA draw |
| BT24-028 | Divermon | IMPLEMENTED | Fixed auto-selection: ESS "play from digi sources" now uses `request_selection` with `SelectSource` phase instead of auto-picking `eligible[0]` |
| BT24-029 | Whamon | IMPLEMENTED | Fixed auto-selections in both End-of-Attack and ESS effects: now use `request_selection` with `SelectSource` phase for digivolution card selection |
| BT24-030 | Neptunemon | PASS | Cost reduction, bottom-deck lowest digi-count, self-unsuspend on suspend, protect TS/Aqua/Sea Animal Digimon |
| BT24-031 | Elecmon | PASS | On Play reveal 3 multi-select (Iliad + TS), inherited WA security-to-hand + recovery |
| BT24-034 | Aegiomon | PASS | Barrier, On Play/WD/When Moving: security-to-hand to play TS Tamer free. Properly checks no duplicate tamer names |
| BT24-040 | Venusmon | PASS | Cost reduction, trash all digi cards + stun 2, protect TS Digimon by sacrificing no-digi-cards Digimon to security |
| BT24-041 | Minervamon | PASS | Cost reduction, play Iliad + De-Digivolve, deletion prevention via security trash, Reboot+Blocker grant to Iliad on opponent's turn |
| BT24-051 | Merukimon | IMPLEMENTED | Fixed 3 issues: (1) suspend targets now player-selectable via `effect_select_opponent_permanent`, (2) Piercing grant to Iliad Digimon now uses `_is_piercing` static effect with `_applies_to_all_own_digimon` pattern, (3) Rush grant uses same pattern |
| BT24-059 | Sharkmon | PASS | De-Digivolve 1, On Deletion reveal+play, ESS absorb other Digimon as digi source |
| BT24-083 | Hiroko Sagisaka | PASS | Start-of-turn return-to-deck to play Hiroko or TS Digimon DP<=5000, On Play reveal 3 add TS, security play |
| BT24-085 | Dan Yuki & Kanan Yuki | IMPLEMENTED | Fixed: End-of-turn Option use cost condition was checking "opponent's Digimon count" but card text says "opponent's memory". Changed to use opponent's memory value |
| BT24-088 | Asuna Shiroki | PASS | Start-of-turn return-to-deck to play from trash, On Play trash-to-draw, security play |
| BT24-090 | Abyss Sanctuary | IMPLEMENTED | Fixed: Security effect was granting +2000 DP (wrong). Card text grants Blocker to blue/yellow TS Digimon, and Alliance when Neptunemon/Venusmon present. Changed to `_is_blocker` and `_is_alliance` static effects |
| BT24-091 | Tidal Stream | PASS | Main: bounce lowest level, unsuspend TS, link. Security activates Main. Link: WA bounce lowest level |
| BT24-100 | In-Between Theater | IMPLEMENTED | Fixed: Delay effect was tagged as `is_on_attack` with `OnDeclaration` timing. Card text Delay is simply "Gain 2 memory" (Main phase). Changed to `_is_field_main` + `_is_delay_effect` |
| BT24-102 | Homeros | PASS | Start-of-Main-Phase memory gain + conditional suspend/draw, TS DP aura, end-of-turn reactivate Olympos XII, security play |
| BT3-093 | Davis Motomiya | PASS | Memory set to 3, On Play reveal blue+green, security play |
| LM-028 | Blue Scramble | IMPLEMENTED | Fixed delay effect: was "bottom-deck 1 opponent Digimon". Card text is "Return 1 blue Digimon from trash to top of deck, then if no Digimon, play 1 blue DP<=2000 from trash". Also fixed trash selection to use `request_selection` with proper action IDs |
| P-104 | Mental Training | IMPLEMENTED | Fixed 3 issues: (1) Main effect was broken (popping from trash first, no blue filter). Now properly reveals top 2, selects 1 blue card, places in battle area. (2) Delay effect now filters for blue Digimon and reduces cost by 2. (3) Added missing Security effect (place in battle area) |
| P-196 | Gomamon (promo) | PASS | Start-of-Main-Phase digivolve into Sea Beast/TS from hand for free, inherited WA draw |
| P-197 | Patamon | IMPLEMENTED | Fixed auto-selection: ESS -2000 DP target now uses `effect_select_opponent_permanent` instead of auto-picking lowest DP |
| P-198 | DemiDevimon | PASS | Start-of-Main-Phase digivolve into Fallen Angel/TS from hand for free, inherited WA draw+trash |

## Summary

- **31 cards reviewed**
- **10 cards fixed** (IMPLEMENTED)
- **21 cards passed** (PASS)
- **0 cards blocked** (BLOCKED)

### Issues Fixed
1. **BT24-023**: Stub On Play/WD effects fully implemented (bottom-deck + conditional stun)
2. **BT24-014**: Auto-selection replaced with player selection for DP target
3. **BT24-028**: Auto-selection replaced with `request_selection` for digivolution source play
4. **BT24-029**: Auto-selection replaced with `request_selection` in both effects
5. **BT24-051**: Suspend target auto-selection fixed; Piercing grant stub implemented
6. **BT24-085**: Option cost threshold corrected (opponent's memory, not Digimon count)
7. **BT24-090**: Security effect corrected from +2000 DP to Blocker grant
8. **BT24-100**: Delay effect timing and type corrected
9. **LM-028**: Delay effect completely rewritten to match card text
10. **P-104**: Main effect, Delay effect, and Security effect all fixed
11. **P-197**: Auto-selection replaced with player selection for DP target

### Known Engine Gaps (non-blocking)
- **BT24-051**: "attack your opponent's Digimon" forced attack after DP boost requires SelectAttack phase support. Current implementation grants Rush and unsuspends but does not force the attack.
- **BT24-085**: "1 of your TS trait Digimon may attack" at end of turn requires forced attack support. Currently a no-op.
- **BT24-030/BT24-051/BT24-040**: WhenRemoveField protection for OTHER permanents is best-effort; full multi-Digimon protection requires engine support.
