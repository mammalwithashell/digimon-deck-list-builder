## ADDED Requirements

### Requirement: Effect-execution code is organized by mechanic across tiers

The engine's effect-execution code SHALL be organized into modules by game mechanic, using a parallel `<tier>/<mechanic>` address scheme. The Tier-3 scripting facade (`EffectContext`) and the Tier-2 operations layer (`game_actions`) SHALL use the same mechanic names so that a given operation lives at a predictable location. `EffectContext` and `Game` SHALL remain single types whose `impl` blocks are split across mechanic files (no new public sub-structs or sub-traits that change the call surface).

#### Scenario: Facade method lives in its mechanic module

- **WHEN** a developer looks for a facade operation on a known mechanic (e.g. a `play_*` method)
- **THEN** it is defined in `effect_context/action/<mechanic>.rs` (e.g. `action/play.rs`) under an `impl EffectContext` block, not in a monolithic `effect_context/mod.rs`

#### Scenario: Tier-2 and Tier-3 use parallel mechanic names

- **WHEN** a mechanic exists in both the operations layer and the facade (e.g. `digivolve`)
- **THEN** both `game_actions/<mechanic>.rs` and `effect_context/action/<mechanic>.rs` exist under the same mechanic name

#### Scenario: Call surface is unchanged after decomposition

- **WHEN** the decomposition is complete
- **THEN** every previously-public `EffectContext` and `Game` method retains its identical name and signature, and callers (card scripts, DSL lowerings, PyO3 bindings) compile without modification

### Requirement: Rules machinery lives in the operations tier, not the facade

Rules machinery — replacement-window dispatch (`try_replace`), observer/event firing (`fire_*`), and direct `battle_area` / stack mutation — SHALL reside in Tier 2 (`game_actions` / core), not in the Tier-3 facade. The facade's responsibilities are limited to: effect-only guards, effect identity injection, type ergonomics, sugar/composition, and effect-resolution entry points. An effect-only operation MAY hold rules machinery in the facade ONLY IF no Tier-2 counterpart exists, and MUST document that exception in a doc comment.

#### Scenario: A facade operation delegates rules machinery downward

- **WHEN** a facade operation needs replacement timing, observer firing, or stack mutation
- **THEN** it delegates to a Tier-2 primitive that performs that machinery, rather than executing it inline

#### Scenario: A facade-resident exception is documented

- **WHEN** an effect-only operation legitimately holds rules logic in the facade because no Tier-2 counterpart exists
- **THEN** its doc comment states why it lives in Tier 3

### Requirement: No layering inversion

No Tier-2 / core function SHALL construct an `EffectContext` solely to invoke a named facade *operation* method that carries rules logic. Constructing an `EffectContext` to hand control to a card-authored effect closure (via `process` / `pay_cost_fn` / `run_steps` and equivalent effect-entry points) is permitted; invoking a named mutation operation upward is not.

#### Scenario: de_digivolve is resolved without inversion

- **WHEN** `de_digivolve_from_effect` performs a de-digivolve
- **THEN** it calls a Tier-2 `de_digivolve` operation directly, and does not construct an `EffectContext` to call a facade `de_digivolve` method

#### Scenario: Effect-entry construction remains allowed

- **WHEN** the engine needs to run a card-authored effect (a queued trigger, a scheduled effect, a replacement, a Main-phase activation)
- **THEN** it MAY construct an `EffectContext` and hand control to the effect's closure, and this is not considered an inversion

### Requirement: Single shared source-trashing primitive

Trashing a digivolution/stacked source SHALL be performed through one shared Tier-2 primitive that pops the source, moves it to trash, and fires the source-trashed observer with the correct cause. Facade methods that trash sources SHALL delegate to this primitive rather than hand-rolling the pop + trash-move + observer-fire sequence.

#### Scenario: Source-trashing methods share one primitive

- **WHEN** any facade method trashes a stacked source
- **THEN** it delegates to the shared Tier-2 source-trashing primitive, and the per-method hand-rolled `pop()` + `trash.push()` + source-trashed-observer sequence does not recur

#### Scenario: Observer firing order is preserved

- **WHEN** the source-trashing primitive replaces a previously hand-rolled copy
- **THEN** the resulting source-trashed observer/trigger ordering is identical to the pre-refactor behavior

### Requirement: Observation is a read-only output port

The observation/tensor layer SHALL be grouped as a named output-port module and SHALL only read game state. Its entry points SHALL take an immutable `&Game` (or equivalent read-only view) and SHALL NOT mutate game state.

#### Scenario: Tensor builders are read-only

- **WHEN** an observation/tensor entry point is invoked
- **THEN** it accepts game state immutably and returns a tensor without mutating the game

#### Scenario: Core does not depend on observation

- **WHEN** the engine resolves rules and effects
- **THEN** it does so without depending on the observation module (the dependency points only from observation into the core)

### Requirement: Behavior is preserved across the decomposition

The decomposition and logic-relocation SHALL NOT change observable engine behavior. Card effects, the action space, tensor layouts/profiles, and cross-engine parity SHALL be identical before and after.

#### Scenario: Full regression suite stays green

- **WHEN** any decomposition or relocation step lands
- **THEN** the behavioral, parity (Rust/Python and DCGO replay), and tensor test suites pass unchanged

#### Scenario: Parity-sensitive relocations are gated

- **WHEN** the de_digivolve relocation or the source-trashing primitive consolidation lands
- **THEN** the de-digivolve/deletion parity and permanent-deletion semantics tests pass without modification to their expectations
