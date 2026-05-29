# Xros Heart Authoring Substrate Baseline - 2026-05-24

This baseline was recorded for OpenSpec change
`complete-xros-heart-authoring-substrate` after resolving both common library
aliases with `code/tools/resolve_deck.py`.

## Resolver Inputs

- `Xros Heart`: 35 decklists, 59 unique cards, meta share `0.009621`, deck pool
  at `qa/archetype-qa/xros-heart/deck_pool.json`.
- `XrosHeart`: 19 decklists, 41 unique cards, meta share `0.005223`, deck pool
  at `qa/archetype-qa/xrosheart/deck_pool.json`.

## Combined Rust YAML Coverage

- Combined unique cards: 64.
- Current Rust YAML present: 42 after the 2026-05-24 card-authoring pass.
- Current Rust YAML missing: 22.

The resolver's legacy Python script coverage is not used as the Rust readiness
signal. This baseline counts production YAML under `code/digimon-engine/cards/`.

## High-Frequency Missing Cards

| Card | Combined frequency | Name | Kind | Substrate focus |
| --- | ---: | --- | --- | --- |
| `BT19-008` | 54 | Shoutmon | Digimon | source-zone effect digivolve from under Tamers |
| `BT19-038` | 53 | JaegerDorulumon | Digimon | timing lockout and cannot-unsuspend |
| `BT19-051` | 53 | AtlurBallistamon | Digimon | return protection and DP modifier |
| `BT19-014` | 52 | Shoutmon EX6 | Digimon | source-color count and current-DP comparison |
| `BT19-035` | 50 | ShootingStarmon | Digimon | played-Xros Heart observer, DP/Security A. modifier |
| `BT10-003` | 41 | Pickmons | DigiEgg | authored 2026-05-24 |
| `BT19-057` | 32 | Sparrowmon | Digimon | source-zone effect digivolve from under Tamers |
| `BT19-012` | 27 | OmniShoutmon | Digimon | authored 2026-05-24 |
| `BT21-011` | 27 | Shoutmon | Digimon | authored 2026-05-24 |
| `AD1-006` | 23 | Shoutmon X7 | Digimon | current-DP comparison and leave-battle source flow |
| `AD1-013` | 23 | ZeigGreymon | Digimon | fewest-source selector and source-color inherited DP |
| `BT19-079` | 22 | Taiki Kudo | Tamer | Tamer-routed DigiXros material access |
| `BT10-029` | 20 | Starmons | Digimon | authored 2026-05-24 |
| `BT19-026` | 16 | ZeigGreymon | Digimon | De-Digivolve, source play, Save |
| `BT20-037` | 16 | Chaosmon: Valdur Arm | Digimon | On Play lockout and cannot-unsuspend |
| `BT21-030` | 13 | Shoutmon X7: Superior Mode | Digimon | trash-as-DigiXros material and no-source payoff |

## Verdict

The remaining blockers are reusable substrate gaps, not one-off card TODOs:

- source-zone effect digivolve from cards under Tamers;
- stack-derived selectors/formulas, especially fewest source count and
  source-color/current-DP comparisons;
- temporary activation lockouts for On Play and When Digivolving effects plus
  cannot-unsuspend expiry.

Once those primitives are covered, the remaining pool should be treated mostly
as production card-authoring work.

## Progress Notes

- 2026-05-24: Source-zone effect digivolve has production proof on `BT19-008`
  and `BT19-057`.
- 2026-05-24: Reveal-pool free play landed as `choose_from_reveal`
  `destination: play_free` plus `EffectContext::play_from_reveal_free`, covering
  `BT19-008`'s On Deletion reveal/play clause.
- 2026-05-24: Stack-derived metric DSL support now covers `source_color_count`
  formulas and `per: source_color_count`, composing with existing `source_dp`,
  no-source filters, and `lowest_material_count`. Production YAML and focused
  behavior tests now cover `BT19-014`, `AD1-006`, `AD1-013`, `BT19-026`, and
  `BT21-030`.
- 2026-05-24: `source_stack_count` now covers predicate-matched source-card
  counts for BT20-037-style "for each level 6 source" bounds and memory math.
- 2026-05-24: Temporary timing lockouts and cannot-unsuspend effects now have
  production YAML and focused behavior tests for `BT19-038`, `BT19-051`,
  `BT19-035`, `BT20-037`, and `BT19-079`.
- 2026-05-24 resolver refresh: `code/tools/resolve_deck.py "Xros Heart"
  --json` and `code/tools/resolve_deck.py "XrosHeart" --json` refreshed the
  deck pools. Combined Rust YAML coverage is now 36/64, with 28 missing cards:
  `BT1-087`, `BT10-003`, `BT10-008`, `BT10-029`, `BT10-034`, `BT10-089`,
  `BT10-090`, `BT11-012`, `BT11-018`, `BT11-076`, `BT11-086`, `BT12-011`,
  `BT17-041`, `BT17-079`, `BT19-001`, `BT19-010`, `BT19-012`, `BT19-013`,
  `BT19-033`, `BT19-047`, `BT19-076`, `BT19-087`, `BT21-011`, `BT8-095`,
  `BT9-083`, `EX5-070`, `LM-045`, and `P-152`.
- 2026-05-24 card-authoring pass: `BT10-003`, `BT10-029`, `BT19-033`, and
  `BT19-047` now have production YAML plus focused behavioral tests. Combined
  Rust YAML coverage is now 40/64, with 24 missing cards: `BT1-087`,
  `BT10-008`, `BT10-034`, `BT10-089`, `BT10-090`, `BT11-012`, `BT11-018`,
  `BT11-076`, `BT11-086`, `BT12-011`, `BT17-041`, `BT17-079`, `BT19-001`,
  `BT19-010`, `BT19-012`, `BT19-013`, `BT19-076`, `BT19-087`, `BT21-011`,
  `BT8-095`, `BT9-083`, `EX5-070`, `LM-045`, and `P-152`.
- 2026-05-24 same-effect DP primitive follow-up: permanent `dp_lte` / `dp_eq`
  / `dp_gte` predicates now skip printed `CardData.dp` during delegated
  card-field checks and evaluate field targets through `effective_dp`. This
  closes the BT19-012 substrate blocker; remaining BT19-012 work is production
  YAML/card tests.
- 2026-05-24 follow-up card-authoring pass: `BT19-012` and `BT21-011` now have
  production YAML plus focused behavioral tests. Combined Rust YAML coverage is
  now 42/64, with 22 missing cards: `BT1-087`, `BT10-008`, `BT10-034`,
  `BT10-089`, `BT10-090`, `BT11-012`, `BT11-018`, `BT11-076`, `BT11-086`,
  `BT12-011`, `BT17-041`, `BT17-079`, `BT19-001`, `BT19-010`, `BT19-013`,
  `BT19-076`, `BT19-087`, `BT8-095`, `BT9-083`, `EX5-070`, `LM-045`, and
  `P-152`.
- Xros Heart is ready to move into broad card authoring for the resolved pool:
  the known blockers in this substrate change are closed, and the remaining
  pool entries should be treated as ordinary card-authoring work unless a new
  reusable primitive is proven by a concrete card test.
