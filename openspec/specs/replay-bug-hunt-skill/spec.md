# replay-bug-hunt-skill

## Purpose

Defines the `/replay-bug-hunt` agent workflow that drives the engine MCP's recording-stepping tools to investigate a single recorded game: the Mode 1 differential playbook (DCGO oracle), the Mode 2 judge playbook (faithfulness vs card text + rules), the oracle framing per recording source, and where confirmed findings are routed.

## Requirements

### Requirement: Replay Bug-Hunt Skill

The system SHALL provide a `/replay-bug-hunt` skill that drives the engine MCP's recording-stepping tools to investigate a single recorded game and produce confirmed, localized engine findings. The skill SHALL take a recording path (or a training run + game selector) as input and SHALL select its playbook from the recording source.

#### Scenario: Skill loads a recording and selects a playbook

- **WHEN** the skill is invoked with a recording path
- **THEN** it loads the recording via `load_recording`, reads the reported `source_format`, and proceeds with the Mode 1 playbook for DCGO sources or the Mode 2 playbook for native (self-play / eval) sources

### Requirement: Mode 1 Differential Playbook

For a DCGO-sourced recording, the skill SHALL use DCGO as the oracle. It SHALL run `scan_divergences` to find the first mask-membership / actor / winner / reveal divergence, step to that point, step back to inspect the lead-up state, localize the divergence to a specific card, and confirm the bug against the card's DCGO C# implementation (read from `$BASE_DCGO`) and `general_rule.pdf`. The skill SHALL treat a recorded action the Rust engine masks out as a Rust-engine bug signal, not a recording error, absent evidence otherwise.

#### Scenario: Differential hunt localizes a masked action

- **WHEN** `scan_divergences` reports a `mask_miss` at step N for a recorded digivolve/attack/effect/play
- **THEN** the skill inspects the state at and before step N, identifies the card whose legality the engine got wrong, and records a finding citing the card text, the relevant DCGO C# behavior, and (where applicable) a `general_rule.pdf` rule number

### Requirement: Mode 2 Judge Playbook

For a native self-play / eval recording (no external oracle), the skill SHALL judge faithfulness. It SHALL step through the game, and for each effect-bearing action read the decoded action, the emitted events, and the state delta, then compare what happened against the card's printed text (`inspect_card`), the official rules (`general_rule.pdf`), and DCGO C# behavior. The skill SHALL use `scan_fizzles` and `scan_panics` as leads. It SHALL produce a per-effect verdict (faithful / not-faithful / blocked) and SHALL NOT report an unrevealed card or a known RNG-replay non-determinism as a faithfulness bug.

#### Scenario: Judge evaluates a removal effect

- **WHEN** an eval game contains an action where the agent used a removal/deletion effect
- **THEN** the skill checks whether the effect fired, selected a legal target, and produced the board change the card text mandates, and records a faithful / not-faithful verdict with the source consulted

#### Scenario: Fizzle lead investigated

- **WHEN** `scan_fizzles` reports an `EffectFizzled` at a step where the card text implies the effect should have done something
- **THEN** the skill investigates that step and records whether the fizzle is a faithful no-op or a missing/incorrect implementation

### Requirement: Findings Routing

Confirmed findings SHALL be written to existing trackers: engine-primitive gaps to `docs/RUST_ENGINE_GAPS.md` and card-effect faithfulness gaps to `qa/archetype-qa/engine-gaps.md`. Each finding SHALL record the recording path and step, the divergence kind or verdict, the card involved, and the source consulted (DCGO C# location and/or `general_rule.pdf` rule number). The skill SHALL NOT fix the engine as part of a hunt.

#### Scenario: Finding is recorded in the right tracker

- **WHEN** the skill confirms a card-effect faithfulness gap
- **THEN** it appends an entry to `qa/archetype-qa/engine-gaps.md` naming the recording + step, the card, the verdict, and the consulted source, without modifying engine code

#### Scenario: Engine-primitive gap routed separately

- **WHEN** the skill confirms a missing engine primitive (not a single card's logic)
- **THEN** it appends the gap to `docs/RUST_ENGINE_GAPS.md` instead of the per-card tracker
