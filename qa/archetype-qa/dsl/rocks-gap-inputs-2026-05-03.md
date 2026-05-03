# Rocks Rust DSL/Engine Gap Inputs

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `Rocks`, refreshed from the local archetype pool with 47 unique card IDs. This document is a spec-input artifact: it separates remaining reusable DSL/engine gaps from Rocks-local YAML authoring and card-test work so a later cross-archetype roadmap can compile only the reusable capability gaps.

Older `qa/archetype-qa/rocks.md` notes are Python-lane QA. They should not be treated as Rust DSL readiness.

## Verdict

`blocked`

Rocks is not currently implementable faithfully as executable Rust YAML DSL. The current engine has many primitives the earlier Rocks audit needed: cross-permanent source selection, source trash bindings, `OnMove`, `WhenAttacking`, `StartOfYourMainPhase`, `Collision`, `Fragment`, ignore-color option masks, `play_cost_lte`, host/source event predicates, and event-context tests.

The remaining blocker is mostly authored card coverage plus a smaller set of reusable gaps that should be folded into the cross-archetype DSL/engine roadmap.

## Coverage Snapshot

- Archetype pool: 47 unique card IDs.
- YAML found under `code/digimon-engine/cards/**`: `BT16-082`, `P-206`, `_examples/BT14-009`, `EX7-074`.
- Top Rocks shell without production YAML: `EX10-032`, `P-167`, `EX8-047`, `EX8-005`, `EX10-036`, `EX10-069`, `EX8-067`, `BT21-055`, `P-107`, `EX8-048`, `EX10-063`, `EX8-051`, `EX10-028`, `EX10-033`, `EX10-025`, `LM-031`, `P-169`, `P-039`, `EX8-055`.
- Existing YAML quality notes:
  - `BT16-082` is still a documented no-op placeholder even though `OnMove` support now exists.
  - `P-206` and `EX7-074` still contain raw-Rust/self-disposition workarounds that should be revisited against newer DSL support.
  - `_examples/BT14-009` is not a production card spec location.

## Reusable Gaps For Cross-Archetype Spec

### G-ROCKS-REVEAL-ORDERING

- **Type:** DSL / engine action-surface gap
- **Blocks Rocks cards:** `P-167`, `EX8-047`, `P-107`, `P-039`, `P-206`, `EX7-074`, `BT16-082`
- **Cross-archetype reuse:** memory boosts, Trainings, searchers, and cards that say "return the rest to the top/bottom of the deck in any order"
- **Printed shape:** reveal top N, choose one or more matching cards for hand/source/play, then return the rest to top or bottom in any order
- **Current evidence:** `docs/RUST_ENGINE_GAPS.md` tracks "Selection: ordered permutation"; current Rocks shell still needs faithful ordering for reveal remainders, especially `P-167` top-or-bottom ordering
- **Required capability:** a reusable ordered-selection primitive for a small revealed set, with the chosen order exposed through pending selection/action masks
- **Suggested DSL shape:**

  ```yaml
  - reveal:
      count: 3
      bind_as: revealed
  - choose_from_reveal:
      from: revealed
      count: 1
      filter: { trait_any: [Mineral, Rock] }
      destination: hand
  - order_remainder:
      from: revealed
      destination:
        choose_one: [deck_top, deck_bottom]
  ```

- **First test:** `P-167` reveals three cards, chooses one legal Mineral/Rock card, then exposes an ordering choice for the remaining cards and preserves that exact order on top or bottom.

### G-ROCKS-DELAY-EVENT-DIGIVOLVE

- **Type:** hybrid DSL / engine verification gap
- **Blocks Rocks cards:** `EX10-069`, plus related Delay options such as `P-107`, `P-039`, `LM-031`
- **Cross-archetype reuse:** Puppet Unique Emblems, Scramble options, Training/Memory Boost style placed options
- **Printed shape:** place an Option in the battle area, later activate `<Delay>` from an event window, then perform a reduced-cost effect-initiated digivolve
- **Current evidence:** event-gated Delay has partial support for `on_suspend` and the Group 5 Delay path, but the Rocks card still needs a production test for "when any Close suspends" and the reduced-cost hand digivolve body
- **Required capability:** Delay activation gated by event-card predicates, with effect-initiated digivolve target and hand-card filters revalidated at activation time
- **Suggested DSL shape:**

  ```yaml
  - kind: delay
    trigger: on_suspend
    active_when:
      event_card_name_contains: Close
    process:
      - effect_initiated_digivolve:
          target:
            trait_any: [Mineral, Rock]
          into:
            zone: hand
            trait_all: [Mineral, LIBERATOR]
          cost_delta: -3
  ```

- **First test:** `EX10-069` is in the battle area, `EX8-067 Close` suspends, and the mask exposes a legal optional Delay activation that digivolves a Mineral/Rock Digimon into a Mineral/LIBERATOR hand card at cost reduced by 3.

### G-ROCKS-SOURCE-TRASH-CONTEXT-COMPLETE

- **Type:** engine verification / producer coverage gap
- **Blocks Rocks cards:** `EX10-032`, `P-167`, `EX8-047`, `EX8-005`, `EX10-036`, `BT21-055`, `EX8-048`, `EX10-028`, `EX10-033`, `EX10-025`, `EX8-055`, `EX11-044`
- **Cross-archetype reuse:** Digi-Burst, Fragment, source-trash costs, inherited "when this card is trashed from digivolution cards" effects
- **Printed shape:** a specific source card is trashed from a specific host stack, and only that card's inherited/source-trash effects should observe the event
- **Current evidence:** direct `select_own_sources` / `trash_selected_sources` and `phase3d_event_context` tests pass. Remaining risk is producer completeness across every source-disposition path, including older return-to-deck, de-digivolve, Fragment, Armor Purge, and keyword-driven source trash routes.
- **Required capability:** every source-trash producer must emit `OnDigivolutionCardTrashed` with stable host permanent/card, trashed source card, source index, and cause player context
- **Suggested DSL shape:** no new author-facing syntax if producer coverage is complete; existing predicates should work:

  ```yaml
  active_when:
    all_of:
      - host_permanent_trait_has: Mineral
      - trashed_source_trait_has: Rock
  ```

- **First test:** Use `EX10-032` to trash exactly one selected Mineral/Rock source from a non-source Digimon, then assert only that source card's inherited de-digivolve effect fires and unrelated sources in the same host do not trigger.

### G-ROCKS-OPTION-SELF-DISPOSITION

- **Type:** DSL ergonomics / raw-Rust removal gap
- **Blocks Rocks cards:** `P-206`, `EX7-074`, `P-107`, `P-039`, `LM-031`, `EX10-069`
- **Cross-archetype reuse:** Trainings, Memory Boosts, Scrambles, Unique Emblems, Vortex/Resonance-style Options
- **Printed shape:** after resolving a Main or Security effect, the Option moves itself to battle area, hand, trash, or other configured destination
- **Current evidence:** `place_self_as_delay_option` and `add_this_option_to_hand` support exist, but older YAML still uses raw-Rust hooks and comments from before those primitives landed
- **Required capability:** production DSL examples and card tests for every self-disposition mode used by option cards, with no raw-Rust fallback for standard flows
- **Suggested DSL shape:**

  ```yaml
  - place_self_as_delay_option: {}
  - add_this_option_to_hand: {}
  - trash_this_option: {}
  ```

- **First test:** Modernize `P-206` or `EX7-074` to use DSL-only self-disposition and verify Main and Security paths move the option to the printed destination.

### G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS

- **Type:** engine / DSL gap
- **Blocks Rocks cards:** `BT14-009`, `BT9-103`, `ST13-08`
- **Cross-archetype reuse:** floodgates and global player-scoped restrictions
- **Printed shape:** while this permanent is in play, restrict a player or both players from playing/reducing/attacking under a condition
- **Current evidence:** `BT14-009` lives in `_examples`; older tracker notes still call out bilateral player-scoped passive modifier shapes
- **Required capability:** author production YAML for player-scoped passive modifiers with controller/opponent/both-player scope and revalidate masks at every affected action point
- **Suggested DSL shape:**

  ```yaml
  - kind: passive_modifier
    modifier: cannot_play_digimon_by_effect
    applies_to: both_players
    filter:
      play_cost_gte: 5
  ```

- **First test:** `BT14-009` in battle area blocks both players from playing cost-5-or-higher Digimon by effects, but does not block normal hand plays or lower-cost effect plays.

## Rocks-Local Authoring And Test Gaps

These should not become cross-archetype gap entries unless authoring proves a reusable primitive is still missing.

| Card(s) | Status | Next Rust test |
|---|---|---|
| `EX10-032` | authoring / test gap | Source-trash selection grants Collision, Piercing, and +3000 DP until opponent turn end; selected source inherited effect fires exactly once |
| `P-167` | authoring / test gap plus reveal-ordering dependency | Start-main and when-digivolving source-trash reveal flow, including add-to-hand vs place-as-source branch |
| `EX8-047`, `BT21-055`, `EX8-005` | authoring / test gap | On-play/search and inherited source-trash delete or memory gain only when that exact source card is trashed |
| `EX10-036` | authoring / test gap | Trash exactly three legal Mineral/Rock sources, delete target, trash top security, place three from trash, and unsuspend once per turn |
| `EX10-069` | authoring / reusable Delay verification gap | Place itself in battle area, then activate Delay only when Close suspends |
| `BT16-082` | placeholder replacement / test gap | OnMove reveal-add flow, bottom remainder handling, then optional hatch without triggering on hatch |
| `P-206`, `EX7-074` | modernization / test gap | Remove raw-Rust self-disposition where standard DSL can express the printed Option flow |
| `_examples/BT14-009` | production placement / test gap | Move to production card location when ready and cover bilateral player-scoped passive mask behavior |

## Stale Tracker Cleanup Candidates

The following older Rocks gap claims should be reviewed before a new roadmap spec is compiled:

- `G-ROCKS-SOURCE-SELECTION-DSL`: now mostly closed for `select_own_sources` and `trash_selected_sources`; keep only producer-context completeness and card authoring work.
- `G-ON-MOVE`: no longer a primitive blocker for `BT16-082`; the card is blocked by placeholder YAML and reveal/hatch authoring.
- `G-COLLISION`: no longer a primitive blocker; combat tests cover Collision.
- `G-IGNORE-COLOR-MASK`, `G-PLAY-COST-LTE`, `color_matches_any_field_digimon`: no longer broad primitive blockers; remaining work is card modernization and tests.
- `Fragment`: printed keyword support exists, but Rocks still needs card-level tests that prove Fragment source-trash/replacement interacts correctly with inherited source-trash observers.

## Suggested Spec Compilation Order

1. Promote `G-ROCKS-REVEAL-ORDERING` into the cross-archetype roadmap because it affects many search and training effects beyond Rocks.
2. Promote `G-ROCKS-SOURCE-TRASH-CONTEXT-COMPLETE` as a producer-audit task, not a new syntax task.
3. Promote `G-ROCKS-DELAY-EVENT-DIGIVOLVE` only for the remaining event-gated Delay + effect-digivolve verification slices not already covered by Puppet work.
4. Promote `G-ROCKS-OPTION-SELF-DISPOSITION` as a DSL cleanup and raw-Rust retirement task.
5. Promote `G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS` if `BT14-009`/`BT9-103`/`ST13-08` production YAML cannot be authored with existing modifier lowering.
6. Keep all remaining Rocks cards as TDD authoring work under `code/digimon-engine/tests/` and `code/digimon-engine/cards/**`, not as roadmap gaps.

## Verification References

Targeted read-only checks from the 2026-05-03 Rocks refresh:

```bash
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- phase3d_event_context
cargo test --manifest-path code\digimon-engine\Cargo.toml --test selection -- source_multi
cargo test --manifest-path code\digimon-engine\Cargo.toml --test combat -- collision
cargo test --manifest-path code\digimon-engine\Cargo.toml --test flood_gates -- group6_option_color
```

All four targeted checks passed during the assessment.
