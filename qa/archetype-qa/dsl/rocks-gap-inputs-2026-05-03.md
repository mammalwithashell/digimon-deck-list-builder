# Rocks Rust DSL/Engine Gap Inputs

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


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
- YAML found under `code/digimon-engine/cards/**` after the 2026-05-04 pool pass plus pulled main updates and the 2026-05-08 EX10-003 Track D slice: 41 of 47 pool cards.
- Rocks pool cards with production YAML/test slices added or audited on 2026-05-04: `BT14-009`, `BT18-064`, `BT21-055`, `BT23-059`, `BT23-096`, `BT4-072`, `BT8-094`, `EX10-025`, `EX10-028`, `EX10-032`, `EX10-033`, `EX10-034`, `EX10-036`, `EX10-063`, `EX10-069`, `EX11-038`, `EX11-044`, `EX7-049`, `EX8-005`, `EX8-046`, `EX8-047`, `EX8-048`, `EX8-050`, `EX8-051`, `EX8-055`, `EX8-067`, `LM-031`, `LM-032`, `P-039`, `P-107`, `P-167`, `P-169`, `P-186`, `P-215`, `ST13-08`, `ST22-11`.
- Remaining Rocks pool cards without production YAML after one pass: `BT21-021`, `BT9-103`, `EX11-065`, `EX8-070`, `P-130`. `BT20-055` now has production YAML/test coverage for its security end-of-opponent-turn self-play slice; its security-flip rider remains gap-routed.
- Existing YAML quality notes:
  - `BT16-082` is still a documented no-op placeholder even though `OnMove` support now exists.
  - `P-206` and `EX7-074` still contain raw-Rust/self-disposition workarounds that should be revisited against newer DSL support.
  - `BT14-009` moved from `_examples` to production YAML on 2026-05-04 and is covered by Rust behavioral tests.

## Reusable Gaps For Cross-Archetype Spec

### G-ROCKS-REVEAL-ORDERING — CLOSED (Phase 2 Track E, 2026-05-17)

- **Status:** CLOSED. Author-facing residual landed via Phase 2 Track E.
- **Type:** DSL / engine action-surface gap
- **Blocked Rocks cards (now unblocked):** `P-167`, `EX8-047`, plus general
  expressibility for `P-107`, `P-039`, `P-206`, `EX7-074`, `BT16-082`
- **Cross-archetype reuse:** memory boosts, Trainings, searchers, and cards
  that say "return the rest to the top/bottom of the deck in any order"
- **Resolution:** two new DSL verbs ship as wrappers over the already-shipped
  `select_reveal` / `select_effect_choice` / `select_ordered_permutation` /
  `place_remainder_on_deck` engine helpers.
- **Realised DSL shape:**

  ```yaml
  - reveal_top_deck:    { of: you, count: 3, bind_as: revealed }
  - choose_from_reveal:
      of: you
      filter: { any_of: [trait_has: Mineral, trait_has: Rock] }
      destination: hand          # | deck_top | deck_bottom | { bottom_source_of: { target: this } }
      bind_as: picked
      optional: true
      prompt: "Add 1 Mineral or Rock card to your hand"
  - order_remainder:
      of: you
      destinations: [deck_top, deck_bottom]   # 1 entry = direct placement; 2 = player effect-choice
  ```

- **Closure evidence:**
  - DSL flow tests: `code/digimon-engine/tests/dsl/track_e_reveal_ordering.rs` (6 behavioral + 2 YAML round-trip)
  - P-167 authored: `code/digimon-engine/cards/p/P-167.yaml` `[Start of Your Main Phase][When Digivolving]` clause
  - EX8-047 authored: `code/digimon-engine/cards/ex8/EX8-047.yaml` `[On Play]` reveal+two-pick clause
- **First test (now passing):** `track_e_reveal_ordering::p_167_style_reveal_choose_order_full_flow`
  reveals three cards, picks one Mineral/Rock to hand, then exposes a
  player effect-choice for top-vs-bottom AND a full ordered-permutation
  selection for the remainder.

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
- **Current evidence:** direct `select_own_sources` / `trash_selected_sources` and `phase3d_event_context` tests pass. `EX8-051` now verifies the trashed source card can fire its own inherited `OnDigivolutionCardTrashed` effect from host/source trigger context, including return-to-deck source disposition after the host leaves the battle area. The 2026-05-07 Track A slices also prove return-to-deck and de-digivolve payload fixtures, route Fragment/source-trash helpers plus Armor Purge through the same source-trash emitter, and prove BT4-072's exact-N Digi-Burst producer through reusable `digi_burst`. The 2026-05-08 count-2 fixture extends that evidence to multi-source Digi-Burst masking and per-source event emission.
- **Required capability:** remaining source-trash producers and broader card-local cost shapes must emit `OnDigivolutionCardTrashed` with stable host permanent/card, trashed source card, source index, and cause player context.
- **Suggested DSL shape:** no new author-facing syntax if producer coverage is complete; existing predicates should work:

  ```yaml
  active_when:
    all_of:
      - host_permanent_trait_has: Mineral
      - trashed_source_trait_has: Rock
  ```

- **First test:** Use `EX10-032` to trash exactly one selected Mineral/Rock source from a non-source Digimon, then assert only that source card's inherited de-digivolve effect fires and unrelated sources in the same host do not trigger.

### G-ROCKS-OPTION-SELF-DISPOSITION — CLOSED (Phase 2 Track E, 2026-05-17)

- **Status:** CLOSED for all six target cards.
- **Type:** DSL ergonomics / raw-Rust removal gap
- **Blocked Rocks cards (now unblocked):** `P-206`, `EX7-074`, `P-107`,
  `P-039`, `LM-031`, `EX10-069`
- **Cross-archetype reuse:** Trainings, Memory Boosts, Scrambles, Unique
  Emblems, Vortex/Resonance-style Options
- **Resolution:** P-206's `raw_rust { fn: p_206_add_self_to_hand }` was
  replaced with native DSL `add_this_option_to_hand: {}` (both call the
  identical `EffectContext::add_pending_security_to_hand` helper — modernization
  is behaviourally a no-op). The dead `p_206_add_self_to_hand` function was
  removed from `code/digimon-engine/src/cards/raw_rust/mod.rs`. EX7-074,
  P-107, P-039, LM-031, EX10-069 were already DSL-clean at the 2026-05-10
  hygiene sweep (no remaining raw_rust calls) — re-audited and confirmed.
- **DSL shapes (now production):**

  ```yaml
  - place_self_as_delay_option: {}     # auto-placement via Delay clause detection
  - add_this_option_to_hand: {}        # post-security "add this card to hand" tail
  ```

  `trash_this_option` was scoped on plan-write but no card in the
  modernization list requires it — Option auto-trash is already handled by
  the engine's `classify_option_subtype` + `dispose_option` for cards
  without a Delay clause. Defer until a card with explicit printed "trash
  this card" semantics surfaces.
- **Closure evidence:**
  - P-206 YAML modernization: `code/digimon-engine/cards/p/P-206.yaml`
  - Removed raw_rust: `code/digimon-engine/src/cards/raw_rust/mod.rs`
  - Behavioral regression: P-206 18-test suite still all-green (no
    regression from raw_rust → native DSL).

### G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS — CLOSED (Phase 2 Track E, 2026-05-17)

- **Status:** CLOSED for BT9-103. No new substrate proved necessary —
  BT9-103 authors cleanly with the existing `add_player_modifier` step,
  `for_each` over a play-cost-filtered opponent battle-area predicate, and
  `add_modifier { CannotAttackPlayer }` per Digimon (BT14-009 / ST13-08
  pattern adapted from declarative `kind: flood_gate` to triggered Main
  process).
- **Type:** engine / DSL gap
- **Blocked Rocks cards (now unblocked):** `BT9-103`
- **Cross-archetype reuse:** floodgates and global player-scoped restrictions
- **Realised YAML:** `code/digimon-engine/cards/bt9/BT9-103.yaml` — Main and
  Security mirror clauses both install `CannotAddSecurityByEffect` on the
  opponent (via `add_player_modifier`) and apply `CannotAttackPlayer` to
  every opponent Digimon with `play_cost_lte: 7` (via `for_each` +
  `add_modifier`), both expiring at `end_of_opponents_turn`.
- **Closure evidence:**
  - Behavioral test:
    `code/digimon-engine/tests/cards_behavioral/bt9/bt9_103.rs::bt9_103_main_installs_modifiers_on_opponent_and_low_cost_digimon`
    verifies opponent gains `CannotAddSecurityByEffect`, opponent Digimon
    cost ≤ 7 gain `CannotAttackPlayer`, opponent Digimon cost > 7 do not.
  - Structural test asserts both the player-modifier and per-Digimon
    `for_each + add_modifier` arms are present in the compiled clause.
- **Tracker note:** the validated_cards_dsl.json `BT9-103` entry was
  marked BLOCKED (yaml_path: null) at the 2026-05-04 pool pass — the YAML
  + test land here; tracker advanced to IMPLEMENTED in this PR.

## Rocks-Local Authoring And Test Gaps

These should not become cross-archetype gap entries unless authoring proves a reusable primitive is still missing.

| Card(s) | Status | Next Rust test |
|---|---|---|
| `EX10-032` | partial YAML/test slice added | Remaining: source-trash selection grants Collision, Piercing, and +3000 DP until opponent turn end |
| `P-167` | partial YAML/test slice added plus reveal-ordering dependency | Remaining: start-main and when-digivolving source-trash reveal flow, including add-to-hand vs place-as-source branch |
| `EX8-047`, `BT21-055`, `EX8-005` | partial/implemented YAML/test slices added | Remaining: `EX8-047`/`BT21-055` face-up search/reduction clauses |
| `EX10-036` | partial YAML/test slice added | Remaining: trash exactly three legal Mineral/Rock sources, delete target, trash top security, place three from trash, and unsuspend once per turn |
| `EX10-069` | partial YAML/test slice added / reusable Delay verification gap | Remaining: place itself in battle area, then activate Delay only when Close suspends |
| `BT16-082` | placeholder replacement / test gap | OnMove reveal-add flow, bottom remainder handling, then optional hatch without triggering on hatch |
| `P-206`, `EX7-074` | modernization / test gap | Remove raw-Rust self-disposition where standard DSL can express the printed Option flow |
| `BT14-009`, `BT18-064`, `EX8-051`, `ST13-08` | implemented 2026-05-04 | Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt14_009 bt18_064 ex8_051 st13_08 --nocapture` |
| `EX10-003` | implemented 2026-05-08 | Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_003`; also proves `select_own_sources` `from:` + `filter:` lowering. |
| `BT21-021`, `BT9-103`, `EX11-065`, `EX8-070`, `P-130` | blocked after pass | See `qa/archetype-qa/dsl/rocks.md` and `qa/qa-reports/validated_cards_dsl.json` for per-card gap routing. |
| `BT20-055` | partial production YAML | `[Security] [End of Opponent's Turn]` self-play covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_055_security_end_of_opponents_turn_plays_self_from_security`; security-flip rider still blocked on face-up security lifecycle. |
| `P-123` | covered by pulled main update | Production YAML/tests are present on main after the pull; no longer counted in the Rocks blocked remainder. |

## Stale Tracker Cleanup Candidates

The following older Rocks gap claims should be reviewed before a new roadmap spec is compiled:

- `G-ROCKS-SOURCE-SELECTION-DSL`: now mostly closed for `select_own_sources`, `from:` host restriction, source-card `filter:`, and `trash_selected_sources`; keep only producer-context completeness and card authoring work.
- `G-ON-MOVE`: no longer a primitive blocker for `BT16-082`; the card is blocked by placeholder YAML and reveal/hatch authoring.
- `G-COLLISION`: no longer a primitive blocker; combat tests cover Collision.
- `G-IGNORE-COLOR-MASK`, `G-PLAY-COST-LTE`, `color_matches_any_field_digimon`: no longer broad primitive blockers; remaining work is card modernization and tests.
- `Fragment`: printed keyword support exists, but Rocks still needs card-level tests that prove Fragment source-trash/replacement interacts correctly with inherited source-trash observers.

## Suggested Spec Compilation Order

1. Promote `G-ROCKS-REVEAL-ORDERING` into the cross-archetype roadmap because it affects many search and training effects beyond Rocks.
2. Promote `G-ROCKS-SOURCE-TRASH-CONTEXT-COMPLETE` as a producer-audit task, not a new syntax task.
3. Promote `G-ROCKS-DELAY-EVENT-DIGIVOLVE` only for the remaining event-gated Delay + effect-digivolve verification slices not already covered by Puppet work.
4. Promote `G-ROCKS-OPTION-SELF-DISPOSITION` as a DSL cleanup and raw-Rust retirement task.
5. Keep `G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS` local to remaining card authoring unless `BT9-103` proves a new reusable primitive is still missing; `BT14-009` and `ST13-08` are production-authored.
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
