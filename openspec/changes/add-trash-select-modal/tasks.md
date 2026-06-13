> Note: tasks below originally referenced the trash action base as `130`. The
> live diagnosis (group 0) proved the engine base is `SELECTION.TRASH_START =
> 1150` (= engine `TRASH_EFFECT_START`); the implementation uses that constant
> and validates against the engine mask/`validIndices` (design D5).

## 0. Production soft-lock — DIAGNOSED, FIXED, VERIFIED (root cause: stale trash action-ID constant)

- [x] 0.1 Reproduced live: browser-dev + scenario MCP staged CresGarurumon ST6-13 on field (2 sources) + purple Lv3s in trash; drove Digi-Burst 2 → forced "play 1 purple Lv3 from trash" selection.
- [x] 0.2 Captured locked state: `pendingSelection.kind = Trash`, `validIndices = [1150..1154]`, `actionMask[1150..1154] = 1` but `actionMask[130..134] = 0`; only legal actions `[93 concede, 1150..1154]` (no PASS → forced).
- [x] 0.3 Root-caused with `document.elementFromPoint`: nothing occludes (card image is top element); the card wrapper is `cursor-not-allowed` because `SelectionPanel` gates on `actionMask[SELECTION.TRASH_START + i]` = `actionMask[130+i]` (stale constant) while the engine emits trash-selection IDs at `TRASH_EFFECT_START = 1150` (`space.rs`). Overlay/dispatch/`agentPending` hypotheses disproven.
- [x] 0.4 Fix: corrected `SELECTION.TRASH_START/END` in `code/frontend/src/utils/constants.ts` from `130/179` → `1150/1194` (matches engine `TRASH_EFFECT_START..END`). Fixes `SelectionPanel.tsx:104-105` and `useActionMask.ts:141`.
- [x] 0.5 Verified live end-to-end: the 5 valid Lv3 cards became `cursor-pointer`, the Lv5 card stayed `cursor-not-allowed`, and clicking a card resolved the selection (DemiDevimon played from trash, modal closed, returned to Main).
- [x] 0.6 Regression guard added: `TrashSelectModal.test.tsx` asserts mask/`validIndices`-driven clickability + dispatch; engine `kinds_exist.rs::count_capped_kind_str_carries_ui_fields` guards the serialized-kind shape. Desktop parity holds (same engine action space → same `1150` range; desktop wire emits the kind via `format!("{:?}")`).

## 1. Engine: expose multi-select floor and distinct flag

- [x] 1.1 Widened the variant in `code/digimon-engine/src/selection.rs` to `CountCappedMultiSelect { min: u8, max: u8, picked: u8, distinct: bool }`.
- [x] 1.2 Install site (`selections.rs:~3218`) constructs `{ min: effective_min, max, picked, distinct: distinct_by.is_some() }`.
- [x] 1.3 No production match arms destructure the variant (grep confirmed only definition + install site).
- [x] 1.4 Updated all test assertions to the new field set (patterns → `..`; `assert_eq!` constructions → `assert!(matches!(.., ..))`; `kinds_exist` construction → all 4 fields).
- [x] 1.5 `cargo test --test selection` green (78 passed); added `count_capped_kind_str_carries_ui_fields` asserting the serialized kind contains `min`/`max`/`picked`/`distinct`.

## 2. Front-end: kind parser util

- [x] 2.1 Added `parseCountCappedKind` + `isTrashAction` (range = `SELECTION.TRASH_START..TRASH_END`) in `code/frontend/src/utils/trashSelection.ts`.
- [x] 2.2 Added `trashSelectionMode(pendingSelection)` → `'single' | 'multi' | null` (kind `Trash`, or `CountCappedMultiSelect` with every `validIndex` in the trash range).
- [x] 2.3 Unit-tested parser + predicates in `trashSelection.test.ts` (10 tests).

## 3. Front-end: TrashSelectModal component

- [x] 3.1 Created `TrashSelectModal.tsx` with the specified props (`onAction` returns `Promise|void`).
- [x] 3.2 Open condition gates on `trashSelectionMode` + local selecting player + not keyword prompt; owner list via `zoneOwner`; card `i` → `SELECTION.TRASH_START + i`; right-click inspects.
- [x] 3.3 Single-select: legality from the mask; click dispatches; Decline shown when `actionMask[62] === 1`.
- [x] 3.4 Deferred multi (`distinct === false`): local ordered toggle, cap at `max`, counter + floor hint, Done gated on `min`, drains picks then PASS (skips PASS at exactly `max`).
- [x] 3.5 Immediate multi (`distinct === true`): per-click commit; consumed cards render selected + non-toggleable; Done = PASS when legal.
- [x] 3.6 Lifecycle guards: local `picked` reset only on open-identity change, never while `committing`; grid frozen during the drain.

## 4. Front-end: wire-up and SelectionPanel cleanup

- [x] 4.1 `SelectionPanel.tsx`: dropped `SelectTrash` from `PANEL_PHASES`, removed the trash branch and the `trashIds`/`opponentTrashIds` props.
- [x] 4.2 `GamePage.tsx`: mounted `<TrashSelectModal>`; removed trash props from `SelectionPanel`; read-only `TrashViewer` board click retained.
- [x] 4.3 `PromptBar` unchanged (still lists `SelectTrash`/`SelectBudgeted`); only `SelectionPanel`/`GamePage` consumed the trash props (both updated; typecheck clean confirms no other consumer).

## 5. Front-end tests

- [x] 5.1 `TrashSelectModal.test.tsx`: single-select dispatch, opponent-trash via `zoneOwner`, optional decline.
- [x] 5.2 Deferred multi: toggle on/off, count + floor gating, Done order = picks then `62`, exactly-`max` omits `62`, cap enforced.
- [x] 5.3 Distinct multi: immediate per-click dispatch, consumed cards non-toggleable, Done = `62`.
- [x] 5.4 Pruned trash cases from `SelectionPanel.test.tsx`; asserts the panel renders nothing for `SelectTrash`.

## 6. Verification

- [x] 6.1 `npm run build` (tsc + vite) green; full vitest suite green (139 tests, 28 files).
- [x] 6.2 Desktop wire: `engine_commands.rs` emits the kind via `format!("{:?}", sel.kind)` (auto-includes new fields, no code change); `cargo check --lib` on `src-tauri` green (Finished, exit 0).
- [x] 6.3 Live smoke: CresGarurumon ST6-13 forced single-select through the NEW `TrashSelectModal` resolves (DemiDevimon played; SelectionPanel not used). Multi-select (deferred toggle/Done + distinct) covered by unit tests; opponent-trash single covered by unit test.
- [x] 6.4 `openspec validate add-trash-select-modal` → valid.
