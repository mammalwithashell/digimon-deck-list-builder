# Proposal: name-activatable-effects-in-action-bar

## Why

The in-game action bar labels every activatable effect generically — e.g. a field [Main] Digiburst shows as `Effect 1:2`, where `1` is the board slot and `2` is an internal sub-slot constant (neither is meaningful to a human). The frontend derives these labels by re-deriving action semantics from raw mask bit-ranges in `useActionMask`, and that re-derivation has drifted out of sync with the engine's action layout, producing three concrete defects:

- **Field [Main] / Digiburst / delayed-Option [Main]** (engine `1000–1149`): rendered, but with the cryptic `Effect {slot}:{subslot}` label and no card identity, even though the source card's name is already in state.
- **Trash [Main]** (engine `1150–1194`): the frontend's effect loop spans `1000–1999`, so these decode to nonexistent battle slots `15–19` → labeled `Effect 15:0` with no card to name (and trash-card names aren't even on the wire).
- **Hand [Main]** (engine `30–59`): collides with the frontend's stale "trash from hand" range, so a `MainFromHand` effect is silently mis-bucketed as "trash this card" and never offered as an activatable effect.

Players cannot tell which card is triggering an effect, and whole categories of activatable effects (trash [Main], hand [Main], breeding `<Training>`) are mislabeled or invisible. The engine already emits all of these correctly (verified green: `mask_and_tensor` parity suite, 170 passing) and already has a per-action decoder (`legal_decoded_actions`) that carries `card_name` per action — it is simply not exposed to the production Tauri/REST clients, and it does not yet surface the effect's own name.

## What Changes

- **Expose the engine's decoded-action list to production clients.** Add a Tauri command and a hosted-API REST endpoint that return `legal_decoded_actions(game, player)` — a per-action list of `ActionExplanation { action_id, kind, source_zone, source_index, card_id, card_name, effect_name, label, ... }`. The frontend stops re-deriving action semantics from raw bit-ranges for activatable effects and renders directly from this list, which decodes every action id correctly by construction (killing the trash/hand/training mis-decode class).
- **Surface the effect's name.** `Effect` already carries a `name: String` field (`effect.rs`), but `explain_action` never reads it. Add `effect_name: Option<String>` to `ActionExplanation`; for field/hand/trash [Main] actions, resolve the *matched* effect (mirroring the mask builder's first-match-wins) and populate its name.
- **Populate names on the main-activatable DSL lowering paths.** Audit the lowerings that produce [Main]-timed activatable effects and ensure each sets a meaningful `.name` — notably the `digi_burst` lowering → `"Digiburst"`, `<Training>`, and delayed-Option `[Main]` bodies. Several lowerings already name their effects (`lower_aura`, `lower_grant_keyword`, `lower_cost_reduction`); this fills the gaps for the activatable categories. Card-name-only is the fallback when an effect name is absent.
- **Action bar labels by card + effect, surfaces all categories.** The action bar renders an entry for **every** currently-legal activatable effect — field [Main], Digiburst, breeding `<Training>`, trash [Main], hand [Main], delayed-Option [Main] — labeled `"{card name}: {effect name}"` (or just `"{card name}"` when no effect name), with the board slot appended **only** when two surfaced entries share a card name. Hover/tooltip shows the source card's main effect text (from the permanent/state DTO, matched by `source_zone` + `source_index`).
- **No new contextual UI.** This change is the decoder/naming substrate. The separately-proposed `add-per-card-command-panel` (DCGO-style click-the-card menu) is a complementary affordance that can consume the same decoded-action list and effect names instead of its planned generic-label fallback.

## Capabilities

### New Capabilities
- `action-bar-activatable-effects`: The production action bar surfaces every currently-legal activatable effect (field [Main], Digiburst, breeding `<Training>`, trash [Main], hand [Main], delayed-Option [Main]), each labeled by its source card and effect name with a main-effect-text tooltip, rendered from an engine-decoded legal-action list exposed over the Tauri and hosted-API surfaces.

### Modified Capabilities
- `live-game-surface`: The "Legal Actions Enumeration" requirement's decoded labels gain the effect's own name. `ActionExplanation` adds an `effect_name` field, and the "Labels include card names" scenario is extended so [Main]-activated actions also carry the matched effect's name.

## Impact

- **Engine** (`code/digimon-engine/`): `action/explain.rs` — add `effect_name` to `ActionExplanation` and resolve the matched effect for field/hand/trash [Main] actions; no change to `action/space.rs` ids or the mask. `dsl_cards/lower_*` — name-population audit for the activatable [Main] lowerings (`digi_burst`, training, delayed-Option). No action-space version bump (the action ids and `ACTION_SPACE_SIZE` are unchanged), so trained models and recordings are unaffected.
- **Desktop** (`code/src-tauri/src/engine_commands.rs`): new Tauri command returning the decoded legal-action list (and `effect_name`) alongside existing state/mask responses.
- **Hosted API** (`code/server/routers/games.py` or engine-only router): new REST endpoint returning the same decoded list for browser play. Engine-only router constraints preserved (no DB/auth imports).
- **Frontend** (`code/frontend/src/`): `components/game/ActionBar.tsx` renders activatable effects from the decoded list (card + effect name, duplicate-name slot disambiguation, tooltip); `api/gameApi.ts` + `types/game.ts` carry the decoded-action DTO and `effect_name`; `hooks/useActionMask.ts` no longer drives activatable-effect labels (its other capability maps are unchanged). Fixes the latent trash-from-hand vs hand-[Main] collision.
- **Relationship**: provides the engine-decoded, named action source that `add-per-card-command-panel` planned to "upgrade to" once available; both surfaces share one decoder, avoiding the re-derivation drift that caused this bug.
- **Tests**: engine unit tests for `effect_name` resolution in `explain_action` (including first-match-wins and the breeding/trash/hand zones); DSL lowering name assertions (e.g. `digi_burst` → "Digiburst"); frontend component tests for label formatting + duplicate-name disambiguation; a Playwright scenario (via the scenario MCP substrate) asserting a named Digiburst/Training/trash entry appears for a staged board.
