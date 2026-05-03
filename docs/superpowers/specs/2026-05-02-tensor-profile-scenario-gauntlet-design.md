# Tensor Profile Scenario Gauntlet Design

## Summary

Extend the tensor profile gauntlet with YAML-authored scenario fixtures that describe constructed board states and expected agent-visible decisions. The first version focuses on general tactical situations such as attack for lethal, block lethal, hatch before pass, and play/search for hand fixing. Later versions can reuse the same expectation model for archetype-specific matchup scenarios and for scenarios built by replaying prior actions through the engine.

This design keeps the current tensor profile benchmark intact. The existing gauntlet still measures throughput, win rate versus greedy, trigger-order evidence, and memory footprint across profiles. Scenario fixtures add a second evaluation mode that asks a more concrete question: for a known board state, does each profile expose the legal choice and enough tensor signal for the expected response?

## Goals

- Let contributors write small YAML fixtures for agent decision scenarios.
- Use constructed Rust-engine board states for v1 so scenarios are fast, deterministic, and easy to review.
- Score profile behavior on expected legal actions, expected policy/oracle action, pending-choice visibility, and tensor section evidence.
- Start with general tactical tests before moving to archetype-specific matchup tests.
- Keep the scenario system standalone under the agent stack, with no FastAPI, database, auth, or hosted pipeline imports.
- Preserve the no-approximations policy: expected decisions must come from legal engine actions and pending-selection contracts, not hidden helper choices.

## Non-Goals

- v1 does not replay a full game history to produce the state.
- v1 does not create a new card-effect DSL.
- v1 does not replace Rust behavioral tests or Python legacy scenario tests.
- v1 does not train or update models; it only evaluates profile/action visibility and optional policy choices.
- v1 does not assert strategic perfection for every archetype. General tactical scenarios come first.

## File Layout

Scenario fixtures should live under source-controlled test data, separate from production card YAML:

```text
code/tests/fixtures/tensor_profile_scenarios/
  general/
    attack_for_lethal.yaml
    block_lethal_no_security.yaml
    hatch_before_pass.yaml
    play_searcher_for_hand_fixing.yaml
  archetypes/
    <archetype>/
      <matchup>/
        <scenario>.yaml
```

The runner and parser should live with standalone agent code:

```text
code/digimon_gym/agents/tensor_profile_scenarios.py
code/tools/profile_tensor_scenarios.py
```

The existing benchmark runner remains in:

```text
code/digimon_gym/agents/tensor_profile_gauntlet.py
code/tools/profile_tensor_profiles.py
```

## Scenario YAML Shape

Each fixture describes metadata, setup, and expectations.

```yaml
id: attack_for_lethal
tier: general
description: P1 has an unsuspended attacker and P2 has no security.
tags:
  - lethal
  - policy_priority

profiles:
  - compact_v1
  - standard_lite_v2
  - standard_full_v2

setup:
  backend: rust
  current_player: 1
  phase: main
  memory: 3
  player1:
    security: 3
    breeding: []
    hand: []
    field:
      - label: attacker
        card: ST1-03
        suspended: false
        can_attack: true
        turn_played: -1
  player2:
    security: 0
    breeding: []
    hand: []
    field: []

expect:
  legal_actions:
    include:
      - kind: attack_player
        source: attacker
  preferred_action:
    kind: attack_player
    source: attacker
  tensor:
    action_mask:
      include_preferred_action: true
```

Blocking lethal uses the same structure but includes a pending response window:

```yaml
id: block_lethal_no_security
tier: general
description: P2 must be able to block a lethal attack while at zero security.
tags:
  - defensive_response
  - trigger_order
  - lethal

setup:
  backend: rust
  current_player: 2
  phase: counter
  memory: 0
  pending_attack:
    attacker_player: 1
    source: attacker
    target: player
  player1:
    security: 2
    field:
      - label: attacker
        card: ST1-03
        suspended: true
        turn_played: -1
  player2:
    security: 0
    field:
      - label: blocker
        card: ST1-07
        suspended: false
        turn_played: -1

expect:
  pending_choice:
    kind: block
  legal_actions:
    include:
      - kind: block
        source: blocker
  preferred_action:
    kind: block
    source: blocker
  tensor:
    pending_choice_features: nonzero
    action_id_features:
      include:
        kind: block
        prompt_flag: true
```

## Setup Semantics

The scenario parser should convert the YAML into a Rust-backed headless game state. It should force `DIGIMON_BACKEND=rust` while building and evaluating the scenario, then restore the previous environment variable.

Card references use production card IDs. Labels are local aliases used by expectations. The runner resolves labels to engine entities after constructing the state.

The v1 setup vocabulary should stay intentionally small:

- `current_player`
- `phase`
- `memory`
- `player1.security`, `player2.security`
- `player*.hand`
- `player*.breeding`
- `player*.field`
- `player*.deck`, optional for search scenarios
- `pending_attack`, for block and response-window scenarios

Search scenarios need enough deck ordering to make expected hand fixing deterministic:

```yaml
id: play_searcher_for_hand_fixing
tier: general
tags:
  - hand_fixing
  - search
  - policy_priority

setup:
  backend: rust
  current_player: 1
  phase: main
  memory: 5
  player1:
    hand:
      - label: searcher
        card: EXAMPLE-SEARCHER
    deck:
      top:
        - ST1-03
        - ST1-03
        - ST1-07
    field: []
  player2:
    security: 5

expect:
  legal_actions:
    include:
      - kind: play
        source: searcher
  preferred_action:
    kind: play
    source: searcher
  tensor:
    hand_features: nonzero
    action_mask:
      include_preferred_action: true
```

If a needed card is not implemented in Rust, the runner should mark the scenario unavailable with a reason rather than silently substituting another card.

## Expectations

Expectations should be declarative and tied to engine-visible facts.

`legal_actions.include` verifies that the engine action mask contains an action matching the requested semantic pattern. This is mandatory for every scenario.

`preferred_action` verifies that the selected oracle or policy chooses the expected action. For v1, the default oracle can be a deterministic scenario oracle that matches semantic action descriptors against the legal action list. The greedy policy can be evaluated as an optional policy under test.

`pending_choice` verifies that the engine exposes a pending decision of the expected kind. This is required for trigger-order scenarios and omitted for normal main-phase priority scenarios.

`tensor` verifies profile-specific visibility. Examples:

- `pending_choice_features: nonzero`
- `action_id_features.include.kind`
- `action_id_features.include.prompt_flag`
- `action_mask.include_preferred_action`
- `hand_features: nonzero`
- `board_features.include.source`

Tensor expectations should avoid asserting raw offsets in YAML. The runner should use the active tensor profile registry and section metadata to interpret expectations.

## Scoring

Scenario results should be reported per profile and per scenario. The initial aggregate columns should be:

- `scenario_count`
- `available_count`
- `legal_action_accuracy`
- `preferred_action_accuracy`
- `pending_choice_accuracy`
- `tensor_signal_accuracy`
- `scenario_oracle_accuracy`

The existing `trigger_order_accuracy` metric should remain on the benchmark gauntlet. Scenario results can feed a richer trigger-specific breakdown:

- `trigger_pending_visible`
- `trigger_prompt_action_visible`
- `trigger_preferred_response`

This separation matters. A profile can expose the correct pending-choice signal while a policy still chooses the wrong legal action, or a policy can choose correctly even when the profile has too little structured signal for learning.

## CLI

Add a new CLI for scenario evaluation:

```powershell
python code/tools/profile_tensor_scenarios.py `
  --profiles compact_v1,standard_lite_v2,standard_full_v2 `
  --scenarios code/tests/fixtures/tensor_profile_scenarios/general `
  --out profile_runs/tensor_scenarios/latest `
  --require-profiles
```

Useful filters:

- `--tags lethal,trigger_order`
- `--tier general`
- `--archetype Greymon`
- `--matchup MirageGaogamon`
- `--policy oracle`
- `--policy greedy`

Outputs should mirror the existing profile gauntlet:

```text
result.json
result.md
```

JSON should include every scenario result and skip reason. Markdown should include a compact comparison table plus a failure section grouped by scenario.

## General Scenario Set

The first fixture set should cover:

1. `attack_for_lethal`: current player can legally attack the opponent at zero security and should do so.
2. `block_lethal_no_security`: defending player has zero security and a legal blocker; block should be visible and preferred.
3. `hatch_before_pass`: breeding phase has an available egg hatch and pass is also legal; hatch should be preferred.
4. `play_searcher_for_hand_fixing`: hand lacks a clean line and contains a legal searcher; playing the searcher should be preferred.
5. `keep_turn_digivolve_before_pass`: a legal digivolution preserves turn; digivolve should be preferred over pass.
6. `avoid_bad_attack_without_lethal`: attack is legal but not lethal and loses to board/security context; safer progression should be preferred.

The first implementation can land the first four. The latter two are good follow-up fixtures once the action matcher and board construction helpers are stable.

## Archetype Matchup Extension

Archetype scenarios should use the same schema with additional metadata:

```yaml
tier: archetype
archetype: Greymon
matchup: MirageGaogamon
deck1_source: data/deck_library.json
deck2_source: data/deck_library.json
```

These scenarios should still construct the board directly in v1. The archetype metadata tells the runner how to validate card availability, produce reports, and group results. It does not require full deck simulation.

The archetype path should support questions like:

- Does `standard_lite_v2` expose the relevant hand/board/search features for this archetype?
- Does `standard_full_v2` expose richer action-row context for this matchup?
- Does compact miss a matchup-specific pending-choice signal?

## Follow-Up: Prior Action Replay

Constructed board states are the right v1 because they are small, deterministic, and easy to author. A later version should allow scenarios to include `prior_actions`:

```yaml
prior_actions:
  - player: 1
    action:
      kind: hatch
  - player: 2
    action:
      kind: pass
  - player: 1
    action:
      kind: play
      source: rookie
```

The runner would start from a normal deck/seed, replay those actions through the Rust engine, then apply the same expectations. This gives stronger end-to-end realism and catches bugs in transition logic, but it is more brittle while the action matcher and scenario vocabulary are still new.

`prior_actions` should therefore be a schema-reserved field in v1. The parser may reject it with a clear "not implemented yet" error, or accept it only behind an experimental flag.

## Validation And Error Handling

The parser should fail fast for malformed YAML:

- missing `id`
- unknown `tier`
- missing `setup`
- missing `expect.legal_actions.include`
- duplicate labels within a player zone
- references to unknown labels
- unsupported expectation keys

Runtime unavailability should not crash the entire batch:

- unknown profile
- missing Rust card implementation
- unsupported state construction field
- no legal action matching the expectation

Each unavailable scenario/profile pair should produce a structured skip reason in JSON and Markdown.

## Tests

Unit tests should cover:

- YAML parsing and validation errors.
- Label resolution.
- Semantic action matcher behavior.
- Tensor expectation evaluation against fake profiles and synthetic observations.
- Skip handling for unavailable profiles/cards.
- Markdown and JSON output.

Integration smoke tests should cover:

- One constructed lethal attack scenario with the Rust backend.
- One constructed blocker/pending-choice scenario if the Rust engine currently exposes the needed pending response state.
- CLI run over a tiny fixture directory.

If blocker pending-state construction is not available immediately, the fixture can land as skipped with an explicit reason, and the test should assert that the skip is reported correctly.

## Open Decisions

- Which real implemented blocker card should be the default for `block_lethal_no_security`.
- Which implemented searcher should be the default for `play_searcher_for_hand_fixing`.
- Whether `preferred_action` should always be required or optional for pure tensor-visibility scenarios.
- Whether scenario reports should be folded into the existing tensor profile gauntlet CLI or remain a separate CLI. The recommended v1 is a separate CLI to keep benchmark games and constructed scenario assertions easy to reason about.

