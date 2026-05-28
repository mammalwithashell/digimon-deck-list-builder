## ADDED Requirements

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
