# Archetype QA: TS Olympos
Date: 2026-03-11
Total cards: 31

## Summary
- PASS: 0
- IMPLEMENTED: 31
- QA-FAIL: 0
- BLOCKED: 0

## Results by Card

### Digi-Eggs (Lv2)
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-002 | Bukamon | IMPLEMENTED | 3 fixes: missing memory cost, missing Blue+TS check, wrong target selection |
| BT24-004 | Wanyamon | IMPLEMENTED | 1 fix: missing owner/Iliad trait filter on condition |

### Rookies (Lv3)
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-019 | Kamemon | IMPLEMENTED | 1 fix: cost reduction applied to any card instead of Blue+TS only |
| BT24-020 | Gomamon | IMPLEMENTED | 3 fixes: bogus trash stub, wrong unsuspend condition, missing hand size gate |
| BT24-031 | Elecmon | IMPLEMENTED | 4 fixes: wrong timing, incorrect security gate, missing optional choice, simplified alt-digi |
| BT24-043 | Tapirmon | IMPLEMENTED | 4 fixes: missing Animal/Sovereign traits, Sea Animal exclusion, bogus stub, wrong timing |
| P-196 | Gomamon | IMPLEMENTED | 5 fixes: missing alt-digi level, memory check, cost override, timing, hand size gate |

### Champions (Lv4)
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-022 | Ikkakumon | IMPLEMENTED | 2 fixes: entirely wrong On Play effect, inherited draw condition |
| BT24-024 | Submarimon | IMPLEMENTED | 5 fixes: wrong On Play removed, timing/filter/cost corrections, alt-digi |
| BT24-025 | Shellmon | IMPLEMENTED | 3 fixes: unsuspend trigger condition, target filter, ignore_requirements |
| BT24-034 | Aegiomon | IMPLEMENTED | 4 fixes: entirely wrong effect replaced, missing OnMove, alt-digi, inherited Barrier |
| BT24-046 | Garurumon | IMPLEMENTED | 4 fixes: alt-digi missing Gabumon name, target filter, inherited timing |
| BT24-058 | Blimpmon | IMPLEMENTED | 3 fixes: stub reveal replaced, deck bottom handling, trait filtering |

### Ultimates (Lv5)
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-028 | Divermon | IMPLEMENTED | 3 fixes: alt-digi missing Aqua, On Play/WD rewritten with tuck mechanic, unsuspend condition |
| BT24-029 | Whamon | IMPLEMENTED | 3 fixes: null guards on tuck callback, missing Digimon/Tamer target filter |
| BT24-059 | Sharkmon | IMPLEMENTED | 3 fixes: alt-digi missing Aqua, On Deletion rewritten, ESS rewritten |
| BT24-063 | Locomon | IMPLEMENTED | 1 fix: On Play/WD rewritten from hand play to deck reveal |

### Megas (Lv6) — Complex
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-030 | Neptunemon | IMPLEMENTED | 4 fixes: alt-digi missing Aqua/Sea Animal, wrong digi-card count, self-suspend guard, protection effect rewritten |
| BT24-040 | Venusmon | IMPLEMENTED | 5 fixes: trash all evo cards not 1, wrong target, wrong modifier, protection rewritten, missing memory gain |
| BT24-041 | Minervamon | IMPLEMENTED | 7 fixes: de-digivolve count, alt-digi TS, BeforePayCost filter, missing deletion prevention, Blocker/Reboot turn, Reboot condition, alias cleanup |
| BT24-051 | Merukimon | IMPLEMENTED | 6 fixes: On Play/WD rewritten, attack timing, missing leave-protection, Rush scope, Piercing gap, unsuspend timing |

### Tamers
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-083 | Hiroko Sagisaka | IMPLEMENTED | 4 fixes: missing memory check, missing deck-return cost, missing DP filter, wrong reveal |
| BT24-085 | Dan & Kanan Yuki | IMPLEMENTED | 3 fixes: unconditional memory gain, wrong effect1, missing digivolve trigger |
| BT24-088 | Asuna Shiroki | IMPLEMENTED | 4 fixes: missing memory check, missing deck-return cost, wrong filter, security blocked |
| BT24-102 | Homeros | IMPLEMENTED | 1 fix: DP buff to all Digimon instead of TS-only |

### Options
| Card | Name | Verdict | Fixes |
|------|------|---------|-------|
| BT24-090 | Abyss Sanctuary | IMPLEMENTED | 7 fixes: wrong Blocker→DP boost, conditions, filters, stubs removed, color ignore |
| BT24-091 | Tidal Stream | IMPLEMENTED | 5 fixes: bounce hand→deck, mandatory unsuspend, Link timing, security, color ignore |
| BT24-094 | Central Town | IMPLEMENTED | 5 fixes: Alliance condition, cost reduction, security filter, color ignore, DP condition |
| BT24-095 | Sonic Shot | IMPLEMENTED | 6 fixes: Link timing, target filter, keyword target, factory keyword, color ignore, Link attach |
| BT24-100 | In-Between Theater | IMPLEMENTED | 6 fixes: trash pop removed, Delay added, security added, battle area placement, color ignore |
| LM-028 | Blue Scramble | IMPLEMENTED | 6 fixes: wrong timing, non-API cost reduction, Delay timing, wrong effect, security, DP check |

## Engine Gaps Found
- **Protect-other-permanent via suspend-self** (BT24-041, BT24-051): `WhenRemoveField` fires post-deletion and cannot abort deletion for a different permanent. Agents tagged this as a gap but implemented best-effort versions.
- **Piercing keyword grant** (BT24-051): `AddSkillClass` not available in engine for granting Piercing to another Digimon. Tagged as partial gap.
- **IMMUNE_FROM_DP_MINUS modifier** (BT24-040): Used with `expiry='end_of_opponent_turn'` — verify engine supports this expiry type.

## Implementation Notes
- Every single script (31/31) had issues ranging from minor (wrong timing enum) to severe (completely wrong effects)
- Common patterns found across multiple cards:
  - Agents incorrectly changed `OnUseAttack` to `OnAllyAttack` for [When Attacking] effects — reverted in post-QA review. `OnUseAttack` is the correct engine timing for [When Attacking].
  - Missing `card_source is not card` guard on BeforePayCost
  - Bogus trash manipulation stubs in reveal-and-select flows
  - Alt-digi conditions missing trait alternatives (Aqua, Sea Animal, specific names)
  - "Ignore color requirements" conditions returning True unconditionally
  - Missing memory ≤ 4 gates on Tamer start-of-turn effects
