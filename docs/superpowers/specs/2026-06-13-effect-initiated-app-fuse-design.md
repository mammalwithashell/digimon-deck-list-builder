# Effect-Initiated App Fuse — Design

**Date:** 2026-06-13
**Status:** Approved (design); implementation pending
**Scope:** Add the effect-initiated App Fuse primitive (DSL `app_fuse` step + one
`EffectContext` entry point) so the "1 of your Digimon may app fuse into a Digimon
card …" rider on 5 Appmon cards becomes a real, faithful clause. Closes the
`docs/RUST_ENGINE_GAPS.md` "App Fuse keyword/primitive" gap for the effect-initiated
direction. The alt-play App Fusion route (digivolve method) is already implemented
and is **reused**, not rewritten.

## Problem

Five cards print a rider clause that the engine cannot express, so they ship
`PARTIAL` with the rider omitted (documented, never stubbed):

| Card | Rider clause | Source zone | Result filter | OPT |
|------|--------------|-------------|---------------|-----|
| BT21-084 Haru Shinkai | "Then, 1 of your Digimon may app fuse into a Digimon card in the hand." | hand | none | no |
| BT23-079 Eri Karan | "Then, 1 of your Digimon may app fuse into a Digimon card in the hand." | hand | none | no |
| P-241 Yujin Ozora | "Then, 1 of your Digimon may app fuse into a Digimon card in the hand." | hand | none | no |
| BT25-089 Kazuki & Itsuki | "[End of Your Turn][Once Per Turn] 1 of your Digimon may app fuse into a Digimon card in the hand." | hand | none | **yes** |
| BT24-087 Rei Katsura | "Then, 1 of your Digimon may app fuse into a Digimon card with the [System],[Life] or [Transmutation] trait in the trash." | **trash** | System/Life/Transmutation | no |

All five are `gap_kind: engine` in `qa/qa-reports/validated_cards_dsl.json`.

## What App Fuse actually does (DCGO-grounded)

Sources read: `DCGO/Assets/Scripts/Script/SelectAppFusionEffect.cs`,
`DCGO/Assets/Scripts/Script/CardSource.cs` (`CanAppFusionFromTargetPermanent`,
`appFusionCondition`), and the five cards' `CardEffect/*.cs`
(`BT23_079.cs`, `BT24_087.cs`, `BT25_089.cs`, `BT21_084.cs`).

Effect-initiated App Fuse = **play an App-Fusion-capable Digimon card *onto* one of
your field Digimon that already has the named App-Fusion materials linked to it.**
It resolves as two engine-driven selections followed by an app-fusion play:

1. **Select 1 own field permanent** (`SelectPermanentEffect`, `canNoSelect: true`).
   Eligible iff some result card in the source zone can app-fuse onto it.
2. **Select 1 result card** in the source zone (`SelectHandEffect`/trash,
   `canNoSelect: true`). Eligible iff `card.CanAppFusionFromTargetPermanent(perm, …)`.
3. **Play it via App Fusion** — DCGO:
   `PlayCardClass(selectedCard, …, selectedPermanent, …).SetAppFusion([frameID, linkIndex]).PlayCard()`.
   The result card stacks on top of the permanent and the consumed link card folds
   into the digivolution sources (printed Cost 0).

### Eligibility (`CardSource.CanAppFusionFromTargetPermanent`, CardSource.cs:3378)

A result card `C` can app-fuse onto permanent `P` iff:
- `C.appFusionCondition.digimonCondition(P)` — `P`'s **top card** matches one of
  `C`'s App-Fusion named conditions (name A), **and**
- there exists a linked card `L` on `P` with `C.appFusionCondition.linkedCondition(P, L)`
  — a linked card matches a **distinct** named condition (name B), **and**
- `C` is not under a "cannot evolve" restriction, and memory ≥ cost (cost is 0 here).

This is exactly the eligibility the alt-play route already computes:
`Game::app_fusion_host_eligible` (`dna_digivolve.rs:464`) +
`app_fusion_condition_names` (`dna_digivolve.rs:1011`) — top matches one name, a
linked card matches a different name. The `root` (zone) param defaults to `Hand`
and is passed `Root.Trash` by BT24-087, so **DCGO already parameterizes the source
zone** for eligibility.

### Consumed link (deterministic)

DCGO consumes the **first** linked card matching name B:
`selectedPermanent.LinkedCards.Where(x => …linkedCondition(perm, x)).First()`. There
is no player choice over *which* link is consumed (the named materials are fixed).
The Rust implementation mirrors this — no extra selection — consistent with the
existing alt-play app-fusion commit.

### DCGO quirk to honor

In `BT24_087.cs` the trash-sourced fusion still constructs
`PlayCardClass(…, SelectCardEffect.Root.Hand, …)` (line 197) even though selection
used `Root.Trash`. The card is removed from wherever the `selectedCard` reference
actually lives. **Implementation requirement:** remove the result card from its
*actual* zone (trash for BT24-087), not assume hand.

## Design

### 1. DSL step (`code/digimon-dsl/`)

New `StepSpec::AppFuse(AppFuseArgs)` with single-key map serde, matching the
existing step idiom:

```yaml
- app_fuse:
    from: hand            # `hand` | `trash`  (default: hand)
    result_filter:        # optional predicate on the result (fusing-in) card
      any_of:
        - trait_has: System
        - trait_has: Life
        - trait_has: Transmutation
    optional: true        # always true for these riders ("may"); default true
```

```rust
pub struct AppFuseArgs {
    #[serde(default)]                 // FromZone::Hand
    pub from: AppFuseZone,            // enum { Hand, Trash }
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_filter: Option<Predicate>,
    #[serde(default = "default_true")]
    pub optional: bool,
}
```

Lowers to `CompiledStep::AppFuse { from_zone, result_filter, optional }`. The step
takes **no explicit target binding** — both the permanent and the result card are
fresh engine-driven selections (matching DCGO; nothing to bind from prior steps).

The result card's App-Fusion materials are NOT authored in the rider — they live on
the result card's own `app_fusion` alt-path (already shipped, e.g. Rebootmon's
`[Bootmon] & [Shutmon]`). The step only needs zone + optional result-trait filter.

### 2. Engine entry point (`EffectContext`)

```rust
// code/digimon-engine/src/effect_context/action/app_fuse.rs  (new module)
impl EffectContext<'_> {
    pub fn initiate_effect_app_fuse(
        &mut self,
        from_zone: AppFuseZone,
        result_filter: Option<&CompiledPredicate>,
        optional: bool,
    );
}
```

Behavior:
1. Build the eligible-pair set: for each own field permanent `P`, for each result
   card `C` in `from_zone` where `C` passes `result_filter` **and**
   `C` has an `app_fusion` alt-path that is `app_fusion_host_eligible` against `P`.
2. If no eligible permanent exists → no-op (mirrors DCGO's
   `HasMatchConditionOwnersPermanent` guard; the whole clause silently does nothing).
3. Install **selection #1** over the eligible permanents (`SelectionKind::OwnField`
   variant; `optional` → PASS legal).
4. On pick `P` → install **selection #2** over result cards in `from_zone` eligible
   for `P` (a hand/trash selection kind; PASS legal).
5. On pick `C` → route through the existing app-fusion commit (see §3).

Both selections route through the standard `pending_selection` state machine — every
choice surfaces to the 2192-action space (no auto-pick), satisfying CLAUDE.md §17.

### 3. Resolution — reuse the existing app-fusion commit

The alt-play path already stacks an app-fusion card and folds the consumed link into
sources: `app_fusion_digivolve_route_for_card` (`dna_digivolve.rs:404`) →
`DigivolveRouteMatch::app_fusion` → the `is_app_fusion` branch in
`game_actions/digivolve.rs:467`. The effect-initiated path commits the chosen
`(P, C)` pair through the **same** commit, with one generalization:

- **Source-zone generalization (the one new piece of plumbing):** the existing
  commit pulls the result card from **hand**. Generalize the pull to remove `C` from
  its actual zone (`hand` or `trash`). This is the only material engine change; the
  stack/link-consume/source-fold logic is untouched.

Cost is 0 (App Fusion printed cost). No `from:` digivolution-requirement check — App
Fusion bypasses normal digivolve color/level requirements (it has its own
named-material condition), exactly as the alt-play route does.

### 4. Action-space / mask

Selection #1 reuses the own-field selection range; selection #2 reuses the
hand/trash selection ranges. No new action IDs — the two selections map onto existing
`SEL_OWN_FIELD_*` / `SEL_HAND_*` / `SEL_TRASH_*` ranges. Confirm the mask exposes
exactly the eligible entries (and PASS when `optional`).

## Testing

### Primitive-level (DebugRunner, `tests/` mechanic dir or inline fixtures)

1. **Hand fuse, happy path** — permanent with the two named materials linked + result
   card in hand → select perm → select card → assert: result card is the new top of
   the permanent's stack, the matching link card is now a digivolution source (not a
   link), result card left hand. Event-log assertion for the play/digivolve events.
2. **Trash fuse** — same but result card in trash; assert it left trash and stacked.
3. **Result-filter gating** — a trash result card failing the trait filter is **not**
   offered in selection #2 (and a passing one is).
4. **Ineligible permanent not offered** — a permanent lacking a matching linked
   material is absent from selection #1.
5. **Decline (optional)** — PASS at selection #1 → no fusion, no state change.
6. **Decline at card pick** — PASS at selection #2 → no fusion (permanent unchanged).
7. **No eligible permanent → silent no-op** — clause resolves with no selection
   installed.
8. **Distinct-name requirement** — a permanent whose top + link match the **same**
   single name (not two distinct names) is ineligible.

### Card-level (convert the 5 omitted riders to real clauses)

For each of BT21-084, BT23-079, P-241, BT25-089, BT24-087:
- Replace the documented-omission rider with an `app_fuse` step in the YAML.
- Add a behavioral test driving the full clause (the card's own trigger → … →
  app_fuse selections → fused result), plus the card-specific shape:
  - BT25-089: OPT lockout (second EoT fuse same turn gated; clears next turn).
  - BT24-087: trash source + System/Life/Transmutation filter.
  - BT23-079 / P-241: app_fuse runs **after** the prior rider effects (DP/Vortex
    grant) in the same `process` body.
- Flip each verdict `PARTIAL → IMPLEMENTED`, `gap_kind: engine → null`.

### Regression

Full `cargo test --manifest-path code/digimon-engine/Cargo.toml` green; the alt-play
app-fusion tests (`tests/cards_behavioral/bt25/app_fusion.rs`) must stay green
(the commit path is shared).

## Out of scope

- The alt-play App Fusion *digivolve method* (already implemented).
- DigiXros / DNA assembly (separate mechanics).
- Any App Fuse variant not on the 5 listed cards (e.g. opponent-initiated, multi-fuse).
  If a future card needs a different shape, extend `AppFuseArgs` then.

## Trackers to update on completion

- `docs/RUST_ENGINE_GAPS.md` — close the effect-initiated App Fuse gap entry
  (the alt-play sub-entry was already cleared 2026-06-12).
- `qa/qa-reports/validated_cards_dsl.json` — 5 verdicts `PARTIAL → IMPLEMENTED`.
- `qa/dsl-vocab-gaps.md` — record the new `app_fuse` DSL step as landed.

## Files touched (anticipated)

- `code/digimon-dsl/src/step.rs` — `AppFuseArgs`, `AppFuseZone`, `StepSpec::AppFuse`,
  serde.
- `code/digimon-dsl/src/compile.rs` + `compiled.rs` — `CompiledStep::AppFuse`.
- `code/digimon-engine/src/effect_context/action/app_fuse.rs` — new module,
  `initiate_effect_app_fuse`.
- `code/digimon-engine/src/dsl_cards/step/…` — lower `CompiledStep::AppFuse` →
  `initiate_effect_app_fuse`.
- `code/digimon-engine/src/dna_digivolve.rs` / `game_actions/digivolve.rs` —
  source-zone generalization of the app-fusion commit (hand|trash).
- The 5 card YAMLs + their behavioral test files.
- DSL schema regen (`cargo run -p dsl-schema-export`).
