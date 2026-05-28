## ADDED Requirements

### Requirement: Gameplay reward shape lives in a separate YAML file

The system SHALL load universal game-mechanic reward shaping from `code/digimon_gym/agents/reward/gameplay.yaml` — a separate file from archetype overlays in `profiles.yaml`. The default path is overridable via `TrainingConfig.reward_gameplay_path`.

`gameplay.yaml` SHALL define exactly one profile named `gameplay`. The profile contains the universal components (terminal_outcome, quick_win_bonus, stall_penalty, step_penalty, security_remove, security_lost, digivolve, dna_digivolve, breeding_digivolve, digivolve_driven_attack) at the values that define the universal baseline.

Loading the file SHALL fail at parse time when:

- The file does not exist (no silent fallback — the gameplay shape is required for the framework to operate).
- The file's top-level structure does not define a profile named `gameplay`.
- The `gameplay` profile attempts to `inherits:` another profile (gameplay is the root of inheritance).
- Any component declaration is malformed (same validation rules as profiles.yaml).

#### Scenario: gameplay.yaml absent fails at load

- **GIVEN** `reward_gameplay_path` points at a non-existent file
- **WHEN** training starts
- **THEN** the training run SHALL fail with a clear error naming the missing path
- **AND** training SHALL NOT proceed with a fallback / synthesized gameplay shape

#### Scenario: gameplay profile attempting to inherit fails

- **GIVEN** `gameplay.yaml` contains a `gameplay` profile with `inherits: something`
- **WHEN** the loader parses the file
- **THEN** loading SHALL fail with an error naming the disallowed `inherits` field
- **AND** the message SHALL explain that the gameplay profile is the inheritance root

#### Scenario: gameplay.yaml defines exactly one profile

- **GIVEN** `gameplay.yaml` contains profiles `gameplay` and `something_else`
- **WHEN** the loader parses the file
- **THEN** loading SHALL fail with an error explaining that `gameplay.yaml` defines exactly one profile (`gameplay`)
- **AND** the message SHALL direct the operator to put additional profiles in `profiles.yaml`

### Requirement: Two-file loader merges namespaces

`ProfileLoader` SHALL accept both `gameplay_path` and `profiles_path` at construction. It SHALL load and parse both files, then merge their profile namespaces into a single map keyed by profile name.

Name collisions between the two files SHALL fail at load time — a profile name appearing in both files is an error, regardless of structural equality of the definitions.

Every profile defined in `profiles.yaml` SHALL declare `inherits:` pointing at a profile reachable (transitively) from a profile defined in `gameplay.yaml`. Profiles in `profiles.yaml` SHALL NOT serve as inheritance roots. Validation runs at parse time.

Inheritance resolution, override semantics, key-params override de-dup, and `key_cards:` expansion all work across files — a `profiles.yaml` profile inheriting `gameplay` resolves its component list by walking into `gameplay.yaml`.

#### Scenario: Profile name collision across files fails

- **GIVEN** `gameplay.yaml` defines profile `foo` AND `profiles.yaml` defines profile `foo`
- **WHEN** the loader parses both files
- **THEN** loading SHALL fail with an error naming the colliding profile name AND both file paths
- **AND** the message SHALL instruct the operator to rename one or the other

#### Scenario: profiles.yaml profile without inherits fails

- **GIVEN** a profile in `profiles.yaml` with no `inherits:` field
- **WHEN** the loader parses the file
- **THEN** loading SHALL fail with an error naming the profile
- **AND** the message SHALL explain that profiles.yaml entries MUST inherit (directly or transitively) from a gameplay-file profile

#### Scenario: profiles.yaml profile inherits from gameplay-file profile

- **GIVEN** `profiles.yaml` defines `dna_omnimon_combo_v1` with `inherits: gameplay`
- **AND** `gameplay.yaml` defines `gameplay`
- **WHEN** the loader resolves `dna_omnimon_combo_v1`
- **THEN** resolution SHALL succeed
- **AND** the resolved component list SHALL include components inherited from `gameplay` plus the child's overrides

#### Scenario: Cycles across files fail with cycle path

- **GIVEN** `gameplay.yaml` profile `gameplay` inherits from `profiles.yaml` profile `aux`, AND `profiles.yaml` profile `aux` inherits from `gameplay`
- **WHEN** the loader parses both files
- **THEN** loading SHALL fail with an error naming each profile in the cycle path

### Requirement: TrainingConfig exposes gameplay path

`TrainingConfig` SHALL include a `reward_gameplay_path: str` field defaulting to `"code/digimon_gym/agents/reward/gameplay.yaml"`. The existing `reward_profiles_path` field is unchanged.

Both paths SHALL be passed into `ProfileLoader` at run-start.

When both files are present and load successfully, the wrapper resolves the active profile from the merged namespace and proceeds as in v1 (override → archetype lookup → `_default` fallback).

#### Scenario: Default config loads both files

- **WHEN** training starts with default `TrainingConfig` values
- **THEN** `ProfileLoader` SHALL load both `gameplay.yaml` and `profiles.yaml`
- **AND** the resulting merged namespace SHALL contain at least: `gameplay`, `_default`, `dna_omnimon_combo_v1`, `bg_imperialdramon_combo_v1`

### Requirement: Sidecar persists gameplay hash separately

The `reward_profiles.meta.json` sidecar (from `reward-profiles` capability) SHALL include four reward-related fields:

- `reward_gameplay_path: str`
- `reward_gameplay_hash: str` — canonical hash of the gameplay.yaml content (sha256:hex form)
- `reward_profiles_path: str` (existing)
- `reward_profiles_hash: str` (existing) — canonical hash of profiles.yaml content

On resume, the system SHALL compare BOTH hashes against the checkpoint's recorded hashes. Mismatch in either file SHALL raise the existing `RewardProfilesHashMismatchError`. The error message SHALL name which file (gameplay or profiles) drifted — operators see "gameplay shape changed" vs "archetype overlay changed" directly.

The `--reward-profiles-override-mismatch` CLI flag covers BOTH file hashes — operators do not need separate flags.

#### Scenario: Sidecar records both hashes

- **WHEN** a training run starts with default config
- **THEN** `models/<run>/reward_profiles.meta.json` SHALL include `reward_gameplay_path`, `reward_gameplay_hash`, `reward_profiles_path`, `reward_profiles_hash`
- **AND** both hashes SHALL start with `sha256:`

#### Scenario: Resume fails with gameplay-named mismatch

- **GIVEN** a checkpoint with `reward_gameplay_hash = "sha256:abc..."` recorded in its sidecar
- **WHEN** the operator edits `gameplay.yaml` to change `stall_penalty.scale` and attempts resume
- **THEN** the resume SHALL fail with `RewardProfilesHashMismatchError`
- **AND** the message SHALL name `gameplay.yaml` as the file that drifted
- **AND** the message SHALL show both checkpoint and current hashes
- **AND** the message SHALL instruct passing `--reward-profiles-override-mismatch` to proceed

#### Scenario: Override flag covers both files

- **GIVEN** both `gameplay.yaml` and `profiles.yaml` have drifted since checkpoint
- **WHEN** resume is attempted with `--reward-profiles-override-mismatch`
- **THEN** the resume SHALL proceed without error
- **AND** a fresh sidecar SHALL be written with the new hashes for both files

### Requirement: quick_win_bonus component

A `quick_win_bonus` component SHALL be registered with the following contract:

- **Parameters**: `peak_turn: int` (default 3), `peak_value: float` (default 5.0), `decay_per_turn: float` (default 1.25).
- **Trigger**: fires only on `TerminalOutcome` occurrences where `winner_id == 1` (the agent won). No emission on loss or draw.
- **Formula**: `max(0, peak_value − decay_per_turn × max(0, turn − peak_turn))`. Uses `turn_count` from the `TerminalOutcome` occurrence (not `step_count`).
- **No firing before `peak_turn`**: when `turn < peak_turn`, emission is zero (the formula's inner max clamps).

#### Scenario: Peak fires at peak_turn on agent win

- **GIVEN** a `quick_win_bonus` component with default parameters (peak_turn=3, peak_value=5, decay=1.25)
- **WHEN** a TerminalOutcome with winner_id=1 and turn_count=3 fires
- **THEN** the component SHALL emit `+5.0`

#### Scenario: Linear decay from peak

- **GIVEN** same default parameters
- **WHEN** TerminalOutcomes fire at turn_count 3, 4, 5, 6, 7
- **THEN** emissions SHALL be `+5.0, +3.75, +2.5, +1.25, 0.0` respectively

#### Scenario: Zero after seam

- **GIVEN** same default parameters
- **WHEN** TerminalOutcomes fire at turn_count 8, 10, 20
- **THEN** all emissions SHALL be `0.0` (formula's outer max clamps)

#### Scenario: No bonus before peak_turn

- **GIVEN** same default parameters
- **WHEN** a TerminalOutcome fires at turn_count=1 with winner_id=1
- **THEN** the component SHALL emit `0.0` (turn < peak_turn → inner max clamps)

#### Scenario: No bonus on agent loss

- **GIVEN** same default parameters
- **WHEN** a TerminalOutcome fires at turn_count=3 with winner_id=2 (opponent won)
- **THEN** the component SHALL emit `0.0` (winner filter fails)

#### Scenario: No bonus on draw

- **GIVEN** same default parameters
- **WHEN** a TerminalOutcome fires at turn_count=3 with winner_id=None
- **THEN** the component SHALL emit `0.0`

### Requirement: stall_penalty component

A `stall_penalty` component SHALL be registered with the following contract:

- **Parameters**: `threshold_turn: int` (default 7), `scale: float` (default 0.1), `apply_to_winner: bool` (default true), `apply_to_loser: bool` (default true).
- **Trigger**: fires on every `TerminalOutcome` occurrence, regardless of winner.
- **Formula**: `−scale × max(0, turn − threshold_turn)²`. Always non-positive. Reads `turn_count`.
- **Apply gates**: emission SHALL be zeroed when winner is agent (1) and `apply_to_winner=false`; or winner is opponent (2) and `apply_to_loser=false`. Draws ALWAYS receive the penalty regardless of the apply flags.

#### Scenario: No penalty at or before threshold

- **GIVEN** default parameters (threshold=7, scale=0.1)
- **WHEN** a TerminalOutcome fires at turn_count 1, 5, 7
- **THEN** all emissions SHALL be `0.0`

#### Scenario: Quadratic growth after threshold

- **GIVEN** default parameters
- **WHEN** a TerminalOutcome fires at turn_count 10, 15, 20, 30
- **THEN** emissions SHALL approximately equal `-0.9, -6.4, -16.9, -52.9` respectively
- **AND** the values SHALL be exactly `-0.1 × (turn - 7)²`

#### Scenario: Applies to all outcomes by default

- **GIVEN** default parameters
- **WHEN** TerminalOutcomes fire at turn_count=15 for each of winner_id ∈ {1, 2, None}
- **THEN** all three SHALL emit `-6.4`

#### Scenario: apply_to_winner=false disables on agent win only

- **GIVEN** `apply_to_winner=false, apply_to_loser=true`
- **WHEN** TerminalOutcomes fire at turn_count=15 for winner_id ∈ {1, 2, None}
- **THEN** winner_id=1 emits `0.0`; winner_id=2 emits `-6.4`; winner_id=None (draw) emits `-6.4`

#### Scenario: Draws always penalized regardless of apply flags

- **GIVEN** `apply_to_winner=false, apply_to_loser=false`
- **WHEN** a TerminalOutcome with winner_id=None fires at turn_count=15
- **THEN** the component SHALL emit `-6.4` (draws are not gated by the apply flags)

### Requirement: breeding_digivolve component

A `breeding_digivolve` component SHALL be registered with the following contract:

- **Parameters**: `reward_per_level: Mapping[int, float]` — required. Default shipped value `{3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}`.
- **Trigger**: fires on `Digivolved` occurrences where `is_breeding == true`. The occurrence's `result_level` is looked up in `reward_per_level`; missing keys produce zero.
- **Per agent only**: `player == 1` filter applied (consistent with existing digivolve components).

#### Scenario: Lv4 raise fires the corresponding reward

- **GIVEN** default `reward_per_level`
- **WHEN** a Digivolved occurrence fires with `player=1, is_breeding=true, result_level=4`
- **THEN** the component SHALL emit `+0.2`

#### Scenario: Lv6 in breeding fires the penalty

- **GIVEN** default `reward_per_level`
- **WHEN** a Digivolved occurrence fires with `player=1, is_breeding=true, result_level=6`
- **THEN** the component SHALL emit `-0.4`

#### Scenario: Battle-area digivolves ignored

- **GIVEN** default `reward_per_level`
- **WHEN** a Digivolved occurrence fires with `player=1, is_breeding=false, result_level=4`
- **THEN** the component SHALL emit `0.0` (is_breeding filter fails)

#### Scenario: Result level not in dict produces zero

- **GIVEN** `reward_per_level = {3: 0.4}`
- **WHEN** a Digivolved occurrence fires with `is_breeding=true, result_level=5`
- **THEN** the component SHALL emit `0.0`

#### Scenario: Opponent's breeding digivolves ignored

- **GIVEN** default `reward_per_level`
- **WHEN** a Digivolved occurrence fires with `player=2, is_breeding=true, result_level=4`
- **THEN** the component SHALL emit `0.0` (player filter fails)

### Requirement: digivolve_driven_attack component

A `digivolve_driven_attack` component SHALL be registered with the following contract:

- **Parameters**: `mode: str` (one of `"this_turn"`, `"has_sources"`, `"either"`, `"both"`; default `"either"`), `attacker_min_level: int` (default 5), `reward: float` (default 0.5), `per_card: bool` (default false).
- **Trigger**: fires on `DigivolveDrivenAttack` occurrences. Per-attack semantics (one occurrence per qualifying attack regardless of Security Attack +N revealing multiple cards).
- **Mode filter** (component-side, not engine):
  - `this_turn`: emission requires `event.this_turn == true`
  - `has_sources`: emission requires `event.has_sources == true`
  - `either`: emission requires `event.this_turn OR event.has_sources`
  - `both`: emission requires `event.this_turn AND event.has_sources`
- **Level filter**: emission requires `event.attacker_level >= attacker_min_level`. The engine's increment site already filters at the bound configured at engine-build time; the component re-checks against its own threshold, which MAY be higher.
- **`per_card`**: in v1, this parameter is accepted but unused. When `per_card=true` is set, the loader SHALL emit a warning that per-card semantics are deferred to v2.

#### Scenario: Mode either fires when this_turn flag set

- **GIVEN** default parameters (`mode=either, attacker_min_level=5, reward=0.5`)
- **WHEN** a DigivolveDrivenAttack occurrence fires with `attacker_level=5, this_turn=true, has_sources=false`
- **THEN** the component SHALL emit `+0.5`

#### Scenario: Mode either fires when has_sources flag set

- **GIVEN** default parameters
- **WHEN** a DigivolveDrivenAttack occurrence fires with `attacker_level=6, this_turn=false, has_sources=true`
- **THEN** the component SHALL emit `+0.5`

#### Scenario: Mode this_turn does not fire on has_sources alone

- **GIVEN** `mode=this_turn`
- **WHEN** a DigivolveDrivenAttack occurrence fires with `attacker_level=5, this_turn=false, has_sources=true`
- **THEN** the component SHALL emit `0.0`

#### Scenario: Below attacker_min_level does not fire

- **GIVEN** `attacker_min_level=6`
- **WHEN** a DigivolveDrivenAttack occurrence fires with `attacker_level=5, this_turn=true, has_sources=true, mode=either`
- **THEN** the component SHALL emit `0.0`

#### Scenario: per_card=true emits load-time warning and behaves as per-attack

- **GIVEN** a profile component with `digivolve_driven_attack: { per_card: true, ... }`
- **WHEN** the profile is loaded
- **THEN** a warning SHALL be logged naming the component and explaining per-card is a v2 feature
- **AND** the component SHALL behave as `per_card=false` at runtime

### Requirement: Telemetry — gameplay-specific TB scalars

The `WinRateCallback` SHALL emit two new TB scalars per eval window:

- `pilot/mean_eval_winning_turn` — mean `turn_count` at terminal across eval-window games where `winner_id == 1`. Null when no wins occurred.
- `pilot/mean_eval_digivolve_driven_attacks` — mean count of agent-side `DigivolveDrivenAttack` occurrences per eval game.

Per-component TB scalars (`pilot/reward/quick_win_bonus/mean_per_game`, `pilot/reward/stall_penalty/mean_per_game`, `pilot/reward/breeding_digivolve/mean_per_game`, `pilot/reward/digivolve_driven_attack/mean_per_game`) SHALL surface automatically via the existing Group 10 telemetry infrastructure with no new wiring.

#### Scenario: Winning turn scalar emitted when wins occurred

- **WHEN** an eval window has at least one game won by the agent at turn_count values [3, 4, 5]
- **THEN** `pilot/mean_eval_winning_turn` SHALL be emitted with value `4.0`

#### Scenario: Winning turn null when no wins

- **WHEN** an eval window has zero agent wins
- **THEN** `pilot/mean_eval_winning_turn` SHALL NOT be emitted (or emitted as null)

#### Scenario: Per-component scalar for new components surfaces

- **WHEN** an eval window includes games where `quick_win_bonus` contributed values [+5.0, +3.75, +2.5] across 100 total games
- **THEN** `pilot/reward/quick_win_bonus/mean_per_game` SHALL equal `(5.0 + 3.75 + 2.5) / 100 = 0.1125`

### Requirement: TrainingRunMetadata persists gameplay shape

The training-run metadata sidecar (`models/<run>/<run>.meta.json` produced by `TrainingRunMetadata`) SHALL include the following top-level fields:

- `reward_gameplay_path: str` — copy of `cfg.reward_gameplay_path` at run-start.
- `reward_gameplay_hash: str` — canonical content hash of the loaded gameplay.yaml.

These fields SHALL be present on every run, populated from the loaded `Profiles` snapshot at run-start. Downstream tooling that compares paired runs (e.g., baseline vs treatment) reads these fields to determine whether the gameplay shape differed.

#### Scenario: Metadata records gameplay path and hash

- **WHEN** a training run starts with default config
- **THEN** the run's metadata sidecar SHALL include `reward_gameplay_path = "code/digimon_gym/agents/reward/gameplay.yaml"` and `reward_gameplay_hash = "sha256:<hex>"`
