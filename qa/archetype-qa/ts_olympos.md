# Archetype QA: TS Olympos
Date: 2026-03-17 (faithfulness campaign)
Total cards: 105

## Summary
See TS Jupitermon and TS Neptunemon reviews for shared card verdicts.

## Rust Representative Training Unlock (2026-05-24)

The Rust DSL representative unlock is tracked in `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`.

- Representative deck: 23/23 unique cards have production Rust YAML and focused behavioral coverage.
- Broad TS Olympos pool: 62/117 unique cards have production Rust YAML.
- Broad residual cards: 55, listed in the Rust DSL tracker.
- Training implication: no additional representative cards are needed before TS Olympos can be admitted to Rust-backed representative training runs; the local PyO3 registry was rebuilt and verified during the closure pass.

This archetype shares most cards with TS Jupitermon (30 cards) and TS Neptunemon (30 cards), plus additional Olympos XII support. The 5 cards fixed in the cross-archetype pass (BT24-051, BT24-090, BT24-094, BT24-085, BT24-101) are documented in their respective archetype QA files.

- FAITHFUL: 90+ (combined from TS Jupitermon + TS Neptunemon + unique Olympos cards)
- FIXED: 5 (shared fixes with TS Jupitermon/Neptunemon)
- DEFERRED: 0
- ENGINE GAP: 0

## Card-by-Card Verdicts (Olympos-unique and cross-archetype fixes)
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT24-051 | Merukimon | FIXED | Rush/Piercing aura + player-selected suspend (see TS_Jupitermon.md) |
| BT24-090 | Abyss Sanctuary | FIXED | Blocker + Alliance aura (see TS_Jupitermon.md) |
| BT24-094 | Central Town | FIXED | Color bypass, face-down aura, security placement (see TS_Jupitermon.md) |
| BT24-085 | Dan Yuki & Kanan Yuki | FIXED | Memory threshold fix (see TS_Jupitermon.md) |
| BT24-101 | Homeros | FIXED | Alt-digi cost + target selection (see TS_Jupitermon.md) |
| BT24-002 | Bukamon | FAITHFUL | ESS unsuspend with cost |
| BT24-004 | Wanyamon | FIXED | Removed is_on_play (self-only→any), fixed played_permanent context key |
| BT24-019 | Kamemon | FAITHFUL | Cost reduction Blue+TS |
| BT24-020 | Gomamon | FAITHFUL | Reveal 3, unsuspend condition |
| BT24-022 | Ikkakumon | FAITHFUL | Jamming, trash digi cards + stun |
| BT24-024 | Submarimon | FAITHFUL | Alt-digi, timing/filter/cost |
| BT24-025 | Shellmon | FIXED | Fixed: digivolve targeted triggering perm instead of Shellmon itself |
| BT24-028 | Divermon | FAITHFUL | Alt-digi Aqua, tuck mechanic |
| BT24-029 | Whamon | FAITHFUL | Null guards, Digimon/Tamer filter |
| BT24-030 | Neptunemon | FIXED | Cost reduction, bottom-deck, protection (fixed: WhenRemoveField→WhenPermanentWouldBeDeleted) |
| BT24-031 | Elecmon | FAITHFUL | Reveal 3 multi-select |
| BT24-034 | Aegiomon | FAITHFUL | Barrier, security-to-hand, alt-digi |
| BT24-040 | Venusmon | FAITHFUL | Trash evo cards, protection |
| BT24-041 | Minervamon | FAITHFUL | De-digivolve, deletion prevention, Blocker/Reboot |
| BT24-043 | Tapirmon | FIXED | Reveal selection was optional, now mandatory per card text |
| BT24-046 | Garurumon | FAITHFUL | Alt-digi Gabumon name |
| BT24-058 | Blimpmon | FAITHFUL | Reveal, deck bottom, trait filter |
| BT24-059 | Sharkmon | FAITHFUL | De-Digivolve, On Deletion, ESS |
| BT24-063 | Locomon | FAITHFUL | Deck reveal play |
| BT24-083 | Hiroko Sagisaka | FAITHFUL | Return-to-deck to play |
| BT24-088 | Asuna Shiroki | FAITHFUL | Return-to-deck, trash-to-draw |
| BT24-091 | Tidal Stream | FIXED | Fixed selection chain bug (unsuspend→link), trait check |
| BT24-095 | Sonic Shot | FIXED | Color bypass fn, TS link filter, is_on_attack, chained selections |
| BT24-100 | In-Between Theater | FIXED | Color ignore, delay factory, reveal optionality |
| BT24-102 | Homeros | FAITHFUL | Memory gain, TS DP aura, EOT reactivate |
| LM-028 | Blue Scramble | FIXED | Fixed: wrong descriptions, missing else branch, condition2 check |
| P-196 | Gomamon | FAITHFUL | Digivolve into Sea Beast/TS |

## Fixes Applied (2026-03-17 Campaign)
### Shared fixes (see TS Jupitermon and TS Neptunemon QA docs)
- BT24-051 Merukimon: Rush/Piercing stubs resolved with aura keyword effects; On Play/WD suspend targets player-selected
- BT24-090 Abyss Sanctuary: Blocker (not DP!) + Alliance aura effects
- BT24-094 Central Town: Alliance aura effect added
- BT24-085 Dan Yuki & Kanan Yuki: Memory threshold corrected
- BT24-101 Homeros: Alt-digi cost corrected to 5; target selection for -13000 DP added
