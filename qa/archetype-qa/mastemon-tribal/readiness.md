# Mastemon Tribal Rust DSL Readiness

Last updated: 2026-05-24 (`unblock-mastemon-full-pool-rust-dsl` baseline)

## Best-deck status

The resolved best deck from `python code/tools/resolve_deck.py "Mastemon (Tribal)" --json` has 20 unique card IDs. All 20 now have production YAML under `code/digimon-engine/cards/<set>/` and behavioral tests under `code/digimon-engine/tests/cards_behavioral/<set>/`.

Best-deck card IDs:

- `BT11-042`, `BT11-083`, `BT11-094`, `BT14-033`, `BT15-003`, `BT15-037`, `BT22-089`, `BT23-031`, `BT23-067`, `BT23-102`
- `BT7-107`, `BT9-082`, `EX6-020`, `EX6-022`, `EX6-029`, `EX6-074`, `EX8-030`, `P-187`, `P-206`, `ST10-04`

No best-deck card is blocked by raw-Rust card escapes, no-op stubs, or comments claiming unimplemented printed text. Stale `_examples` fixtures for `BT7-107`, `BT15-003`, and `BT11-042` were replaced with their production DSL shapes so the example pack no longer contradicts the production card lane.

## Closed substrate

This change closed the Mastemon-critical substrate without expanding `ACTION_SPACE_SIZE` or changing active tensor profiles:

- Owner-routed selected permanent to security: `place_permanent_on_owners_security`
- Security-stack cost gates: `if_trash_top_security_cost`, `if_place_permanent_on_owners_security_cost`
- Selected-security play with success tails: `play_security_card`
- Selected-security effect digivolve proof: bound security card plus `effect_initiated_digivolve`
- Trash recursion / self replay: `play_this_from_trash_free`
- Recovery by successful deletion count: `recover_for_deleted`
- Result-log propagation through `for_each` and `per_selected`
- Optional outer prompt condition evaluation under queued trigger context

## Full-pool follow-up coverage

The full resolved pool has 93 unique cards. Current coverage is 36 cards with both production YAML and behavioral tests. The remaining cards below are follow-up coverage for the broader pool, not blockers for resolved best-deck readiness.

The current full-pool baseline is recorded in `qa/archetype-qa/mastemon-tribal/full-pool-baseline.md`. Summary:

- Resolver output: `Mastemon (Tribal)`, 55 decklists, 93 unique cards, 20 best-deck unique cards.
- Rust coverage: 36 production YAML cards, 37 behavioral-test cards, 36 cards with both.
- Missing Rust production YAML: 57 cards.
- Missing behavioral tests: 56 cards.
- RL contract baseline before full-pool substrate work: `ACTION_SPACE_SIZE` 2192; `standard_lite_v2` tensor size 8410, schema `standard_lite_v2.2`, layout hash `sha256:e9cef3987168ea77bd7e99fee731cb66ec365245cb9ec1df3d12636f5c00d823`; `standard_compact_v1` tensor size 1375, schema `standard_compact_v1.1`, layout hash `sha256:7a06fb143d60e854cec0cc36763d8a886afdf98d58f05e638fcd475e1256ca74`.

Planned full-pool substrate blocker groups:

- Effect-created digivolution-source placement observer context for the CS package: `BT22-004`, `BT22-043`, `BT22-044`, `BT22-054`, `BT22-093`.
- Choice-shaped top-or-bottom security trash costs: `BT15-038`, `BT15-042`.
- Aggregate visible-zone play-cost budget selection and batch free play: `EX8-064`.
- Conditional attack/timing suppression keyed to Security Attack: `BT10-042`.
- Temporary rules-visible original-name mutation: `BT11-043`.
- Security follow-up activation from effect-trashed security: promote `BT13-106` from behavioral-test-only to production YAML coverage.

Higher-frequency non-best-deck follow-up:

- `BT14-003` (36), `BT22-034` (30), `BT10-042` (16), `BT13-034` (16), `ST10-02` (15), `BT15-038` (14), `ST10-12` (13)

Lower-frequency non-best-deck follow-up:

- `BT1-087` (9), `BT8-071` (9), `BT11-080` (6), `EX8-064` (6), `BT22-043` (5), `EX6-030` (5), `EX6-053` (5), `ST20-05` (5)
- `BT22-031` (4), `BT22-044` (4), `BT22-046` (4), `BT22-093` (4), `BT13-003` (3), `BT22-004` (3), `BT22-054` (3), `BT22-056` (3), `EX6-016` (3), `ST10-14` (3)
- `BT11-043` (2), `BT14-084` (2), `BT23-027` (2), `BT7-032` (2), `EX4-005` (2), `ST10-06` (2)
- `BT10-101` (1), `BT13-106` (1), `BT14-037` (1), `BT14-093` (1), `BT14-102` (1), `BT15-034` (1), `BT15-042` (1), `BT16-088` (1), `BT18-082` (1), `BT21-004` (1), `BT22-101` (1), `BT23-037` (1), `BT4-084` (1), `BT4-111` (1), `BT6-089` (1), `BT6-100` (1), `BT8-035` (1), `BT8-077` (1), `BT8-082` (1), `EX10-031` (1), `EX10-051` (1), `EX2-003` (1), `EX7-064` (1), `LM-043` (1), `P-221` (1), `P-225` (1)

Coverage note: `BT13-106` currently has a behavioral test file but no production YAML in the pool coverage scan, so it remains a follow-up until YAML and test coverage are both present.
