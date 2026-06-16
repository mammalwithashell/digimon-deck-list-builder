## Context

`engine_py_legacy` is the sunset Python engine, but it still ships **live production code** the hosted API and `code/tools/` import. The deferred `excise-legacy-engine-from-hosted-api` change scopes the *whole* server migration as one large, risky unit gated on a net-new interactive Rust runner. A 2026-06-14 investigation (11 read-only agents + adversarial verification) found that the server's legacy coupling cleanly splits along one line: **the live PvP/WebSocket runtime vs everything else.** Everything else is low-risk-or-dead and can land independently now.

The Rust engine already provides the primitives this change consumes: `parse_deck`/`validate_deck`/`summarize_deck`/`validate_deck_for_game_mode`/`PyCardRestriction` (PyO3, `lib.rs:309-401`), the replay core (`runners/replay.rs` with a `NativeAdapter` that reads the persisted recording format), and `RustHeadlessGame.to_ui_json()` (already serving `/games` + `/debug`). The redaction filter is engine-agnostic and already runs over that `to_ui_json` output in production.

## Goals / Non-Goals

**Goals**
- Make every hosted-API surface **except the live PvP runtime** import zero `engine_py_legacy`.
- Make the retained `code/tools/` CLI import zero `engine_py_legacy`; delete the obsolete tools.
- Preserve every contract: deck legality + restricted list, per-player redaction (rules 9 & 14), recording/replay compatibility.
- Shrink `excise-legacy-engine-from-hosted-api` to the PvP wire only.

**Non-Goals**
- The PvP/WebSocket interactive runtime (`InteractiveGame` in `ws_*` / `lobby`), the interactive Rust PyO3 runner, and GameEvent wiring — all stay with `excise`.
- Deleting `code/engine_py_legacy/` (the deletion gate is downstream, gated on the PvP cutover; this change only **relocates** the live non-PvP modules out of it).
- Any change to gameplay rules, the action space, or the wire JSON schema.

## Decisions

- **D1 — The cut line is the PvP runtime.** After this change the only server files still importing `engine_py_legacy` are `ws_games.py`, `ws_manager.py`, and `lobby.py` (all via `InteractiveGame`). `matchmaking.py`'s only legacy import is an **unused** `InteractiveGame` re-export (`# noqa: F401`) — delete it; matchmaking becomes legacy-free for free.
- **D2 — `state_filter` relocates verbatim, no rewrite.** The module has zero engine imports and is keyed entirely off dict string keys (`player1`/`player2` → `handIds`/`handCards`/`securityIds`) that Rust `to_ui_json` emits identically, with the Python 1/2 player-id convention. Move it to `code/server/state_filter.py` (which both READMEs already wrongly reference), repoint importers + the test; no logic edit. Add one integration assertion that `filter_state_for_player(RustHeadlessGame.to_ui_json(), 1)` redacts the right keys, to lock the Rust-output contract (the filter is allowlist-by-omission, so any *new* hidden key must be added to `_redact_player`).
- **D3 — `no_restriction` via game-mode, not a custom-restriction overload.** `decks.py` currently passes an empty `CardRestriction()` to bypass the restricted list. Rust already supports `validate_deck_for_game_mode(ids, "no_restriction")` (skips card_limits + choice_groups), which is behaviorally equivalent — so route through it and **do not** add a custom-`CardRestriction` PyO3 API.
- **D4 — `titan`/`edh_commander`: thin Python wrapper (default) vs Rust `DeckRuleset` arms (alt).** Rust `from_game_mode` maps both to `None` today, while `decks.py` hand-rolls their size/singleton logic in Python over the (Rust) `CardDatabase`/`CardKind` egg classification. Lower-risk increment: keep the thin Python size/singleton wrapper but point its card-kind lookups at the Rust binding (already exported), so no `engine_py_legacy` import remains. Adding Rust arms (using the existing `Rules::edh()`/`titan()` presets) is the cleaner end-state but is net-new validation code — defer unless desktop parity needs it (desktop currently errors on these modes).
- **D5 — Replay: thin PyO3 wrapper + per-step state via `to_ui_json`.** Add `RustReplayRunner` over `ReplaySession`/`NativeAdapter`. Rust `ReplayStepResult` has no per-step `state` field (server schema requires one) → populate it from `to_ui_json`. Reconcile 0-based Rust seek vs 1-based server/legacy indexing in the wrapper. Fix the save gate (`isinstance` → `RustHeadlessGame`) and enable `record_actions` (hardcoded `False` at `games.py:185`).
- **D6 — `script_promotion` is retired, not migrated.** It promotes *Python* card scripts (sunset model); cards are now Rust DSL (rules 21/28), and its admin-pipeline feeders already point at the nonexistent `code/digimon_gym/engine/data/scripts` path. Remove the import + its two endpoints from `admin_ai.py`; leave the rest of the admin AI pipeline (a separate decision).
- **D7 — Tools tiered: delete / re-home / port.** TIER-1 delete (no caller): `promote_script`, `archive/bootstrap_frozen_manifest`, `check_frozen_integrity`, `run_qa_batch`. TIER-2 re-home (one edit each): `meta_loader` (inline the `RE_CARD_ID` regex), `resolve_deck` (delete the dead legacy `except` fallback). TIER-3 real work: `ingest_cards` (port `parse_xros_req`/`parse_digixros_req` — the **only** blocker for later deleting `card_database.py`; must emit byte-identical `dna_costs`/`digixros_costs`), `run_scenario` (delete after repointing the `gameplay-qa` skill at `digimon-engine-cli`), `train_card_autoencoder` (delete — its warm-start path is already dead). `build_tested_cards` is **not** a legacy importer — leave it.
- **D8 — Restricted-list drift.** There are **four** hand-maintained copies in sync today: `deck_loader.py`, `rules.rs` (`OFFICIAL_ENG_RESTRICTION`, exposed via `restricted_list()`), `deck_tools.rs` constants (used by `validate_deck`), and `tests/deck_tools/main.rs`. As `decks.py` migrates off the Python copy, **delete `deck_loader.py`'s list** (or add a cross-engine sync assertion) so the drift surface shrinks.

## Risks / Trade-offs

- **`ingest_cards` parser parity** is the one correctness-sensitive item: `parse_xros_req`/`parse_digixros_req` must produce byte-identical `dna_costs`/`digixros_costs` or `cards.json` regeneration silently drops DNA/DigiXros costs (DigiXros / Appmon / Xros archetypes). `tools/xros_req_parser.py` is a *different* parser, not a drop-in. Gate on a diff of regenerated `cards.json` against the current file.
- **Restricted-list four-copy drift** until the Python copy is deleted (D8).
- **`run_scenario` deletion is gated** on the `gameplay-qa` skill rewrite; until then a human may still invoke it.
- **Recording corpus may be empty** — the replay routers may have been dead since the Rust cutover; the data-compat guarantee is real but the practical regression corpus needs confirmation (a DB query for `game_recordings` rows).
- **`train_card_autoencoder` warm-start** regressed silently during a prior phase (path points at a nonexistent dir). Confirm with the user that pretrained card embeddings are not a desired RL feature before deleting the trainer (deletion is otherwise safe).

## Open Questions

- `titan`/`edh_commander`: Rust `DeckRuleset` arms now, or keep the thin Python wrapper over the Rust binding? (D4 — default is the wrapper.)
- `ingest_cards` xros parsers: port to Rust + PyO3, or a standalone pure-Python parser under `code/tools/`? (The Rust engine only consumes the pre-parsed JSON, so a pure-Python tool-side parser may be simplest.)
- Delete `train_card_autoencoder` outright, or revive the warm-start path as a separate feature fix? (Out of scope here; confirm intent.)
- Should `code/tests/ai_pipeline/test_ai_pipeline.py` be repointed or removed when `script_promotion` retires? (Depends on whether the broader admin AI pipeline survives — a separate decision.)
