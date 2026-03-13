# Archetype QA: Royal Knights
Date: 2026-03-13
Total cards: 46

## Summary
- PASS: 2 (existing scripts, verified correct)
- IMPLEMENTED: 42 (scripts fixed/completed)
- PARTIAL: 1 (BT19-072 — attack redirect stubbed)
- BLOCKED: 0
- Engine gap stubs: 4 cards affected

## Smoke Test
- **50/50 games completed successfully** (RK mirror match, greedy policy, 500 step cap)

## Batch Results

### Batch 1: King Drasil + Omekamon (5 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-007 | King Drasil_7D6 | IMPLEMENTED | Added BeforePayCost leak guard |
| BT23-072 | King Drasil_7D6 | IMPLEMENTED | [Hand][Main] stub filled with complete process |
| BT13-093 | Omekamon | PASS | No changes needed |
| BT20-083 | Omekamon | IMPLEMENTED | **Name collision fix**: `'Omnimon'` → `'Omnimon (X Antibody)'` |
| EX11-053 | Omekamon | IMPLEMENTED | On Deletion rewritten: hand+King Drasil search, place-under callback |

### Batch 2: Omnimon (3 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-112 | Omnimon | IMPLEMENTED | Royal Knight from breeding play logic implemented (was stub) |
| BT20-102 | Omnimon (X Antibody) | IMPLEMENTED | X Antibody trait check fixed (was checking names, not traits) |
| BT20-100 | The Last Guardian | IMPLEMENTED | WhenRemoveField prevention + Delay guard implemented |

### Batch 3: Cool Boy + Tamers (5 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT20-091 | Cool Boy | IMPLEMENTED | Fixed digivolve trigger timing |
| EX11-071 | Cool Boy | IMPLEMENTED | 3 bugs: spurious trash pop, free→manual_reduction=2, missing tamer bounce |
| P-206 | Digital Gate Open | PASS | No changes needed |
| RB1-035 | Hokuto Amanokawa | IMPLEMENTED | OnStartMainPhase → OnStartTurn |
| BT8-094 | Digimon Emperor | IMPLEMENTED | OnRemovedField → OnDestroyedAnyone |

### Batch 4: Gallantmon variants (5 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-111 | Gallantmon | IMPLEMENTED | Cost reduction gated on minimum trash count |
| BT23-014 | Gallantmon | IMPLEMENTED | Play-from-trash restriction via CANNOT_PLAY_CARD modifier; DP calc fixed |
| P-186 | Gallantmon | IMPLEMENTED | Delete targets both fields; alt-digi added |
| BT17-018 | Gallantmon: Crimson Mode | IMPLEMENTED | Alt-digi added, When Attacking timing fixed, security trash fixed |
| EX4-065 | Trident Gaia | IMPLEMENTED | New file created (was missing) |

### Batch 5: Gallantmon X / MedievalGallantmon (3 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| EX8-073 | Gallantmon (X Antibody) | IMPLEMENTED | 3 critical: source check, delete-or-trash conditional, immunity as continuous |
| EX8-074 | MedievalGallantmon | IMPLEMENTED | BeforePayCost process uses player selection |
| BT8-097 | Crimson Blaze | IMPLEMENTED | Modifier arg order fixed, leak guard added |

### Batch 6: Jesmon + Gankoomon (4 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT20-017 | Jesmon | IMPLEMENTED | Force attack stub filled with FORCE_ATTACK modifier |
| BT20-021 | Jesmon GX | IMPLEMENTED | Missing process callbacks implemented, unsuspend+security trash fixed |
| BT23-057 | Gankoomon | IMPLEMENTED | Fixed permanent sourcing and deck placement direction |
| BT13-019 | Gankoomon | IMPLEMENTED | Royal Knight from breeding branch fully implemented |

### Batch 7: Sistermon (3 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT6-082 | Sistermon Blanc | IMPLEMENTED | Aura Blocker grant with is_declarative=True |
| ST12-12 | Sistermon Blanc | IMPLEMENTED | trash_from_hand, Decoy aura fixed |
| BT23-077 | Sistermon Ciel | IMPLEMENTED | Removed incorrect inherited effects, de-digivolve filter relaxed |

### Batch 8: Alphamon (4 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-075 | Alphamon | IMPLEMENTED | Complete rework: trash-to-digi-stack, blanket CANNOT_ATTACK, WhenRemoveField |
| BT20-056 | Alphamon | IMPLEMENTED | DP mod via register_modifier, breeding digivolve with trash fallback |
| BT20-060 | Alphamon: Ouryuken | IMPLEMENTED | DNA check implemented, blast DNA names added |
| BT9-103 | Kongou | IMPLEMENTED | grant_keyword → register_modifier loop |

### Batch 9: Dynasmon / LordKnightmon / Kentaurosmon / Examon (5 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-087 | Dynasmon | IMPLEMENTED | Self-trigger bug fixed |
| BT19-072 | LordKnightmon | PARTIAL | Timing fixed; attack redirect remains stubbed (engine gap) |
| BT22-041 | Kentaurosmon | IMPLEMENTED | Cost reduction, security placement, suspend trigger all fixed |
| BT23-035 | Dynasmon | IMPLEMENTED | Security trash, DP mod, trigger conditions fixed |
| BT23-047 | Examon | IMPLEMENTED | Security trigger condition fixed; force_attack/aura stubs remain |

### Batch 10: Leopardmon / Magnamon / Craniamon / Options (6 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT22-052 | Leopardmon | IMPLEMENTED | DP filter, Blocker grant to Lv3+, self-exclusion on WhenRemoveField |
| BT23-054 | Magnamon | IMPLEMENTED | Modifier call verified, empty-target guard |
| BT23-058 | Craniamon | IMPLEMENTED | WhenRemoveField ownership check, correct suspend target |
| BT13-040 | Magnamon | IMPLEMENTED | Veemon filter, digi-source play path, self-trigger |
| BT13-110 | Royal Knights of the Purge | IMPLEMENTED | Delay digi-source iteration fixed, Rush via modifier |
| BT15-092 | Revelation of Light | IMPLEMENTED | Major rewrite: security search/play, shuffle, Kari Kamiya condition |

### Batch 11: Remaining Tamers (3 cards)
| Card ID | Name | Verdict | Changes |
|---------|------|---------|---------|
| BT13-102 | Keenan Crier | IMPLEMENTED | Removed incorrect is_on_play, fixed permanent sourcing |
| BT15-084 | Kari Kamiya | IMPLEMENTED | Security A -1 process implemented, suspend-as-cost fixed |
| BT21-086 | Marcus Damon | IMPLEMENTED | Piercing grant with turn expiry, callback chaining |

## Engine Gaps Affecting These Cards

| Gap | Cards Affected | Status |
|-----|---------------|--------|
| Effect-Based Play Lock (#6) | BT23-014, BT8-097 | Best-effort with CANNOT_PLAY_CARD modifier |
| Aura-Style CANNOT_UNSUSPEND (#7) | BT23-047 | Applied to current field only |
| Attack Target Redirect | BT19-072 | Stubbed — SwitchDefender not supported |
| Suppress On Play Effects | BT13-110 | Stubbed — no On Play suppression mechanism |
| Force Optional Attack | BT20-017, BT23-047 | BT20-017 uses FORCE_ATTACK modifier; BT23-047 stubbed |
| Decoy Color Restriction | ST12-12 | Engine auto-activates Decoy without color filter |
| Also Treated As (Name Aliasing) | BT23-077 | Descriptive-tagged |

## Key Bug Fixes
1. **Name collision (BT20-083)**: `'Omnimon'` substring matched both Omnimon variants → fixed to exact `'Omnimon (X Antibody)'`
2. **Missing process callbacks (BT20-021)**: 3 effects had no process → fully implemented
3. **Inverted conditional (EX8-073)**: Delete-or-trash logic always trashed → fixed to try delete first
4. **Wrong immunity type (EX8-073)**: One-shot modifier → conditional continuous CANNOT_BE_AFFECTED
5. **Spurious trash pop (EX11-071)**: Removed pre-reveal trash manipulation bug
