## Why

Pilot agents currently see live board state, own hand, known zones, pending decisions, and deck sizes, but they do not see the composition of the deck they are piloting. Providing the agent with its own original decklist gives it stable strategic context for planning without revealing shuffled deck order, face-down security identity, or opponent private information.

## What Changes

- Add a new observation profile named `standard_lite_deck_v2`.
- Base the new profile on `standard_lite_v2` and append an own-original-decklist section.
- Encode the observing pilot's submitted deck as sorted unique card rows with card ID and original count.
- Include main-deck and Digi-Egg flags for each decklist row.
- Keep `standard_lite_v2` unchanged as the existing default unless implementation deliberately changes training configuration later.
- Do not expose opponent decklists, current hidden deck order, or inferred hidden card locations.
- Do not change `ACTION_SPACE_SIZE` or action masking behavior.
- Treat the new profile as a distinct observation contract for model metadata, export, and compatibility checks.

## Capabilities

### New Capabilities

- `decklist-aware-observation-profile`: Defines the `standard_lite_deck_v2` profile and its own-original-decklist tensor contract.

### Modified Capabilities

- None.

## Impact

- Rust engine tensor profile registry, layout metadata, tensor writer, observation profile parsing, and tests.
- Rust game/player setup state so original submitted deck composition is available to observation writers.
- PyO3 observation layout exports and Python tensor profile consumers.
- RL tests, model metadata expectations, ONNX/export compatibility, and tensor documentation.
- No action decoder, action mask, or action-space constant changes.
