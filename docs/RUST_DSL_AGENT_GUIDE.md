# Rust DSL Agent Implementation Guide

**Audience:** AI agents and humans implementing Digimon card effects through the Rust YAML DSL in `code/digimon-engine/cards/`.

This guide is the practical authoring companion to:

- [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) for the hand-written Rust `EffectContext` API.
- [`RUST_DSL_TEST_API.md`](RUST_DSL_TEST_API.md) for Rust tests around DSL-authored cards.
- [`RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md), [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md), and [`qa/archetype-qa/engine-gaps.md`](../qa/archetype-qa/engine-gaps.md) for reusable blockers found by archetype audits.
- [`docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`](superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md) for the capability-first roadmap.

**Last refreshed:** 2026-05-15. Tracks A–H folded in via PR #471 (2026-05-14); the `source_is_unsuspended` predicate (PR #472) and the Track I engine-only callout were added 2026-05-15. Phase 1 DSL pipeline completion (2026-05-15) closed `G-ALT-PATH-CONDITION` by adding `condition:` to `alt_paths` entries (Digivolve route) and wired eval arms for `all_turns`, `source_is_tamer`, `of_permanent`, `has_alt_path`, and `has_inherited` predicates. See [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) §0 "Tracks A–K Substrate Quick Reference" for the canonical substrate index, and [`RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md) (swept 2026-05-15) for the live gap state.

The DSL exists so card behavior can be authored declaratively while preserving the no-approximations rule: every legal player choice must be surfaced through engine actions or `PendingSelection`. Do not use YAML stubs, hidden auto-picks, broad `raw_rust` bypasses, or UI-only decisions to claim a card is ready.

## 1. Agent Workflow

Before authoring a card:

1. Read printed text from `data/cards.json`.
2. Check `docs/RULES_CONTEXT.md` for keyword and timing rules.
3. Search existing YAML in `code/digimon-engine/cards/` and examples in `code/digimon-engine/cards/_examples/`.
4. Search the gap trackers for the capability, not just the card ID:
   - `rg "source selection|Delay|replacement|same-level|<card id>" docs/RUST_ENGINE_GAPS.md qa/dsl-vocab-gaps.md qa/archetype-qa`
5. Write or update a failing Rust behavioral test before changing card behavior.
6. Prefer a DSL primitive. Use hand-written Rust only when the reusable engine primitive exists but no DSL vocabulary can lower to it yet, and document that as a DSL gap.
7. Update the relevant gap tracker when a reusable blocker closes, narrows, or turns out to be card-local authoring.

Do not expand `ACTION_SPACE_SIZE`, tensor layouts, PyO3 contracts, frontend constants, or RL wrappers as a side effect of card unlock work. If a new choice cannot fit the existing pending-selection machinery, stop and plan an action/tensor contract change.

## 2. Files and Pipeline

Production card specs live under:

```text
code/digimon-engine/cards/<set>/<CARD-ID>.yaml
```

The DSL crate lives under `code/digimon-dsl/`:

- `spec.rs`: top-level `CardSpec`.
- `clause.rs`: triggered clauses and declarative clauses.
- `step.rs`: one-key `process:` step verbs.
- `predicate.rs`: filters for cards, permanents, events, replacement context, and bindings.
- `formula.rs`: scalar formulas used by costs, DP predicates, auras, and capped selections.
- `compile.rs` and `compiled.rs`: YAML spec to compiled IR.

Runtime lowering into engine effects happens in `code/digimon-engine/src/dsl_cards/`. Tests should usually live under `code/digimon-engine/tests/cards_behavioral/<set>/<card_id>.rs`.

## 3. YAML Shape

Most card files follow this shape:

```yaml
card: BT24-031
name: Elecmon
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
traits: [Beast, Olympos XII, Iliad, TS]

alt_paths:
  - kind: digivolve
    from: { level_eq: 3, trait_has: TS }
    cost: 2

effects:
  - when: on_play
    summary: "Reveal 3 and add matching cards"
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          no_duplicate_cards: true
          buckets:
            - bind_as: iliad_pick
              filter: { trait_has: Iliad }
              max: 1
            - bind_as: ts_pick
              filter: { trait_has: TS }
              max: 1
      - add_to_hand_from_reveal: { of: you, card: iliad_pick }
      - add_to_hand_from_reveal: { of: you, card: ts_pick }
      - place_remainder_on_deck: { of: you, position: bottom }
```

Key rules:

- `effects:` entries are either triggered clauses with `when:` or declarative clauses with `kind:`.
- `process:` is a list of one-key maps, such as `draw:`, `select_hand:`, `delete_permanent:`, or `effect_initiated_digivolve:`.
- Use `bind_as` for anything later steps must reference.
- `optional: true` on selection steps exposes PASS/decline where printed text says "may" or "up to".
- Clause scope (set on the triggered or declarative clause): `face_up` (default), `inherited` for inherited text, `both` for clauses that fire from both top-of-stack and inherited positions, `security` for face-up security cards (e.g. registered triggered effects on a revealed Option), `linked` for linked-card behavior — only when the lowering and tests prove it works for the card.

Other top-level fields used as needed: `form`, `attribute`, `ace_overflow` (negative integer, lowered to an on-leave-field hook), `identity` (X-Antibody aliases, see [`identity.rs`](../code/digimon-dsl/src/identity.rs)), `digixros_aliases`, `dual` (DUAL face metadata), `use_requirement` (Option color-satisfaction predicate), `alt_paths`. See [`spec.rs`](../code/digimon-dsl/src/spec.rs) for the complete schema; the generated JSON schema is in [`tools/dsl-schema-export/`](../code/tools/dsl-schema-export/).

## 4. Clause API

Use triggered clauses for effects that fire at a timing:

```yaml
- when: [on_play, when_digivolving]
  optional: true
  once_per_turn: true
  active_when: { your_turn: true }
  process:
    - gain_memory: 1
```

Timings authored in `when:` (full list in [`clause.rs` `Timing` enum](../code/digimon-dsl/src/clause.rs); they map 1:1 onto engine `EffectTiming`):

- Play / evolution / combat: `on_play`, `when_digivolving`, `when_attacking`, `on_attack`, `on_ally_attack`, `on_opponent_attack`, `end_of_attack`, `end_of_battle`, `on_attack_target_change`.
- Deletion / movement / state: `on_deletion`, `on_any_deletion`, `on_leave_field`, `on_enter_field_anyone`, `on_suspend`, `on_unsuspend`, `on_move`, `on_hatch`.
- Digivolve observers: `on_digivolve`, `on_dna_digivolve`, `on_digixros`, `on_digivolution_card_trashed`, `on_any_digimon_played`, `on_ally_played`.
- Security observers: `on_security`, `on_security_check`, `on_lose_security`, `on_discard_security`, `on_opponent_security_removed`, `on_own_security_removed`, `on_place_security`, `on_added_to_security`, `on_option_placed`.
- Phases and activations: `main`, `start_of_your_turn`, `start_of_opponents_turn`, `start_of_your_main_phase`, `end_of_your_turn`, `end_of_opponents_turn`, `end_of_your_next_turn`, `end_of_opponents_next_turn`, `until_next_unsuspend`, `main_from_hand`, `main_on_field`, `main_from_trash`, `counter`, `before_pay_cost`, `delayed`.

Use declarative clauses for persistent or registered behavior. The full set of `kind:` values is in [`clause.rs` `DeclarativeKind`](../code/digimon-dsl/src/clause.rs):

```yaml
# Static keyword grant.
- kind: grant_keyword
  keyword: Blocker

# Continuous aura. New in Track H: security_attack / security_attack_fn for live
# Security Attack ±N, and while_condition for install-once continuous gates
# (UntilCondition modifier — evicts on first false; does NOT re-arm).
- kind: aura
  target: { trait_has: Royal Knight }
  dp_modifier: 2000
  security_attack: 1
  while_condition:
    any_field_permanent:
      of: opponent
      kind: digimon
      is_unsuspended: false

# Cost reduction. Either flat `amount:` or a dynamic `amount_fn:` formula.
# `pay_cost:` carries an in-place cost ritual (e.g. trash N from hand).
- kind: cost_reduction
  reduction_timing: before_pay_cost
  when_playing_this: true
  condition: { security_count_lte: 3 }
  amount: 5

# Player- or target-level restriction. Use only when action-mask enforcement exists.
- kind: flood_gate
  modifier: CannotPlayDigimonByEffect
  target_player: opponent
  expiry: end_of_opponents_turn

# Static ACE overflow value (negative integer; lowered to on-leave-field hook).
- kind: ace_overflow
  value: -2

# Link Option requirement.
- kind: link_requirement
  cost: 1
  filter: { trait_has: Royal Knight }

# Registers an alt-path entry at a printed trigger (e.g. Burst, Assembly).
- kind: alt_path_registration
  trigger: on_attack
  registers: { ... }

# Declares which sources form this stack's digivolution partition
# (Decode / Partition mechanics — Track C/D).
- kind: partition
  sources:
    - trait_has: Olympos XII
  exclude_cause: [overclock]

# Escape hatch — references a hand-written Rust closure.
# File a DSL-vocab gap when you reach for this.
- kind: raw_rust
  fn: my_card_fallback
  triggers: [on_play]
```

### Replacement framework (Track B, PR #449)

`kind: replacement` is the canonical shape for would-leave / prevention effects. The body has structured fields beyond `process:`: `trigger:` (alias `timing:`), `cost:` (with `delay_self: true` for Delay self-disposition), `choose:` (player picks from hand), `outcome:` (currently `prevent`), and `then:` (typed sub-steps such as `digivolve_without_cost`).

```yaml
# Simple "prevent leaving the field" replacement.
- kind: replacement
  trigger: when_would_leave_battle_area
  optional: true
  once_per_turn: true
  active_when:
    all_of:
      - replacement_subject_is_mine: true
      - trait_has: TS
      - none_of:
          - replacement_cause: own_effect
  process:
    - trash_top_security_and_cancel_replacement: { of: you }

# Replacement that delays itself, picks an Imperialdramon-name card from hand,
# digivolves the subject onto it without cost, and prevents the deletion.
# (BT17-097 Clause B shape — see code/digimon-engine/cards/bt17/BT17-097.yaml.)
- kind: replacement
  trigger: when_would_leave_battle_area
  cost: { delay_self: true }
  choose:
    from: hand
    card_filter: { name_contains: "Imperialdramon" }
    min: 1
    max: 1
  outcome: prevent
  then:
    - digivolve_without_cost:
        target: replacement_subject   # the permanent that would leave
        card: chosen                  # the hand card bound by `choose:`
```

Replacement causes available in `active_when` predicates: `battle`, `own_effect`, `opponent_effect`, `security_check`, `cost`, `overclock` (see `ReplacementCauseSpec` in [`predicate.rs`](../code/digimon-dsl/src/predicate.rs)).

### Track I — Option Plug-In lifecycle (engine-only, no DSL surface yet)

Track I (PRs #461 + #466, 2026-05-10) added the substrate for Option Plug-In lifecycle on the engine side: `EffectTiming::OnOptionTrashed`, `TriggerContext.option_last_field_state`, `OptionFieldState::{LinkedPlugIn, OrphanedPlugIn}`, and `Game::orphan_linked_plug_in` / `orphan_plug_in` / `relink_plug_in` helpers. See [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) §0 "Tracks A–K Substrate Quick Reference" for the engine surface.

**The DSL does not expose these yet.** `on_option_trashed` does not lower to a `when:` value (verified against `code/digimon-engine/src/dsl_cards/timing_map.rs`), and no DSL verb constructs an orphan/relink operation. If a card needs Plug-In lifecycle behavior today, you must reach for `raw_rust:` and call the `EffectContext` helpers directly — and file a `qa/dsl-vocab-gaps.md` entry for the missing DSL surface so it can be planned in. Do not stub the behavior or omit the carrier-loss cascade.

## 5. Step API by Pattern

### Selection

Selections are the heart of agent-safe card scripting. They install a `PendingSelection`, expose legal action IDs, and bind the chosen object.

Use:

- Field permanents: `select_own_permanent`, `select_opponent_permanent`, `select_any_permanent`, `select_own_breeding_permanent`.
- Zones: `select_hand`, `select_trash`, `select_security`, `select_union_zone`.
- Reveal pools: `select_reveal`, `select_reveal_buckets`, `select_ordered_permutation`, `select_count_capped_multi`.
- Stacks / sources: `select_own_sources`, `select_material`, `digi_burst` (Burst-style material picks), `select_dna_pair`.
- Modal / cost shapes: `select_effect_choice` for printed modal choices, `select_opponent_dp_budget` for "delete Digimon up to DP X total".
- `as_selecting_player` to flip the selector to the opponent (e.g. "your opponent chooses").

Example modal branch:

```yaml
- select_effect_choice:
    labels: ["Delete", "Digivolve"]
    bind_as: mode
    prompt: "Choose 1 effect"
- if:
    condition: { equals: [mode, 0] }
    then:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
          prompt: "Choose a Digimon to delete"
      - delete_permanent: { target: target }
```

### Zone Movement

Use these instead of mutating engine internals:

- Hand / deck / trash: `draw`, `trash_from_top`, `add_to_hand_from_deck`, `add_to_hand_from_trash`, `add_to_hand_from_security`, `trash_from_hand_by_index`, `shuffle_deck`, `return_all_trash_to_deck_bottom`, `trash_opponent_hand_to_count`.
- Reveal: `reveal_top_deck`, `add_to_hand_from_reveal`, `return_to_deck_from_reveal`, `trash_from_reveal`, `place_remainder_on_deck`.
- Field: `play_from_hand`, `play_from_hand_free`, `play_from_trash`, `play_from_trash_free`, `play_from_security`, `play_from_materials`, `return_to_hand`, `return_to_deck`, `delete_permanent`, `delete_bound_permanents`, `suspend`, `unsuspend`, `de_digivolve`, `hatch`, `play_token`, `bind_permanent_property` (capture a permanent's static metadata into a binding for later predicate lookups).
- Stack / source: `place_as_bottom_source`, `trash_top_source`, `trash_all_sources`, `select_own_sources`, `trash_selected_sources`, `play_selected_sources_free`, `trash_top_n_digivolution_cards_of_each`.
- Security: `trash_top_security`, `trash_bottom_security`, `add_top_security_to_hand`, `may_add_top_security_to_hand`, `recover`, `place_on_security`, `add_this_option_to_hand`, `search_own_security_stack`, `mark_security_face_up`, `shuffle_security`, `security_place_stacked_card`, `security_place_top_stacked_card`, `bounce_self`, `place_self_at_security`, `place_self_option_at_security`. The `..._and_cancel_replacement` and `..._and_handle_replacement` variants exist for replacement-flow card text (`trash_top_security_and_cancel_replacement`, `place_permanent_on_security_and_handle_replacement`, etc.) — use them only inside a `kind: replacement` clause's `process:`.

Example search pattern:

```yaml
- reveal_top_deck: { of: you, count: 3, bind_as: revealed }
- select_reveal:
    of: you
    bind_as: pick
    filter: { trait_has: LIBERATOR }
    prompt: "Add 1 LIBERATOR card"
    optional: true
- add_to_hand_from_reveal: { of: you, card: pick }
- place_remainder_on_deck: { of: you, position: bottom }
```

Example effect-initiated digivolve:

```yaml
- select_own_permanent:
    bind_as: base
    filter: { kind: digimon }
    prompt: "Choose a Digimon to digivolve"
    optional: true
- select_hand:
    of: you
    bind_as: evo
    filter: { trait_has: LIBERATOR }
    prompt: "Choose a card to digivolve into"
- effect_initiated_digivolve:
    target: base
    from:
      zone: hand
      of: you
      card: evo
    cost: { reduce: 3 }
```

### Modifiers, Auras, and Keywords

Use `grant_keyword` for printed static keywords:

```yaml
- kind: grant_keyword
  keyword: Raid
```

Use `add_dp_modifier`, `add_modifier`, `add_player_modifier`, and `grant_effect_immunity` inside a triggered effect:

```yaml
- add_dp_modifier:
    target: ally
    value: 3000
    expiry: end_of_turn
- grant_effect_immunity:
    target: self
    source_kind: digimon
    source_controller: opponent
    expiry: end_of_opponents_turn
```

Use `grant_triggered_effect` (Track H, PR #467) to install a granted triggered ability on each matching target permanent — DCGO `AddSkillClass.cs` analog. The granted body fires on the carrier's matching `timing:` and carries an `expiry:`. Source attribution remains the grantor for "by [card]" checks; carrier semantics apply for "this Digimon" reads. EX1-068 Ice Wall! is the canonical fixture.

Use `kind: aura` or `kind: flood_gate` only when the effect is continuous. Aura supports `dp_modifier` / `dp_modifier_fn`, the new `security_attack` / `security_attack_fn` (Track H §1), `grant_keyword`, named `modifier:` grants, and `while_condition:` for install-once UntilCondition gates. Mask-affecting keywords and restrictions must be enforced by the engine mask and decoder, not just represented in YAML.

### Combat

Use `battle:` for card text that says one Digimon battles another by effect. This is not an attack and must not trigger Piercing/security continuation.

```yaml
- battle:
    attacker: this
    defender: battle_target
```

Use `may_attack_now` for text that says a Digimon attacks or may attack. It preserves attack restrictions, optional decline, and normal combat windows.

`force_attack` (PR #450) is for cards that compel an attack without the may/optional shape. Both `may_attack_now` and `force_attack` route through the centralized attack flow so security checks, Piercing, Counter windows, and replacement hooks all fire as printed.

Other combat-flow process outcomes (use these only when card text says so — they are not freely composable):

- `redirect_attack_target` — re-route an in-flight attack to a different defender (e.g. Decoy / Defense Training).
- `cancel_attack` — abort the attack before damage / security.
- `open_counter_window` — explicit Counter timing window.
- `end_attack: true|false` — conclude the attack flow (`true` to skip remaining security checks).
- `refire_effect` (PR #463) — re-fire a queued cross-card effect when valid effects remain (BT24-102 Homeros pattern). The lower-level `EffectContext::refire_target_effect` is documented in [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md).

### Replacement-flow outcomes

These step verbs are only legal inside a `kind: replacement` clause's `process:`:

- `cancel_replacement` — fully prevent the would-leave/zone-move.
- `handle_replacement` — accept the outcome (no-op terminator).
- `redirect_replacement` — redirect to an alternate subject (e.g. Decoy redirects deletion).
- `substitute_replacement` — substitute a different card for the subject (e.g. Fragment).

### Control flow

`if` (with `condition` / `then` / `else`), `for_each`, `per_selected`, `optional`, `schedule_delayed` (delayed sub-process), `place_self_as_delay_option` (resolve this Option onto field as a Delay), and `link_to_own_digimon` (Link Option attachment). See [`step.rs`](../code/digimon-dsl/src/step.rs) for argument shapes.

## 6. Predicate and Formula API

Predicates are AND-joined by default:

```yaml
filter:
  kind: digimon
  trait_has: Reptile
  dp_lte: 4000
```

Use compounds for alternatives:

```yaml
filter:
  any_of:
    - trait_has: Mineral
    - trait_has: Rock
```

Common predicate families (full list in [`predicate.rs`](../code/digimon-dsl/src/predicate.rs)):

- Identity: `kind`, `level_eq`, `level_eq_binding`, `level_lte`, `level_gte`, `color_is`, `color_only`, `color_matches_any_field_digimon`, `color_matches_binding`, `trait_has`, `form_is`, `attribute_is`, `name_is`, `name_contains`, `name_in`, `card_number_is`, `play_cost_lte`, `can_digivolve_from_source`.
- Permanent state: `dp_eq`, `dp_lte`, `dp_gte`, `is_suspended`, `is_unsuspended`, `materials_count_lte`, `materials_count_gte`, `stack_size_lte`, `stack_size_gte`, `has_keyword`, `has_inherited` (nested predicate), `of_permanent`.
- Source-relative (resolves against `ctx.source_card` / `ctx.source_permanent`): `source_is_tamer`, `source_name_contains`, `source_permanent_trait_has`, `source_is_unsuspended` (PR #472 — checks the source permanent's suspension state from a triggered clause), `self_digivolution_contains_name`.
- Context / global: `your_turn`, `opponents_turn`, `all_turns`, `memory_lte`, `memory_gte`, `security_count_lte`, `security_count_gte`, `can_hatch`, `in_breeding`, `on_field`, `dna_origin`.
- Event payloads (PR #451 event-payload contract — only valid inside event-driven triggers): `event_target_kind`, `event_target_trait_has`, `event_target_owner`, `event_target_is_player`, `event_target_was_self`, `event_permanent_is_source`, `event_is_effect_initiated`, `event_card_trait_has`, `event_card_name_contains`, `event_cause`, `attacker_trait_has`, `attack_target_change_reason`, `host_permanent_trait_has`, `trashed_source_trait_has`, `trashed_source_card_id_is`.
- Replacement payloads (Track B, only valid inside `kind: replacement` clauses): `replacement_cause`, `replacement_source_is_opponent`, `replacement_subject_is_mine`.
- Bindings: `not_in_binding`, `binding_owner`, `binding_exists`, `binding_present`, `binding_absent`, `equals`, `not_equals`.
- Effect-history rollups (used to gate follow-on clauses, e.g. "if you returned a card by this effect"): `effect_suspended_any_own_digimon`, `effect_returned_any_card`, `effect_deleted_any_own_digimon`, `effect_deleted_any_opponent_digimon`, `effect_played_any_digimon`, `effect_digivolved_any_digimon`, `effect_added_any_card_to_hand`.
- Aggregates / existentials: `level_matches_aggregate`, `count_lte`, `count_gte`, `any_permanent`, `any_field_permanent`, `no_permanent`, `all_permanents`, `has_alt_path`.

### Formula-valued thresholds (PR #470)

Numeric threshold predicates accept either a literal `i32` or a `{ formula: ... }` block. The fields that resolve through the `DpConstraint` parser are: `level_lte`, `level_gte`, `dp_eq`, `dp_lte`, `dp_gte`, `play_cost_lte`, `stack_size_lte`, `stack_size_gte`, `materials_count_lte`, `materials_count_gte`, `memory_lte`, `memory_gte`, `security_count_lte`, `security_count_gte`, `CountAggregate.n`. Both literal and formula forms parse:

```yaml
# Literal threshold.
filter: { dp_lte: 5000 }

# Formula threshold — "DP ≤ (7000 + 2000 per 10 cards in shared trash)".
filter:
  dp_lte:
    formula:
      base: 7000
      per:
        shared_trash_count: { bucket: 10 }
      delta: 2000
```

This is what makes selections like "delete opponent Digimon with DP ≤ (shared trash count × 1000)" expressible without hand-written Rust (BT15-096, BT21-102 are the canonical fixtures).

### Formula vocabulary

`FormulaSpec` (in [`formula.rs`](../code/digimon-dsl/src/formula.rs)) supports:

- `Literal` — bare `i32`.
- `BasePerDelta` — `{ base, per, bucket?, delta }`.
- `binding_dp: <name>` / `binding_play_cost: <name>` — read DP / printed cost from a bound permanent or card.
- `source_stack_dp_sum: { target, filter? }` — sum DP across a permanent's digivolution stack.
- Compound forms: `floor_div: [a, b]`, `max: [a, b, …]`, `min: [a, b, …]`, `raw_rust: "<fn_name>"`.
- `aggregate:` — either bare `AggregateSelector` (lowest_dp / highest_dp / lowest_level / highest_level) or `{ selector, scope: <PlayerRef> }` for cross-player scoping.

`PerSelector` variants used inside `BasePerDelta.per`: `material_count`, `stack_size`, `ally_count`, `suspended_count: { of }`, `digivolution_color_count`, `same_level_pairs_in_sources`, `shared_trash_count: { bucket? }`, `card_count_in_zone: { zone, of, filter? }`, `distinct_colors_count: { zone, of, filter? }`.

Examples:

```yaml
amount_fn:
  base: 0
  per:
    card_count_in_zone:
      zone: battle_area
      of: opponent
      filter: { kind: digimon }
  delta: -1
```

```yaml
amount_fn:
  source_stack_dp_sum:
    target: self
    filter: { trait_has: Iliad }
```

```yaml
amount_fn:
  floor_div:
    - { base: 0, per: ally_count, delta: 1 }
    - 2
```

When a predicate or formula parses but does not have proven runtime enforcement, keep the card test ignored or marked partial and file a gap. A parsed YAML field is not proof of faithful behavior.

## 7. Common Archetype Patterns

These patterns recur across the archetype DSL gap files and should guide implementation choices.

### Reveal and Search

Used by Medusamon, TS Olympos, Red Hybrid, DNA Omnimon, Rocks, and generic rookies.

Preferred shape:

- `reveal_top_deck`
- `select_reveal` or `select_reveal_buckets`
- `add_to_hand_from_reveal`
- `place_remainder_on_deck`

Use `select_reveal_buckets` when one reveal pool feeds multiple printed categories and the same revealed card cannot satisfy both categories.

### Source Stack Costs and Rocks-Style Source Selection

Used heavily by Rocks and by Decode/Partition-like mechanics.

Preferred shape:

- `select_own_sources` for cross-permanent source picks.
- `select_material` for a single known stack.
- `trash_selected_sources`, `play_selected_sources_free`, or `place_as_bottom_source` for the selected refs.
- Event predicates such as `trashed_source_trait_has` and `host_permanent_trait_has` for observers.

Never scan trash after the fact to infer which source was trashed. The event payload must carry the exact source and host.

### Event Observers

Used by Medusamon security-removed effects, Rocks source-trash observers, Royal Knights breeding/option observers, Puppets event-gated Delay, and hand-resident DNA Omnimon observers.

Preferred shape:

- Use the exact timing token: `on_opponent_security_removed`, `on_digivolve`, `on_move`, `on_digivolution_card_trashed`, `on_option_placed`, `on_any_deletion`.
- Gate with event predicates, not board-state guesses.
- Bind `target: event_target` only when tests prove the lowering maps to the event subject instead of the observer.

### Replacement and Prevention

Used by TS Olympos, Puppets, BG Imperial, DNA Omnimon, Armor Purge, Fragment, Scapegoat, Decode, and Partition.

Preferred shape:

- `kind: replacement`
- `active_when` over replacement subject, source, and cause.
- Cost selections inside `process` or cost-specific bodies.
- `cancel_replacement`, `redirect_replacement`, `substitute_replacement`, or a specialized canceling step.

Do not implement prevention by undoing a zone move after it happened.

### Options, Delay, Plug-In, Link, and Training

Used by Scrambles, Memory Boosts, Training cards, Royal Knights options, Puppets event-gated Delay, Link/Plug-In cards, and security self-disposition.

Preferred shape:

- `kind: delay` for Delay effects with explicit `trigger` and `process`.
- `place_self_as_delay_option` when the main effect places the resolving Option.
- `add_this_option_to_hand` when a security Option moves itself to hand.
- `kind: flood_gate` for color/use-requirement bypasses only when action-mask enforcement exists.

If the card needs linked-card scope, re-linking, or Plug-In-specific state and the lowering is not proven, mark it as a DSL or engine gap.

### Effect Digivolve, DNA, and Alt Paths

Used by DNA Omnimon, BG Imperial, Royal Knights, Chaos Control, Red Hybrid, and Training/Scramble cards.

Preferred shape:

- `alt_paths` for printed alternate digivolution, DNA, DigiXros, Assembly, Burst, or activated digivolve entry paths.
- `effect_initiated_digivolve` for card effects that choose a base and an evolution card.
- `effect_initiated_dna_digivolve` only where tests prove the material sources match the printed card.
- `select_dna_pair` for player-visible material choices.

Do not fake an effect digivolve with raw stack mutation. It must fire the standard digivolve events and When Digivolving queue.

**Activation gate on an alt-path (Phase 1, 2026-05-15).** When a printed alt-path is conditional on game state beyond the source filter and extra-cost ritual — e.g. "[Hand] [Main] **If you have [Owen Dreadnought]**, by placing 1 [Dimetromon] …, [Elizamon] digivolves into this card" — use `condition:` on the alt-path. It evaluates after the source filter and before the extra-cost flow, with the source permanent as `PredicateSubject`. Currently consumed by the Digivolve route only (not yet by DigiXros / BurstDigivolve / Assembly / AppFusion — file a gap entry if you need them).

```yaml
alt_paths:
  - kind: activated_digivolve
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Owen Dreadnought"
    from: { name_contains: "Elizamon" }
    cost: 3
    ignore_requirements: true
    extra_cost:
      - select_trash:
          of: you
          bind_as: source
          filter: { name_contains: "Dimetromon" }
      - place_as_bottom_source:
          source: source
          target: { binding: activated_digivolve_target }
```

### Auras, Flood Gates, and Keywords

Used by nearly every archetype.

Preferred shape:

- `kind: grant_keyword` for printed keywords.
- `kind: aura` for continuous DP, Security Attack, keyword, or modifier effects.
- `kind: flood_gate` for player or target restrictions.
- Formula fields for dynamic values.

Every legality-changing aura or keyword must be checked in masks and execution validation.

### Tokens and Card Data

Used by Puppets, Medusamon, and many archetypes that create named tokens.

Preferred shape:

- Tokens should be real card definitions with `CardData` and effects.
- `play_token` should reference a registered token name.
- Token On Deletion behavior belongs on the token card, not in every creator.

## 8. Gap Filing Rules

Use capability-centric language:

- Good: "Cross-permanent source selection with exact/up-to-N counts."
- Bad: "Implement EX10-032."

File the gap in the right place:

- `docs/RUST_ENGINE_GAPS.md`: engine cannot faithfully expose or execute the behavior.
- `qa/dsl-vocab-gaps.md`: engine primitive exists, but YAML cannot express or lower it.
- `qa/archetype-qa/engine-gaps.md`: legacy/Python-lane or cross-reference gap surfaced by QA.
- Per-archetype docs under `qa/archetype-qa/dsl/`: source documents and readiness notes.

Each gap should include:

- Printed effect text.
- Missing reusable primitive.
- Existing engine API it should lower to, if any.
- Suggested YAML shape.
- First card/archetype that surfaced it.
- Current status: open, partial, resolved, or covered by existing primitive.

## 9. Testing Checklist

For each DSL card implementation:

- Parser/compile coverage: YAML parses and compiles through the embedded pack.
- Structural assertions: clause count, timing, scope, optionality, OPT flags.
- Positive behavior: the printed effect changes the game state as expected.
- Negative behavior: conditions, filters, and absence of legal targets suppress prompts.
- Mask behavior: player-visible choices appear as pending selections and illegal choices are absent.
- Optionality: PASS/decline is legal when printed text says "may" or "up to".
- Expiry: modifiers clear at the printed timing.
- Event payload: observer effects use the actual event subject/card/source, not a board scan.
- Tracker hygiene: any remaining omitted clause is documented as a precise gap.

Use targeted commands while iterating:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- <pattern> --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_id_or_module> --nocapture
```

Run broader suites when shared DSL, action-mask, selection, or engine primitives change.

## 10. Red Flags

Stop and file or resolve a gap when you see:

- A `raw_rust` function that only hides a missing reusable DSL verb.
- A `process: []` placeholder on production YAML.
- A selection where the printed text gives the player a choice but YAML auto-picks.
- A predicate that parses but has no runtime test proving it filters the mask.
- A clause that mutates `ctx.game` internals instead of using `EffectContext` or DSL lowering.
- A card marked ready while some printed text is omitted because "it rarely matters."
- A change that would require new action IDs or tensor fields but does not update `ACTION_SPEC.md`, `TENSOR_SPEC.md`, PyO3 exports, RL wrappers, and frontend constants together.

Faithful DSL work is slower than stubbing, but it leaves the engine teachable. That matters for both human gameplay and RL agents.
