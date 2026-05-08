# Track E — Zone Movement and Source/Material Operations

## Overview
Implement a complete set of zone-movement primitives in the Rust game engine at `code/digimon-engine/`. Centralise zone movement so every move routes through observer-safe APIs, returns stable handles, and surfaces every player-visible choice through `pending_selection`.

## Why this matters
Many printed cards move cards between hand, trash, security, deck, breeding, source stacks, and battle area in ways the current engine cannot express:
- Effect-driven plays from hand / trash / security with cost override or On-Play suppression.
- Effect-driven digivolves from hand, trash, source, security, or mixed material zones.
- Cross-permanent source selection — choosing a specific digivolution source on any of your permanents and trashing or playing it.
- Security stack ops: trash top, place top/bottom face-up/face-down, top security to hand, self-to-security, stacked-card-to-security, Recovery, shuffle.
- Bulk moves: return all trash to deck bottom, trash top N digivolution cards of every opponent permanent, forced opponent hand reduction.
- Cast-time stack construction — placing N differently-named cards from battle-area or trash UNDER the played card during the play step itself.
- Effect-played permanent cleanup provenance — knowing which permanent was played by which effect so end-of-turn cleanup or On-Play-suppression hits the right card.
- Move from breeding to battle by effect (currently only natural hatch).
- Trash all digivolution cards of a permanent (unbounded stack-peel).
- Pop top source from a named permanent.
- Reveal-zone overlay so a card's type/level read while in deck or being revealed reflects the synthesised state.

## Read these first (in order)
1. `CLAUDE.md` — Working Rules 17–22 (no-approximations, TDD via DebugRunner, parity tracker check). Source priority: printed text + Comprehensive Rules Manual + fandom wiki come before DCGO.
2. `docs/RUST_ENGINE_API.md` — current `EffectContext` / `Effect` builder / `CardEffect` trait. Note any existing zone-movement helpers (`play_from_hand`, `return_to_hand`, `trash_security_top`, `select_own_sources`, etc.) so you extend rather than fork.
3. `code/digimon-engine/src/effect_context.rs`, `game.rs` (zone-mutation sites and security ops), `permanent.rs` (battle-area + breeding + suspended/active state), `card_source.rs` (digivolution stack model + inherited dispatch), `card_data.rs` and `card_registry.rs` (card metadata + registry), `selection.rs`, `action/mask.rs`, `enums.rs` (zone enum, `Timing` variants relevant to zone moves) — the surfaces you will modify.
4. `code/digimon-engine/tests/` — existing zone-move tests. Search for `play_from`, `return_to`, `trash_security`, `select_own_sources`. New tests follow the same shape; you will create or extend `code/digimon-engine/tests/zone_movement.rs`.
5. `docs/RULES_CONTEXT.md` — §3 zones, §6 playing cards, §7 digivolution, §10 security, §16 keywords (specifically Decode, Save, Material Save, Decoy interactions with movement).
6. DCGO C# reference for processing order and helper shape only — printed text wins on disagreements:
   - `DCGO/Assets/Scripts/Script/CardSource.cs`
   - `DCGO/Assets/Scripts/Script/Permanent.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/IsDigivolvedByTheEffect.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/RevealLibrary.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/SelectAssemblyClass.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/CanSelectAssemblyClass.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs`
   - `DCGO/Assets/Scripts/Script/CardEffectCommons/TrashLinkedCards.cs`
   - `DCGO/Assets/Scripts/Script/MainPhaseAction/PlayCardAction.cs`
   - `DCGO/Assets/Scripts/Script/SecurityObject.cs`, `SecurityBreakGlass.cs`
7. Cross-archetype gap reports (skim only):
   - `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md`
   - `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md`
   - `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md`
   - `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
   - `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`
   - `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md`
   - `qa/archetype-qa/dsl/millenniummon.md`
   - `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`
   - `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md`
   - `qa/archetype-qa/dsl/alter-s-ladder-2026-05-03.md`

## Work to be done
1. Stable handle discipline — `PermanentHandle`, `SourceHandle(carrier_handle, source_id)`. No raw indices in public helpers.
2. Owner vs. controller split — owner-routed deck-bottom/top placement, return-to-hand of opponent-controlled permanents.
3. Centralised movement helpers on `EffectContext` — play, digivolve, source/material, return, security, bulk/specialised. (See full list in original prompt.)
4. Provenance and cleanup tokens — `EffectMoveToken` carrying handle, source effect, originating zone, optional cleanup hook.
5. Cast-time stack construction — `cast_time_assembly` step inside play helper, before `OnPlay` dispatch, after cost calculation.
6. On-Play suppression — scoped to specific permanent via provenance token; global observers still fire.
7. Reveal-zone overlay — keyed by card identity, consulted by predicates, torn down on resolve.
8. Effect battles — `cause = NonBattleDeletion`, no attack-only movement triggers.
9. DSL schema and lowering — YAML verbs for new helpers + selectors.
10. Card fixtures — BT13-112, BT5-106, BT17-077, EX10-032, BT12-028, EX11-022/Puppets, G-RH-02, G-RH-06, EX4-060/BT22-015, EX9-021, BT24-031/BT24-101, G-ASL-03, P-130, G-BG-02, cast-time stack-construction card, owner-routing fixture.
11. Tests — framework unit tests in `code/digimon-engine/tests/zone_movement.rs` + card-shaped behavioral tests.

## Acceptance gates
- No movement helper takes/returns raw `Vec` index; all use stable handles.
- Security observers fire correctly on visible departures.
- Source movement removes exact selected source even after battle-area shifts.
- Effect-created permanents return stable provenance tokens.
- Owner routing correct when control differs from ownership.
- No raw `Vec` mutations; all routing through observer-safe APIs.
- Cast-time stack construction installs sources before `OnPlay`.
- `suppress_on_play` scoped to specific permanent.
- Reveal-zone overlay visible to predicates while active.
- DNA-origin context populated by DNA digivolves.
- Every player-visible choice surfaces through `pending_selection` and the action mask.

## Constraints
- No-approximations: every player-visible choice through `pending_selection`. No silent auto-selection.
- Do not expand `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, frontend constants, or RL wrappers.
- Source priority: printed text wins over DCGO.
- Don't transliterate. Sync resolution + pending-selection state machines, not C# coroutines.
- TDD discipline: failing test before implementation.
- Working Rules 21, 22: no Python-side card scripts; no imports from `engine_py_legacy`.
- Movement paths are primary write surface. File gaps for missing event payloads, modifier variants, replacement windows, or selection shapes the engine cannot already produce.

## Verification
```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_movement
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- play digivolve security source
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- relevant `qa/archetype-qa/dsl/*.md` rollups
- `docs/RUST_ENGINE_API.md` for public helper signatures
