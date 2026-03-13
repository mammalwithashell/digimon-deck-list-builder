# Archetype QA: ExMaquinamon
Date: 2026-03-13
Total cards: 16

## Summary
- PASS: 3 (BT3-103, EX11-062, EX11-071)
- IMPLEMENTED: 1 (LM-048 — CLEAN)
- QA-FAIL → FIXED: 10
- BLOCKED: 2 (engine gaps)

## Implemented Cards
### LM-048 Chrome Memory Boost!
- Pattern: Memory Boost option (reveal 3, add green/black Digimon, Delay +2 memory, security: place in BA)

## Fixed Cards

### EX11-006
- digi_filter now restricts to cards with [Maquinamon] in text

### EX11-027
- process1: uses effect_reveal_and_select with Maquinamon filter (was unconditional trash pop)
- effect2 (WhenRemoveField): added process callback for linked card placement

### EX11-029
- effect1 (OnMove) + effect2 (When Digivolving): added process callbacks for link Maquinamon
- play_filter: now filters for [Unchained] cards

### EX11-033
- effect1 (OnMove): added process callback
- grant_keyword: now targets selected opponent's Digimon (was targeting self)
- unsuspend: now calls perm.unsuspend() directly (was using selection)

### EX11-036
- grant_keyword: fixed to target opponent's Digimon
- effect6: added process callback
- **BLOCKED: force_attack** — engine gap, stub remains

### EX11-040
- effect1 (On Play) + effect2 (When Digivolving): added process callbacks for link Maquinamon
- play_filter: now filters for [Unchained] cards

### EX11-042
- Delete filter: added cost <= 5 check
- **BLOCKED: redirect_attack** — engine gap, stub remains

### EX11-045
- effect6: added process callback
- Delete targeting: auto-selects lowest-cost opponent Digimon (was player choice)

### EX11-070
- condition2: removed wrong Maquinamon text check on Tamer card
- process2: added DNA digivolve step via effect_dna_digivolve_from_hand
- effect3: changed from CANNOT_BE_SELECTED_BY_EFFECT to CHANGE_DP floor semantics
- **Note:** DP floor is registered but not enforced until engine adds DP_FLOOR support

### EX11-073
- effects 0-3: added process callbacks (Jogress condition/link slot markers)
- effect4 (When Digivolving): implemented linking up to 3 Maquinamon cards
- effect5 (End Turn): fixed to trash own security + return opponent's Digimon to deck bottom per linked card count

### EX6-072
- Security: added level >= 6 Digimon filter + self-return-to-hand
- OptionSkill: replaced play with DNA digivolve via effect_dna_digivolve_from_hand

### P-151
- Fixed execution order: reveal → add to hand → deck bottom → play
- Reveal filter: now checks LIBERATOR trait
- Play filter: now checks LIBERATOR trait + cost <= 3
- Removed bogus trash_cards.pop()

## Blocked Cards (Engine Gaps)
| Card | Gap | Status |
|------|-----|--------|
| EX11-036 | force_attack | Stub remains — no engine API to force attack |
| EX11-042 | redirect_attack | Stub remains — no engine API to redirect attack target |
| EX11-070 | DP floor | Registered as CHANGE_DP but not enforced |

## Smoke Test
- 50/50 mirror games completed successfully
- 25/25 cross-archetype games (vs TS Jupitermon) completed successfully
