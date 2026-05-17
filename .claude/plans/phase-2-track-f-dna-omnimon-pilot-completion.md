# Phase 2 Track F — DNA Omnimon Pilot Completion

You are closing the DSL surface gaps blocking the DNA Omnimon pilot archetype (29 stuck cards as of 2026-05-17) after Tracks A–D land. The remaining gaps after Track A's eval-arm sweep absorbs the easy ones are concentrated in a few DSL verbs and one substrate edge specific to effect-initiated digivolve with a permanent-target chain.

Independent of Tracks E, G, H. Has a soft dependency on Track A — if Track A lands first, the residual tag list here shrinks by ~25 refs (G-PRED-DP-LTE, G-COUNT-GTE-NOT-EVALUATED, G-FORMULA-SOURCE-DP, G-DSL-DISTINCT-TAMER-COLORS-FORMULA, G-ALT-PATH-CONDITION sweep). The plan below assumes Track A is in flight or done.

## Why this matters

DNA Omnimon scored 29 PARTIAL/BLOCKED cards in `validated_cards_dsl.json`. Top remaining tag refs after Track A absorbs the eval-arm tags:

| Tag | Refs | Type |
|---|---:|---|
| **G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM** | 10 | DSL verb (engine helper exists) |
| **G-ALT-PATH-DIRECTION-INTO** | 5 | DSL alt-path schema gap |
| **G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH** | 4 | DSL verb |
| **G-DSL-GAIN-MEMORY-FN** | 4 | DSL formula-valued step |
| **G-DSL-HAS-ON-DELETION-EFFECT** | 3 | DSL permanent predicate |
| **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET** | 3 | engine substrate edge |
| **G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA** | 3 | DSL formula (sibling of TAMER-COLORS) |

Expected unblock after this track + Track A: **~25 DNA Omnimon cards advanced to IMPLEMENTED**, ~30 ignored tests freed.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17, §18.
2. `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` — full archetype gap doc with per-card pressure points and the curated 11-item reusable gap backlog. Read end-to-end.
3. `qa/qa-reports/validated_cards_dsl.json` — search `"archetype": "DNA Omnimon"` for `PARTIAL`/`BLOCKED` cards. Notes describe per-card blockers.
4. `qa/dsl-vocab-gaps.md` — search for each tag in the table above. Each has a "Suggested DSL syntax" block describing the author-facing shape.
5. `code/digimon-engine/src/effect_context/mod.rs` — `place_as_bottom_source`, source-stack helpers (G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM is a DSL bridge to this existing helper).
6. `code/digimon-engine/src/effect_context/selections.rs` — `select_source` and source-binding APIs.
7. `code/digimon-engine/src/dsl_cards/alt_path.rs` and `code/digimon-dsl/src/alt_path.rs` — alt-path schema. G-ALT-PATH-DIRECTION-INTO is a new schema field for "this card may digivolve INTO X" inverse direction (ST20-10 Agumon-shape).
8. `code/digimon-engine/src/dsl_cards/predicate.rs` — site of G-DSL-HAS-ON-DELETION-EFFECT permanent predicate.
9. `code/digimon-engine/src/dsl_cards/formula_eval.rs` — site of G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA.
10. `code/digimon-engine/src/game_actions.rs` and `effect_context/mod.rs::effect_initiated_digivolve` — site of G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET substrate edge.
11. DCGO references for the heavyweight DNA Omnimon top-ends only as tiebreaker:
   - `DCGO/Assets/Scripts/CardEffect/BT17/.../BT17_078.cs` (Counter Blast DNA)
   - `DCGO/Assets/Scripts/CardEffect/EX9/.../EX9_021.cs` (Omnimon Alter-S)
   - `DCGO/Assets/Scripts/CardEffect/BT22/.../BT22_015.cs` (printed Decode)

## Work to be done

Each item below is shippable in isolation. Suggested order is roughly low-risk → high-risk; consider splitting into 2–3 PRs if one item turns deep.

### 1. `G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM` (10 refs)

Engine helper exists (`EffectContext::place_top_source_as_bottom(target)` or similar — find it). Missing: DSL verb that lowers to it. Add to `code/digimon-engine/src/dsl_cards/step/sources.rs` (or wherever stack-manipulation steps live):

```yaml
- place_top_source_as_bottom:
    target: source     # or any permanent binding
```

Add `CompiledStep::PlaceTopSourceAsBottom { target: PermanentRef }` variant; lowering bridge; variant-coverage compliance.

### 2. `G-ALT-PATH-DIRECTION-INTO` (5 refs)

ST20-10 Agumon-shape: "This card may digivolve INTO X" (rather than alt-path's default "X may digivolve from this card"). Add a `direction: AltPathDirection` field to `AltPathSpec` in `code/digimon-dsl/src/alt_path.rs`, defaulting to current behavior. New variant `AltPathDirection::IntoTarget` makes the registered alt-path apply from this card's perspective as the candidate-from-hand side.

Threading through `code/digimon-engine/src/dna_digivolve.rs::find_matching_alt_path` and `code/digimon-engine/src/game.rs::digivolve_from_hand` so the mask emitter and decode validator both honor the inverse direction.

### 3. `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH` (4 refs)

The DSL needs an inherited-clause verb that substitutes "instead of trashing this card, do X" — a sibling of the leave-field replacement framework, but scoped to an inherited source card's would-be trash. Pattern from `qa/dsl-vocab-gaps.md`. Lowers onto Track B (replacement framework, already landed). DSL surface only.

### 4. `G-DSL-GAIN-MEMORY-FN` (4 refs)

Formula-valued `gain_memory: { formula: ... }` so the memory delta can be derived (e.g., "gain memory equal to count of own [Color] Digimon"). Add formula evaluator integration; the engine `EffectContext::gain_memory(amount)` already takes a scalar — accept the formula at compile time, evaluate per resolution.

### 5. `G-DSL-HAS-ON-DELETION-EFFECT` (3 refs)

DSL permanent predicate `has_on_deletion_effect: true` that returns true if the target permanent has any registered `OnDeletion`-timed effect. EX1-021 / sibling-card use. Implement in `predicate.rs` consulting `effects_for_card(...).iter().any(|e| e.timing == OnDeletion)`.

### 6. `G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA` (3 refs)

Sibling of the Track A `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` arm. Same shape, different scope (both players, all permanents). Add to `formula_eval.rs`.

### 7. `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` (3 refs — substrate edge)

This is the only real substrate edge in the DNA Omnimon residue. BT16-040 Wormmon and sibling shape: an effect plays a Digimon by digivolving from hand INTO an already-bound permanent target (rather than the player picking the digivolve target at action time).

Current `EffectContext::effect_initiated_digivolve(...)` takes the source permanent but doesn't accept a pre-bound target chain. Extend:

```rust
pub fn effect_initiated_digivolve_with_target(
    &mut self,
    target: PermanentHandle,  // bound earlier in the effect
    hand_index: usize,
    ...
) -> Option<PermanentHandle>
```

Threading the bound target through the existing digivolve helpers, firing standard OnDigivolve observers, preserving `effect_initiated: true` and `dna_origin` payload bits. Add DSL surface (effect-initiated digivolve verb with `target: <binding>`).

### 8. Author DNA Omnimon production YAML for the unblocked cards

Walk the per-card list in the DNA Omnimon gap doc § "Core Archetype Cards" + the PARTIAL entries in `validated_cards_dsl.json`. For each card whose remaining blocker is now closed, complete YAML + behavioral test. Pace yourself — expect ~10–15 cards in one session.

## Acceptance gates

- All 6 DSL gaps closed: verbs/schema/predicate/formula land with parse + lowering + behavioral coverage.
- G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET substrate edge closed; BT16-040 test (or equivalent) passes.
- ≥ 10 DNA Omnimon cards advance from PARTIAL/BLOCKED → IMPLEMENTED.
- `dsl_eval_arm_coverage` lint still passes.
- No regression in `cards_behavioral`, `dsl`, `dna_digivolve`, or `digivolve` test suites.

## Constraints

- No-approximations: G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET's bound-target version skips the "which target?" player choice that's normally exposed — but only because the target was *already* a player choice earlier in the effect (bound via `select_own_permanent`). Verify the binding genuinely traces back to a player choice; do not allow this surface to skip selection arbitrarily.
- Working Rule 1: no tensor/action contract churn.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. For ST20-10-style inverse-direction alt-paths, printed text is unambiguous on "may digivolve into".

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `qa/dsl-vocab-gaps.md` — close each closed-tag entry; relocate to `qa/resolved-gaps.md` under "Phase 2 Track F closure — 2026-05-XX".
- `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` — annotate each closed item with the PR # in the existing per-item status notes.
- `qa/qa-reports/validated_cards_dsl.json` — advance DNA Omnimon cards as YAML completes.
- `docs/RUST_ENGINE_GAPS.md` — only G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET is a canonical engine entry (search for the gap title). Mark resolved.

## Order of operations

1. The 4 DSL eval-arm/verb items (G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM, G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH, G-DSL-GAIN-MEMORY-FN, G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA) first — low risk, batch in one commit.
2. G-DSL-HAS-ON-DELETION-EFFECT predicate.
3. G-ALT-PATH-DIRECTION-INTO schema extension + threading.
4. G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET substrate edge.
5. Card authoring walk.
6. Tracker hygiene + PR(s).

## Out of scope

- Counter-window Blast DNA (already closed per DNA Omnimon gap doc § 1, with 2026-05-08 update).
- Decode keyword (closed for BT22-015 / EX4-060 / EX9-021).
- Force-follow-up-attack (closed for BT20-102 / BT22-015 / AD1-009 / EX9-013 via `may_attack_now`).
- Hand-resident observer fan-out (separate engine gap; tracker).
- Source-scoped immunity (separate engine gap).
- `BT15-102` Apocalymon cast-time stack-construction (one-card, separate planned).

## Discovery rider

DNA Omnimon makes heavy use of inherited effects, so if Track D (inherited dispatch) hasn't landed when you start, some failing tests will be Track-D-blocked rather than substrate-blocked. Identify Track-D-blocked tests early and leave them ignored. Do NOT bring Track D's work into this PR.
