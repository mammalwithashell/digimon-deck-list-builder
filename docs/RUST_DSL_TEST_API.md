# Rust DSL Card Test API Reference

**Audience:** AI agents (and humans) writing tests for DSL-defined Digimon card effects in `digimon-engine`.

**Last refreshed: 2026-05-15** — Tracks A–K substrate sweep. Added §15
(replacement / aura / granted-triggered / breeding-area / RevealBucket
test patterns) and refreshed the DebugRunner method index against the
shipping runner. See `docs/RUST_ENGINE_API.md` for the matching engine API
refresh.

This document is the canonical reference for how Rust-side card tests are structured. Read alongside:

- [`docs/RUST_ENGINE_API.md`](RUST_ENGINE_API.md) — `EffectContext` API surface, `CardEffect` trait, engine primitives.
- [`docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`](superpowers/specs/2026-04-21-card-scripting-dsl.md) — DSL syntax, compile pipeline, vocabulary.
- [`docs/RUST_PYTHON_PARITY.md`](RUST_PYTHON_PARITY.md) — cross-engine divergences. Check before assuming a behavior carries across.
- [`docs/RUST_ENGINE_GAPS.md`](RUST_ENGINE_GAPS.md) — live engine-primitive gap tracker. Use the gap-id when marking a test `#[ignore]`.

The `/batch-implement-cards-rust-dsl` skill cites this document directly. Every per-card test the skill authors must conform to the patterns described here.

---

## 1. Project layout

```
code/digimon-engine/
├── cards/<set>/<card_id>.yaml          # Production DSL card specs (canonical)
├── cards/_examples/                    # Hand-curated YAML fixtures used in docs and infra tests
├── src/
│   ├── dsl_cards/                      # Lowering: CompiledCard → Effect closures
│   ├── dsl_registry.rs                 # Embedded + on-disk pack loaders
│   └── debug_runner.rs                 # Test harness (see §3)
└── tests/
    ├── cards_behavioral/               # Per-card TDD tests (the home for DSL card tests)
    │   ├── main.rs                     # Module index
    │   ├── <set>/                      # One subdirectory per set (bt15/, bt17/, ex11/, ...)
    │   │   ├── mod.rs                  # Registers all card tests in the set
    │   │   └── <card_id>.rs            # One file per card (e.g. bt15_003.rs)
    │   └── ...
    ├── combat/                         # Combat math, attack windows, security DP
    ├── effects/                        # Cross-card effect interactions
    ├── selection/                      # Pending-selection routing
    ├── replacements/                   # Would-replacement plumbing
    ├── flood_gates/                    # Player-restriction modifiers
    ├── option_flow/                    # Option card resolution
    ├── keyword_parsing.rs              # Printed-keyword parser
    └── dsl/                            # DSL infra (parser, validator, lowering)
```

### File and module naming

`card_id` carries dashes (`BT15-003`) but Rust modules cannot. Convert to lowercase with underscores:

| Card ID | Module | File |
|---|---|---|
| `BT15-003` | `bt15_003` | `tests/cards_behavioral/bt15/bt15_003.rs` |
| `EX11-027` | `ex11_027` | `tests/cards_behavioral/ex11/ex11_027.rs` |
| `P-117` | `p_117` | `tests/cards_behavioral/p/p_117.rs` |
| `LM-029` | `lm_029` | `tests/cards_behavioral/lm/lm_029.rs` |

Each `<set>/mod.rs` registers every card module in the set:

```rust
// tests/cards_behavioral/bt15/mod.rs
mod bt15_003;
mod bt15_077;
mod bt15_092;
```

`tests/cards_behavioral/main.rs` registers the set modules, plus any cross-cutting suites that share the binary:

```rust
mod bt15;
mod ex11;
mod de_digivolve;
mod tokens;
```

### Production YAML location

Production card specs live at `code/digimon-engine/cards/<set>/<card_id>.yaml`. The pack build (`build.rs`) scans the whole `cards/` tree by direct subdirectory, including production set directories and `_examples/` fixtures, and bundles the valid specs into `OUT_DIR/cards.pack`. `dsl_registry::from_embedded()` loads that pack at runtime. Tests using `runner.dsl_card("BT15-003")` resolve through the embedded pack - they read the same spec the shipping engine reads.

---

## 2. Test taxonomy

Four buckets. Each bucket has a clear home; do not mix them.

### Per-card behavioral tests (`tests/cards_behavioral/<set>/<card_id>.rs`)

One file per card. Tests every clause that card declares — structural shape, condition gating, behavioral outcome, branch coverage. Most of the test suite lives here. **§4** is the full walkthrough.

### Mechanic-level tests (`tests/{combat, effects, selection, replacements, flood_gates, option_flow, keywords}/...`)

Cross-card mechanic interactions: attack windows, Piercing math, security DP, OPT enforcement across copies of the same card, replacement-effect ordering, flood-gate stacking. These tests load *DSL fixture cards* (real shipping YAML or minimal inline YAML) — never synthetic `TEST-*` cards if a real card exhibits the mechanic. **§8** covers patterns.

### Archetype interaction tests (`tests/archetypes/<archetype_slug>.rs`)

One file per archetype. Each `#[test]` asserts a **named combo from that archetype's model** (`qa/archetype-qa/<archetype>-model.md`) — multiple *real implemented* cards playing together as the deck does, asserting the combo's claimed *mechanical* outcome (and, where useful, the unhappy path where a missing enabler breaks it). Unlike mechanic-level tests (which pin one engine mechanic, often with fixture cards), these are *system-level*: each test is traceable to a combo a human can check against card text, and a failure is triaged like a replay divergence (confirm vs card text + `general_rule.pdf` + DCGO C# before filing). Authored by the `/archetype-interaction-test-author` skill (the capstone that runs after the archetype's cards are implemented and per-card tests are green); the per-card behavioral coverage stays in `cards_behavioral/`.

Shared multi-card fixtures live in `tests/archetypes/support.rs`:

- `dsl_builder(&["BT17-102", ...])` → a `DebugRunnerBuilder` with N DSL cards loaded (panics, naming the card, on a typo / un-migrated id). Chain `.add_card(...)` for synthetic targets, then the usual `.hand()` / `.deck()` / `.security()` / `.memory()` / `.start()`.
- `BoardSnapshot` + `snapshot(&runner)` → both players' `field`/`hand`/`security`/`trash`/`deck` sizes + `memory` in one struct; diff a before/after pair to assert the combo's net board change.
- `run_actions(&mut runner, &[(player, action_id)])` → drive a scripted decision sequence, auto-resolving mandatory follow-ups between steps.

Richer board setup (digivolution stacks via `place_stack` / `place_field_stack`, breeding, security/trash seeding) is already on `DebugRunner` / `DebugRunnerBuilder`. See `tests/archetypes/rocks.rs` for the exemplar (the Koromon-enabled Greymon removal combo: the same opponent board flips from deletable to safe depending on whether the enabler is in the stack — a system-level fact no per-card test sees).

### DSL infra tests (`tests/dsl/...`)

Parser, validator, and lowering tests. Out of scope for card-script authors — touched only when the DSL vocabulary itself changes. Existing infra tests already use inline YAML extensively (see `tests/dsl/phase2b_end_to_end.rs`); preserve that style there.

Current Phase 3 infra coverage includes:

- `phase3d_formula_zone_count.rs`: `card_count_in_zone` formulas with `zone` and `of` payloads.
- `phase3d_aggregate_scope.rs`: aggregate formulas scoped to controller, opponent, active player, or any player.
- `phase3d_raw_rust_formula.rs`: runtime dispatch for registered `raw_rust` formula callbacks.
- `phase3d_event_context.rs`: `event_target_*`, `event_card_trait_has`, `event_target`, and `event_card`.
- `phase3d_scheduled_generation.rs`: next-turn delayed timings such as `end_of_your_next_turn`.
- `phase3e_scheduled_reentry.rs`: scheduled effects that park on a DSL selection and then resume.
- `phase3e_on_dna_digivolve.rs`: `on_dna_digivolve` for effect-initiated and user-action DNA.

### Decision tree

> Is the test about a single card's effects? → **per-card** in `cards_behavioral/<set>/`.
> Is the test about one engine *mechanic* across ≥2 cards (e.g. Alliance triggering off another card's attack)? → **mechanic-level** under the relevant mechanic dir.
> Is the test a *named combo* from an archetype's model (its real cards playing as the deck does)? → **archetype interaction** in `tests/archetypes/<slug>.rs`.
> Is the test about whether the DSL parses, validates, or lowers correctly? → **DSL infra** under `tests/dsl/`.

When a test could plausibly live in two buckets, prefer per-card. Mechanic tests should focus on mechanics that span ≥2 cards; archetype interaction tests should be traceable to a specific combo in the archetype-model doc.

---

## 3. DebugRunner DSL test surface

Tests drive the engine through `DebugRunner`
(`code/digimon-engine/src/debug_runner.rs`). The DSL-aware additions to the
runner are listed below. The original spec used **(spec)** annotations for
methods that had not yet landed; as of 2026-05-15 every method below is
implemented — see `debug_runner.rs` for signatures and `tests/debug_runner_dsl.rs`
for the self-tests. Treat the **(spec)** markers as historical context only.

### Loading a card

| Method | Purpose | When to use |
|---|---|---|
| `DebugRunner::builder().dsl_card("BT15-003")` **(spec)** | Look up `BT15-003` in the embedded card pack, register its `DslCardEffect`, populate `card_data` with metadata derived from the compiled spec. | Per-card behavioral tests, mechanic tests using real cards. The default. |
| `DebugRunner::builder().from_dsl_yaml(yaml_str)` **(spec)** | Parse `yaml_str` as a `CardSpec`, compile, register. Returns the runner with the card's `card_id` available in subsequent `.hand()` / `.deck()` calls. | DSL infra tests, parser edge cases, "this card with one field tweaked" tests. |
| `DebugRunner::builder().add_card(...)` (existing) | Synthetic test card (`make_test_card`). | TEST-* cards only — never use to stand in for a real DSL card. |

The two DSL methods can be chained: load several real cards by ID, then add an inline fixture, then populate hands and decks normally.

```rust
let mut runner = DebugRunner::builder()
    .dsl_card("BT17-015")             // WarGreymon
    .dsl_card("BT5-091")              // Tai Kamiya (for cost-reduction condition)
    .hand(0, &["BT17-015"])
    .deck(0, &["FILLER"; 50])
    .memory(8)
    .start();
```

### Driving the action loop

| Method | Purpose |
|---|---|
| `runner.play(player, hand_index)` (existing) | Play from hand; returns the new battle-area index. |
| `runner.attack_digimon(attacker, defender, vortex)` / `attack_player(...)` (existing) | Combat entry points. |
| `runner.execute_action(action_id: u16)` **(spec)** | Submit an action ID to the action decoder. Used to drive a specific branch of a pending selection. |
| `runner.auto_resolve()` **(spec)** | Pick the *first legal action* at every prompt until no `pending_selection` remains. Mirror of Python `runner.auto_resolve()`. Use only when the test is asserting end-state aggregates, not branch-specific behavior. |
| `runner.execute_branch(label_index: usize)` **(spec)** | For `EffectChoice` selections only — submit the action ID matching `effect_choices[label_index].action_id`. Convenience wrapper over `execute_action`. |

### Inspecting pending selections

| Method | Purpose |
|---|---|
| `runner.pending_selection()` **(spec)** → `Option<&PendingSelection>` | Reference to the currently-parked selection (or `None`). |
| `runner.pending_selection_view()` **(spec)** → `Option<PendingSelectionView>` | Cloneable, callback-free snapshot — use for assertions. |
| `runner.pending_kind()` **(spec)** → `Option<SelectionKind>` | Convenience for the most common assertion. |
| `runner.pending_is_optional()` **(spec)** → `bool` | Whether `PASS` is a legal action. |
| `runner.pending_action_count()` **(spec)** → `usize` | Number of legal action IDs (excluding PASS). |

### Inspecting the compiled card

| Method | Purpose |
|---|---|
| `runner.compiled_card(card_id)` **(spec)** → `&CompiledCard` | Borrow the compiled spec for structural assertions (clause count, kinds, OPT flags). |
| `runner.dsl_clause(card_id, idx)` **(spec)** → `&CompiledClause` | Convenience for `compiled_card(...).effects.get(idx)`. |

### Permanent placement and modifiers (existing, for cross-reference)

| Method | Purpose |
|---|---|
| `runner.place_on_field(player, card_id, turn_played_override)` | Skip the play action; drop a permanent directly onto the field. |
| `runner.fire_on_play(player, field_index)` | Manually fire the OnPlay batch for a permanent already on the field. |
| `runner.effective_dp(handle)` / `runner.dp_of(handle)` | Base + modifier sum. |
| `runner.modifiers()` → `&ModifierRegistry` | Inspect typed modifiers + expiry. |

### Event log inspection (replaces Python `_fire_timing` spy)

| Method | Purpose |
|---|---|
| `runner.events_since(checkpoint)` **(spec)** → `&[GameEvent]` | Slice of events emitted after a checkpoint cursor. |
| `runner.event_checkpoint()` **(spec)** → `usize` | Capture the current event-log length for later slicing. |
| `runner.events_of_kind(kind)` **(spec)** → `Vec<&GameEvent>` | All events matching a discriminant (e.g. `OnDiscardSecurity`). |

The `GameEvent` type already exists in `code/digimon-engine/src/events.rs`; the helpers above are thin views over `Game::event_log`.

---

## 4. Example card pool — research task

Worked examples in this document (§5–§8 and §11) draw from a curated card pool selected by an explicit research task. The task is defined here so the selection is reproducible.

### Deliverable

`qa/dsl-test-pool.md` — a table of 22–28 cards covering the pattern taxonomy in §4.3. One card may anchor multiple patterns; the table cross-references which doc sections each card serves.

### Inputs

All required:

- `data/deck_library.json` — every meta decklist scraped to date.
- `data/cards.json` — full card metadata for filtering by trait / level / cost.
- `code/engine_py_legacy/engine/data/scripts/` — Python ground-truth implementations.
- `DCGO/Assets/Scripts/CardEffect/` — C# behavioral reference.
- `qa/archetype-qa/INDEX.md` and the per-archetype QA docs for the **17 launch archetypes** (Chaos Control, Medusamon, Dark Masters, TS Jupitermon, Royal Knights, DNA Omnimon, Jesmon, Puppets, Hudiemon, TS Neptunemon, Millenniummon, ExMaquinamon, Galacticmon, Zephagamon, BG Imperial, Rocks, TS Olympos). Verdict tables surface canonical examples per pattern.
- `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` — exclude any card depending on an unclosed engine primitive or DSL vocabulary gap.

### Selection rules (per row)

1. The card must have an existing Python script *or* DCGO C# reference (test author needs ground truth).
2. For deck-anchored patterns (Liberator, Dark Masters, etc.): pick the most-included card across decklists; break ties by simplest YAML.
3. For generic patterns (searching rookie, OPT, etc.): pick a card whose YAML is ≤40 lines so the worked example reads cleanly.
4. Skip any card depending on an unclosed engine gap.

### Pattern taxonomy

The pool collectively must cover all eight groups below.

#### Group A — Zone moves & search

| # | Pattern | Canonical examples |
|---|---|---|
| A1 | Searching rookie (top-N reveal, add by trait/name) | BT9-092 Hina Kurihara, Vemmon variants, BT22-017 Gabumon |
| A2 | Two-pass reveal (reveal, sort, add or place) | BT18-060 Vemmon, BT15-077 LadyDevimon, BT24-066 Guilmon |
| A3 | Reveal + select-multi (reveal N, choose ≤K) | BT13-087 Dynasmon, EX4-038 Agumon |
| A4 | Trash → hand recursion | BT7-107 Calling From the Darkness |
| A5 | Stack shift / digivolve from trash | EX11-005 Yaamon, BT24-070 Growlmon |
| A6 | Trash-to-digi-stack (place under) | BT13-075 Alphamon |

#### Group B — Tamers & start-of-main

| # | Pattern | Canonical examples |
|---|---|---|
| B1 | Start-of-main tamer (memory swing / draw / recur) | EX7-063 Arisa Kinosaki, BT24-083 Hiroko Sagisaka, BT22-088 Arisa |
| B2 | Tamer play-4 anchor (cost-4 persistent on-your-turn) | BT22-084 Nokia Shiramine, BT22-089 Mirei Mikagura |
| B3 | Trigger-on-event tamer (on hatch, on play, on deletion) | BT17-093 Kari Kamiya, BT3-093 Davis Motomiya |

#### Group C — Linked cards & DigiXros / Assembly

| # | Pattern | Canonical examples |
|---|---|---|
| C1 | Linked-card source (digivolve from hand OR link cards) | EX11-033 Maneuvermon, EX11-042 MockingBirdmon |
| C2 | Linked-with-X gating | EX11-006 Flickmon (linked-with-Maquinamon check) |
| C3 | WhenRemoveField link selection (3-way zone choice on removal) | EX11-027 Maquinamon, EX11-073 ExMaquinamon |
| C4 | DigiXros source play | BT18-065 Snatchmon, EX11-070 Unchained |
| C5 | Mega Digimon Assembly! (security trash-to-hand with selection) | EX6-072 |

#### Group D — Conditional effects & modifiers

| # | Pattern | Canonical examples |
|---|---|---|
| D1 | Conditional DP buff (trait/board gate, EndOfAttack expiry) | BT17-015 WarGreymon, EX10-010 BlackWarGreymon |
| D2 | Cost reduction with BeforePayCost leak guard | BT13-007 King Drasil, BT13-111 Gallantmon, BT9-112 DeathXmon |
| D3 | Color ignore / bypass | BT21-100 The Digimon I Designed, BT24-090 Abyss Sanctuary |
| D4 | Declarative aura (passive +DP / keyword grant) | BT6-082/084 Sistermons, BT11-042 Angewomon |
| D5 | Aura with face-down security gate | BT24-090 Abyss Sanctuary, BT24-094 Central Town |
| D6 | Flood gate / opponent restriction | BT14-009 Gotsumon (CANNOT_PLAY_CARD) |

#### Group E — Branch choices & optionality

| # | Pattern | Canonical examples |
|---|---|---|
| E1 | Branch-choice OnPlay ("choose: A or B") | BT17-015 WarGreymon (2-way), EX11-027 Maquinamon (3-way) |
| E2 | OPT + optional decline (cost-paid, declinable) | BT15-003 Nyaromon |
| E3 | OPT shared hash across copies | BT21-029 Medusamon |

#### Group F — Combat, deletion, replacement

| # | Pattern | Canonical examples |
|---|---|---|
| F1 | Force attack / FORCE_ATTACK | BT20-017 Jesmon, EX11-036 Dalphomon |
| F2 | Redirect attack | BT11-070 Destromon, P-094 Destromon |
| F3 | Replacement effect (would-trash → suspend instead) | EX11-073 ExMaquinamon, BT23-058 Craniamon (will_not_be_removed) |
| F4 | WhenRemoveField vs WhenPermanentWouldBeDeleted | BT24-030 Neptunemon (the timing-distinction is the test point) |
| F5 | OnDeletion → security | EX10 Dark Masters suite (EX10-012/020/035/057/061) |
| F6 | Effect immunity / IMMUNE_FROM_X | BT21-060 Destromon, EX11-074 Vortexdramon, EX10-010 BlackWarGreymon |
| F7 | Cannot suspend / cannot unsuspend | EX10-012 MetalSeadramon, BT12-057 Quartzmon |
| F8 | Scapegoat / Barrier / decoy prevention | BT15-072 Vilemon, EX11-019 Shoemon (Barrier) |
| F9 | Security-loss conditioned tamers (Owen Dreadnought) | BT18-087 / BT24-082 Owen Dreadnought |

#### Group G — Archetype/structural patterns

| # | Pattern | Canonical examples |
|---|---|---|
| G1 | also_treated_as / name aliases | BT23-077 Sistermon Ciel, BT20-083 Omekamon, BT24-080 Megidramon |
| G2 | DNA digivolve (multi-source materials, dna_costs) | EX9-013 BlitzGreymon, BT16-077 Dinobeemon |
| G3 | DNA digivolve On Deletion | BT18-015 Kimeramon, EX11-045 Metatromon |
| G4 | Inherited When Attacking on DigiEgg (OPT + cost + selection) | BT15-003 Nyaromon |

#### Group H — Printed keywords (one canonical card each)

Every shipping keyword should have one card in the pool that exercises it cleanly, so §8's keyword section can cite a real card per row.

| # | Keyword | Canonical example |
|---|---|---|
| H1 | Rush | BT13-110 Royal Knights of the Purge (Rush via modifier) |
| H2 | Jamming | BT23-018 Garurumon |
| H3 | Piercing | BT20-045 Examon, EX11-074 Vortexdramon |
| H4 | Security A. ±N | BT15-084 Kari Kamiya (-1), BT22-006 (+1) |
| H5 | Blocker | EX10-061 Apocalymon (Blocker grant), BT5-035 Knightmon |
| H6 | Decoy | ST12-12 Sistermon Blanc — **engine gap** (Decoy color restriction). Pool entry remains `#[ignore = "pending: decoy-color-restriction"]` until the gap closes. |
| H7 | Reboot | BT23-085 Ryuji Mishima |
| H8 | Evade | BT15-101 MetalGarurumon |
| H9 | Raid | BT23-008 Greymon, BT17-078 Omnimon |
| H10 | Alliance | BT23-041 Kabuterimon, BT23-051 Golemon |
| H11 | Overclock | BT22-036 Chaperomon, EX7-027 Chaperomon |
| H12 | Blast (Blast Digivolve / Blast DNA) | BT17-078 Omnimon, BT20-060 Alphamon: Ouryuken |
| H13 | ACE | BT20-101 Zephagamon, BT24-017 Megidramon |
| H14 | Barrier | EX11-019 Shoemon |
| H15 | Scapegoat | EX11-022 Karakurumon |
| H16 | Vortex (end-of-turn attack) | BT20-101 Zephagamon (Piercing factory + Vortex) |

### Acceptance criteria

1. Every pattern row in groups A–H has at least one card.
2. Every card in the pool cross-references which doc sections it anchors.
3. Collectively the pool exercises these `EffectContext` surfaces (see `RUST_ENGINE_API.md`): `gain_memory`, `draw`, `select_*`, `delete_permanent`, `suspend`/`unsuspend`, `add_dp_modifier`, `add_to_hand_from_trash`, `trash_security`, `effect_initiated_digivolve`, token spawn, replacement registration.
4. At least 6 of the 17 launch archetypes are represented.
5. Pool size: 22–28 cards. Fewer than 22 means coverage is too sparse; more than 28 means the doc's worked-example list is too long to maintain.

The pool is regenerated when the DSL vocabulary expands or new launch archetypes are added.

---

## 5. Per-card test pattern

Every per-card test file follows the same shape. **Spotlight example: BT15-003 Nyaromon.** It exercises the most patterns at once (DigiEgg, inherited When Attacking, OPT, optional, top/bottom branch selection, cost-as-trashing, event firing).

### File header

```rust
//! BT15-003 Nyaromon — Digi-Egg, Lv.2, Yellow.
//!
//! # Card text (cards.json)
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//! By trashing the top or bottom card of your security stack, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_003.cs
//!
//! # Patterns this test covers
//! - G4 inherited When Attacking on DigiEgg
//! - E2 OPT + optional decline
//! - F5-adjacent: trash-as-cost firing OnDiscardSecurity / OnLoseSecurity
//! - Selection: Top/Bottom branch via SelectionKind::EffectChoice
```

The file header docstring is mandatory. It records the card text verbatim, the C# reference path, and the pattern rows from §4.3 the test covers. Maintainers reading a failing test should not need to open `cards.json` to understand intent.

### Section 1 — Structural assertions

These assert the *shape* of the compiled card. They are fast, do not run the engine, and catch DSL drift at the source.

```rust
use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::DebugRunner;

fn nyaromon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT15-003")
        .dsl_card("ST1-03")              // top card the egg lives under
        .memory(5)
        .build()
}

#[test]
fn bt15_003_has_one_inherited_when_attacking_clause() {
    let runner = nyaromon();
    let compiled = runner.compiled_card("BT15-003");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "Nyaromon has exactly one triggered clause");
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert_eq!(clause.when, vec![CompiledTiming::WhenAttacking]);
    assert!(clause.optional);
    assert!(clause.once_per_turn);
}
```

What to assert structurally — **only** what the DSL guarantees stays stable across vocabulary expansions:

- Number of triggered clauses, by `scope` (own / inherited / security).
- The `when` vector for each triggered clause.
- `optional`, `once_per_turn`, `max_per_turn` flags.
- For declaratives: the `CompiledDeclarativeClause` variant (Aura / CostReduction / Replacement / Partition / GrantKeyword / AceOverflow).
- For alt-paths: `CompiledAltPathKind` (Digivolve / DnaDigivolve / DigiXros / BurstDigivolve / Assembly / ActivatedDigivolve).

Do **not** assert on `process` step contents — those will change as DSL verbs evolve, and the behavioral tests below cover what the steps actually do.

### Section 2 — Condition gating

For every clause with a `condition`, write one positive and one negative test. A single test asserting both directions is forbidden — split them.

```rust
#[test]
fn bt15_003_condition_blocks_with_no_security() {
    let mut runner = nyaromon();
    runner.game.players[0].security.clear();
    let perm = runner.place_on_field(0, "BT15-003", None);
    runner.fire_on_play(0, perm.index as usize); // structural setup; no effect fires here

    // Drive the When Attacking by faking an attack — but the condition should
    // gate before any selection installs.
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));
    let defender = runner.place_on_field(1, "ST1-03", Some(0));
    runner.attack_digimon(attacker, defender, false);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when security is empty"
    );
}

#[test]
fn bt15_003_condition_passes_with_security() {
    let mut runner = nyaromon();
    let perm = runner.place_on_field(0, "BT15-003", None);
    let attacker = runner.place_on_field(0, "ST1-03", Some(0));
    let defender = runner.place_on_field(1, "ST1-03", Some(0));
    runner.attack_digimon(attacker, defender, false);

    let kind = runner.pending_kind().expect("Top/Bottom prompt must install");
    assert_eq!(kind, SelectionKind::EffectChoice);
}
```

### Section 3 — Behavioral outcome (per branch)

Every observable branch gets its own test. For Nyaromon: trash-top, trash-bottom, decline.

```rust
#[test]
fn bt15_003_trash_top_security_gains_one_memory() {
    let mut runner = nyaromon();
    // ... attack setup as above ...

    let security_before = runner.security_count(0);
    let memory_before = runner.memory();
    let top_card = runner.game.players[0].security[0].handle();

    runner.execute_branch(0); // 0 = "Top"
    runner.auto_resolve();

    assert_eq!(runner.security_count(0), security_before - 1);
    assert_eq!(runner.memory(), memory_before + 1);
    // Top card landed in trash, not bottom.
    assert!(runner.game.players[0].trash.iter().any(|c| c.handle() == top_card));
}

#[test]
fn bt15_003_trash_bottom_security_gains_one_memory() {
    // Mirror of above, but execute_branch(1).
}

#[test]
fn bt15_003_declining_does_nothing() {
    let mut runner = nyaromon();
    // ... attack setup ...

    let security_before = runner.security_count(0);
    let memory_before = runner.memory();
    assert!(runner.pending_is_optional());

    runner.execute_action(action::PASS);

    assert_eq!(runner.security_count(0), security_before);
    assert_eq!(runner.memory(), memory_before);
}
```

### Section 4 — Cost firing (events)

Replaces the Python `_fire_timing` spy. When an effect's cost has side effects on the game (trashing security fires `OnDiscardSecurity` and `OnLoseSecurity`), assert via the event log:

```rust
#[test]
fn bt15_003_trashing_security_fires_discard_and_lose_events() {
    use digimon_engine::events::GameEventKind;

    let mut runner = nyaromon();
    // ... attack setup ...

    let cp = runner.event_checkpoint();
    runner.execute_branch(0);
    runner.auto_resolve();

    let events = runner.events_since(cp);
    let discard_count = events.iter()
        .filter(|e| matches!(e.kind, GameEventKind::OnDiscardSecurity { .. }))
        .count();
    let lose_count = events.iter()
        .filter(|e| matches!(e.kind, GameEventKind::OnLoseSecurity { .. }))
        .count();

    assert!(discard_count >= 1, "OnDiscardSecurity must fire");
    assert!(lose_count >= 1, "OnLoseSecurity must fire");
}
```

This is what stops a regression where a DSL `trash_security` step silently bypasses the events that BT15-038 Angewomon and similar cards subscribe to.

### Section 5 — OPT enforcement

For every `once_per_turn: true` clause, write a test that exercises the lockout.

```rust
#[test]
fn bt15_003_opt_blocks_second_activation_in_same_turn() {
    let mut runner = nyaromon();
    // ... fire once with auto-resolve ...
    runner.execute_branch(0);
    runner.auto_resolve();

    // Second attack same turn → no selection installs.
    let attacker = runner.perm_handle(0, 1);
    let defender2 = runner.place_on_field(1, "ST1-03", Some(0));
    runner.attack_digimon(attacker, defender2, false);
    assert!(runner.pending_selection().is_none(), "OPT must lock the second attempt");

    // After end_turn, the lockout clears.
    runner.end_turn(); // now it's player 1's turn
    runner.end_turn(); // player 0 again
    let defender3 = runner.place_on_field(1, "ST1-03", Some(0));
    runner.attack_digimon(attacker, defender3, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
}
```

---

## 6. Structural assertions on `CompiledCard`

Reference table for what to assert and which DSL field to check.

| Card-text element | DSL field | Assertion |
|---|---|---|
| "[When Attacking]" | `clause.when` contains `CompiledTiming::WhenAttacking` | `clause.when == vec![WhenAttacking]` |
| "[On Play]" | `clause.when` contains `CompiledTiming::OnPlay` | as above |
| "[End of Your Turn]" | `clause.when` contains `CompiledTiming::EndOfYourTurn` | as above |
| "[Once Per Turn]" | `clause.once_per_turn == true` | direct |
| "[Up to N times per turn]" | `clause.max_per_turn == Some(N)` | direct |
| Optional ("you may", "By Xing,…") | `clause.optional == true` | direct |
| Inherited effect | `clause.scope == CompiledScope::Inherited` | direct |
| Security effect | `clause.scope == CompiledScope::Security` | direct |
| Aura "All your X get +N DP" | `CompiledDeclarativeClause::Aura { dp_modifier: Some(N), target, .. }` | match arm + target predicate check |
| Aura grants keyword | `CompiledDeclarativeClause::Aura { grant_keyword: Some(_), .. }` | match arm + keyword equality |
| "Reduce play cost by N" | `CompiledDeclarativeClause::CostReduction { amount: Some(N), .. }` | direct |
| Replacement ("instead") | `CompiledDeclarativeClause::Replacement { trigger, .. }` | trigger string match |
| ACE | `compiled.ace_overflow.is_some()` | direct |
| also_treated_as | `compiled.identity.is_some()` and inspect aliases | match `CompiledNameAlias { treat_as, zone }` |
| "Digivolve cost N" alt-path | `compiled.alt_paths` contains entry with `kind == Digivolve`, `cost: Some(Literal(N))` | iterate alt_paths |
| DNA digivolve | alt-path `kind == DnaDigivolve`, `materials.len() >= 2` | iterate alt_paths |
| DigiXros | alt-path `kind == DigiXros` | iterate alt_paths |
| Mega Digimon Assembly | alt-path `kind == Assembly` | iterate alt_paths |

What **not** to assert structurally:

- Individual `CompiledStep` shapes inside `process`. Those are vocabulary-dependent and will refactor; behavioral tests already cover what they do.
- `summary` / `summary_key` strings. Cosmetic only.
- `active_when` / `condition` predicate trees — use behavioral tests (positive + negative branch) instead.

---

## 7. Behavioral assertions through `EffectContext`

For per-card tests where you want to invoke a process closure manually (e.g. testing a specific clause without setting up the full trigger), use the pattern from `tests/dsl/phase2b_end_to_end.rs`:

```rust
let dsl_effect = DslCardEffect::new(Arc::new(compiled));
let card_handle: CardHandle = runner.game.players[0].hand[0].handle();

let effects = dsl_effect.effects(card_handle);
let on_play = effects.iter().find(|e| e.is_on_play()).expect("OnPlay clause exists");
let process = on_play.process.as_ref().expect("has process closure");

{
    let mut ctx = EffectContext::new(&mut runner.game, card_handle, None, /* player */ 0);
    process(&mut ctx);
}
// Assert pending selection or state delta...
```

This bypasses timing dispatch and the effect queue. Use it for **clause-isolated** tests; use the play/attack/end-turn helpers for **integrated** tests where timing matters. Per-card files should have at least one integrated test per clause — bypassing the queue is for surgical assertions, not the full coverage.

### Asserting state deltas

| State element | Read via |
|---|---|
| Memory | `runner.memory()` |
| Hand size / contents | `runner.hand_size(p)`, `runner.game.players[p as usize].hand` |
| Battle area size | `runner.battle_area_size(p)` |
| Security count | `runner.security_count(p)` |
| Trash size | `runner.trash_size(p)` |
| DP of a permanent | `runner.dp_of(handle)` / `runner.effective_dp(handle)` |
| Modifier presence | `runner.modifiers().sum(handle, ModifierType::X)` or `.has(handle, ModifierType::X)` |

Use `place_on_field` + `fire_on_play` only when you genuinely need to skip the play action (e.g. testing inherited effects on an already-stacked permanent). Otherwise drive the full play action so cost payment, OnPlay timing, and OnPlay-related observers all run.

---

## 8. Selection patterns

Every selection kind has a stable test idiom. The patterns below are exhaustive — anything not listed should be flagged for new-pattern review rather than improvised.

### EffectChoice (top/bottom, choose-one branches)

```rust
let kind = runner.pending_kind().expect("EffectChoice prompt must install");
assert_eq!(kind, SelectionKind::EffectChoice);

let view = runner.pending_selection_view().unwrap();
let labels: Vec<&str> = view.effect_choices.as_ref().unwrap()
    .iter().map(|c| c.label.as_str()).collect();
assert_eq!(labels, vec!["Top", "Bottom"]);

runner.execute_branch(0);   // first label
runner.auto_resolve();
```

### Target / OwnField / OppField

```rust
assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
let view = runner.pending_selection_view().unwrap();
assert_eq!(view.valid_action_ids.len(), 2, "two opponent permanents in scope");

// Drive the first legal action explicitly when the test cares which target.
runner.execute_action(view.valid_action_ids[0]);
runner.auto_resolve();
```

### Hand / Trash / Reveal

```rust
assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
runner.execute_action(/* SEL_TRASH_START + index */);
runner.auto_resolve();
```

Use `digimon_engine::action::space::*` constants (`SEL_TRASH_START`, `SEL_HAND_START`, `SEL_OWN_FIELD_START`, etc.) — never hard-code action IDs.

### CountCappedMultiSelect (reveal N, choose ≤K)

```rust
assert!(matches!(
    runner.pending_kind(),
    Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
));

runner.execute_action(view.valid_action_ids[0]); // first pick
// runner now re-installs the prompt with picked=1
runner.execute_action(view.valid_action_ids[0]); // second pick (indices shift after first)
// player declines further picks
runner.execute_action(action::PASS);
```

### OrderedPermutation (pick N in order)

Drive the picks one at a time; the prompt re-installs with `remaining` decremented.

```rust
while let Some(SelectionKind::OrderedPermutation { remaining }) = runner.pending_kind() {
    if remaining == 0 { break; }
    runner.execute_action(runner.pending_selection_view().unwrap().valid_action_ids[0]);
}
```

### UnionZone (pick from hand OR trash)

```rust
let view = runner.pending_selection_view().unwrap();
match view.kind {
    SelectionKind::UnionZone { zones } => {
        assert!(zones.contains(Zone::Hand));
        assert!(zones.contains(Zone::Trash));
    }
    _ => panic!("expected UnionZone"),
}
```

### Replacement (accept / decline)

Replacement prompts are always optional.

```rust
assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
assert!(runner.pending_is_optional());

// Accept:
runner.execute_action(view.valid_action_ids[0]); // exactly one ACCEPT entry
// or decline:
runner.execute_action(action::PASS);
```

### Material / Source

For DNA / digivolve material picks:

```rust
assert_eq!(runner.pending_kind(), Some(SelectionKind::Material));
// drive each material in order; the engine re-installs the prompt per material
```

### Anti-pattern: silent auto-resolve through a multi-branch prompt

`runner.auto_resolve()` picks the first legal action at every prompt. That makes it a **bad** way to test which branch fires — it always picks the same one. For per-branch tests, drive the specific action explicitly with `execute_branch` or `execute_action`, then call `auto_resolve` only after the branching choice is locked in.

---

## 9. Mechanic-level test patterns

These tests live outside `cards_behavioral/` and use real DSL fixture cards (preferred) or minimal inline YAML (when no real card fits cleanly).

### Combat (`tests/combat/`)

Loads fixture cards into both players' battle areas, drives `attack_digimon` or `attack_player`, asserts on:

- Effective DP at each combat phase (`DuringAttack`, post-modifier).
- Modifier expiry windows (`Expiry::EndOfAttack`, `Expiry::EndOfTurn`).
- Security DP application (Security A. ±N).
- Interrupt ordering (Alliance / Counter / Block).

Spotlight: D1 Conditional DP buff (BT17-015 WarGreymon). Test: with trait-matching ally on field, attack lifts DP by 2000 during the attack window only. Drives the attack, asserts `effective_dp` at `phase == GamePhase::DuringAttack`, then again after `auto_resolve` — the buff must be gone in the second snapshot.

### Effects (`tests/effects/`)

Cross-card effect interactions. Examples:

- Cost-reduction stacking and `BeforePayCost` leak guards (D2). Load BT13-007 King Drasil + BT13-111 Gallantmon, verify neither leaks cost reduction onto unrelated plays.
- DNA digivolve On Deletion (G3). BT18-015 Kimeramon being deleted while its DNA-digivolve trigger is in scope must offer the multi-source material selection.
- Owen Dreadnought security-loss conditioning (F9). BT18-087 / BT24-082's effect window opens specifically when the controller has lost security; test that the condition checks `security_count` of the *controller*, not the opponent.

### Selection (`tests/selection/`)

Selection-state-machine tests that span cards: chained selections (effect_select_opponent_permanent → effect_select_own_permanent), OPT shared hash across copies of BT21-029 Medusamon, the difference between `WhenRemoveField` and `WhenPermanentWouldBeDeleted` (F4) being observable in the order callbacks fire on BT24-030 Neptunemon.

### Replacements (`tests/replacements/`)

Would-replacement plumbing. Each replacement card (F3) gets a test ensuring:
- The replacement registers when the card enters play.
- The replacement triggers on the right event (would-trash / would-be-deleted / would-lose-security).
- The replacement de-registers when the card leaves play.

### Flood gates (`tests/flood_gates/`)

Player-restriction modifiers. Test that BT14-009 Gotsumon's `CANNOT_PLAY_CARD` mask is consulted at action-mask generation, not just at execute time, so the RL action mask correctly hides the masked actions.

### Keywords (`tests/keyword_parsing.rs` + `tests/keywords/`)

`keyword_parsing.rs` already tests printed-keyword extraction. Behavioral keyword tests (one per H-row in §4.3) live in a new `tests/keywords/` dir or, where the keyword is a single card's mechanic (Vortex, ACE), in that card's per-card file with a cross-reference comment.

| Keyword | Test target |
|---|---|
| Rush | Played card can attack same turn (no summoning sickness). |
| Jamming | Suspends defender during attack instead of being deleted by it. |
| Piercing | Excess attack damage flows to security. |
| Security A. ±N | Security check reveals N more / fewer cards. |
| Blocker | Reactive suspend redirects attack to blocker. |
| Reboot | Skip opponent's unsuspend phase for this Digimon. |
| Evade | Once-per-turn unsuspend on attack instead of being deleted. |
| Raid | Switch attack target on attack-declaration. |
| Alliance | Trigger-on-ally-attack +DP / keyword grant. |
| Overclock | Free digivolve during specific window. |
| Blast | Digivolve from breeding/stack at instant speed. |
| ACE | Memory overflow penalty on attack-induced overflow. |
| Barrier | Damage prevention with cost. |
| Scapegoat | Decoy effect — redirects deletion. |

Each keyword test loads its canonical example card from §4.3, exercises the keyword, asserts the expected modifier or state delta.

---

## 10. Inline DSL fixtures

Use `from_dsl_yaml(yaml_str)` instead of `dsl_card(id)` only when:

1. **DSL infra tests** — parser, validator, lowering. Lives in `tests/dsl/`.
2. **Hypothetical-spec tests** — testing that the engine handles a spec shape, not whether a specific card ships with it.
3. **One-field tweak tests** — start from a real card, tweak one field, confirm the engine reacts. Document the diff in the test docstring.

Style guide for inline YAML:

- Keep the YAML ≤30 lines. If it's longer, the test belongs in a per-card file using `dsl_card`.
- Use `make_test_card` for non-effect metadata (level, dp, color) that doesn't matter to the test.
- Card IDs in inline fixtures use `DSL-` prefix (e.g. `DSL-AURA-001`) to make them obvious in test failures.

```rust
let yaml = r#"
card: DSL-AURA-001
name: Test Aura
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
effects:
  - kind: aura
    target: { kind: digimon, owner: you, other: true }
    dp_modifier: 1000
"#;
let runner = DebugRunner::builder()
    .from_dsl_yaml(yaml)
    .hand(0, &["DSL-AURA-001"])
    .memory(5)
    .start();
```

Anti-pattern: pasting a real shipping card's YAML inline. If you want to test BT17-015, use `dsl_card("BT17-015")`. Inline copies drift from the production spec and silently mask failures.

---

## 11. Anti-patterns

A consolidated list. Each anti-pattern fails review.

1. **Don't paste production card YAML inline.** Use `dsl_card(id)`. Inline copies drift; behavioral tests must follow the shipping spec.
2. **Don't reach into `Game` internals when `EffectContext` or runner helpers cover it.** `runner.dp_of(handle)` not `&game.modifiers; sum hand-rolled`. The escape hatch (`runner.game_mut()`) exists but its use is a smell.
3. **Don't write a single test asserting both branches of a conditional.** Split positive and negative — failing one half should fail one test, not the whole pair.
4. **Don't auto-resolve when testing a specific branch.** `auto_resolve` picks the first legal action at every prompt; it cannot tell you which branch fired. Use `execute_branch` / `execute_action`, then `auto_resolve` only after the branching decision is locked.
5. **Don't test DSL syntax in `cards_behavioral/`.** Parser / validator / lowering belongs in `tests/dsl/`. Card behavioral tests assume the DSL works.
6. **Don't use synthetic `TEST-*` cards when a real DSL card from the same set covers the mechanic.** `make_test_card` is for the runner's own self-tests and for one-off engine primitive coverage, not for archetype work.
7. **Don't assert on `process` step contents.** Step vocabulary changes; behavioral assertions cover the outcome.
8. **Don't stub a test with `#[ignore]` for missing features without naming the gap.** Use `#[ignore = "pending: <gap-id> from docs/RUST_ENGINE_GAPS.md"]` or `#[ignore = "pending: <gap-id> from qa/dsl-vocab-gaps.md"]` so the unblock is mechanical.
9. **Don't share fixtures across unrelated tests via mutable global state.** Each test builds its own runner; cross-test contamination is forbidden.
10. **Don't omit the file header docstring.** Card text + C# reference + pattern row tags are mandatory.
11. **Don't use `place_on_field` to skip a real OnPlay path you should be testing.** `place_on_field` skips cost payment, OnPlay observers, and registry-driven triggers. Use it when the test is about *post-play* state (combat, inherited effects on a stacked egg) — not as a shortcut around play resolution.
12. **Don't hard-code action IDs.** Use the `digimon_engine::action::space::*` constants.

---

## 12. TDD walkthrough — implementing a new card from card text

Worked example: implementing **EX11-027 Maquinamon** from scratch (3-way zone choice, WhenRemoveField link selection — patterns C3 and E1).

### Step 1 — Read the card text

> **[On Play]** [Once Per Turn] Choose one: trash 1 of your link cards, trash 1 of your hand cards, or trash 1 of your digivolution cards. Then, this Digimon gets +2000 DP for the turn.
> **[When Removed from the Battle Area]** Choose 1 of your link cards. Move it to your hand.

Identify pattern rows from §4.3: **C3** (link selection on removal), **E1** (3-way branch choice on play), **E2** (OPT), **D1** (DP buff with end-of-turn expiry).

### Step 2 — Write the failing test file

`tests/cards_behavioral/ex11/ex11_027.rs`:

```rust
//! EX11-027 Maquinamon — Lv.6, Yellow.
//!
//! # Card text (cards.json)
//! ... (full text) ...
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Yellow/EX11_027.cs
//!
//! # Patterns
//! - C3 WhenRemoveField link selection
//! - E1 3-way branch choice OnPlay
//! - E2 OPT
//! - D1 conditional DP buff (turn-scoped)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;

fn maquinamon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-027")
        .dsl_card("EX11-006")  // a link-eligible card
        .hand(0, &["EX11-027"])
        .memory(12)
        .start()
}

// --- Structural ---

#[test]
fn ex11_027_has_on_play_and_when_removed_clauses() { /* ... */ }

#[test]
fn ex11_027_on_play_is_once_per_turn() { /* ... */ }

#[test]
fn ex11_027_on_play_offers_three_branches() { /* ... */ }

// --- OnPlay branches ---

#[test]
fn ex11_027_on_play_branch_trash_link() { /* execute_branch(0); assert link card in trash, +2000 DP */ }

#[test]
fn ex11_027_on_play_branch_trash_hand() { /* execute_branch(1) */ }

#[test]
fn ex11_027_on_play_branch_trash_digivolution() { /* execute_branch(2) */ }

#[test]
fn ex11_027_on_play_dp_buff_expires_at_end_of_turn() { /* play, end_turn, assert DP back to base */ }

#[test]
fn ex11_027_opt_blocks_second_play_same_turn() { /* not directly applicable — OnPlay can't repeat; skip with note */ }

// --- WhenRemoveField ---

#[test]
fn ex11_027_when_removed_offers_link_selection() { /* delete the perm; assert OwnField selection over link cards */ }

#[test]
fn ex11_027_when_removed_picked_card_moves_to_hand() { /* execute_action; assert hand += 1, link -= 1 */ }
```

### Step 3 — Author the YAML

`code/digimon-engine/cards/ex11/EX11-027.yaml`:

```yaml
card: EX11-027
name: Maquinamon
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 13000
traits: [ExMaquinamon]
effects:
  - when: on_play
    once_per_turn: true
    process:
      - select_effect_choice:
          bind_as: branch
          labels: ["Trash a link card", "Trash a hand card", "Trash a digivolution card"]
      - if:
          condition: { equals: [branch, 0] }
          then:
            - select_link_card: { of: you, bind_as: target }
            - trash_link_card: { of: you, card: target }
      # ... branches 1 and 2 ...
      - add_dp_modifier:
          target: self
          amount: 2000
          expiry: end_of_turn
  - when: when_removed_from_battle_area
    process:
      - select_link_card: { of: you, bind_as: pick }
      - move_link_card_to_hand: { of: you, card: pick }
```

### Step 4 — Run the tests

```bash
cargo test --test cards_behavioral -- ex11_027
```

Tests fail. Make each pass by:

1. Verifying the YAML compiles (`cargo test --test dsl -- --ignored ex11_027` if a parser test exists).
2. Adding any missing DSL verbs to `code/digimon-dsl/` if `cargo test --test dsl loader` reports unknown step (this is the rare case — the verbs above all exist in Phase 2b).
3. If a verb is missing entirely, route the blocker by layer: engine primitives go in `docs/RUST_ENGINE_GAPS.md`, while DSL verbs, predicates, schema, or lowering gaps go in `qa/dsl-vocab-gaps.md`; then `#[ignore = "pending: <gap-id>"]` the affected test until the gap closes.

### Step 5 — Verify no regression

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Full suite must stay green. `cargo test --test cards_behavioral` runs only the per-card binary.

---

## 13. Appendix: Python → Rust test idiom map

For anyone porting a test from `code/engine_py_legacy/tests/behavioral/btXX/test_btXX_NNN.py`. Listed by what the Python test does, then the Rust equivalent.

| Python | Rust |
|---|---|
| `runner.place_on_field(p, ["BT15-003", "ST1-03"])` | `runner.place_on_field(p, "ST1-03", None)` after `dsl_card(...)` for both cards. The Rust runner places one card at a time; build the stack manually. |
| `inherited_wa.is_optional` | `compiled.effects[i]` matched as `CompiledClause::Triggered(c)` then `c.optional` |
| `inherited_wa.max_count_per_turn == 1` | `c.once_per_turn == true` (or `c.max_per_turn == Some(1)`) |
| `inherited_wa.hash_string` | OPT identity is by `(card_id, clause_index)` in Rust. Cross-copy OPT enforcement is asserted by behavior (G3-style test: two copies of BT21-029 share the lockout). |
| `inherited_wa.timing == EffectTiming.OnUseAttack` | `c.when == vec![CompiledTiming::WhenAttacking]` |
| `inherited_wa.is_inherited_effect` | `c.scope == CompiledScope::Inherited` |
| `effect.on_process_callback({...})` | `let mut ctx = EffectContext::new(...); process(&mut ctx);` (see §7) |
| `runner.auto_resolve()` | `runner.auto_resolve()` (signature spec'd in §3) |
| `runner.execute(SEL_EFFECT_CHOICE_START + 1)` | `runner.execute_branch(1)` or `runner.execute_action(action::SEL_EFFECT_CHOICE_START + 1)` |
| `runner.inject_card(p, "ST1-01", "security_top")` | Push directly: `runner.game.players[p as usize].security.insert(0, ...)`. Helper not yet provided; file under `runner-extensions` if frequent. |
| `game.player1.trash_security_card(card)` | Drive through the engine — call the security-trash code path via an effect, not a direct method. The events fire correctly only when invoked through engine APIs. |
| `_fire_timing` spy | `runner.events_since(checkpoint)` (see §5 Section 4) |

The translation is rarely 1:1. Port what each Python test *proves*, not its asserts. Many Python tests assert against fields that exist only because Python is duck-typed — those don't have Rust analogues, and the equivalent property is checked through the typed `CompiledClause` enum or through behavior.

---

## 14. Tracks A–K test patterns

The Tracks A–K substrate landed several new behaviors that per-card and
mechanic tests now exercise. The patterns below are stable idioms; reach
for them in the corresponding situations.

### 14.1 Replacement effects (Track B / Phase 7)

Cards with `<Barrier>`, `<Evade>`, `<Decode>`, `<Fragment(N)>`, `<Save>`,
`<Decoy>`, `<Partition>`, `<MaterialSave(N)>`, `<ArmorPurge>`, or custom
`WhenWouldBe*` clauses produce a `SelectionKind::Replacement` prompt. The
prompt is **always optional** when `.optional()` is set on the effect
builder — the test must drive both the accept and decline branches.

```rust
use digimon_engine::selection::SelectionKind;
use digimon_engine::action::space::PASS;

// Setup: an attacker about to delete a `<Barrier>`-tagged defender.
let _ = runner.attack_digimon(atk, def, false);

// Replacement prompt parks for the defender's controller.
assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
assert!(runner.pending_is_optional(),
    "Barrier is `(may)` — both accept and decline must surface");
let view = runner.pending_selection_view().unwrap();
assert_eq!(view.valid_action_ids.len(), 1, "exactly one ACCEPT entry");

// Drive the accept branch:
runner.execute_action(view.valid_action_ids[0]).unwrap();
runner.auto_resolve().unwrap();

// In a separate test, drive the decline branch via PASS:
//   runner.execute_action(PASS).unwrap();
```

Per-replacement-kind canonical regression locations:

- `tests/replacements/native_keywords.rs` — Barrier / Evade / Decode auto-installs.
- `tests/replacements/nested_select_fragment.rs`, `nested_select_save.rs`, `nested_select_decoy.rs` — selection-bearing keyword pattern.
- `tests/replacements/partition.rs` — Partition source-pair pick.
- `tests/replacements/passive_modifier_migration.rs` — `CannotBeReturnedToHand` / `CannotBeReturnedToDeck` / `CannotBeDeDigivolved` / `CannotBeTrashedByEffect` automatic mandatory cancels.
- `tests/replacements/source_scoped_immunity.rs` — `cause_filter`-bearing modifier entries.

### 14.2 Aura effects (Track H)

Cards with declarative `kind: aura` install per-permanent modifiers each
controller tick. Tests assert the modifier presence on currently-matching
permanents and absence on currently-non-matching ones.

```rust
// Aura source is on the field; an ally Digimon should be buffed.
let aura = runner.place_on_field(0, "BT11-042", Some(0));   // Angewomon aura source
let ally = runner.place_on_field(0, "ST1-03", Some(0));
runner.game_mut().tick_declarative_effects();

let dp_delta = runner.modifiers().sum(ally, ModifierType::ChangeDp);
assert_eq!(dp_delta, 1000);

// Remove the aura source — the modifier evaporates on next tick.
runner.execute_action(/* delete aura */).unwrap();
runner.game_mut().tick_declarative_effects();
assert_eq!(runner.modifiers().sum(ally, ModifierType::ChangeDp), 0);
```

For formula-backed `dp_modifier_fn` / `security_attack_fn` auras, assert via
`runner.effective_dp(handle)` and security-check counts — formula auras do
not materialize into the modifier registry, they are continuously
recomputed at query time. See `tests/dsl/group6_auras.rs` and
`tests/dsl/group6_dynamic_formulas.rs`.

### 14.3 Granted-triggered effects (Track H)

Cards using `grant_triggered_effect` install a body that fires on every
matching `timing` event until `expiry`. Tests must drive the trigger event
twice to confirm persistence — distinct from `refire_effect` which is
one-shot.

```rust
// EX10-040-style grant: target gains "[End of Your Turn]: gain 1 memory"
runner.play(0, hand_index_of_grantor).unwrap();
runner.auto_resolve().unwrap();
let target = /* ... */;
runner.end_turn();  // first fire
runner.end_turn();  // returns to grantor's turn
let m_before_second = runner.memory();
runner.end_turn();  // second fire
assert_eq!(runner.memory(), m_before_second + 1, "granted body must persist past one fire");
```

### 14.4 OPT triggered with shared hash across copies (Track C / E)

When two copies of the same card share an OPT hash, activating one must
lock the other. Drive both activations in the same turn and assert no
selection installs on the second:

```rust
let copy_a = runner.place_on_field(0, "BT21-029", Some(0));   // Medusamon
let copy_b = runner.place_on_field(0, "BT21-029", Some(0));
// Fire copy_a's [On Play] / [Main] trigger and resolve.
// ... drive the selection ...

// Now try copy_b in the same turn — OPT must lock it.
runner.fire_on_play(0, copy_b.index as usize);
assert!(runner.pending_selection().is_none(),
    "shared OPT hash blocks copy_b after copy_a fired");
```

### 14.5 Breeding-area observer (`BreedingPermanent`)

`StartOfYourMainPhase`, `OnHatch`, and several Tamer turn-boundary timings
scan the breeding area as a distinct trigger source. Tests that exercise
breeding observers must place a permanent in breeding via
`runner.place_in_breeding(player, card_id)` and assert the observer fires:

```rust
runner.place_in_breeding(0, "BT15-003");          // egg under breeding
runner.end_turn();
runner.end_turn();                                 // back to player 0; Main phase enters
runner.auto_resolve().unwrap();
// Assert the egg's StartOfYourMainPhase observer fired …
```

`SelectionKind::BreedingPermanent` is the prompt installed by
`select_own_breeding_permanent` — used by cards that target the egg
permanent. The mask for this kind reuses the own-field range plus the
breeding sentinel.

### 14.6 RevealBucket flows

Cards with two-pass reveal text (e.g. BT18-060 Vemmon, BT15-077 LadyDevimon)
install `SelectionKind::RevealBucket { bucket_index, min, max, picked }`.
Each bucket parks its own selection; tests drive each bucket independently
and assert `bucket_index` advances:

```rust
runner.play(0, 0).unwrap();   // reveal-top-N effect
let view1 = runner.pending_selection_view().unwrap();
assert!(matches!(view1.kind,
    SelectionKind::RevealBucket { bucket_index: 0, .. }));
runner.execute_action(view1.valid_action_ids[0]).unwrap();
// runner re-installs the prompt with picked += 1 …

// Once bucket 0 is satisfied (picked >= min), PASS commits and bucket 1 opens:
runner.execute_action(PASS).unwrap();
let view2 = runner.pending_selection_view().unwrap();
assert!(matches!(view2.kind,
    SelectionKind::RevealBucket { bucket_index: 1, .. }));
```

See `tests/selection/reveal_buckets.rs` for the canonical multi-bucket
template.

### 14.7 DpBudget selection

Cards with text like "delete opponent's Digimon with a total of N DP or less"
install `SelectionKind::DpBudget { remaining_dp, picked }`. Each pick
decrements `remaining_dp` by the picked permanent's effective DP; PASS
commits the accumulated set. The mask masks out any opponent permanent
whose DP exceeds the remaining budget.

```rust
let view = runner.pending_selection_view().unwrap();
let SelectionKind::DpBudget { remaining_dp, picked } = view.kind else {
    panic!("expected DpBudget");
};
assert_eq!(picked, 0);
assert!(remaining_dp >= 5000);
```

See `tests/selection/dp_budget.rs`.

### 14.8 Action-mask assertions for new SelectionKind variants

When a test cares about *which* action IDs are exposed (not just that a
selection installed), use `runner.pending_selection_view().valid_action_ids`
and the `digimon_engine::action::space::*` constants:

```rust
use digimon_engine::action::space::{SEL_HAND_START, SEL_TRASH_START, PASS};

let view = runner.pending_selection_view().unwrap();
// Hand picks present, trash picks excluded:
assert!(view.valid_action_ids.iter().any(|&a| a >= SEL_HAND_START && a < SEL_TRASH_START));
assert!(!view.valid_action_ids.iter().any(|&a| a >= SEL_TRASH_START));
// PASS gated by is_optional:
assert_eq!(view.valid_action_ids.contains(&PASS), runner.pending_is_optional());
```

### 14.9 Event-log assertions (Track A)

Track A established the `TriggerContext` payload contract: deletion observers
read `deleted_object`, attack-target-change observers read
`attack_target_change`, source-trash observers read `event_host_*`. Tests
verifying these payloads use the event log:

```rust
let cp = runner.event_checkpoint();
runner.attack_digimon(atk, def, false);
runner.auto_resolve().unwrap();

let events = runner.events_since(cp);
let deletion = events.iter().find(|e| matches!(e.kind, GameEventKind::OnAnyDeletion { .. }));
assert!(deletion.is_some(), "OnAnyDeletion must fire after battle");
```

Track-A-shaped tests live across `tests/effects/`, `tests/replacements/`,
and `tests/combat/deletion_cause_observer.rs`. Use `events_of_kind` when you
want all events of a specific discriminant since a checkpoint.

### 14.10 Worked examples directory

Curated DSL fixtures used as docs and infra examples live in
`code/digimon-engine/cards/_examples/`. As of 2026-05-15:

| Card ID | Demonstrates |
|---|---|
| `BT15-003.yaml` | Inherited When Attacking + OPT + EffectChoice (top/bottom) |
| `BT17-007.yaml` | Conditional DP buff + trait gate |
| `BT17-015.yaml` | Two-way branch + EndOfAttack DP buff |
| `BT18-019.yaml`, `BT18-102.yaml` | Track-A event-payload-bound observers |
| `BT22-084.yaml` | Tamer play-4 anchor (start-of-main) |
| `BT11-042.yaml` | Declarative aura with target predicate |
| `BT5-093.yaml`, `BT7-107.yaml`, `BT9-092.yaml` | Search / trash-to-hand recursion |
| `BT12-112.yaml` | DigiXros source play |
| `BT13-007.yaml`, `BT13-060.yaml` | Cost-reduction patterns |
| `BT20-083.yaml` | also-treated-as alias |
| `EX6-072.yaml` | Mega Digimon Assembly |
| `EX11-027.yaml` | 3-way branch + WhenRemoveField link selection |
| `AD1-025.yaml`, `BT10-111.yaml`, `ST2-13.yaml` | Generic patterns |
| `TST_DNA_TRIGGER.yaml` | DNA digivolve trigger payload |

Tests under `tests/dsl/real_cards_json.rs` and `tests/dsl/parse_*.rs` exercise
each example through the embedded pack loader. When adding a new pattern,
prefer adding a worked example here over writing inline YAML in a test file.

---

## 15. Cross-references

- `RUST_ENGINE_API.md` — `EffectContext`, `Effect`, `CardEffect`, modifier types, timing enums.
- `2026-04-21-card-scripting-dsl.md` — DSL spec, vocabulary, compile pipeline.
- `RUST_PYTHON_PARITY.md` — known cross-engine divergences.
- `qa/archetype-qa/INDEX.md` — 17 launch archetypes with verdict tables.
- `RUST_ENGINE_GAPS.md` — open Rust engine primitive gaps (cards depending on these stay `#[ignore]`).
- `qa/dsl-vocab-gaps.md` — open DSL vocabulary and lowering gaps.
- `qa/dsl-test-pool.md` — example card pool (regenerated per §4 research task).
