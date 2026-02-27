# Audit Report: BT19

- **Total cards**: 103
- **Cards audited**: 103
- **Cards with issues**: 37
- **Cards below threshold**: 33
- **Cards missing script**: 0
- **Average score**: 0.7315

## Worst-Scoring Cards

| Card ID | Name | Score | Forward | Reverse | Timing |
|---------|------|-------|---------|---------|--------|
| BT19-022 | MailBirdramon | 0.13 | 1 | 0 | 0 |
| BT19-004 | Tokomon | 0.20 | 0 | 0 | 0 |
| BT19-005 | Hopmon | 0.20 | 0 | 0 | 0 |
| BT19-010 | Shoutmon X4 | 0.20 | 0 | 0 | 0 |
| BT19-018 | Swimmon | 0.20 | 0 | 0 | 0 |
| BT19-058 | SkullKnightmon | 0.20 | 0 | 0 | 1 |
| BT19-059 | DeadlyAxemon | 0.20 | 0 | 0 | 0 |
| BT19-045 | FunBeemon | 0.33 | 0 | 0 | 1 |
| BT19-035 | ShootingStarmon | 0.40 | 0 | 0 | 0 |
| BT19-098 | King Device | 0.40 | 0 | 0 | 0 |
| BT19-003 | Viximon | 0.50 | 1 | 0 | 0 |
| BT19-006 | Pagumon | 0.50 | 1 | 0 | 0 |
| BT19-033 | Dorulumon | 0.50 | 2 | 0 | 1 |
| BT19-040 | Sakuyamon | 0.50 | 2 | 0 | 0 |
| BT19-097 | Bonds of True Love | 0.51 | 0 | 0 | 1 |
| BT19-042 | Dynasmon (X Antibody) | 0.55 | 1 | 0 | 0 |
| BT19-020 | Greymon | 0.57 | 1 | 0 | 0 |
| BT19-028 | Xiangpengmon | 0.57 | 1 | 0 | 0 |
| BT19-047 | Ballistamon | 0.57 | 1 | 0 | 0 |
| BT19-048 | ForgeBeemon | 0.57 | 1 | 0 | 2 |

## Top Forward Issues (API mentions X, script missing)

- **digivolve_into**: 6 cards
- **save**: 5 cards
- **mill**: 4 cards
- **piercing**: 3 cards
- **bounce**: 2 cards
- **reveal_top**: 1 cards
- **memory_gain**: 1 cards
- **dp_modification**: 1 cards
- **destruction_immunity**: 1 cards
- **suspend_target**: 1 cards
- **play**: 1 cards
- **token_play**: 1 cards
- **once_per_turn**: 1 cards
- **attack_prevention**: 1 cards

## Top Reverse Issues (script claims X, API doesn't mention)

- **_is_save**: 3 cards
- **_is_material_save**: 3 cards
- **_is_decode**: 2 cards
- **_is_decoy**: 1 cards
- **_is_overclock**: 1 cards

## Timing Issues

- **has inherited effect text but no is_inherited_effect flag**: 5 cards
- **timing 'Security' -> is_security_effect not found**: 5 cards
- **[Once Per Turn] in API but no set_max_count_per_turn**: 1 cards
