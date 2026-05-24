## Why

`EffectContext::trash_card_source` panics with `"card not in this permanent's stack"` when its `CardHandle` argument no longer matches a card in the carrier's `card_sources`. This fired in RL training (run `generalist_1m_v2`, game 9728, recorder action 87 — TS Olympos turn 10 with parallel EX10-033 + EX10-032 [WD] triggers) and is deterministically reproducible by replay. Root cause: `install_source_multi_selection` snapshots candidate `SourceSelectionRef`s into `action_to_source` at install time and never re-validates them; intervening observer effects between install and the player's submission can drain the picked source, leaving the picker offering a phantom candidate that fails the `expect(...)` in `trash_card_source` when submitted. DCGO's analog (`ITrashDigivolutionCards.TrashDigivolutionCards`, `CardEffectCommons.SelectTrashDigivolutionCards`) treats source-trash primitives as advisory: targets are re-filtered against the live stack at trash time and silently dropped if gone. Rust engine should mirror that contract — the printed text ("by trashing N source cards") is naturally permissive about availability, and the engine should resolve observer-interleaved chains without panicking on rules-natural fizzles.

## What Changes

- **BREAKING (Rust API)**: `EffectContext::trash_card_source` returns `bool` (true iff the card was actually trashed) instead of `()`. Stale or invalidated handles silently no-op rather than panicking. Three `.expect(...)` calls in the function body — `permanent not found`, `card not in this permanent's stack`, and the implicit `top_card()` empty-stack panic — all soften to `return false`.
- DSL step `CompiledStep::TrashSelectedSources` ([zone_moves.rs:211](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:211)) and `CompiledStep::TrashUnionBound` ([zone_moves.rs:300](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:300)) iterate source refs and discard the bool return (informational only) — surviving picks trash, stale ones no-op. Matches DCGO's `SelectTrashDigivolutionCards` interleave shape.
- `install_source_multi_selection`'s submit callback ([selections.rs:2586](code/digimon-engine/src/effect_context/selections.rs:2586)) re-resolves each picked `source_ref` against the live `card_sources` at submit time. If the picked card has vanished, refuse the action and re-install the SourceMulti pending with current candidates (preserves prior valid picks). Mirrors DCGO's `customRootCardList: selectedPermanent.DigivolutionCards` live-list semantics.
- Engine callers that depend on atomic semantics (`<Fragment>` keyword install at [keyword_effects.rs:294-326](code/digimon-engine/src/cards/keyword_effects.rs:294)) keep their pre-validation gates — the new soft-fail is only consulted on the rare stale-pick path.
- Record `G-DSL-TRASH-SOURCES-STALE-HANDLE` in `qa/archetype-qa/panic-families.json` and `qa/archetype-qa/engine-gaps.md` for traceability.
- Add a regression test that replays the captured recording (`models/.../train_env_000_game_009728_draw_crash.json`) and asserts the replay seeks past step 85 without panicking; plus a synthetic unit test that calls `trash_card_source` with a stale handle and asserts `false`.

## Capabilities

### New Capabilities

- `source-trash-soft-fail`: DCGO-parity contract for stack-source trash primitives — declarative intent in, actuals out, no panics on rules-natural fizzles. Covers the `trash_card_source` primitive, the `TrashSelectedSources` / `TrashUnionBound` DSL step semantics, and `install_source_multi_selection`'s live revalidation behavior.

### Modified Capabilities

(none — the existing `permanent-deletion-semantics`, `zombie-permanent-cleanup`, and `dsl-card-scripting-vocabulary` specs cover adjacent topics but their requirements do not change. Cross-references go in this change's design.md.)

## Impact

- **Rust engine**: `code/digimon-engine/src/effect_context/mod.rs` (`trash_card_source` signature change + soft-fail body), `code/digimon-engine/src/effect_context/selections.rs` (`install_source_multi_selection` submit-callback revalidation).
- **DSL step runner**: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` (`TrashSelectedSources`, `TrashUnionBound` ignore new bool return; minimal change).
- **Existing direct callers** of `trash_card_source` — `<Fragment>` install (`code/digimon-engine/src/cards/keyword_effects.rs:326`), `trash_all_sources` ([mod.rs:4206](code/digimon-engine/src/effect_context/mod.rs:4206)), `trash_top_n_digivolution_cards_of_each` ([mod.rs:5351](code/digimon-engine/src/effect_context/mod.rs:5351)) — discard the bool. They already pre-validate; new return value is informational only. No call-site rewrites beyond `let _ = ctx.trash_card_source(...)`.
- **PyO3 surface** (`code/digimon-engine-py/src/lib.rs`): unchanged — `trash_card_source` is not exposed to Python directly; Python uses `RustHeadlessGame` step/submit only.
- **Behavioral tests**: existing `code/digimon-engine/tests/effect_context/trash_card_source.rs` tests assume the panic on stale handle — those assertions get inverted (expect `false` now). New regression test + recording fixture added under `code/digimon-engine/tests/recordings/`.
- **No DSL YAML changes**: card scripts in `code/digimon-engine/cards/` are unaffected (the DSL surface keeps `select_own_sources { then: trash_selected_sources }` shape; only its runtime behavior under stale-pick conditions changes).
- **No card-text faithfulness changes**: silent no-op of stale picks is the rules-natural outcome (printed "by trashing N cards" is upper-bound, not lower-bound) and matches DCGO behavior.
- **Out of scope for this change** (tracked separately): recorder/replay JSON schema mismatch ([runners/replay.rs:134](code/digimon-engine/src/runners/replay.rs:134) expects `initial_state` at top-level but recorder nests it under `recording`) — separate chip.
- **Out of scope for this change** (Tier 3 in diagnosis): full DCGO-shape two-step per-permanent picker (SelectPermanent → SelectCard scoped to live stack). Tier 1 (soft-fail primitive) + Tier 2 (picker revalidation) close the panic class without that refactor.
