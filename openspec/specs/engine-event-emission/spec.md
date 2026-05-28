# engine-event-emission Specification

## Purpose
Wires the Rust engine to emit `GameEvent::Attack`, `GameEvent::Trash`, and `GameEvent::SecurityReveal` at their existing call sites and extends the payloads of the already-emitted `GameEvent::Play` (with `cost_paid`, `cost_printed`, `via_alt_path`) and `GameEvent::Digivolve` (with `was_dna`, `was_blast_dna`, `memory_paid`). Covers the event surface required by the `reward-profiles` v1 components (`block_event`, `opp_deletion`, `own_deletion`, `digivolve_into_named_card`, cost-aware `play_named_card`) and remains independently useful to replay and UI consumers. PyO3 binding surfaces the new variants unchanged.

## Requirements

### Requirement: Rust engine emits Attack events at attack declaration

The Rust engine SHALL emit a `GameEvent::Attack` from `code/digimon-engine/src/game.rs` when an attack is declared, before any block resolution or interrupt window opens. The emission SHALL include `seq` (allocated via `Game::next_event_seq`), `player` (the attacker), `attacker_field_index` (Rust 0-based), `target_field_index` (`Some(field_index)` when targeting an opposing Digimon, `None` when targeting the security stack), and `target_player` (`Some(opponent)` for any attack against an opponent, `None` for self-targeting attacks where applicable).

Emission SHALL be unconditional — not gated on a feature flag or env variable, and not skipped when no consumer is attached. Consumers that receive the variant for the first time are expected to use a default-skip pattern (already established in replay and UI code paths).

#### Scenario: Attack on a Digimon emits Attack with field-index target

- **GIVEN** Player 1 has a Digimon at field index 0 and Player 2 has a Digimon at field index 0
- **WHEN** Player 1's Digimon declares an attack on Player 2's Digimon
- **THEN** the next drained event SHALL be `GameEvent::Attack` with `player = P1`, `attacker_field_index = 0`, `target_field_index = Some(0)`, and `target_player = Some(P2)`

#### Scenario: Attack on security emits Attack with None target_field_index

- **WHEN** Player 1's Digimon attacks Player 2's security stack
- **THEN** the drained `GameEvent::Attack` SHALL have `target_field_index = None` and `target_player = Some(P2)`

#### Scenario: Attack event precedes Block resolution

- **GIVEN** Player 2 has a blocker available
- **WHEN** Player 1 declares an attack and Player 2 blocks
- **THEN** the `GameEvent::Attack` SHALL appear in `drain_events()` before any block-resolution event (currently none, but the ordering invariant SHALL hold once block events are added)

### Requirement: Rust engine emits Trash events for every card-to-trash migration

The Rust engine SHALL emit a `GameEvent::Trash` for every individual card that moves into a player's trash zone from any source. Coverage SHALL include:

- Permanents deleted via `Game::delete_permanents_batch` (the post-2026-05-23 batched flow per CLAUDE.md rule 25). One `Trash` event SHALL fire per card in the deleted permanent's stack (top card plus all `digi_source` cards plus any inherited DigiEgg).
- Cards discarded from hand via effect-driven discard.
- Cards moved to trash from the security stack via effects (separate from `SecurityReveal` emission for the resolution path).
- Cards moved to trash from any other zone (deck, battle area's digi-source, etc.) by effect.

Each `Trash` emission SHALL include `seq`, `player` (the owner of the trash zone receiving the card — Rust 0-based), and `card_id` (the canonical card ID string).

Emission order SHALL match the order of physical card movement: when a permanent stack of N cards is trashed, N consecutive `Trash` events fire in stack-top-first order.

#### Scenario: Deleting a permanent emits Trash per card in the stack

- **GIVEN** a Player 1 Digimon at field index 0 whose stack contains a top card "BT8-079", one digi-source "BT4-091", and an inherited DigiEgg "ST1-01"
- **WHEN** the permanent is deleted via `Game::delete_permanents_batch`
- **THEN** three consecutive `GameEvent::Trash` events SHALL be drained with `player = P1` and `card_id` values `["BT8-079", "BT4-091", "ST1-01"]` in that order

#### Scenario: Discarding from hand emits Trash

- **WHEN** an effect causes Player 2 to discard "BT5-110" from hand
- **THEN** a `GameEvent::Trash` SHALL be drained with `player = P2` and `card_id = "BT5-110"`

#### Scenario: Effect-driven security trash emits Trash, not SecurityReveal

- **WHEN** an effect (not a security check) moves "BT3-103" from Player 1's security stack to Player 1's trash
- **THEN** a `GameEvent::Trash` SHALL be drained with `player = P1` and `card_id = "BT3-103"`
- **AND** no `GameEvent::SecurityReveal` SHALL fire for that movement

### Requirement: Rust engine emits SecurityReveal events at security-check resolution

The Rust engine SHALL emit a `GameEvent::SecurityReveal` for every card revealed during a security check (an attack reaching the defender's security stack). One event SHALL fire per revealed card. The emission SHALL include `seq`, `defender` (the player whose security is being checked — Rust 0-based), and `card_id`.

`SecurityReveal` SHALL fire at the moment of reveal, before any security-effect resolution and before the card moves to its post-check zone (trash, hand, deck, etc., per card-specific resolution). When the post-check movement sends the card to trash, a subsequent `GameEvent::Trash` SHALL also fire — `SecurityReveal` and `Trash` are independent events with different semantics.

When a single attack triggers reveal of multiple security cards (e.g., via Security Attack +N), one `SecurityReveal` SHALL fire per revealed card in reveal order.

#### Scenario: Single-security check emits one SecurityReveal

- **WHEN** Player 1 attacks Player 2's security and one card "BT2-058" is revealed
- **THEN** a `GameEvent::SecurityReveal` SHALL be drained with `defender = P2` and `card_id = "BT2-058"`

#### Scenario: Security Attack +1 emits two SecurityReveal events in order

- **GIVEN** Player 1's attacker has Security Attack +1 and Player 2's top two security cards are "BT2-058" then "BT3-103"
- **WHEN** the attack reaches security
- **THEN** two consecutive `GameEvent::SecurityReveal` events SHALL be drained with `defender = P2` and `card_id` values `["BT2-058", "BT3-103"]` in that order

#### Scenario: SecurityReveal precedes its corresponding Trash

- **WHEN** Player 1 reveals security card "BT2-058" which resolves to the trash zone
- **THEN** the drained event sequence SHALL contain `SecurityReveal { card_id: "BT2-058" }` followed by `Trash { card_id: "BT2-058", player: P2 }`

### Requirement: Event emission is covered by integration tests

The engine repository SHALL include integration tests under `code/digimon-engine/tests/event_emission/` that exercise each newly-emitted variant (`Attack`, `Trash`, `SecurityReveal`) via `DebugRunner` and assert both the event shape and emission ordering. The tests SHALL run as part of the default `cargo test --manifest-path code/digimon-engine/Cargo.toml` invocation.

#### Scenario: Test suite exercises all three variants

- **WHEN** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test event_emission` is run
- **THEN** at least one test SHALL exercise `GameEvent::Attack` emission
- **AND** at least one test SHALL exercise `GameEvent::Trash` emission for permanent deletion
- **AND** at least one test SHALL exercise `GameEvent::SecurityReveal` emission
- **AND** all tests SHALL pass

### Requirement: GameEvent::Play carries cost-paid and alt-path payload

The `GameEvent::Play` variant in `code/digimon-engine/src/events.rs` SHALL include three additional fields beyond its existing `seq`, `player`, `card_id`, `field_index`:

- `cost_paid: i16` — the actual memory paid for this play AFTER all cost reductions (tamer-driven, alt-path discounts, etc.). May be zero or even negative when an alt-path grants free play.
- `cost_printed: i16` — the card's printed `play_cost` from `CardData::play_cost` at the moment of play. Captured at emission time so consumers do not need to re-read CardData.
- `via_alt_path: Option<String>` — the canonical alt-path key from `CompiledAltPathKind::as_key()` (one of `"digivolve"`, `"dna_digivolve"`, `"blast_dna_digivolve"`, `"digixros"`, `"burst_digivolve"`, `"app_fusion"`, `"assembly"`, `"activated_digivolve"`) when the card was played through a registered alt-path that bypassed the printed cost. `None` when the card was played through the standard PLAY action (with or without a generic tamer-driven cost reduction — generic reductions do not surface as an alt-path key).

The Play event SHALL be emitted at every site where a card enters the battle area from hand, regardless of how the play was initiated (standard `PLAY_HAND` action, on-play effect, alt-path effect, etc.). The new fields SHALL be populated at every emission site (the field types are non-Option for `cost_paid` and `cost_printed` to make compiler errors flag any forgotten emission site).

#### Scenario: Standard play emits Play with via_alt_path=None and matched costs

- **GIVEN** the agent plays BT17-015 WarGreymon (printed cost 11) via the standard PLAY action with Tai & Matt's cost reduction reducing the cost to 8
- **WHEN** the Play event is emitted
- **THEN** the event SHALL have `card_id = "BT17-015"`, `cost_paid = 8`, `cost_printed = 11`, `via_alt_path = None`

#### Scenario: Alt-path play emits Play with via_alt_path populated

- **GIVEN** AD1-025 Omnimon is replayed via the Partition alt-path after leaving the battle area
- **WHEN** the Play event is emitted for the new permanent
- **THEN** the event SHALL have `card_id = "AD1-025"`, `cost_paid = 0`, `cost_printed = 15`, `via_alt_path = Some("assembly")`

#### Scenario: Effect-initiated free play emits Play with cost_paid=0 and via_alt_path=None

- **GIVEN** Davis & Ken's `[Start of Your Main Phase]` effect plays a Lv3 Veemon from hand without paying its cost
- **WHEN** the Play event is emitted
- **THEN** the event SHALL have `cost_paid = 0`, `cost_printed = <Veemon's printed cost>`, `via_alt_path = None`
- **AND** profile components matching on `cost_paid_eq: 0` SHALL capture this play (alt-path keys are reserved for `CompiledAltPathKind` variants; generic on-play-triggered free plays do not surface as an alt-path)

### Requirement: GameEvent::Digivolve carries DNA flags and result identity

The `GameEvent::Digivolve` variant in `code/digimon-engine/src/events.rs` SHALL include three additional fields beyond its existing `seq`, `player`, `top_card_id`, `field_index`, `from_stack_top`:

- `was_dna: bool` — `true` for any DNA-style digivolve, including: standard `dna_costs` path, registered end-of-turn DNA alt-paths, Blast DNA, and xros_req-driven DNA. `false` for normal evo-cost digivolves.
- `was_blast_dna: bool` — narrower flag, `true` only when the digivolve was through a `CompiledAltPathKind::BlastDnaDigivolve` alt-path. Implies `was_dna = true`.
- `memory_paid: i16` — the actual memory cost paid for this digivolve. May be zero (e.g., Blast DNA at cost 0, xros_req at cost 0) or the printed evo cost.

The Digivolve event SHALL be emitted at every digivolve site: the standard battle-area `digivolve_from_hand` path (already wired), the breeding-area path, the on-play-triggered free digivolve paths (e.g., WarGreymon BT17-015's "1 of your Gabumon may digivolve into MetalGarurumon" branch), DNA digivolve, and Blast DNA digivolve. Every emission SHALL populate the new fields; the field types are non-Option to force compiler-time coverage of all emission sites.

`result_traits` and `result_level` are NOT added to the event payload — components requiring trait/level matching SHALL look them up via the registry using `top_card_id`. Rationale: keeping the event payload narrow avoids redundancy with `CardData`, and component implementations already have access to the registry through the `RewardEventBus` (the bus owns the registry handle).

#### Scenario: Standard digivolve emits Digivolve with was_dna=false

- **GIVEN** the agent digivolves a Lv5 base into BT17-015 WarGreymon by paying the printed 3-memory evo cost
- **WHEN** the Digivolve event is emitted
- **THEN** the event SHALL have `top_card_id = "BT17-015"`, `was_dna = false`, `was_blast_dna = false`, `memory_paid = 3`

#### Scenario: Blast DNA digivolve emits Digivolve with both flags

- **GIVEN** the agent Blast-DNA digivolves WG + MG → BT17-078 Omnimon at the cost-0 path
- **WHEN** the Digivolve event is emitted
- **THEN** the event SHALL have `top_card_id = "BT17-078"`, `was_dna = true`, `was_blast_dna = true`, `memory_paid = 0`

#### Scenario: xros_req DNA digivolve emits was_dna=true, was_blast_dna=false

- **GIVEN** the agent DNA digivolves into AD1-025 Omnimon via its xros_req DNA path (Lv6 Greymon-name + Lv6 Garurumon-name, cost 0)
- **WHEN** the Digivolve event is emitted
- **THEN** the event SHALL have `top_card_id = "AD1-025"`, `was_dna = true`, `was_blast_dna = false`, `memory_paid = 0`

#### Scenario: On-play-triggered free digivolve still emits Digivolve

- **GIVEN** WarGreymon BT17-015's on-play effect digivolves a Gabumon → BT17-027 MetalGarurumon ignoring requirements and without paying the cost
- **WHEN** the resulting Digivolve event is emitted
- **THEN** the event SHALL have `top_card_id = "BT17-027"`, `was_dna = false`, `was_blast_dna = false`, `memory_paid = 0`

**Counter-bump non-requirement (deferred):** the digivolve counter `n_digivolutions[player]` is NOT required to increment for effect-initiated digivolves. Current engine behavior at `game_actions.rs::effect_initiated_digivolve_from_source_inner` explicitly does not bump the counter for these paths (see code comment at `game_actions.rs:4155`), and this change preserves that behavior. The reward signal will undercount effect-initiated digivolves; a follow-up proposal MAY revisit this once shaping experiments reveal whether the undercount materially affects training.

### Requirement: PyO3 binding surfaces newly-emitted events unchanged

The Python `digimon_engine` PyO3 binding SHALL surface `GameEvent::Attack`, `GameEvent::Trash`, and `GameEvent::SecurityReveal` through the existing event drain path without requiring any binding-layer changes (the events serialize via the existing `serde` derivation on `GameEvent` and the JSON-to-Python conversion already in place). Python consumers SHALL receive the new variants as ordinary dict entries in the event stream returned by the runner's event-drain accessor.

#### Scenario: Python consumer receives new Attack variant

- **WHEN** a Python test triggers an attack and reads the drained event stream from the runner
- **THEN** one of the drained events SHALL be a dict with `type = "Attack"`, `attacker_field_index`, `target_field_index`, and `target_player` keys
- **AND** Python integers in `target_player` SHALL follow the Python 1/2 player-id convention OR remain Rust 0-based and be translated by the consumer — whichever the existing event-drain pathway already does (binding behavior unchanged, only the variant is new)

### Requirement: get_rl_state exposes turn_count

The Python `RustHeadlessGame.get_rl_state()` accessor SHALL include a `turn_count` key holding the current `game.turn_count` value as an integer. The field SHALL be present on every call, including the first call before any action has been taken.

Wiring lands in `code/digimon-engine-py/src/lib.rs` alongside the existing digivolve counter exposures.

#### Scenario: Initial turn_count is exposed at game start

- **WHEN** `RustHeadlessGame(deck1, deck2, seed=0)` is constructed and `get_rl_state()` is called before any `step()`
- **THEN** the returned dict SHALL contain `turn_count` as an integer

#### Scenario: turn_count advances across turns

- **GIVEN** a fresh `RustHeadlessGame`
- **WHEN** the players take turns (each Pass Turn action incrementing the counter)
- **THEN** the `turn_count` value returned by `get_rl_state()` SHALL advance monotonically (1, 2, 3, ...)

### Requirement: Rust engine exposes n_digivolve_driven_attacks counter

The Rust engine SHALL maintain a per-player counter `n_digivolve_driven_attacks: [u32; 2]` on `Game`. The counter SHALL be incremented exactly once per qualifying attack, where a qualifying attack satisfies ALL of:

- The attacking permanent's effective level is ≥ 5 (parameter — initial value 5; future refinements MAY add a config knob).
- The attack's target is the opponent's security stack (i.e., `AttackTarget::Player`).
- The attack actually connects with security — blocked or cancelled attacks SHALL NOT increment the counter.
- Per-attack semantics: the counter bumps once per attack regardless of `Security Attack +N` revealing multiple cards. Per-card semantics are explicitly NOT supported by the engine counter.

The counter is exposed via `get_rl_state()` as `n_digivolve_driven_attacks` — a 2-element array indexed by Rust 0-based player ID (matching the existing digivolve counter exposure pattern).

The increment site lives in `code/digimon-engine/src/combat.rs` at the appropriate point in the attack-resolution path. The engine does NOT filter by "this turn" or "has sources" — the bus/component layer handles those mode predicates.

#### Scenario: Lv5+ attacker on security increments

- **GIVEN** a Lv5 Digimon on field index 0 (P1 side), opponent security has cards
- **WHEN** Player 1 declares an attack on security and the attack lands (not blocked)
- **THEN** `n_digivolve_driven_attacks[0]` SHALL increment by 1

#### Scenario: Lv4 attacker on security does not increment

- **GIVEN** a Lv4 Digimon attacks security
- **WHEN** the attack lands
- **THEN** `n_digivolve_driven_attacks[0]` SHALL NOT change

#### Scenario: Lv5+ attacker on digimon does not increment

- **GIVEN** a Lv5 Digimon attacks an opposing Lv4 Digimon
- **WHEN** the attack resolves (battle, regardless of outcome)
- **THEN** `n_digivolve_driven_attacks[0]` SHALL NOT change

#### Scenario: Blocked Lv5+ attack on security does not increment

- **GIVEN** a Lv5 Digimon declares an attack on security
- **AND** an opposing Digimon with Blocker declares a block
- **WHEN** the block resolves (attack diverted to the blocker)
- **THEN** `n_digivolve_driven_attacks[0]` SHALL NOT change (the attack did not reach security)

#### Scenario: Security Attack +N revealing multiple cards still increments once

- **GIVEN** a Lv5 Digimon with Security Attack +1 attacks security
- **AND** the attack reveals two security cards in sequence
- **WHEN** the attack completes
- **THEN** `n_digivolve_driven_attacks[0]` SHALL increment by exactly 1 (per-attack, not per-card)

#### Scenario: Counter exposed in get_rl_state

- **WHEN** `get_rl_state()` is called
- **THEN** the returned dict SHALL contain `n_digivolve_driven_attacks` as a 2-element list or tuple
- **AND** indices [0] and [1] SHALL hold the per-player counts (Rust 0-based)

### Requirement: PyO3 binding exposes breeding-area marker constant

The `digimon_engine` PyO3 module SHALL export a constant `BREEDING_TARGET` (and/or `BREEDING_SLOT`) reflecting the value used in the action space and battle-area indexing to identify the breeding slot. Python consumers (the `RewardEventBus`) consume this to distinguish breeding-area `Digivolve` events from battle-area ones.

The constant's value SHALL match the one used at the Rust side (`crate::action::space::BREEDING_TARGET` or equivalent). Cross-language drift is prevented by having the binding read from the canonical Rust source rather than redefining.

#### Scenario: Constant importable from Python

- **WHEN** Python code does `from digimon_engine import BREEDING_TARGET`
- **THEN** the import SHALL succeed
- **AND** `BREEDING_TARGET` SHALL be an integer matching the Rust-side definition
