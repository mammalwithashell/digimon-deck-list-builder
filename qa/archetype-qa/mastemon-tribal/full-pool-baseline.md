# Mastemon Tribal Full-Pool Baseline

Captured: 2026-05-24 for OpenSpec change `unblock-mastemon-full-pool-rust-dsl`.

## Resolver Baseline

Command:

```powershell
$env:PYTHONIOENCODING='utf-8'; python code/tools/resolve_deck.py "Mastemon (Tribal)" --json
```

- Resolved archetype: `Mastemon (Tribal)`
- Total decklists: 55
- Unique cards: 93
- Best-deck unique cards: 20
- Deck pool path reported by resolver: `qa\archetype-qa\mastemon-tribal\deck_pool.json`

## Rust Coverage Baseline

Rust coverage is measured against production YAML under `code/digimon-engine/cards/<set>/` and focused behavioral tests under `code/digimon-engine/tests/cards_behavioral/<set>/`. The resolver's `script_status` field tracks legacy Python script coverage and is not used as the Rust DSL readiness source.

- Production YAML cards: 36 / 93
- Behavioral-test cards: 37 / 93
- Cards with both production YAML and behavioral tests: 36 / 93
- Best-deck production YAML cards: 20 / 20
- Best-deck behavioral-test cards: 20 / 20
- Missing production YAML cards: 57
- Missing behavioral-test cards: 56

`BT13-106` has behavioral-test-only coverage at this baseline and remains a follow-up until production YAML exists.

## RL Contract Baseline

- `ACTION_SPACE_SIZE`: 2192
- `standard_lite_v2`: tensor size 8410, tensor version 2, feature schema `standard_lite_v2.2`, layout hash `sha256:e9cef3987168ea77bd7e99fee731cb66ec365245cb9ec1df3d12636f5c00d823`
- `standard_compact_v1`: tensor size 1375, tensor version 1, feature schema `standard_compact_v1.1`, layout hash `sha256:7a06fb143d60e854cec0cc36763d8a886afdf98d58f05e638fcd475e1256ca74`

## Missing Production YAML

- `BT1-087`, `BT10-042`, `BT10-101`, `BT11-043`, `BT11-080`, `BT13-003`, `BT13-034`, `BT13-106`, `BT14-003`, `BT14-037`
- `BT14-084`, `BT14-093`, `BT14-102`, `BT15-034`, `BT15-038`, `BT15-042`, `BT16-088`, `BT18-082`, `BT21-004`, `BT22-004`
- `BT22-031`, `BT22-034`, `BT22-043`, `BT22-044`, `BT22-046`, `BT22-054`, `BT22-056`, `BT22-093`, `BT22-101`, `BT23-027`
- `BT23-037`, `BT4-084`, `BT4-111`, `BT6-089`, `BT6-100`, `BT7-032`, `BT8-035`, `BT8-071`, `BT8-077`, `BT8-082`
- `EX10-031`, `EX10-051`, `EX2-003`, `EX4-005`, `EX6-016`, `EX6-030`, `EX6-053`, `EX7-064`, `EX8-064`, `LM-043`
- `P-221`, `P-225`, `ST10-02`, `ST10-06`, `ST10-12`, `ST10-14`, `ST20-05`

## Missing Behavioral Tests

- `BT1-087`, `BT10-042`, `BT10-101`, `BT11-043`, `BT11-080`, `BT13-003`, `BT13-034`, `BT14-003`, `BT14-037`, `BT14-084`
- `BT14-093`, `BT14-102`, `BT15-034`, `BT15-038`, `BT15-042`, `BT16-088`, `BT18-082`, `BT21-004`, `BT22-004`, `BT22-031`
- `BT22-034`, `BT22-043`, `BT22-044`, `BT22-046`, `BT22-054`, `BT22-056`, `BT22-093`, `BT22-101`, `BT23-027`, `BT23-037`
- `BT4-084`, `BT4-111`, `BT6-089`, `BT6-100`, `BT7-032`, `BT8-035`, `BT8-071`, `BT8-077`, `BT8-082`, `EX10-031`
- `EX10-051`, `EX2-003`, `EX4-005`, `EX6-016`, `EX6-030`, `EX6-053`, `EX7-064`, `EX8-064`, `LM-043`, `P-221`
- `P-225`, `ST10-02`, `ST10-06`, `ST10-12`, `ST10-14`, `ST20-05`

## Planned Substrate Blocker Groups

- Effect-created digivolution-source placement observer context for the CS package: `BT22-004`, `BT22-043`, `BT22-044`, `BT22-054`, `BT22-093`.
- Choice-shaped top-or-bottom security trash costs: `BT15-038`, `BT15-042`.
- Aggregate visible-zone play-cost budget selection and batch free play: `EX8-064`.
- Conditional attack/timing suppression keyed to Security Attack: `BT10-042`.
- Temporary rules-visible original-name mutation: `BT11-043`.
- Security follow-up activation from effect-trashed security: promote `BT13-106` from behavioral-test-only to production YAML coverage.
