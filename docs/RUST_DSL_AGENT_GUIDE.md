# Rust DSL Agent Implementation Guide

**Audience:** AI agents and humans implementing Digimon card effects through the Rust YAML DSL in `code/digimon-engine/cards/`.

This guide is the practical authoring companion to:

- [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) for the hand-written Rust `EffectContext` API.
- [`RUST_DSL_TEST_API.md`](RUST_DSL_TEST_API.md) for Rust tests around DSL-authored cards.
- [`RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md), [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md), and [`qa/archetype-qa/engine-gaps.md`](../qa/archetype-qa/engine-gaps.md) for reusable blockers found by archetype audits.
- [`docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`](superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md) for the capability-first roadmap.

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
- `scope: inherited` is for inherited text. `scope: linked` is reserved for linked-card behavior and should only be used when the lowering and tests prove it works for the card.

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

Common timings include:

- Play/evolution/combat: `on_play`, `when_digivolving`, `when_attacking`, `end_of_attack`, `end_of_battle`, `on_attack_target_change`.
- Events: `on_suspend`, `on_unsuspend`, `on_move`, `on_digivolve`, `on_dna_digivolve`, `on_digivolution_card_trashed`, `on_opponent_security_removed`, `on_any_deletion`, `on_ally_played`.
- Phases and activations: `start_of_your_turn`, `start_of_your_main_phase`, `end_of_your_turn`, `end_of_opponents_turn`, `main_from_hand`, `main_on_field`, `main_from_trash`, `counter`, `on_security`.

Use declarative clauses for persistent or registered behavior:

```yaml
- kind: grant_keyword
  keyword: Blocker

- kind: aura
  target: { trait_has: Royal Knight }
  dp_modifier: 2000

- kind: cost_reduction
  reduction_timing: before_pay_cost
  when_playing_this: true
  condition: { security_count_lte: 3 }
  amount: 5

- kind: flood_gate
  modifier: CannotPlayDigimonByEffect
  target_player: opponent
  expiry: end_of_opponents_turn
```

Use `kind: replacement` for would-leave/prevention effects:

```yaml
- kind: replacement
  timing: when_would_leave_battle_area
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
```

## 5. Step API by Pattern

### Selection

Selections are the heart of agent-safe card scripting. They install a `PendingSelection`, expose legal action IDs, and bind the chosen object.

Use:

- `select_own_permanent`, `select_opponent_permanent`, `select_any_permanent`.
- `select_hand`, `select_trash`, `select_union_zone`.
- `select_reveal`, `select_reveal_buckets`, `select_ordered_permutation`, `select_count_capped_multi`.
- `select_own_sources`, `select_material`, `select_security`, `select_dna_pair`.
- `select_effect_choice` for printed modal choices.
- `as_selecting_player` when the opponent makes the choice.

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

- Hand/deck/trash: `draw`, `trash_from_top`, `select_trash`, `add_to_hand_from_trash`, `trash_from_hand_by_index`.
- Reveal: `reveal_top_deck`, `add_to_hand_from_reveal`, `return_to_deck_from_reveal`, `trash_from_reveal`, `place_remainder_on_deck`.
- Field: `play_from_hand`, `play_from_hand_free`, `play_from_trash`, `play_from_trash_free`, `play_from_security`, `return_to_hand`, `return_to_deck`, `delete_permanent`.
- Stack/source: `place_as_bottom_source`, `trash_top_source`, `trash_all_sources`, `select_own_sources`, `trash_selected_sources`, `play_selected_sources_free`, `play_from_materials`.
- Security: `trash_top_security`, `add_top_security_to_hand`, `may_add_top_security_to_hand`, `recover`, `place_on_security`, `add_this_option_to_hand`.

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

Use `add_dp_modifier`, `add_modifier`, and `grant_effect_immunity` inside a triggered effect:

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

Use `kind: aura` or `kind: flood_gate` only when the effect is continuous. Mask-affecting keywords and restrictions must be enforced by the engine mask and decoder, not just represented in YAML.

### Combat

Use `battle:` for card text that says one Digimon battles another by effect. This is not an attack and must not trigger Piercing/security continuation.

```yaml
- battle:
    attacker: this
    defender: battle_target
```

Use `may_attack_now` only for text that says a Digimon attacks or may attack. It must preserve attack restrictions, optional decline, and normal combat windows.

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

Common predicate families:

- Identity: `kind`, `level_eq`, `level_lte`, `color_is`, `color_only`, `trait_has`, `name_is`, `name_contains`, `card_number_is`, `play_cost_lte`.
- Permanent state: `dp_lte`, `dp_gte`, `is_suspended`, `is_unsuspended`, `materials_count_lte`, `stack_size_gte`, `has_keyword`.
- Context: `your_turn`, `opponents_turn`, `all_turns`, `security_count_lte`, `memory_gte`.
- Event payloads: `event_target_trait_has`, `event_target_owner`, `event_card_trait_has`, `event_card_name_contains`, `host_permanent_trait_has`, `trashed_source_trait_has`.
- Replacement payloads: `replacement_cause`, `replacement_source_is_opponent`, `replacement_subject_is_mine`.
- Bindings: `not_in_binding`, `equals`, `not_equals`.
- Aggregates: `level_matches_aggregate`, `count_lte`, `count_gte`, `any_permanent`, `no_permanent`.

Formula examples:

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
dp_lte:
  base: 7000
  per:
    shared_trash_count:
      bucket: 10
  delta: 2000
```

```yaml
amount_fn:
  source_stack_dp_sum:
    target: self
    filter: { trait_has: Iliad }
```

When a predicate parses but does not have proven runtime enforcement, keep the card test ignored or marked partial and file a gap. A parsed YAML field is not proof of faithful behavior.

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
