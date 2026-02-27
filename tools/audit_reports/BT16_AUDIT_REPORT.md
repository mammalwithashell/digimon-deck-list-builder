# Audit Report: BT16

- **Total cards**: 103
- **Cards audited**: 103
- **Cards with issues**: 46
- **Cards below threshold**: 29
- **Cards missing script**: 0
- **Average score**: 0.7637

## Worst-Scoring Cards

| Card ID | Name | Score | Forward | Reverse | Timing |
|---------|------|-------|---------|---------|--------|
| BT16-013 | Valkyrimon | 0.10 | 2 | 0 | 1 |
| BT16-002 | DemiVeemon | 0.20 | 0 | 0 | 0 |
| BT16-003 | Upamon | 0.20 | 0 | 0 | 0 |
| BT16-032 | Sheepmon | 0.20 | 0 | 0 | 0 |
| BT16-038 | Terriermon (X Antibody) | 0.20 | 2 | 0 | 1 |
| BT16-050 | Commandramon | 0.20 | 0 | 0 | 0 |
| BT16-051 | Dorumon | 0.20 | 0 | 0 | 0 |
| BT16-052 | Hagurumon (X Antibody) | 0.20 | 3 | 0 | 1 |
| BT16-009 | Lynxmon | 0.33 | 0 | 0 | 0 |
| BT16-014 | Goldramon (X Antibody) | 0.33 | 0 | 0 | 2 |
| BT16-022 | Mantaraymon | 0.40 | 0 | 0 | 0 |
| BT16-094 | Dragon's Breath | 0.40 | 3 | 0 | 0 |
| BT16-036 | Chaosmon | 0.47 | 1 | 1 | 0 |
| BT16-080 | Shroudmon | 0.50 | 2 | 0 | 2 |
| BT16-082 | Ukkomon | 0.50 | 1 | 0 | 0 |
| BT16-057 | Mekanorimon | 0.55 | 1 | 0 | 0 |
| BT16-021 | Togemogumon | 0.57 | 1 | 0 | 0 |
| BT16-007 | Hawkmon | 0.60 | 1 | 0 | 0 |
| BT16-029 | Agumon | 0.60 | 1 | 0 | 1 |
| BT16-077 | Dinobeemon | 0.60 | 0 | 1 | 0 |

## Top Forward Issues (API mentions X, script missing)

- **dp_modification**: 9 cards
- **bounce**: 6 cards
- **digivolve_into**: 6 cards
- **attack_prevention**: 4 cards
- **mill**: 3 cards
- **blocker**: 2 cards
- **delete_opponent**: 2 cards
- **destruction_immunity**: 2 cards
- **suspend_target**: 2 cards
- **piercing**: 2 cards
- **security_trash**: 2 cards
- **reveal_top**: 2 cards
- **memory_gain**: 1 cards
- **play**: 1 cards
- **de_digivolve**: 1 cards

## Top Reverse Issues (script claims X, API doesn't mention)

- **_is_partition**: 5 cards

## Timing Issues

- **has inherited effect text but no is_inherited_effect flag**: 10 cards
- **timing 'When Digivolving' -> is_when_digivolving not found**: 3 cards
- **timing 'Security' -> is_security_effect not found**: 2 cards
- **timing 'When Attacking' -> is_on_attack not found**: 1 cards
- **[Once Per Turn] in API but no set_max_count_per_turn**: 1 cards
