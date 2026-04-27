---
title: Declarative YAML card-scripting DSL for the Rust engine
date: 2026-04-21
status: design
area: digimon-engine
supersedes: —
related:
  - docs/RUST_ENGINE_API.md
  - docs/RUST_PYTHON_PARITY.md
  - .claude/plans/rust-engine-gaps-dna-omnimon.md
  - .claude/plans/rust-engine-gaps-dark-masters.md
  - .claude/plans/rust-engine-gaps-medusamon.md
  - .claude/plans/rust-engine-gaps-rocks.md
  - .claude/plans/rust-engine-gaps-ts-olympos.md
---

# Card-scripting DSL — design spec

## 0. TL;DR

Hand-writing a `CardEffect` struct in Rust for every one of ~4,000 cards is a
dead end: it bloats `digimon-engine/`, slows LLM authoring, blocks hot-reload
during RL training, and ties every card revision to a `cargo rebuild`. This
spec defines a **declarative YAML DSL** that compiles ahead-of-time (AOT) into
`Effect` closures against the existing `EffectContext` API. The DSL is the
authoring surface; Rust is the runtime surface.

Decisions locked in this spec (not up for re-litigation during planning):

- **Evaluator:** AOT compiler at `CardRegistry::load()`, not an interpreter.
- **Escape hatch:** hybrid YAML + `raw_rust:` fallback registry, composable
  at *both* clause granularity (`kind: raw_rust`) and `process:` step
  granularity (one step invokes a named Rust fn, bindings flow through). No
  Rhai, no embedded scripting runtime.
- **No-approximations policy:** unchanged. DSL `select_*` verbs lower to the
  same `PendingSelection` primitives a hand-written script would call. The
  DSL is forbidden from inventing auto-selections it cannot express in
  `EffectContext`.
- **Parity contract:** tensor, action mask, and policy training are
  invariant across "hand-written CardEffect" and "DSL-compiled CardEffect"
  backends for the same card text.

Non-goals (will bounce back to scope owners):

- Redesigning engine primitives. The DSL consumes gaps, it does not remove
  them. The Tier-1..Tier-6 gap list in the archetype gap plans is *still
  required engine work* before the DSL can reach 95% coverage.
- Authoring cards. That is `/batch-implement-cards-rust-dsl`'s job.
- LLM prompt templates for authoring. Separate effort once vocabulary is
  frozen at end of Phase 1.
- Runtime modification of the DSL schema. Schema changes are compile-time
  events that bump a card-pack version.

## 1. Goals and non-goals

### 1.1 Goals

1. **Replace hand-written `CardEffect` structs for 95–99% of the card pool
   with data, not code.** The residual 1–5% uses the `raw_rust:` escape
   hatch, either as a whole-clause fallback or as a single step inside an
   otherwise YAML clause.
2. **Expose a bounded, validated vocabulary** that LLM agents can author
   against from card text. Vocabulary growth is a cost; every new verb or
   predicate is a schema change reviewed at Phase boundaries.
3. **Hot-reload card definitions during RL training** without a
   `cargo rebuild`. The training loop reads YAML, compiles to closures,
   swaps the `CardEffect` trait-object behind the existing registry lock.
4. **Preserve determinism and tensor/mask parity.** The DSL evaluator
   routes all randomness through `Game::rng`; it never invents
   randomness. Every DSL-compiled effect produces the same observation
   tensor and action mask as a hand-written Rust equivalent given the
   same game state.

### 1.2 Non-goals

- **Engine primitive design.** If a mechanic requires a new `EffectContext`
  method, a new `EffectTiming`, or a new `SelectionKind`, that work lives in
  the engine gap plans (`.claude/plans/rust-engine-gaps-*.md`) — the DSL
  consumes whatever the engine exposes.
- **Cards needing cross-card runtime introspection beyond what the engine
  exposes.** The one clear case is BT10-111 Shoutmon (King Version)'s
  "replace 1 of the DigiXros requirements" which demands a card-to-card
  wildcard substitution the engine has no primitive for today. That card
  goes through the `raw_rust:` escape hatch.
- **Replacing the `DCGO/` reference corpus.** DCGO remains the behavioral
  source of truth; DSL authoring still reads C# to disambiguate card text
  edge cases.
- **A domain-specific runtime.** There is no VM, no bytecode, no dynamic
  dispatch beyond the existing `Box<dyn CardEffect>`. A DSL-compiled card
  is a regular `CardEffect` implementation after `load()` returns.

## 2. Evidence for viability

### 2.1 Vocabulary growth curve (34-card exploration)

An exploratory pass over 34 cards sampled across 8 archetypes and every
major mechanic (DigiXros, DNA Digivolve, Burst Mode, Hybrid/Legendary
Warriors, Ace Overflow, X-Antibody, Royal Knight, Olympos XII, Medusamon
LIBERATOR, Partition, Decode-adjacent, Rosemon Burst teardown) measured
the number of new vocabulary items required per additional card.

| Cards explored | Cumulative vocab | Items per new card |
|----------------|------------------|--------------------|
| 1              | 8                | 8.00               |
| 4              | 29               | 7.25               |
| 10             | 52               | 5.20               |
| 20             | 96               | 4.80               |
| 34             | 172              | 5.06               |

The curve is **sub-linear and flattening**. Extrapolating to 4,000 cards
projects a stable ceiling of **~180 vocabulary items** — ~30 timings,
~15 clause kinds, ~30 keywords, ~50 mutation verbs, ~40 filter
predicates, ~20 modifiers, ~15 expiries, ~10 structural primitives.
The per-card slope stays under 0.05 items/card past card 34 because new
cards reuse existing vocab and rarely contribute a primitive not already
seen in the 34-card corpus.

### 2.2 Five inline worked sketches (complexity ladder)

Each sketch compiles against today's (or imminent) `EffectContext`. Card
text is from `digimon_gym/engine/data/cards.json` as of 2026-04-21.
Detailed walkthroughs for all 15 worked cards live in §10.

**Tier 0 — Hello world (`ST2-13 Hammer Spark`):** `[Main] Gain 1 memory.`
plus `[Security] Gain 2 memory.`

```yaml
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - gain_memory: 1
  - when: on_security
    process:
      - gain_memory: 2
```

**Tier 1 — Simple searcher (`BT17-007 Agumon`):** triggered, name-filtered
trash-to-hand.

```yaml
card: BT17-007
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0
effects:
  - when: start_of_your_main_phase
    optional: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Tai Kamiya"
    process:
      - select_trash:
          of: you
          bind_as: selected
          filter:
            any_of:
              - name_contains: "Garurumon"
              - name_contains: "Greymon"
              - name_contains: "Omnimon"
          prompt: "Return a card to hand"
      - add_to_hand_from_trash: { card: selected }
  - scope: inherited
    when: end_of_your_turn
    kind: alt_path_registration
    registers:
      kind: dna_digivolve
      target_zone: hand
```

**Tier 2 — Declarative aura (`BT5-093 Tai & Matt`):** persistent
Omnimon-named Security Attack buff.

```yaml
card: BT5-093
name: Tai Kamiya & Matt Ishida
kind: tamer
color: [red, blue]
cost: 4
effects:
  - when: start_of_your_turn
    condition:
      any_permanent:
        of: opponent
        zone: [battle_area]
        level_gte: 6
    process:
      - gain_memory: 2
  - kind: aura
    active_when: your_turn
    target:
      of: you
      zone: [battle_area]
      name_contains: "Omnimon"
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }
```

**Tier 3 — Boss triggered effect with cost hook (`BT17-015 WarGreymon`):**
cost reduction + on-play effect choice + inherited once-per-turn.

```yaml
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
traits: [Dragonkin]
alt_paths:
  - kind: digivolve
    from: { level_eq: 5, name_contains: "Greymon" }
    cost: 3
effects:
  - kind: cost_reduction
    scope: before_pay_cost
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Tai Kamiya"
    amount: 3
  - when: [on_play, when_digivolving]
    process:
      - select_effect_choice:
          bind_as: branch
          labels:
            - "Delete opponent Digimon (≤8000 DP)"
            - "Digivolve Gabumon into MetalGarurumon free"
      - if: { equals: [branch, 0] }
        then:
          - select_opponent_permanent:
              bind_as: target
              filter:
                all_of:
                  - kind: digimon
                  - dp_lte: 8000
              prompt: "Delete a Digimon (≤8000 DP)"
          - delete_permanent: { target: target }
      - if: { equals: [branch, 1] }
        then:
          - select_own_permanent:
              bind_as: base
              filter: { name_contains: "Gabumon" }
              prompt: "Choose Gabumon"
          - select_hand:
              bind_as: evo
              filter: { name_contains: "MetalGarurumon" }
              prompt: "Digivolve into…"
          - effect_initiated_digivolve:
              target: base
              from_hand: evo
              cost: 0
              ignore_requirements: true
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    condition: { source_name_contains: "Omnimon" }
    process:
      - trash_top_security: { of: opponent }
```

**Tier 4 — Stress test, hybrid composition (`BT13-007 King Drasil_7D6`):**
breeding-scoped locks, Royal-Knight cost reduction with a per-source
formula, and a start-of-main-phase Digi-Egg reveal + batch placement.
The Royal Knight cost-reduction trigger is YAML; the per-source formula
uses a single `raw_rust:` step because it needs a stack-size lookup
that is not expressible as a pure filter aggregate (it is — see §3.5 —
but we exhibit the hybrid form here to show the pattern).

```yaml
card: BT13-007
name: King Drasil_7D6
kind: digi_egg
color: [yellow]
cost: 0
effects:
  - kind: flood_gate
    scope: face_up        # applies while in breeding
    active_when: { all_of: [in_breeding, your_turn] }
    modifier: CannotDigivolve
    target: { of: you, zone: [battle_area] }

  - kind: cost_reduction
    scope: before_pay_cost
    active_when: { all_of: [in_breeding, your_turn] }
    once_per_turn: true
    when_any_ally_played: { trait_has: "Royal Knight" }
    amount_fn:
      raw_rust: bt13_007_royal_knight_cost_reduction   # returns 4 + stack_size

  - when: start_of_your_main_phase
    scope: face_up
    active_when: in_breeding
    process:
      - reveal_top_deck: { of: you, zone: digi_egg_deck, count: 1, bind_as: shown }
      - place_as_bottom_source:
          source: shown
          target: { source_permanent: self }
      - for_each:
          over: { of: you, zone: [battle_area], trait_has: "Royal Knight" }
          bind_as: rk
          body:
            - place_as_bottom_source:
                source: { permanent: rk }
                target: { source_permanent: self }

  - scope: inherited
    when: on_option_placed
    active_when: { all_of: [in_breeding, your_turn] }
    once_per_turn: true
    condition: { event_card_trait_has: "Royal Knight" }
    process:
      - gain_memory: 1
```

The `amount_fn: { raw_rust: bt13_007_royal_knight_cost_reduction }` is the
fine-grained hybrid escape hatch (§6). The fn signature is `fn(&EffectReadContext) -> i32`;
it reads `source_permanent().stack_size()` and returns `4 + stack_size`.

### 2.3 Consumed `cards.json` fields

Authoritative structured data on every card record is consumed by the DSL
**instead of** re-parsing `effect_description_eng` where possible. This
narrows the surface that must be expressed in YAML:

| `cards.json` field                     | DSL consumption                                 |
|----------------------------------------|-------------------------------------------------|
| `card_id`                              | primary key                                     |
| `card_name_eng`                        | `name:`                                         |
| `card_kind`                            | `kind:`                                         |
| `play_cost`                            | `cost:`                                         |
| `dp`                                   | `dp:`                                           |
| `level`                                | `level:`                                        |
| `card_colors`                          | `color:`                                        |
| `type_eng`                             | `traits:` — **critical**; drives trait filters  |
| `form_eng`                             | available as filter predicate                   |
| `attribute_eng`                        | available as filter predicate                   |
| `evo_costs`                            | primary digivolve path — auto-lowered to `alt_paths` without requiring YAML |
| `xros_req`                             | DigiXros, DNA, Burst, App Fusion, Hybrid paths populated on ~1,155 cards — see §3.3 |
| `effect_description_eng`               | authoring reference only; parsed into `effects:` list by humans/LLMs |
| `inherited_effect_description_eng`     | same, lowered into `effects:` with `scope: inherited` |
| `security_effect_description_eng`      | same, lowered into `effects:` with `when: on_security` |

`xros_req` text parsing — whether this happens at DSL load time, at ingest
time into a structured field on `CardData`, or by convention in authoring
tools — is an **open question** deferred to §9.

### 2.4 Load-time cost model

With 4,000 cards averaging ~4 clauses per card and ~16,000 total `Effect`
closures to synthesize, AOT compilation at `CardRegistry::load()` is
budgeted at:

- **Parse**: ~30 ms (serde_yaml on ~20 MB of YAML).
- **Validate**: ~40 ms (schema check + filter type check + timing reachability).
- **Lower**: ~30 ms (clause-tree → closure tree; all closures are plain
  owned Rust values, no codegen to disk).

Total: ~100 ms one-time at process startup. Hot-reload during RL training
costs the per-card delta (~25 μs/card). The hot path — `CardEffect::effects()`
called per play — is a cloneable `Vec<Effect>` handoff identical in shape to
what a hand-written `CardEffect` returns; there is no DSL overhead after
`load()`.

## 3. DSL surface specification

This section is a contract. Downstream authoring tools (LLM prompts,
schema validators, IDE tooling) consume it as authoritative.

### 3.1 File layout

One YAML file per card under `digimon-engine/cards/<set>/<card_id>.yaml`.
A card-pack manifest `digimon-engine/cards/manifest.yaml` lists sets and
versions. Tokens live under `digimon-engine/cards/_tokens/<name>.yaml`.
Identity-aliased cards (X-Antibody, etc.) are authored as a single file
per printed card; the alias is expressed as an `identity:` section, not a
second file.

### 3.2 Top-level sections

| Field         | Required | Type                                     | Notes                                          |
|---------------|----------|------------------------------------------|------------------------------------------------|
| `card`        | yes      | string (card_id)                         | Must match `cards.json` `card_id`              |
| `name`        | yes      | string                                   | Authoring-only; validated against `cards.json` |
| `kind`        | yes      | enum(`digimon`/`tamer`/`option`/`digi_egg`/`token`) | Drives which sections are legal       |
| `level`       | digimon/digi_egg | int                              | For filter matching                            |
| `color`       | yes      | list of color enum                       | Multi-color cards have >1                      |
| `cost`        | digimon/tamer/option | int                          | Printed play cost                              |
| `dp`          | digimon  | int                                      | Printed DP                                     |
| `traits`      | optional | list of string                           | From `type_eng`; no free-form new traits       |
| `form`        | optional | string                                   | From `form_eng`                                |
| `attribute`   | optional | string                                   | From `attribute_eng`                           |
| `identity`    | optional | object — see §3.4                        | Name aliases (X-Antibody, etc.)                |
| `alt_paths`   | optional | list of object — see §3.3                | Digivolve/DNA/DigiXros/Burst/Hybrid/App Fusion |
| `effects`     | optional | list of object — see §3.5                | Triggered + declarative clauses                |
| `ace_overflow`| optional | int (negative)                           | Ace `<-N>` — lowered to an on-leave clause     |

### 3.3 `alt_paths:` — evolution and assembly entry points

`alt_paths:` replaces *every* mechanism by which a card enters the battle
area other than paying its printed `cost:` from hand.

| Kind                | Drives                                                                 | Required fields                                                                                                                                   |
|---------------------|------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `digivolve`         | Standard digivolution from battle-area                                 | `from:` (filter), `cost:` (int or `formula:`)                                                                                                    |
| `dna_digivolve`     | DNA (2 materials)                                                      | `materials:` (list of 2 filters), `cost:` (int). `source_treated_as` optional — e.g. X-Antibody identity.                                         |
| `digixros`          | DigiXros (1..∞ materials, `distinct_by` rules)                         | `materials:` (list of filters; each entry may carry `repeat: unbounded`, `distinct_by: card_number`). `cost:` (int or `{ per: ..., formula: }`). |
| `burst_digivolve`   | Burst Digivolve from battle-area + extra cost (return X to hand etc.) | `from:` filter, `cost:` int, `extra_cost:` list, optional `on_burst_turn_end:` block                                                              |
| `app_fusion`        | App Fusion (materials across zones)                                    | `materials:` (list of filters, any zone). `cost:` int.                                                                                           |
| `assembly`          | Hybrid / Legendary Warrior (materials + named Tamer)                   | `materials:` (list of filters), `from:` filter, `cost:` int                                                                                       |
| `activated_digivolve` | `[Hand] [Main]` "digivolves into this" self-replacing evolution      | `from:` filter, `cost:` int, optional `extra_cost:` list (e.g. Lamiamon "place Dimetromon from trash as bottom source"), `ignore_requirements:` bool |

Example — DigiXros ∞ (BT12-112 Shoutmon X7: Superior Mode):

```yaml
alt_paths:
  - kind: digixros
    materials:
      - filter:
          any_of:
            - trait_has: "Xros Heart"
            - trait_has: "Blue Flare"
        repeat: unbounded
        distinct_by: card_number
    cost:
      formula:
        base: 15
        per: material_count
        delta: -1
```

Example — Burst Digivolve with turn-end teardown (BT13-060 Rosemon: Burst Mode):

```yaml
alt_paths:
  - kind: burst_digivolve
    from: { name_is: "Rosemon" }
    cost: 0
    extra_cost:
      - return_to_hand:
          target:
            of: you
            zone: [battle_area]
            kind: tamer
            name_is: "Yoshino Fujieda"
    on_burst_turn_end:
      - trash_top_source: { target: self }
```

`extra_cost:` uses the exact mutation vocabulary as `process:` (§3.7) —
there is no separate "cost verb" vocabulary. This reuses `pay_cost_fn` on
the underlying `Effect` struct.

### 3.4 `identity:` — name aliases

Used almost exclusively for X-Antibody cards (~23 cards) whose digivolve
and DNA paths treat them as having the un-X-Antibody printed name. The
identity block registers name-overlay rules enforced by filter predicates.

```yaml
identity:
  name_aliases:
    - treat_as: "Omnimon"
      when:
        zone: [battle_area]
        has_inherited: { card_number_is: "BT9-109" }   # printed-name guard
```

### 3.5 `effects:` — clause list

Each entry in `effects:` is one clause. Clauses come in two families:

1. **Triggered clauses** — have a `when:` timing, optional `condition:`,
   optional `optional:` / `once_per_turn:` / `max_per_turn:` flags, and a
   `process:` list of mutation steps (§3.7).
2. **Declarative clauses** — have a `kind:` discriminator. No `when:` or
   `process:`; semantics are fixed by the kind.

**Clause ordering (locked, replay-deterministic):** when two or more
clauses on the **same card** share a timing, they fire in the **order
they appear in the YAML file**. Cross-card same-timing ordering follows
the engine's existing trigger-queue rules (controller turn order, then
permanent index, then enqueue order — see `effect_queue.rs`); the YAML
order rule applies only within a single card's `effects:` list. This
property is load-bearing for replay determinism (a re-ordered YAML file
is a different card from the engine's perspective; the AOT compiler
preserves order). Authoring tools and LLM agents must not reorder
clauses without intent.

#### 3.5.1 Triggered-clause schema

| Field           | Required | Type                                                      | Notes                                                   |
|-----------------|----------|-----------------------------------------------------------|---------------------------------------------------------|
| `when`          | yes      | string or list of string (§3.6 timing table)              | List means "fire on any of these timings"               |
| `scope`         | optional | `face_up` (default) / `inherited` / list of both           | `inherited` clauses run while this card is a digivolution source under another permanent |
| `active_when`   | optional | compound predicate — §3.8                                 | Adds to `condition`; gates the clause by zone/turn flags |
| `condition`     | optional | filter predicate — §3.8                                   | Runs via `Effect::condition` on `EffectReadContext`     |
| `optional`      | optional | bool (default false)                                      | Surfaces a yes/no `select_effect_choice` before `process` |
| `once_per_turn` | optional | bool (default false)                                      | Lowers to `Effect::once_per_turn()`                     |
| `max_per_turn`  | optional | int                                                       | Overrides `once_per_turn`                               |
| `process`       | yes      | list of step — §3.7                                       | Empty allowed only for clauses whose entire body is a `raw_rust:` invocation |

#### 3.5.2 Declarative-clause kinds

| `kind`                | Purpose                                                                                      | Required fields                                                            |
|-----------------------|----------------------------------------------------------------------------------------------|----------------------------------------------------------------------------|
| `aura`                | Persistent modifier applied to a target set while `active_when` holds                        | `target:` filter, one of `dp_modifier` / `grant_keyword` / `modifier`      |
| `cost_reduction`      | `BeforePayCost` cost hook — static or formula; optional cost-resolution body + post-cost unlocks | `when_playing_this` OR `when_any_ally_played`, `amount` or `amount_fn`; optional `pay_cost:` (list of mutation steps performed at cost-resolution) and `unlocks:` (list of structured directives that mutate the play context after the cost resolves — see §3.5.4) |
| `replacement`         | Interrupt + prompt + cancel-or-redirect (Evade, Partition, WouldBeDeleted)                   | `trigger:` (e.g. `would_be_deleted_in_battle`), optional `provenance:` filter (matches the leaving permanent's stack source), `process:` steps |
| `partition`           | **Sugar over `replacement`** — see §3.5.5; lowers to a `replacement { trigger: would_leave_field, provenance: <sources>, process: [play_from_materials_free] }` clause with the Partition keyword's built-in exclusions (`Battle`) auto-applied | `sources:` list of filters |
| `ace_overflow`        | Declarative `<Ace -N>`                                                                       | `value:` int (negative)                                                    |
| `grant_keyword`       | Declarative `<Blocker>`, `<Rush>`, etc.                                                      | `keyword:` enum, optional `value:` for parametric                          |
| `delay`               | Option-side `<Delay>` — persistent Option placement                                          | `trigger:` when to activate, `process:` steps                              |
| `flood_gate`          | Player-scoped `CannotX` modifier                                                             | `modifier:` enum, `active_when:` required                                  |
| `alt_path_registration` | Inherited alt-path registration — "any of your Digimon may DNA digivolve into hand"; see §3.5.6 for `applies_to:` projection semantics | `registers:` mini alt-path block; optional `applies_to:` filter (defaults to "every permanent the carrier owns")  |
| `raw_rust`            | Escape hatch — invoke a named Rust fn as a whole clause                                      | `fn:` string (must be registered in the `CardEffectExtensionRegistry`)     |

#### 3.5.3 Scope semantics

- `face_up` (default) — clause is active only while this card is a
  face-up card on the field (top of stack, in breeding, Tamer zone). It is
  **not** active while this card is a digivolution source under another
  permanent.
- `inherited` — clause is active only while this card is **under** another
  permanent as a digivolution source. The `carrier` binding is the
  permanent holding the source. `source` remains this card.
- `[face_up, inherited]` — active in both positions. Used rarely (Ace
  Overflow `inherited` sugar is handled separately).

#### 3.5.4 `cost_reduction` — `pay_cost:` body + `unlocks:` directives

Some cards (the X7 / DigiXros family) couple a `BeforePayCost` discount
with a synchronous mini-process that runs **at cost resolution** and a
set of **directives that mutate the play context** for that single play.
Both fields are optional and only apply to `when_playing_this: true`
clauses.

- `pay_cost:` — list of `process:` steps (§3.7) that run after the
  amount discount is applied but before the engine commits the play.
  Bindings created here are visible to `unlocks:` directives that follow.
  If any step parks a selection, the play is suspended until the
  selection resolves; on decline, the entire play is cancelled (the cost
  reduction does not apply, the player keeps their memory, the card
  returns to its source zone).
- `unlocks:` — list of structured directives that the engine consumes
  before re-evaluating play legality. Each directive is one of:

| Directive                  | Shape                                       | Effect                                                                                   |
|----------------------------|---------------------------------------------|------------------------------------------------------------------------------------------|
| `digixros_zones_extend:`   | list of zone enums                          | For this play only, the DigiXros material search includes these additional source zones. |
| `digixros_count_minus:`    | int                                         | Reduce the required material count for this play by N (floored at 0).                    |
| `treat_source_as:`         | `{ name_is: <string> }`                     | Override the played card's name for this play's alt-path matching (X-Antibody pattern).  |
| `add_play_color:`          | list of color enum                          | Add temporary colors to the source for color-requirement matching.                       |
| `raw_rust:`                | `{ fn: <name>, args: {...} }`               | Escape hatch — see §6.                                                                   |

Directive list is closed; new directives are a schema change. The
validator rejects unknown directive keys.

#### 3.5.5 `partition` — sugar over `replacement`

`partition` is **deprecated as a separate clause kind** and exists only
for backwards-compatible authoring. The validator emits an info-level
diagnostic suggesting the equivalent `replacement` shape.

The Partition keyword carries its own rule-mandated exclusions — most
notably "does not trigger on battle deletion". These exclusions are
**not** authored on the clause; they are baked into the lowering and
apply uniformly to every Partition card. An author writing `kind:
partition` writes only the `sources:` list:

```yaml
# Authored:
- kind: partition
  sources: [{ name_is: "Royal Knight" }, { trait_has: "Olympos XII" }]

# Equivalent (preferred — what it lowers to):
- kind: replacement
  trigger: would_leave_field
  exclude_cause: [Battle]   # ← injected by the lowering, not authored
  provenance:
    any_of:
      - { name_is: "Royal Knight" }
      - { trait_has: "Olympos XII" }
  process:
    - play_from_materials_free: { source: { binding: provenance_match } }
```

The `provenance:` filter matches against the leaving permanent's stack
sources (top + every digivolution source); the matched source becomes
the implicit `provenance_match` binding. This unifies `partition` with
the Phase 7 engine work (`WhenWouldBeDeleted` with provenance filter)
so there is one underlying primitive rather than two.

**Authoring rule:** if a card needs Partition behavior with *additional*
or *different* exclusions than the keyword's defaults, author the
`replacement` form directly with the `exclude_cause:` list. Don't extend
`partition` with exclusion overrides — keep that sugar minimal so it
stays a 1:1 representation of the printed keyword.

#### 3.5.6 `alt_path_registration` — `applies_to:` scope projection

`alt_path_registration` (used by BT17-007 Agumon's "Yu may DNA
digivolve into hand from any of your Digimon" pattern) registers an
alt-path that applies to a set of permanents *other than* the source.
The `applies_to:` filter projects the registration scope:

```yaml
- scope: inherited
  kind: alt_path_registration
  trigger: end_of_your_turn
  applies_to:
    of: you
    zone: [battle_area]
    # implicit `kind: digimon` — alt-paths only apply to digimon
  registers:
    kind: dna_digivolve
    target_zone: hand
```

Semantics: while this clause is active (its `scope:` and the carrier's
position in play satisfy the activation rules), every permanent matching
`applies_to:` gains the `registers:` alt-path as an additional entry in
its alt-path list, computed lazily by the engine when the digivolve /
play check runs against that permanent. Registration is **observational**
— the registered alt-path is not stored on the target permanent; it is
re-derived per check. This avoids the "what happens if the carrier
leaves mid-action" foot-gun.

`applies_to:` defaults to `{ of: you, zone: [battle_area], kind: digimon }`
when omitted. Set explicitly when the registration should target Tamers,
opponent permanents, or specific named subsets.

### 3.6 Timing catalogue

Enumerates every value allowed in `when:`. Maps 1:1 to a variant in
`digimon-engine/src/enums.rs` `EffectTiming`. New DSL timings require a
corresponding engine variant first.

| DSL string                     | Rust variant                  | Notes                                                |
|--------------------------------|-------------------------------|------------------------------------------------------|
| `on_play`                      | `OnPlay`                      |                                                      |
| `when_digivolving`             | `WhenDigivolving`             |                                                      |
| `when_attacking`               | `WhenAttacking`               | Observer; fires on attack declaration                |
| `end_of_attack`                | `EndOfAttack`                 |                                                      |
| `end_of_battle`                | `EndOfBattle`                 |                                                      |
| `on_attack`                    | `OnAttack`                    |                                                      |
| `on_deletion`                  | `OnDeletion`                  |                                                      |
| `on_any_deletion`              | `OnAnyDeletion`               |                                                      |
| `on_enter_field_anyone`        | `OnEnterFieldAnyone`          | Used for OnAllyPlayed / cross-player observers       |
| `on_ally_played`               | synthesized from `OnEnterFieldAnyone` + owner filter | DSL sugar     |
| `on_leave_field`               | `OnLeaveField`                |                                                      |
| `on_suspend`                   | `OnSuspend`                   |                                                      |
| `on_unsuspend`                 | `OnUnsuspend`                 |                                                      |
| `on_hatch`                     | `OnHatch`                     |                                                      |
| `on_digivolve`                 | `OnDigivolve`                 |                                                      |
| `on_dna_digivolve`             | `OnDnaDigivolve`              |                                                      |
| `on_digixros`                  | `OnDigiXros`                  |                                                      |
| `on_opponent_security_removed` | `OnOpponentSecurityRemoved`   | Medusamon observer                                   |
| `on_digivolution_card_trashed` | `OnDigivolutionCardTrashed`   | Rocks observer                                       |
| `on_security_check`            | `OnSecurityCheck`             |                                                      |
| `on_lose_security`             | `OnLoseSecurity`              |                                                      |
| `on_security`                  | `SecuritySkill`               | The `[Security]` primary trigger                     |
| `on_option_placed`             | synthesized from `OnEnterFieldAnyone` + `kind: option` filter | DSL sugar |
| `start_of_your_turn`           | `StartOfYourTurn`             |                                                      |
| `start_of_opponents_turn`      | `StartOfOpponentsTurn`        |                                                      |
| `start_of_your_main_phase`     | `StartOfYourMainPhase`        |                                                      |
| `end_of_your_turn`             | `EndOfYourTurn`               |                                                      |
| `end_of_opponents_turn`        | `EndOfOpponentsTurn`          |                                                      |
| `on_attack_target_change`      | `OnAttackTargetChange`        |                                                      |
| `main_from_hand`               | `MainFromHand`                | `[Hand] [Main]` activated                            |
| `main_on_field`                | `MainOnField`                 | `[Main]` activated from field                        |
| `main_from_trash`              | `MainFromTrash`               | `[Main]` activated from trash                        |
| `counter`                      | `CounterEffect`               |                                                      |
| `before_pay_cost`              | `BeforePayCost`               | Internal — set by `cost_reduction` lowering          |
| `delayed`                      | synthesized via `schedule_delayed` + trigger timing | Sugar                  |

### 3.7 Mutation verbs (`process:` steps)

Each verb is a pure wrapper over an `EffectContext` method. The set is
bounded; a new verb is a schema change. Grouped by family.

#### 3.7.1 Memory / turn

| Verb              | Arguments                | Lowering                                           |
|-------------------|--------------------------|----------------------------------------------------|
| `gain_memory`     | int                      | `ctx.gain_memory(n)`                               |
| `lose_memory`     | int                      | `ctx.lose_memory(n)`                               |
| `set_memory`      | int                      | `ctx.set_memory(n)` (guarded by flood gates)       |

#### 3.7.2 Draw / deck / hand / trash

| Verb                          | Arguments                                      | Lowering                                     |
|-------------------------------|------------------------------------------------|----------------------------------------------|
| `draw`                        | `{ of, count }`                                | `ctx.draw(player, count)`                    |
| `trash_from_top`              | `{ of, count }`                                | `ctx.trash_from_top(player, count)`          |
| `add_to_hand_from_deck`       | `{ of, card }`                                 | `ctx.add_to_hand_from_deck(...)`             |
| `add_to_hand_from_trash`      | `{ of, card }`                                 | `ctx.add_to_hand_from_trash(...)`            |
| `add_to_hand_from_reveal`     | `{ of, card }`                                 | `ctx.add_to_hand_from_reveal(...)`           |
| `trash_from_hand_by_index`    | `{ of, hand_index }`                           | `ctx.trash_from_hand_by_index(...)`          |
| `trash_from_reveal`           | `{ of, card }`                                 | `ctx.trash_from_reveal(...)`                 |
| `return_to_deck_from_reveal`  | `{ of, card, position }`                       | `ctx.return_to_deck_from_reveal(...)`        |
| `shuffle_deck`                | `{ of }`                                       | `ctx.shuffle_deck(player)`                   |
| `reveal_top_deck`             | `{ of, count, bind_as }`                       | `ctx.reveal_top_deck(...)` + reveal bindings |
| `place_remainder_on_deck`     | `{ of, position }`                             | `ctx.place_remainder_on_deck(player, pos)`   |

#### 3.7.3 Field / permanent

| Verb                          | Arguments                                                               | Lowering                                              |
|-------------------------------|-------------------------------------------------------------------------|-------------------------------------------------------|
| `delete_permanent`            | `{ target }`                                                            | `ctx.delete_permanent(h)`                             |
| `return_to_hand`              | `{ target }`                                                            | `ctx.return_to_hand(h)`                               |
| `return_to_deck`              | `{ target, position }`                                                  | `ctx.return_to_deck(h, pos)`                          |
| `suspend`                     | `{ target }`                                                            | `ctx.suspend(h)`                                      |
| `unsuspend`                   | `{ target }`                                                            | `ctx.unsuspend(h)`                                    |
| `de_digivolve`                | `{ target, amount, stop_at_level }`                                     | `ctx.de_digivolve(h, stop, amt)`                      |
| `place_on_security`           | `{ of, source, position, face_up }`                                     | `ctx.place_on_security(...)`                          |
| `play_token`                  | `{ controller, token_name }`                                            | `ctx.play_token(...)`                                 |
| `place_as_bottom_source`      | `{ source, target }`                                                    | `ctx.place_as_bottom_source(...)`                     |
| `trash_top_source`            | `{ target }`                                                            | `ctx.de_digivolve(h, None, Some(1))`                  |
| `hatch`                       | `{ of }`                                                                | `ctx.hatch(player)`                                   |

#### 3.7.4 Play / digivolve

| Verb                              | Arguments                                                              | Lowering                                              |
|-----------------------------------|------------------------------------------------------------------------|-------------------------------------------------------|
| `play_from_hand`                  | `{ of, hand_index, cost_delta }`                                       | `ctx.play_from_hand_with_cost(...)`                   |
| `play_from_hand_free`             | `{ of, hand_index }`                                                   | planned `ctx.play_from_hand_free(...)` (Tier-1 gap)   |
| `play_from_trash`                 | `{ of, trash_index, cost_delta }`                                      | `ctx.play_from_trash_with_cost(...)`                  |
| `play_from_trash_free`            | `{ of, trash_index }`                                                  | planned `ctx.play_from_trash_free(...)` (Tier-1 gap)  |
| `play_from_security`              | `{}`                                                                   | `ctx.play_from_security()`                            |
| `play_from_materials`             | `{ target, source_index, cost_delta }`                                 | planned (Tier-1 gap)                                  |
| `effect_initiated_digivolve`      | `{ target, from_hand, cost, ignore_requirements }`                     | `ctx.effect_initiated_digivolve(...)`                 |
| `effect_initiated_dna_digivolve`  | `{ target_a, target_b, from_hand, cost, ignore_requirements }`         | planned (Tier-3 gap)                                  |

#### 3.7.5 Security

| Verb                              | Arguments                                 | Lowering                                     |
|-----------------------------------|-------------------------------------------|----------------------------------------------|
| `trash_top_security`              | `{ of }`                                  | `ctx.trash_top_security(player)`             |
| `mark_security_face_up`           | `{ of, card }`                            | `ctx.mark_security_face_up(...)`             |

#### 3.7.6 Modifiers

| Verb                          | Arguments                                                              | Lowering                                              |
|-------------------------------|------------------------------------------------------------------------|-------------------------------------------------------|
| `add_dp_modifier`             | `{ target, value, expiry }`                                            | `ctx.add_dp_modifier(...)`                            |
| `add_modifier`                | `{ target, modifier, value, expiry }`                                  | `ctx.add_modifier(...)`                               |
| `grant_keyword`               | `{ target, keyword, expiry, value }`                                   | `ctx.grant_keyword(...)` (value for parametric)       |

#### 3.7.7 Selection verbs (install `PendingSelection`)

Each `select_*` verb takes a `filter:`, optional `bind_as:`, and the rest
of the `process:` continues *after* selection resolves — the DSL compiler
closes over the post-select steps in the callback closure.

| Verb                          | Arguments                                                              | Lowering                                              |
|-------------------------------|------------------------------------------------------------------------|-------------------------------------------------------|
| `select_own_permanent`        | `{ filter, bind_as, prompt, optional }`                                | `ctx.select_own_permanent(...)`                       |
| `select_opponent_permanent`   | `{ filter, bind_as, prompt, optional }`                                | `ctx.select_opponent_permanent(...)`                  |
| `select_hand`                 | `{ of, filter, bind_as, prompt, optional }`                            | `ctx.select_hand(...)`                                |
| `select_trash`                | `{ of, filter, bind_as, prompt, optional }`                            | `ctx.select_trash(...)`                               |
| `select_material`             | `{ of_permanent, filter, bind_as, prompt, optional }`                  | `ctx.select_material(...)`                            |
| `select_reveal`               | `{ filter, bind_as, prompt, optional }`                                | `ctx.select_reveal(...)`                              |
| `select_security`             | `{ of, filter, bind_as, prompt, optional }`                            | `ctx.select_security(...)`                            |
| `select_union_zone`           | `{ of, zones, filter, bind_as, prompt, optional }`                     | `ctx.select_union_zone(...)`                          |
| `select_ordered_permutation`  | `{ items, bind_as, prompt }`                                           | `ctx.select_ordered_permutation(...)`                 |
| `select_count_capped_multi`   | `{ of, zone, max, filter, bind_as, prompt, optional_zero }`            | `ctx.select_count_capped_multi(...)`                  |
| `select_effect_choice`        | `{ labels, bind_as, prompt }`                                          | `ctx.select_effect_choice(...)`                       |
| `as_selecting_player`         | `{ of, body }`                                                         | scope wrapper lowered to `ctx.as_selecting_player(id).select_*(...)` |

#### 3.7.8 Control flow

| Form                      | Shape                                                             | Lowering                                         |
|---------------------------|-------------------------------------------------------------------|--------------------------------------------------|
| `if / then / else`        | `{ if: <predicate>, then: [steps], else: [steps] }`               | direct Rust `if { } else { }` in closure        |
| `for_each`                | `{ over: <filter>, bind_as: id, body: [steps] }`                  | iteration over filter result; *not* a selection; for observer-scoped iterations only |
| `per_selected`            | sugar on top of a `select_count_capped_multi` — body runs per pick | expanded to `select_*` + `for_each` over bound list |
| `schedule_delayed`        | `{ when: <timing>, body: [steps] }`                               | planned (Tier-2 gap) — delayed one-shot trigger  |
| `raw_rust` (step form)    | `{ raw_rust: fn_name }` or `{ raw_rust: fn_name, args: {...} }`    | calls `fn(&mut EffectContext, &BindingMap) -> BindingMap` |

**Iteration semantics (locked):**

- **Snapshot at entry.** `for_each` and `per_selected` evaluate their
  iteration set **once**, at entry, before any body step runs. Body steps
  that mutate the iterated zone (`delete_permanent`, `return_to_hand`,
  `return_to_deck`, `place_as_bottom_source`) do **not** add or remove
  iterations; the loop drives the snapshot to completion. Susanoomon
  ("for each material → place on security") is the canonical case: the
  snapshot ensures the placed material is iterated even after the
  trash/place body has consumed its source slot.
- **Iteration order.** `for_each`: P0's battle_area in ascending index,
  then P1's; within each player, ascending index at snapshot time. Stable
  and turn-independent. `per_selected`: the order in which the player
  picked them (the engine preserves pick order in the bound list).
- **Vanished handles.** If a body step deletes a handle that a *later*
  iteration would target, the iteration whose subject no longer resolves
  silently no-ops on its body's binding-consuming verbs — the loop does
  not skip iterations or panic. (This matches the 2b/2c silent-no-op
  convention for invalid binding refs.)
- **Inner park.** A body step that parks a selection halts the loop at
  the current iteration; remaining iterations are abandoned in v1.
  Faithful per-iteration resumption is a Phase-3 enhancement; cards that
  truly need it (rare — Susanoomon does not) must use raw_rust until
  then.

### 3.8 Filter / predicate catalogue

A predicate evaluates to `bool`. Predicates split into three **scope
types**, enforced by the validator. Mixing scopes inside `all_of:` /
`any_of:` is allowed (each leaf is checked against the available
context); mixing them in a position that requires one specific scope
(e.g. a step's `target:` `filter:` requires `CandidatePredicate`) is a
validation error rejected at lowering time, not at runtime.

| Scope type            | Takes                                | Used in                                                      |
|-----------------------|--------------------------------------|--------------------------------------------------------------|
| `BoolPredicate`       | game state only (no candidate)       | `if:` condition, clause-level `condition:`, `active_when:`   |
| `CandidatePredicate`  | game state + a card / permanent      | `target:` `filter:`, `for_each` `over:`, `select_*` `filter:`|
| `AggregatePredicate`  | game state + an enumeration scope    | `count_lte:`, `any_permanent:`, `no_permanent:`, `all_permanents:` (these wrap a `CandidatePredicate` and an `of:` / `zone:` scope) |

Each leaf predicate's scope type is fixed by its row in the table below.
Combinators (`all_of:`, `any_of:`, `none_of:`, `not:`) take on the union
scope of their children; if any child is a `CandidatePredicate`, the
combinator becomes a `CandidatePredicate` (the candidate threads through
to every child that needs one and is ignored by `BoolPredicate` children).

The validator promise from §3.13 ("schema validator rejects typos")
extends to scope: a `BoolPredicate` placed where a `CandidatePredicate`
is required (e.g. `target: { your_turn: true }`) is a hard validation
error with the offending path.

Leaf predicates:

| Predicate                          | Applies to                   | Notes                                              |
|------------------------------------|------------------------------|----------------------------------------------------|
| `kind`                             | card / permanent             | `digimon`/`tamer`/`option`/`digi_egg`/`token`      |
| `level_eq` / `level_lte` / `level_gte` | card / permanent         | int; `level` of a permanent is its top-card level  |
| `color_is`                         | card / permanent             | enum; matches any of printed colors                |
| `color_only`                       | card / permanent             | multi-color match (all colors in set)              |
| `trait_has`                        | card / permanent             | string match against `type_eng`                    |
| `form_is`                          | card / permanent             | string match against `form_eng`                    |
| `attribute_is`                     | card / permanent             | string match                                       |
| `name_is`                          | card / permanent             | exact name                                         |
| `name_contains`                    | card / permanent             | substring match; case-insensitive                  |
| `name_in`                          | card / permanent             | list                                               |
| `card_number_is`                   | card / permanent             | printed card_id                                    |
| `dp_lte` / `dp_gte` / `dp_eq`      | permanent                    | int literal or `formula:`                          |
| `stack_size_lte` / `stack_size_gte`| permanent                    | int                                                |
| `materials_count_lte` / `_gte`     | permanent                    | sugar for `stack_size`                             |
| `has_inherited`                    | permanent                    | filter over its digivolution stack sources         |
| `is_suspended`                     | permanent                    | bool                                               |
| `is_unsuspended`                   | permanent                    | bool                                               |
| `has_keyword`                      | permanent                    | keyword enum                                       |
| `zone`                             | card / permanent             | `hand` / `deck` / `trash` / `battle_area` / `security` / `breeding` / `reveal` / `digi_egg_deck` |
| `owner`                            | card / permanent             | `you` / `opponent` / `any`                         |
| `other`                            | permanent                    | "not this card" — skips the source                 |
| `source_is_tamer`                  | source card                  | from `ctx.source_is_tamer()`                       |
| `source_name_contains`             | source permanent             | applies to this card's top name                    |
| `source_permanent_trait_has`       | source permanent             | same                                               |
| `count_lte` / `count_gte`          | aggregate                    | `{ count_gte: { filter: <...>, n: 2 } }`           |
| `any_permanent`                    | existential                  | `{ any_permanent: { of, zone, <filter fields> } }` |
| `no_permanent`                     | negated existential          |                                                    |
| `all_permanents`                   | universal                    |                                                    |
| `memory_lte` / `memory_gte`        | global                       | memory check                                       |
| `security_count_lte` / `_gte`      | global                       | security stack length                              |
| `your_turn`                        | global                       | phase check                                        |
| `in_breeding`                      | source-zone                  | this card is in breeding area                      |
| `on_field`                         | source-zone                  | this card is top of a battle-area permanent        |
| `event_target_*`                   | observer context             | filter against the triggering event's target       |
| `event_card_*`                     | observer context             | filter against the triggering event's card         |
| `equals` / `not_equals`            | binding comparison (Bool)    | `{ equals: [branch, 0] }`                          |

**Scope-type column** is implicit in "Applies to":
- `card / permanent` → `CandidatePredicate`
- `permanent` → `CandidatePredicate` (permanent-only)
- `aggregate` / `existential` → `AggregatePredicate`
- `global` / `source-zone` / `binding comparison` / `observer context`
  → `BoolPredicate`

`active_when:` accepts only `BoolPredicate` leaves plus combinators;
candidate-shaped predicates inside an `active_when:` are a validation
error (the clause has no candidate when the gate is evaluated).

### 3.9 Binding system

Scripting needs identifiers to thread values between selections and later
steps. Bindings are passed by name through `BindingMap`, a small
Rust-side map from `&'static str` to a tagged `Binding` enum
(`PermanentHandle` / `CardHandle` / `usize` / `i32` / `Vec<CardHandle>`).

| Binding name       | Origin                                                | Type                      |
|--------------------|-------------------------------------------------------|---------------------------|
| `source`           | implicit — this card                                  | `CardHandle`              |
| `source_permanent` | implicit — this card's permanent on field (if any)    | `Option<PermanentHandle>` |
| `carrier`          | implicit (inherited scope) — permanent carrying source| `PermanentHandle`         |
| `self`             | alias for `source_permanent` when on field; sugar     | `PermanentHandle`         |
| `event_target`     | observer context — triggering event's target          | depends on timing         |
| `event_card`       | observer context — triggering event's card            | `CardHandle`              |
| `<user-named>`     | `bind_as:` on a `select_*` verb                       | depends on verb           |
| `returned`         | last `return_to_*` verb's result                      | `CardHandle` (if any)     |
| `placed`           | last `place_*` verb's result                          | depends                   |
| `revealed_pool`    | post-`reveal_top_deck` implicit binding               | `Vec<CardHandle>`         |

Bindings are lexically scoped to the clause; they do not leak across
clauses. `for_each` introduces a per-iteration binding.

#### 3.9.1 Optional-select decline semantics

A `select_*` step with `optional: true` may be **declined** by the
selecting player. Decline semantics are uniform across every selection
verb:

1. The named `bind_as:` binding is **not installed** (the slot stays
   absent in the binding map).
2. The remainder of the `process:` body **short-circuits** — every step
   that follows the declined selection is skipped, the clause resolves
   immediately, and the engine advances to the next pending effect /
   queued trigger.
3. Inside an `optional:` *clause* (a triggered clause with
   `optional: true`), decline of the clause-level yes/no
   (`select_effect_choice`) collapses the entire `process:` body — same
   short-circuit, no bindings installed.

This is the v1 contract: declining a select aborts the clause. Cards
that need branching on decline ("if you didn't, do X instead" — e.g.
Millenniummon's DNA-origin bonus) use the explicit `select_effect_choice`
yes/no pattern with two `if/then/else` arms keyed on the chosen label,
**not** an optional select with a fallback. The `select_effect_choice`
binding is always installed regardless of the chosen label.

Continuation closures (Phase 2b/2c/2d `run_steps` callback path) must
treat decline as a sentinel: the engine's selection-resolution path
invokes the callback with a "declined" marker (action ID outside the
candidate range, or a dedicated `pending_selection.declined` flag); the
callback skips its body and drains `Game::dsl_outer_tail` (Phase 2d
Task 7) so the outer slice resumes correctly.

### 3.10 Formula primitives

Scalar formulas (`dp_lte: { formula: ... }`, `cost: { formula: ... }`,
`amount_fn: { formula: ... }`) are built from:

| Primitive       | Shape                                                       |
|-----------------|-------------------------------------------------------------|
| literal         | `5`                                                         |
| `base:`         | `{ base: 15, per: material_count, delta: -1 }`              |
| `per:`          | `material_count` / `stack_size` / `ally_count` / `digivolution_color_count` / `card_count_in_zone` |
| `floor_div`     | `{ floor_div: [<expr>, <int>] }`                            |
| `max` / `min`   | `{ max: [<expr>, <int>] }`                                  |
| `aggregate:`    | `{ aggregate: lowest_dp }` / `highest_dp` / `lowest_level`  |
| `raw_rust`      | `{ raw_rust: fn_name }` — signature `fn(&EffectReadContext) -> i32` |

### 3.11 Expiry catalogue

Matches `Expiry` enum in `enums.rs`.

| DSL string                       | Rust                         |
|----------------------------------|------------------------------|
| `end_of_your_turn`               | `EndOfYourTurn`              |
| `end_of_opponents_turn`          | `EndOfOpponentsTurn`         |
| `end_of_your_next_turn`          | `EndOfYourNextTurn`          |
| `end_of_opponents_next_turn`     | `EndOfOpponentsNextTurn`     |
| `end_of_turn`                    | `EndOfTurn`                  |
| `end_of_battle`                  | `EndOfBattle`                |
| `end_of_attack`                  | `EndOfAttack`                |
| `permanent`                      | `Permanent`                  |
| `until_next_unsuspend`           | `UntilNextUnsuspend`         |
| `while_source_exists`            | `WhileSourceExists`          |

### 3.12 Modifier catalogue

Maps to `ModifierType` in `enums.rs`. Authored values like `CannotDigivolve`,
`CannotBeAffected`, `GrantBlocker` are the direct Rust variant names — the
schema validator rejects typos.

### 3.13 Keyword catalogue

Maps to `Keyword` in `enums.rs`. Parametric keywords (`SecurityAttackPlus(n)`,
`DeDigivolve(n)`, `DrawX(n)`) are authored as `{ keyword: SecurityAttackPlus,
value: 1 }`.

### 3.14 `tests:` — card-level behavioral tests (co-located)

The single largest authoring-velocity win in this spec. Every card YAML
**may** carry a top-level `tests:` block that the build pipeline lowers
into generated `#[test]` functions wired to `DebugRunner`. This
collapses two authoring surfaces (YAML + a separate Rust test file)
into one, and lets LLM agents author behavioral coverage in the same
edit as the effect they implement.

The `/batch-implement-cards-rust` skill already mandates DebugRunner
tests written *before* implementation; co-location enforces that
discipline at the schema level — a card with `effects:` but no
`tests:` warns at validation, and a card with `tests:` whose generated
`#[test]` fails fails the build.

Each entry in `tests:` is one scenario:

| Field         | Required | Type                                          | Notes                                                                            |
|---------------|----------|-----------------------------------------------|----------------------------------------------------------------------------------|
| `name`        | yes      | string                                        | Snake-case; becomes the generated `#[test]` fn name (`test_<card_id>_<name>`).   |
| `setup`       | yes      | object — see below                            | Initial game state.                                                              |
| `actions`     | yes      | list of action shorthand                      | Sequence to drive; each entry is a named DSL action or a numeric action ID.      |
| `expect`      | yes      | list of state-assert shorthand                | Game-state assertions checked after `actions` resolve.                           |
| `seed`        | optional | int                                           | RNG seed for deterministic shuffles (defaults to 0).                             |

`setup:` shape:

| Field         | Type                                       | Notes                                                                  |
|---------------|--------------------------------------------|------------------------------------------------------------------------|
| `hand`        | `{ p0: [card_id...], p1: [card_id...] }`   |                                                                        |
| `field`       | `{ p0: [{ card: id, materials: [id...] }], p1: ... }` | Bottom-of-stack first.                                       |
| `trash`       | `{ p0: [card_id...], p1: ... }`            |                                                                        |
| `security`    | `{ p0: [card_id...], p1: ... }`            | Top first.                                                             |
| `deck`        | `{ p0: [card_id...], p1: ... }`            | Top first.                                                             |
| `memory`      | int                                        | Initial memory; positive = P0's, negative = P1's.                      |
| `turn_player` | `p0` / `p1`                                | Defaults to `p0`.                                                      |

`actions:` entries — each is one of:

| Form                                       | Lowering                                                            |
|--------------------------------------------|---------------------------------------------------------------------|
| `play_from_hand: { of: p0, card: <id> }`   | Compute action ID + `runner.submit(...)`                            |
| `digivolve: { of: p0, into: <id>, source: <field_index> }` | Same                                                |
| `attack: { of: p0, attacker: <field_index>, target: security }` | Same                                          |
| `pass`                                     | End turn                                                            |
| `select: { pick: <int> }`                  | Resolve pending selection by candidate offset                       |
| `decline`                                  | Resolve pending optional selection by declining                     |
| `select_many: { picks: [<int>...] }`       | Resolve `select_count_capped_multi` with N picks then submit        |
| `raw_action: <int>`                        | Submit raw action ID — escape hatch for cases not yet sugared       |

`expect:` entries — each is one assertion:

| Form                                          | Asserts                                                          |
|-----------------------------------------------|------------------------------------------------------------------|
| `field_count: { of: p0, eq: <int> }`          | Player's battle_area length                                      |
| `field_contains: { of: p0, card: <id> }`      | Some battle_area permanent has `card` as its top                 |
| `hand_count: { of: p0, eq: <int> }`           |                                                                  |
| `trash_contains: { of: p0, card: <id> }`      |                                                                  |
| `memory: { eq: <int> }`                       |                                                                  |
| `dp: { permanent: { of: p0, index: <int> }, eq: <int> }` | Effective DP including modifiers                      |
| `has_modifier: { permanent: ..., modifier: <enum>, present: <bool> }` |                                          |
| `has_keyword: { permanent: ..., keyword: <enum>, present: <bool> }`   |                                          |
| `winner: <p0 | p1 | null>`                    | Game-over winner check                                           |
| `pending_selection: { kind: <enum> | absent }`| Used by partial-resolution scenarios                             |

Worked example (compresses ~80 lines of hand-written Rust test):

```yaml
card: BT17-007
name: "Agumon"
# ... (effects definition omitted for brevity)

tests:
  - name: search_pulls_one_card
    setup:
      hand:    { p0: ["BT17-007"] }
      deck:    { p0: ["BT3-008", "BT2-001"] }
      memory:  3
      turn_player: p0
    actions:
      - play_from_hand: { of: p0, card: "BT17-007" }
      - select: { pick: 0 }   # pick BT3-008 from the search result
    expect:
      - field_count: { of: p0, eq: 1 }
      - hand_count:  { of: p0, eq: 1 }
      - hand_contains: { of: p0, card: "BT3-008" }

  - name: search_decline_keeps_hand_empty
    setup: { ... }   # same shape
    actions:
      - play_from_hand: { of: p0, card: "BT17-007" }
      - decline
    expect:
      - field_count: { of: p0, eq: 1 }
      - hand_count:  { of: p0, eq: 0 }
```

**Build-time lowering:** the `build.rs` step (§7a.1) emits one
`#[test]` fn per scenario into a generated module
(`src/cards/_generated_tests.rs`, included via a `pub mod` in
`lib.rs`). Each generated test is a thin wrapper: build the
`DebugRunner` from `setup:`, replay `actions:`, evaluate `expect:`,
panic on mismatch. Failure messages include the source YAML path +
scenario `name`.

**Strict semantics:**
- A card with `effects:` and no `tests:` block emits a validation
  **warning** (not error) — vanillas legitimately don't need them.
- Tests are NOT shipped in the desktop `cards.pack` blob (§7a). The
  `build.rs` emits them only when compiling the test binary
  (`#[cfg(test)]` gates the generated module).
- `actions:` is strict — if an action doesn't apply (illegal play,
  attempt to resolve a selection that isn't pending), the test fails
  with a clear "expected pending selection of kind X, got Y" message.
- `seed:` defaults to 0; for tests that depend on shuffle outcomes,
  set `seed:` explicitly to lock determinism across machines.

**Coverage policy:** the `/batch-implement-cards-rust` skill is updated
in lockstep with this section to require ≥1 PASS + ≥1 negative-path
(decline / no-legal-target / cost-can't-be-paid) scenario per
non-vanilla card. The skill rejects cards whose `tests:` block lacks
both.

## 4. Evaluator architecture (AOT)

### 4.1 Locked: ahead-of-time compiler, not interpreter

An interpreter that walks a parsed YAML tree per `CardEffect::effects()`
invocation would be simpler to ship but fails on two fronts:

1. **RL throughput.** A single training run does ~10^7 `effects()` calls.
   Interpreter dispatch costs (~2–5 μs/invocation on a hot card) multiply
   into tens of seconds of per-run overhead. AOT brings the hot path back
   to native closure dispatch.
2. **Debuggability.** An AOT closure is a stack frame with card context;
   an interpreter stack is a generic `step_eval` frame repeated N times.
   Native tracing (`RUST_LOG=debug`) is meaningful in AOT, opaque in the
   interpreter.

AOT is the evaluator. No opt-in interpreter fallback ships.

### 4.2 Compilation pipeline

`CardRegistry::load(card_pack_path)` runs this pipeline:

1. **Glob YAML files** under the card-pack directory.
2. **Parse** each file via `serde_yaml` into a `CardSpec` struct mirroring
   §3.2. Malformed YAML fails the whole load (no partial loads — the
   registry is an atomic value).
3. **Validate** each `CardSpec`:
   - card_id matches `cards.json` entry;
   - timings in `when:` are real `EffectTiming` values;
   - filter predicates type-check (e.g. `dp_lte` only on permanent filters);
   - modifier/keyword names are real enum variants;
   - `raw_rust: fn_name` references a registered fn in the
     `CardEffectExtensionRegistry` (§6).
4. **Lower** to `CompiledCard { card_id, effects: Vec<Effect>,
   alt_paths: Vec<AltPath>, identity: Option<Identity>, ... }`:
   - each triggered clause → one `Effect` with `condition` and `process`
     closures that capture the lowered filter/process subtrees;
   - each declarative clause → either (a) a declarative `Effect` (for
     `aura`, `cost_reduction`, `flood_gate`, `grant_keyword`, `ace_overflow`)
     or (b) structured metadata consumed by the engine at non-`effects()`
     touchpoints (e.g. `alt_paths` and `identity` feed the digivolve /
     DigiXros / name-overlay subsystems);
5. **Install** into `CardRegistry` keyed by `card_id`.

### 4.3 Closure shapes

The core output of lowering is a `process` closure of type
`Box<dyn Fn(&mut EffectContext) + Send + Sync>`. The compiler walks the
lowered step tree and synthesizes one closure per clause; no runtime
dispatch through a step enum.

Selections are lowered to `ctx.select_*` calls whose callback closures
capture the *remainder* of the `process:` list. Nested selections nest
the capture. Because `FnOnce` captures are fine for the single-use
callback and the engine guarantees exactly one callback fire per
`PendingSelection`, the compiler does not need `Arc<Mutex<_>>` except for
`select_count_capped_multi`'s PASS path — which is already solved inside
`selections.rs` and the DSL simply defers to it.

Filter predicates lower to `Fn(&Game, PermanentHandle) -> bool` (or the
appropriate signature for hand / trash / reveal / union-zone) closures.
Each predicate is a small owned struct (`NameContainsPredicate { needle:
ArcStr }`); composition uses owned `Vec<Predicate>` rather than boxed
`Fn` chains, so the hot path is a single enum dispatch per predicate
rather than dynamic dispatch.

### 4.4 Bindings

`BindingMap` is a tagged array. Capacity is **provisional at 8** — this
covers every clause seen in the 34-card exploration with margin, but is
not measured against the full corpus. Phase 2 of the migration must run
the lowering pipeline against `cards.json` and emit the per-card max
binding count; if any card exceeds 8, switch the storage to `SmallVec<[_;
8]>` (zero allocation in the common case, heap spill in the long tail)
rather than bumping the inline capacity to a worst-case figure that
penalises every card. Binding lookup is O(n) linear scan; the common
case is 0–2 bindings per step so this is faster than a HashMap.

### 4.5 Parity with hand-written Rust

For each DSL-compiled card there exists, by construction, a hand-written
Rust equivalent producing the same `Vec<Effect>`. The compiler is a
deterministic pure function from `CardSpec` to `CompiledCard`. Two
parity tests anchor this:

- **Golden parity test** (`digimon-engine/tests/dsl_parity.rs`): for a
  rotating set of cards, both a hand-written `CardEffect` and a DSL
  `CardEffect` are registered; every game event's resulting tensor and
  action mask are byte-identical.
- **Property test**: fuzz the input `CardSpec` (valid only) and assert
  `effects().len()` and `Effect::timing` distribution stays stable
  across schema changes — guards against accidental lowering drift.

### 4.6 Hot-reload during RL training

The RL worker exposes a `reload_cards(card_pack_path)` method that:

1. Parses + validates + lowers the pack on a scratch registry.
2. If validation fails, logs and keeps the old registry — training
   continues.
3. On success, swaps the `Arc<CardRegistry>` atomically.

Workers complete the current game before picking up the new registry;
in-flight games are unaffected. The property preserved is: **a
hot-reload never corrupts an in-flight game.**

## 5. Engine dependencies

The DSL does not remove gaps; it consumes them. This section enumerates
what the engine must expose for the DSL to reach its coverage targets.
Each bullet references a tier in the archetype gap plans under
`.claude/plans/rust-engine-gaps-*.md`.

### 5.1 Tier 1 — Foundational (blocks DSL Phase 1 coverage)

- `EffectContext::play_from_hand_free(player, hand_index)` — the single
  most load-bearing gap. Generalizes `play_from_security`. Used by
  Nokia, WarGreymon branch 2, every "play without paying the cost"
  card.
- `EffectContext::play_from_trash_free(player, trash_index)` and
  `play_from_materials(target, source_index, cost_delta)`.
- Reveal pipeline: `reveal_top_deck`, `add_to_hand_from_reveal`,
  `trash_from_reveal`, `return_to_deck_from_reveal`,
  `place_remainder_on_deck`. Partly landed on `serene-brahmagupta`; DSL
  depends on the full set.
- Zone-exit mutators: `return_to_hand`, `return_to_deck` with stack
  position, `place_as_bottom_source` for digi-egg / Royal Knight /
  Shoutmon KV flows.
- `trash_from_hand_by_index`, `add_to_hand_from_trash`.
- Security-zone mutators: `place_on_security` with face-up and position,
  `add_top_security_to_hand`, `trash_top_security`,
  `shuffle_security`.
- **Observer context enrichment:** `EffectContext::triggering_permanent`,
  `triggering_card`, `triggering_defender`, `leave_cause`. Required by
  `event_target_*` / `event_card_*` DSL predicates — roughly 25 cards
  out of the 34-card corpus need them for `condition:` on observer
  clauses.
- **EffectBuilder cost-payment hooks:** `.pay_cost_suspend_self()`,
  `.pay_cost_return_self_to_deck_bottom()`, generic
  `.pay_cost(|ctx| bool)`. The DSL's `extra_cost:` block in `alt_paths:`
  lowers to these.

### 5.2 Tier 2 — Observer triggering

- `StartOfYourTurn`, `StartOfYourMainPhase`, `OnLeaveField` +
  `LeaveCause`, `OnAllyPlayed` / `OnEnterFieldAnyone` fan-out,
  `OnDigivolve` / `OnDnaDigivolve`, `OnSecurityCheck`, `OnSuspend`,
  `OnHatch` + `OnMoveFromBreeding`, `EndOfAttack` + `WhenAttacking`
  firing, `BeforePayCost` scanning, delayed one-shot turn-scheduled
  triggers.
- Every DSL clause with `when: <timing>` requires the corresponding
  firing site to be wired. If a timing is not fired, the DSL clause is
  silently dead; the compiler marks this with a validation warning
  keyed to a per-branch "timings available" manifest the engine crate
  exports.

### 5.3 Tier 3 — Digivolve override subsystem

- Effect-driven digivolve at fixed/zero cost ignoring requirements
  (`effect_initiated_digivolve` already partly exists).
- DNA digivolve via effect (from hand, outside Main) — needed by
  Susanoomon, Omnimon Alter-S, Jupitermon, etc.
- Alternate digivolution source registration — the data channel that
  `alt_paths:` with `kind: dna_digivolve | digixros | burst_digivolve |
  app_fusion | assembly | activated_digivolve` populates. Today this is
  Python's `_alt_digi_*`; Rust equivalent is the "Alternate digivolution
  source registration" Tier-3 gap.
- Blast DNA / Blast Digivolve Counter-window support — extends the
  existing Counter infrastructure.
- `WhenDigivolving` context-flag for DNA origin detection.

### 5.4 Tier 4 — Selection-kind expansion

- Multi-pick distinct, ordered permutation, cross-player select-any,
  self-stack materials multi-select, budgeted multi-select. The DSL
  exposes these as `select_ordered_permutation`,
  `select_count_capped_multi`, and `per_selected` — some already
  landed on `serene-brahmagupta` (see `selections.rs`).
- `if-effect-didn't-resolve` / `on_decline` hook for optional clauses
  that branch on player decline (Millenniummon's conditional return-to-deck
  for the DNA-origin bonus is the canonical case).

### 5.5 Tier 5 — Keyword / modifier expansion

- **Native keyword parsing:** `CardData.keywords: Vec<Keyword>` +
  unified `has_printed_or_granted_keyword`. The DSL
  `has_keyword: Blocker` filter is meaningless until this lands because
  printed-only keywords are not currently queryable.
- `Delay` keyword lifecycle — depends on Option persistent placement.
- `Evade` keyword + deletion-interrupt hook (used by the `replacement`
  clause kind).
- `Ace Overflow` memory penalty — depends on `LeaveCause`.
- `De-Digivolve N` executor — already landed via
  `EffectContext::de_digivolve`.
- `Partition` keyword + provenance-filtered leave-field replacement —
  depends on the Evade-class replacement infra.
- `Raid` attack-target switch interrupt + redirect-attack primitive.
- `MayAttack` scoped variants.
- Source-scoped `CannotBeAffected` (used heavily by Olympos XII).
- `CannotPlayDigimonByEffect` — already landed.
- `Expiry::EndOfOpponentsNextTurn`.

### 5.6 Tier 6 — Option / Tamer flow + auras

- Option `[Main]` play flow (RUST_ENGINE_API.md §9 known gap).
- Option persistent placement (for `Delay` / Ace).
- Script-driven color-requirement bypass — the DSL's
  `ignore_requirements: true` and `color_delta: ...` on `alt_paths:`
  lower into this.
- Security-effect return-to-hand / place-on-field.
- **Granted triggered ability** — attach an `Effect` to another
  permanent. Required by several Tamer cards that grant `<Security A.
  +1>` or triggered effects on Omnimon-named Digimon. The DSL's `aura`
  with `grant_keyword` lowers to this; a `grant_triggered_ability`
  clause is reserved but not in the Phase-1 vocab.
- Named-target declarative aura (DP / keyword grants filtered).
- Declarative aura sourced from security zone (ST20-15).
- Variable / computed static DP modifier — lowers from `dp_modifier: {
  formula: ... }`.
- Digivolution-stack name overlay (BT17-102) — connects to `identity:`.
- Replacement effect: prevent battle deletion by paying a cost (EX5-015).
- `Decode` keyword (composite).
- Grant-attack-permission after `WhenDigivolving`.

### 5.7 Gap → DSL coverage mapping

| Gap tier      | DSL features enabled                                              | Card % coverage added |
|---------------|-------------------------------------------------------------------|-----------------------|
| T1 + T2       | Basic triggered cards, simple Tamers, searchers                   | ~35%                  |
| + T3          | DNA / Burst / DigiXros / Hybrid alt-paths, boss-tier bodies        | ~65%                  |
| + T4          | Multi-select, permutation, cross-player decks                     | ~75%                  |
| + T5          | Keyword-native filters, Ace, Partition, Raid, source-scoped C.B.A | ~88%                  |
| + T6          | Options, persistent placement, auras, granted abilities, Decode   | ~97%                  |
| + raw_rust    | The long tail (incl. BT10-111)                                    | ~99%                  |

## 6. Escape hatch — hybrid YAML + raw_rust

### 6.1 No Rhai

Embedding Rhai (or a similar scripting runtime) was considered and
rejected. Reasons:

1. Two authoring surfaces (YAML + Rhai) doubles the LLM prompting
   cost, the validator scope, and the test matrix.
2. Rhai's types don't align with engine types (`PermanentHandle`,
   `CardHandle`, `CardSource`) without a bindings layer that is itself
   a substantial project.
3. The residual card count (~1–5%) is small enough that writing Rust
   fns for them costs less than maintaining a second runtime.
4. Rhai calls don't participate in the same `Send + Sync` static checks
   that `CardEffect` demands. This is fixable but adds friction.

### 6.2 raw_rust registry

An engine-owned `EngineRawRustRegistry` holds named Rust fns authored
alongside the engine crate:

```
digimon-engine/src/cards/raw_rust/
├── mod.rs           # pub fn build_registry() -> EngineRawRustRegistry
├── bt13_007.rs      # fn royal_knight_cost_reduction(ctx: &EffectContext, target: PermanentHandle) -> i32
├── bt10_111.rs      # fn digixros_wildcard(ctx: &mut EffectContext, bindings: &mut Bindings)
└── ...
```

`build_registry()` runs once at crate startup. The registry is frozen
before DSL card registration so the engine can resolve `raw_rust:
fn_name` references for whole clauses, process steps, formulas, and
scheduled bodies.

Implementation note: formula raw_rust receives `(&EffectContext,
PermanentHandle)` so it shares the same target-resolution point as
`formula_eval::evaluate_with_raw`. Functions that only need read-only
state must treat the context as read-only.

### 6.3 Granularity (locked: hybrid both-levels)

The DSL supports `raw_rust:` at **two** levels. The fine-grained form is
essential — without it, a single complex requirement forces the whole
card into Rust, losing YAML's readability for the easy parts.

#### 6.3.1 Whole-clause

```yaml
effects:
  - kind: raw_rust
    fn: bt10_111_replacement_wildcard
    triggers:
      - on_digixros_prepare
```

The fn signature for a whole-clause raw_rust is:

```rust
fn(card: CardHandle) -> Vec<Effect>
```

— it returns as many `Effect`s as the clause needs, hand-wiring their
timing and process.

#### 6.3.2 Step-level

```yaml
effects:
  - when: on_play
    process:
      - select_own_permanent: { ..., bind_as: target }
      - raw_rust:
          fn: bt10_111_compute_xros_substitution
          consumes: [target]
          binds: [substitution_slot]
      - place_as_bottom_source:
          source: { binding: substitution_slot }
          target: target
```

The fn signature is:

```rust
fn(ctx: &mut EffectContext, bindings: &mut BindingMap)
```

The compiler generates the glue: `ctx` is the live context; the fn
reads named inputs from `bindings` (type-checked at load time against
`consumes:`), mutates / queries `ctx` arbitrarily, writes named outputs
back into `bindings` (type-checked against `binds:`). The next YAML step
sees the updated bindings.

#### 6.3.3 Formula-level

```yaml
kind: cost_reduction
amount_fn: { raw_rust: bt13_007_royal_knight_cost_reduction }
```

Signature `fn(&EffectReadContext) -> i32`. Used for any scalar computation
that resists the formula-primitive set in §3.10.

### 6.4 Card that justifies the whole-clause escape hatch

BT10-111 Shoutmon (King Version): `On Play, You may return 1 card with
a DigiXros requirement from your trash to your hand. When you would
DigiXros this turn, this Digimon may replace 1 of the DigiXros
requirements.`

The second sentence cannot be expressed in the DSL today because:

1. It requires cross-card runtime introspection (iterate every card's
   `alt_paths:` with `kind: digixros`, find a material filter to
   substitute).
2. The substitution is not a pre-play rewrite — it is a DigiXros-time
   hook that alters the candidate-material check for *any* other card
   this turn.
3. A turn-scoped "you may substitute" flag must hook into the DigiXros
   material-legality check itself.

The implementation path: `raw_rust: bt10_111_digixros_substitution`
registers a turn-scoped `ModifierType::XrosSubstitutionAvailable` on
the controller; the DigiXros material-legality check consults the flag
and extends the candidate set. This is a whole-clause raw_rust, not a
step-level one, because the hook is an engine-side registration rather
than a mutation sequence.

### 6.5 Budget

The raw_rust registry is budgeted at **≤ 3% of card count** (120 fns
for a 4,000-card pool). Exceeding this budget during migration is the
signal to widen the DSL vocabulary, not to write more raw Rust. A
per-set tally is emitted as a load-time log line and surfaced in the
`/batch-implement-cards-rust-dsl` progress board.

## 7. Migration path

Five phases. Each ends with a well-defined card count unblocked and a
demonstrable test. Hand-written cards are retired card-by-card; the
registry supports both side-by-side during migration.

### 7.1 Phase 0 — Schema definition + YAML loader + validator

**Scope:** no engine integration. Just:

- `CardSpec` serde structs mirroring §3.2.
- Loader that parses YAML + `cards.json` cross-check.
- Validator emitting errors per §4.2 step 3.
- Pretty-printer that round-trips YAML → CardSpec → YAML.
- Schema export (JSON Schema) for editor tooling.

**TDD:** a `tests/dsl_schema.rs` file that loads all 15 worked card
YAMLs from this spec and asserts they parse, validate, and round-trip.

**Exit criteria:** 15/15 worked YAMLs pass validation; schema exported.

### 7.2 Phase 1 — AOT compiler for declarative-only subset

**Scope:** alt_paths + identity + aura + cost_reduction + flood_gate +
grant_keyword + ace_overflow. No `process:` lowering yet. This buys
~30% of the card pool mechanically from `xros_req` + structured
fields, without requiring the hardest engine work.

Representative cards implementable at end of Phase 1: BT17-007 (alt-path
+ inherited DNA registration only, without the Start-of-Main trigger),
ST2-13 (wait — ST2-13 needs Option `[Main]` play flow, which is a Tier-6
gap; Phase 1 cannot actually cover it), Koromon vanillas, etc.

**TDD:** `tests/dsl_phase1_alt_paths.rs` — for 50 cards, load YAML,
register as `CardEffect`, step the engine through a digivolve, assert
the alt-path matches the digivolve requirement check.

**Exit criteria:** 50 hand-written digivolve-path-only cards retired.

### 7.3 Phase 2 — Imperative `process:` compiler

**Scope:** `select_*` verbs, mutation verbs, bindings, `if/then/else`,
`for_each`, optional/once_per_turn. Covers hello-world triggered effects
across every timing in §3.6 that is actually fired by the engine.

End of Phase 2: 500+ hand-written cards retired. The 15 worked examples
in §10 all compile and pass behavioral tests (subject to their Tier-5/6
engine prerequisites landing in parallel).

**TDD:** `tests/dsl_phase2_behavioral.rs` — each worked card from §10
with a card-text-derived behavioral test via `DebugRunner`. Parity test:
for every card with both a hand-written and DSL implementation, the
tensor+mask stream through 20 random game seeds is byte-identical.

**Exit criteria:** ≥500 retired; parity suite green; one archetype
(e.g. BT17 Tai/Matt DNA Omnimon core, ~50 cards) fully DSL-authored.

**Sub-phase progress:**
- **2a** (landed) — triggered clause lowering + memory/draw + `run_steps`
  scaffold.
- **2b** (landed) — selection steps (`SelectHand` / `SelectTrash` /
  `SelectOwn|OpponentPermanent`), binding refs, continuation dispatcher,
  zone moves.
- **2c** (landed) — permanent mutations (Suspend / Unsuspend / Delete /
  ReturnToHand / ReturnToDeck / DeDigivolve), AddDpModifier, AddModifier
  binding-target, GrantKeyword, control flow (`If` / `Optional`).
- **2d** (landed 2026-04-25) — multi-result bindings
  (`BindingValue::PermanentList` + `CardList`), iteration verbs
  (`ForEach`, `PerSelected`), multi-pick selection
  (`SelectCountCappedMulti` over Hand/Trash), `AddModifier` filter-target
  arm, and the `run_steps` continuation propagation fix
  (`RunOutcome { Synchronous, Parked }` + `Game::dsl_outer_tail`).
  Defers to 2e+: `ScheduleDelayed` (needs `ctx.schedule_delayed` engine
  primitive), remaining selection kinds (Reveal / Security / Material /
  UnionZone / OrderedPermutation / EffectChoice / AsSelectingPlayer),
  play / digivolve / placement steps, formula values in modifier
  `value` fields, and `distinct_by` enforcement on
  `SelectCountCappedMulti`.
- **2e** (landed 2026-04-25) — remaining selection kinds
  (`SelectEffectChoice`, `SelectReveal`, `SelectSecurity`,
  `SelectMaterial`, `SelectUnionZone`, `SelectOrderedPermutation`) and
  `distinct_by` enforcement on `SelectCountCappedMulti` (`CardNumber` /
  `Level` / `Name`, with `Level` treating two `None`-level cards as
  *different* — Tamers / Options never lock each other out).
  `Bindings::insert_literal` helper added for `SelectEffectChoice`'s
  branch-index binding. Defers to 2f+: `AsSelectingPlayer` (needs
  override-persistence across selection callbacks — engine work),
  play / digivolve / placement steps, formula values in `add_modifier`
  `value`, and `ScheduleDelayed` (needs `ctx.schedule_delayed` engine
  primitive).
- **2f1** (landed 2026-04-26) — play / digivolve / placement step
  lowering: `PlayFromHand`, `PlayFromHandFree`, `PlayFromTrash`,
  `PlayFromTrashFree`, `PlayFromSecurity`, `PlayFromMaterials`,
  `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`,
  `PlayToken`, `PlaceOnSecurity`, `PlaceAsBottomSource`,
  `TrashTopSource`. New engine primitives: `play_from_hand_free`,
  `play_from_security`, `play_from_materials`,
  `effect_initiated_dna_digivolve`, `trash_top_source`. The existing
  `play_from_security()` (security-skill replay) was renamed to
  `play_pending_security()` to free the name for the persistent-zone
  primitive — Rust forbids method-name overloading. Cost-delta
  translation (`Free` / `Printed` / `Literal`) lives in the DSL
  step-handler `dsl_cards/step/play_digivolve.rs` via
  `lower_cost_delta`, mapping `CompiledCostDelta` →
  `crate::enums::CostDelta` so the engine surface stays free of DSL
  imports. Fixture quirks worth noting: (a) `play_from_trash*` YAML
  uses `hand_index:` (struct reuse); (b) `EffectInitiatedDigivolve`
  still requires a matching `evo_costs` row by level even with
  `ignore_requirements: true` (the flag covers color/level/memory but
  not the cost-table lookup itself); (c) `from_hand` resolves via
  `HandIndex(i)` for single-target digivolve but `Card(handle)` for
  DNA digivolve — asymmetric IR shapes; (d) `EffectInitiatedDigivolve`
  carries `cost: i32`, which can express `Free (0)` and `Fixed(n)` but
  not `Reduce(n)` — a Phase 3 IR-widening concern. Tracked follow-ups:
  fire `OnDnaDigivolve` from both this primitive and the canonical
  user-action path once `TODO(dna-digivolve-execute)` lands; widen
  `BindingValue::HandIndex` / `TrashIndex` to carry a `PlayerId` so
  placement steps can target opponent zones. Defers to 2f2+: formula
  values in `add_modifier`, `AsSelectingPlayer` override-persistence,
  `ScheduleDelayed`.
- **2f2** (landed 2026-04-26) — formula values in `add_dp_modifier` /
  `add_modifier`. New IR shape `CompiledModifierValue { Literal(i32) |
  Formula(CompiledFormula) }` replaces the bare `value: i32` field on
  both step variants. YAML accepts either a bare int (`value: 3000`) or
  a `{ formula: ... }` block via `serde(untagged)` `ModifierValueSpec`,
  reusing `alt_path::FormulaCost` for the wrapper. Runtime evaluator
  lives at `code/digimon-engine/src/dsl_cards/formula_eval.rs` —
  signature `evaluate(&CompiledFormula, &EffectContext, target:
  PermanentHandle) -> i32`. `target` is mandatory because per-selectors
  (`StackSize` / `MaterialCount` / `DigivolutionColorCount`) resolve
  against the bound or matched permanent; `AllyCount` resolves against
  the target's player; aggregates (`LowestDp` / `HighestDp` /
  `LowestLevel` / `HighestLevel`) scope to `ctx.player`'s battle area.
  Filter-target `add_modifier` evaluates the formula **per match**
  inside the scan loop, never hoisted (load-bearing for Susanoomon-style
  "+X DP per material on this Digimon" semantics — pinned by
  `add_modifier_filter_target_formula_evaluated_per_match`'s
  `assert_ne!` on two permanents with different stack sizes). The
  modifier-value pipeline is **i32 end-to-end**
  (`formula_eval::evaluate` → `EffectContext::add_modifier` →
  `ModifierEntry.value` → `ModifierRegistry::sum`); no narrowing today.
  The `add_dp_modifier_formula_large_value_passes_through` test pins
  the contract with `Literal(40000)` (above i16::MAX) and is the
  tripwire if a future engine change narrows the type — saturation
  goes in `step/modifiers.rs::resolve_modifier_value` if it ever
  becomes necessary. Defensive convention: degenerate formula inputs
  (FloorDiv arity != 2, divide by zero, missing target, empty
  aggregate set) return `0` rather than panic; `RawRust(name)` and
  `CardCountInZone` are Phase 3 placeholders that return `0`.
  Debug-build `eprintln!` warnings emit on the two card-author-bug
  branches (`RawRust` unregistered, `FloorDiv` wrong arity). A
  compile-time `const _: () = assert!((CardColor::Purple as u8) < 8)`
  guards the `DigivolutionColorCount` `u8` bitmask. Tracked
  follow-ups: wire `RawRust` formula dispatch through
  `RawRustRegistry`; widen `CompiledPerSelector::CardCountInZone`
  with a `CompiledZone` payload; consider opponent / universal scope
  for `CompiledAggregateSelector`; widen `lookup_modifier_type` to
  expose value-bearing names (`ChangeDp` etc.) for filter-target
  numeric modifiers; extend `AddDpModifier` to accept
  `CompiledModifierTarget::Filter` (binding-only today). Defers to
  2f3+: `AsSelectingPlayer` override-persistence, `ScheduleDelayed`.
- **2f3** (landed 2026-04-26) — `AsSelectingPlayer` step lowering with
  override-persistence across selection callbacks. Engine refactor: every
  `pub fn select_*` callback in `code/digimon-engine/src/effect_context/selections.rs`
  now constructs the post-resolution `EffectContext` via
  `EffectContext::new_with_override(game, source_card, source_permanent,
  controller, override_pin)` (10 call sites — `select_hand`,
  `select_trash`, `install_field_selection` shared by own/opponent
  permanent, `select_count_capped_multi`, `select_effect_choice`,
  `select_reveal`, `select_security`, `select_material`,
  `select_union_zone`, `select_ordered_permutation`). The seeding line
  `let selecting_player = self.override_selecting_player.unwrap_or(self.player);`
  (which feeds `pending_selection.selecting_player`) is preserved verbatim
  at every site — only the callback's reconstructed ctx changes shape so
  it carries `(controller, override_pin)` rather than collapsing them.
  Field `override_selecting_player` stays `pub(super)`; a new
  `pub(crate) fn set_override_selecting_player(&mut self, p:
  Option<PlayerId>)` setter is the only mutation path for callers
  outside `effect_context/`. DSL lowering at
  `code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs`
  follows a save → set → run_steps → conditional-restore pattern: on
  `RunOutcome::Synchronous` the previous override is restored; on
  `Parked` it is NOT restored (Task 1's `new_with_override` carries it
  through the parked-callback boundary). Outer-tail leak fix:
  `drain_dsl_outer_tail(cb_ctx)` now calls
  `set_override_selecting_player(None)` BEFORE running the parked
  outer-tail steps, so an outer sibling `select_*` after
  `as_selecting_player` correctly routes to the controller, not the
  override. Pinned by
  `as_selecting_player_outer_tail_select_does_not_inherit_override`
  (regression guard — empirically verified by the implementer that
  removing the clear makes the test fail). End-to-end YAML test at
  `code/digimon-engine/tests/dsl/phase2f3_end_to_end.rs` exercises the
  canonical "your opponent chooses one of your Digimon" card text via a
  TST-VOTE Opponent's Vote card with `as_selecting_player: { of:
  opponent, body: [select_own_permanent, add_dp_modifier: -3000] }`.
  Pre-existing systemic divergence surfaced (out of 2f3 scope, tracked
  as a follow-up): the DSL validator accepts snake_case expiry strings
  (`end_of_turn`) but the engine's `lookup_expiry` only matches
  PascalCase (`EndOfTurn`). Cards authored with the validator-blessed
  form silently no-op modifiers at runtime — the
  `phase2f3_end_to_end.rs` YAML uses `EndOfTurn` to work around it.
  The follow-up should land before card scripts are authored against
  Phase 2f3. Defers to 2f4+: `ScheduleDelayed` (needs
  `ctx.schedule_delayed` engine primitive).
- **2f4** (landed 2026-04-26) — `schedule_delayed` engine subsystem +
  DSL lowering. New `code/digimon-engine/src/scheduled_effects.rs`
  introduces `pub struct ScheduledEffect { when: EffectTiming, body:
  Vec<CompiledStep>, source_card: CardHandle, source_permanent:
  Option<PermanentHandle>, controller: PlayerId, captured_bindings:
  Bindings }` and `pub fn fire_scheduled_for_timing(game, t)` that
  drains every queued effect whose `when` matches `t` in FIFO order.
  `Game::scheduled_effects: Vec<ScheduledEffect>` field added. New
  primitive `EffectContext::schedule_delayed(when, body,
  captured_bindings)` captures `(self.source_card, self.source_permanent,
  self.player)` plus the passed args. Stored as `CardHandle` (Copy)
  rather than the plan's suggested `CardSource` (Clone) — cleaner, no
  new trait bound, matches `EffectContext::new`'s `source_card`
  parameter type. Drain wired into 4 observer-fire boundaries with
  scheduled bodies firing AFTER printed observers (so observers see
  pre-scheduled state and scheduled bodies see post-observer state):
  `EndOfYourTurn` (in `game_phases.rs::fire_end_of_your_turn`),
  `EndOfOpponentsTurn` (in `game_phases.rs::rotate_turn_player`),
  `EndOfBattle` (in `combat.rs::resolve_battle`), and `EndOfAttack` (in
  `combat.rs::cleanup_attack`, BEFORE `expire_end_of_attack` so
  scheduled bodies see same attack context). The unified `EndOfTurn`
  variant doesn't exist in `EffectTiming` (split into
  `EndOfYourTurn` + `EndOfOpponentsTurn`); `EndOfYourNextTurn`,
  `EndOfOpponentsNextTurn`, `UntilNextUnsuspend` deferred to Phase 3
  (need a generation counter on `ScheduledEffect` for "next turn"
  semantics — out of 2f4 scope). Re-entrancy / parked-selection
  guard: `fire_scheduled_for_timing` includes a per-iteration
  `debug_assert!(game.dsl_outer_tail.is_none())` with a TODO(phase-3)
  comment for retry logic — most scheduled bodies are synchronous
  (`gain_memory`, `draw`, `add_modifier`); cards that schedule a
  body that itself parks would trip the assertion in debug builds and
  must wait for Phase 3. DSL lowering at
  `code/digimon-engine/src/dsl_cards/step/schedule_delayed.rs`:
  `compiled_timing_to_engine(*when)` maps `CompiledTiming` →
  `EffectTiming`, then `ctx.schedule_delayed(t, body.clone(),
  bindings.clone())`. Bindings are cloned at schedule time so
  subsequent caller mutations don't leak into the captured copy.
  End-to-end YAML test at
  `code/digimon-engine/tests/dsl/phase2f4_end_to_end.rs` (DelayedDraw
  card: `on_play → schedule_delayed: { when: end_of_your_turn, body:
  [draw: { of: you, count: 1 }] }`) exercises the full pipeline.
  Notable structural finding: the timing pipeline (`TimingSpec` →
  `CompiledTiming` → `EffectTiming` via direct enum-variant matching
  in `timing_map.rs::compiled_timing_to_engine`) is divergence-immune,
  unlike the expiry pipeline (string-keyed `lookup_expiry` —
  pre-existing snake_case-vs-PascalCase divergence surfaced in 2f3).
  A future cleanup should convert `lookup_expiry` to the same
  enum-match-only design as `compiled_timing_to_engine` to eliminate
  that class of bug.

## Phase 2 status

**Phase 2 is feature-complete (sub-phases 2a–2f4 landed 2026-04-23
through 2026-04-26).** Every variant of `CompiledStep` in the IR is
wired to engine behaviour:

| Sub-phase | Scope |
|---|---|
| 2a | Triggered clause lowering + memory/draw + `run_steps` scaffold |
| 2b | Selection steps (`SelectHand` / `SelectTrash` / `SelectOwn|OpponentPermanent`), binding refs, continuation dispatcher, zone moves |
| 2c | Permanent mutations (Suspend / Unsuspend / Delete / ReturnToHand / ReturnToDeck / DeDigivolve), AddDpModifier, AddModifier (binding-target), GrantKeyword, control flow (`If` / `Optional`) |
| 2d | Multi-result bindings (`PermanentList` / `CardList`), iteration verbs (`ForEach`, `PerSelected`), multi-pick selection (`SelectCountCappedMulti`), `AddModifier` filter-target arm, run_steps continuation propagation |
| 2e | `SelectEffectChoice`, `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectOrderedPermutation`, `distinct_by` enforcement on `SelectCountCappedMulti` |
| 2f1 | Play / digivolve / placement steps (`PlayFromHand*`, `PlayFromTrash*`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiated*Digivolve*`, `PlayToken`, `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`) + 5 new engine primitives |
| 2f2 | Formula values in `add_modifier` / `add_dp_modifier` (`CompiledModifierValue` IR + `formula_eval::evaluate` runtime evaluator) |
| 2f3 | `AsSelectingPlayer` override-persistence across selection callbacks (engine `new_with_override` constructor + DSL lowering) |
| 2f4 | `schedule_delayed` engine subsystem + DSL lowering (4 observer-fire wiring sites) |

Subsequent work moves to Phase 3 (§7.4) — replacement clauses,
broader `event_target_*` predicates, per-iteration park resumption,
formula primitives beyond literals (`raw_rust` registry dispatch,
`CardCountInZone` zone payload), opponent / universal `Aggregate`
scope, IR widening for `BindingValue::HandIndex` / `TrashIndex` to
carry `PlayerId`, multi-parking drains in `ScheduledEffect`, and
`OnDnaDigivolve` trigger wiring (alongside the canonical
user-action DNA digivolve flow).

### 7.4 Phase 3 — Advanced clauses

**Scope:** `replacement`, `partition`, `delay`, `schedule_delayed`,
formula primitives beyond literals, `per_selected`, `event_target_*`
predicates. Unblocks the boss-tier 15 worked examples and the long
tail of replacement-effect cards (Evade, Partition, EX5-015).

**TDD:** each new primitive gets a behavioral test *before* its
lowering lands. Parity suite expands.

**Exit criteria:** ~3,000 cards retired (75%). Hand-written card
footprint capped at ~1,000.

**Phase 3 reducer status (2026-04-26):** LANDED. Replacement process
bodies now lower through the shared DSL step runtime, replacement
`active_when` gates are honored, partition `active_when` / source /
cause gates are applied, common selection reducers (`select_any_permanent`,
`select_dna_pair`) are available, cost deltas can express printed-cost
reductions, formula-backed cost reductions and synchronous `pay_cost`
bodies lower without `raw_rust`, and DSL DNA alt-path metadata can enrich
the engine `CardData::dna_costs` surface consumed by action masks and DNA
execution.

### 7.5 Phase 4 — raw_rust escape hatch + long tail

**Scope:** `CardEffectExtensionRegistry`, raw_rust at all three
granularities, card-by-card triage of the last ~1,000. Most of that
~1,000 migrates to pure YAML as engine T5/T6 gaps close; the residual
~40–120 cards use raw_rust.

**TDD:** registry tests, per-card raw_rust tests (each raw_rust fn is
tested as a unit before wiring).

**Exit criteria:** ≥99% DSL+raw_rust coverage; hand-written
`CardEffect` crate entirely retired except for the raw_rust fns.

**Phase 4 status (2026-04-26):** LANDED.
`EngineRawRustRegistry` supports whole-clause, process-step, and
formula-level raw_rust dispatch. Step runtime is threaded through nested
DSL execution, selection continuations, delayed scheduling, and scheduled
body replay. The residual hand-written production card surface now lives
under `code/digimon-engine/src/cards/raw_rust/`; `src/cards/` no longer
contains production set modules outside the raw_rust shell.

### 7.6 Retirement schedule for `digimon-engine/src/cards/`

```
Phase 0 (end)   src/cards/  unchanged   ~34 hand-written (test cards + worked examples)
Phase 1 (end)   src/cards/  unchanged   same — no cards retired yet
Phase 2 (end)   src/cards/  shrinks     first 500 migrate; tests/*.rs rewritten
Phase 3 (end)   src/cards/  shrinks     ~3000 migrate
Phase 4 (end)   src/cards/  becomes     src/cards/raw_rust/*.rs + src/cards/test/*.rs
                             + src/cards/tokens/*.rs + keyword_effects.rs
                             a thin shell for tests, tokens, keyword auto-effects,
                             and ~120 raw_rust fns
```

## 7a. Distribution — build-time compilation, no YAML on desktop

The DSL has two different runtime profiles that must not be conflated:

1. **Dev / RL training** wants hot-reload. Engineers edit YAML, the training
   worker swaps the registry on the next episode (§4.6). YAML files live
   on disk, `CardRegistry::load()` parses them at startup.
2. **Shipped desktop app** wants the opposite: immutable, fast, no YAML
   parser in the binary, no filesystem card layout to manage. End users
   never see a YAML file.

### 7a.1 Build-time compilation to an embedded blob

`digimon-engine` grows a `build.rs` that runs at `cargo build` time:

1. Globs `digimon-engine/cards/**/*.yaml`.
2. Runs the **same validator + lowering pipeline** as `CardRegistry::load()`
   (§4.2). Any validation error fails the build.
3. Serializes the resulting `Vec<CompiledCard>` to a binary blob using
   **rkyv** (zero-copy deserialize) or **bincode** as a fallback. Format
   decision locked to rkyv in Phase 1 unless a benchmark reveals a
   load-time regression.
4. Writes the blob to `$OUT_DIR/cards.pack`.

The library crate exposes two constructors:

- `CardRegistry::from_embedded()` — `include_bytes!(concat!(env!("OUT_DIR"), "/cards.pack"))`,
  zero-copy deserialize into a `CardRegistry`. ~5 ms on desktop boot.
- `CardRegistry::load(path)` — YAML loader, dev/training only, ~100 ms.

Desktop (`src-tauri`) only ever calls `from_embedded()`. Neither `serde_yaml`
nor the validator code needs to link into the desktop binary; both are
gated behind a `dsl-yaml-loader` Cargo feature enabled by default in the
library crate but disabled by the Tauri crate's dependency declaration.
Binary-size budget: strip ~400 KB of YAML parser + schema-validation
tables from the desktop build.

### 7a.2 Runtime update channel (post-release card packs)

Follows the existing model-download pattern in [src-tauri/src/models.rs](src-tauri/src/models.rs):

- The hosted API serves `/cards/manifest.json` listing available packs
  with versions, SHA-256 hashes, and `min_engine_version:` / `max_engine_version:`
  compatibility bounds.
- Downloadable packs at `/cards/<pack_id>.pack` are **pre-compiled rkyv
  blobs** (produced server-side by the same build.rs logic running as a
  release job). No YAML is ever served to desktop clients.
- Desktop caches packs under `dirs::data_dir()/digimon-tcg/cards/<pack_id>/`
  with SHA verification on download.
- At startup, desktop consults the cache directory first:
  - If a cached pack with a compatible engine version exists and is newer
    than the embedded pack, load it via `CardRegistry::from_pack_file(path)`.
  - Otherwise fall back to `from_embedded()`.
- Pack loads are atomic — either the new registry fully deserializes or
  the old one stays; never a half-loaded state.

This lets new card sets ship without an installer update, mirroring how
`models.rs` delivers trained policies today.

### 7a.3 Consequences

1. **`raw_rust` fns stay in the binary.** They are Rust code, not data;
   they compile into `digimon-engine`. A runtime-downloaded pack can only
   add or revise cards whose implementation is pure YAML + references to
   already-shipped raw_rust fn names. Adding a new raw_rust fn is still
   an installer-update event. Pack manifests declare which raw_rust fns
   they require; the desktop rejects a pack that names an unknown fn.

2. **Pack versioning becomes load-bearing.** Every pack carries:
   - `pack_version:` — the pack's own semver.
   - `min_engine_version:` — minimum `digimon-engine` version that can
     load this pack.
   - `max_engine_version:` — optional upper bound for breaking schema
     changes.
   - `required_raw_rust_fns:` — list of fn names used by the pack.
   Desktop compares these against the built-in engine version / registry
   before accepting a pack.

3. **Blob format is not YAML and not JSON.** rkyv is the default for
   zero-copy deserialize (important for ~4,000 cards × ~4 clauses each).
   Bincode is the fallback if rkyv's stability proves insufficient at the
   engine-version boundary. JSON / YAML / MessagePack are explicitly
   rejected: parsing cost on desktop startup is the metric being
   optimized against.

4. **Dev experience: YAML is still the source-of-truth.** `git` tracks
   `cards/**/*.yaml`. The blob is a build artifact (`.gitignore`d in the
   engine crate; checked in only under `src-tauri/resources/` if a
   frozen-in-time fallback is useful — TBD, see §9).

5. **Training worker behavior.** RL training continues to read YAML
   directly via `CardRegistry::load()` — it never consumes the blob
   format. This keeps hot-reload (§4.6) trivial and avoids an extra
   serialization hop on every reload.

### 7a.4 Phase integration

The build.rs step and the `from_embedded()` constructor land in **Phase 1**
alongside the AOT compiler for the declarative subset — the blob format
is just "serialized output of the already-required lowering pipeline," so
implementing it at Phase 1 costs approximately one day on top of the
lowering work.

The runtime update channel (cache dir, manifest, SHA verification, pack
selection) lands in **Phase 3** — desktop shipping doesn't need it until
a meaningful number of cards are DSL-authored, and it reuses the existing
[src-tauri/src/models.rs](src-tauri/src/models.rs) pattern so the
implementation is a port, not a design.

## 7b. Localization + effect summaries

This section scaffolds i18n into the DSL now — not because localization
ships in v1 (it does not), but because retrofitting localization keys
into ~4,000 cards later is substantially more expensive than authoring
them from day one. Every card effect shipped through the DSL becomes a
stable localization key; when JP/ZH/etc. support is added in the
future, it is a data-loading change (new locale files), not a schema
change (no DSL edits).

### 7b.1 Three load-bearing principles

1. **Engine stays locale-agnostic.** `digimon-engine` never holds a
   localized string. It emits structured events and structured keys;
   the UI layer (Tauri shell, React frontend, hosted API clients)
   resolves keys to strings at render time via a separate
   `LocalizationDb`. This is the same architectural pattern as the
   existing tensor/mask boundary — the engine exposes data, the UI
   renders it.
2. **Every displayable string is an authored key.** Prompts
   (`prompt:` on `select_*` verbs), effect summaries (the new
   `summary:` field, §7b.3), effect choice labels (`labels:` on
   `select_effect_choice`) are all stable localization keys. The
   authored English text doubles as the `en-US` locale entry — no
   placeholder-only authoring.
3. **Card text is not re-parsed from `cards.json`** to build help
   popups. The `effect_description_eng` field stays as reference
   prose for human authors, not as a runtime data source for
   summaries.

### 7b.2 LocalizationDb — separate service, outside the engine

```
digimon-engine/
└── src/
    └── localization.rs       # NEW — trait + key types + InMemoryDb
                              # Depends only on std + serde. No
                              # engine-type imports; can be used by
                              # the Tauri shell, the hosted API, or
                              # anywhere else a string is rendered.
```

The trait:

```rust
pub trait LocalizationDb: Send + Sync {
    fn lookup(&self, key: &LocKey, locale: &Locale) -> Option<&str>;
}

pub struct LocKey {
    pub card_id: String,             // e.g. "BT17-015"
    pub kind: LocKeyKind,
}

pub enum LocKeyKind {
    CardName,
    ClauseSummary { clause_index: u16 },
    Prompt { clause_index: u16, step_path: String },  // e.g. "process[1]"
    EffectChoiceLabel { clause_index: u16, step_path: String, label_index: u8 },
    AltPathPrompt { alt_path_index: u16 },
}

pub type Locale = String;   // RFC 5646: "en-US", "ja-JP", "zh-CN"
```

Fallback chain: `locale` → `en-US` → empty string. A `LocalizationDb`
implementation loads one JSON/YAML per locale (`locales/en-US.json`,
`locales/ja-JP.json`) keyed by the same structured key shape. For v1
only `en-US` is populated, but the key system is complete.

The card-pack build step (`build.rs`, spec §7a.1) also emits a
`locales/en-US.json` blob by harvesting every `summary:` / `prompt:` /
`labels:` value from the lowered card pack. That blob ships
alongside the `.pack` blob via `include_bytes!`. Post-release locale
additions ship as additional cached files under
`dirs::data_dir()/digimon-tcg/locales/<locale>.json` through the same
hosted-API update channel used for card packs.

### 7b.3 New DSL field — `summary:` on every clause

Each triggered and declarative clause gains an **optional** `summary:`
field. Authored in English (the source locale) at ~10–25 chars —
fits the DCGO-style helper popup that fires when an inherited or
all-turns effect activates.

Schema additions:

| Field            | On                              | Type       | Notes                                              |
|------------------|---------------------------------|------------|----------------------------------------------------|
| `summary:`       | `TriggeredClause`               | string     | Optional. Short effect summary for UI popups.      |
| `summary:`       | `DeclarativeClause`             | string     | Optional. Used for `aura` / `grant_keyword` / `cost_reduction` declarative popups. |
| `summary_key:`   | both                            | string     | Optional override. If absent, derived from `(card_id, clause_index)`. |
| `prompt_key:`    | `select_*` step args            | string     | Optional override for the existing `prompt:` field. |

Authored example (BT17-015 WarGreymon):

```yaml
effects:
  - kind: cost_reduction
    scope: before_pay_cost
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Tai Kamiya"
    amount: 3
    summary: "-3 cost (Tai Kamiya)"

  - when: [on_play, when_digivolving]
    summary: "Delete 8000 DP or digivolve Gabumon free"
    process:
      - select_effect_choice:
          bind_as: branch
          labels:
            - "Delete opponent Digimon (≤8000 DP)"
            - "Digivolve Gabumon into MetalGarurumon free"
          prompt: "Choose effect"
      # ... rest of process
```

At card-pack build time the summaries are harvested:

```json
{
  "BT17-015.clause[0].summary": "-3 cost (Tai Kamiya)",
  "BT17-015.clause[1].summary": "Delete 8000 DP or digivolve Gabumon free",
  "BT17-015.clause[1].step[0].labels[0]": "Delete opponent Digimon (≤8000 DP)",
  "BT17-015.clause[1].step[0].labels[1]": "Digivolve Gabumon into MetalGarurumon free"
}
```

### 7b.4 No interpolation in v1

`summary:` strings are **static** — no `{source_name}` / `{target_dp}`
template variables. The UI layer composes source + summary via a
stable visual pattern:

```
┌──────────────────────────────────────┐
│  [icon]  WarGreymon                  │
│          -3 cost (Tai Kamiya)        │
└──────────────────────────────────────┘
```

This keeps v1 trivial: locale files are flat key→string maps. If a
future card genuinely needs interpolation ("deal {n} damage" where
`n` is computed), we add ICU MessageFormat support then. The 34-card
exploration surfaced zero cases where interpolation was required for
a summary — every effect fit in one static line.

Note: `prompt:` strings *do* sometimes benefit from interpolation
("Return {pick_count} cards to hand"). v1 carves out a narrow
exception: `prompt:` may reference `PendingSelection` struct fields
**only** — `{max}`, `{count_so_far}`, `{remaining}`, `{candidate_count}`.
Interpolation happens at UI-render time, not at YAML parse time, and
the placeholder set is a closed enum (the validator rejects unknown
`{...}` tokens in `prompt:`). Binding-value interpolation
(`{tgt.name}`) is **not** supported in v1 — that gates on the wider
ICU MessageFormat work scoped for `summary:`.

### 7b.5 Engine-side event emission

The engine gains one small addition to its event stream: when a
`process:` closure starts executing, the engine emits an
`EffectActivated { card_id, clause_index, source_permanent }` event
before the closure runs. The UI listens for this event, looks up
`(card_id, clause_index).summary`, and renders the helper popup.

This requires an `Effect` struct gain a `clause_index: u16` field
populated by the AOT compiler at lowering time. Existing
hand-written `CardEffect` implementations would need updating to set
the field, but during migration hand-written effects can default to
`clause_index = 0` and skip the popup. No parity concern — the popup
is a UI affordance, not a game-state effect.

### 7b.6 Impact on lobby pre-compilation (spec §2.4 / §7a)

Because the card pack already bakes every summary / prompt / label
into `locales/en-US.json` at build time, **no additional lobby-time
pre-compilation is required** for effect summaries. Both clients
have the full locale file bundled (desktop: embedded; online: pushed
by the hosted API on connect, or cached from a prior session).

If a per-match bundle ever becomes desirable (e.g. for smaller
over-the-wire footprint on mobile), it would filter the existing
locale file down to the union of both decks' card IDs — a pure
lookup, not a new computation. Defer until a measurement justifies
it.

### 7b.7 Phase integration

- **Phase 0:** `summary:` / `summary_key:` fields parseable on every
  clause; no engine emission yet. The JSON Schema export (Task 17)
  includes the fields so IDE auto-complete works from day one.
- **Phase 1:** card-pack build step (`build.rs`) harvests summaries
  + prompts into `locales/en-US.json` alongside `cards.pack`. The
  AOT compiler records `clause_index` into `Effect` struct field.
- **Phase 2:** engine emits `EffectActivated` events. UI adds the
  helper popup, reading from the embedded `en-US` locale.
- **Later (v2+):** JP/ZH locale files land as data-only additions;
  the hosted API's `/locales/manifest.json` endpoint ships
  community-contributed translations.

### 7b.8 Consequences for the open-questions list

The following §9 items are resolved by §7b and removed from the open
list:

- **#6 Localization** — resolved: keys scaffolded from day one, data
  deferred.

New §9 entries (added below):

- **#12 Locale-file format** — JSON (default, via serde_json) vs
  gettext `.po` (translator-friendly, more tooling) vs a custom
  format. Default JSON because it's trivial; revisit if professional
  translators are engaged.
- **#13 Clause-index stability** — if a card's YAML is reordered,
  `clause_index` changes and existing translations break. Default:
  use `summary_key:` explicit overrides for cards with translations
  already in flight. Document in the DSL authoring guide.
- **#14 Sub-clause summary granularity** — some clauses (e.g. an
  `if/then/else` branch) might want distinct summaries per branch.
  Default: one summary per top-level clause; branches inherit. If
  users want per-branch summaries, a `branch_summary:` field on
  `if` steps can be added without schema churn.

## 8. RL training implications

### 8.1 Policy invariance across backends

**Contract:** for a given card-text, the hand-written `CardEffect` and
the DSL-compiled `CardEffect` produce identical observation tensors and
identical action masks at every game step. Consequences:

- A policy trained against hand-written Rust cards continues training
  against DSL-compiled cards with no reset.
- Checkpoints are portable in both directions during migration.
- The parity test suite (§4.5) is the mechanical guarantor. If parity
  breaks on any card, the DSL compiler has drifted; revert to hand-
  written until fixed.

### 8.2 Throughput

| Backend                                | Steps/s (single env, standard deck) |
|----------------------------------------|--------------------------------------|
| Python engine                           | ~200                                |
| Rust + hand-written `CardEffect`        | ~6,000                              |
| Rust + DSL-compiled (AOT)               | ~5,000                              |
| Rust + interpreter (hypothetical)       | ~1,500                              |

AOT's overhead relative to hand-written is mostly the `BindingMap`
array scan and filter-predicate enum dispatch (~20% hot-path cost). The
end-to-end RL training speedup over Python is ~25× either way — the
DSL tax is small enough to be invisible at the training-run timescale.

### 8.3 Determinism

- Evaluator must route all randomness through `Game::rng`. The DSL has
  no `random:` verb; verbs that need randomness (shuffle, reveal,
  Random stack position) lower to engine methods that already seed
  from `Game::rng`.
- No implicit iteration order matters — `for_each` iterates in a
  documented order (ascending `PermanentHandle::index`, with ties
  broken by player id).
- `BindingMap` is not heap-allocated after construction; no allocator
  non-determinism.

### 8.4 Hot-reload during training

Hot-reload (§4.6) lets a researcher edit a YAML card file and see the
new behavior on the next training episode without a `cargo rebuild`
cycle. This is the single biggest authoring productivity win:

- Today: edit Rust, `cargo check` (~30 s), `cargo test` (~120 s),
  restart training process (~30 s) = ~3 min/edit.
- Post-DSL: edit YAML, `reload_cards()` (~25 ms/card), next episode
  sees the change = ~30 s/edit (dominated by episode length).

### 8.5 Curriculum / deck disabling

Removing a card from training is file-system-level (move the YAML to a
`_disabled/` subdir and hot-reload). Curriculum schedules become
trivially expressible as a set of file moves; no Rust feature-flag
proliferation.

### 8.6 Debugging introspection

Native: a `CardEffect` trace prints `RosemonBurstMode::effects()`.
DSL: a trace prints `DSL: BT13-060 clause 2 (when: when_attacking) at
step trash_top_source[target=event_target]`. The latter is substantially
easier to read and point users at.

## 9. Open questions

The spec deliberately does not resolve the following. They are called
out here so planning and Phase 0 scope discussions can take them up in
priority order.

1. **YAML flavor.** Plain YAML 1.2 via `serde_yaml` is the default. Some
   teams prefer `KDL` for tree-structured data or a custom indented
   syntax to avoid YAML's quoting footguns. Decision should gate on
   whether LLM authoring reliability with YAML is acceptable — empirical
   test during Phase 0.
2. **Schema validation mechanism.** `serde_yaml` + hand-written
   validators vs. a `#[derive(CardSpec)]` proc-macro that generates
   validators from the struct definition. Proc-macro scales better
   as the vocab stabilizes; `serde` is faster to ship.
3. **IDE tooling / LSP.** A Phase-2-or-later effort. Minimum: a JSON
   Schema export for VS Code's YAML extension. Stretch: a custom
   language server with card-text cross-reference.
4. **`xros_req` parsing locus.** Three options: (a) parse at DSL load
   time (loader implements a small Earley parser over the `xros_req`
   text), (b) parse at cards.json ingestion time and promote to a
   structured field on `CardData`, (c) hand-author `alt_paths:` in YAML
   and treat `xros_req` as reference prose. Option (b) is the
   lowest-friction but requires an ingestion-pipeline change;
   recommended but not locked.
5. **Tokens.** Tokens (Petrification, Petite Meteor, Ragna-Lördmon, etc.)
   are not full cards — they have no printed cost, no alt_paths, and
   are spawned rather than played. Candidate representations: (a) one
   YAML per token under `cards/_tokens/`; (b) a shared `tokens.yaml`
   with an entry per token. Recommended (a) for uniformity with the
   card loader, but this is a representation question, not a semantic
   one.
6. ~~**Localization.**~~ Resolved by §7b — i18n keys scaffolded into
   every clause via `summary:` / `summary_key:` / `prompt_key:`,
   engine stays locale-agnostic, `LocalizationDb` is a separate
   service. Only data (locale files) is deferred, not schema.
7. **Per-card versioning.** §7a.3 locks pack-level versioning
   (`pack_version`, `min_engine_version`, `required_raw_rust_fns`).
   Whether individual card files additionally carry a
   `spec_version: 1` frontmatter for finer-grained schema migrations
   within a pack is open. Default: no — the pack-level version is
   sufficient; migrations bump the pack.
8. **Lint and formatter.** A `dslfmt` tool analogous to `rustfmt` —
   canonicalizes key ordering, indentation, and alias expansion. Not
   needed for correctness; significant for LLM authoring stability
   (reduces cosmetic diff noise).
9. **Checked-in frozen fallback blob.** Should a pre-compiled
   `cards.pack` be checked in under `src-tauri/resources/` as a
   belt-and-suspenders fallback when `build.rs` cannot run (e.g.
   partial checkouts, LSP indexing, `cargo check` without the full
   YAML tree)? Default: no — `build.rs` always has access to the
   YAML tree in the monorepo. Revisit only if it becomes a friction
   point.
10. **Pack granularity for runtime updates.** One pack per TCG set
    (BT17, BT22, etc.) vs one monolithic pack vs per-archetype packs.
    Per-set is the likely default — matches how sets release — but
    cross-set cards (alt-path references, named-target auras) may
    force a looser grouping. Defer until the first post-release set
    ships.
11. **`required_raw_rust_fns` enforcement granularity.** Hard reject
    on missing fn, or soft-warn and load the pack with the
    raw_rust-dependent cards disabled? Hard reject is safer (no
    silent degradation mid-game); soft-warn is better UX for rare
    forward-compat scenarios. Default: hard reject.
12. **Locale-file format.** JSON (default, via `serde_json`) vs
    gettext `.po` (translator-friendly, more tooling) vs a custom
    format. Default JSON; revisit if professional translators are
    engaged.
13. **Clause-index stability.** If a card's YAML is reordered,
    positional `clause_index` changes and existing translations
    break. Default: `summary_key:` explicit overrides for cards
    whose translations are already in flight; positional keys for
    the rest. Document in the DSL authoring guide.
14. **Sub-clause summary granularity.** Some clauses (e.g. an
    `if/then/else`) might want distinct summaries per branch.
    Default: one summary per top-level clause. Add `branch_summary:`
    on `if` steps later if needed — non-breaking.

## 10. Worked examples (15 cards)

Each example pairs the card-text source-of-truth (trimmed from
`cards.json`) with a DSL sketch sized to its complexity. Sketches assume
the engine gap dependencies called out in §5 have landed; comments
flag where they have not yet.

### 10.1 ST2-13 Hammer Spark — hello-world Option

Card text: `[Main] Gain 1 memory.` Security: `[Security] Gain 2 memory.`

```yaml
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
  - when: on_security
    process:
      - gain_memory: 2
```

*Engine dependencies:* T6 Option `[Main]` play flow for the primary
clause. The `on_security` clause is live today.

### 10.2 BT17-007 Agumon — simple on-trigger searcher

Card text: `[Start of Your Main Phase] If you have a Tamer with [Tai
Kamiya] in its name, return 1 card with [Garurumon], [Greymon] or
[Omnimon] in its name from your trash to the hand.` Inherited: `[End
of Your Turn] This Digimon and any of your other Digimon may DNA
digivolve into a Digimon card in the hand.`

```yaml
card: BT17-007
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0

effects:
  - when: start_of_your_main_phase
    optional: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Tai Kamiya"
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter:
            any_of:
              - name_contains: "Garurumon"
              - name_contains: "Greymon"
              - name_contains: "Omnimon"
          prompt: "Return a card to hand"
      - add_to_hand_from_trash: { of: you, card: pick }

  - scope: inherited
    kind: alt_path_registration
    trigger: end_of_your_turn
    registers:
      kind: dna_digivolve
      target_zone: hand
      applies_to:
        of: you
        zone: [battle_area]
```

*Engine dependencies:* T2 `StartOfYourMainPhase`; T3 effect-driven DNA
digivolve from hand; T5 observer-timing context for the inherited
registration clause (every ally polls the registered alt-path at
end-of-turn).

### 10.3 BT22-084 Nokia Shiramine — Tamer with three clauses

Card text: `[Start of Your Turn] If you have 2 or less memory, set it
to 3. | [Start of Your Main Phase] [On Play] If you have 1 or fewer
Digimon, you may play 1 [Agumon] or [Gabumon] from your hand without
paying the cost. | [All Turns] All your Digimon with [Greymon],
[Garurumon] or [Omnimon] in their names get +1000 DP.`

```yaml
card: BT22-084
name: Nokia Shiramine
kind: tamer
color: [red, blue]
cost: 5
effects:
  - when: start_of_your_turn
    condition: { memory_lte: 2 }
    process:
      - set_memory: 3

  - when: [start_of_your_main_phase, on_play]
    optional: true
    condition:
      count_lte:
        filter: { of: you, zone: [battle_area], kind: digimon }
        n: 1
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter:
            any_of:
              - name_contains: "Agumon"
              - name_contains: "Gabumon"
          prompt: "Play for free"
      - play_from_hand_free: { of: you, hand_index: pick }

  - kind: aura
    active_when: all_turns
    target:
      of: you
      zone: [battle_area]
      any_of:
        - name_contains: "Greymon"
        - name_contains: "Garurumon"
        - name_contains: "Omnimon"
    dp_modifier: 1000

  - when: on_security
    process:
      - play_from_security: {}
```

*Engine dependencies:* T1 `play_from_hand_free`; T2 `StartOfYourTurn`
firing; T5 observer enrichment for the aura filter.

### 10.4 BT5-093 Tai Kamiya & Matt Ishida — Security Attack grant

Card text: `[Start of Your Turn] If your opponent has a level 6 or
higher Digimon in play, gain 2 memory. | [Your Turn] All of your Digimon
with [Omnimon] in their name gain <Security A. +1>.`

```yaml
card: BT5-093
name: Tai Kamiya & Matt Ishida
kind: tamer
color: [red, blue]
cost: 4
effects:
  - when: start_of_your_turn
    condition:
      any_permanent:
        of: opponent
        zone: [battle_area]
        kind: digimon
        level_gte: 6
    process:
      - gain_memory: 2

  - kind: aura
    active_when: your_turn
    target:
      of: you
      zone: [battle_area]
      name_contains: "Omnimon"
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }

  - when: on_security
    process:
      - play_from_security: {}
```

### 10.5 BT17-015 WarGreymon — boss triggered

See §2.2 for the full sketch. Dependencies: T1 `play_from_hand_free` is
*not* required here (the digivolve branch uses
`effect_initiated_digivolve`), T2 `BeforePayCost` + `cost_reduction_fn`
scanning, T3 effect-driven digivolve ignoring requirements.

### 10.6 AD1-025 Omnimon — Partition + DNA + triggered

Card text: `<Raid> <Blocker> <Partition ([WarGreymon] &
[MetalGarurumon])> | [On Play] [When Digivolving] Return all of your …`
(truncated — see cards.json). DNA path: `[DNA Digivolve] Lv.6
w/[Greymon] in name + Lv.6 w/[Garurumon] in name : Cost 0`.

```yaml
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
traits: [Holy Warrior, Royal Knight, ADVENTURE, Hero]
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, name_contains: "Greymon" }
      - { level_eq: 6, name_contains: "Garurumon" }
    cost: 0
    stacks_unsuspended: true

effects:
  - kind: grant_keyword
    keyword: Raid
  - kind: grant_keyword
    keyword: Blocker

  - kind: partition
    sources:
      - { name_contains: "WarGreymon" }
      - { name_contains: "MetalGarurumon" }
    # No exclude_cause: the Partition keyword's "not by battle" exclusion
    # is keyword-default and applied automatically at lowering — see §3.5.5.

  - when: [on_play, when_digivolving]
    process:
      # (truncated body — see DCGO for full effect)
      - raw_rust: { fn: ad1_025_on_play_process }
```

*Engine dependencies:* T5 Partition keyword + Evade-class replacement
infra; T5 source-provenance filter on replacement (`exclude_cause`);
T3 DNA digivolve via effect.

### 10.7 BT24-016 Lamiamon — source-scoped replacement + LIBERATOR

Card text: `[Hand] [Main] If you have [Owen Dreadnought], by placing 1
[Dimetromon] from your trash as any of your [Elizamon]'s bottom
digivolution card, it digivolves into this card for a digivolution
cost of 3, ignoring digivolution requirements. | [When Digivolving]
[When Attacking] [Once Per Turn] Your opponent places 1 card from their
hand as the bottom security card. Then, trash their top security card.`
Inherited: `[All Turns] [Once Per Turn] When your opponent's security
stack is removed from, you may play 1 5000 DP or lower [Reptile] or
[Dragonkin] Digimon card from your hand without paying the cost.`

```yaml
card: BT24-016
name: Lamiamon
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
traits: [Dragonkin, LIBERATOR]
alt_paths:
  - kind: digivolve
    from: { level_eq: 4 }
    cost: 3
  - kind: activated_digivolve
    from:
      of: you
      zone: [battle_area]
      name_contains: "Elizamon"
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        name_contains: "Owen Dreadnought"
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

effects:
  - when: [when_digivolving, when_attacking]
    once_per_turn: true
    process:
      - as_selecting_player:
          of: opponent
          body:
            - select_hand:
                of: opponent
                bind_as: pick
                filter: {}
                prompt: "Place a card as your bottom security card"
        # The opponent placing isn't optional per card text.
      - place_on_security:
          of: opponent
          source: { binding: pick, zone: hand }
          position: bottom
          face_up: false
      - trash_top_security: { of: opponent }

  - scope: inherited
    when: on_opponent_security_removed
    once_per_turn: true
    optional: true
    active_when: all_turns
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 5000
              - any_of:
                  - trait_has: Reptile
                  - trait_has: Dragonkin
          prompt: "Play a Reptile / Dragonkin Digimon free"
      - play_from_hand_free: { of: you, hand_index: pick }
```

*Engine dependencies:* T1 `play_from_hand_free`, `place_on_security`
with bottom position; T2 `OnOpponentSecurityRemoved` firing; T3
activated-digivolve alt-path flow with `extra_cost:`; T4 `as_selecting_player`
scope.

### 10.8 BT18-019 Millenniummon — DNA + on-deletion + conditional

Card text: `[On Play] [When Digivolving] Delete 1 of your opponent's
Digimon. Then, if DNA digivolving, by returning 1 of each Digimon card
with different levels from your opponent's trash to the top of the
deck, gain 1 memory for each card returned. | [On Deletion] By
returning 1 [Kimeramon] and 1 [Machinedramon] from your trash to the
bottom of the deck, you may play 1 [Millenniummon] from your trash
without paying the cost.`

```yaml
card: BT18-019
name: Millenniummon
kind: digimon
level: 7
color: [black]
cost: 14
dp: 13000
traits: [Composite]
alt_paths:
  - kind: dna_digivolve
    materials:
      - { name_is: "Kimeramon" }
      - { name_is: "Machinedramon" }
    cost: 0
    stacks_unsuspended: true

effects:
  - when: [on_play, when_digivolving]
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Delete a Digimon"
      - delete_permanent: { target: tgt }
      - if: { dna_origin: true }     # true when this clause fired via OnDnaDigivolve
        then:
          - select_count_capped_multi:
              of: opponent
              zone: trash
              max: 10
              bind_as: returns
              filter: { kind: digimon }
              distinct_by: level
              prompt: "Return cards (different levels each)"
          - for_each:
              over: { binding: returns }
              bind_as: c
              body:
                - return_to_deck_from_reveal:
                    of: opponent
                    card: c
                    position: top
                - gain_memory: 1

  - when: on_deletion
    optional: true
    process:
      - select_trash:
          of: you
          bind_as: kim
          filter: { name_is: "Kimeramon" }
          prompt: "Return Kimeramon"
      - select_trash:
          of: you
          bind_as: mach
          filter: { name_is: "Machinedramon" }
          prompt: "Return Machinedramon"
      - return_to_deck_from_reveal: { of: you, card: kim, position: bottom }
      - return_to_deck_from_reveal: { of: you, card: mach, position: bottom }
      - select_trash:
          of: you
          bind_as: millen
          filter: { name_is: "Millenniummon" }
          prompt: "Play Millenniummon free"
      - play_from_trash_free: { of: you, trash_index: millen }
```

*Engine dependencies:* T1 `play_from_trash_free`; T3 DNA-origin context
flag (`dna_origin` predicate); T4 distinct-by multi-select.

### 10.9 BT20-083 Omekamon — X-Antibody flavor

Card text: `<Blocker> | [On Play] If you have 1 or fewer security
cards, this Digimon may digivolve into [Omnimon (X Antibody)] in the
hand, ignoring digivolution requirements and without paying the cost.
| [On Deletion] You may place this card as the bottom digivolution
card of your [King Drasil_7D6] in the breeding area.` Inherited:
`[Breeding] [Opponent's Turn] When your security stack is removed
from, by suspending this Digimon, play 1 [Omekamon] from this
Digimon's digivolution cards without paying the cost.`

```yaml
card: BT20-083
name: Omekamon
kind: digimon
level: 4
color: [red, blue]
cost: 5
dp: 4000
traits: [Puppet, X Antibody, LIBERATOR]

effects:
  - kind: grant_keyword
    keyword: Blocker

  - when: on_play
    optional: true
    condition: { security_count_lte: 1 }
    process:
      - select_hand:
          of: you
          bind_as: omni_x
          filter: { name_is: "Omnimon (X Antibody)" }
          prompt: "Digivolve into Omnimon (X Antibody) free"
      - effect_initiated_digivolve:
          target: self
          from_hand: omni_x
          cost: 0
          ignore_requirements: true

  - when: on_deletion
    optional: true
    process:
      - select_own_permanent:
          bind_as: kd
          filter:
            all_of:
              - name_is: "King Drasil_7D6"
              - zone: [breeding]
          prompt: "Place as King Drasil bottom source"
      - place_as_bottom_source:
          source: { permanent: self }
          target: kd

  - scope: inherited
    when: on_opponent_security_removed
    active_when: { all_of: [in_breeding, opponents_turn] }
    optional: true
    process:
      - suspend: { target: carrier }
      - select_material:
          of_permanent: carrier
          bind_as: slot
          filter: { name_is: "Omekamon" }
          prompt: "Play Omekamon from materials"
      - play_from_materials:
          target: carrier
          source_index: slot
          cost_delta: free
```

*Engine dependencies:* T1 `play_from_materials` and
`place_as_bottom_source`; T3 effect-initiated digivolve from hand; T5
provenance-aware source-zone checks for the inherited breeding clause.

### 10.10 BT18-102 Susanoomon — Hybrid 10-material + Blast Digivolve

Card text: `[Hand] [Counter] <Blast Digivolve> | [When Digivolving]
[When Attacking] Delete 1 of your opponent's Digimon with 10000 DP or
less. For each color in this Digimon's digivolution cards, add 2000
to this DP deletion effect's maximum. | [When Attacking] By placing up
to 5 Tamer cards from this Digimon's digivolution cards as your bottom
security cards, trash your opponent's top security card for each card
placed by this effect.` Inherited: `Ace Overflow <-5>`. Digivolve
requirement: `Lv.6 [Takuya Kanbara] or [Koji Minamoto] w/10 [Hybrid]
trait cards under: Cost 6`.

```yaml
card: BT18-102
name: Susanoomon
kind: digimon
level: 7
color: [red, blue, yellow, green, black, purple]  # all six
cost: 9
dp: 15000
traits: [Shaman]
ace_overflow: -5
alt_paths:
  - kind: assembly
    from:
      any_of:
        - { name_is: "Takuya Kanbara", level_eq: 6 }
        - { name_is: "Koji Minamoto", level_eq: 6 }
    materials:
      - filter: { trait_has: "Hybrid" }
        repeat: { min: 10, max: 10 }
        stack_under: true
    cost: 6
  - kind: burst_digivolve
    marker: true    # [Hand] [Counter] <Blast Digivolve>
    cost: 0

effects:
  - when: [when_digivolving, when_attacking]
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter:
            dp_lte:
              formula:
                base: 10000
                per: digivolution_color_count
                delta: 2000
          prompt: "Delete"
      - delete_permanent: { target: tgt }

  - when: when_attacking
    optional: true
    process:
      - select_count_capped_multi:
          of: you
          zone: material
          max: 5
          bind_as: placed
          filter:
            all_of:
              - of_permanent: self
              - kind: tamer
          prompt: "Place Tamer materials as bottom security"
      - for_each:
          over: { binding: placed }
          bind_as: m
          body:
            - place_on_security:
                of: you
                source: { binding: m, zone: material, of_permanent: self }
                position: bottom
                face_up: false
            - trash_top_security: { of: opponent }
```

*Engine dependencies:* T3 Hybrid/assembly alt-path, Blast Digivolve
Counter-window handling; T4 material-zone multi-select; T5 Ace Overflow
memory penalty with `LeaveCause`.

### 10.11 BT13-060 Rosemon: Burst Mode — Burst with turn-end teardown

Card text: `[When Digivolving] Suspend 1 of your opponent's Digimon
and 1 of their Tamers. Until the end of your opponent's turn, all of
their Digimon and Tamers don't unsuspend. | [When Attacking] Trash the
top card of your opponent's security stack for every 2 of your
opponent's suspended Digimon or Tamers.` Digivolve: `[Burst Digivolve]
[Rosemon]: By returning 1 [Yoshino Fujieda] to hand, cost 0. At the end
of the burst digivolution turn, trash this Digimon's top card.`

```yaml
card: BT13-060
name: "Rosemon: Burst Mode"
kind: digimon
level: 7
color: [green]
cost: 15
dp: 15000
traits: [Fairy]
alt_paths:
  - kind: digivolve
    from: { level_eq: 6, name_is: "Rosemon" }
    cost: 5
  - kind: burst_digivolve
    from: { level_eq: 6, name_is: "Rosemon" }
    cost: 0
    extra_cost:
      - select_own_permanent:
          bind_as: yoshi
          filter: { kind: tamer, name_is: "Yoshino Fujieda" }
          prompt: "Return Yoshino Fujieda"
      - return_to_hand: { target: yoshi }
    on_burst_turn_end:
      - trash_top_source: { target: self }

effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: d
          filter: { kind: digimon }
          prompt: "Suspend opponent Digimon"
      - suspend: { target: d }
      - select_opponent_permanent:
          bind_as: t
          filter: { kind: tamer }
          prompt: "Suspend opponent Tamer"
      - suspend: { target: t }
      - add_modifier:
          target: { of: opponent, zone: [battle_area], any_of: [{ kind: digimon }, { kind: tamer }] }
          modifier: CannotUnsuspend
          value: 1
          expiry: end_of_opponents_turn

  - when: when_attacking
    process:
      - lose_count_bound:   # sugar for "for every N matching, do X once"
          filter:
            of: opponent
            zone: [battle_area]
            any_of: [{ kind: digimon }, { kind: tamer }]
            is_suspended: true
          divisor: 2
          body:
            - trash_top_security: { of: opponent }
```

*Engine dependencies:* T3 Burst Digivolve alt-path + burst-turn-end
teardown hook; T5 plural-aura modifier targeting (the
`add_modifier` with a filter target); `lose_count_bound` sugar is new —
it is equivalent to `for_each` over a bounded iterator and deserves a
design note in §9 to decide whether to fold into `for_each`.

### 10.12 BT13-007 King Drasil_7D6 — stress test (Royal Knight)

See §2.2 for the full sketch. Dependencies: T1 breeding-scope zone
predicates + `place_as_bottom_source`; T2 `StartOfYourMainPhase` in
breeding scope; T5 "`on_option_placed`" observer (sugar for
`OnEnterFieldAnyone` filtered by `kind: option`); T5 computed
cost-reduction formula (or raw_rust fallback).

### 10.13 BT12-112 Shoutmon X7: Superior Mode — ∞ DigiXros

Card text: `When you would play this card, by placing 1 [Shoutmon] as
a digivolution card under this Digimon, reduce the cost by 1 and you
may place cards in your trash as digivolution cards for a DigiXros. |
[On Play] Return all of the digivolution cards of 1 of your opponent's
Digimon to the bottom of its owner's deck, and return that Digimon to
the bottom of its owner's deck. | [Your Turn] All of your opponent's
[Security] effects on Option cards don't activate.`

```yaml
card: BT12-112
name: "Shoutmon X7: Superior Mode"
kind: digimon
level: 7
color: [red]
cost: 15
dp: 17000
traits: [Composite, Xros Heart, Blue Flare]
alt_paths:
  - kind: digixros
    materials:
      - filter:
          any_of:
            - trait_has: "Xros Heart"
            - trait_has: "Blue Flare"
        repeat: unbounded
        distinct_by: card_number
        zones: [hand, battle_area, trash]    # per the [When would be played] sugar
    cost:
      formula:
        base: 15
        per: material_count
        delta: -1
  - kind: cost_reduction
    scope: before_pay_cost
    when_playing_this: true
    pay_cost:
      - select_own_permanent:
          bind_as: shout
          filter: { name_is: "Shoutmon" }
          prompt: "Place Shoutmon as source"
      - place_as_bottom_source:
          source: { permanent: shout }
          target: self
    amount: 1
    unlocks:
      - digixros_zones_extend: [trash]

effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Return to deck"
      - return_to_deck: { target: tgt, position: bottom, include_sources: true }

  - kind: flood_gate
    scope: face_up
    active_when: your_turn
    modifier: CannotActivateSecurityEffects
    target:
      of: opponent
      filter: { kind: option }
```

*Engine dependencies:* T3 DigiXros with cross-zone materials and
unbounded `repeat`; T3 digivolve-override "when you would play this"
sugar (expressed as a `cost_reduction` clause with `pay_cost:` and
`unlocks:`); T6 per-kind `CannotActivateSecurityEffects` flood gate
scoped to Option-source effects only (today it is player-scoped; a
source-kind-scoped variant is new).

### 10.14 BT10-111 Shoutmon (King Version) — raw_rust escape hatch probe

Card text: `[On Play] You may return 1 card with a DigiXros requirement
from your trash to your hand. When you would DigiXros this turn, this
Digimon may replace 1 of the DigiXros requirements. | <Material Save 1>`
Inherited: `[Your Turn] While this Digimon has [Shoutmon] in its name,
it gets +2000 DP.`

```yaml
card: BT10-111
name: "Shoutmon (King Version)"
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
traits: [Mini Dragon, Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { trait_has: "Xros Heart" }
        repeat: { min: 1, max: 1 }
    cost:
      formula:
        base: 5
        per: material_count
        delta: -2

effects:
  - when: on_play
    optional: true
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter: { has_alt_path: digixros }
          prompt: "Return a DigiXros card to hand"
      - add_to_hand_from_trash: { of: you, card: pick }

  - when: on_play
    kind: raw_rust
    fn: bt10_111_arm_digixros_wildcard_for_turn

  - kind: grant_keyword
    keyword: MaterialSave
    value: 1

  - scope: inherited
    kind: aura
    active_when: { all_of: [on_field, source_name_contains: "Shoutmon"] }
    target: { permanent: carrier }
    dp_modifier: 2000
```

The second `on_play` clause is a whole-clause raw_rust. Its fn
registers a turn-scoped
`ModifierType::XrosSubstitutionAvailable` on the controller; the
DigiXros material-legality check consults the flag. This is the
prototypical case where the DSL vocabulary cannot reach and raw_rust
is the right tool.

### 10.15 EX11-012 Medusamon — cross-mechanic LIBERATOR

Card text: `<Rush> <Progress> | [When Digivolving] [End of Attack] You
may delete 1 of your opponent's Digimon with as much or less DP as
this Digimon. Then, by returning 1 card from your opponent's trash to
the bottom of the deck, they play 1 [Petrification] Token.
(Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend. [On
Deletion] Trash your top security card.)` (truncated; the `[All
Turns]` clause is cut from cards.json in the sample).

```yaml
card: EX11-012
name: Medusamon
kind: digimon
level: 6
color: [purple]
cost: 11
dp: 11000
traits: [Dragonkin, LIBERATOR]

effects:
  - kind: grant_keyword
    keyword: Rush
  - kind: grant_keyword
    keyword: Progress

  - when: [when_digivolving, end_of_attack]
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter:
            all_of:
              - kind: digimon
              - dp_lte: { formula: { of: source_permanent, value: dp } }
          prompt: "Delete"
      - delete_permanent: { target: tgt }
      - optional:
          - select_trash:
              of: opponent
              bind_as: r
              filter: {}
              prompt: "Return a card to bottom of deck"
          - return_to_deck_from_reveal: { of: opponent, card: r, position: bottom }
          - play_token:
              controller: opponent
              token_name: Petrification

  # (remainder of [All Turns] clause elided — see cards.json for the full body)
```

*Engine dependencies:* T1 `play_token` is live today; T5 `Progress`
keyword; T5 source-scoped `CannotBeAffected` — required by `<Progress>`
but implementable via modifier with `while_attacking` expiry.

## 11. Summary

The DSL is data, not a language. Its tractability comes from the
bounded vocabulary (~180 items at the 4,000-card scale), the AOT
compile step (no runtime interpreter), and the hybrid raw_rust escape
hatch that keeps the 1–5% of stubborn cards from distorting the design.
The DSL does not shortcut engine work — every Tier-1..Tier-6 gap in the
archetype gap plans remains on the critical path. What the DSL does
shortcut is the *next 4,000 cards after the engine is complete*: cards
become data that an LLM authors from card text against a strict
schema, with parity, determinism, and RL throughput invariants
preserved end-to-end.

Open questions in §9 (YAML flavor, schema validation mechanism, LSP
tooling, `xros_req` parsing locus, tokens, localization, versioning,
lint tool) are the items planning must resolve in priority order
before Phase 0 can begin. None of them changes the architecture locked
above.
