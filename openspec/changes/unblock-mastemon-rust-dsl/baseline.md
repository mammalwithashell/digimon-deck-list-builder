# Mastemon Rust DSL Baseline

Generated while applying `unblock-mastemon-rust-dsl`.

## Resolver

Command:

```powershell
$env:PYTHONIOENCODING='utf-8'; python code/tools/resolve_deck.py "Mastemon (Tribal)" --json
```

Results:

- Archetype: `Mastemon (Tribal)`
- Decklists: `55`
- Unique resolved pool cards: `93`
- `qa/archetype-qa/mastemon-tribal/deck_pool.json` entries: `93`
- Resolver output matches `deck_pool.json`: `true`
- Best deck size: `54`
- Best deck unique cards: `20`
- Legacy frozen-script coverage from resolver: `0/93 frozen (0.0%)`

## Rust Coverage Snapshot

- Pool YAML coverage: `21/93`
- Pool behavioral-test coverage: `22/93`
- Best-deck YAML coverage: `5/20`
- Best-deck behavioral-test coverage: `5/20`

Best-deck cards missing Rust YAML and behavioral tests:

- `BT11-042`
- `BT11-083`
- `BT11-094`
- `BT14-033`
- `BT15-037`
- `BT23-031`
- `BT23-067`
- `BT23-102`
- `BT7-107`
- `BT9-082`
- `EX6-020`
- `EX6-022`
- `EX6-074`
- `P-187`
- `ST10-04`

High-frequency cards with no Rust YAML at baseline:

| Card | Frequency | Name |
| --- | ---: | --- |
| `BT11-042` | `51/55` | Angewomon |
| `BT11-083` | `49/55` | LadyDevimon |
| `BT11-094` | `51/55` | Mirei Mikagura |
| `BT14-033` | `51/55` | Patamon |
| `BT23-031` | `47/55` | Angewomon |
| `BT23-067` | `50/55` | LadyDevimon |
| `BT23-102` | `50/55` | Mastemon |
| `EX6-020` | `42/55` | Gatomon |
| `EX6-074` | `49/55` | Mirei Mikagura |
| `P-187` | `52/55` | Mastemon |
| `ST10-04` | `51/55` | Gatomon |

## RL Contract Baseline

- `ACTION_SPACE_SIZE`: `2192`
- `standard_lite_v2`
  - `tensor_size`: `8410`
  - `feature_schema_version`: `standard_lite_v2.2`
  - `layout_hash`: `sha256:e9cef3987168ea77bd7e99fee731cb66ec365245cb9ec1df3d12636f5c00d823`
- `standard_compact_v1`
  - `tensor_size`: `1375`
  - `feature_schema_version`: `standard_compact_v1.1`
  - `layout_hash`: `sha256:7a06fb143d60e854cec0cc36763d8a886afdf98d58f05e638fcd475e1256ca74`
