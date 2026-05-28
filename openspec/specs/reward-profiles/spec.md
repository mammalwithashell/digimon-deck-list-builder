# reward-profiles Specification

## Purpose
Composable, YAML-defined reward profiles for RL pilot training. Each profile is a list of reward components (with per-component and per-profile budgets) selected per episode via agent-archetype assignment, with a required `_default` fallback, optional inheritance, hot-reload on file change, run-metadata reproducibility hashing, and per-component / per-profile / per-(archetype × component) telemetry. The shipped `_default` profile reproduces the legacy `DigimonEnv._compute_reward` recipe float-for-float; legacy shaping fields on `TrainingConfig` remain as deprecated no-ops in v1.

## Requirements

### Requirement: Reward components are composable units with a uniform interface

The system SHALL define a `RewardComponent` protocol where each component exposes a stable string `name` and a `compute(occurrences, episode_state) -> float` method. Components SHALL be pure with respect to their inputs — given the same `occurrences` list and `episode_state` mapping, `compute` SHALL return the same float. Components SHALL NOT read engine state directly; all signals SHALL be derived from the `occurrences` stream and from `episode_state` (which carries per-episode memory between steps).

The system SHALL ship a component registry mapping each component's `kind` string (as used in `profiles.yaml`) to its implementation class. Loading a profile that references an unknown `kind` SHALL fail at load time with an error listing the registered kinds.

#### Scenario: Component compute is deterministic over occurrences

- **WHEN** the same `RewardComponent` instance is called twice with identical `occurrences` lists and equal `episode_state` mappings
- **THEN** both calls SHALL return the same float

#### Scenario: Unknown component kind fails at load

- **WHEN** `profiles.yaml` declares a profile with a component `kind: not_a_real_component`
- **THEN** profile loading SHALL raise an error whose message names the unknown kind and lists the registered kinds
- **AND** no training run SHALL start with the malformed profile loaded

### Requirement: v1 component catalog

The system SHALL ship the following 12 component kinds in v1, each implemented and registered:

1. `terminal_outcome` — emits the win/loss/draw scalar plus the fast-win bonus curve at episode termination. Parameters: `win_base`, `fast_win_bonus_max`, `fast_win_par_steps`, `loss`, `draw`.
2. `step_penalty` — emits `weight` once per step.
3. `security_remove` — emits `weight × n` per step where `n` is the count of opponent security cards removed this step (zero when none removed).
4. `security_lost` — emits `weight × n` per step where `n` is own security cards lost this step.
5. `digivolve` — emits `weight × n` per step where `n` is the agent's digivolutions performed this step.
6. `dna_digivolve` — emits `weight × n` per step where `n` is the agent's DNA digivolutions this step.
7. `play_named_card` — emits `weight` each step the agent plays a card matching `match` criteria. `match` SHALL support keys `card_id` (exact, string or list), `card_name` (exact, string or list), and `trait` (membership). At least one match key SHALL be present. The component SHALL additionally support optional gating keys: `cost_paid_lt`, `cost_paid_eq`, `cost_paid_gte` (integer comparisons against the event's `cost_paid` field); `cost_paid_lt_printed` and `cost_paid_gte_printed` (boolean — compares `cost_paid` against the event's `cost_printed`); `via_alt_path` (string or list — matches the event's `via_alt_path` Option). When any gating key is set, the component SHALL fire only when ALL gating keys match.
8. `digivolve_into_named_card` — emits `weight` each step the agent digivolves into a card matching `match` criteria. `match` SHALL support keys `card_id`, `card_name`, `trait`. The component SHALL additionally support optional gating keys: `was_dna` (boolean — true / false / unset matches DNA / non-DNA / either), `was_blast_dna` (same shape, narrower flag), and `min_result_level` (integer — restrict to digivolves whose result level is at least N). When any gating key is set, the component SHALL fire only when ALL gating keys match.
9. `memory_swing` — emits `weight × delta` per step where `delta` is signed memory movement toward the agent's side this step.
10. `block_event` — emits `weight × n` per step where `n` is the count of attacks the agent blocked this step. Requires the engine's `GameEvent::Attack` and block-resolution wiring (see `engine-event-emission` capability).
11. `opp_deletion` — emits `weight × n` per step where `n` is the count of opponent Digimon deleted (moved to trash from battle area) this step. Requires `GameEvent::Trash` wiring.
12. `own_deletion` — emits `weight × n` per step where `n` is the count of the agent's own Digimon deleted this step. Requires `GameEvent::Trash` wiring.

Each component SHALL support an optional `once_per_episode: true` flag accepted as a deprecated alias for `budget: { max_fires_per_episode: 1 }`. The loader SHALL emit a `DeprecationWarning` when `once_per_episode` is set and SHALL produce equivalent behavior. In BO3 match-format, "once per episode" means once per match (matching the per-component budget semantics in the budget-controls requirement).

#### Scenario: play_named_card with cost gating fires only on reduced-cost arrivals

- **GIVEN** a profile with `play_named_card { match: {card_id: ["BT17-078"]}, cost_paid_lt_printed: true, weight: 4.0 }`
- **WHEN** the agent plays BT17-078 (printed cost 9) at `cost_paid=0` via the Blast DNA alt-path
- **THEN** the component SHALL emit `+4.0` on that step
- **WHEN** the agent later plays a second BT17-078 at `cost_paid=9` (full hardcast)
- **THEN** the component SHALL emit `0.0` on that step

#### Scenario: digivolve_into_named_card with was_dna gating

- **GIVEN** a profile with `digivolve_into_named_card { match: {card_id: ["BT17-078"]}, was_dna: true, weight: 4.0 }`
- **WHEN** the agent DNA-digivolves into BT17-078 via Blast DNA
- **THEN** the component SHALL emit `+4.0`
- **WHEN** the agent normal-digivolves a Lv6 → BT17-078 by paying the printed evo cost (5 memory)
- **THEN** the component SHALL emit `0.0` (was_dna=false on that event)

#### Scenario: digivolve_into_named_card with min_result_level

- **GIVEN** a profile with `digivolve_into_named_card { match: {trait: "Royal Knight"}, min_result_level: 6, weight: 0.5 }`
- **WHEN** the agent digivolves into a Royal-Knight-trait card at result level 5
- **THEN** the component SHALL emit `0.0`
- **WHEN** the agent later digivolves into a Royal-Knight-trait card at result level 6
- **THEN** the component SHALL emit `+0.5`

#### Scenario: security_remove fires only on the step security count drops

- **WHEN** the agent removes 2 opponent security cards on step `t` and 0 on step `t+1`
- **THEN** the `security_remove` component SHALL return `2 × weight` on step `t` and `0.0` on step `t+1`

#### Scenario: play_named_card with once_per_episode fires once per match in BO3

- **WHEN** a profile includes `play_named_card { match: {card_name: "Omnimon"}, weight: 3.0, once_per_episode: true }`
- **AND** the agent plays Omnimon in game 1 and again in game 2 of the same BO3 match
- **THEN** the component SHALL return `3.0` on the step Omnimon is first played in game 1
- **AND** SHALL return `0.0` on all subsequent steps in games 2 and 3 of that match
- **AND** SHALL again be eligible to return `3.0` on the first Omnimon play of the next BO3 match (after the wrapper's `reset()`)

#### Scenario: terminal_outcome fast-win bonus is linear in remaining steps

- **WHEN** `terminal_outcome` is configured with `win_base=10.0`, `fast_win_bonus_max=5.0`, `fast_win_par_steps=200`
- **AND** the agent wins on step `150`
- **THEN** the component SHALL emit `10.0 + (200 - 150) / 200 × 5.0 = 11.25` on the terminal step

#### Scenario: Occurrences carry pre-resolved result_level and result_traits

- **GIVEN** the `RewardEventBus` derives a `Digivolved` occurrence from a `GameEvent::Digivolve` with `top_card_id = "BT17-078"`
- **WHEN** the occurrence is passed to a component
- **THEN** the occurrence SHALL carry registry-resolved `result_level` (the printed level of BT17-078) and `result_traits` (the printed trait list)
- **AND** the component SHALL NOT need to perform any card-registry lookup itself

### Requirement: Per-component budget controls

Each component instance SHALL support an optional `budget:` sub-key with three controls:

- `max_fires_per_episode: int | null` — hard cap on the number of times this component instance emits a non-zero contribution per Gym episode. In BO3, "per episode" means per match.
- `max_total_per_episode: float | null` — hard cap on the absolute magnitude this component instance contributes per episode. For positive-weight components this is a ceiling; for negative-weight components this is a floor. The clamping respects sign.
- `diminishing_returns_factor: float | null` (default `1.0`) — multiplier applied to each successive fire. The nth fire of a component with weight `w` and factor `f` emits `w × f^(n-1)` before further budget clamping.

Resolution order per fire:

1. Compute base emission `weight × event_multiplier`.
2. Apply `diminishing_returns_factor` based on current per-instance fire count.
3. Apply `max_total_per_episode` clamp (sign-respecting; reduces magnitude toward zero).
4. Apply `max_fires_per_episode` gate (if reached, emission is zero).
5. Apply profile-level cap/floor (per separate requirement).

The `once_per_episode: true` flag is a deprecated alias for `budget: { max_fires_per_episode: 1 }`. The loader SHALL emit a `DeprecationWarning` when `once_per_episode` is set in v1; v2 removes the alias.

Budget state SHALL live in `episode_state` (one entry per component instance keyed by the canonical component key tuple) and SHALL be cleared on each `RewardProfileWrapper.reset()`.

#### Scenario: max_fires_per_episode caps fire count

- **GIVEN** a component with `weight: 0.5` and `budget: { max_fires_per_episode: 2 }`
- **WHEN** the underlying event fires on three separate steps in one episode
- **THEN** the component SHALL emit `+0.5`, `+0.5`, `0.0` (third fire blocked)

#### Scenario: diminishing_returns_factor reduces successive fires

- **GIVEN** a component with `weight: 1.0` and `budget: { diminishing_returns_factor: 0.5 }`
- **WHEN** the underlying event fires four times in one episode
- **THEN** the component SHALL emit `+1.0`, `+0.5`, `+0.25`, `+0.125`

#### Scenario: max_total_per_episode clamps magnitude for positive weight

- **GIVEN** a component with `weight: 1.0` and `budget: { max_total_per_episode: 1.5 }`
- **WHEN** the underlying event fires three times in one episode
- **THEN** the component SHALL emit `+1.0`, `+0.5` (clamped from +1.0), `0.0`

#### Scenario: max_total_per_episode floors magnitude for negative weight

- **GIVEN** a component with `weight: -1.5` and `budget: { max_total_per_episode: -1.5 }`
- **WHEN** the underlying event fires twice in one episode
- **THEN** the component SHALL emit `-1.5`, `0.0`

#### Scenario: once_per_episode emits deprecation warning

- **GIVEN** a profile component declared with `once_per_episode: true` and no `budget:` block
- **WHEN** the profile loads
- **THEN** loading SHALL succeed AND a `DeprecationWarning` SHALL fire referencing `budget.max_fires_per_episode`
- **AND** the component SHALL behave as if `budget: { max_fires_per_episode: 1 }` were set

### Requirement: Per-profile budget cap and floor

The system SHALL support optional top-level profile budgets that bound the per-episode sum of shaping-component contributions. Each profile MAY declare a top-level `budget:` block with two controls:

- `per_episode_cap: float | null` — maximum sum of POSITIVE shaped-component contributions per episode.
- `per_episode_floor: float | null` — maximum sum of NEGATIVE shaped-component contributions per episode (closer to zero than this floor is allowed; further from zero is clamped).

The cap/floor SHALL apply only to "shaping" components (everything except `terminal_outcome` and `step_penalty`). When summing a step's contributions, the wrapper SHALL track per-episode running totals `profile_positive_total` and `profile_negative_total` and SHALL clamp the next contribution so the relevant running total does not exceed the cap/floor. Clamped amounts SHALL be reported in `info["reward_breakdown_clamped"]` per step with keys matching the component names.

When no `budget:` block is present (including the shipped `_default` profile), no profile-level clamping SHALL occur, preserving byte-identical legacy behavior.

#### Scenario: per_episode_cap clamps a positive component's last fire

- **GIVEN** a profile with `budget: { per_episode_cap: 5.0 }` and a single shaping component `dna_digivolve { weight: 4.0 }`
- **WHEN** the agent DNA digivolves on three separate steps in one episode
- **THEN** the per-step component contributions SHALL be `+4.0`, `+1.0` (clamped from +4.0), `0.0`
- **AND** `info["reward_breakdown_clamped"]` on the second step SHALL contain `{"dna_digivolve": 3.0}` (the clamped-away amount)

#### Scenario: terminal_outcome and step_penalty are exempt from per-profile budget

- **GIVEN** a profile with `budget: { per_episode_cap: 1.0 }` and components `terminal_outcome { win_base: 10.0 }`, `step_penalty { weight: -0.001 }`, `dna_digivolve { weight: 4.0 }`
- **WHEN** the agent DNA digivolves once and then wins the game in 100 steps
- **THEN** the cumulative shaped-component reward SHALL be capped at +1.0 (from `dna_digivolve`)
- **AND** the cumulative terminal + step rewards SHALL be `+10.0 + 100 × (-0.001) = +9.9` (uncapped)
- **AND** the total episode return SHALL be `+10.9`

#### Scenario: Default profile has no per-profile budget

- **WHEN** the shipped `_default` profile loads
- **THEN** the resolved profile SHALL have no `budget:` block set
- **AND** no per-profile clamping SHALL occur in any episode using `_default`
- **AND** per-step rewards SHALL match the legacy `_compute_reward` path float-for-float (per the existing default-profile parity requirement)

### Requirement: Key-cards declaration and expansion

A profile MAY include a top-level `key_cards:` list. Each entry SHALL be an object with required field `cards: list[str]` (canonical card IDs) and required field `reward: float`, and MAY include optional fields `diminishing_factor: float` (default `0.4`), `max_per_episode: int | null` (default `null`), `hardcast_penalty: float | null` (default `null`), `hardcast_max_per_episode: int` (default `1`), `alt_path_reward: float | null` (default `null`), and `alt_paths: list[str] | null` (default `null`).

At profile load time, the loader SHALL expand each `key_cards:` entry into 1–3 synthetic component declarations and SHALL insert them into the profile's component list before inheritance resolution runs:

- **Reward component (always)**: `digivolve_into_named_card { match: {card_id: <cards>}, weight: <reward>, budget: { max_fires_per_episode: <max_per_episode>, diminishing_returns_factor: <diminishing_factor> } }`.
- **Hardcast penalty component (only when `hardcast_penalty` is set)**: `play_named_card { match: {card_id: <cards>}, cost_paid_gte_printed: true, weight: <hardcast_penalty>, budget: { max_fires_per_episode: <hardcast_max_per_episode> } }`. The penalty component SHALL NOT inherit `diminishing_factor` from the entry — penalties are hard-capped per `hardcast_max_per_episode` only.
- **Alt-path reward component (only when both `alt_path_reward` AND `alt_paths` are set)**: `play_named_card { match: {card_id: <cards>}, via_alt_path: <alt_paths>, weight: <alt_path_reward>, budget: { max_fires_per_episode: <max_per_episode>, diminishing_returns_factor: <diminishing_factor> } }`.

If `alt_path_reward` is set without `alt_paths` (or vice versa), the loader SHALL fail with a clear error naming the missing field.

Synthetic components from `key_cards:` participate in inheritance, override, and the per-component budget engine identically to hand-written components. Hand-written components MAY coexist with `key_cards:` entries; the two are additive (they expand into separate component instances).

A child profile that declares `key_cards:` SHALL replace its parent's `key_cards:` wholesale (matching the inheritance-replace semantics for components in this spec). To inherit and augment, the child SHALL re-declare the parent's entries.

#### Scenario: Minimal key_cards entry expands into a single component

- **GIVEN** a profile with `key_cards: [ { cards: [BT17-078], reward: 6.0 } ]`
- **WHEN** the profile is resolved
- **THEN** the resolved component list SHALL contain exactly one synthetic `digivolve_into_named_card` with `match: {card_id: [BT17-078]}`, `weight: 6.0`, `budget.diminishing_returns_factor: 0.4`, `budget.max_fires_per_episode: null`
- **AND** the resolved component list SHALL NOT contain any synthetic `play_named_card` components

#### Scenario: Full key_cards entry expands into three components

- **GIVEN** a profile with `key_cards: [ { cards: [BT17-078, AD1-025], reward: 6.0, hardcast_penalty: -1.5, alt_path_reward: 2.0, alt_paths: [assembly] } ]`
- **WHEN** the profile is resolved
- **THEN** the resolved component list SHALL contain three synthetic components:
  - `digivolve_into_named_card { match: {card_id: [BT17-078, AD1-025]}, weight: 6.0, budget: { max_fires_per_episode: null, diminishing_returns_factor: 0.4 } }`
  - `play_named_card { match: {card_id: [BT17-078, AD1-025]}, cost_paid_gte_printed: true, weight: -1.5, budget: { max_fires_per_episode: 1 } }`
  - `play_named_card { match: {card_id: [BT17-078, AD1-025]}, via_alt_path: [assembly], weight: 2.0, budget: { max_fires_per_episode: null, diminishing_returns_factor: 0.4 } }`

#### Scenario: Default reward decay produces big-first, taper-fast schedule

- **GIVEN** a `key_cards:` entry with `reward: 6.0` (and default `diminishing_factor: 0.4`)
- **WHEN** the expanded reward component fires four times in one episode
- **THEN** the component SHALL emit `+6.0`, `+2.4`, `+0.96`, `+0.384` on those four fires
- **AND** the cumulative reward across infinite fires SHALL be bounded above by `6.0 / (1 - 0.4) = 10.0`

#### Scenario: Per-match decay carries across BO3 games

- **GIVEN** `match_format: bo3` and a `key_cards:` entry with `reward: 6.0` (default `diminishing_factor: 0.4`)
- **WHEN** the agent digivolves into a key card in game 1 of a BO3 match
- **AND** then digivolves into a key card in game 2 of the same match
- **THEN** the expanded reward component SHALL emit `+6.0` for the game-1 arrival
- **AND** SHALL emit `+2.4` for the game-2 arrival (decay applied, NOT reset between games)
- **AND** SHALL emit `+6.0` again for the first arrival of the next BO3 match (decay reset on `RewardProfileWrapper.reset()`)

#### Scenario: Hardcast penalty is hard-capped, not decayed

- **GIVEN** a `key_cards:` entry with `hardcast_penalty: -1.5` (default `hardcast_max_per_episode: 1`)
- **WHEN** the agent hardcasts a key card at full cost twice in one episode
- **THEN** the expanded penalty component SHALL emit `-1.5` on the first hardcast
- **AND** SHALL emit `0.0` on the second hardcast (capped by `max_fires_per_episode: 1`)

#### Scenario: alt_path_reward without alt_paths fails at load

- **GIVEN** a `key_cards:` entry with `alt_path_reward: 2.0` and no `alt_paths` field
- **WHEN** the profile is loaded
- **THEN** loading SHALL fail with an error naming the missing `alt_paths` field

#### Scenario: Child key_cards replaces parent key_cards wholesale

- **GIVEN** a parent profile with `key_cards: [ { cards: [BT17-078], reward: 6.0 } ]` and a child inheriting from it with `key_cards: [ { cards: [AD1-025], reward: 8.0 } ]`
- **WHEN** the child profile is resolved
- **THEN** the resolved component list SHALL contain a synthetic `digivolve_into_named_card` for `AD1-025` at weight `8.0`
- **AND** SHALL NOT contain any synthetic component referencing `BT17-078`

#### Scenario: Hand-written components coexist with key_cards entries

- **GIVEN** a profile with `key_cards: [ { cards: [BT17-078], reward: 6.0 } ]` AND a hand-written `block_event { weight: 0.15 }` component
- **WHEN** the profile is resolved
- **THEN** the resolved component list SHALL contain BOTH the synthetic `digivolve_into_named_card` for BT17-078 AND the hand-written `block_event`

### Requirement: Boss-cards set and arrival-aware sidecar columns

Each profile SHALL have a derived boss-cards set, computed as the union of every `key_cards:` entry's `cards` list. Profiles with no `key_cards:` block SHALL have an empty boss-cards set.

The pilot training `evals.jsonl` row SHALL include per-game arrival-aware columns derived from the active profile's boss-cards set. These columns SHALL be populated for every game regardless of which profile was active (including `_default` games, which carry zero in every boss-arrival column because `_default` has no `key_cards:`):

- `digivolves_into_boss_agent: int` — count of `Digivolved` occurrences in this game where the result `top_card_id` is in the active profile's boss-cards set.
- `digivolves_into_boss_dna_agent: int` — same, restricted to `was_dna = true` occurrences.
- `hardcasts_of_boss_agent: int` — count of `PlayedCard` occurrences whose `card_id` is in the boss-cards set.
- `hardcasts_of_boss_full_cost_agent: int` — same, restricted to `cost_paid >= cost_printed`.
- `digivolve_discipline_agent: float | null` — equals `digivolves_into_boss_agent / (digivolves_into_boss_agent + hardcasts_of_boss_agent)` when the denominator is non-zero; `null` otherwise.
- `reward_profile_id: str` — the active profile name for this game.
- `reward_profile_hash: str` — the canonical hash of the profile content actually used for this game (records the post-reload value if hot-reload triggered before this game; see hot-reload requirement).

The pilot training `WinRateCallback` SHALL emit three corresponding TB scalars aggregating across the eval window:

- `pilot/mean_eval_digivolves_into_boss_per_game` — mean of `digivolves_into_boss_agent` over all games in the eval window.
- `pilot/mean_eval_hardcasts_of_boss_per_game` — mean of `hardcasts_of_boss_agent`.
- `pilot/mean_eval_digivolve_discipline` — mean of `digivolve_discipline_agent` over games where it is non-null.

The callback SHALL also emit `pilot/profile/<profile>/clamp_share` — the fraction of steps in games using this profile where the per-profile cap or floor clamped at least one component contribution (sourced from `info["reward_breakdown_clamped"]`).

#### Scenario: Boss-cards set derives from key_cards declarations

- **GIVEN** a profile with `key_cards: [ { cards: [BT17-078, AD1-025], reward: 6.0 }, { cards: [BT22-015], reward: 4.0 } ]`
- **WHEN** the profile is resolved
- **THEN** the profile's boss-cards set SHALL equal `{BT17-078, AD1-025, BT22-015}`

#### Scenario: Profile without key_cards has empty boss-cards set

- **WHEN** the shipped `_default` profile is resolved
- **THEN** the profile's boss-cards set SHALL be empty
- **AND** every eval-row column derived from the boss-cards set SHALL equal `0` for games using `_default`
- **AND** `digivolve_discipline_agent` SHALL equal `null`

#### Scenario: digivolve_discipline_agent is null when no boss interaction occurred

- **WHEN** a game using a profile with non-empty boss-cards completes with zero digivolves-into-boss and zero hardcasts-of-boss
- **THEN** the eval row SHALL have `digivolves_into_boss_agent = 0`, `hardcasts_of_boss_agent = 0`, and `digivolve_discipline_agent = null`

#### Scenario: digivolve_discipline_agent computes correctly

- **WHEN** a game completes with `digivolves_into_boss_agent = 3` and `hardcasts_of_boss_agent = 1`
- **THEN** the eval row SHALL have `digivolve_discipline_agent = 0.75` (`3 / (3 + 1)`)

#### Scenario: Boss-arrival columns populated for _default profile games

- **GIVEN** a generalist run mixing `_default` and `dna_omnimon_combo_v1` profiles across eval games
- **WHEN** the eval rows are written
- **THEN** every row SHALL contain all six boss-arrival columns
- **AND** `_default`-profile rows SHALL have `digivolves_into_boss_agent = 0`, `digivolve_discipline_agent = null`
- **AND** `dna_omnimon_combo_v1`-profile rows SHALL count toward the boss-cards set declared in that profile

#### Scenario: clamp_share reflects per-profile budget clamping frequency

- **GIVEN** a profile with `budget: { per_episode_cap: 5.0 }` and components that consistently exceed the cap
- **WHEN** 100 eval games run with this profile and 30% of total steps trigger clamping
- **THEN** `pilot/profile/<profile>/clamp_share` SHALL equal approximately `0.30` for that eval window

### Requirement: Hot reload of profiles file on episode boundaries

The system SHALL re-read `reward_profiles_path` at each `RewardProfileWrapper.reset()` and re-parse the file when its mtime is newer than the last loaded mtime. The reload SHALL apply to subsequent episodes — the active profile resolved at `reset()` time SHALL remain in effect for that entire episode regardless of further mid-episode file changes.

When `TrainingConfig.reward_profiles_hot_reload` is `False`, the system SHALL NOT re-stat the file and SHALL retain the profiles loaded at run-start for the entire run.

When hot reload triggers a reload, the system SHALL recompute the canonical content hash AND SHALL include the new hash in the per-game `evals.jsonl` row's `reward_profile_hash` field (introduced by `add-per-game-eval-log`). The `models/<run>/metadata.json::reward_profiles_hash` field SHALL remain the RUN-START hash, not the latest reload hash; the resume-check (see reproducibility requirement) compares against this run-start value.

If a hot reload produces a file that fails to parse, the system SHALL log a warning and SHALL retain the previously-loaded profiles. The next reset SHALL re-attempt the load (it will succeed once the file parses again).

#### Scenario: mtime advance triggers reload at next reset

- **GIVEN** `reward_profiles_hot_reload = true` and a running training job
- **WHEN** the operator edits and saves `profiles.yaml` between two episodes
- **AND** the next `env.reset()` is called
- **THEN** the file SHALL be re-parsed
- **AND** the new episode SHALL use the updated profile resolution

#### Scenario: mid-episode edits do not apply mid-game

- **GIVEN** `reward_profiles_hot_reload = true`
- **WHEN** an episode is in progress (after `reset()`, before terminal)
- **AND** the operator saves a profile change mid-episode
- **THEN** the running episode SHALL continue using the profile resolved at its `reset()` call
- **AND** the change SHALL take effect at the next `reset()`

#### Scenario: parse failure preserves previous profiles

- **GIVEN** `reward_profiles_hot_reload = true` and a successfully-loaded profile set
- **WHEN** the operator saves a malformed YAML to `profiles.yaml`
- **AND** the next `env.reset()` is called
- **THEN** the wrapper SHALL log a warning naming the parse error
- **AND** the previously-loaded profiles SHALL remain active
- **AND** the next `env.reset()` after the YAML is fixed SHALL successfully reload

#### Scenario: hot reload disabled retains run-start profiles

- **GIVEN** `reward_profiles_hot_reload = false`
- **WHEN** the operator edits `profiles.yaml` mid-run
- **AND** any subsequent `env.reset()` is called
- **THEN** the file SHALL NOT be re-stat()ed
- **AND** the run SHALL continue using the run-start profile resolution unchanged

#### Scenario: per-game sidecar records the actual hash used per game

- **GIVEN** `reward_profiles_hot_reload = true` and a run that reloads profiles between game 5 and game 6 of an eval window
- **WHEN** the `evals.jsonl` rows are written for that eval window
- **THEN** games 1-5 SHALL carry the pre-reload `reward_profile_hash` value
- **AND** games 6-N SHALL carry the post-reload `reward_profile_hash` value

### Requirement: Reward profiles are defined in versioned YAML with inheritance

The system SHALL load reward profiles from a YAML file (default `code/digimon_gym/agents/reward/profiles.yaml`) with two top-level keys: `profiles` (mapping of profile name to definition) and `assignments` (mapping of archetype name to profile name).

Each profile definition SHALL contain a `components:` list and MAY contain an `inherits:` string naming another profile in the same file. When `inherits` is present, the child profile's components SHALL start from a copy of the parent's resolved components and then apply each child component as follows:

- If the child declares a component with a `kind` and key-parameter tuple matching a parent component, the child's declaration SHALL replace the parent's wholesale (not sum, not merge).
- If the child declares a component whose `(kind, key-params)` tuple does not match any parent component, the child's declaration SHALL be appended.

Key parameters per kind:

- `play_named_card`: `match` dict (full match contents).
- `terminal_outcome`: nothing (at most one per profile).
- `step_penalty`, `security_remove`, `security_lost`, `digivolve`, `dna_digivolve`, `memory_swing`, `block_event`, `opp_deletion`, `own_deletion`: nothing (at most one each per profile).

Inheritance SHALL be single-parent only. Cycles SHALL fail at load time with a clear error.

Profile names beginning with `_` SHALL be treated as private. Private profiles MAY be referenced in `inherits:` but SHALL NOT appear as values in the `assignments` map.

#### Scenario: Child overrides parent component weight

- **GIVEN** a parent profile with `security_remove { weight: 1.5 }` and a child profile inheriting from it with `security_remove { weight: 3.0 }`
- **WHEN** the child profile is resolved
- **THEN** the resolved `security_remove` weight SHALL be `3.0` (not `4.5`)

#### Scenario: Multiple play_named_card with different matches coexist

- **GIVEN** a profile with two `play_named_card` components, one matching `card_name: "Omnimon"` and one matching `card_name: "Imperialdramon"`
- **WHEN** the profile is resolved
- **THEN** both components SHALL be present in the resolved component list
- **AND** the agent SHALL receive rewards for plays of either named card

#### Scenario: Inheritance cycle fails at load

- **GIVEN** profiles `a` inherits from `b`, `b` inherits from `a`
- **WHEN** the profiles file is loaded
- **THEN** loading SHALL fail with an error naming the cycle

#### Scenario: Private profile cannot be assigned

- **GIVEN** a profile `_base_terminal` and an assignment `assignments: { "Rocks": _base_terminal }`
- **WHEN** the profiles file is loaded
- **THEN** loading SHALL fail with an error explaining that profiles starting with `_` cannot be assigned

### Requirement: Profile assignment is agent-archetype-keyed with required default fallback

The system SHALL select the active profile per episode by reading `info["deck1_archetype"]` (set by `GeneralistDeckPoolWrapper` or `DeckPoolWrapper`) and looking it up in the `assignments` map. If `deck1_archetype` is absent from `info`, or if its value is not in the `assignments` map, the system SHALL use the profile named by `assignments["_default"]`.

The `assignments` map SHALL contain a `_default` key. Loading a profiles file without a `_default` assignment SHALL fail at load time.

When `TrainingConfig.reward_profile_override` is set to a non-null profile name, the system SHALL use that profile for every episode regardless of `info["deck1_archetype"]`. The override SHALL NOT bypass the existence check — an override naming a non-existent profile SHALL fail at load.

#### Scenario: Archetype maps to its assigned profile

- **GIVEN** `assignments: { "_default": "_default", "DNA Omnimon": "dna_omnimon_combo_v1" }`
- **WHEN** `info["deck1_archetype"] = "DNA Omnimon"` at episode start
- **THEN** the active profile for that episode SHALL be `dna_omnimon_combo_v1`

#### Scenario: Missing archetype falls back to default

- **WHEN** `info["deck1_archetype"]` is absent (gauntlet mode with no generalist wrapper)
- **THEN** the active profile for that episode SHALL be the profile named by `assignments["_default"]`

#### Scenario: Override bypasses archetype lookup

- **GIVEN** `TrainingConfig.reward_profile_override = "rocks_aggro_v1"`
- **WHEN** the episode starts with `info["deck1_archetype"] = "DNA Omnimon"`
- **THEN** the active profile SHALL be `rocks_aggro_v1` (not `dna_omnimon_combo_v1`)

#### Scenario: Missing _default fails at load

- **WHEN** a profiles file is loaded with no `_default` key in `assignments`
- **THEN** loading SHALL fail with an error naming the missing key

### Requirement: RewardProfileWrapper sits between DigimonEnv and OpponentWrapper

The system SHALL provide a `RewardProfileWrapper` Gymnasium wrapper that wraps `DigimonEnv` directly. The wrapper SHALL:

- On `reset()`: call the wrapped env's reset, read `info["deck1_archetype"]` from the returned info, resolve the active profile (with override and default fallback), initialize a fresh `episode_state` dict, and write `info["reward_profile"]` (the resolved profile name).
- On `step(action)`: call the wrapped env's step, collect the env's emitted `occurrences` list from the returned info (under `info["reward_occurrences"]`), invoke each component in the active profile's component list (passing `occurrences` and `episode_state`), compute the weighted sum as the new reward, write `info["reward_breakdown"]` (mapping component name to its contribution this step) and `info["reward_profile"]` (the active profile name), and return the new scalar reward.

The wrapper SHALL NOT modify the observation, terminated, or truncated values returned by the wrapped env.

The wrapper SHALL replace the env's scalar reward with the component-derived sum. Downstream wrappers (including `OpponentWrapper`) SHALL receive only the post-profile reward.

#### Scenario: Profile activates at reset based on deck1_archetype

- **GIVEN** profile assignments mapping "Rocks" to `rocks_aggro_v1`
- **WHEN** `env.reset()` returns info with `deck1_archetype="Rocks"`
- **THEN** `info["reward_profile"]` SHALL equal `rocks_aggro_v1`
- **AND** subsequent `step()` calls SHALL apply the `rocks_aggro_v1` profile's component weights

#### Scenario: reward_breakdown sums to step reward

- **WHEN** a single step writes `info["reward_breakdown"] = {"step_penalty": -0.001, "security_remove": 3.0, "terminal_outcome": 0.0}`
- **THEN** the scalar reward returned for that step SHALL equal `−0.001 + 3.0 + 0.0 = 2.999`

#### Scenario: Profile is stable across BO3 games within a match

- **GIVEN** `match_format=bo3` and `deck1_archetype="DNA Omnimon"`
- **WHEN** a BO3 match plays through games 1, 2, and 3
- **THEN** `info["reward_profile"]` SHALL equal the same profile name across all steps of all three games

### Requirement: Per-component, per-profile, and per-(archetype × component) telemetry

The pilot training `WinRateCallback` SHALL aggregate three tiers of reward-decomposition telemetry, accumulated cumulatively from callback construction (matching the existing `_archetype_wins` lifecycle, with no cross-resume persistence).

**Tier 1 (TensorBoard, per-component global)**: For every component name that appeared in any eval game since the callback was constructed, the callback SHALL emit a TB scalar `pilot/reward/<component>/mean_per_game` equal to `total_contribution_for_component / total_eval_games`.

**Tier 2 (TensorBoard, per-profile aggregate)**: For every profile name that was active in any eval game, the callback SHALL emit:

- `pilot/profile/<profile_name>/mean_reward` — sum of all step rewards in games using this profile, divided by the count of those games.
- `pilot/profile/<profile_name>/share_<component>` — sum of that component's contribution in games using this profile, divided by the profile's `mean_reward × games` (i.e., the component's fractional share of total reward under that profile). Emitted only when the profile's total reward is non-zero.

**Tier 3 (evals.jsonl sidecar, per-archetype × component)**: Each `evals.jsonl` row SHALL extend its existing `by_archetype` value object with a `component_means` sub-map: `{<component_name>: <mean_contribution_per_game>}` keyed by opponent archetype (matching existing semantics). The row SHALL also include a top-level `by_agent_archetype` map with the same shape, keyed by agent archetype, when `deck1_archetype` was present.

Component contributions SHALL be sourced from `info["reward_breakdown"]` written by `RewardProfileWrapper`. Counters SHALL be incremented at the per-game post-terminal hook, summing the breakdowns across all steps of that game.

#### Scenario: Per-component scalar accumulates across eval cycles

- **WHEN** three eval cycles run, producing total `security_remove` contribution of `7.5`, `9.0`, `6.0` across 50 + 50 + 50 = 150 games
- **THEN** the final cycle's `pilot/reward/security_remove/mean_per_game` SHALL equal `(7.5 + 9.0 + 6.0) / 150 = 0.15`

#### Scenario: Per-profile share is fractional

- **GIVEN** a profile whose total reward across 20 games is `200.0` and whose `security_remove` component contributed `80.0` of that
- **THEN** `pilot/profile/<profile>/share_security_remove` SHALL equal `80.0 / 200.0 = 0.4`

#### Scenario: Sidecar carries per-archetype component drilldown

- **WHEN** an eval row is written after 10 games against opponent "Royal Knights" with `security_remove` totaling `12.0` and `block_event` totaling `4.0` across those games
- **THEN** the row's `by_archetype["Royal Knights"].component_means` SHALL include `{"security_remove": 1.2, "block_event": 0.4}`

#### Scenario: Resumed run starts fresh component tallies

- **WHEN** a training run is checkpointed and resumed in a new process with a fresh `WinRateCallback`
- **THEN** the per-component, per-profile, and per-(archetype × component) accumulators SHALL start at zero

### Requirement: Run reproducibility via profile name and content hash

The pilot training `models/<run>/metadata.json` SHALL include four reward-profile fields:

- `reward_profiles_path` — the file path the profiles were loaded from.
- `reward_profiles_hash` — `sha256:` prefix followed by the SHA-256 hex of the canonicalized profile content (parsed YAML re-serialized with sorted keys and normalized number formatting, NOT the raw file bytes).
- `reward_profile_override` — the value of `TrainingConfig.reward_profile_override` (`null` when archetype-driven).
- `reward_assignments_snapshot` — a copy of the resolved `assignments` map at run-start time.

On resume (`TrainingConfig.resume_from` non-null), the system SHALL load the resumed checkpoint's metadata, compute the current profiles' canonical hash, and if the two hashes differ SHALL raise an error naming both hashes. The error SHALL instruct the user to pass `--reward-profiles-override-mismatch` to proceed. With that flag, the resume SHALL succeed and the new metadata SHALL record the new hash.

The hash SHALL be insensitive to whitespace, YAML key ordering, and trailing comments — re-saving the file with a YAML formatter SHALL NOT trigger a mismatch.

#### Scenario: Metadata records resolved profile state

- **WHEN** a training run starts with `reward_profiles_path = "code/digimon_gym/agents/reward/profiles.yaml"` and `reward_profile_override = null`
- **THEN** `models/<run>/metadata.json` SHALL include `reward_profiles_path`, a `reward_profiles_hash` starting with `sha256:`, `reward_profile_override = null`, and a `reward_assignments_snapshot` matching the file's `assignments` map

#### Scenario: Resume fails loudly on hash mismatch

- **WHEN** a run is checkpointed, the profiles file is edited to change a component weight, and resume is attempted without the override flag
- **THEN** the resume SHALL fail with an error naming both the checkpoint hash and the current hash
- **AND** the run SHALL NOT advance past the resume check

#### Scenario: Cosmetic YAML edits do not trigger mismatch

- **WHEN** a run is checkpointed and the profiles file is re-saved with different key order and added whitespace but no semantic changes
- **THEN** resume SHALL succeed without warning
- **AND** the recomputed hash SHALL equal the checkpoint's hash

### Requirement: Deprecation path for legacy reward-shaping fields

The `TrainingConfig` fields `digivolve_reward` and `dna_digivolve_bonus` SHALL remain present in this change but SHALL be unread by the reward computation pipeline (the loaded gameplay + profile components drive all shaping). Setting either field to a non-default value SHALL emit a `DeprecationWarning` from `TrainingConfig._validate` directing the user to define a custom profile or edit `gameplay.yaml`.

The `digivolve_shaping` boolean SHALL remain accepted with no warning, but it SHALL become **inert** in this change — it no longer maps to a `_digivolve_shaped` profile (that profile is removed). Setting `digivolve_shaping=True` in a config has no effect on the active reward shape; the gameplay default already includes digivolve weights universally.

The v1 deprecation timeline (warning in v1, removal in v2) for `digivolve_reward` / `dna_digivolve_bonus` is unchanged. `digivolve_shaping` is reserved for v2 removal alongside.

This change SHALL NOT remove the fields. Their removal is reserved for a follow-up proposal.

#### Scenario: Non-default digivolve_reward still fires deprecation warning

- **WHEN** `TrainingConfig(digivolve_reward=0.5)` is constructed
- **THEN** a `DeprecationWarning` SHALL be raised by `_validate`
- **AND** the warning message SHALL reference editing `gameplay.yaml` or defining a custom profile

#### Scenario: Default values do not warn

- **WHEN** `TrainingConfig()` is constructed with the default `digivolve_reward = 0.1` and `dna_digivolve_bonus = 3.9`
- **THEN** no `DeprecationWarning` SHALL fire for those fields

#### Scenario: digivolve_shaping=True is now inert (no profile mapping)

- **WHEN** `TrainingConfig(digivolve_shaping=True)` is constructed with otherwise-default values
- **THEN** no warning SHALL fire (preserves v1 contract)
- **AND** the active profile SHALL resolve via the standard archetype/override path (NOT to `_digivolve_shaped`, which no longer exists)
- **AND** training SHALL proceed under the new universal gameplay shape

### Requirement: Profile loader cross-file inheritance

The `ProfileLoader` SHALL support inheritance chains that cross file boundaries. When a profile in `profiles.yaml` declares `inherits: gameplay`, the loader SHALL resolve that reference against the merged namespace (`gameplay.yaml`'s profiles + `profiles.yaml`'s profiles).

Override semantics, key-params override de-dup, and `key_cards:` expansion SHALL all work transparently across file boundaries — a child profile in `profiles.yaml` can override components inherited from a parent in `gameplay.yaml`.

Profile name collisions across the two files SHALL fail at parse time (per the gameplay-reward-config capability's "Two-file loader merges namespaces" requirement). Inheritance cycles SHALL fail at parse time with a cycle path that names all involved profiles regardless of which file they live in.

#### Scenario: Cross-file inheritance resolves correctly

- **GIVEN** `gameplay.yaml` defines profile `gameplay` with `security_remove { weight: 1.5 }`
- **AND** `profiles.yaml` defines `aggressive` with `inherits: gameplay` and override `security_remove { weight: 3.0 }`
- **WHEN** the loader resolves `aggressive`
- **THEN** the resolved component list SHALL include `security_remove` with `weight: 3.0` (child override wins)
- **AND** all other components from `gameplay` SHALL be inherited unchanged
