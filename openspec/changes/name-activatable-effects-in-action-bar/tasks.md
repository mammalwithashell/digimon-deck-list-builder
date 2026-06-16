# Tasks: name-activatable-effects-in-action-bar

## 1. Engine — effect-name resolution in the decoder

- [x] 1.1 Factor the "first eligible [Main] effect for (carrier/card, timing)" selection used by `build_action_mask` into one shared helper so the mask builder and the decoder select the same effect (`action/mask.rs`). → `action/main_effect_select.rs`; mask refactored at all four sites; `mask_and_tensor` 170/170 green (behavior-preserving).
- [x] 1.2 Add `effect_name: Option<String>` to `ActionExplanation` (`action/explain.rs`) and to the `DecodedAction` exposed by `LiveGame`. → `LiveGame::legal_actions` returns `Vec<ActionExplanation>`, so the field flows through directly.
- [x] 1.3 In `explain_action`, for field [Main] (`1000–1149`), trash [Main] (`1150–1194`), and hand [Main] (`30–59`) action ids, resolve the matched effect via the shared helper and populate `effect_name` from `Effect::name`; include the effect name in `label` when present. → `finalize_main_effect` / `resolve_field_main_name`.
- [x] 1.4 Ensure breeding `<Training>` (`1142`) and delayed-Option `[Main]` actions resolve a sensible `effect_name`/`card_name` (breeding carrier / Option card) consistent with the mask. → breeding training names via `breeding_training_match`; delayed-Option keeps card-name fallback (`effect_name = None`).
- [x] 1.5 Unit test: mask-vs-decoder agreement — the effect the decoder names is exactly the effect the mask surfaced (first-match-wins) across field/hand/trash zones. → `mask_and_decoder_agree_on_field_main` (OPT-exhaustion retraction) + the existing `action_main_effects_parity` agreement tests.
- [x] 1.6 Unit tests: field/hand/trash [Main] actions return the expected `effect_name`, `card_name`, `source_zone`, `source_index`; an unnamed effect returns `effect_name = None` with card still identified. → `decoder_names_{field,hand,trash}_main_effect` + `decoder_unnamed_effect_returns_none_but_identifies_card`.

## 2. Engine — effect-name source (RESCOPED)

Decision (user, short-derived-tag): the action bar shows `"{card}: {short tag}"` where the short tag is derived **in the frontend** from the effect's authored DSL `summary`, with the full summary + printed text in the tooltip. The engine already sets `Effect.name = summary` (`lower_triggered.rs:184`) and the decoder now surfaces it as `effect_name`, so no `.name` lowering edits are needed — the naming source already exists. Short-tag derivation moves to task group 5.

- [x] 2.1 Audit lowerings that emit [Main]-timed activatable effects for the name source. → `lower_triggered.rs` already names every clause from its YAML `summary`; CresGarurumon ST6-13 carries `"[Main] Digi-Burst 2: …"`. No lowering change required.
- [~] 2.2 ~~Set `.name("Digiburst")` on the `digi_burst` lowering~~ → N/A: summary supplies the name; the short "Digi-Burst 2" tag is derived in the frontend (task 5.2a).
- [~] 2.3 ~~Set `.name` on `<Training>` / delayed-Option lowerings~~ → N/A for the same reason; card-name fallback when a clause has no summary.
- [ ] 2.4 Engine test: CresGarurumon ST6-13's `main_on_field` effect lowers with a non-empty `name` containing `"Digi-Burst 2"` (validates the summary→name→decoder path for the headline card).

## 3. Transport — expose the decoded list to production clients

- [x] 3.1 Add a Tauri command in `code/src-tauri/src/engine_commands.rs` returning `legal_decoded_actions` for the current decision player. → `rust_get_decoded_actions` (registered in `main.rs`); Tauri crate `cargo check` green.
- [x] 3.2 Add a hosted-API REST endpoint on an engine-only router returning the same decoded list; no DB/auth imports. → `GET /games/{id}/decoded-actions` in `games.py` + `RustHeadlessGame::legal_decoded_actions` PyO3 method (`digimon-engine-py` `cargo check` green). No new imports in the engine-only router.
- [x] 3.3 Confirm an inactive/non-decision player request returns an empty list on both surfaces. → Both surfaces query via `current_decision_player()`, so a non-decider's list is structurally unreachable; `legal_decoded_actions`/`LiveGame::legal_actions` already return empty for non-deciders (covered by `legal_actions_for_non_decision_player_returns_empty`).
- [x] 3.4 Desktop/browser DTO parity: identical `ActionExplanation` serialization on both wires. → Desktop returns `Vec<ActionExplanation>` directly; browser serializes the same `Vec<ActionExplanation>` via serde_json → same field names/shape. Both expose it as a dedicated call (Tauri command / REST endpoint), parallel to the action mask.

## 4. Frontend — types and data plumbing

- [x] 4.1 Add `effect_name` to the decoded-action DTO/type and decode it for both wires. → `DecodedAction.effectName` (types/game.ts), `DecodedActionDto.effect_name` + `toDecodedAction` mapping, and `getDecodedActions(gameId)` in gameApi.ts (Tauri `rust_get_decoded_actions` / REST `/decoded-actions`).
- [x] 4.2 Surface the decoded-action list in the game store so `ActionBar` can consume it per state update. → `gameStore.decodedActions` + `setDecodedActions`; refreshed by a `useEffect` in GamePage keyed on gameId/actionMask/agentPending/isGameOver (skipped in WebSocket modes).

## 5. Frontend — action bar rendering

- [x] 5.1 Render activatable-effect entries in `ActionBar.tsx` from the decoded list (filtered to field/hand/trash effect kinds), with a mask-derived fallback when the decoded list is unavailable (WebSocket modes). Click submits `action.actionId` directly (no manual range encoding).
- [x] 5.2 Compose labels via `utils/effectLabel.ts`: `"{card}: {short tag}"` (5.2a: `shortEffectTag` strips leading `[..]`/`<..>` timing tags + truncates per the user's short-derived-tag decision), falling back to `"{card}"` when no usable tag.
- [x] 5.3 Append `(slot N)` only when two or more surfaced entries share a card name (`effectNameCounts`); unique names show no slot.
- [~] 5.4 Tooltip = full authored effect summary (`effectTooltip` → `effectName ?? label`), matching the selected preview's tooltip line. NOTE: enriching with the printed `mainEffectText` from the permanent DTO (the preview's "(+ printed main effect text)") is deferred — it needs perspective-player resolution and trash-card text that isn't on the state wire; tracked as a follow-up.
- [x] 5.5 Stop routing hand [Main] (`30–59`) into `canTrashFromHand` (`useActionMask.ts`) — those ids are HAND_EFFECT and are now surfaced via the decoded list; the unused field is retained empty for interface compatibility.
- [x] 5.6 Component tests (`ActionBar.test.tsx`) + util tests (`effectLabel.test.ts`): label formatting, duplicate-name slot disambiguation, missing-effect-name fallback, trash [Main] named by card (no "Effect 15:0"), action-id click, mask fallback. 17 tests green; full frontend suite 165 green; `tsc --noEmit` clean.

## 6. End-to-end verification

- [ ] 6.1 **(pending live verification)** Playwright scenario (via the scenario MCP substrate): stage a board with a field Digiburst, a breeding `<Training>`, and a trash [Main] card; assert each appears in the action bar with its card+effect name, and that activating one submits the correct action id. Requires the dev server + rebuilt PyO3 binding (`maturin develop`) + Playwright — not run in this session.
- [ ] 6.2 **(pending live verification)** Manual verification in the desktop app: the screenshot case (CresGarurumon field Digiburst) reads as `"CresGarurumon: Digi-Burst 2"` instead of `Effect 1:2`. Requires launching the desktop/dev app — not run in this session (desktop dev build also has a known intermittent startup crash, see memory).
- [x] 6.3 Run engine tests; confirm green and no action-space version bump. → `mask_and_tensor` 175/175 green (incl. new decoder/agreement tests), ST6-13 digiburst name-source test green; `space.rs` untouched, `ACTION_SPACE_SIZE` = 2192 / `SCHEMA_VERSION` = 1 unchanged. `cargo check` green for `digimon-engine`, `digimon-engine-py`, `digimon-tcg`. Frontend: 165 tests + `tsc --noEmit` green.
