# Phase 2 Track I — Puppets Pilot Completion

You are unblocking the Puppets pilot archetype (36 stuck cards as of 2026-05-17). Puppets is the third-largest stuck pile and has the most heterogeneous blocker profile of any pilot — many one-card-shaped substrate or DSL gaps documented in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` as `PUPPETS-G001` through `PUPPETS-G032`.

Has a hard consumer dependency on Track B (`activation_cost(...)` builder) — PUPPETS-G023 and PUPPETS-G028 are explicit consumers. Independent of Tracks A, C–H.

## Why this matters

Puppets is one of the named meta archetypes in `data/deck_library.json`. 36 cards stuck. The blocker tags by ref count:

| Tag | Refs | Type | Resolution |
|---|---:|---|---|
| **PUPPETS-G023** | 6 pending | suspend-this-Tamer cost on event observer | Track B downstream |
| **PUPPETS-G003** | 4 pending | effect-played permanent cleanup provenance | engine gap (separate ProvenanceToken cleanup work) |
| **PUPPETS-G008** | 3 pending | inherited opp-security DP aura DSL bridge | DSL |
| **PUPPETS-G009** | 3 pending | standard Delay Main-phase action | engine gap (substrate edge per RUST_ENGINE_GAPS.md) |
| **G-OPPONENT-SECURITY-DP-AURA** | 3 | DSL sibling of PUPPETS-G008 | DSL |
| **G-CARD-PRED-DP-LTE** | 3 | (sibling of G-PRED-DP-LTE, closed by Track A) | un-ignore sweep |
| **G-COSTED-SELF-DIGIVOLVE-STABLE-SOURCE** | 3 BLOCKED | UNCLEAR — needs first-test write | EX9-032 |
| **PUPPETS-G028** | 3 BLOCKED | return-this-Tamer cost | Track B downstream |
| **G-UNION-ZONE-PLAY-FROM-ORIGIN** | 2 pending | DSL: play from union of zones | DSL |
| **G-HAND-TRASH-CARD-DP-FILTER** | 2 pending | DSL predicate for hand/trash card DP | DSL |
| Long tail (PUPPETS-G014, PUPPETS-G024/G025, PUPPETS-G029, PUPPETS-G030, PUPPETS-G031, PUPPETS-G032) | ~10 | mixed | per-card |

Plus the two UNCLEAR substrate items from the 2026-05-15 audit: EX9-032 (G-COSTED-SELF-DIGIVOLVE-STABLE-SOURCE) and EX4-074 (end-of-attack mandatory chain) both have "first test recommended" footers in `docs/RUST_ENGINE_GAPS.md`. EX9-032 is a Puppet card; EX4-074 surfaced in Puppets resolver Batch 10.

Expected unblock after Tracks A + B + I: **~15 Puppets cards advance to IMPLEMENTED**.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17, §18.
2. `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` — full archetype gap doc, ~30 numbered PUPPETS-G entries. Read end-to-end.
3. `qa/qa-reports/validated_cards_dsl.json` — `"archetype": "Puppets"` (and Puppets Batch N variants).
4. `qa/dsl-vocab-gaps.md` — search PUPPETS-G023, PUPPETS-G028, G-OPPONENT-SECURITY-DP-AURA, etc.
5. `docs/RUST_ENGINE_GAPS.md` § "Standard Delay main-phase activation action" — PUPPETS-G009 substrate.
6. `docs/RUST_ENGINE_GAPS.md` § "Effect-played permanent cleanup provenance" — PUPPETS-G003.
7. `docs/RUST_ENGINE_GAPS.md` § "Costed self-digivolve stable source binding (UNCLEAR)" — EX9-032 first-test target.
8. `docs/RUST_ENGINE_GAPS.md` § "End-of-attack mandatory self-delete chain (UNCLEAR — EX4-074)" — EX4-074 first-test target.
9. `code/digimon-engine/src/aura.rs` — Track H aura substrate (PUPPETS-G008 DSL bridge consumes this).
10. `code/digimon-engine/tests/cards_behavioral/ex9/ex9_032.rs` (if exists) and `code/digimon-engine/tests/cards_behavioral/ex4/ex4_074.rs` (if exists) — the UNCLEAR first-test sites.

## Work to be done

### 1. Sequencing pre-check (Track B)

If Track B (`activation_cost(...)`) has landed: proceed with PUPPETS-G023, PUPPETS-G028 card-author migrations.
If Track B has NOT landed: defer PUPPETS-G023 and PUPPETS-G028 to a follow-up; close the other items.

### 2. UNCLEAR first-test writes — FREE WINS

#### EX9-032 (G-COSTED-SELF-DIGIVOLVE-STABLE-SOURCE)

Per the audit footer in `docs/RUST_ENGINE_GAPS.md` — write the failing test FIRST per the entry's own "First test" recommendation:

> Trigger EX9-032, delete a lower-index own Puppet as the cost, and assert the original EX9-032 stack digivolves into the selected hand Puppet.

If the test passes with existing substrate (`ctx.source_permanent` snapshot semantics + existing cost-payment): mark the entry RESOLVED, move to `qa/resolved-gaps.md`, no engine work needed.

If the test fails: file as a real BLOCKING substrate item with the failure mode documented. DO NOT fix in this PR — substrate work spawns its own track.

#### EX4-074 (end-of-attack mandatory chain)

Same approach. Test:

> Attack with EX4-074, resolve End of Attack with a Tamer and a legal opponent Digimon, and assert Ruin Mode deletes itself, the chosen opponent Digimon is deleted, Recovery +1 resolves, and a Digi-Egg is hatched only when the Tamer/breeding conditions are met.

Outcome: either RESOLVED + tracker move, or BLOCKING + new substrate track filed.

### 3. PUPPETS-G023 + PUPPETS-G028 (Track B consumers)

If Track B has landed: rewrite BT13-101, P-136, BT22-088 YAML to use `activation_cost: suspend_self` or `activation_cost: return_self_to_deck_bottom`. Confirm behavioral tests pass.

### 4. PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA (3 + 3 refs)

DSL: inherited opp-security DP aura. The Track H aura substrate supports security-zone-sourced auras (per the 2026-05-15 closure). PUPPETS-G008 is the author-facing DSL bridge for the opp-security flavor (the DP-debuff aura that fires from the opponent's security stack). Confirm the substrate's `target_player` and aura-scope fields support this; add DSL shape if missing.

### 5. PUPPETS-G009 — Standard Delay Main-phase activation action

Per `docs/RUST_ENGINE_GAPS.md` entry: standard Delay cards (Memory Boosts, Trainings, Scrambles) are currently scheduled as automatic end-of-turn fires rather than exposing the controller's `[Main]` decision to activate or decline.

This is a real substrate edge:

- Add a main-phase action that surfaces "activate this Delay Option" choice for each persistent Delay Option in the controller's battle area after the placing turn.
- The action's effect: trash the Option as cost, run the Delay body.
- Pass / decline leaves the Option in battle area for later activation.
- Action mask reuses existing pending-selection / field-effect surface — do NOT expand `ACTION_SPACE_SIZE`.

DCGO reference: `DCGO/Assets/Scripts/CardEffect/` — search Delay Option activation flow.

### 6. PUPPETS-G003 — Effect-played permanent cleanup provenance

Per `docs/RUST_ENGINE_GAPS.md` entry: ProvenanceToken half is wired (Track A), cleanup half pending. Add `EffectContext::schedule_delete_at_end_of_turn(token: ProvenanceToken, source: CardHandle)` and the `Game.scheduled_eot_deletions` queue drained inside `end_turn`.

Multiple cards depend on this: EX11-022, EX11-061, and Dark Masters family (EX10-012/020/035/057/061/072, P-216) via the sibling "Effect-spawned permanent EOT-deletion rider" gap. **Coordinate scope:** this PR closes the Puppet half; the Dark Masters half is its own card-author follow-up.

### 7. Long-tail PUPPETS-G entries

Walk through PUPPETS-G014, PUPPETS-G024/G025, PUPPETS-G029, PUPPETS-G030 (cross-references BT5-106 substrate gap "Effect play with played-Digimon On Play suppression" — UNCLEAR if absorbable here), PUPPETS-G031, PUPPETS-G032. Each is one-card-shaped; complete what's tractable, defer what's substrate-deep.

### 8. Card authoring walk

Walk Puppets cards in deck-pool order. Expect partial completion — Puppets has the most card-specific quirks of any pilot.

## Acceptance gates

- EX9-032 and EX4-074 first-tests written; each entry either RESOLVED-in-place or re-filed as concrete BLOCKING with failure mode.
- PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA DSL bridge land.
- PUPPETS-G009 Standard Delay Main-phase action lands without ACTION_SPACE_SIZE change.
- PUPPETS-G003 EOT cleanup half lands; existing Track A ProvenanceToken consumers wired.
- PUPPETS-G023 + PUPPETS-G028 card YAML migrated (if Track B landed).
- ≥ 10 Puppets cards advance to IMPLEMENTED.

## Constraints

- No-approximations: Standard Delay [Main] activation must be a player-visible action (Working Rule 17).
- Working Rule 1: no `ACTION_SPACE_SIZE` change. Delay-activate reuses pending-selection or field-effect surface.
- Working Rule 17: cleanup-via-EOT-deletion does NOT need to surface a player choice — the deletion is mandatory per printed text. But the WHICH permanent gets deleted must be uniquely traceable via ProvenanceToken (no over-broad scan).
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. Standard Delay Option timing is documented; cleanup is implied by "delete the Digimon this effect played".

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_032 ex4_074 bt13_101 p_136 bt22_088
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `docs/RUST_ENGINE_GAPS.md` — resolve EX9-032 / EX4-074 entries (one way or the other); resolve PUPPETS-G009 substrate; partially-resolve PUPPETS-G003.
- `qa/dsl-vocab-gaps.md` — close PUPPETS-G008, PUPPETS-G023, PUPPETS-G028 if Track B consumers migrated.
- `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` — annotate per-gap status with PR # citations.
- `qa/qa-reports/validated_cards_dsl.json` — advance Puppets cards.

## Order of operations

1. Sequencing pre-check (Track B status).
2. UNCLEAR first-test writes (EX9-032, EX4-074) — FREE WINS first.
3. PUPPETS-G008 DSL bridge.
4. PUPPETS-G023 / PUPPETS-G028 card-author migration (if Track B available).
5. PUPPETS-G003 ProvenanceToken cleanup half.
6. PUPPETS-G009 Standard Delay [Main] action.
7. Long-tail PUPPETS-G entries.
8. Card authoring walk.
9. Tracker hygiene + PR(s).

## Out of scope

- BT5-106 "Effect play with played-Digimon On Play suppression" (PUPPETS-G030 partially) — separate planned substrate.
- BT16-055 "narrow opponent-effect protection for DP reduction and De-Digivolve" (PUPPETS-G024/G025) — separate planned substrate.
- Counter Blast DNA (closed).
- The 12 BLOCKING items from `RUST_ENGINE_GAPS.md` not directly mentioned above — leave for separately-planned tracks.

## Discovery rider

Several Puppets cards (P-165 ShoeShoemon, the Dark Masters EOT cleanup family) need both ProvenanceToken cleanup AND token-creation provenance binding. If you find that PUPPETS-G003 closure requires the token-bound provenance shape, scope-creep risk is real — defer P-165 to a follow-up rather than expanding this PR.

If a Puppets card surfaces a NEW gap (not in PUPPETS-G001..G032), file under a new PUPPETS-G033+ ID in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md` and leave the card PARTIAL.
