# Close Track G (Keyword Library)

You are completing the keyword library in the Rust game engine at `code/digimon-engine/`. Most of Track G is already landed — 16 keywords have `keyword_to_auto_effect` arms in `code/digimon-engine/src/cards/keyword_effects.rs`, and another 11+ are wired as direct `has_keyword(...)` consults in `combat.rs` / `game_phases.rs` / `game.rs`. Test coverage is organized in phased folders: `tests/combat/`, `tests/keyword_phase_d/`, `tests/keyword_phase_e/`, `tests/keyword_phase_f/`, `tests/replacements/`, `tests/dsl/`. Your job is to close the residual gaps and finish the audit.

## Why this matters

Card-shaped DSL fixtures and the Track L production YAML migration depend on every keyword being either fully wired or explicitly out-of-scope. The remaining gaps are small but real:

* `<Digi-Burst N>` parses but installs nothing — any printed Digi-Burst card silently no-ops.
* `<Decoy>` ignores its color-filter parameter — color-filtered Decoy cards substitute for the wrong attack color.
* `<Progress>` consults work but have no dedicated card-shaped test for inherited Progress + stack-depth interactions.
* `<Evade>` has an arm but no test file.
* Audit-table tracker entries are stale relative to the phase D/E/F landings.

These are cheap but non-trivial: each could mask a wrong-outcome bug for a real card.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules 17–22 (no-approximations, TDD via DebugRunner, parity tracker check). Source priority: printed text + Comprehensive Rules Manual + fandom wiki come before DCGO.
2. `docs/RUST_ENGINE_API.md` — current `EffectContext` / `Effect` builder / `CardEffect` trait. Note the `select_count_capped_multi` helper used by `Fragment(N)` — `Digi-Burst N` will reuse this.
3. `code/digimon-engine/src/cards/keyword_effects.rs` — the canonical big-match for keyword auto-effects. Read end-to-end before adding `Digi-Burst`. Pay close attention to:
   * `Fragment(N)` arm at line ~197 — the closest shape for `Digi-Burst N` (also a select-N-from-stack pattern, but as `[Main]` activation cost rather than leave-field replacement).
   * `Decoy` arm at line ~563 — current parameterless implementation; comment at line 559 explicitly notes "Color/parameter filtering is NOT in scope here".
   * `Training` arm at line ~1489 — the closest `[Main]`-activation-shaped keyword reference for Digi-Burst's overall flow.
4. `code/digimon-engine/src/enums.rs` — `Keyword::DigiBurst(u8)` (line 396), `Keyword::Decoy` (line 400), `Keyword::Progress` (line 417), `Keyword::Evade` (line 409). Note the `(u8)` payload pattern from `Fragment(u8)` and `MaterialSave(u8)` for the Decoy color-filter refactor.
5. `code/digimon-engine/src/card_data.rs:578` and `dsl_cards/modifier_map.rs:372` — Digi-Burst parsing entry points. Already wired; the missing piece is the `keyword_to_auto_effect` arm.
6. `code/digimon-engine/src/game.rs:1965-2000` — `progress_excludes()` Progress consult. Existing test at line ~2814 (`progress_excludes_only_when_attacking_and_opponent_sourced`) is unit-level; you'll need card-shaped fixture coverage on top.
7. `code/digimon-engine/tests/keyword_phase_f/` — the most recent phase batch. Match its file structure and test naming for any new test files you add. `helpers.rs` and `main.rs` are the entry points.
8. `code/digimon-engine/tests/keyword_phase_d/fragment_n.rs` — the closest test-shape reference for the Digi-Burst test you'll write.
9. `code/digimon-engine/tests/keyword_parsing.rs:542` — the existing Digi-Burst parse test. Confirm it round-trips `<Digi-Burst N>` for N ∈ {1, 2, 3} before adding effect-side tests.
10. DCGO C# reference for processing order and emitter shape only — printed text wins on disagreements:
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Decoy.cs` (70 lines) — color-filter parameter shape.
   * `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Progress.cs` (110 lines) — Progress mechanics.
   * `DCGO/Assets/Scripts/Script/Effects.cs` — search for `DigiBurst` references.
   * DCGO does not have a dedicated `Evade.cs`; nearest references are `Decode.cs` and `Barrier.cs`.
11. `data/cards.json` — search for cards using each affected keyword.

## Work to be done

1. **`<Digi-Burst N>` `keyword_to_auto_effect` arm** — add `[Main]` activation gate, select N stack sources, trash, then run printed `[Main]` body.
2. **`<Decoy>` color-filter parameterisation — only if cards exist** — change enum to `Keyword::Decoy(Vec<Color>)`, update parser + arm + every `has_keyword` site.
3. **`<Progress>` card-shaped test backfill** — native, granted, inherited, stack-depth, modifier-expiry, negative-scope.
4. **`<Evade>` test backfill** — battle deletion, effect deletion, decline, self-scope, empty-deck edge.
5. **Audit-table tracker hygiene** — update `docs/RUST_ENGINE_GAPS.md` and `qa/archetype-qa/engine-gaps.md`.
6. **Card-shaped fixtures** — Digi-Burst, Progress inherited, Evade, optional parameterised Decoy.

## Acceptance gates

* `Keyword::DigiBurst(N)` parses, installs `[Main]` activation, selects N sources, trashes, runs printed body, respects once-per-turn.
* `<Decoy>` honors filter or has documented audit entry.
* `<Progress>` covered for native/granted/inherited/stack-depth/modifier-expiry/negative-scope.
* `<Evade>` covered for both deletion causes, decline, self-scope, empty-deck.
* `RUST_ENGINE_GAPS.md` reflects actual code state.
* All four card-shaped fixtures pass.
* Every player-visible choice surfaces through `pending_selection`.

## Constraints

* No-approximations: every choice surfaces through `pending_selection` and the action mask.
* Do not expand `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, frontend constants, or RL wrappers.
* Source priority: printed text > Rules Manual > fandom wiki > DCGO.
* TDD discipline: failing test before implementation.
* No Python-side card scripts (Working Rule 21).
* No imports from `code/engine_py_legacy/` (Working Rule 22).
* Match `keyword_effects.rs` conventions exactly.
* Decoy parameterisation is highest-risk; drop §2 if no cards need it.
* If a builder method doesn't exist, add it to `effect.rs` — no stubs.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_d
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_e
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_f
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Currently 2067 cards_behavioral tests passing. Adding fixtures should push count up; nothing should regress.

## Tracker discipline

* `docs/RUST_ENGINE_GAPS.md` — update "Native printed keyword parsing" entry; add/close entries for Digi-Burst, Decoy color-filter, Progress inherited, Evade backfill, Memory Boost clarification.
* `qa/archetype-qa/engine-gaps.md` — close any keyword entries the phase D/E/F batches landed but didn't update.
* `qa/dsl-vocab-gaps.md` — only if you added DSL schema for Digi-Burst.
* `qa/archetype-qa/dsl/*.md` rollups — narrow / close entries waiting on Digi-Burst, Decoy color-filter, or Progress inherited semantics.
* `docs/RUST_ENGINE_API.md` — add `Effect::digi_burst_activation(N)` builder method to API reference.

## Order of operations

1. `Digi-Burst` arm + builder + parse confirmation + one fixture (highest impact, real bug fix).
2. Audit-table tracker hygiene (§5) — half-hour cleanup.
3. Progress test backfill + fixture — audit-style work, low risk.
4. Evade test backfill + fixture — audit-style work, low risk.
5. Decoy color-filter parameterisation (§2) — only if cards need it.

## Initial scope-deciding greps

```
grep -l "Digi-Burst" data/cards.json
grep -lE "<Decoy [\[\(]" data/cards.json
```
