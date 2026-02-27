# Audit Report: BT11

- **Total cards**: 112
- **Cards audited**: 102
- **Cards with issues**: 22
- **Cards below threshold**: 24
- **Cards missing script**: 10
- **Average score**: 0.7759

## Worst-Scoring Cards

| Card ID | Name | Score | Forward | Reverse | Timing |
|---------|------|-------|---------|---------|--------|
| BT11-013 | Garudamon | 0.00 | 0 | 0 | 0 |
| BT11-021 | SnowGoblimon | 0.00 | 0 | 0 | 0 |
| BT11-026 | Hyogamon | 0.00 | 0 | 0 | 0 |
| BT11-035 | ClearAgumon | 0.00 | 0 | 0 | 0 |
| BT11-037 | Kotemon | 0.00 | 0 | 0 | 0 |
| BT11-048 | ModokiBetamon | 0.00 | 0 | 0 | 0 |
| BT11-051 | Ogremon | 0.00 | 0 | 0 | 0 |
| BT11-053 | Digitamamon | 0.00 | 0 | 0 | 0 |
| BT11-066 | Tekkamon | 0.00 | 0 | 0 | 0 |
| BT11-075 | DoKunemon | 0.00 | 0 | 0 | 0 |
| BT11-034 | Cutemon | 0.20 | 0 | 0 | 0 |
| BT11-060 | Monmon | 0.20 | 0 | 0 | 0 |
| BT11-067 | Gigadramon | 0.20 | 0 | 0 | 0 |
| BT11-078 | Soulmon | 0.20 | 0 | 0 | 0 |
| BT11-080 | Devimon | 0.20 | 0 | 0 | 0 |
| BT11-109 | Astral Snatcher | 0.20 | 0 | 0 | 0 |
| BT11-010 | Grizzlymon | 0.40 | 0 | 0 | 0 |
| BT11-014 | GrapLeomon | 0.40 | 0 | 0 | 0 |
| BT11-064 | Greymon (X Antibody) | 0.40 | 0 | 0 | 1 |
| BT11-100 | Megalo Spark | 0.47 | 0 | 0 | 0 |

## Top Forward Issues (API mentions X, script missing)

- **digivolve_into**: 4 cards
- **memory_gain**: 3 cards
- **bounce**: 2 cards
- **dp_modification**: 1 cards
- **save**: 1 cards
- **piercing**: 1 cards
- **once_per_turn**: 1 cards
- **de_digivolve**: 1 cards
- **destruction_immunity**: 1 cards
- **retaliation**: 1 cards
- **play**: 1 cards
- **mill**: 1 cards

## Top Reverse Issues (script claims X, API doesn't mention)

- **_is_save**: 4 cards
- **_is_material_save**: 4 cards
- **_is_decoy**: 1 cards

## Timing Issues

- **has inherited effect text but no is_inherited_effect flag**: 2 cards
- **timing 'When Digivolving' -> is_when_digivolving not found**: 1 cards
- **[Once Per Turn] in API but no set_max_count_per_turn**: 1 cards
