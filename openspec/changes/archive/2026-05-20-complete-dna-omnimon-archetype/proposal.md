## Why

DNA Omnimon (64 unique cards, 66 decklists, 3.5% meta share) is the lead pilot archetype for the Rust DSL card pipeline, and it is ~85% implemented — but it cannot be declared complete because the trackers, the engine code, and the behavioral test files have drifted out of sync. Test files still carry `#[ignore = "pending: G-XYZ"]` markers for substrate gaps the trackers record as *resolved* (`G-OPT-TRIGGERED`, `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`, `G-ALT-PATH-DIRECTION-INTO`), the per-card verdict ledger `validated_cards_dsl.json` that every Phase 2 plan references does not exist on `main`, and 5 cards — including the 63-of-66-deck staple BT22-084 Nokia Shiramine — have no YAML at all. The archetype's true remaining scope is currently unknowable, which blocks a clean Phase 3 handoff.

## What Changes

- **Reconciliation sweep**: every DNA Omnimon `#[ignore]`'d behavioral test (~112 markers across 42 cards) is triaged against the *current* engine/DSL substrate, then either re-enabled with its card clause authored, or kept ignored with a verified, accurate gap reference.
- **Verdict ledger**: a real `validated_cards_dsl.json` entry set for all 64 DNA Omnimon cards is produced as the sweep's output, replacing tracker guesswork with evidence.
- **Missing card authoring**: production YAML + behavioral tests for the 5 unauthored cards — BT22-084, BT17-007, ST2-13, BT5-093, AD1-019 — completed TDD-first per CLAUDE.md §18.
- **Deferred DSL gap closure**: `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` (an inherited-source "substitute return-to-deck for trash" replacement clause) is implemented, unblocking EX5-015 Clause C — the one gap Track F explicitly deferred.
- **Residual substrate gaps**: any genuinely-open engine/DSL gap the sweep proves real (e.g. Option-card-in-battle-area placement, security-zone aura source) is closed or filed as a scoped follow-up — no stale markers left behind.
- **`raw_rust` minimization**: the 18 `raw_rust` escapes across 8 DNA Omnimon cards are reviewed; escapes now expressible in pure DSL are migrated, and the remainder are documented as justified.
- Tracker hygiene: `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` are reconciled with the verified end state.

No `ACTION_SPACE_SIZE` or observation-tensor contract changes. No-approximations policy (CLAUDE.md §17) applies throughout: every player choice stays exposed via `pending_selection`.

## Capabilities

### New Capabilities

- `dna-omnimon-archetype-coverage`: Every card in the DNA Omnimon decklist pool is faithfully implemented as DSL YAML with behavioral-test coverage that reflects card text; no behavioral test is ignored for a substrate gap that is already closed; an accurate per-card verdict ledger exists; `raw_rust` escapes are minimized to a documented, justified set.
- `dsl-inherited-substitute-trash`: The DSL can express an inherited-source replacement clause that substitutes "return this card to the bottom of the deck" for the would-be trash of an inherited (digivolution-source) card, with the multi-pick cost and atomic cost-then-cancel guard exposed as player choices.

### Modified Capabilities

<!-- None. No existing specs in openspec/specs/; all capabilities here are new. -->

## Impact

- **Cards**: `code/digimon-engine/cards/{bt22,bt17,st2,bt5,ad1,ex5,...}/*.yaml` — 5 new files, plus clause additions across cards with stale-ignore reconciliation.
- **Tests**: `code/digimon-engine/tests/cards_behavioral/**` — ~112 `#[ignore]` markers re-evaluated; new behavioral tests for the 5 missing cards.
- **Engine/DSL**: `code/digimon-dsl/` and `code/digimon-engine/src/dsl_cards/` — the `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` primitive (parser + compiled step + lowering), plus any residual substrate gap the sweep proves real.
- **Trackers/QA**: `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md`, and a populated `validated_cards_dsl.json`.
- **No impact** on PyO3 bindings, action/tensor contracts, the Python legacy engine, or the FastAPI server.
