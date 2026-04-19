# Rust Engine API Reference

**Audience:** AI agents (and humans) implementing Digimon card effects in Rust against `digimon-engine`.

This document is the canonical scripting reference. Before writing any card effect, read this in full. The engine intentionally exposes a curated API (`EffectContext`); do not reach around it into `Game` internals.

---

## 1. Project layout

```
digimon-engine/
├── src/
│   ├── lib.rs                  # Re-exports the public API
│   ├── game.rs                 # Game state + turn machine
│   ├── player.rs, permanent.rs, card_source.rs   # Zones / permanents / card instances
│   ├── card_data.rs            # Static card metadata loaded from cards.json
│   ├── rules.rs                # Rules presets (standard, edh, titan_boss, titan_team)
│   ├── enums.rs                # Phase, Timing, Keyword, Modifier, Expiry, Zone, Color
│   ├── effect.rs               # Effect + EffectBuilder + CardEffect trait
│   ├── effect_context.rs       # EffectContext — THE card-scripting API
│   ├── modifiers.rs            # ModifierRegistry (typed modifiers + expiry)
│   ├── combat.rs               # Attack / battle / security
│   ├── tensor.rs               # Observation tensor (1375 floats, matches Python)
│   ├── action/                 # Action space + mask (2168 actions, matches Python)
│   ├── cards.rs                # CardEffectRegistry + registration glue
│   ├── cards/test_cards.rs     # TEST-001..005 — hand-written examples
│   └── debug_runner.rs         # Deterministic test harness
└── tests/                      # Integration tests (engine_core, tensor_and_mask,
                                # test_cards_behavioral, combat_scenarios)
```

Each card's effect script lives in `src/cards/<set>/<card_id>.rs`, implements
the `CardEffect` trait, and registers itself via the set's `register` fn.

---

## 2. Writing a card effect

Every card's effects live in a struct that implements `CardEffect`:

```rust
use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};

/// BT1-010: "On Play: Gain 2 memory. When Digivolving: Draw 1."
pub struct Bt1010;

impl CardEffect for Bt1010 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card)
                .name("Gain 2 memory")
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("Draw 1")
                .process(|ctx| {
                    let me = ctx.player;
                    ctx.draw(me, 1);
                })
                .build(),
        ]
    }
}

pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("BT1-010", Arc::new(Bt1010));
}
```

Key rules:

- One struct per `card_id`. The struct is zero-sized (`pub struct Foo;`).
- `effects()` returns **all** of that card's effects — OnPlay, WhenDigivolving, inherited, security, etc.
- Closures in `process(...)` and `condition(...)` are `Fn + Send + Sync + 'static`. They capture `Copy` handles (`CardHandle`, `PermanentHandle`, `PlayerId`) — never borrow.
- Register in the set's `register(&mut CardEffectRegistry)` function.

### Effect builder constructors

| Constructor | Timing flag set | When it fires |
|-------------|-----------------|---------------|
| `Effect::on_play(card)` | `on_play` | When played from hand |
| `Effect::when_digivolving(card)` | `when_digivolving` | Played on top of another permanent |
| `Effect::on_attack(card)` | `on_attack` | When the permanent declares an attack |
| `Effect::on_deletion(card)` | `on_deletion` | Before the permanent is deleted |
| `Effect::inherited(card)` | `inherited` | Always — carried up the stack to the top card |
| `Effect::security(card)` | `security` | Revealed in the opponent's security check |
| `Effect::declarative(card)` | `declarative` | Passive / always-on |

### Builder methods

- `.name("human label")` — used for logs and UI.
- `.condition(|ctx| bool)` — gate the effect. If false, `process` is skipped.
- `.process(|ctx| { ... })` — the mutation body.
- `.optional()` — player may decline (UI prompt).
- `.once_per_turn()` — `max_per_turn = 1`.
- `.timing(EffectTiming::...)` — override the timing enum (rare).
- `.dp_modifier(n)` — static DP buff (for declarative effects).
- `.cost_reduction(n)` — static cost reduction.
- `.build()` — finalize into `Effect`.

---

## 3. `EffectContext` API

The context passed into `condition` and `process` closures.

### Fields

- `ctx.game: &mut Game` — escape hatch (use sparingly).
- `ctx.source_card: CardHandle` — the card whose effect is resolving.
- `ctx.source_permanent: Option<PermanentHandle>` — the permanent containing that card, if any.
- `ctx.player: PlayerId` — the controller of the source. Use this, not hardcoded player IDs.

### Read-only queries

```rust
ctx.memory() -> i16
ctx.turn_count() -> u16
ctx.rules() -> &Rules
ctx.card_data() -> &[CardData]

ctx.player(id) -> &Player        // any player
ctx.my_player() -> &Player        // ctx.player's player
ctx.opponent_id() -> PlayerId     // first clockwise opponent
ctx.opponent() -> &Player
ctx.opponents() -> Vec<PlayerId>  // all non-eliminated opponents

ctx.battle_area(id) -> &[Permanent]
ctx.hand(id) -> &[CardSource]
ctx.trash(id) -> &[CardSource]
ctx.security_count(id) -> usize

ctx.source_permanent() -> Option<&Permanent>
```

### Memory

```rust
ctx.gain_memory(amount: i16)
ctx.lose_memory(amount: i16)
ctx.set_memory(value: i16)
```

Memory is the seesaw — positive favors the active player, negative crosses into the opponent's turn. Use `gain_memory(n)` and `lose_memory(n)`; the engine clamps to `rules.memory_range`.

### Card flow

```rust
ctx.draw(player: PlayerId, count: u8) -> u8        // returns cards actually drawn
ctx.trash_from_top(player: PlayerId, count: u8) -> u8
```

### Field mutations

```rust
ctx.delete_permanent(target: PermanentHandle)
ctx.suspend(target: PermanentHandle)
ctx.unsuspend(target: PermanentHandle)
```

`delete_permanent` removes the permanent and moves all cards in its stack to trash. It also clears modifiers attached to that handle. **Does not fire OnDeletion** — use `Game::delete_permanent_with_effects` for that when you're calling from combat paths. From a card script, `ctx.delete_permanent` is usually what you want (OnDeletion is handled by combat, not effect).

### Modifiers

```rust
ctx.add_dp_modifier(target: PermanentHandle, value: i32, expiry: Expiry)
ctx.add_modifier(target, modifier: ModifierType, value: i32, expiry: Expiry)
ctx.grant_keyword(target, keyword: Keyword, expiry: Expiry)
```

See §5 for `ModifierType` and `Expiry` values.

---

## 4. Handles

- `CardHandle(u16)` — identifies a specific card instance by its unique `card_index`. Copy. Used in effect closures so they can be captured cheaply.
- `PermanentHandle { player, index }` — the battle-area slot holding a permanent. Copy. **Not stable across deletions** — if something earlier in the battle area is deleted, indices shift. If you care about identity across arbitrary game state changes, snapshot the card handle instead.

When a card effect needs to refer to the permanent it came from:

```rust
.process(|ctx| {
    if let Some(me) = ctx.source_permanent {
        ctx.add_dp_modifier(me, 1000, Expiry::EndOfTurn);
    }
})
```

---

## 5. Key enums

### `EffectTiming`

```
OnPlay, WhenDigivolving, OnAttack, OnDeletion, WhenAttacking, OnBlock,
SecurityEffect, CounterEffect,
StartOfYourTurn, StartOfOpponentsTurn, EndOfYourTurn, EndOfOpponentsTurn, EndOfAttack,
OnAllyAttack, OnOpponentAttack, OnDrawCard, OnTrash, OnReturn, OnSuspend, OnUnsuspend,
OnAddToHand, OnReveal, OnPlaceSecurity,
OnEnterField, OnEnterFieldAnyone, OnLeaveField,
BeforePayCost, WhenPlayedFromHand,
OnDigivolve, OnDnaDigivolve, OnDigiXros,
AlwaysActive, Declarative,
OptionMain, OptionSecurity,
None
```

### `Expiry`

```
Permanent             # never expires on its own
EndOfTurn             # cleared at the end of any turn
EndOfOpponentsTurn    # cleared at the end of the source-player's opponent's turn
EndOfAttack           # cleared when the current attack resolves
EndOfBattle           # same as EndOfAttack for most purposes
UntilLeaveField       # cleared when the permanent leaves the field
```

### `ModifierType` (partial — see `enums.rs` for full list)

- DP: `ChangeDp`, `ChangeBaseDp`, `DpFloor`, `DontHaveDp`
- Cost: `ChangePlayCost`, `ChangeDigivolveCost`, `CannotReduceCost`
- Protection: `CannotBeDestroyed`, `CannotBeDestroyedByBattle`, `CannotBeDestroyedByEffect`
- Attack: `CannotAttack`, `CannotAttackPlayer`, `CanAttackUnsuspended`, `CanAttackActivePlayer`
- Suspend: `CannotSuspend`, `CannotUnsuspend`
- Targeting: `CannotBeSelectedByEffect`, `CannotBeAffected`
- Granted keywords: `GrantBlocker`, `GrantRush`, `GrantJamming`, `GrantPiercing`, `GrantReboot`, `GrantBlitz`, `GrantAlliance`, `GrantRaid`, `GrantBarrier`, `GrantArmor`, `GrantDecoy`
- Security: `SecurityAttackChange`
- Color/level: `ChangeColor`, `AddColor`, `ChangeLevel`

### `Keyword`

```
Blocker, SecurityAttackPlus(i8), SecurityAttackMinus(i8),
Rush, Jamming, Piercing, Reboot, DeDigivolve(u8), DrawX(u8),
Blitz, Armor, Raid, Alliance, Blast, Save, Fortitude, Overclock,
Barrier, Decoy, Material, Partition
```

---

## 6. Common patterns

### Gain memory on play

```rust
Effect::on_play(card)
    .name("Gain 1 memory")
    .process(|ctx| ctx.gain_memory(1))
    .build()
```

### Conditional draw

```rust
Effect::on_play(card)
    .name("Draw 2 if opponent has 3+ Digimon")
    .condition(|ctx| {
        let opp = ctx.opponent_id();
        ctx.battle_area(opp).iter()
            .filter(|p| p.is_digimon(ctx.card_data()))
            .count() >= 3
    })
    .process(|ctx| {
        let me = ctx.player;
        ctx.draw(me, 2);
    })
    .build()
```

### Buff all your Digimon for the turn

```rust
Effect::on_play(card)
    .name("All your Digimon +1000 DP")
    .process(|ctx| {
        let me = ctx.player;
        let count = ctx.battle_area(me).len();
        for i in 0..count {
            let h = PermanentHandle { player: me, index: i as u8 };
            ctx.add_dp_modifier(h, 1000, Expiry::EndOfTurn);
        }
    })
    .build()
```

### Grant a keyword

```rust
Effect::on_play(card)
    .name("Grant Rush to self")
    .process(|ctx| {
        if let Some(me) = ctx.source_permanent {
            ctx.grant_keyword(me, Keyword::Rush, Expiry::EndOfTurn);
        }
    })
    .build()
```

### Security effect — trigger when this card is revealed in security

```rust
Effect::security(card)
    .name("Deal 3000 DP to attacker")
    .process(|ctx| {
        // Engine wires `ctx.source_permanent` to the attacker when firing
        // SecurityEffect timings (forthcoming — see engine-gaps.md).
    })
    .build()
```

---

## 7. Anti-patterns (don't do this)

### ❌ Holding a reference to Player across `ctx.player_mut` calls

```rust
// Wrong — player borrow dies when we call ctx.draw
let p = ctx.player(ctx.player);
let count = p.hand.len();   // OK
ctx.draw(ctx.player, 1);    // ERROR: p still borrowed
```

Fix: read the immutable data into locals first, then mutate.

```rust
let count = ctx.my_player().hand.len();
let me = ctx.player;
ctx.draw(me, 1);
```

### ❌ Stashing `PermanentHandle` across deletions

```rust
// Wrong — after deleting handle 0, handle 2 now points to a different permanent.
let h0 = PermanentHandle { player: me, index: 0 };
let h2 = PermanentHandle { player: me, index: 2 };
ctx.delete_permanent(h0);
ctx.delete_permanent(h2);   // likely wrong target!
```

Fix: delete from highest index to lowest, or collect stable identifiers (card_index) first.

### ❌ Panicking on a missing card

```rust
let id = ctx.player(0).hand[0].card_id(ctx.card_data()); // panic if empty
```

Fix: bounds-check before indexing.

### ❌ Capturing non-Copy state in closures

```rust
// Wrong — Vec<String> isn't Copy, and the closure might outlive it
let names: Vec<String> = collect_some_names();
Effect::on_play(card)
    .process(move |ctx| {
        for n in &names {  // needs 'static
            ...
        }
    })
```

Fix: move the collection into the closure by value (already correct with `move`), or derive everything from `ctx.card_data()` inside the closure.

---

## 8. Testing

Use `DebugRunner` for behavioral tests.

```rust
use digimon_engine::debug_runner::{DebugRunner, make_test_card};

#[test]
fn my_card_gains_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BT1-010", "Example"))
        .hand(0, &["BT1-010"])
        .start();
    let m_before = r.memory();
    r.play(0, 0);
    assert_eq!(r.memory(), m_before + 2);
}
```

Builder methods:
- `.add_card(CardData)` — register a card.
- `.with_card_data(HashMap<String, CardData>)` — bulk load.
- `.hand(player, &["ID1", "ID2"])` — deterministic hand (no shuffle).
- `.deck(player, &[...])` — deterministic deck (last element = top of deck).
- `.security(player, &[...])` — explicit security stack.
- `.digitama(player, &[...])` — explicit digitama deck.
- `.with_rules(Rules)` — override rules (default standard).
- `.with_registry(CardEffectRegistry)` — inject a custom registry.
- `.start()` — builds and advances past Mulligan into turn 1.
- `.build()` — builds without starting (remains in Mulligan).

Runner methods:
- `.play(player, hand_index)` — `Game::play_from_hand` + OnPlay.
- `.place_on_field(player, "ID", turn_played_override)` — bypasses play for combat setup. Pass `Some(0)` to avoid summoning sickness.
- `.attack_digimon(attacker, defender)` / `.attack_player(attacker, defender_player)`
- `.end_turn()` / `.pass_turn()`
- `.memory()`, `.hand_size(p)`, `.battle_area_size(p)`, `.deck_size(p)`, `.trash_size(p)`, `.security_count(p)`
- `.effective_dp(handle)`, `.dp_of(handle)`
- `.game_over()`, `.winner()`

---

## 9. Known gaps (as of Phase 4)

These are documented in `qa/archetype-qa/engine-gaps.md`. Notable items:

- **Block / Counter / Alliance interrupt phases** not yet wired — combat is atomic.
- **OnSecurityCheck** / **OnStartBattle** / **OnEndBattle** / **OnEndAttack** timings are not yet fired by the combat module.
- **Rush** from a card's innate keyword list — only modifier-granted Rush currently exempts summoning sickness. Native Rush needs the effect-listing query.
- **Security effects** don't have a fully wired timing that exposes the attacker to the script.
- **BeforePayCost** for cost reduction scanning the entire battle area is not implemented.
- **Option cards** have no play flow yet (they hit the field as a permanent like Digimon).

When implementing a card that needs one of these, log the gap and pick a safe fallback.

For a comprehensive Rust ↔ Python divergence catalog with severity and fix order,
see [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md).

---

## 10. Registering a card

1. Create `digimon-engine/src/cards/<set>/<card_id>.rs` implementing `CardEffect`.
2. Create/update `digimon-engine/src/cards/<set>/mod.rs` with a `register` function that calls `registry.insert` for every card in the set.
3. Add `pub mod <set>;` to `digimon-engine/src/cards.rs`.
4. Call `<set>::register(&mut registry)` inside `cards::build_registry()`.

A card is **not** active until it appears in `build_registry()`.

---

## 11. The two registries & what happens when a new set drops

There are **two separate registries**. Don't confuse them.

### `CardRegistry` — card_id ↔ integer index (for the RL tensor)

Defined in [card_registry.rs](../digimon-engine/src/card_registry.rs). Built with `CardRegistry::from_cards(&HashMap<String, CardData>)`. Provides:

- `get_index(card_id) -> u16` — integer for tensor encoding. `0` = padding/unknown.
- `get_norm_id(card_id) -> f32` — normalized float for non-embedding tensor slots.
- `get_id(u16) -> Option<&str>` — reverse lookup.

**Parity rule:** when cards.json contains an explicit `index` field on each entry (the production format), the Rust registry uses that value verbatim. This is what Python's `CardRegistry.initialize()` does, and it **must** match — pretrained embeddings, serialized replays, and ONNX models all key off these indices.

When cards.json entries omit `index` (legacy arrays, inline test fixtures), the Rust registry falls back to **alphabetically sorted, 1-based** assignment.

Duplicate indices panic at construction. Missing indices in otherwise-production data are silently skipped (treated as unknown).

### `CardEffectRegistry` — card_id → `Arc<dyn CardEffect>` (for effect scripts)

Defined in [cards.rs](../digimon-engine/src/cards.rs). Populated at compile time by `build_registry()`, which calls each set's `register()` function.

A missing entry here means the card plays as **vanilla** — no effect, no error.

### When a new set drops (e.g. BT25, 100 new cards)

1. **cards.json gets updated** by the card pipeline. New cards are appended with fresh `index` values (likely 4083..4182). **Existing indices never change.** The Rust `CardRegistry` will pick up the new mappings automatically on next load.

2. **New effect scripts** go into `digimon-engine/src/cards/bt25/*.rs`. Add `pub mod bt25;` to `cards.rs` and `bt25::register(&mut registry)` to `build_registry()`. Cargo rebuild.

3. **Cards with no script yet** play as vanilla — they're in the `CardRegistry` (so they have tensor positions) but not in the `CardEffectRegistry` (so no effects fire). Perfectly fine; the engine silently no-ops.

4. **Cross-engine parity** is preserved: Python's `CardRegistry.initialize()` reads the same `index` field. Python and Rust feed identical tensor positions to the model.

### What would break parity

- Hand-editing cards.json `index` values after the fact → trained models become garbage.
- Relying on the alphabetical-sort fallback in production (don't — always use explicit indices).
- Two different cards.json files between Python and Rust environments.

The test `tests/card_registry_parity.rs` loads the real cards.json and asserts the Rust mapping matches `CardData.index` for every card. Keep that green.

---

## 12. Setup and mulligan

The setup sequence in `Game::new` + `start_game` matches Python's mulligan flow. If you're writing card effects you almost never touch this — it's relevant mainly when writing tests that want to exercise opening-hand decisions, or when building a UI / RL loop that surfaces the mulligan choice.

### Sequence

1. **`Game::new(decks, cards, rules, seed)`** — builds players, shuffles each deck and digitama via the seeded rng, then:
   - Shuffles `turn_order` (first-player coin flip; deterministic under the same seed).
   - Draws `rules.starting_hand` cards for every player.
   - Initializes `mulligan_pending` (clone of `turn_order`) and `mulligan_used` (all false).
   - Leaves the game in `GamePhase::Mulligan`. **Security is not yet laid.**

2. **Each decider answers.** Either the caller walks through `accept_mulligan`, or a convenience caller invokes `start_game` to auto-keep.

3. **`finalize_mulligan` runs automatically** once the last decider answers. It lays `rules.security_count` security cards per player, sets `turn_count = 1`, `memory = 0`, and calls `begin_turn`.

### API surface

```rust
// Read-only
game.mulligan_current_player() -> Option<PlayerId>
game.mulligan_pending          // Vec<PlayerId> — FIFO of remaining deciders
game.mulligan_used             // Vec<bool> indexed by player id

// Mutating
game.accept_mulligan(player, keep: bool) -> Result<(), &'static str>
//   keep == true  : hand stays as-is, advance to next decider
//   keep == false : hand → deck, reshuffle with game.rng, draw starting_hand,
//                   set mulligan_used[player] = true, advance

// Shorthand
game.start_game()  // auto-keep every pending player, then finalize
```

`accept_mulligan` returns `Err` if:
- The caller passes a player who isn't the current decider (the message contains "different player").
- Mulligan is already complete (message: "already complete").

### Action mask

During `GamePhase::Mulligan`:
- Only the current decider's mask has any bits set. Every other perspective is all zeros.
- Bit 0 is **keep** — always available.
- Bit 1 is **mulligan** — suppressed once `mulligan_used[current] == true`.

### DebugRunner

The common test path bypasses mulligan: `DebugRunner::builder().start()` runs `start_game` which auto-keeps, and `build_inner` clears `mulligan_pending` so the test's explicit zones aren't cannibalized by `setup_security`.

Mulligan-specific tests should construct `Game::new` directly with a real deck (so redraws have cards) and optionally wrap with `DebugRunner::wrap(game)` for the convenience helpers:

```rust
let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();
let first = game.mulligan_current_player().unwrap();
game.accept_mulligan(first, /* keep */ false).unwrap(); // redraw
game.start_game(); // auto-keep for the rest
```

Or via DebugRunner:
```rust
let mut r = DebugRunner::wrap(game);
r.mulligan_decide(false)?;     // redraw current decider
r.skip_mulligan();              // auto-keep everyone else
```

### Tauri / UI

The DTO returned from every `rust_*` command carries `mulligan_current_player: Option<PlayerId>` and `mulligan_used: Vec<bool>`. The frontend hides the Mulligan button for any player whose `mulligan_used` is true, and only enables keep/mulligan controls for the player whose id matches `mulligan_current_player`.

`rustMulliganDecide(keep: boolean)` from `frontend/src/api/rustEngine.ts` applies the decision for the current decider and returns the updated state.

### What's NOT here (yet)

- **Rule-driven mulligan variants** (e.g. redraw fewer cards, double mulligan). Digimon TCG has a single, full-size redraw only.
- **Tensor slot for mulligan context.** The current tensor has no dedicated "who's deciding" slot; if RL needs to learn mulligan policy, we'd extend the selection-context section of the tensor. Not required for current training loops.

---

## 13. OPT (once-per-turn) tracking and tensor DP contributions

Card scripts rarely touch this directly, but two things matter when you write an effect with `max_per_turn > 0` or a static `dp_modifier`:

### Once-per-turn activation counters

`Permanent::effect_activations: HashMap<(CardHandle, u8), u8>` tracks how many times each `(source_card, effect_slot_index)` has fired this turn. The slot index is the effect's position in the `Vec<Effect>` returned by `CardEffect::effects(handle)` — so order your effects intentionally and keep them stable across edits, or the counters key into the wrong slot.

Counters reset in `Permanent::new_turn` (called from `Player::new_turn` inside `Game::begin_turn`). You don't need to call any reset manually.

To gate your own effect firing:
```rust
.condition(|ctx| {
    let Some(h) = ctx.source_permanent else { return false; };
    let perm = &ctx.game.players[h.player as usize].battle_area[h.index as usize];
    perm.activation_count(ctx.source_card, /* my slot */ 0) == 0
})
.process(|ctx| {
    // ... do the work ...
    if let Some(h) = ctx.source_permanent {
        let perm = &mut ctx.game.players[h.player as usize].battle_area[h.index as usize];
        perm.record_activation(ctx.source_card, 0);
    }
})
```

(A `ctx`-level `record_activation` helper would be a nice follow-up; not yet wired.)

### Static `dp_modifier` and the tensor

Use `Effect::declarative(card).dp_modifier(n)` for a non-inherited static buff (e.g. "This Digimon gains +1000 DP"), or `Effect::inherited(card).dp_modifier(n)` for an inherited version (carries up the stack). Add a `.condition(...)` if the buff only applies in certain situations — the condition is evaluated at tensor-build time too, so `"[Your Turn] +3000"` contributes 0 on the opponent's turn.

Avoid encoding DP-change effects via `ctx.add_dp_modifier(...)` for *static* buffs — that writes to `ModifierRegistry`, which the tensor's per-source contributions don't currently sum (permanent-level, yes; per-source, no). Stick with `dp_modifier` on `Effect` for anything you want the tensor to see per source.

### How the tensor reads these

`build_tensor` calls four `Game` helpers per permanent slot:

```rust
game.opt_total(handle)            -> u32        // slot offset +3
game.opt_used(handle)             -> u32        // slot offset +4
game.source_dp_contribution(h, i) -> i32 / f32  // per-source offset +2
game.source_opt_state(h, i)       -> f32        // per-source offset +1
```

Each one iterates effects via `CardEffectRegistry::get(card_id).effects(handle)`, applies the inherited/top filter (`is_under == effect.inherited`), and evaluates conditions through `EffectReadContext`. You can call them yourself in tests or diagnostics.

---

## Writing a card effect (TDD walkthrough)

This is the onramp the forthcoming Rust `batch-fix-cards` skill will hand to sub-agents. Authors (human or AI) follow it directly until the skill exists. The flow mirrors the Python `/batch-fix-cards` convention: decompose → test-first → implement → verdict.

Worked example pattern: `digimon-engine/src/cards/test_cards.rs` (the `TEST-001..TEST-022` structs) with paired tests in `digimon-engine/tests/test_cards_behavioral.rs`. Read both side-by-side before starting a real card.

### 1. Decompose the card text into numbered clauses

Copy the official card text verbatim. Split on each independent clause and number them. Example for a hypothetical card:

```
[On Play]
  (1) Trash the top card of your deck.
  (2) If that card was a Lv.5 Digimon, gain 1 memory.
```

Each clause becomes a discrete assertion in the test. The numbering stays in comments so the implementation's `process` closure mirrors it.

### 2. Write failing behavioral tests first

Create or extend a test file under `digimon-engine/tests/`. Use `DebugRunner::builder()` to construct a minimal game state — inject only what the clause exercises. One `#[test]` per clause outcome, including both positive and negative branches.

```rust
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn clause1_trashes_top_of_deck() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("YOUR-001", "Example"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["YOUR-001"])
        .deck(0, &["FILLER"])
        .memory(5)
        .start();

    r.play(0, 0);

    assert_eq!(r.deck_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn clause2_gains_memory_when_trashed_card_is_lv5() {
    // … inject a Lv.5 card at the top of the deck, assert +1 memory …
}

#[test]
fn clause2_no_memory_when_trashed_card_is_not_lv5() {
    // … negative branch …
}
```

`DebugRunner` setup helpers (`.hand`, `.deck`, `.memory`, `.add_card`, `.start`) are the canonical surface — don't reach into `Game` directly from tests.

### 3. Run the tests and confirm they fail

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test your_card_behavioral
```

Expect compile failures (no `CardEffect` impl yet) or assertion failures. A passing test at this point means the test doesn't actually exercise the clause — rewrite it until it fails for the right reason.

### 4. Implement the `CardEffect`

Add a zero-sized struct in `digimon-engine/src/cards/` (under a set-scoped submodule for real cards — e.g. `src/cards/bt16/bt16_052.rs`). Implement `CardEffect::effects` using the `Effect` builder. Use `EffectContext` for every mutation — never reach into `Game` internals from a `process` closure.

```rust
use std::sync::Arc;
use crate::card_source::CardHandle;
use crate::cards::CardEffectRegistry;
use crate::effect::{CardEffect, Effect};

pub struct YourCard001;

impl CardEffect for YourCard001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Trash top, gain 1 if Lv.5")
            .process(|ctx| {
                // (1) Trash the top card of your deck.
                let me = ctx.player;
                let trashed = ctx.trash_from_top(me, 1);
                // (2) If that card was a Lv.5 Digimon, gain 1 memory.
                if trashed.first().is_some_and(|c| c.level() == Some(5) && c.is_digimon()) {
                    ctx.gain_memory(1);
                }
            })
            .build()]
    }
}
```

Numbered comments inside the closure match the clause decomposition from Step 1.

### 5. Register the effect

Wire the card into the registry. Real cards register from their set module; test cards register from `test_cards::register`. Follow the existing pattern in `digimon-engine/src/cards.rs` and `digimon-engine/src/cards/test_cards.rs`.

```rust
// digimon-engine/src/cards/bt16/mod.rs
pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("BT16-001", Arc::new(bt16_001::BT16_001));
    // …
}
```

### 6. Run the tests and confirm they pass

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test your_card_behavioral
```

All clause tests green. Then run the full suite to catch regressions:

```bash
cargo test --manifest-path digimon-engine/Cargo.toml
```

### 7. Emit a verdict

Use the same verdict vocabulary as the Python `/batch-fix-cards` skill so the eventual Rust skill inherits it cleanly:

- **IMPLEMENTED** — every clause has a passing test and a faithful implementation.
- **PARTIAL** — some clauses landed; the rest are blocked on a specific engine gap. Document which clauses and why.
- **BLOCKED** — the card requires engine infrastructure that doesn't yet exist (new `EffectTiming` variant, modifier type, selection kind, etc.). Do not ship a stub. File the gap in `qa/archetype-qa/engine-gaps.md` and move on.

### No-approximations checklist (Rust)

Before claiming IMPLEMENTED, re-read the card text against the implementation and confirm:

1. No clause is silently dropped.
2. Every player choice uses `ctx.select_*` — never auto-select a target when the card text allows multiple.
3. Optional effects (`(Optional)`, "you may") are modeled with the `optional` builder flag + a declined branch in the test.
4. Memory cost is paid through the standard play/digivolve path, not re-implemented inside `process`.
5. Static keywords (Blitz, Rush, Piercing, Blocker, …) on the face of the card are handled via `grant_keyword` or the appropriate `Keyword` query, not hard-coded booleans.
6. Inherited effects use `Effect::inherited(card)`, not `Effect::on_play(card)` with a manual under-the-stack check.
7. Trait / name matching uses `CardSource::contains_card_name` / trait accessors (case-insensitive), not raw string equality.
8. Every closure is `Send + Sync + 'static` — if you have lifetime errors, you're capturing a borrow; use handles (`Copy`) instead.

---

## 14. Zone Manipulation (Phase 2)

Added in Phase 2 to support card-script movement between zones (hand, deck, trash, battle area, digivolution stack, security, reveal pool) and effect-initiated plays. All methods live on `EffectContext` and delegate to `Game`-level helpers. Card-moving methods return `Option<PermanentHandle>` or `Option<CardHandle>` for provenance — follow-up effects thread the handle into the next primitive.

### Shared types

```rust
pub enum CostDelta {
    Free,               // pay 0
    Reduce(i16),        // max(0, printed - n); negative n increases cost
    Fixed(i16),         // max(0, n); replaces printed cost
}

pub enum StackPosition { Top, Bottom, Random }

pub enum CardSourceRef {
    Hand(PlayerId, usize),
    Trash(PlayerId, usize),
    DeckTop(PlayerId),
    Reveal(CardHandle),
}
```

### Play from zone

| Method | Purpose |
|--------|---------|
| `play_from_hand_with_cost(player, hand_index, CostDelta) -> Option<PermanentHandle>` | Play from hand at a computed cost. `CostDelta::Free` bypasses printed cost. |
| `play_from_trash_with_cost(player, trash_index, CostDelta) -> Option<PermanentHandle>` | Play from trash. Same cost-delta contract. |

Example — free play from hand inside an OnPlay effect:

```rust
Effect::on_play(card).process(|ctx| {
    ctx.play_from_hand_with_cost(ctx.player, 0, CostDelta::Free);
}).build()
```

### Card movement

| Method | Purpose |
|--------|---------|
| `add_to_hand_from_deck(player, CardHandle)` → `bool` | Move a specific deck card to hand. Does NOT shuffle. |
| `add_to_hand_from_trash(player, CardHandle)` → `bool` | Same, from trash. |
| `add_to_hand_from_reveal(player, CardHandle)` → `bool` | Same, from the reveal pool. |
| `trash_from_hand_by_index(player, hand_index)` → `Option<CardHandle>` | Trash a specific hand slot. |
| `trash_from_reveal(player, CardHandle)` → `bool` | Trash a revealed card. |
| `return_to_hand(PermanentHandle)` → `Option<CardHandle>` | Bounce a permanent: top → hand, sources under → trash. |
| `return_to_deck(PermanentHandle, StackPosition)` → `bool` | Bounce to deck at Top/Bottom/Random. |
| `return_to_deck_from_reveal(player, CardHandle, StackPosition)` → `bool` | Reveal pool → deck. |
| `shuffle_deck(player)` | Pair with `add_to_hand_from_deck` for "search and shuffle" effects. |

### Reveal pool

`reveal_top_deck(player, n) -> Vec<CardHandle>` — move up to N cards from deck top into the transient reveal pool (`game.revealed_cards`, cleared on turn rotation).

`revealed() -> &[CardSource]` — read-only snapshot of the pool. Scripts inspect it to decide follow-up moves.

### Placement

| Method | Purpose |
|--------|---------|
| `place_as_bottom_source(CardSourceRef, target: PermanentHandle)` → `bool` | Insert a card at the bottom of target's digivolution stack. |
| `place_on_security(player, CardSourceRef, StackPosition, face_up: bool)` → `bool` | Move to security stack at Top/Bottom/Random; optionally face-up. |
| `hatch(player) -> bool` | Move top of digitama deck to breeding area. Returns false if breeding is occupied or digitama deck is empty. |
| `effect_initiated_digivolve(player, hand_index, target, CostDelta, ignore_color)` → `bool` | Script-driven digivolve. Validates level match; optionally bypasses color check. Fires WhenDigivolving. |

### No-approximations note

Each of these primitives is a pure movement or cost-payment operation. *Which* card to move is always the caller's responsibility, and the choice must surface through a `PendingSelection` built with `select_hand`, `select_trash`, `select_reveal`, or `select_own_permanent`. Never let a script auto-pick a target without a selection — the RL action space must observe the branch.
