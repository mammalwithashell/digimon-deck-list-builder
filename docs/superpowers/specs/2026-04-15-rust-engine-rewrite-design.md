# Rust Engine Rewrite Design

**Date:** 2026-04-15
**Status:** Draft
**Goal:** Rewrite the Digimon TCG game engine in Rust for training speed, correctness, and desktop-native play. Tensor/action-space-first design. Card effects written test-first by AI agents.

---

## 1. Motivation

The Python engine works but is too slow for RL training at scale. The existing card script corpus (~1000+ transpiled/generated scripts) is largely unfaithful — only ~300 cards have proper behavioral tests. Rather than fixing Python scripts, we're starting fresh with a Rust engine that:

- Compiles natively into the Tauri desktop app (no Python sidecar for gameplay)
- Exposes the same observation tensor (1375 floats) and action space (2168 actions) as the Python engine, so trained models transfer without retraining
- Provides a curated `EffectContext` API that AI agents can target reliably
- Later adds PyO3 bindings for Python RL training (drop-in replacement for `HeadlessGame`)

## 2. Multiplayer-Ready Foundation

The engine is designed for standard 2-player Digimon TCG but bakes in multiplayer awareness from the start to support EDH Commander (4-player singleton) and Titan Mode (1v2-3 asymmetric) without a rewrite. See `docs/EDH_COMMANDER_MODE.md` and `docs/TITAN_MODE.md` for full format specs.

**What we build now (low-cost foundations):**
- `Rules` struct — configurable game parameters (player count, deck size, security count, singleton, commander, memory range, max turns). Factory methods: `Rules::standard()`, `Rules::edh()`, `Rules::titan()`.
- `Vec<Player>` — not `player1`/`player2`. Indexed by `PlayerId`. Standard creates 2, EDH creates 4.
- Turn rotation as `Vec<PlayerId>` — supports clockwise rotation and skipping eliminated players.
- Memory seesaw as `(PlayerId, PlayerId)` pair — the active player and the "next" player. Standard: always `(1, 2)`. EDH: clockwise pair shifts each turn.
- `Player.commander_zone: Option<CardSource>` — always `None` in standard.
- `Player.is_eliminated: bool` — always `false` in standard.
- `EffectContext.opponents()` returns `&[PlayerId]` — standard returns `[other]`, EDH returns the other 3. `opponent()` is sugar for `opponents()[0]` (clockwise-next).
- Tensor/action space sizes derived from `Rules` — standard layout (1375/2168) is default, EDH layout (~1876/~2360) activates with `Rules::edh()`.

**What we defer:**
- Commander zone interactions (tax, replacement effects)
- EDH-specific tensor/action encoding
- Elimination cascading
- Titan-specific balancing

## 3. Architecture

### 3.1 Core Invariant

**The engine's primary interface is `(observation_tensor, action_mask, step)`.** Both RL agents and the Tauri UI consume the game through the same action space. The UI JSON view is a secondary serialization of the same state.

```
                    +------------------+
                    |   Game State     |
                    +--------+---------+
                             |
              +--------------+-------------+
              v              v             v
        f32[1375]       u8[2168]        JSON
        observation     action_mask     (UI state)
              |              |             |
              v              v             v
         RL Agent       Both use       Tauri
         (PyO3,        same action    Frontend
          later)       encoding
```

The frontend renders from JSON but **submits actions as integer action IDs** from the same 2168-space. The action mask drives which UI controls are enabled.

### 3.2 Crate Structure

```
digimon-engine/                 # Standalone library crate (no Tauri dependency)
  Cargo.toml
  src/
    lib.rs
    game.rs                     # Game struct, turn state machine, phase flow
    player.rs                   # Player zones: hand, deck, trash, security, field
    permanent.rs                # Digivolution stack, DP calc, keyword queries
    card_source.rs              # Card instance (links to CardData)
    card_data.rs                # Static card metadata (from cards.json)
    card_registry.rs            # card_id <-> integer index (shared with tensor)
    effect.rs                   # CardEffect trait, Effect struct, EffectTiming enum
    effect_context.rs           # EffectContext — the curated script API
    modifiers.rs                # ModifierRegistry, ModifierType, ModifierEntry
    combat.rs                   # Attack resolution, security checks, battle
    phases.rs                   # GamePhase enum, transitions
    action/
      mod.rs
      space.rs                  # ACTION_SPACE_SIZE=2168, range constants
      mask.rs                   # build_action_mask(game, player_id) -> [u8; 2168]
      decoder.rs                # decode_action(game, action_id, player_id) -> mutates game
    tensor.rs                   # build_tensor(game, player_id) -> [f32; 1375]
    runner.rs                   # HeadlessRunner: step/mask/tensor API
    state_json.rs               # Game state -> serde_json::Value for UI
    cards/                      # Card effect implementations
      mod.rs                    # Registry: card_id -> Box<dyn CardEffect>
      bt16/
        mod.rs
        bt16_001.rs
      ...
    enums.rs                    # CardKind, CardColor, GamePhase, EffectTiming, Keyword
  tests/
    helpers/
      mod.rs
      debug_runner.rs           # DebugRunner test harness
    behavioral/                 # Per-card behavioral tests
      bt16/
        mod.rs
        bt16_052.rs

src-tauri/                      # Tauri app (depends on digimon-engine)
  Cargo.toml                    # Workspace member, depends on digimon-engine
  src/
    main.rs                     # Tauri app entry
    commands.rs                 # Tauri invoke() handlers (thin wrappers)
    state.rs                    # Managed game state (Arc<Mutex<HeadlessRunner>>)

digimon-engine-py/              # PyO3 bindings (Phase 9, later)
  Cargo.toml
  src/lib.rs                    # RustHeadlessGame Python class
```

### 3.3 Dependency Graph

```
digimon-engine          (pure Rust, no framework deps)
  |
  +-- src-tauri         (Tauri app, depends on digimon-engine)
  |
  +-- digimon-engine-py (PyO3 bindings, depends on digimon-engine — Phase 9)
```

`digimon-engine` has zero Tauri/UI/Python dependencies. It depends on:
- `serde` + `serde_json` (card data loading, UI JSON)
- `rand` (shuffling, random effects)
- No async runtime (pure synchronous)

## 4. Core Types

### 4.1 Rules

```rust
pub struct Rules {
    pub player_count: u8,           // 2 (standard), 4 (EDH), 3-4 (Titan)
    pub deck_size: u16,             // 50 (standard), 70 (EDH), 80 (Titan boss)
    pub security_count: u8,         // 5 (standard), 7 (EDH), 15 (Titan boss)
    pub starting_hand: u8,          // 5 (standard), 7 (Titan boss)
    pub field_slots: u8,            // 14
    pub singleton: bool,            // false (standard), true (EDH)
    pub commander: bool,            // false (standard), true (EDH)
    pub memory_range: (i16, i16),   // (-10, 10)
    pub max_turns: u16,             // 200 (standard), 600 (EDH)
    pub skip_first_draw: SkipDraw,  // P1Only (standard), AllRound1 (EDH)
}

impl Rules {
    pub fn standard() -> Self;
    pub fn edh() -> Self;
    pub fn titan_boss() -> Self;
    pub fn titan_team() -> Self;
}
```

### 4.2 Game

```rust
pub struct Game {
    pub rules: Rules,
    pub players: Vec<Player>,       // indexed by PlayerId (0-based internally)
    pub turn_count: u16,
    pub current_phase: GamePhase,
    pub memory: i16,                // seesaw between active pair
    pub memory_pair: (PlayerId, PlayerId),  // (active, next) — who the seesaw is between
    pub turn_order: Vec<PlayerId>,  // rotation order (eliminated players removed)
    pub turn_player_idx: usize,     // index into turn_order
    pub modifiers: ModifierRegistry,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pending_selection: Option<SelectionRequest>,
    effect_queue: Vec<QueuedEffect>,
}

impl Game {
    pub fn turn_player(&self) -> PlayerId;
    pub fn player(&self, id: PlayerId) -> &Player;
    pub fn player_mut(&self, id: PlayerId) -> &mut Player;
    pub fn opponents(&self, id: PlayerId) -> Vec<PlayerId>;  // all non-eliminated opponents
    pub fn next_clockwise(&self, id: PlayerId) -> PlayerId;  // next in turn order
}
```

### 4.3 Player

```rust
pub struct Player {
    pub id: PlayerId,
    pub hand: Vec<CardSource>,
    pub deck: Vec<CardSource>,
    pub digitama_deck: Vec<CardSource>,
    pub security: Vec<CardSource>,
    pub trash: Vec<CardSource>,
    pub battle_area: Vec<Permanent>,    // up to rules.field_slots
    pub breeding_area: Option<Permanent>,
    pub commander_zone: Option<CardSource>,  // None in standard, populated in EDH
    pub commander_tax: u16,                 // 0 in standard, increments by 2 in EDH
    pub is_eliminated: bool,                // false in standard, used for EDH/Titan elimination
}
```

### 4.4 Permanent

```rust
pub struct Permanent {
    pub card_sources: Vec<CardSource>,  // digivolution stack [base, evo1, evo2, ...]
    pub linked_cards: Vec<CardSource>,  // sideways-attached options
    pub is_suspended: bool,
    pub turn_played: u16,
    pub turn_digivolved: u16,
    pub attacks_this_turn: u8,
}

impl Permanent {
    pub fn top_card(&self) -> &CardSource;
    pub fn level(&self) -> Option<u8>;
    pub fn dp(&self, modifiers: &ModifierRegistry) -> Option<i32>;
    pub fn is_digimon(&self) -> bool;
    pub fn is_tamer(&self) -> bool;
    pub fn has_keyword(&self, kw: Keyword, modifiers: &ModifierRegistry) -> bool;
    pub fn digivolution_cards(&self) -> &[CardSource];
    pub fn contains_card_name(&self, name: &str) -> bool;
}
```

### 4.5 CardSource

```rust
pub struct CardSource {
    pub data: &'static CardData,    // static metadata from cards.json
    pub owner: u8,                  // player 1 or 2
    pub card_index: u16,            // unique instance index
    pub is_token: bool,
    pub also_treated_as: Vec<String>,
}

impl CardSource {
    pub fn card_id(&self) -> &str;
    pub fn card_names(&self) -> Vec<&str>;  // base + also_treated_as
    pub fn card_colors(&self) -> &[CardColor];
    pub fn level(&self) -> Option<u8>;
    pub fn play_cost(&self) -> u16;
    pub fn dp(&self) -> Option<i32>;
    pub fn is_digimon(&self) -> bool;
    pub fn is_tamer(&self) -> bool;
    pub fn is_option(&self) -> bool;
    pub fn traits(&self) -> &[String];
    pub fn contains_card_name(&self, name: &str) -> bool;
}
```

### 4.6 CardData (static, loaded from cards.json)

```rust
pub struct CardData {
    pub card_id: String,
    pub card_name: String,
    pub card_kind: CardKind,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub play_cost: u16,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,        // Form + Attribute + Type
    pub evo_costs: Vec<EvoCost>,
    pub dna_costs: Vec<DnaCost>,
    pub digixros_costs: Vec<DigiXrosCost>,
    pub effect_text: String,
    pub inherited_text: String,
    pub security_text: String,
    pub is_ace: bool,
    pub ace_overflow_cost: u16,
}
```

## 5. Effect System

### 5.1 CardEffect Trait

```rust
pub trait CardEffect: Send + Sync {
    fn effects(&self, card: CardHandle, game: &GameView) -> Vec<Effect>;
}
```

One struct per card implements this trait. `GameView` provides read-only state access for building condition closures.

**Handle types** are lightweight, `Copy` index-based references into game state (not borrows). This lets closures capture them freely without lifetime issues:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardHandle(pub u16);       // index into a flat card pool

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PermanentHandle(pub u8);   // index into player's battle_area

pub type PlayerId = u8;               // 1 or 2
```

Handles are resolved to references via `game.card(handle)` or `game.permanent(player, handle)`. Invalid handles (deleted permanent, trashed card) return `None`.

### 5.2 Effect Struct

```rust
pub struct Effect {
    pub timing: EffectTiming,
    pub name: String,

    // Flags
    pub optional: bool,
    pub on_play: bool,
    pub when_digivolving: bool,
    pub on_attack: bool,
    pub on_deletion: bool,
    pub inherited: bool,
    pub security: bool,
    pub counter: bool,
    pub declarative: bool,
    pub max_per_turn: u8,           // 0 = unlimited

    // Behavior
    pub can_use: Option<ConditionFn>,
    pub can_activate: Option<ConditionFn>,
    pub process: Option<ProcessFn>,

    // Modifiers (declarative effects)
    pub dp_modifier: i32,
    pub cost_reduction: i32,
}

type ConditionFn = Box<dyn Fn(&EffectContext) -> bool + Send + Sync>;
type ProcessFn = Box<dyn Fn(&mut EffectContext) + Send + Sync>;
```

Builder pattern for ergonomic construction:

```rust
impl Effect {
    pub fn on_play(card: CardHandle) -> EffectBuilder;
    pub fn when_digivolving(card: CardHandle) -> EffectBuilder;
    pub fn on_attack(card: CardHandle) -> EffectBuilder;
    pub fn on_deletion(card: CardHandle) -> EffectBuilder;
    pub fn inherited(card: CardHandle) -> EffectBuilder;
    pub fn security(card: CardHandle) -> EffectBuilder;
    pub fn declarative(card: CardHandle) -> EffectBuilder;
}

impl EffectBuilder {
    pub fn optional(self) -> Self;
    pub fn once_per_turn(self) -> Self;
    pub fn timing(self, t: EffectTiming) -> Self;
    pub fn condition(self, f: impl Fn(&EffectContext) -> bool + Send + Sync + 'static) -> Self;
    pub fn process(self, f: impl Fn(&mut EffectContext) + Send + Sync + 'static) -> Self;
    pub fn build(self) -> Effect;
}
```

### 5.3 EffectContext — The Script API

This is the curated API surface that card scripts and AI agents target. It wraps `&mut Game` but exposes only safe operations.

```rust
pub struct EffectContext<'a> {
    game: &'a mut Game,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub player: PlayerId,
}

impl<'a> EffectContext<'a> {
    // --- Queries (read-only) ---
    pub fn memory(&self) -> i16;
    pub fn turn_count(&self) -> u16;
    pub fn rules(&self) -> &Rules;
    pub fn player(&self, id: PlayerId) -> &Player;
    pub fn my_player(&self) -> &Player;
    pub fn opponent(&self) -> &Player;            // clockwise-next opponent (sugar for opponents()[0])
    pub fn opponents(&self) -> &[PlayerId];       // all non-eliminated opponents (1 in standard, 3 in EDH)
    pub fn battle_area(&self, id: PlayerId) -> &[Permanent];
    pub fn hand(&self, id: PlayerId) -> &[CardSource];
    pub fn trash(&self, id: PlayerId) -> &[CardSource];
    pub fn security_count(&self, id: PlayerId) -> usize;
    pub fn source_permanent(&self) -> Option<&Permanent>;

    // --- Selection requests (pauses game for player choice) ---
    pub fn select_field(&mut self, filter: impl Fn(&Permanent) -> bool,
                        optional: bool, prompt: &str);
    pub fn select_opponent_field(&mut self, filter: impl Fn(&Permanent) -> bool,
                                  optional: bool, prompt: &str);
    pub fn select_own_field(&mut self, filter: impl Fn(&Permanent) -> bool,
                             optional: bool, prompt: &str);
    pub fn select_hand(&mut self, filter: impl Fn(&CardSource) -> bool,
                       optional: bool, prompt: &str);
    pub fn select_trash(&mut self, filter: impl Fn(&CardSource) -> bool,
                        optional: bool, prompt: &str);
    pub fn select_from_revealed(&mut self, cards: &[CardSource],
                                 filter: impl Fn(&CardSource) -> bool,
                                 optional: bool, prompt: &str);

    // --- Actions (mutate game) ---
    pub fn draw(&mut self, player: PlayerId, count: u8);
    pub fn trash_from_top(&mut self, player: PlayerId, count: u8);
    pub fn trash_card(&mut self, card: CardHandle);
    pub fn gain_memory(&mut self, amount: i16);
    pub fn lose_memory(&mut self, amount: i16);
    pub fn set_memory(&mut self, amount: i16);
    pub fn delete_permanent(&mut self, target: PermanentHandle);
    pub fn return_to_hand(&mut self, target: PermanentHandle);
    pub fn return_to_bottom_deck(&mut self, target: PermanentHandle);
    pub fn return_to_top_deck(&mut self, target: PermanentHandle);
    pub fn suspend(&mut self, target: PermanentHandle);
    pub fn unsuspend(&mut self, target: PermanentHandle);
    pub fn play_from_hand(&mut self, card: CardHandle, cost: Option<u16>);
    pub fn play_from_trash(&mut self, card: CardHandle, cost: Option<u16>);
    pub fn digivolve(&mut self, base: PermanentHandle, card: CardHandle, cost: Option<u16>);
    pub fn play_token(&mut self, name: &str, level: u8, colors: &[CardColor],
                      traits: &[&str], dp: i32);

    // --- Modifiers ---
    pub fn register_modifier(&mut self, target: PermanentHandle,
                              modifier: ModifierType, value: i32,
                              expiry: Expiry);
    pub fn register_conditional_modifier(&mut self, target: PermanentHandle,
                                          modifier: ModifierType, value: i32,
                                          condition: impl Fn(&Permanent) -> bool,
                                          expiry: Expiry);
    pub fn grant_keyword(&mut self, target: PermanentHandle,
                          keyword: Keyword, expiry: Expiry);

    // --- Reveal/search ---
    pub fn reveal_from_deck(&mut self, player: PlayerId, count: u8,
                             filter: impl Fn(&CardSource) -> bool);
    pub fn search_deck(&mut self, player: PlayerId,
                        filter: impl Fn(&CardSource) -> bool,
                        count: u8, optional: bool);

    // --- Grant effects ---
    pub fn grant_piercing(&mut self, target: PermanentHandle, expiry: Expiry);
    pub fn grant_security_attack_plus(&mut self, target: PermanentHandle,
                                       amount: i8, expiry: Expiry);
    pub fn grant_blocker(&mut self, target: PermanentHandle, expiry: Expiry);
    pub fn grant_rush(&mut self, target: PermanentHandle, expiry: Expiry);
    pub fn grant_reboot(&mut self, target: PermanentHandle, expiry: Expiry);
}
```

### 5.4 Selection / Interrupt Model

Same pattern as the Python engine — selections are state machine pauses:

1. Effect calls `ctx.select_opponent_field(filter, optional, prompt)`
2. Game sets `pending_selection` with valid indices and returns control to the caller
3. Next `step()` call from UI/agent resolves the selection via action ID
4. Callback fires with the chosen index, game continues

This maps naturally to both Tauri commands (user picks a target) and RL actions (agent picks from mask).

### 5.5 Card Effect Registration

```rust
// cards/mod.rs
pub fn register_all(registry: &mut CardEffectRegistry) {
    // Each set module registers its cards
    bt16::register(registry);
    bt17::register(registry);
    // ...
}

// cards/bt16/mod.rs
pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("BT16-001", Box::new(bt16_001::BT16_001));
    registry.insert("BT16-052", Box::new(bt16_052::BT16_052));
    // ...
}
```

Cards without registered effects use a default no-op implementation (vanilla stats only).

## 6. Runner API

The `HeadlessRunner` is the primary interface for both RL training and Tauri commands.

```rust
pub struct HeadlessRunner {
    game: Game,
}

impl HeadlessRunner {
    /// Create a new game. `decks` length must match `rules.player_count`.
    pub fn new(decks: Vec<Vec<String>>, rules: Rules) -> Self;

    // Primary interface — same for RL and UI
    pub fn step(&mut self, action_id: u16);
    pub fn action_mask(&self) -> Vec<u8>;           // rules-derived size (2168 standard, ~2360 EDH)
    pub fn observation(&self, player_id: u8) -> Vec<f32>;  // rules-derived size (1375 standard, ~1876 EDH)

    // Game state queries
    pub fn is_game_over(&self) -> bool;
    pub fn winner(&self) -> Option<u8>;
    pub fn current_player(&self) -> u8;
    pub fn current_phase(&self) -> GamePhase;
    pub fn rules(&self) -> &Rules;

    // UI-specific (secondary view)
    pub fn to_ui_json(&self, player_id: u8) -> serde_json::Value;
}
```

## 7. Action Space & Tensor (Exact Parity)

### 7.1 Action Space

Standard mode uses the same encoding as the Python engine — `ACTION_SPACE_SIZE = 2168`. The action space size is derived from `Rules` (EDH expands attack targeting for 3 opponents → ~2360 actions). Mask/decoder functions take `&Rules` to determine valid ranges.

| Range | Action | Count |
|-------|--------|-------|
| 0-29 | Play card from hand (by hand index) | 30 |
| 30-59 | [Hand][Main] effects | 30 |
| 60 | Hatch | 1 |
| 61 | Move from breeding | 1 |
| 62 | Pass / end turn / decline | 1 |
| 63-92 | DNA Digivolve | 30 |
| 100-399 | Attack (attacker*15 + target, 14=security) | 300 |
| 400-999 | Digivolve (hand*15 + field, 14=breeding) | 600 |
| 1000-1149 | Activate field effect (perm*10 + effectIdx) | 150 |
| 1150-1194 | [Trash][Main] effects | 45 |
| 2000-2167 | Source selection (field*12 + sourceIdx) | 168 |

### 7.2 Observation Tensor

Standard mode layout — `TENSOR_SIZE = 1375`, float32. EDH mode expands to ~1876 (4 player perspectives). Tensor builder takes `&Rules` to determine layout size and structure.

| Offset | Content | Size |
|--------|---------|------|
| 0-9 | Global (turn, phase, memory) | 10 |
| 10-569 | My battle area (14 slots x 40) | 560 |
| 570-1129 | Opponent battle area (14 x 40) | 560 |
| 1130-1149 | My hand (20 card IDs) | 20 |
| 1150-1169 | Opponent hand (20 card IDs) | 20 |
| 1170-1214 | My trash (45 card IDs) | 45 |
| 1215-1259 | Opponent trash (45 card IDs) | 45 |
| 1260-1269 | My security (10 card IDs) | 10 |
| 1270-1279 | Opponent security (10 card IDs) | 10 |
| 1280-1319 | My breeding (1 x 40) | 40 |
| 1320-1359 | Opponent breeding (1 x 40) | 40 |
| 1360-1369 | Revealed cards (10 card IDs) | 10 |
| 1370-1374 | Selection context (5 floats) | 5 |

Per-slot layout (SLOT_SIZE = 40):
- +0: Top card registry ID
- +1: Current DP (normalized by 30000.0)
- +2: Suspended (0/1)
- +3: OPT total
- +4: OPT used
- +5: Linked card count
- +6: Source count
- +7..39: Source entries (11 x 3: card_id, opt_state, dp_contribution)

Card IDs in the tensor are **CardRegistry integer indices**, same mapping as the Python engine. The `cards.json` file and `CardRegistry` are shared across both engines.

### 7.3 Parity Testing

Before any card effects are written, the engine must pass parity tests:
- Given identical game state, Rust and Python produce identical tensors
- Given identical game state, Rust and Python produce identical action masks
- Action ID N in both engines produces the same state mutation

This is validated by a cross-engine test harness that replays recorded action sequences from the Python engine through the Rust engine and compares outputs.

## 8. Testing

### 8.1 DebugRunner (Test Harness)

```rust
pub struct DebugRunner {
    runner: HeadlessRunner,
}

impl DebugRunner {
    pub fn new() -> Self;
    pub fn with_memory(initial: i16) -> Self;

    // Setup
    pub fn inject_card(&mut self, player: u8, card_id: &str, zone: Zone);
    pub fn inject_permanent(&mut self, player: u8, card_id: &str) -> PermanentHandle;
    pub fn inject_digivolution(&mut self, perm: PermanentHandle, card_id: &str);
    pub fn set_phase(&mut self, phase: GamePhase);
    pub fn set_memory(&mut self, memory: i16);

    // Find actions from mask
    pub fn find_action(&self, description: &str) -> Option<u16>;
    pub fn find_attack(&self, attacker: usize, target: usize) -> Option<u16>;
    pub fn find_digivolve(&self, hand: usize, field: usize) -> Option<u16>;
    pub fn valid_actions(&self) -> Vec<u16>;

    // Execute
    pub fn execute(&mut self, action_id: u16);
    pub fn auto_resolve(&mut self);

    // Snapshot
    pub fn snapshot(&self) -> Snapshot;
}

pub struct Snapshot {
    pub hand_size: [usize; 2],
    pub field_count: [usize; 2],
    pub trash_count: [usize; 2],
    pub security_count: [usize; 2],
    pub memory: i16,
    pub field: [Vec<PermanentSnapshot>; 2],
    pub phase: GamePhase,
    pub game_over: bool,
    pub winner: Option<u8>,
}
```

### 8.2 Example Behavioral Test

```rust
#[cfg(test)]
mod tests {
    use digimon_engine::test_helpers::DebugRunner;

    #[test]
    fn on_play_gain_memory_when_opponent_has_lv5() {
        let mut r = DebugRunner::with_memory(3);
        r.inject_card(1, "BT16-052", Zone::Hand);
        r.inject_permanent(2, "BT16-088"); // Lv5 opponent
        r.set_phase(GamePhase::Main);

        let action = r.find_action("Play Agumon").unwrap();
        r.execute(action);
        r.auto_resolve();

        let snap = r.snapshot();
        assert_eq!(snap.memory, 3 - 3 + 1); // cost 3, gain 1
    }

    #[test]
    fn on_play_no_gain_without_lv5() {
        let mut r = DebugRunner::with_memory(3);
        r.inject_card(1, "BT16-052", Zone::Hand);
        r.inject_permanent(2, "BT16-003"); // Lv3 opponent
        r.set_phase(GamePhase::Main);

        let action = r.find_action("Play Agumon").unwrap();
        r.execute(action);
        r.auto_resolve();

        let snap = r.snapshot();
        assert_eq!(snap.memory, 3 - 3); // cost 3, no gain
    }
}
```

### 8.3 Test Categories

1. **Engine unit tests** — game mechanics, phase transitions, memory, combat
2. **Action space tests** — mask correctness, decoder round-trips
3. **Tensor tests** — layout correctness, normalization, parity with Python
4. **Behavioral tests** — per-card effect tests via DebugRunner (written by AI agents)
5. **Parity tests** — replay Python game recordings through Rust, compare step-by-step

## 9. Tauri Integration

### 9.1 Commands

```rust
#[tauri::command]
fn new_game(decks: Vec<Vec<String>>, rules: Option<Rules>,  // None = Rules::standard()
            state: State<GameManager>) -> GameId;

#[tauri::command]
fn step(game_id: GameId, action_id: u16, state: State<GameManager>) -> StepResult;

#[tauri::command]
fn get_action_mask(game_id: GameId, state: State<GameManager>) -> Vec<u8>;

#[tauri::command]
fn get_state(game_id: GameId, player_id: u8, state: State<GameManager>) -> serde_json::Value;

#[tauri::command]
fn get_valid_decks() -> Vec<DeckInfo>;
```

### 9.2 StepResult

```rust
pub struct StepResult {
    pub state: serde_json::Value,   // full UI game state
    pub action_mask: Vec<u8>,       // updated legal moves
    pub game_over: bool,
    pub winner: Option<u8>,
    pub events: Vec<GameEvent>,     // animations, sounds
}
```

The frontend receives everything it needs in a single response per action — no extra round-trips.

### 9.3 Frontend Changes

The React frontend currently calls the Python FastAPI backend via REST/WebSocket. For the Tauri desktop build:

- Replace `fetch()` / WebSocket calls with `invoke()` Tauri commands
- Action submission: `invoke("step", { gameId, actionId })` instead of `POST /games/{id}/action`
- State updates: received synchronously from `step()` result (no WebSocket push needed)
- Action mask: received in `StepResult`, used to enable/disable UI controls
- The hosted API path (FastAPI + WebSocket) remains for future online play

## 10. Batch-Implement-Rust Skill

Adapts the `batch-fix-cards` pattern for Rust card effects.

### 10.1 Orchestration Flow

Same as Python `batch-fix-cards`:
1. Resolve archetype card pool via `resolve_deck`
2. Batch into groups of 4 (cross-referenced cards together)
3. Present plan, wait for user approval
4. Pre-read shared context: Rust engine API reference + engine gaps
5. Per batch: dispatch 4 parallel Opus agents (worktree isolation)
6. Review agent validates each batch
7. Merge, run `cargo test`, update tracking

### 10.2 Per-Agent Workflow

Each agent receives one card and follows:
1. **Decompose** card text into numbered clauses
2. **Write tests first** — Rust behavioral tests using `DebugRunner`
3. **Run tests** — `cargo test` (expect failures)
4. **Implement** — write `CardEffect` implementation
5. **Run tests** — verify passing
6. **Report verdict** — IMPLEMENTED / PARTIAL / BLOCKED

### 10.3 Agent Context Pack

Each agent receives:
- Card ID, name, metadata, full effect text
- C# reference from DCGO (if available)
- Rust Engine API Reference (EffectContext methods, Effect builder, enums)
- Engine gaps document
- Error checklist (adapted for Rust)

### 10.4 Error Checklist (Rust-Adapted)

1. `BeforePayCost` condition checks `source_card != card` first
2. `[When Attacking]` uses `EffectTiming::OnAttack`, not `OnAllyAttack`
3. No stubs — every effect has a complete `process` closure
4. Inherited effects use separate `Effect::inherited()` builder
5. Alt-digi includes ALL qualifying traits/names from card text
6. Tamer `[Start of Your Turn]` checks memory gate
7. `register_modifier` uses correct `ModifierType` and `Expiry`
8. Option cards: main = `EffectTiming::OptionMain`, security = `EffectTiming::OptionSecurity`
9. All closures are `Send + Sync + 'static` (required by trait bounds)
10. Target selections offer ALL valid targets — no auto-selection
11. Piercing: `ctx.grant_piercing(target, expiry)`
12. DP modification via `register_modifier(CHANGE_DP)`, not direct mutation
13. Field presence: check `source_permanent().is_some()`
14. Use `battle_area()`, not any other field accessor
15. String comparisons for card names use `contains_card_name()` (case-insensitive)
16. Reveal flows use `ctx.reveal_from_deck()`, not manual deck manipulation

### 10.5 Tracking

Same as Python pipeline:
- `validated_cards.json` — per-card verdicts
- Notion Archetype Verification Tracker — cross-archetype aggregation
- Engine gaps document — accumulates missing mechanics

## 11. Project Phases

### Phase 1: Engine Core
Game, Player, Permanent, CardSource, CardData, enums. Turn state machine with phase transitions. Memory management. No card effects yet.
**Validation:** Unit tests of game mechanics — draw, hatch, move from breeding, pass turn.

### Phase 2: Action Space + Tensor
Action mask builder, decoder, tensor builder. Constants matching Python engine exactly.
**Validation:** Round-trip tests. Encode-decode identity. Manual tensor inspection.

### Phase 3: Effect System + DebugRunner
CardEffect trait, EffectContext API, Effect builder, modifier registry. Test harness.
**Validation:** 3-5 hand-written test cards covering On Play, When Digivolving, On Attack, inherited, security, modifiers, selections.

### Phase 4: Combat + Security
Attack resolution, blocker/counter/alliance phases, security check, battle DP comparison.
**Validation:** Combat scenario tests — attack player, attack Digimon, blocker intercept, security reveal.

### Phase 5: Tauri Integration
Commands, state JSON serialization, frontend adapter. Wire React UI to Rust engine via `invoke()`.
**Validation:** Playable game with test cards in desktop app.

### Phase 6: Engine API Reference Doc
Written for AI agent consumption — complete EffectContext method reference, Effect builder patterns, EffectTiming enum values, common patterns and anti-patterns.

### Phase 7: Batch-Implement-Rust Skill
Adapt batch-fix-cards orchestration for Rust. Test the skill on a small set first.

### Phase 8: First Archetypes (2-3)
Implement via the skill. Fully tested. Playable in desktop app.
**Milestone: Desktop beta release.**

### Phase 9: PyO3 Bindings
`digimon-engine-py` crate. `RustHeadlessGame` class — drop-in for Python `HeadlessGame`.
**Validation:** `DigimonEnv` works with Rust backend. Parity tests pass.
**Milestone: RL training begins.**

## 12. Key Design Decisions

### Why Rust over C#/Go
- Near-C performance with zero-cost abstractions
- First-class Python interop via PyO3 (zero-copy numpy)
- Already in the toolchain (Tauri)
- Compiles directly into Tauri app (no sidecar)
- Strong type system catches card script bugs at compile time

### Why trait objects for card effects (not DSL or scripting)
- Full expressiveness — complex effects need arbitrary logic
- Compile-time checks — typos, type errors caught before runtime
- AI agents can target a well-typed API (EffectContext)
- No runtime overhead from interpreter
- Patterns translate recognizably from DCGO C# (closures instead of coroutines)

### Why tensor-first design
- Engine and RL agent see the same game state representation
- UI uses the same action IDs — no separate encoding
- Parity testing is mechanical (compare arrays)
- Models trained on Python engine transfer to Rust engine without retraining

### Why desktop-first beta
- No Python sidecar needed — simpler distribution
- Human testing catches bugs RL can't find (visual state, UX)
- Engine gets battle-tested before training begins
- Tauri app is already the target distribution format
