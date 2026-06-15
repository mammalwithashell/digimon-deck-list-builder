## MODIFIED Requirements

### Requirement: Rust engine emits Attack events at attack declaration

The Rust engine SHALL emit a `GameEvent::Attack` from `code/digimon-engine/src/combat/mod.rs` when an attack is declared, before any block resolution or interrupt window opens. The emission SHALL include `seq` (allocated via `Game::next_event_seq`), `player` (the attacker), `attacker_field_index` (Rust 0-based), `target_field_index` (`Some(field_index)` when targeting an opposing Digimon, `None` when targeting the security stack), and `target_player` (`Some(opponent)` for any attack against an opponent, `None` for self-targeting attacks where applicable).

The emission SHALL additionally carry card identity:

- `attacker_card_id: String` and `attacker_card_name: String` — the id and name of the attacking permanent's top card, read from `CardData` at declaration time. Non-optional.
- `target_card_id: Option<String>` and `target_card_name: Option<String>` — the id and name of the defending Digimon's top card when `target_field_index` is `Some`; `None` when the attack targets the security stack.

The emission SHALL also carry effective DP for battle visibility:

- `attacker_dp: Option<i32>` — the attacking permanent's effective DP at declaration (via `Game::effective_dp`, including modifiers). `None` only if the attacker has no DP.
- `target_dp: Option<i32>` — the defending Digimon's effective DP when the attack targets a Digimon; `None` for a security-stack attack.

Emission SHALL be unconditional — not gated on a feature flag or env variable, and not skipped when no consumer is attached. Consumers that receive the variant for the first time are expected to use a default-skip pattern (already established in replay and UI code paths).

#### Scenario: Attack on a Digimon emits Attack with field-index target

- **GIVEN** Player 1 has a Digimon at field index 0 and Player 2 has a Digimon at field index 0
- **WHEN** Player 1's Digimon declares an attack on Player 2's Digimon
- **THEN** the next drained event SHALL be `GameEvent::Attack` with `player = P1`, `attacker_field_index = 0`, `target_field_index = Some(0)`, and `target_player = Some(P2)`

#### Scenario: Attack on a Digimon carries attacker and target identity

- **GIVEN** Player 1's attacker top card is "BT1-009" (Greymon) and Player 2's target top card is "BT25-020" (Tyrannomon)
- **WHEN** the attack is declared
- **THEN** the `GameEvent::Attack` SHALL have `attacker_card_id = "BT1-009"`, `attacker_card_name = "Greymon"`, `target_card_id = Some("BT25-020")`, and `target_card_name = Some("Tyrannomon")`

#### Scenario: Attack on a Digimon carries effective DP for both sides

- **GIVEN** Player 1's attacker has effective DP 5000 and Player 2's target Digimon has effective DP 3000
- **WHEN** the attack is declared
- **THEN** the `GameEvent::Attack` SHALL have `attacker_dp = Some(5000)` and `target_dp = Some(3000)`
- **AND** a security-stack attack SHALL have `target_dp = None`

#### Scenario: Attack on security emits Attack with None target_field_index

- **WHEN** Player 1's Digimon attacks Player 2's security stack
- **THEN** the drained `GameEvent::Attack` SHALL have `target_field_index = None`, `target_player = Some(P2)`, `target_card_id = None`, and `target_card_name = None`

#### Scenario: Attack event precedes Block resolution

- **GIVEN** Player 2 has a blocker available
- **WHEN** Player 1 declares an attack and Player 2 blocks
- **THEN** the `GameEvent::Attack` SHALL appear in `drain_events()` before any block-resolution event (currently none, but the ordering invariant SHALL hold once block events are added)

### Requirement: PyO3 binding surfaces newly-emitted events unchanged

The Python `digimon_engine` PyO3 binding SHALL surface `GameEvent::Attack`, `GameEvent::Trash`, `GameEvent::Mill`, `GameEvent::SecurityReveal`, `GameEvent::MemoryChange`, the new `GameEvent::EffectTarget`, and the new reveal event(s) through the existing event drain path, including all card-identity fields added by this change. The binding's `event_to_pydict` mapping SHALL be updated so each new field and new variant appears as ordinary dict entries on the shared frontend `GameEvent` shape (`source_card_id`, `source_card_name`, `target_card_id`, `target_card_name`, and `meta` as appropriate). Player ids SHALL continue to follow whatever convention the existing event-drain pathway uses.

#### Scenario: Python consumer receives Attack identity

- **WHEN** a Python test triggers an attack and reads the drained event stream from the runner
- **THEN** one drained event SHALL be a dict with `type = "Attack"` carrying the attacker id+name and (for a Digimon target) the target id+name

#### Scenario: Python consumer receives EffectTarget variant

- **WHEN** a Python test triggers an effect that selects a target and reads the drained event stream
- **THEN** one drained event SHALL be a dict with `type = "EffectTarget"` carrying the source effect id+name and the chosen target id(s)+name(s)

## ADDED Requirements

### Requirement: Name-bearing card events carry card_name

The `GameEvent::Play`, `GameEvent::Digivolve`, `GameEvent::Trash`, `GameEvent::Mill`, and `GameEvent::SecurityReveal` variants SHALL each carry a non-optional `card_name: String` alongside their existing card-id field (`card_id` / `top_card_id`), populated from `CardData` at every emission site. The name field type SHALL be non-`Option` so the compiler flags any emission site that fails to populate it.

#### Scenario: Trash event carries the card name

- **WHEN** an effect causes Player 2 to discard "BT25-061" from hand
- **THEN** a `GameEvent::Trash` SHALL be drained with `card_id = "BT25-061"` and `card_name = "Offmon"`

#### Scenario: SecurityReveal carries the card name

- **WHEN** Player 1 reveals security card "BT25-098"
- **THEN** a `GameEvent::SecurityReveal` SHALL be drained with `card_id = "BT25-098"` and `card_name = "Cyber Engage"`

#### Scenario: Play and Digivolve carry the card name

- **WHEN** a card "BT25-007" is played and later digivolved into "BT25-045"
- **THEN** the `Play` event SHALL carry `card_name = "Gatchmon"` and the `Digivolve` event SHALL carry `card_name = "Onmon"`

### Requirement: GameEvent::MemoryChange carries optional effect-source identity

The `GameEvent::MemoryChange` variant SHALL carry `source_card_id: Option<String>` and `source_card_name: Option<String>`. When a memory change originates from a card effect (routed through `EffectContext::gain_memory` / `lose_memory`), both fields SHALL be populated with the effect's source card identity. When the change originates from cost payment, turn-pass, or any structural path with no card source, both fields SHALL be `None`. The source SHALL be threaded explicitly from `EffectContext` into the memory-mutation path; the engine SHALL NOT special-case tamers — every effect-sourced memory change is attributed uniformly.

#### Scenario: Tamer start-of-turn memory gain is attributed

- **GIVEN** a tamer "BT25-098" with a `[Start of Your Main Phase]` effect that gains 1 memory
- **WHEN** the effect resolves and emits `MemoryChange`
- **THEN** the event SHALL have `delta = +1`, `source_card_id = Some("BT25-098")`, and `source_card_name = Some("Cyber Engage")`

#### Scenario: Cost-payment memory change is unattributed

- **GIVEN** the agent plays a card paying a memory cost
- **WHEN** the resulting `MemoryChange` is emitted
- **THEN** the event SHALL have `source_card_id = None` and `source_card_name = None`

### Requirement: Rust engine emits EffectTarget events at selection commit

The Rust engine SHALL emit a `GameEvent::EffectTarget` when a card effect commits its target selection. The event SHALL carry `seq`, `player` (the controller of the effect), `source_card_id` + `source_card_name` (the effect's source card, from the `PendingSelection` `source_card` / `source_permanent`), and `targets` — a list of the chosen target cards each carrying `card_id` + `card_name`. The event SHALL be emitted for every committed target selection, including selections with a single legal target (forced picks), consistent with the no-approximations policy that routes such picks through `pending_selection`.

#### Scenario: Effect selecting one target emits EffectTarget

- **GIVEN** an effect on source "BT1-009" (Greymon) that selects one opposing Digimon "BT25-020" (Tyrannomon)
- **WHEN** the selection commits
- **THEN** a `GameEvent::EffectTarget` SHALL be drained with `source_card_id = "BT1-009"`, `source_card_name = "Greymon"`, and `targets` containing one entry `{ card_id: "BT25-020", card_name: "Tyrannomon" }`

#### Scenario: Forced single-target selection still emits EffectTarget

- **GIVEN** an effect that must target the only legal Digimon on the board
- **WHEN** the selection resolves with no real choice
- **THEN** a `GameEvent::EffectTarget` SHALL still be drained for that target

### Requirement: Rust engine emits reveal events for non-security reveals

The Rust engine SHALL emit a reveal event for each card revealed at a non-security reveal site: reveal-from-deck-top and trash-from-deck-top reveal. Each event SHALL carry `seq`, `player` (the player whose card is revealed), the revealed `card_id` + `card_name`, and a `source_zone` discriminator (`RevealZone::DeckTop` / `RevealZone::TrashFromDeckTop`) identifying which reveal site produced it. One event SHALL fire per revealed card, in reveal order. (`SecurityReveal` remains a distinct variant for the security-check path. A reveal-from-hand zone is intentionally omitted — the engine has no reveal-from-hand primitive and no card in the pool reveals from hand.)

#### Scenario: Reveal-deck-top emits a reveal event

- **WHEN** an effect reveals the top card "BT25-052" of Player 1's deck
- **THEN** a reveal event SHALL be drained with `player = P1`, `card_id = "BT25-052"`, `card_name = "Logimon"`, and a discriminator indicating deck-top

#### Scenario: Trash-from-deck-top reveal emits per card

- **WHEN** an effect trashes the top two cards of Player 2's deck, "BT25-056" then "BT22-009"
- **THEN** two consecutive reveal events SHALL be drained with `card_id` values `["BT25-056", "BT22-009"]` in that order, each with its name and a trash-from-deck-top discriminator

### Requirement: Identity, effect-target, and reveal emission is covered by integration tests

The engine repository SHALL include integration tests under `code/digimon-engine/tests/event_emission/` that assert: card-identity fields on `Attack` / `Play` / `Digivolve` / `Trash` / `Mill` / `SecurityReveal`; effect-source attribution on `MemoryChange`; `EffectTarget` emission (including a forced single-target case); and each of the three non-security reveal events. The tests SHALL run as part of the default `cargo test --manifest-path code/digimon-engine/Cargo.toml` invocation.

#### Scenario: Test suite exercises identity and new variants

- **WHEN** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test event_emission` is run
- **THEN** at least one test SHALL assert attacker/target identity on `GameEvent::Attack`
- **AND** at least one test SHALL assert `source_card_id` attribution on `GameEvent::MemoryChange`
- **AND** at least one test SHALL assert `GameEvent::EffectTarget` emission for a forced single-target selection
- **AND** at least one test SHALL assert each non-security reveal zone (deck-top, trash-from-deck-top)
- **AND** all tests SHALL pass
