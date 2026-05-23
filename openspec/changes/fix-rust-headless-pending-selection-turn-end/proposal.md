## Why

Rust-backed headless training can soft-lock when a card action crosses memory and installs a mandatory pending selection. The current turn-end path can move the phase to `End` while the pending selection remains unresolved, causing the action mask to expose only pass and the greedy opponent to loop until timeout.

## What Changes

- Preserve pending selections as the active decision whenever a card effect installs one, even if memory has crossed to the opponent's side.
- Defer turn-end rotation until the pending selection and any follow-up effect chain resolves.
- Ensure Rust headless action masks expose pending-selection action IDs while a selection is active.
- Add regression coverage for a Rust headless game where the greedy baseline plays a setup card that creates a mandatory On Play selection, resolves it, and advances to the next player instead of timing out.
- Use DCGO turn/end processing as a behavioral reference for phase flow, while keeping printed card text and repository rules docs as the authority for card-specific behavior.

## Capabilities

### New Capabilities
- `rust-headless-turn-progression`: Headless Rust engine turn progression must preserve player-visible pending decisions and rotate turns only after required choices/effect chains resolve.

### Modified Capabilities

## Impact

- Affected Rust engine areas: `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/game_phases.rs`, `code/digimon-engine/src/action/mask.rs`, `code/digimon-engine/src/action/decode.rs`, and related behavioral tests.
- Affected Python/RL surface: `code/digimon_gym/digimon_gym.py` and `code/digimon_gym/agents/pilot_training.py` only for verification of Rust-backed masks, greedy opponent behavior, and generalist training smoke runs.
- Reference dependency: initialized `DCGO/` submodule source may be inspected for turn/end processing semantics; no production dependency is added.
