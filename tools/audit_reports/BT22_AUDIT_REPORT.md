# Audit Report: BT22

- **Total cards**: 102
- **Cards audited**: 102
- **Cards with issues**: 51
- **Cards below threshold**: 39
- **Cards missing script**: 0
- **Average score**: 0.7315

## Worst-Scoring Cards

| Card ID | Name | Score | Forward | Reverse | Timing |
|---------|------|-------|---------|---------|--------|
| BT22-021 | Shellmon | 0.20 | 0 | 1 | 0 |
| BT22-060 | Datamon | 0.20 | 2 | 0 | 0 |
| BT22-010 | Meramon | 0.40 | 0 | 0 | 0 |
| BT22-040 | Cendrillmon | 0.40 | 1 | 1 | 0 |
| BT22-085 | Rina Shinomiya | 0.40 | 0 | 0 | 0 |
| BT22-099 | Kuremi Detective Agency | 0.47 | 1 | 0 | 1 |
| BT22-097 | Music of the Heart | 0.51 | 0 | 0 | 0 |
| BT22-062 | MetalTyrannomon (X Antibody) | 0.55 | 1 | 0 | 0 |
| BT22-063 | Alphamon | 0.55 | 1 | 0 | 0 |
| BT22-079 | Eater (Species Form) | 0.55 | 1 | 0 | 1 |
| BT22-100 | Cyberspace EDEN | 0.55 | 0 | 0 | 0 |
| BT22-087 | Torajiro Asuka | 0.57 | 1 | 0 | 0 |
| BT22-090 | Rie Kishibe | 0.57 | 1 | 0 | 0 |
| BT22-102 | Sayo | 0.57 | 1 | 0 | 0 |
| BT22-035 | Entermon | 0.58 | 0 | 0 | 1 |
| BT22-075 | Fakemon | 0.58 | 0 | 1 | 1 |
| BT22-018 | Sangomon | 0.60 | 1 | 0 | 0 |
| BT22-045 | WezenGammamon | 0.60 | 1 | 0 | 1 |
| BT22-049 | Vegiemon | 0.60 | 1 | 0 | 1 |
| BT22-054 | Hagurumon | 0.60 | 1 | 0 | 0 |

## Top Forward Issues (API mentions X, script missing)

- **dp_modification**: 8 cards
- **bounce**: 4 cards
- **memory_gain**: 4 cards
- **destruction_immunity**: 3 cards
- **play**: 3 cards
- **once_per_turn**: 2 cards
- **piercing**: 2 cards
- **mill**: 1 cards
- **suspend_target**: 1 cards
- **armor_purge**: 1 cards
- **fortitude**: 1 cards
- **blocker**: 1 cards
- **de_digivolve**: 1 cards
- **delete_opponent**: 1 cards

## Top Reverse Issues (script claims X, API doesn't mention)

- **_is_decode**: 5 cards
- **_is_overclock**: 3 cards
- **_is_blocker**: 1 cards
- **_is_fragment**: 1 cards
- **_is_scapegoat**: 1 cards

## Timing Issues

- **has inherited effect text but no is_inherited_effect flag**: 16 cards
- **[Once Per Turn] in API but no set_max_count_per_turn**: 1 cards
- **timing 'Security' -> is_security_effect not found**: 1 cards
