## 1. Seed Utilities and API Contracts

- [x] 1.1 Add shared frontend/backend seed validation rules for decimal-string `u64` values, including empty, non-numeric, negative, and out-of-range cases.
- [x] 1.2 Update frontend create-game types so request seed input is optional and response effective seed is a decimal string.
- [x] 1.3 Update browser `/games` schemas and response payloads to accept optional seed input and return the effective seed string on create/state/action paths.

## 2. Desktop Local Game Creation

- [x] 2.1 Update the Tauri `rust_create_game` command to accept an optional seed string, parse it to `u64`, and generate a random `u64` when omitted.
- [x] 2.2 Replace the hardcoded `Some(42)` desktop game seed with the effective seed selected by the command.
- [x] 2.3 Return the effective seed string from the Tauri create-game response and thread it through `gameApi`/`playApi`.
- [x] 2.4 Update existing deterministic desktop/offline tests to pass explicit seeds, and add a regression test that omitted seed no longer always uses `42`.

## 3. Room Match Seed Flow

- [x] 3.1 Extend room lobby server state to store pending explicit seed mode separately from generated-seed mode and expose that state in room responses.
- [x] 3.2 Add host-only room seed update/clear handling before match start, reusing the same decimal-string validation rules.
- [x] 3.3 Update room start logic so explicit seed mode uses the exact seed unchanged and derives initial player from that seed.
- [x] 3.4 Preserve first-player selector behavior for generated-seed mode by generating a seed compatible with the selected first-player option.
- [x] 3.5 Add server tests for explicit seed start, seed clear, invalid seed rejection, and generated-seed first-player compatibility.

## 4. Frontend Seed Controls and Display

- [x] 4.1 Add optional seed entry controls to bot game launch surfaces, with generated-random behavior when left blank.
- [x] 4.2 Add host seed entry/clear controls to room match lobby UI and disable/read-only the first-player selector while explicit seed mode is active.
- [x] 4.3 Persist the effective seed in game/play-flow state after bot creation or room start.
- [x] 4.4 Add an in-game seed display chip/control with copy-to-clipboard behavior.
- [x] 4.5 Add effective seed display and copy-to-clipboard behavior to the result overlay.

## 5. Result Navigation

- [x] 5.1 Update the result overlay return action wiring so win/loss return resets transient game state and navigates to `/`.
- [x] 5.2 Ensure desktop builds land on the launcher page after the result return action instead of the legacy in-game home state.
- [x] 5.3 Keep the terminal result overlay visible until the user activates the return action so the seed remains copyable.

## 6. Verification

- [x] 6.1 Add frontend unit tests for seed validation, large seed string preservation, seed copy controls, and result return navigation.
- [x] 6.2 Add API/room tests proving explicit seed replay metadata and generated seed metadata are returned losslessly.
- [x] 6.3 Run targeted Rust/Tauri, server, and frontend test suites affected by the seed and result-navigation changes.
- [x] 6.4 Manually verify a bot game can copy a random seed, start a new bot game with that seed, and reproduce the initial hand/deck order.
