# Rust DSL Agent Implementation Guide

**Audience:** AI agents and humans implementing Digimon card effects through the Rust YAML DSL in `code/digimon-engine/cards/`.

This guide is the practical authoring companion to:

- [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) for the hand-written Rust `EffectContext` API.
- [`RUST_DSL_TEST_API.md`](RUST_DSL_TEST_API.md) for Rust tests around DSL-authored cards.
- [`RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md), [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md), and [`qa/archetype-qa/engine-gaps.md`](../qa/archetype-qa/engine-gaps.md) for reusable blockers found by archetype audits.
- [`docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`](superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md) for the capability-first roadmap.

**Reference completeness is generated, not hand-maintained.** The exhaustive, always-current index of every step verb, predicate, timing, and declarative kind lives in the **[DSL Vocabulary Reference](#dsl-vocabulary-reference-generated)** at the end of this file — generated directly from the `digimon-dsl` enums by [`code/tools/dsl-doc-export/`](../code/tools/dsl-doc-export/) and kept in sync by a CI drift gate (`.github/workflows/dsl-vocab-doc-drift.yml`, the rule-27 codegen pattern). The hand-written sections below (§1–§10) are the *curated* layer: workflow, idioms, nuance, and red flags that judgment requires and the enums can't encode. When the narrative and the generated reference ever disagree, the generated reference is authoritative for *what exists*; the narrative is authoritative for *how and why*. See [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) §0 "Tracks A–K Substrate Quick Reference" for the engine-side substrate index and [`RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md) for the live gap state.

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

Timings authored in `when:` map 1:1 onto engine `EffectTiming`. The **complete, current list with usage and docs is in the generated [Timings table](#timings-when)**; the groupings below orient you to the common ones:

- Play / evolution / combat: `on_play`, `when_digivolving`, `when_attacking`, `on_attack`, `on_block`, `on_ally_attack`, `on_opponent_attack`, `end_of_attack`, `end_of_battle`, `on_attack_target_change`.
- Deletion / movement / state: `on_deletion`, `on_any_deletion`, `on_leave_field`, `on_enter_field_anyone`, `on_suspend`, `on_unsuspend`, `on_move`, `on_hatch`, `on_add_to_hand`.
- Digivolve observers: `on_digivolve`, `on_dna_digivolve`, `on_digixros`, `on_digivolution_card_trashed`, `on_any_digimon_played`, `on_ally_played`.
- Security observers: `on_security`, `on_security_check`, `on_check_face_up_security`, `on_lose_security`, `on_discard_security`, `on_opponent_security_removed`, `on_own_security_removed`, `on_place_security`, `on_added_to_security`, `on_option_placed`, `on_option_trashed`.
- Link (DigiLink / Appmon): `on_any_link`, `when_linked`, `when_card_linked_to_this`, `when_would_link_to_this` — use on a `scope: linked` effect; see the generated table for the exact lowering of each.
- Phases and activations: `main`, `start_of_your_turn`, `start_of_opponents_turn`, `start_of_your_main_phase`, `end_of_your_turn`, `end_of_opponents_turn`, `end_of_your_next_turn`, `end_of_opponents_next_turn`, `until_next_unsuspend`, `main_from_hand`, `main_on_field`, `main_from_trash`, `counter`, `before_pay_cost`, `before_pay_cost_observe`, `delayed`.

Use declarative clauses for persistent or registered behavior. The full set of `kind:` values is in the generated [Declarative kinds table](#declarative-kinds-kind) — including `link_condition` (a DigiLink gating clause alongside `link_requirement`). Common ones:

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

### Track I — Option Plug-In lifecycle

Track I (PRs #461 + #466, 2026-05-10) added the substrate for Option Plug-In lifecycle on the engine side: `EffectTiming::OnOptionTrashed`, `TriggerContext.option_last_field_state`, `OptionFieldState::{LinkedPlugIn, OrphanedPlugIn}`, and `Game::orphan_linked_plug_in` / `orphan_plug_in` / `relink_plug_in` helpers. See [`RUST_ENGINE_API.md`](RUST_ENGINE_API.md) §0 "Tracks A–K Substrate Quick Reference" for the engine surface.

`when: on_option_trashed` now lowers to `EffectTiming::OnOptionTrashed` (2026-05-23, `complete-rocks-archetype`) and can be used for ordinary battle-area Option trash observers. The remaining DSL gap in this area is Plug-In lifecycle construction: no DSL verb constructs an orphan/relink operation. If a card needs Plug-In orphan/relink behavior today, use a planned substrate change rather than stubbing the behavior or omitting the carrier-loss cascade.

## 5. Step API by Pattern

> **The complete, current step-verb list is the generated [Step verbs table](#step-verbs-process--extra_cost)**, grouped by family, with arg shape, usage count, a fixture card, and a `tag` flagging `unused`/`rare` vocabulary. This section is the *curated* layer: it teaches the recurring patterns and the per-verb nuance the table can't, foregrounds the verbs cards actually use, and is deliberately **not** exhaustive. Reach for a verb the table marks `unused` only with a reason — prefer the live idioms shown here.
>
> **Linking:** author with `link_cards` (`from`, `filter`, `to: self|own_digimon`, `count: { up_to|exactly: N }`, `cost: free`, optional `bind_as` to capture the linked card(s) for an `if { binding_present }` gate). The old single-card `link_card_to_self` verb was removed in `collapse-dsl-step-idioms` §4 — all its cards migrated to `link_cards`.

### Selection

Selections are the heart of agent-safe card scripting. They install a `PendingSelection`, expose legal action IDs, and bind the chosen object.

Use:

- Field permanents: `select_own_permanent`, `select_opponent_permanent`, `select_any_permanent`, `select_own_breeding_permanent`.
- Field permanent selectors support `selector:` filters such as `lowest_dp`, `highest_dp`, `lowest_play_cost`, and `highest_play_cost` to constrain the legal target set before the player chooses among ties.
- Zones: `select_hand`, `select_trash`, `select_security`, `select_union_zone`. A `select_union_zone` pick (hand ∪ trash ∪ material) records the **origin zone** of the chosen card in its `bind_as` binding, not just the card handle. Pair it with `play_union_bound_free: { binding: <name>, bind_as: <opt> }` to replay that card for free from its true zone (`play_from_hand_free` for hand, `play_from_trash` Free for trash, `play_from_materials` Free for material under the source permanent). If the `select_union_zone` is `optional`, declining still runs all mandatory tail steps.
- Reveal pools: `select_reveal`, `select_reveal_buckets`, `select_ordered_permutation`, `select_count_capped_multi`. Pair a reveal selection with `play_from_revealed_free: { of, card }` when printed text plays a chosen revealed card without paying its cost; the step consumes the reveal-pool card directly and does not fire add-to-hand observers.
- Stacks / sources: `select_own_sources`, `select_opponent_sources` (opponent-side mirror of `select_own_sources` — player-visible exact-N / up-to-N pick of digivolution-source cards across the **opponent's** battle-area stacks; same `min` / `max`, PASS-after-min, optional `filter:`, optional `target:` to restrict to one opponent permanent binding, stable cross-permanent refs, and `then:` tail; drives BT16-085's DNA branch "trash any 3 digivolution cards under your opponent's Digimon"), `select_material`, `digi_burst` (Burst-style material picks), `select_dna_pair`.
- Modal / cost shapes: `select_effect_choice` for printed modal choices, `select_opponent_dp_budget` for "delete Digimon up to DP X total". `dp_budget` accepts a literal or a formula such as `{ source_dp: {} }` for "this Digimon's DP"; `min_picks: 1` hides PASS until at least one target is selected.
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

- Hand / deck / trash: `draw`, `trash_from_top`, `add_to_hand_from_deck`, `add_to_hand_from_trash`, `add_to_hand_from_security`, `trash_from_hand_by_index`, `shuffle_deck`, `return_all_trash_to_deck_bottom`, `return_trash_list_to_deck_bottom`, `move_trash_card_to_deck_top` (`{ of, card }` — move a single `select_trash`-bound card to the **top** of its owner's deck; `of` names whose trash currently holds it, the card always returns to its owner's deck; single-card deck-TOP analog of `return_trash_list_to_deck_bottom`; drives LM-030's Delay clause), `trash_opponent_hand_to_count`.
- Reveal: `reveal_top_deck`, `add_to_hand_from_reveal`, `return_to_deck_from_reveal`, `trash_from_reveal`, `place_remainder_on_deck`.
- Field: `play_from_hand`, `play_from_hand_free`, `play_from_revealed_free`, `play_from_trash`, `play_from_trash_free`, `play_union_bound_free` (play a `select_union_zone`-bound card for free from its true origin zone — see Selection note below), `play_from_security`, `play_from_materials`, `return_to_hand`, `return_to_deck`, `delete_permanent`, `delete_bound_permanents`, `schedule_delete_played_at_turn_end` (schedule a `bind_as` permanent for deletion at this turn's end — see Control flow below), `suspend`, `unsuspend`, `de_digivolve`, `hatch`, `play_token`, `bind_permanent_property` (capture a permanent's static metadata into a binding for later predicate lookups).
- Stack / source: `place_as_bottom_source`, `trash_top_source`, `trash_all_sources`, `select_own_sources`, `select_opponent_sources`, `trash_selected_sources`, `return_selected_sources_to_hand` (`{ source_refs }` — mirror of `trash_selected_sources` that returns each `select_own_sources`-bound digivolution-source card to its **owner's** hand instead of trash; fires no `OnDigivolutionCardTrashed` since it is a return; drives BT12-031's "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's digivolution cards to its owner's hand" alt-cost), `play_selected_sources_free`, `trash_top_n_digivolution_cards_of_each`.
- Security: `trash_top_security`, `trash_bottom_security`, `add_top_security_to_hand`, `may_add_top_security_to_hand`, `recover`, `place_on_security`, `add_this_option_to_hand`, `search_own_security_stack`, `mark_security_face_up`, `shuffle_security`, `security_place_stacked_card`, `security_place_top_stacked_card`, `bounce_self`. **`place_on_security` is the single source-polymorphic placement verb** (collapse §3): `source: { card: … } | { permanent: … } | self | self_option`, `position: top|bottom|choice`, `face_up`, plus — inside a `kind: replacement` body — a `disposition: cancel|handle|observed` (with `include_sources` for `observed`). It folds in the former `place_self_at_security` / `place_self_option_at_security` / `place_permanent_on_security[_and_handle_replacement | _bottom_…_and_cancel_replacement | _observed]` verbs. The standalone non-placement replacement verb `trash_top_security_and_cancel_replacement` remains for replacement-flow card text — use it only inside a `kind: replacement` clause's `process:`.

`play_from_trash_free` accepts an optional `suppress_on_play: true` flag (default `false`). When set, the played Digimon's own `[On Play]` effects do **not** activate for that play event — for card text like "Any [On Play] effects on Digimon played with this effect don't activate." (BT5-106 Demonic Disaster). The suppression is scoped strictly to the just-played permanent and that single play: other permanents' On Play, and every other timing (`OnEnterFieldAnyone` / `OnAllyPlayed`), still fire normally. The flag is honored only by `play_from_trash_free`; setting it on `play_from_hand` / `play_from_trash` is a compile error.

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

The plain "reveal → add 1..N matching → bottom the rest" searcher above also has a
one-step shorthand — **`reveal_search { of, count, buckets: [{ filter, to: hand|trash|deck, max, optional }], remainder: top|bottom|choose }`** (collapse §2). It expands at runtime to exactly the reveal → bucketed-select → per-bucket move → place-remainder sequence (parks/resumes identically), so prefer it for ordinary searchers; drop to the longhand only when a later step needs the named bucket bindings.

The field/zone select verbs (`select_hand` / `select_trash` / `select_own_permanent` / `select_opponent_permanent` / …) also accept an inline **`then: [ … ]`** action-tail (collapse §1) that runs scoped to the pick — e.g. `select_trash: { …, then: [ { add_to_hand_from_trash: { card: <bind> } } ] }`. It is the data-driven equivalent of a separate following step (it is the cloneable VM's `ResumeFrame::RunTail`); reach for it when the follow-up is small and pick-scoped.

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

Use `grant_narrow_opponent_effect_protection` for the narrow "can't have its DP reduced **by your opponent's effects** and isn't affected by ＜De-Digivolve＞ effects" protection bundle (BT16-055 Namakemon). It installs two genuinely opponent-effect-scoped modifiers: an `ImmuneFromDPMinus` with an opponent-only filter (only negative `ChangeDp` deltas from an opponent effect are suppressed — the controller's own DP-reduction still applies) and a `CannotBeDeDigivolved` via the passive-replacement route (`OpponentEffect` cause filter — own-side De-Digivolve still applies). Do NOT hand-roll this with `add_modifier`: a plain `add_modifier: { modifier: ImmuneFromDPMinus }` / `{ modifier: CannotBeDeDigivolved }` installs the *broad* unscoped variant and over-protects against the controller's own effects.

```yaml
- grant_narrow_opponent_effect_protection:
    target: ally
    expiry: end_of_opponents_turn
```

Use `grant_triggered_effect` (Track H, PR #467) to install a granted triggered ability on matching target permanents — DCGO `AddSkillClass.cs` analog. `target:` accepts either a predicate filter, which grants to every battle-area match, or a binding ref from a prior selection, which grants to exactly that selected permanent. The granted body fires on the carrier's matching `timing:` and carries an `expiry:`. Source attribution remains the grantor for "by [card]" checks; use `carrier` in the body for "this Digimon" reads. EX1-068 Ice Wall! is the broadcast fixture; EX10-034 is the selected-binding fixture.

Use `arm_digivolve_cost_reducer` for `[Main]` cost-reduction text that arms a future digivolve and has no field permanent to host it (an Option that resolves and trashes itself). It installs a **player-scoped**, turn-scoped ("For the turn") future-digivolve cost reducer; at the next qualifying digivolution the player is offered an accept/decline prompt, and on accept an optional `suspend_cost` prompts them to suspend 1 of their own Digimon — both surfaced through `pending_selection`. Fields: `amount` (required `i32` reduction), `single_fire` (default `false` — fire exactly once for "would next digivolve"), `target_color` (optional — gate to digivolutions whose top card includes this color; omit for any color), `suspend_cost` (default `false`). Drives BT3-103 Hidden Potential Discovered!:

```yaml
- arm_digivolve_cost_reducer:
    amount: 5
    single_fire: true
    target_color: green
    suspend_cost: true
```

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

`if` (with `condition` / `then` / `else`), `for_each`, `per_selected`, `optional`, `schedule_delayed` (delayed sub-process), `schedule_delete_played_at_turn_end`, `place_self_as_delay_option` (resolve this Option onto field as a Delay), and `link_to_own_digimon` (Link Option attachment). See [`step.rs`](../code/digimon-dsl/src/step.rs) for argument shapes.

`schedule_delete_played_at_turn_end: { binding: <name> }` — for card text "At turn end, delete the Digimon this effect played." `binding` must name a `bind_as` from a preceding free-play step (`play_union_bound_free`, `play_from_hand_free`, `play_token`, …). The deletion is keyed to the played permanent's stable provenance identity, so it hits the right permanent even after battle-area indices shift, and is a silent no-op if that permanent already left the battle area (or the optional play was declined). Drained in `end_turn` after the `EndOfYourTurn` observers, as the controller's own effect. Canonical fixtures: EX11-022 Karakurumon, EX11-061 Mirai Kinosaki.

Optional `at` field selects the turn boundary:
- `at: your_turn` (default, matches the behaviour above — omit for EX11-022 / EX11-061 style).
- `at: opponents_turn` — deletion fires at the end of the **opponent's** turn instead (`rotate_turn_player` drain, after `EndOfOpponentsTurn` observers). Used for card text "At the end of your opponent's turn, delete that token." Canonical fixture: P-165 ShoeShoemon.

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

The complete predicate list (145, with arg shapes, usage, and docs) is the generated [Predicates table](#predicates-filter--condition--active_when). The families below highlight the ones with non-obvious semantics worth reading before you reach for them:

- Identity: `kind`, `level_eq`, `level_eq_binding`, `level_lte`, `level_gte`, `color_is`, `color_only`, `color_matches_any_field_digimon`, `color_matches_binding`, `trait_has`, `form_is`, `attribute_is`, `name_is`, `name_contains`, `name_in`, `card_number_is`, `play_cost_lte`, `can_digivolve_from_source`.
- Permanent state: `dp_eq`, `dp_lte`, `dp_gte`, `is_suspended`, `is_unsuspended`, `materials_count_lte`, `materials_count_gte`, `stack_size_lte`, `stack_size_gte`, `has_keyword`, `has_inherited` (nested predicate), `of_permanent`.
- Source-relative (resolves against `ctx.source_card` / `ctx.source_permanent`): `source_is_tamer`, `source_name_contains`, `source_permanent_trait_has`, `source_is_unsuspended` (PR #472 — checks the source permanent's suspension state from a triggered clause), `self_digivolution_contains_name`, `rules_text_contains` (string — case-insensitive substring match against the subject (or `source_permanent`) permanent's printed rules text: `effect_text + inherited_text + security_text` of its top card; in an inherited `active_when` gate the subject resolves to the carrier Digimon; card driver: BT16-055 Namakemon "[All Turns] While this Digimon has [Pulsemon] in its text, it gets +1000 DP"; PUPPETS-G025).
- Context / global: `your_turn`, `opponents_turn`, `all_turns`, `memory_lte`, `memory_gte`, `security_count_lte`, `security_count_gte`, `can_hatch`, `in_breeding`, `on_field`, `dna_origin`.
- Event payloads (PR #451 event-payload contract — only valid inside event-driven triggers): `event_target_kind`, `event_target_trait_has`, `event_target_owner`, `event_target_is_player`, `event_target_was_self`, `event_permanent_is_source`, `event_host_permanent_is_source`, `event_is_effect_initiated`, `event_card_trait_has`, `event_card_name_contains`, `event_card_color_only` (list — subset semantics: true when **every** color of the triggering event card is in the given set; pair with `event_card_color_count` to express exact multi-color constraints, e.g. "2-color black/yellow only"; PUPPETS-G023), `event_card_color_has` (list — intersection semantics: true when the triggering event card has **at least one** of the listed colors; sibling of `event_card_color_only` and **not** a faithful substitute for it; drives BT16-085's "when a blue or green Digimon digivolves" gate — a multi-color card including blue or green qualifies), `event_card_color_count` (integer — true when the triggering event card has exactly N distinct colors; PUPPETS-G023), `event_cause`, `attacker_trait_has`, `attack_target_change_reason`, `host_permanent_trait_has`, `trashed_source_trait_has`, `trashed_source_card_id_is`.
- Replacement payloads (Track B, only valid inside `kind: replacement` clauses): `replacement_cause`, `replacement_source_is_opponent`, `replacement_subject_is_mine`.
- Bindings: `not_in_binding`, `binding_owner`, `binding_exists`, `binding_present`, `binding_absent`, `equals`, `not_equals`.
- Effect-history rollups (used to gate follow-on clauses, e.g. "if you returned a card by this effect"): `effect_suspended_any_own_digimon`, `effect_suspended_any_opponent_digimon` (opponent-side sibling — true when a prior step of the current effect suspended one of the controller's **opponent's** Digimon; drives BT16-025 Paildramon's "If this effect didn't suspend, unsuspend this Digimon"), `effect_returned_any_card` (bare bool, alias `any_returned_card`), `returned_card_matching` (nested card-filter — filtered variant of `effect_returned_any_card`: true when ≥1 card returned by a preceding return / zone-move step in the same effect satisfies the inner predicate, evaluated as a `Card` subject against the per-effect `returned_to_deck` result log; distinct field name from the `any_returned_card` alias so the two never collide; drives BT17-077's "If this effect returned a white level 7 card, gain 3 memory"), `effect_deleted_any_own_digimon`, `effect_deleted_any_opponent_digimon`, `effect_played_any_digimon`, `effect_digivolved_any_digimon`, `effect_added_any_card_to_hand`.
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

### Uniform comparator (preferred — unify-dsl-scalar-and-comparators §2)

The per-metric `_eq`/`_lte`/`_gte` flat keys above are the *legacy* surface. The canonical form is a single uniform comparator field per metric — `dp`, `level`, `play_cost`, `stack_size`, `materials_count`, `security_count`, `event_target_dp`, `event_target_level` — taking a `Comparator { op: eq | gte | lte, value: <FormulaSpec> }`, as one map or a list (a list is AND-joined, so it expresses a range):

```yaml
# single comparator
filter: { dp: { op: lte, value: 5000 } }

# range (gte AND lte)
filter: { dp: [{ op: gte, value: 3000 }, { op: lte, value: 8000 }] }

# eq is available for EVERY metric (the legacy surface lacked it on
# play_cost / stack_size / materials_count / security_count):
filter: { play_cost: { op: eq, value: 3 } }

# `value` is a FormulaSpec — literal or formula, same as the legacy form:
filter: { dp: { op: lte, value: { source_dp: {} } } }
```

The comparator value resolves **read-safely** at eval (no game mutation), so it is valid in action-mask and search contexts. The legacy flat keys (`dp_lte: N`, `level_eq: N`, …) still parse and lower to the **identical** compiled form, so existing cards need no change — but **don't set both** the canonical field and a legacy flat key for the same metric on one predicate: the canonical comparator overrides the matching legacy op at compile time (a lint to reject the ambiguity outright is planned). One exception: `level` / `event_target_level` `eq` is literal-only (the compiled slot is a `u8`); a formula `eq` on those is a compile error (`lte`/`gte` stay formula-capable).

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
  - Standard printed `<Delay>` ("By trashing this card after the placing turn,
    activate the effect below") uses **`trigger: delayed`** — a player-visible
    `[Main]`-phase activation action. The Option parks on the battle area and
    its controller chooses, on any later main phase, to trash it (the
    activation cost) to run the body. It never auto-fires; the choice surfaces
    through the `FIELD_EFFECT` action range, and the placing turn is gated out
    (RULES_CONTEXT 16-16). Standard Memory Boost / Training / Scramble Options
    take this trigger.
  - `trigger: start_of_your_turn` / `end_of_your_turn` and event triggers
    (`on_suspend`, …) remain engine-scheduled auto-fire Delay timings used by
    start/end-of-turn and event-gated Delay bodies.
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

<!-- BEGIN GENERATED:dsl-vocab -->
<!-- vocab-structural-sha: 6558ee2dd7a17d79b19a4ec1434b83207d4e9f3779b1f329dfbaba56fe9228aa -->
<!-- DO NOT EDIT BY HAND. Generated by code/tools/dsl-doc-export/emit_markdown.py
     from the digimon-dsl enums. Regenerate with:
       cargo run -q -p dsl-schema-export | python code/tools/dsl-doc-export/emit_markdown.py
     CI gate: .github/workflows/dsl-vocab-doc-drift.yml (rule 27 pattern). -->

## DSL Vocabulary Reference (generated)

Complete, enum-derived index of every authoring primitive: **161 step verbs**, **183 predicates**, **59 timings**, **12 declarative kinds**. `uses` = card YAMLs referencing the key; `tag` flags `unused` (0 uses) or `rare` (1–2) vocabulary; `fixture` is a real card to open.

### Step verbs (`process:` / `extra_cost:`)

#### Combat — 10 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `battle` | `BattleArgs` | 5 |  | `bt17/BT17-095.yaml` |  |
| `cancel_attack` | `EmptyArgs` | 2 | rare | `bt21/BT21-060.yaml` |  |
| `end_attack` | `bool` | 5 |  | `bt13/BT13-088.yaml` |  |
| `force_attack` | `ForceAttackArgs` | 6 |  | `bt20/BT20-102.yaml` |  |
| `may_attack_now` | `MayAttackNowArgs` | 30 |  | `ad1/AD1-004.yaml` |  |
| `open_counter_window` | `EmptyArgs` | 0 | unused | — |  |
| `redirect_attack_target` | `RedirectAttackTargetArgs` | 11 |  | `ad1/AD1-012.yaml` |  |
| `refire_card_effect` | `RefireCardEffectArgs` | 1 | rare | `bt15/BT15-102.yaml` | Activate a timing-filtered effect printed on a CARD OBJECT (not a battle-area permanent's top card) — the foreign-card refire variant (BT15-102 Apocalymon: "activate 1 [On Play] effect on that card as an effect of this Digimon"). The `card` binding is a `CardHandle` (typically bound by `place_as_bottom_source.bind_placed_as`), and the card must currently be a digivolution source of this effect's carrier permanent. The chosen effect runs with the CARRIER as "this Digimon" (DCGO `EffectList_ForCard(timing, card)` — the foreign card's effect bodies are constructed against the carrier's card), one EffectChoice surfaces when the card has >1 eligible effect, and the pick is mandatory (DCGO `canNoSelect: () => false`). |
| `refire_effect` | `RefireEffectArgs` | 4 |  | `bt16/BT16-102.yaml` |  |
| `refund_opt` | `EmptyArgs` | 1 | rare | `ad1/AD1-024.yaml` | Refund this clause's once-per-turn use (DCGO `ActivateClass.RemoveUse()` — "if nothing executed, the per-turn use is not consumed"). Place it under a final `if:` whose condition detects the nothing-executed case (typically `binding_absent` over every pick the body could make). Only meaningful inside a `once_per_turn`/`max_per_turn` triggered clause; a no-op elsewhere. G-OPT-REFUND-ON-DECLINE. |

#### Control flow — 8 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `activation_cost` | `ActivationCostArgs` | 47 |  | `ad1/AD1-019.yaml` |  |
| `for_each` | `ForEachStep` | 37 |  | `_examples/BT13-007.yaml` |  |
| `if` | `IfStep` | 214 |  | `_examples/BT17-015.yaml` |  |
| `optional` | `OptionalStep` | 478 |  | `_examples/BT11-042.yaml` |  |
| `per_selected` | `PerSelectedStep` | 36 |  | `ad1/AD1-014.yaml` |  |
| `place_self_as_delay_option` | `EmptyArgs` | 41 |  | `bt13/BT13-110.yaml` |  |
| `schedule_delayed` | `ScheduleDelayedStep` | 2 | rare | `bt1/BT1-090.yaml` |  |
| `schedule_delete_played_at_turn_end` | `ScheduleDeletePlayedAtTurnEndArgs` | 4 |  | `bt23/BT23-037.yaml` | PUPPETS-G003 — schedule the bound permanent for deletion at turn end. |

#### DigiXros — 5 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_digixros_cost_delta` | `DigixrosCostDeltaArgs` | 0 | unused | — |  |
| `add_digixros_wildcard_to_pending_transaction` | `DigixrosWildcardArgs` | 0 | unused | — |  |
| `allow_digixros_material_zone` | `AllowDigixrosMaterialZoneArgs` | 5 |  | `_examples/BT12-112.yaml` |  |
| `preattach_digixros_material` | `PreattachDigixrosMaterialArgs` | 3 |  | `_examples/BT12-112.yaml` |  |
| `register_digixros_wildcard_for_turn` | `DigixrosWildcardArgs` | 1 | rare | `bt10/BT10-111.yaml` |  |

#### Effect digivolve / DNA — 5 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `effect_initiated_digivolve` | `EffectDigivolveArgs` | 78 |  | `_examples/BT17-015.yaml` |  |
| `effect_initiated_dna_digivolve` | `EffectDnaDigivolveArgs` | 7 |  | `ad1/AD1-009.yaml` |  |
| `effect_initiated_dna_digivolve_hand_partner` | `EffectDnaDigivolveHandPartnerArgs` | 2 | rare | `_examples/EX6-072.yaml` |  |
| `effect_initiated_dna_digivolve_trash_partner` | `EffectDnaDigivolveTrashPartnerArgs` | 2 | rare | `bt18/BT18-015.yaml` | DNA digivolve where one material is a battle-area permanent (`target`) and the other is a card in the controller's TRASH (`trash_partner`); the merged permanent is topped with `from_hand` (the result card, from hand). BT18-015 / BT18-073 `[On Deletion]` shape. Lowers to `EffectContext::effect_initiated_dna_digivolve_trash_partner` (G-ENGINE-DNA-TRASH-MATERIAL). G-DSL-DNA-TRASH-PARTNER. |
| `may_dna_digivolve_now` | `MayDnaDigivolveNowArgs` | 11 |  | `bt12/BT12-021.yaml` | G-DSL-EOT-DNA-INLINE — surface the printed `[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand` flow AS A PLAYER CHOICE AT TRIGGER FIRE, rather than registering an alt-path action for a later turn. Used by BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017. See the `CompiledStep::MayDnaDigivolveNow` docstring for the full contract. |

#### Field mutation — 10 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `bind_permanent_property` | `BindPermanentProperty` | 2 | rare | `bt17/BT17-078.yaml` |  |
| `de_digivolve` | `DeDigivolveArgs` | 39 |  | `ad1/AD1-009.yaml` |  |
| `delete_bound_permanents` | `DeleteBoundPermanentsArgs` | 4 |  | `bt17/BT17-018.yaml` |  |
| `delete_permanent` | `TargetArg` | 142 |  | `_examples/BT17-015.yaml` |  |
| `hatch` | `PlayerArg` | 6 |  | `bt16/BT16-082.yaml` |  |
| `return_to_deck` | `ReturnPermanentArgs` | 30 |  | `_examples/BT12-112.yaml` |  |
| `return_to_hand` | `TargetArg` | 18 |  | `_examples/BT13-060.yaml` |  |
| `suspend` | `TargetArg` | 73 |  | `_examples/BT13-060.yaml` |  |
| `trash_breeding_permanent` | `TrashBreedingPermanentArgs` | 1 | rare | `bt13/BT13-112.yaml` |  |
| `unsuspend` | `TargetArg` | 46 |  | `ad1/AD1-006.yaml` |  |

#### Hand / Deck / Trash — 12 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_to_hand_from_deck` | `HandleMoveArgs` | 0 | unused | — |  |
| `add_to_hand_from_trash` | `HandleMoveArgs` | 20 |  | `_examples/BT7-107.yaml` |  |
| `draw` | `DrawArgs` | 104 |  | `_examples/TST_DNA_TRIGGER.yaml` |  |
| `move_from_breeding` | `PlayerArg` | 1 | rare | `p/P-130.yaml` | Move the specified player's eligible breeding-area Digimon to the battle area through the effect-initiated engine path. Pair with `select_own_breeding_permanent optional: true` when printed text says "you may move..." so the accept/decline choice stays visible. |
| `move_trash_card_to_deck_top` | `MoveTrashCardToDeckTopArgs` | 6 |  | `bt18/BT18-019.yaml` |  |
| `recover` | `DrawArgs` | 17 |  | `_examples/BT11-042.yaml` |  |
| `return_all_trash_to_deck_bottom` | `PlayerArg` | 1 | rare | `bt17/BT17-077.yaml` |  |
| `return_trash_list_to_deck_bottom` | `ReturnTrashListToDeckBottomArgs` | 9 |  | `bt13/BT13-083.yaml` |  |
| `shuffle_deck` | `PlayerArg` | 0 | unused | — |  |
| `trash_from_hand_by_index` | `IndexedMoveArgs` | 45 |  | `ad1/AD1-002.yaml` |  |
| `trash_from_top` | `TrashFromTopArgs` | 5 |  | `bt15/BT15-102.yaml` |  |
| `trash_opponent_hand_to_count` | `TrashOpponentHandToCountArgs` | 1 | rare | `bt19/BT19-075.yaml` |  |

#### Link / AppFuse — 6 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `app_fuse` | `AppFuseArgs` | 5 |  | `bt21/BT21-084.yaml` | `app_fuse:` — effect-initiated App Fuse (see [`AppFuseArgs`]). |
| `link_cards` | `LinkCardsArgs` | 16 |  | `_examples/EX11-027.yaml` | Gap 2 — link 1..N chosen cards from a set of source zones onto a Digimon host, without paying a link cost. Drives BT25-060 Rebootmon / BT25-075 Vulcanusmon / BT25-089 Kazuki & Itsuki. The authoring verb over the engine's `link_chosen_card_into_host` primitive: per pick it presents a zone-choice prompt (when ≥2 source zones have candidates — DCGO ST22_12 parity), a single-zone card select, then (for `to: own_digimon`) a host select, then attaches the card and fires `OnLink`. |
| `link_to_own_digimon` | `LinkToOwnDigimonArgs` | 7 |  | `bt24/BT24-091.yaml` |  |
| `reduce_link_cost` | `ReduceLinkCostArgs` | 2 | rare | `bt25/BT25-004.yaml` | Gap 5 — reduce the cost of the link about to resolve in the active `WhenWouldLink` window by `amount`. Authoring verb over the engine's `reduce_pending_link_cost` primitive; the body of a host-side `when: when_would_link_to_this` reducer clause (BT25-004 / BT25-045). |
| `relink_self_to_own_digimon` | `RelinkSelfToOwnDigimonArgs` | 1 | rare | `_examples/EX11-027.yaml` | `relink_self_to_own_digimon:` — move the effect's own standing permanent to become a link card on a chosen OTHER own Digimon (EX11-027 "link this Digimon to 1 of your other Digimon"). Over `absorb_standing_digimon_as_link`. |
| `trash_link_card_of_own_digimon` | `TrashLinkCardOfOwnDigimonArgs` | 1 | rare | `bt25/BT25-073.yaml` | G-DSL-LINK-TRASH-AS-COST (BT25-073 Dragomon) — the ACTIVATION-cost sibling of the replacement-only `cost: { trash_own_link_card: true }`. Picks one of `of`'s Digimon that carries ≥1 link card (first selection), then one of ITS link cards (second selection), trashes it (firing `OnLinkedCardTrashed`), and runs the step's tail ONLY if the trash happened (no-approximations cost gate). When no own Digimon has a link card the cost is unpayable → the clause's remaining steps are skipped (`TailAlreadyRan`). DCGO `BT25_073.cs`: `SelectPermanentEffect` (`!HasNoLinkCards`) → `SelectCardEffect Root.LinkedCards` → `TrashLinkCardsAndProcessAccordingToResult` → `successProcess`. |

#### Memory — 3 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `gain_memory` | `FormulaSpec` | 115 |  | `_examples/BT11-042.yaml` |  |
| `lose_memory` | `FormulaSpec` | 11 |  | `bt1/BT1-090.yaml` |  |
| `set_memory` | `FormulaSpec` | 23 |  | `bt1/BT1-085.yaml` |  |

#### Modifier / Keyword — 8 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_dp_modifier` | `AddDpModifierArgs` | 87 |  | `ad1/AD1-016.yaml` |  |
| `add_modifier` | `AddModifierArgs` | 69 |  | `_examples/BT13-060.yaml` |  |
| `add_player_modifier` | `AddPlayerModifierArgs` | 9 |  | `bt12/BT12-043.yaml` |  |
| `arm_digivolve_cost_reducer` | `ArmDigivolveCostReducerArgs` | 2 | rare | `bt3/BT3-103.yaml` | G-COST-REDUCE-ALLY-DIGIVOLVE — install a player-scoped one-shot future-digivolve cost reducer. Used by BT3-103 Hidden Potential Discovered!'s `[Main]` clause: "For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5." The reducer fires at the next qualifying digivolution; if `suspend_cost` is set the player is prompted to suspend 1 of their own Digimon (a player-visible cost). |
| `grant_effect_immunity` | `GrantEffectImmunityArgs` | 17 |  | `ad1/AD1-009.yaml` |  |
| `grant_keyword` | `GrantKeywordArgs` | 74 |  | `_examples/BT11-042.yaml` |  |
| `grant_narrow_opponent_effect_protection` | `GrantNarrowOpponentEffectProtectionArgs` | 1 | rare | `bt16/BT16-055.yaml` | PUPPETS-G024 — install the narrow opponent-effect protection bundle (ImmuneFromDPMinus opponent-scoped + CannotBeDeDigivolved opponent-scoped). For text like BT16-055's "can't have its DP reduced by your opponent's effects and isn't affected by ＜De-Digivolve＞ effects". |
| `grant_triggered_effect` | `GrantTriggeredEffectArgs` | 9 |  | `bt21/BT21-073.yaml` | Track H §3 — install a granted triggered effect on each permanent matching `target`. The granted body fires on the carrier's matching `timing` (DCGO `AddSkillClass.cs` analog). EX1-068 Ice Wall! is the canonical fixture. |

#### Other — 11 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_this_option_to_hand` | `EmptyArgs` | 20 |  | `_examples/BT7-107.yaml` |  |
| `bounce_self` | `EmptyArgs` | 0 | unused | — |  |
| `delete_for_cost_reduction` | `TargetArg` | 1 | rare | `bt13/BT13-103.yaml` | `delete_for_cost_reduction: { target: <binding> }` — delete AS A COST and reduce the in-flight cost by the deleted permanent's printed play cost. `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST` (BT13-103). |
| `delete_one_per_opponent_color` | `DeleteOnePerOpponentColorArgs` | 1 | rare | `ex9/EX9-074.yaml` | G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR — per-color mandatory pick + batch delete (EX9-074 Kimeramon Branch B). |
| `move_self_option_under_permanent` | `MoveSelfOptionUnderPermanentArgs` | 2 | rare | `st23/ST23-15.yaml` | Relocate THIS effect's source Option (an in-battle-area field Option) face-down under a chosen own permanent — a new Option-lifecycle exit distinct from trashing. Fires neither `OnOptionTrashed` nor `OnDigivolutionCardTrashed`. Drives ST23-15 e-Pulse / ST24-15 DNA Charge "By placing this card from the battle area face down under any of your [BEATBREAK]/[DATA SQUAD] trait Tamers, …". G-MOVE-SELF-OPTION-UNDER-PERMANENT. |
| `place_remainder_on_deck` | `PlaceRemainderArgs` | 74 |  | `_examples/EX11-027.yaml` |  |
| `place_self_under_permanent` | `PlaceSelfUnderPermanentArgs` | 3 |  | `ex7/EX7-070.yaml` | Place THIS effect's source Option as the bottom digivolution card of a chosen own permanent — the "[Main] … Then, place this card as the bottom digivolution card of 1 of your … Digimon" tail on P-180 / EX7-070 / EX7-071. Unlike `move_self_option_under_permanent` (which requires a live field-Option `source_permanent`), this composes with the standard Option [Main]-play path by claiming the in-flight `pending_option`, so the subsequent `dispose_option` finds nothing and the card is seated (FACE-UP by default) instead of trashed. Lowers to `EffectContext::place_self_under_permanent`. G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL. |
| `raw_rust` | `RawRustStep` | 3 |  | `_examples/AD1-025.yaml` |  |
| `trash_option_from_own_stacks` | `TrashOptionFromOwnStacksArgs` | 0 | unused | — | Trash-Option-from-{digivolution\|link}-cards ACTIVATION cost (BT25-085 BeelStarmon): pick one of `of`'s Digimon whose digivolution cards OR link cards carry ≥1 Option, then one such Option, and trash it — the correct observer fires per zone (`OnDigivolutionCardTrashed` for a digivolution source, `OnLinkedCardTrashed` for a link card). The tail runs only if a card was trashed. When no own Digimon has an Option in its digivolution/link cards the cost is unpayable → the clause's remaining steps are skipped (`TailAlreadyRan`). DCGO `BT25_085.cs`: `SelectPermanentEffect(PermanentWithTrashableCard)` → `SelectCardEffect Mode.Discard` over `permanent.DigivolutionOrLinkCards.Any(IsOption)`. The union-of-sources sibling of `TrashLinkCardOfOwnDigimon`. |
| `trash_top_n_digivolution_cards_of_each` | `TrashTopNDigivolutionCardsOfEachArgs` | 1 | rare | `bt12/BT12-028.yaml` |  |
| `trash_union_bound` | `UnionBoundArgs` | 3 |  | `bt25/BT25-083.yaml` | Trash a `select_union_zone`-bound card from its true origin zone. Used for costs that can be paid by trashing a card from hand or from one of your Digimon's digivolution cards. |

#### Play / zone-in — 11 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `play_from_hand` | `PlayFromHandArgs` | 16 |  | `ad1/AD1-019.yaml` |  |
| `play_from_hand_free` | `PlayFromHandFreeArgs` | 68 |  | `bt12/BT12-038.yaml` |  |
| `play_from_materials` | `PlayFromMaterialsArgs` | 15 |  | `_examples/BT20-083.yaml` |  |
| `play_from_trash` | `PlayFromHandArgs` | 2 | rare | `bt19/BT19-099.yaml` |  |
| `play_from_trash_free` | `PlayFromHandArgs` | 35 |  | `_examples/BT18-019.yaml` |  |
| `play_or_use_from_hand` | `PlayOrUseFromHandArgs` | 5 |  | `bt25/BT25-041.yaml` | Unified "play OR use 1 card from hand" — inspects the bound hand card's kind and routes Digimon/Tamer → play and Option → use (a DUAL card surfaces a "Play as Digimon / Use as Option" face choice). The card is bound by an upstream `select_hand` step. `G-PLAY-OR-USE-FROM-HAND`. |
| `play_token` | `PlayTokenArgs` | 12 |  | `bt20/BT20-017.yaml` |  |
| `play_union_bound_free` | `PlayUnionBoundFreeArgs` | 30 |  | `ad1/AD1-002.yaml` | PUPPETS-G014 — play a `select_union_zone`-bound card for free from its true origin zone (hand, trash, or material), recovered from the binding. |
| `use_option_bound` | `UseOptionBoundArgs` | 1 | rare | `bt21/BT21-062.yaml` | Use a `select_union_zone{hand,trash}`-bound Option from its true origin zone, applying `cost` to its printed use cost (omitted = free). Driver BT21-062 "use 1 [Ragnarok Cannon] from your hand or trash without paying the cost". `G-DSL-USE-OPTION-FROM-SOURCES`. |
| `use_option_from_hand` | `UseOptionFromHandArgs` | 1 | rare | `bt24/BT24-085.yaml` |  |
| `use_option_from_trash` | `UseOptionFromTrashArgs` | 1 | rare | `bt25/BT25-083.yaml` | Effect-driven Option USE from the controller's trash with a cost delta (Gap 2, `G-DSL-USE-OPTION-FROM-SOURCES`). Trash analogue of `use_option_from_hand`. |

#### Replacement-flow — 4 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `cancel_replacement` | `EmptyArgs` | 23 |  | `bt13/BT13-075.yaml` |  |
| `handle_replacement` | `EmptyArgs` | 0 | unused | — |  |
| `redirect_replacement` | `RedirectReplacementArgs` | 0 | unused | — |  |
| `substitute_replacement` | `SubstituteReplacementArgs` | 3 |  | `ex11/EX11-022.yaml` |  |

#### Reveal — 11 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_to_hand_from_reveal` | `HandleMoveArgs` | 78 |  | `_examples/EX11-027.yaml` |  |
| `choose_from_reveal` | `ChooseFromRevealArgs` | 6 |  | `bt11/BT11-089.yaml` | Phase 2 Track E (2026-05-17): pick one card from the current reveal pool and route it to a single typed destination. Ergonomic combo of `select_reveal` + `{add_to_hand_from_reveal,return_to_deck_from_reveal, place_as_bottom_source}`. Pair with `order_remainder` for the "reveal N, choose 1 to hand/source, place rest top-or-bottom in any order" pattern that recurs across Rocks searchers and general training effects. |
| `order_remainder` | `OrderRemainderArgs` | 5 |  | `bt11/BT11-089.yaml` | Phase 2 Track E (2026-05-17): place all remaining revealed cards onto the controller's deck. Unlike `place_remainder_on_deck`, the destination (top vs bottom) can itself be a player choice when the printed text reads "top or bottom" (P-167 et al). Always surfaces the `select_ordered_permutation` ordering selection per Working Rule §17. |
| `play_from_revealed_free` | `PlayFromRevealedFreeArgs` | 6 |  | `bt11/BT11-105.yaml` |  |
| `return_to_deck_from_reveal` | `ReturnToDeckArgs` | 3 |  | `_examples/BT18-019.yaml` |  |
| `reveal_search` | `RevealSearchArgs` | 2 | rare | `bt21/BT21-058.yaml` | collapse-dsl-step-idioms §2 — the reveal-search idiom as one composite verb. Lowers to the existing sequence (reveal_top_deck → a single `select_reveal_buckets` over all buckets → per-bucket reveal-move → place_remainder_on_deck); each bucket is an RL-visible pick honoring its `optional`. Cross-bucket de-dup is always on (a revealed card lands in at most one bucket). See `RevealSearchArgs`. |
| `reveal_top_deck` | `RevealArgs` | 93 |  | `_examples/BT13-007.yaml` |  |
| `select_reveal` | `SelectZoneArgs` | 37 |  | `_examples/EX11-027.yaml` |  |
| `select_reveal_buckets` | `SelectRevealBucketsArgs` | 49 |  | `bt11/BT11-061.yaml` |  |
| `trash_from_reveal` | `HandleMoveArgs` | 11 |  | `bt11/BT11-105.yaml` |  |
| `use_option_from_revealed` | `UseOptionFromRevealedArgs` | 1 | rare | `ex7/EX7-048.yaml` | Use an Option previously picked from the game-level reveal pool by a `select_reveal` step, applying `cost` to its printed use cost (omitted = free), preserving the full Option lifecycle. `card` names the reveal-pool binding. Driver EX7-048 "reveal top 6, use 1 [Three Musketeers] Option among them free"; the remaining revealed cards are handled by the enclosing reveal step. `G-DSL-USE-OPTION-FROM-SOURCES`. |

#### Security — 17 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `add_bottom_security_to_hand` | `PlayerArg` | 6 |  | `bt24/BT24-090.yaml` |  |
| `add_to_hand_from_security` | `HandleMoveArgs` | 2 | rare | `_examples/BT11-042.yaml` |  |
| `add_top_security_to_hand` | `PlayerArg` | 12 |  | `bt11/BT11-033.yaml` |  |
| `flip_security_face_up` | `PlayerArg` | 1 | rare | `bt20/BT20-055.yaml` |  |
| `mark_security_face_up` | `MarkSecurityArgs` | 0 | unused | — |  |
| `may_add_top_security_to_hand` | `PlayerArg` | 2 | rare | `bt24/BT24-031.yaml` |  |
| `place_on_security` | `PlaceOnSecurityArgs` | 29 |  | `_examples/BT18-102.yaml` |  |
| `play_from_security` | `PlayFromSecurityArgs` | 99 |  | `ad1/AD1-019.yaml` |  |
| `play_security_card` | `HandleMoveArgs` | 2 | rare | `bt13/BT13-012.yaml` | Play a specific bound card FROM the security stack without paying its cost. The `card` binding is a `CardHandle` (typically produced by a prior `select_security` step). G-PLAY-SELECTED-SECURITY-CARD. Used by BT13-012 ("you may play 1 red or yellow Tamer card among it without paying its cost"). |
| `return_selected_security_to_deck` | `ReturnToDeckArgs` | 1 | rare | `lm/LM-020.yaml` | Move a specific bound card FROM a player's security stack to that player's deck (top or bottom; Digi-Eggs route to the digitama deck). The `card` binding is a `CardHandle` (typically from a prior `select_security` step). G-DSL-RETURN-SELECTED-SECURITY-TO-DECK. Used by LM-020 Quantumon ("place 1 card among them on top of your opponent's deck"). YAML: `return_selected_security_to_deck: { of, card, position }`. |
| `search_own_security_stack` | `SearchOwnSecurityStackArgs` | 2 | rare | `_examples/BT11-042.yaml` |  |
| `select_security` | `SelectZoneArgs` | 5 |  | `bt13/BT13-012.yaml` |  |
| `shuffle_security` | `PlayerArg` | 6 |  | `_examples/BT11-042.yaml` |  |
| `trash_bottom_security` | `PlayerArg` | 4 |  | `ad1/AD1-017.yaml` |  |
| `trash_selected_security` | `HandleMoveArgs` | 1 | rare | `bt24/BT24-018.yaml` | Trash a specific bound card FROM a player's security stack. The `card` binding is a `CardHandle` (typically produced by a prior `select_security` step). G-TRASH-SELECTED-SECURITY. Used by BT24-018 ("You may trash any 1 of your opponent's security cards"). |
| `trash_top_security` | `TrashTopSecurityArgs` | 48 |  | `_examples/BT17-015.yaml` |  |
| `trash_top_security_and_cancel_replacement` | `PlayerArg` | 2 | rare | `bt20/BT20-056.yaml` |  |

#### Selection — 17 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `as_selecting_player` | `AsSelectingPlayerArgs` | 6 |  | `bt13/BT13-102.yaml` |  |
| `digi_burst` | `DigiBurstArgs` | 3 |  | `bt4/BT4-072.yaml` |  |
| `select_any_permanent` | `SelectFieldArgs` | 16 |  | `bt19/BT19-065.yaml` |  |
| `select_count_capped_multi` | `SelectCountCappedArgs` | 28 |  | `_examples/BT18-019.yaml` |  |
| `select_dna_pair` | `SelectDnaPairArgs` | 5 |  | `ad1/AD1-009.yaml` |  |
| `select_effect_choice` | `SelectEffectChoiceArgs` | 61 |  | `_examples/BT17-015.yaml` |  |
| `select_hand` | `SelectZoneArgs` | 216 |  | `_examples/BT17-015.yaml` |  |
| `select_material` | `SelectMaterialArgs` | 11 |  | `bt21/BT21-060.yaml` |  |
| `select_materials` | `SelectMaterialsArgs` | 5 |  | `_examples/BT20-083.yaml` |  |
| `select_opponent_dp_budget` | `SelectOpponentDpBudgetArgs` | 2 | rare | `bt17/BT17-018.yaml` |  |
| `select_opponent_permanent` | `SelectFieldArgs` | 255 |  | `_examples/BT12-112.yaml` |  |
| `select_opponent_play_cost_budget` | `SelectOpponentPlayCostBudgetArgs` | 2 | rare | `ex4/EX4-073.yaml` |  |
| `select_ordered_permutation` | `SelectPermutationArgs` | 0 | unused | — |  |
| `select_own_breeding_permanent` | `SelectOwnBreedingPermanentArgs` | 7 |  | `_examples/BT20-083.yaml` |  |
| `select_own_permanent` | `SelectFieldArgs` | 178 |  | `_examples/BT12-112.yaml` |  |
| `select_trash` | `SelectZoneArgs` | 88 |  | `_examples/BT18-019.yaml` |  |
| `select_union_zone` | `SelectUnionArgs` | 45 |  | `ad1/AD1-002.yaml` |  |

#### Stack / Source — 16 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `place_as_bottom_source` | `PlaceAsBottomSourceArgs` | 51 |  | `_examples/BT13-007.yaml` |  |
| `place_as_top_source` | `PlaceAsTopSourceArgs` | 2 | rare | `bt13/BT13-088.yaml` | G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05): place a card as `target`'s TOP digivolution source — inserted directly beneath the active top card (DCGO `Permanent.AddDigivolutionCardsTop`, which `Insert(1, …)`s into its top-first `cardSources`). The permanent's top card / identity is unchanged (placing a digivolution card never changes what the permanent IS). Sibling of `place_as_bottom_source` with the same args shape. Consumers: EX9-074 Kimeramon ("as this Digimon's top digivolution card"), BT13-088 Belphemon: Sleep Mode ("on top of this Digimon's digivolution cards"). |
| `place_top_source_as_bottom` | `TargetArg` | 2 | rare | `bt23/BT23-008.yaml` | Phase 2 Track F (2026-05-17): move `target`'s top stacked card (the digivolution source immediately beneath the active top card) to the bottom of its own stack. Closes G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM (BT23-008 / BT23-018-shape "place top stacked card as bottom" costs). Per the no-approximations policy this is a deterministic source pick — the printed text identifies a singular top source, so no `select_material` choice is exposed. |
| `play_selected_sources_free` | `TrashSelectedSourcesArgs` | 1 | rare | `st9/ST9-06.yaml` |  |
| `return_selected_sources_to_deck` | `ReturnSelectedSourcesToDeckArgs` | 4 |  | `bt13/BT13-075.yaml` | G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM (2026-06-14) — deck-routing sibling of `ReturnSelectedSourcesToHand`. Return each `select_own_sources`-bound digivolution source card to its owner's deck (`position: top \| bottom`). Like the to-hand verb this is a return, NOT a trash, so it fires no `OnDigivolutionCardTrashed`. Closes BT13-075 Alphamon's would-leave self-protection cost (return 1 [X Antibody]/[Royal Knight] source to the BOTTOM OF YOUR DECK to prevent leaving). |
| `return_selected_sources_to_hand` | `TrashSelectedSourcesArgs` | 1 | rare | `bt12/BT12-031.yaml` | G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (2026-05-21) — return each `select_own_sources`-bound digivolution source card to its owner's hand. Mirrors `TrashSelectedSources` but routes the source `Card` to the owner's hand instead of trash; fires no `OnDigivolutionCardTrashed` (this is a return, not a trash). Closes BT12-031's Imperialdramon: Dragon Mode alt-cost. |
| `security_place_stacked_card` | `SecurityPlaceStackedCardArgs` | 1 | rare | `bt25/BT25-038.yaml` |  |
| `security_place_top_stacked_card` | `SecurityPlaceTopStackedCardArgs` | 2 | rare | `bt20/BT20-055.yaml` |  |
| `select_opponent_sources` | `SelectOpponentSourcesArgs` | 9 |  | `bt16/BT16-085.yaml` |  |
| `select_own_sources` | `SelectOwnSourcesArgs` | 21 |  | `bt12/BT12-031.yaml` |  |
| `trash_all_sources` | `TargetArg` | 2 | rare | `bt17/BT17-077.yaml` |  |
| `trash_bottom_sources` | `TrashBottomSourcesArgs` | 6 |  | `bt25/BT25-026.yaml` |  |
| `trash_selected_sources` | `TrashSelectedSourcesArgs` | 23 |  | `bt16/BT16-085.yaml` |  |
| `trash_top_source` | `TargetArg` | 3 |  | `_examples/BT13-060.yaml` |  |
| `trash_top_stacked_sources` | `TrashTopStackedSourcesArgs` | 4 |  | `bt13/BT13-030.yaml` |  |
| `use_option_from_sources` | `UseOptionFromSourcesArgs` | 0 | unused | — | Use an Option from a permanent's digivolution stack (a `select_own_sources` binding), applying `cost` to its printed use cost (omitted = free). `card` names the source-refs binding. Driver BT25-085 "use 1 [Three Musketeers]/[TS] Option from your hand or this Digimon's digivolution cards free" (the digivolution-cards origin — the engine `OptionSource::Source` fork resolves both `card_sources` and `linked_cards`). `G-DSL-USE-OPTION-FROM-SOURCES`. |

#### Under-Tamer source — 7 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `move_matching_sources_under_tamer` | `MoveMatchingSourcesUnderTamerArgs` | 1 | rare | `bt21/BT21-092.yaml` |  |
| `place_selected_card_under_tamer` | `PlaceSelectedCardUnderTamerArgs` | 12 |  | `bt11/BT11-095.yaml` |  |
| `place_selected_sources_under_tamer` | `PlaceSelectedSourcesUnderTamerArgs` | 1 | rare | `bt21/BT21-027.yaml` |  |
| `play_under_tamer_source` | `PlayUnderTamerSourceArgs` | 5 |  | `bt19/BT19-014.yaml` |  |
| `select_under_tamer_sources` | `SelectOwnSourcesArgs` | 10 |  | `bt10/BT10-093.yaml` |  |
| `trash_bottom_face_down_source_under_tamer` | `TrashBottomFaceDownSourceUnderTamerArgs` | 20 |  | `bt25/BT25-027.yaml` |  |
| `trash_bottom_face_down_sources_under_tamers` | `TrashBottomFaceDownSourcesUnderTamersArgs` | 3 |  | `bt25/BT25-035.yaml` | G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER (2026-06-15) — the multi-count / multi-Tamer sibling of `TrashBottomFaceDownSourceUnderTamer`. Pays a `count`-total bottom-face-down trash cost distributed across the controller's Tamers: each pick installs a real `select_own_permanent` over `{ kind: tamer, has_face_down_source: true }` and trashes that Tamer's bottom face-down source, re-evaluating eligibility before the next pick — so "2 from one Tamer" and "1 from each of two Tamers" are both reachable, every pick surfaced (no auto-resolve). When fewer than `count` face-down sources exist across all Tamers the cost is unpayable and the clause's remaining steps are skipped (`TailAlreadyRan`). Drives BT25-035 Cougarmon's "by trashing 2 bottom face-down cards from under any of your Tamers" cost. |

### Predicates (`filter:` / `condition:` / `active_when:`)

#### Aggregate / existential — 15 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `all_permanents` | `ExistentialPredicate?` | 0 | unused | — |  |
| `any_field_permanent` | `ExistentialPredicate?` | 26 |  | `bt21/BT21-093.yaml` |  |
| `any_permanent` | `ExistentialPredicate?` | 196 |  | `_examples/BT11-042.yaml` |  |
| `color_matches_any_field_digimon` | `PlayerRefSelector?` | 1 | rare | `p/P-206.yaml` |  |
| `count_gte` | `CountAggregate?` | 95 |  | `_examples/EX11-027.yaml` |  |
| `count_lte` | `CountAggregate?` | 22 |  | `bt13/BT13-111.yaml` |  |
| `distinct_named_count_gte` | `DistinctNamedCountPredicate?` | 1 | rare | `bt21/BT21-040.yaml` | True when the observer's battle-area permanents matching `filter` include at least `n` DISTINCT (synth-identity-aware) card names. A no-subject global predicate — does not inspect the candidate. Modeled on `distinct_tamer_colors_gte`, but keyed on distinct card names among a filtered permanent set rather than distinct Tamer colors. Driver: BT21-040 — "you have 3 or more [Hero] trait Tamers with different names". G-DSL-DISTINCT-NAMED-PERMANENT-COUNT. |
| `has_alt_path` | `str?` | 4 |  | `bt10/BT10-087.yaml` |  |
| `level_matches_aggregate` | `LevelAggregatePredicate?` | 9 |  | `ad1/AD1-012.yaml` |  |
| `materials_count` | `MetricComparators?` | 0 | unused | — |  |
| `materials_count_gte` | `DpConstraint?` | 12 |  | `bt13/BT13-030.yaml` |  |
| `materials_count_lte` | `DpConstraint?` | 16 |  | `ad1/AD1-025.yaml` |  |
| `materials_count_matches_aggregate` | `MaterialCountAggregatePredicate?` | 1 | rare | `bt24/BT24-030.yaml` |  |
| `no_permanent` | `ExistentialPredicate?` | 20 |  | `bt12/BT12-016.yaml` |  |
| `own_source_stack_color_count_gte` | `i32?` | 1 | rare | `ex9/EX9-074.yaml` | True when the effect CARRIER's NON-FLIPPED digivolution-source color set has at least N distinct colors. A no-subject, carrier-scoped global predicate — it does NOT inspect the candidate, it reads `ctx.source_permanent` and applies the SAME shared extraction as `color_matches_own_source_stack` (`non_flipped_source_colors`: sources beneath the top card, face-down/flipped excluded, deduplicated). The YAML-reachable numeric branch discriminant for EX9-074 Kimeramon: "If this Digimon has 6 or more colors in its digivolution cards, instead delete 1 of each of your opponent's Digimon with different colors" — `if: { condition: { own_source_stack_color_count_gte: 6 }, … }`. Mirrors DCGO `DigivolutionCards.Filter(!IsFlipped) .SelectMany(CardColors).Distinct().Count >= N`. G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD (driver EX9-074). |

#### Binding — 11 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `binding_absent` | `str?` | 4 |  | `ad1/AD1-024.yaml` |  |
| `binding_card_kind` | `BindingCardKindPredicate?` | 1 | rare | `lm/LM-020.yaml` | True when the card bound to `binding` has the given card category. Resolves the named card binding (e.g. from `reveal_top_deck { bind_as }`) and compares its printed kind. Used by LM-020 Quantumon to test whether the revealed opponent deck-top matches the declared category. |
| `binding_count_eq` | `BindingCountPredicate?` | 5 |  | `bt12/BT12-031.yaml` | True when the named list-typed binding (a `source_refs`, permanent-list or card-list binding produced by a multi-select / `select_own_sources` step) holds exactly `n` entries. Used by EX4-073 clause C's "if you trashed 3 cards" tail. A scalar / single binding counts as 1; a missing binding counts as 0. G-DSL-BINDING-COUNT-EQ. |
| `binding_exists` | `str?` | 9 |  | `ad1/AD1-024.yaml` |  |
| `binding_owner` | `BindingOwnerPredicate?` | 2 | rare | `bt24/BT24-047.yaml` |  |
| `binding_present` | `str?` | 65 |  | `ad1/AD1-002.yaml` |  |
| `cost_target` | `PredicateSpec?` | 15 |  | `bt11/BT11-061.yaml` | BeforePayCost target card predicate. When present, the inner predicate is evaluated against the card whose cost is currently being computed (`cost_target_card` on the effect read context), treated as a `Card` subject. Fails when no cost target is active (i.e., outside `BeforePayCost` cost-calc dispatch). Use the full card-shape vocabulary inside: `trait_has`, `color_is`, `name_contains`, `level_eq`/`_lte`/`_gte`, `kind`, etc. Example: a cost-reduction clause that fires only when the card being digivolved into has the [Free] trait: ```yaml active_when: your_turn: true cost_target: { trait_has: Free } ``` G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure). |
| `equals` | `[]?` | 61 |  | `_examples/BT17-015.yaml` |  |
| `is_source` | `bool?` | 2 | rare | `ad1/AD1-024.yaml` | `is_source: true` — the subject permanent must BE the effect's source permanent (the mirror of `other: true`). Use it to filter a select down to "this Digimon" — e.g. DCGO's standalone "Will you unsuspend this card?" prompt becomes an optional `select_own_permanent` with `is_source: true`, exposing the Yes/No to the RL action space. |
| `not_equals` | `[]?` | 0 | unused | — |  |
| `not_in_binding` | `str?` | 5 |  | `bt15/BT15-101.yaml` |  |

#### Compound / structural — 7 (0 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `all_of` | `[PredicateSpec]` | 549 |  | `_examples/BT11-042.yaml` |  |
| `any_of` | `[PredicateSpec]` | 317 |  | `_examples/BT11-042.yaml` |  |
| `none_of` | `[PredicateSpec]` | 34 |  | `bt13/BT13-019.yaml` |  |
| `not` | `PredicateSpec?` | 10 |  | `ad1/AD1-017.yaml` |  |
| `other` | `bool?` | 24 |  | `ad1/AD1-012.yaml` |  |
| `owner` | `PlayerRef?` | 54 |  | `_examples/BT11-042.yaml` |  |
| `zone` | `[Zone]` | 355 |  | `_examples/BT11-042.yaml` |  |

#### Context / global — 22 (4 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `all_turns` | `bool?` | 124 |  | `ad1/AD1-001.yaml` |  |
| `can_hatch` | `PlayerRef?` | 3 |  | `bt16/BT16-082.yaml` |  |
| `dna_origin` | `bool?` | 16 |  | `_examples/BT18-019.yaml` |  |
| `face_up_security_count_gte` | `DpConstraint?` | 0 | unused | — | True when the observer's face-up security-card count is at least this threshold. Face-up state lives in `Player.face_up_security`. |
| `face_up_security_count_lte` | `DpConstraint?` | 6 |  | `bt24/BT24-090.yaml` | True when the observer's face-up security-card count is at most this threshold. Face-up state lives in `Player.face_up_security`. |
| `in_breeding` | `bool?` | 2 | rare | `_examples/BT13-007.yaml` |  |
| `memory_gte` | `DpConstraint?` | 2 | rare | `bt24/BT24-102.yaml` |  |
| `memory_lte` | `DpConstraint?` | 34 |  | `bt1/BT1-085.yaml` |  |
| `no_face_up_security_named` | `FaceUpSecurityNamedPredicate?` | 2 | rare | `ex10/EX10-020.yaml` | True when the named player has NO face-up security card matching the given identity filter. Face-up state lives in `Player.face_up_security` (a `card_index` index set), which is unreachable from any other predicate leaf — security cards are raw `Card`s, not `Permanent`s, so `any_permanent { zone: [security] }` cannot see them and has no face-up discriminator. Models card text of the form "While you have no face-up [Name] security cards, ...". G-PRED-NO-FACE-UP-SECURITY-NAMED. |
| `on_field` | `bool?` | 1 | rare | `bt21/BT21-093.yaml` |  |
| `opponent_security_count_gte` | `DpConstraint?` | 2 | rare | `bt25/BT25-043.yaml` |  |
| `opponent_security_count_lte` | `DpConstraint?` | 2 | rare | `bt21/BT21-024.yaml` |  |
| `opponents_turn` | `bool?` | 34 |  | `_examples/BT11-042.yaml` |  |
| `own_memory_gte` | `DpConstraint?` | 0 | unused | — |  |
| `own_memory_lte` | `DpConstraint?` | 2 | rare | `bt17/BT17-016.yaml` | Memory from the perspective of the predicate's CONTROLLER (the effect's owner), unlike `memory_lte`/`memory_gte` which compare the raw turn-player-perspective gauge. "While you have 0 or less memory" (EX8-073 / BT17-016 immunity) is `own_memory_lte: 0` — true when the controller's signed memory (the gauge when it is their turn, the negated gauge otherwise) is at or below the bound. G-DSL-OWN-MEMORY-PREDICATE. |
| `security_count` | `MetricComparators?` | 1 | rare | `bt7/BT7-032.yaml` |  |
| `security_count_gte` | `DpConstraint?` | 17 |  | `ad1/AD1-017.yaml` |  |
| `security_count_lte` | `DpConstraint?` | 16 |  | `_examples/BT20-083.yaml` |  |
| `total_security_count_eq` | `DpConstraint?` | 0 | unused | — |  |
| `total_security_count_gte` | `DpConstraint?` | 0 | unused | — |  |
| `total_security_count_lte` | `DpConstraint?` | 1 | rare | `bt13/BT13-106.yaml` | True when the SUM of both players' security-stack card counts (the controller's + the opponent's) is at most / at least / equal to this threshold. Gates BT13-106 Odin's Breath's "[Then], if there're 6 or fewer total cards in both players' security stacks, …" clause. DCGO `card.Owner.SecurityCards.Count + card.Owner.Enemy.SecurityCards.Count <= 6`. G-DSL-TOTAL-SECURITY-COUNT-PREDICATE (driver BT13-106). |
| `your_turn` | `bool?` | 202 |  | `_examples/BT11-042.yaml` |  |

#### Effect-history — 11 (3 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `effect_added_any_card_to_hand` | `bool?` | 0 | unused | — |  |
| `effect_deleted_any_opponent_digimon` | `bool?` | 7 |  | `bt12/BT12-016.yaml` |  |
| `effect_deleted_any_own_digimon` | `bool?` | 1 | rare | `ex3/EX3-057.yaml` |  |
| `effect_deleted_opponent_digimon_dp_gte` | `DpConstraint?` | 1 | rare | `ex4/EX4-065.yaml` | True iff at least one OPPONENT Digimon deleted by THIS effect had pre-removal effective DP `>= N`. The DP-threshold sibling of `effect_deleted_any_opponent_digimon`; reads the per-deletion DP snapshot recorded in the effect-result log (the carrier is in trash by the time a rider evaluates, so the snapshot is the only faithful DP source). Driver: EX4-065 Trident Gaia ("If a Digimon with 13000 DP or more is deleted by this effect, trash the opponent's top security card"). G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD. |
| `effect_digivolved_any_digimon` | `bool?` | 0 | unused | — |  |
| `effect_played_any_digimon` | `bool?` | 2 | rare | `bt13/BT13-110.yaml` |  |
| `effect_returned_any_card` | `bool?` | 3 |  | `bt11/BT11-033.yaml` |  |
| `effect_suspended_any_opponent_digimon` | `bool?` | 1 | rare | `bt16/BT16-025.yaml` | Opponent-side sibling of `effect_suspended_any_own_digimon`. True when the current effect's result log records a suspend of any of the controller's OPPONENT's Digimon. Used by BT16-025 Paildramon clause 2 ("If this effect didn't suspend, unsuspend this Digimon"). G-DSL-EFFECT-SUSPENDED-RESULT. |
| `effect_suspended_any_own_digimon` | `bool?` | 0 | unused | — |  |
| `effect_text_contains` | `str?` | 14 |  | `_examples/EX11-027.yaml` | Case-insensitive substring scan against the candidate card's printed text — `effect_text`, `inherited_text`, and `security_text` concatenated. Distinct from `name_contains`, which only scans `card_name`. Used by BT22-017's bucket 1 ("1 card with [Omnimon] in its text"). DCGO `source.HasText(s)`. G-DSL-PREDICATE-TEXT-CONTAINS. |
| `returned_card_matching` | `PredicateSpec?` | 1 | rare | `bt17/BT17-077.yaml` | Filtered variant of `effect_returned_any_card`. True when at least one card moved by a preceding return / zone-move step in the SAME effect satisfies the inner card-shape predicate. The inner predicate is evaluated as a `Card` subject against each returned card identity in the per-effect result log (`returned_to_deck`). Distinct field name from the bare-bool `any_returned_card` alias so the two never collide. Example: `returned_card_matching: { color_is: white, level_eq: 7 }`. G-ANY-RETURNED-CARD-PREDICATE — driver BT17-077 clause 1c. |

#### Event payload — 39 (7 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `event_add_to_hand_player` | `PlayerRef?` | 3 |  | `bt11/BT11-033.yaml` | For `OnAddToHand` observers: the player whose hand gained cards (`TriggerContext.affected_player`) must match this player-ref, resolved relative to the observer (`you` / `opponent`). See G-ON-ADD-TO-HAND-OBSERVER. |
| `event_card_color_count` | `i32?` | 1 | rare | `bt13/BT13-101.yaml` | True when the triggering event card has exactly N distinct colors. Pair with `event_card_color_only` to express "exactly 2-color black/yellow". PUPPETS-G023. |
| `event_card_color_has` | `[ColorSpec]?` | 5 |  | `bt16/BT16-085.yaml` | True when the triggering event card has AT LEAST ONE of the listed colors (intersection / "has" semantics). Sibling of `event_card_color_only` (subset semantics — not a faithful substitute). Used by BT16-085's "when a blue or green Digimon digivolves" trigger gate. G-EVENT-CARD-COLOR-IS. |
| `event_card_color_only` | `[ColorSpec]?` | 1 | rare | `bt13/BT13-101.yaml` | True when every color of the triggering event card is within the given set. Used to gate observers on "the just-played card is black/yellow only" without listing individual card names. Mirrors `color_only` but operates on the event payload rather than the predicate subject. PUPPETS-G023. |
| `event_card_in_text_contains` | `str?` | 0 | unused | — | Event-side analogue of the static `in_text_contains` — the BROAD whole-card "[X] in its text" scan (DCGO `CardSource.HasText`) applied to the triggering event card. Unlike `event_card_text_contains` (printed text only) this also scans the event card's NAME, aliases, and TRAITS, so an observer on "when you play a Digimon with [Knightmon]/[Lucemon] in its text" matches a trait-only card. Consumer: AD1-018 LordKnightmon. G-DSL-EVENT-CARD-IN-TEXT-CONTAINS. |
| `event_card_level_eq` | `i32?` | 0 | unused | — |  |
| `event_card_level_gte` | `DpConstraint?` | 0 | unused | — |  |
| `event_card_name_contains` | `str?` | 15 |  | `ad1/AD1-001.yaml` |  |
| `event_card_text_contains` | `str?` | 1 | rare | `ad1/AD1-018.yaml` | Case-insensitive substring scan against the triggering event card's PRINTED text (effect / inherited / security). Sibling of `event_card_name_contains` (which matches the NAME) and the event-side analogue of the static `effect_text_contains`. Gates observers on "when you play a card with <X> in its text". G-DSL-EVENT-CARD-TEXT-CONTAINS. |
| `event_card_trait_has` | `str?` | 6 |  | `_examples/BT13-007.yaml` |  |
| `event_cause` | `EventCauseSpec?` | 5 |  | `bt16/BT16-101.yaml` |  |
| `event_caused_by_own_effect` | `bool?` | 1 | rare | `st16/ST16-14.yaml` | For `on_discard_hand` observers: true when the causing effect belongs to the OBSERVER ("when one of YOUR effects trashes a card in your hand", ST16-14 Matt Ishida). Compares `discard_cause_controller` to the observer's controller. G-ENGINE-ON-DISCARD-HAND. |
| `event_discard_player` | `PlayerRef?` | 1 | rare | `st16/ST16-14.yaml` | For `on_discard_hand` observers: the player whose HAND was trashed (`TriggerContext.discard_hand_player`) must match this player-ref ("your hand is trashed from" ⇒ `you`). G-ENGINE-ON-DISCARD-HAND. |
| `event_host_is_own_tamer` | `bool?` | 2 | rare | `bt25/BT25-029.yaml` | `OnDigivolutionCardTrashed` observer gate: true when the trashing event's host permanent is a **Tamer owned by the observer** — i.e. "effects trash cards from under YOUR Tamers". Distinct from `event_host_permanent_is_source` (host == this exact permanent): this matches ANY of the controller's Tamers, which is what a Digimon-borne "trash from under your Tamers" observer needs (ST24-11 Rosemon clause 2, BT25-029 MirageGaogamon clause 2; DCGO `CanTriggerOnTrashDigivolutionCard(IsPermanentExistsOnOwnerBattleAreaTamer)`). |
| `event_host_permanent_is_source` | `bool?` | 7 |  | `bt18/BT18-065.yaml` | True when the triggering event's host permanent is this effect's source permanent. Used by OnDigivolutionCardTrashed observers that care about "this Digimon's digivolution cards" rather than any own stack. |
| `event_is_effect_initiated` | `bool?` | 13 |  | `ad1/AD1-024.yaml` |  |
| `event_permanent_is_source` | `bool?` | 25 |  | `ad1/AD1-021.yaml` |  |
| `event_target_color_any_of` | `[ColorSpec]?` | 10 |  | `bt11/BT11-089.yaml` | Match when the *event target* permanent's printed color set intersects this list — i.e. the digivolving / played / deleted / suspended permanent on the triggered-effect read context has at least one of the listed colors. Sibling of `event_target_kind` / `event_target_trait_has`, using the same `event_target_card` resolver. Used by BT13-012's inherited clause ("when one of your red or yellow Tamers becomes suspended"). G-EVENT-TARGET-COLOR. |
| `event_target_dp` | `MetricComparators?` | 0 | unused | — | Canonical uniform comparator for the trigger EVENT card's DP (unify-dsl-scalar-and-comparators §2 Stage A). Same single\|list shape as `dp`, lowered byte-identically into `event_target_dp_eq`/`_lte`/`_gte`. Legacy `event_target_dp_{eq,lte,gte}` keys still parse. |
| `event_target_dp_eq` | `DpConstraint?` | 0 | unused | — | Match the event target's effective DP. Deletion events read the deleted-object snapshot captured immediately before removal. |
| `event_target_dp_gte` | `DpConstraint?` | 1 | rare | `bt25/BT25-016.yaml` |  |
| `event_target_dp_lte` | `DpConstraint?` | 2 | rare | `st3/ST3-01.yaml` |  |
| `event_target_has_digivolution_cards` | `bool?` | 1 | rare | `ex1/EX1-066.yaml` | Deletion-subject source-count gate. True when the event-target permanent has at least one digivolution card (a non-empty source stack below the top). For a deletion event, reads the PRE-REMOVAL source count from the rule-25 deletion snapshot (`source_count_just_before`); for a live event target, reads the permanent's current `card_sources` stack. Driver: EX1-066 — "When one of your level 5 or higher Digimon **with a digivolution card** is deleted, …". G-DSL-EVENT-TARGET-SOURCE-COUNT. |
| `event_target_is_player` | `bool?` | 4 |  | `bt25/BT25-065.yaml` |  |
| `event_target_is_source` | `bool?` | 4 |  | `bt13/BT13-087.yaml` |  |
| `event_target_kind` | `CardKind?` | 75 |  | `ad1/AD1-014.yaml` |  |
| `event_target_level` | `MetricComparators?` | 1 | rare | `ex5/EX5-060.yaml` |  |
| `event_target_level_eq` | `i32?` | 2 | rare | `bt8/BT8-094.yaml` | Match the event-target permanent's printed level. Works for live event targets (played/digivolved/moved/suspended permanents) and deleted-object snapshots. G-EVENT-TARGET-LEVEL-LTE. |
| `event_target_level_gte` | `DpConstraint?` | 2 | rare | `ex1/EX1-066.yaml` |  |
| `event_target_level_lte` | `DpConstraint?` | 1 | rare | `bt8/BT8-094.yaml` |  |
| `event_target_name_contains` | `str?` | 2 | rare | `_examples/BT11-042.yaml` | Case-insensitive substring scan against the *event target* permanent's card name — i.e. the digivolving / played / deleted permanent carried on the triggered-effect read context. Used by EX4-061's clause 2 ("if that Digimon has [Greymon] in its name"). Sibling of `event_target_trait_has` / `event_target_kind`. G-EVENT-TARGET-NAME-CONTAINS. |
| `event_target_owner` | `PlayerRef?` | 110 |  | `_examples/BT11-042.yaml` |  |
| `event_target_same_level_as_previous` | `bool?` | 1 | rare | `bt9/BT9-092.yaml` |  |
| `event_target_stack_size_gte` | `i32?` | 0 | unused | — | Deletion-subject source-count threshold. True when the event-target permanent's digivolution-stack size (top card + sources) is at least `N`. Reads the PRE-REMOVAL count from the rule-25 deletion snapshot for a deletion event, or the live `card_sources` length otherwise. The count includes the top card, so a lone card is stack size 1. Companion of `event_target_has_digivolution_cards`. G-DSL-EVENT-TARGET-SOURCE-COUNT. |
| `event_target_trait_contains` | `str?` | 1 | rare | `bt11/BT11-089.yaml` | Substring / root-trait sibling of `event_target_trait_has`. Matches when ANY of the event-target permanent's traits CONTAINS this token (case-insensitive substring) — the observer-side analogue of the static `trait_contains` leaf. Tolerant of pluralization / compound traits: the token `Beast` matches `Beast`, `Beasts`, `Sea Beast`, etc. Works for live event targets AND deletion-object snapshots. Driver: BT11-089 — "when an effect plays any of your red Digimon with [Avian], [Bird], [Beast], [Animal] or [Sovereign] … in any of their traits" (official Q&A: "regardless of other words or pluralizations"). G-DSL-EVENT-TARGET-TRAIT-CONTAINS. |
| `event_target_trait_has` | `str?` | 23 |  | `ad1/AD1-019.yaml` |  |
| `event_target_was_self` | `bool?` | 0 | unused | — |  |
| `event_winner_owner` | `PlayerRef?` | 1 | rare | `bt25/BT25-020.yaml` | For `on_ally_won_battle` observers: the battle WINNER's controller must match this player-ref, resolved relative to the observer (`you` / `opponent`). Reads `TriggerContext.battle_winner`. Never matches on a tie (no winner). G-DSL-BATTLE-WINNER-BOARDWIDE. |
| `event_winner_trait_has` | `str?` | 1 | rare | `bt25/BT25-020.yaml` | For `on_ally_won_battle` observers: the battle winner's top card must carry this trait (case-insensitive). G-DSL-BATTLE-WINNER-BOARDWIDE. |

#### Identity / state — 60 (9 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `attack_target_change_reason` | `str?` | 1 | rare | `st5/ST5-14.yaml` |  |
| `attacker_trait_has` | `str?` | 1 | rare | `bt23/BT23-096.yaml` |  |
| `attribute_is` | `str?` | 2 | rare | `bt16/BT16-077.yaml` |  |
| `binding_card_color` | `BindingCardColorPredicate?` | 1 | rare | `bt1/BT1-087.yaml` | True when the card bound to `binding` shares ≥1 printed color with the `color_is` set (printed-color-set intersection). Sibling of `binding_card_kind` — resolves a named card binding (e.g. from a `reveal`/`add_to_hand` step's `bind_as`) and tests its `CardData.colors`. Fails closed when the binding is unset or the card can't be resolved. Driver: BT1-087 — "add 1 revealed security card to hand; if THAT card is yellow, trigger <Recovery +1>". G-DSL-BINDING-CARD-COLOR. |
| `binding_card_name_is` | `BindingCardNamePredicate?` | 1 | rare | `bt21/BT21-087.yaml` | True when the card bound to `binding` has the given name (exact, case-insensitive), effective-name aware — the printed `card_name` and every static `also_treated_as` identity alias are compared. Sibling of `binding_card_kind`. Fails closed when the binding is unset or unresolvable. Driver: BT21-087 — "play 1 [Vemmon] …" gating a bound revealed card by name. G-DSL-BINDING-CARD-NAME-EQUALS. |
| `can_digivolve_from_source` | `bool?` | 3 |  | `p/P-196.yaml` |  |
| `card_number_is` | `str?` | 57 |  | `bt13/BT13-083.yaml` |  |
| `color_is` | `ColorSpec?` | 389 |  | `_examples/BT11-042.yaml` |  |
| `color_matches_binding` | `str?` | 1 | rare | `p/P-156.yaml` |  |
| `color_matches_own_source_stack` | `SourceStackScope?` | 1 | rare | `ex9/EX9-074.yaml` | True when the candidate card shares ≥1 color with the effect CARRIER's NON-FLIPPED digivolution-source color set (the colors printed on the carrier's face-up digivolution cards, excluding the carrier's own top card). Mirrors DCGO `EX9_074.cs`: `card.PermanentOfThisCard().DigivolutionCards.Filter(!IsFlipped) .SelectMany(CardColors).Distinct()`. Authored as `color_matches_own_source_stack: { of: self }` — `of: self` marks the carrier (the effect's `source_permanent`); the scope is fixed there (there is no "opponent's source stack" reading in the printed corpus). The candidate side is kind-aware exactly like `color_matches_binding`. G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK. Driver: EX9-074 Kimeramon. |
| `color_matches_returned_card` | `bool?` | 1 | rare | `ex10/EX10-068.yaml` | True when the candidate card shares ≥1 color with ANY card recorded in this effect's `returned_to_deck` result log (the cards a preceding `return_trash_list_to_deck_bottom` / `return_all_trash_to_deck_bottom` moved). The returned card never becomes a permanent, so it cannot be a permanent binding — this leaf reads the result log directly rather than a binding name. Candidate side is kind-aware exactly like `color_matches_binding`. G-RETURNED-CARD-COLOR-BINDING (driver EX10-068). |
| `color_only` | `[ColorSpec]?` | 1 | rare | `ex8/EX8-045.yaml` |  |
| `digimon_attacked_this_turn` | `PlayerRef?` | 2 | rare | `st5/ST5-04.yaml` | True when the referenced player has attacked with at least one Digimon during the current turn. Supports normal `not` / `none_of` negation for printed text such as "if your opponent didn't attack with a Digimon this turn". |
| `distinct_tamer_colors_gte` | `i32?` | 1 | rare | `st20/ST20-10.yaml` | True when the observer's Tamers (battle-area Tamer permanents) collectively have at least N distinct colors. A no-subject global predicate — does not inspect the candidate. Used by ST20-10's warp-into-WarGreymon alt-path condition ("your Tamers have 3 or more total colors"). G-DSL-DISTINCT-TAMER-COLORS. |
| `dp` | `MetricComparators?` | 528 |  | `_examples/AD1-025.yaml` | Canonical uniform DP comparator (unify-dsl-scalar-and-comparators §2). `dp: { op: lte, value: 5000 }` or `dp: [{op: gte, value: 3000}, {op: lte, value: 8000}]` (a list expresses a range). Lowered at compile time into the same `dp_eq`/`dp_lte`/`dp_gte` compiled fields the legacy flat keys produce, so the compiled IR is byte-identical and engine eval is unchanged. The legacy `dp_eq`/`dp_lte`/`dp_gte` keys still parse (a predicate carrying both the canonical field and a legacy flat key is rejected by the lint, since they would silently merge). |
| `dp_eq` | `DpConstraint?` | 0 | unused | — |  |
| `dp_gte` | `DpConstraint?` | 13 |  | `bt13/BT13-111.yaml` |  |
| `dp_lte` | `DpConstraint?` | 66 |  | `_examples/BT17-015.yaml` |  |
| `face_down_sources_under_tamers_gte` | `i32?` | 3 |  | `bt25/BT25-035.yaml` | True when the observer's battle-area Tamer permanents collectively carry at least N face-down digivolution sources. A no-subject global predicate — does not inspect the candidate. Gates the `[Then]` clause of BT25-035 Cougarmon ("by trashing 2 bottom face-down cards from under any of your Tamers") so the optional digivolve is only offered when the trash-2 cost is actually payable. G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER. |
| `form_is` | `str?` | 0 | unused | — |  |
| `has_face_down_source` | `bool?` | 2 | rare | `st23/ST23-01.yaml` | Permanent-subject predicate. Matches whether the permanent's digivolution stack contains at least one face-down source. |
| `has_inherited` | `PredicateSpec?` | 3 |  | `ad1/AD1-002.yaml` |  |
| `has_keyword` | `str?` | 3 |  | `bt3/BT3-002.yaml` |  |
| `has_on_deletion_effect` | `bool?` | 1 | rare | `ex1/EX1-021.yaml` | Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) — true when the permanent's top card (or any card in its digivolution stack) has a triggered effect with `EffectTiming::OnDeletion` either via a compiled DSL clause or a hand-written `CardEffect` impl. Used by EX1-021 MetalGarurumon's "[When Attacking] return 1 opponent Digimon **that has an [On Deletion] effect** to the bottom of deck." DCGO `permanent.HasOnDeletionEffect`. |
| `has_security_attack_change` | `bool?` | 1 | rare | `bt10/BT10-042.yaml` | Permanent-subject predicate. True when the candidate currently has any Security Attack delta, whether from printed/granted `<Security A. +/-N>` keywords, temporary `SecurityAttackChange` modifiers, or formula-driven security-attack auras. |
| `in_text_contains` | `str?` | 8 |  | `bt18/BT18-065.yaml` | Whole-card "[X] in its text" scan — the broad DCGO `CardSource.HasText` surface (`CardSource.cs`). Unlike `effect_text_contains` (which scans ONLY effect / inherited / security text of the top card), this scans the card's **name**, **also-treated-as aliases**, **DigiXros aliases**, **traits (the Type line, incl. Rule-granted traits)**, and **all printed text** (effect / inherited / security, plus both dual faces). Case- insensitive substring. Required for the official ruling that "[X] in its text refers to a card that contains the text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements" — the structured requirement strings the CardData model carries live inside the printed text, and the trait line is scanned directly. Concrete regression: the [Three Musketeers]-TRAIT cards (BT6-017 MagnaKidmon, BT6-065 Gundramon, ST14-09 BeelStarmon) carry that trait but NOT the literal string in effect text — `effect_text_contains` misses them; `in_text_contains` matches via the trait scan. Driver family: BT21-098 Ragnarok Cannon ("[Vemmon] in its text"), 12 store-champs cards. G-DSL-IN-TEXT-CONTAINS. |
| `is_bottom_source` | `bool?` | 0 | unused | — | Source-subject predicate. Matches whether the source sits at `card_sources` index 0 (the bottom of the digivolution stack). Only meaningful when the predicate subject is a digivolution-stack source (e.g. inside a `select_own_sources` filter). |
| `is_face_down` | `bool?` | 0 | unused | — | Source-subject predicate (Tamer face-down stash). Matches `CardSource.face_down`. Only meaningful when the predicate subject is a digivolution-stack source (e.g. inside a `select_own_sources` filter). |
| `is_source_permanent` | `bool?` | 2 | rare | `bt24/BT24-062.yaml` | True when the subject permanent IS the effect's source permanent (the carrier/host). Lets `kind: flood_gate` / `kind: aura` target `self` — install a modifier on the carrier itself instead of scanning the whole board with an aux self-identity predicate. Pair with `scope: both` so the carrier resolves to the active top (face_up) AND the host of the digivolution stack (inherited). (BT24-062 attack-target lock.) |
| `is_suspended` | `bool?` | 23 |  | `ad1/AD1-012.yaml` |  |
| `is_unsuspended` | `bool?` | 29 |  | `bt13/BT13-095.yaml` |  |
| `kind` | `CardKind?` | 774 |  | `_examples/AD1-025.yaml` |  |
| `level` | `MetricComparators?` | 565 |  | `_examples/AD1-025.yaml` | Canonical uniform comparator for level (unify-dsl-scalar-and-comparators §2.4) and the trigger-event card's level (§2.2). NOTE: `eq` is literal-only here (the compiled `_eq` is `u8`); `lte`/`gte` are formula-capable. Lowered byte-identically into `level_*` / `event_target_level_*`. Legacy flat keys still parse. |
| `level_eq` | `i32?` | 483 |  | `_examples/AD1-025.yaml` |  |
| `level_eq_binding` | `str?` | 1 | rare | `bt17/BT17-078.yaml` |  |
| `level_gte` | `DpConstraint?` | 21 |  | `_examples/EX6-072.yaml` |  |
| `level_gte_binding` | `str?` | 0 | unused | — | Card/permanent-subject leaf: the subject's level must be `>=` the literal value bound to `binding`. Sibling of `level_eq_binding` / `level_lte_binding`. |
| `level_lte` | `DpConstraint?` | 58 |  | `ad1/AD1-014.yaml` |  |
| `level_lte_binding` | `str?` | 0 | unused | — | Card/permanent-subject leaf: the subject's level must be `<=` the literal value bound to `binding`. Sibling of `level_eq_binding` (which tests equality). Driver: BT8-107 — "delete 1 opponent Digimon with a level less than or equal to the deleted Digimon's level" (the deleted Digimon's level captured into a literal binding). |
| `name_contains` | `str?` | 165 |  | `_examples/AD1-025.yaml` |  |
| `name_in` | `[str]?` | 12 |  | `ad1/AD1-005.yaml` |  |
| `name_is` | `str?` | 113 |  | `_examples/BT12-112.yaml` |  |
| `name_not_shared_by_field_digimon` | `PlayerRefSelector?` | 1 | rare | `bt23/BT23-013.yaml` | Card-subject leaf: true when NO battle-area Digimon belonging to the scoped player shares the candidate card's name. Models the printed "This effect can't play cards with the same names as any of your Digimon" exclusion on the Jesmon family (BT23-013) — applied as a filter on a `select_union_zone` (hand+trash) play candidate set so the in-play names are masked out, never auto-picked. G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2). |
| `name_not_shared_by_field_tamer` | `PlayerRefSelector?` | 1 | rare | `bt24/BT24-034.yaml` | Card-subject leaf: true when NO battle-area Tamer belonging to the scoped player shares the candidate card's name. |
| `of_permanent` | `str?` | 17 |  | `_examples/BT18-102.yaml` |  |
| `play_cost` | `MetricComparators?` | 1 | rare | `st23/ST23-06.yaml` | Canonical uniform comparators for the identity metrics that the legacy flat surface left without an `_eq` operator (unify-dsl-scalar-and-comparators §2.4). `play_cost: { op: eq, value: 3 }`, ranges via a list, etc. Lowered byte-identically into the `<metric>_eq`/`_lte`/`_gte` compiled fields (the `_eq` compiled slots are new). Legacy `<metric>_lte`/`_gte` keys still parse. |
| `play_cost_eq_binding` | `PlayCostBindingPredicate?` | 1 | rare | `bt19/BT19-099.yaml` | Candidate-card play-cost comparator relative to a bound/event permanent's play cost, with an integer offset: `{ binding: replacement_subject\|event_target\|<name>, offset, op }`. The candidate matches when `candidate.play_cost OP (subject.play_cost + offset)`. Mirrors `level_eq_binding` resolution but for play cost and with a comparator + offset. Driver: BT19-099 — "play 1 [Wicked God] Digimon with a play cost 1 higher than that [leaving] Digimon". G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT. |
| `play_cost_gte` | `DpConstraint?` | 4 |  | `bt13/BT13-075.yaml` |  |
| `play_cost_lte` | `DpConstraint?` | 41 |  | `ad1/AD1-018.yaml` |  |
| `play_or_use_cost_lte` | `DpConstraint?` | 2 | rare | `bt25/BT25-073.yaml` | Card-subject leaf (G-PLAY-OR-USE-COST-LTE): true when the larger of the candidate's *play* cost (Digimon / Tamer) and *use* cost (Option / the Option face of a Dual) is at most this threshold. Mirrors DCGO `CardSource.GetCostItself <= N` over a "play or use 1 ... card with a play or use cost of N or less" hand filter (ST24-06 RizeGreymon). For a pure Option the play and use costs coincide; for a Dual it compares the max of both faces; for a Digimon / Tamer it is exactly `play_cost`. |
| `played_by_effect` | `bool?` | 0 | unused | — | True when the permanent carrying this effect was played by an effect (`PlaySource::ByEffect`) — read at the OnPlay firing. Models BT25-080's "if played by an effect, …" main-clause tail. G-ENGINE-ON-DISCARD-HAND. |
| `rules_text_contains` | `str?` | 2 | rare | `bt16/BT16-055.yaml` | True when the carrier Digimon's printed rules text (effect_text + inherited_text + security_text of the top card) contains the given substring (case-insensitive). Evaluated against the subject permanent in an inherited-aura `while_condition` context. Card driver: BT16-055 Namakemon — "[All Turns] While this Digimon has [Pulsemon] in its text, it gets +1000 DP." PUPPETS-G025. |
| `stack_size` | `MetricComparators?` | 0 | unused | — |  |
| `stack_size_gte` | `DpConstraint?` | 13 |  | `bt1/BT1-085.yaml` |  |
| `stack_size_lte` | `DpConstraint?` | 5 |  | `bt12/BT12-028.yaml` |  |
| `trait_contains` | `str?` | 2 | rare | `ex3/EX3-014.yaml` | Substring sibling of `trait_has`. Matches when ANY of the subject's traits CONTAINS this token (case-insensitive substring), mirroring DCGO `CardSource.ContainsTraits`. `trait_has` is an EXACT case-insensitive match; `trait_contains` is the substring reading demanded by printed text of the form "[Dragon], [saur] or [Ceratopsian] in any of its traits" — where e.g. `saur` only ever appears inside `Dinosaur` / `Plesiosaur` and `Dragon` mostly inside `Dragonkin` / `Dark Dragon`. Threaded identically to `trait_has`, including synth-identity / `ChangeTraits` overlay visibility. G-DSL-TRAIT-CONTAINS-SUBSTRING. Driver: EX3-014 Dorbickmon. |
| `trait_has` | `str?` | 399 |  | `_examples/BT11-042.yaml` |  |
| `trashed_source_card_id_is` | `str?` | 15 |  | `bt21/BT21-055.yaml` |  |
| `trashed_source_trait_has` | `str?` | 0 | unused | — |  |
| `would_link_card_trait_any_of` | `[str]?` | 2 | rare | `bt25/BT25-004.yaml` | True when the card about to link in the active `WhenWouldLink` window (the standing-Digimon link subject) carries AT LEAST ONE of the listed traits. Used by a host-side `when_would_link_to_this` reducer to gate on the linking card's traits — "when a [Social]/[Tool]/[Game] trait card would link to this Digimon" (Gap 5 — BT25-004 / BT25-045). `None` outside a standing-link `WhenWouldLink` window. |

#### Replacement payload — 3 (1 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `replacement_cause` | `ReplacementCauseSpec?` | 27 |  | `bt13/BT13-075.yaml` |  |
| `replacement_source_is_opponent` | `bool?` | 0 | unused | — |  |
| `replacement_subject_is_mine` | `bool?` | 33 |  | `bt13/BT13-075.yaml` |  |

#### Source-relative — 15 (2 unused)

| key | arg | uses | tag | fixture | description |
|-----|-----|------|-----|---------|-------------|
| `battle_opponent_no_sources` | `bool?` | 1 | rare | `st2/ST2-01.yaml` | True when this effect's carrier is currently battling an opposing Digimon with zero digivolution source cards. Used by inherited battle-only auras such as ST2-01 Tsunomon. |
| `host_kind_is` | `CardKind?` | 0 | unused | — | Source-subject predicate. Matches the `CardKind` of the host permanent's top card (e.g. `tamer` for a source stashed under a Tamer). Only meaningful when the predicate subject is a digivolution-stack source (e.g. inside a `select_own_sources` filter). |
| `host_permanent_trait_has` | `str?` | 14 |  | `bt21/BT21-055.yaml` |  |
| `self_color_count_gte` | `i32?` | 4 |  | `bt12/BT12-031.yaml` |  |
| `self_digivolution_contains_name` | `str?` | 6 |  | `bt12/BT12-059.yaml` |  |
| `self_digivolution_sources_contain_name` | `str?` | 9 |  | `_examples/BT20-083.yaml` | Like `self_digivolution_contains_name` but scans ONLY the digivolution *source* cards beneath the carrier — the carrier's own top card is excluded. `self_digivolution_contains_name` calls `Permanent::contains_card_name`, which scans the top card too, so a card named "Omnimon (X Antibody)" always self-matches "Omnimon" and the negative case ("no Omnimon among the digivolution cards") is inexpressible. BT20-102 needs the sources-only scan. G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY. |
| `self_digivolution_sources_trait_has` | `str?` | 5 |  | `bt13/BT13-075.yaml` | Like `self_digivolution_sources_contain_name`, but matches any digivolution source card carrying the named trait. Used by Royal Knights breeding-source effects to gate carriers that actually contain playable [Royal Knight] sources. |
| `self_source_count` | `SelfSourceCountPredicate?` | 3 |  | `bt18/BT18-065.yaml` | Source-relative threshold predicate — compares the number of the effect carrier's OWN digivolution source cards (those beneath its top card) that match `filter` against `value` under `op`. A no-subject global predicate: it does NOT inspect the candidate, it reads `ctx.source_permanent`. Reuses the same source-counting logic as the `source_stack_count` formula (`formula_eval`), so the two agree. Gates a conditional self-aura: BT21-006 Tsumemon inherited "[All Turns] This Digimon with 4 or more [Vemmon] digivolution cards gets +3000 DP" is `active_when: { self_source_count: { filter: { name_is: Vemmon }, op: gte, value: 4 } }`. DCGO `card.PermanentOfThisCard().DigivolutionCards.Count(cond) >= N`. G-DSL-SELF-SOURCE-COUNT-THRESHOLD (driver BT21-006). |
| `source_count` | `SourceCountPredicate?` | 1 | rare | `p/P-094.yaml` | Permanent-subject predicate. True when the candidate carries at least `at_least` digivolution SOURCE cards (the cards beneath its top card) matching the nested `filter`. Unlike `materials_count_gte` (which counts ALL sources by raw stack length), this counts only sources satisfying an arbitrary card predicate — the DCGO `DigivolutionCards.Count(predicate) >= N` idiom. Drives P-094 Destromon's inherited gate: "1 of your [Galacticmon]'s digivolution cards" must carry ≥2 [Vemmon] before the return-2-Vemmon cost is offered. The nested `filter` is evaluated against each source card (source subject), so it accepts `name_is` / `name_contains` / `trait_has` / `kind` / etc. G-DSL-SOURCE-COUNT-FILTERED. |
| `source_deleted_battle_opponent` | `bool?` | 7 |  | `bt16/BT16-004.yaml` | True when the current deletion event's target is this effect source's battle opponent and the source's carrier is still present. Used for inherited "deletes an opponent's Digimon in battle and survives" clauses. |
| `source_is_cost_target_permanent` | `bool?` | 18 |  | `bt11/BT11-061.yaml` | True when the effect's `source_permanent` is the (or one of the) permanent(s) being digivolved by the action whose cost is being computed. Use to gate "When THIS Digimon would digivolve into …" printed semantics so the observer / cost reducer only fires when its carrier permanent is actually the digivolution target. Single entry for normal digivolve; both DNA materials for DNA digivolve. Always false outside cost-calc dispatch and for effects whose `source_permanent` is `None`. G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (Phase 2 Track H closure). |
| `source_is_tamer` | `bool?` | 0 | unused | — |  |
| `source_is_unsuspended` | `bool?` | 31 |  | `_examples/BT20-083.yaml` |  |
| `source_name_contains` | `str?` | 14 |  | `_examples/BT17-015.yaml` |  |
| `source_permanent_trait_has` | `str?` | 24 |  | `ad1/AD1-013.yaml` |  |

### Timings (`when:`)

| key | uses | description |
|-----|------|-------------|
| `before_pay_cost` | 46 |  |
| `before_pay_cost_observe` | 2 | Sibling of `before_pay_cost` for observer-style triggered bodies (e.g. "[Your Turn] When this Digimon would DNA digivolve into a green Digimon card, gain 1 memory."). Fires at the same dispatch point as `before_pay_cost` but runs the clause's `process:` body instead of accumulating cost reduction. Pair with `cost_target: { ... }` predicates inside `active_when:` to gate on the digivolve-target card's traits/colors/level/name. G-BEFORE-PAY-COST-GAIN-MEMORY (Phase 2 Track H closure). |
| `counter` | 16 |  |
| `delayed` | 29 |  |
| `end_of_attack` | 14 |  |
| `end_of_battle` | 0 |  |
| `end_of_opponents_next_turn` | 7 |  |
| `end_of_opponents_turn` | 111 |  |
| `end_of_your_next_turn` | 13 |  |
| `end_of_your_turn` | 35 |  |
| `main` | 42 |  |
| `main_from_hand` | 118 |  |
| `main_from_trash` | 0 |  |
| `main_on_field` | 15 |  |
| `on_add_to_hand` | 3 | `[All Turns]`-style observer: an EFFECT added one or more cards to a player's hand (return-to-hand, security/trash/deck/reveal-to-hand, …). Gate the gaining player with `event_add_to_hand_player:` and effect-vs-draw with `event_is_effect_initiated:` inside `active_when:`. See G-ON-ADD-TO-HAND-OBSERVER. |
| `on_added_to_security` | 4 |  |
| `on_ally_attack` | 7 |  |
| `on_ally_played` | 12 |  |
| `on_ally_won_battle` | 1 | Board-wide battle-winner observer: "[All Turns] When any of your [X] Digimon win a battle, …" (`when: on_ally_won_battle`, BT25-020 Marsmon; G-DSL-BATTLE-WINNER-BOARDWIDE). Rides the `EndOfBattle` dispatch with NO forced self-filter (unlike the carrier-scoped `source_deleted_battle_opponent` idiom on `on_any_deletion`). Gate scope via `active_when:` — `event_winner_owner: you` (the winner's controller) and `event_winner_trait_has: <trait>` (the winner's trait). Never fires on a tie (mutual destruction — no winner) or a direct player attack. |
| `on_any_deletion` | 27 |  |
| `on_any_digimon_played` | 11 |  |
| `on_any_link` | 7 | DigiLink board-wide observer: "[Your Turn] When your Digimon get linked, …" (`when: on_any_link`). Fires for EVERY link event the engine dispatches — no forced self/host filter (unlike `when_linked` / `when_card_linked_to_this`). Gate the scope with `active_when:` predicates: `event_target_owner: you` (the link HOST's controller), `event_card_trait_has:` (the just-linked card's traits), and `your_turn: true`. Mirrors DCGO `CanTriggerWhenLinked` with a board-wide `PermanentCondition` (BT21-084 / BT21-101 / P-217 / P-241). G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED. |
| `on_attack` | 1 |  |
| `on_attack_target_change` | 4 |  |
| `on_block` | 2 |  |
| `on_check_face_up_security` | 1 |  |
| `on_deletion` | 54 |  |
| `on_digivolution_card_trashed` | 26 |  |
| `on_digivolve` | 35 |  |
| `on_digixros` | 0 |  |
| `on_discard_hand` | 1 | Hand-discard observer: "[All Turns] When your hand is trashed from, …" (`when: on_discard_hand`, BT25-080/084; ST16-14; G-ENGINE-ON-DISCARD-HAND). Fires ONCE after an EFFECT trashes ≥1 card from a player's hand (a draw, mulligan, or rule discard never fires it). Gate with `event_discard_player: you` ("your hand", BT25-080/084) and, for own-effect-only ("one of YOUR effects", ST16-14 Matt Ishida), `event_caused_by_own_effect: true`. |
| `on_discard_security` | 6 |  |
| `on_dna_digivolve` | 6 |  |
| `on_enter_field_anyone` | 26 |  |
| `on_hatch` | 1 |  |
| `on_leave_field` | 1 |  |
| `on_lose_security` | 6 |  |
| `on_move` | 13 |  |
| `on_opponent_attack` | 20 |  |
| `on_opponent_security_removed` | 20 |  |
| `on_option_placed` | 2 |  |
| `on_option_trashed` | 2 |  |
| `on_own_security_removed` | 18 |  |
| `on_place_security` | 0 |  |
| `on_play` | 293 |  |
| `on_security` | 212 |  |
| `on_security_check` | 0 |  |
| `on_source_returned_to_deck_bottom` | 2 | `[All Turns]`-style observer: a digivolution source was RETURNED to the bottom of a player's deck (not trashed) — "when any [Vemmon] return to the bottom of the deck from this Digimon's digivolution cards" (BT21-058, BT18-065). Gate host-scope with `active_when: { event_host_permanent_is_source: true }` and the returned card's name with `event_card_name_contains:`. Distinct from `on_digivolution_card_trashed`. G-ENGINE-DIGIVOLUTION-CARD-RETURNED-TO-DECK-BOTTOM. |
| `on_suspend` | 34 |  |
| `on_unsuspend` | 2 |  |
| `start_of_opponents_turn` | 1 |  |
| `start_of_your_main_phase` | 64 |  |
| `start_of_your_turn` | 33 |  |
| `until_next_unsuspend` | 0 |  |
| `when_attacking` | 145 |  |
| `when_card_linked_to_this` | 13 | DigiLink host-side: "[When Linked] when a card gets linked **to this Digimon**" (`when: when_card_linked_to_this`). The effect lives on the HOST (a face-up `scope`), not on the linked card. Lowers to `OnLink` + a host self-filter (`event_permanent == source_permanent`) so it fires once for the host the card actually attached to and not for a sibling host. Mirrors DCGO `CardEffectCommons.CanTriggerWhenLinked`. |
| `when_digivolving` | 286 |  |
| `when_linked` | 16 | DigiLink Shape-B: "when this Digimon gets linked" (`when: when_linked`). Use on a `scope: linked` effect; lowers to `OnLink` + a self-filter. |
| `when_would_link_to_this` | 2 | DigiLink host-side pre-link **replacement**: "when a card **would** link **to this Digimon**" (`when: when_would_link_to_this`). The effect lives on the HOST (a face-up `scope`). Lowers to a `WhenWouldLink` REPLACEMENT effect (not a triggered observer) + a host self-filter (`pending_link_host() == source_permanent`) so it fires only while the linking card is attaching to THIS permanent. Pair with an `optional` clause + a `reduce_link_cost` step to express "you may reduce the cost" (Gap 5 — BT25-004 Tapmon / BT25-045 Onmon). Filter the would-link card's traits via `active_when: { would_link_card_trait_any_of: [...] }`. |

### Declarative kinds (`kind:`)

| key | uses | description |
|-----|------|-------------|
| `ace_overflow` | 19 |  |
| `alt_path_registration` | 6 |  |
| `aura` | 126 |  |
| `cost_reduction` | 63 |  |
| `delay` | 47 |  |
| `flood_gate` | 47 |  |
| `grant_keyword` | 280 |  |
| `link_condition` | 21 |  |
| `link_requirement` | 9 |  |
| `partition` | 9 |  |
| `raw_rust` | 15 |  |
| `replacement` | 67 |  |

<!-- END GENERATED:dsl-vocab -->
