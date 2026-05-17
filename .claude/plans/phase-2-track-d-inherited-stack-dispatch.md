# Phase 2 Track D — Inherited Triggered-Effect Dispatch from Digivolution Stacks

You are extending `code/digimon-engine/src/effect_queue.rs::enqueue_from_permanent` to walk every permanent's digivolution-stack `card_sources` and dispatch triggered effects sourced from those inherited card-source instances — not just from the top card, linked Plug-Ins, and Training option permanents that the current scan covers. This closes the **G-INHERITED-DISPATCH** substrate residue.

This track is independent of Track A (DSL eval-arm sweep) and Track B (`activation_cost(...)` builder). It has a *sequencing* dependency on Track C (OPT-slot enforcement): if Track D lands first, the OPT activation-count keying must be stable across the new inherited dispatch sources, but the current activation-count machinery doesn't track inherited-source identity correctly. Either land Track C first, or coordinate explicitly with the Track C author.

## Why this matters

A huge family of inherited triggered observers — "[When Digivolving]" / "[On Play]" / "[When Attacking]" / "[All Turns] When …" effects printed on Lv.3–5 Digimon that have evolved into something larger — silently fail to fire today because `enqueue_from_permanent` only scans the carrier's *top card* and a couple of side-channels (linked Plug-Ins, Training options). The stacked sources beneath the top card never see the trigger.

This is the architectural anti-pattern Rocks, BG Imperial, DNA Omnimon, and Medusamon archetypes all routinely hit. The audit's projected 107 refs has been pared down by adjacent PRs to **27 BLOCKED + ~14 pending refs across ignored tests** — but those remaining refs are concentrated in the densest part of the test tree (per-archetype "inherited [When Attacking] OPT lock", "inherited OnDigivolve trait-filter", "inherited security-removed observer" tests).

## Tags to close

| Tag | Refs | Where it's broken |
|---|---:|---|
| **G-INHERITED-DISPATCH** | ~27 BLOCKED + ~14 pending | `enqueue_from_permanent` scan scope omits `card_sources` |
| **G-WHEN-DIGIVOLVING-DISPATCH** | 4 BLOCKED | sibling — When Digivolving fan-out on inherited sources |
| Various combo tags (`G-INHERITED-DISPATCH + G-OPT-TRIGGERED`) | ~10 | will pass once both this track and Track C land |

Expected unblock: **~25 tests** become passable post-Track-D in isolation; another **~10 tests** become passable when combined with Track C.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no-approximations: every inherited-source observer that fires must surface its choices through pending_selection — there should be no hidden auto-fire for "you may" inherited clauses).
2. `docs/RUST_ENGINE_GAPS.md` — search "Inherited triggered-effect dispatch: `enqueue_from_permanent` must walk digivolution stack" — this is the RESOLVED entry; read it to confirm what was claimed closed and what's actually still open (per the test-tag residue, the closure was incomplete).
3. `qa/archetype-qa/engine-gaps.md` § "Inherited triggered-effect dispatch" — shadow notes.
4. `code/digimon-engine/src/effect_queue.rs:1246` (`enqueue_from_permanent`) — read end-to-end. Note every existing dispatch site (top-card, linked_cards, Training-option). The new walk slots in alongside these.
5. `code/digimon-engine/src/permanent.rs` — `Permanent::card_sources: Vec<CardSource>`. The stack we need to walk.
6. `code/digimon-engine/src/card_source.rs` — `CardSource` already supports inherited effect lookups via `effects_for_inherited(...)` or similar. Confirm the inherited-effect enumeration API.
7. `code/digimon-engine/src/trigger_context.rs` — confirm `TriggerContext.source_permanent` vs `source_card` semantics for inherited-source effects. The inherited-source effect's *source_card* identifies the stacked card; *source_permanent* is the carrier on the field. Many predicates depend on this distinction.
8. `code/digimon-engine/src/effect_queue.rs` — find existing tests for inherited dispatch already passing (e.g., `inherited_source_fires_on_digivolve` in `tests/timing_dispatch.rs`) to understand the scan invariants you must preserve.
9. Three failing test exemplars:
   - Medusamon: pick one with `"BLOCKED: G-INHERITED-DISPATCH"` (grep `code/digimon-engine/tests/cards_behavioral/`).
   - DNA Omnimon: pick one with the same tag.
   - A `G-WHEN-DIGIVOLVING-DISPATCH` test (4 BLOCKED across Medusamon Batch 8 — see `qa/qa-reports/validated_cards_dsl.json` for which cards).
10. DCGO reference: `DCGO/Assets/Scripts/CardEffect/` — DCGO walks the full stack via `IBattleAreaPermanent.AllEffects()` enumeration. Use as tiebreaker for ordering: top-down vs bottom-up matters for security-trigger interactions.

## Work to be done

### 1. Audit the current scan scope

Document explicitly what `enqueue_from_permanent` currently scans:

- The carrier's top card (via `effects_for_card(top_card_id)`).
- Linked Plug-In cards (per Track I substrate).
- Training option permanents bound to this carrier.
- Anything else?

Cite the file:line of each scan site. Then enumerate the missing dispatch target: the carrier's `card_sources[..]` (every stacked source card beneath the top).

### 2. Add the digivolution-stack walk

In `enqueue_from_permanent`, after the top-card scan and before the existing linked/Training scans (or wherever fits the existing convention), add:

```rust
// Walk inherited digivolution sources.
for (source_idx, source) in permanent.card_sources.iter().enumerate() {
    let inherited_effects = self.inherited_effects_for_card(source.card_handle, ...);
    for (slot, effect) in inherited_effects.iter().enumerate() {
        // Filter to triggered effects matching this trigger source.
        if !effect.timing_matches(trigger_source) { continue; }

        // Enqueue with carrier=current permanent_handle, source_card=source.card_handle.
        self.queue.push_back(QueuedEffect {
            card_id: source.card_handle.card_id,
            source_card: source.card_handle,
            source_permanent: Some(permanent_handle),
            source_kind: TriggerSource::PermanentBattleArea(...),
            effect_slot: slot,
            ...
        });
    }
}
```

(Pseudocode — match the real `QueuedEffect` constructor pattern from existing dispatch sites.)

Key invariants:

- **Source-card identity preserved.** The triggered effect's `source_card` MUST be the stacked card, not the carrier's top card. Predicates that read `event_card_name_contains` on the source card need this.
- **Source-permanent identity correct.** The `source_permanent` IS the carrier — that's what allows position-on-field aware predicates to work.
- **OPT slot key stable.** This must match the slot-key shape Track C aligned. Confirm with Track C author / read Track C's diagnosis note before designing the keying.
- **Trigger payload identical.** Whatever `TriggerContext` payload the top-card dispatch builds, the inherited dispatch builds the same.
- **No double-fire.** If a card's effect happens to live BOTH on its printed top-card text and inherited from a stack below, we should fire it once per *source slot*, not once per *card identity*. Verify with an authored test.

### 3. Ordering

The existing scan order is: top → linked → training. Add the stack walk in a defined position. Suggested: **top → stack (top of stack first, then descending) → linked → training**. This mirrors DCGO's `AllEffects()` enumeration order.

Document the ordering in a comment on the walk loop. Several archetype tests depend on deterministic ordering for multi-effect resolution.

### 4. Filtering by trigger source

`enqueue_from_permanent` is called from many sites (`fire_on_digivolve`, `broadcast_on_enter_field_anyone`, etc.). For each call site, ensure the inherited walk respects the timing filter — an `OnLeaveField` trigger should not fire an inherited `[When Attacking]` effect. This is mostly automatic if the filter is centralized in the effect-enumeration helper, but verify.

### 5. Un-ignore tests

For every test tagged `BLOCKED: G-INHERITED-DISPATCH`, un-ignore and confirm it passes. For combined tags `BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED`:

- If Track C has landed: un-ignore and confirm pass.
- If Track C has not landed: trim the tag to `BLOCKED: G-OPT-TRIGGERED` and leave `#[ignore]`'d.

For `BLOCKED: G-WHEN-DIGIVOLVING-DISPATCH` tests: these are the 4 Medusamon Batch 8 cards. After the digivolution-stack walk lands, they should pass.

### 6. Write a regression test

In `code/digimon-engine/tests/timing_dispatch.rs` (or wherever the existing inherited-dispatch tests live), add a test exercising:

- A 3-card stack (top: Greymon, middle: Garurumon, bottom: Agumon) where Agumon prints an `[On Play]` inherited effect.
- Play the carrier from hand → inherited [On Play] fires for Agumon (the carrier didn't change top, but the trigger needs to walk).

Wait — `[On Play]` doesn't traditionally walk the stack at play time (you don't fire inherited On Plays of materials when their host is played). Re-read printed text: inherited [On Play] effects do NOT fire at carrier-play time. They DO fire when the carrier *digivolves into* something else — for "[When Digivolving]" — and they fire continuously for "[All Turns]" / "[Your Turn]" timings.

Adjust the regression test accordingly: use `[All Turns] When …` or `[Your Turn] When …` shapes that genuinely should fire from inherited sources. The DCGO reference confirms these timings walk the stack.

## Acceptance gates

- `enqueue_from_permanent` walks `permanent.card_sources` with documented ordering.
- The three pinned failing tests pass.
- Net `#[ignore]` count drops by at least 20 (more if Track C has landed).
- New regression test added under `tests/timing_dispatch.rs`.
- No regression in `tests/cards_behavioral`, `tests/option_flow`, `tests/replacements`, `tests/combat`.
- The PR description cites every existing dispatch site that was already passing and confirms they still pass.

## Constraints

- No-approximations: inherited "you may" effects must surface optionality through `pending_selection`, not auto-fire.
- Working Rule 1: tensor / action / mask contracts unchanged.
- Working Rule 9 (state filter): inherited-effect activation must not leak the contents of opponents' face-down digivolution sources to the network client. Test that `state_filter.py` (Python side) still redacts inherited card metadata for opponents — engine-side dispatch should fire correctly even if the network layer redacts what it broadcasts.
- Do NOT change OPT-slot semantics in this PR — that's Track C. Just ensure your enqueue produces slot keys Track C can consume.
- Do NOT change the `linked_cards` / Training scan order — preserve existing behavior.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. DCGO confirms top-down ordering and per-source-slot keying.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_queue
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements
cargo test --manifest-path code/digimon-engine/Cargo.toml
git grep -c '#\[ignore' code/digimon-engine/tests | awk -F: '{s+=$2} END {print s}'
```

## Tracker discipline

- `docs/RUST_ENGINE_GAPS.md` — search "Inherited triggered-effect dispatch" and update the closure note. The 2026-05-15 sweep claimed RESOLVED; this PR completes that closure.
- `qa/archetype-qa/engine-gaps.md` — close shadow entries for G-INHERITED-DISPATCH and G-WHEN-DIGIVOLVING-DISPATCH.
- `qa/dsl-vocab-gaps.md` — sweep references.
- `qa/qa-reports/validated_cards_dsl.json` — Medusamon Batch 8 cards should advance. DNA Omnimon inherited-source cards should advance.

## Order of operations

1. Coordinate with Track C author (or read Track C's diagnosis note) on slot-key shape.
2. Audit scan scope (write PR-description audit paragraph).
3. Add the `card_sources` walk to `enqueue_from_permanent`.
4. Run pinned failing tests — confirm pass.
5. Write the timing_dispatch regression test (inherited [All Turns] fan-out).
6. Sweep tag annotations across the test tree (un-ignore G-INHERITED-DISPATCH, trim combo tags, leave Track-C-dependent ones alone if Track C hasn't landed).
7. Tracker hygiene + PR.

## Out of scope

- OPT-slot enforcement (Track C).
- New observer timings or `TriggerSource` variants.
- DSL surface changes (no new step or predicate).
- Card YAML changes.
- Inherited-effect lookup performance optimization — this walk is O(stack_depth) per dispatch, generally tiny; do not pre-cache unless a benchmark surfaces a real cost.

## Discovery rider

If, while auditing, you discover that the existing top-card / linked / Training scan also has bugs (e.g., dispatches the wrong `source_card` for one of them), document it in the PR but do NOT fix in this same PR unless the fix is one-liner-trivial. A scope-creep fix here can stretch the PR review beyond Track D's clear boundaries.

If the new walk surfaces a regression in an unrelated test (e.g., `tests/option_flow` test now over-fires because some Plug-In observer was de-facto only firing once due to the missing walk), STOP and investigate. The regression is informative — it may mean a different observer was relying on the missing dispatch as a load-bearing bug. Document the finding before patching.
