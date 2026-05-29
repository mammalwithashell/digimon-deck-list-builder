## Context

`unblock-mastemon-rust-dsl` closed the best-deck Mastemon path: 20 unique best-deck cards now have production YAML and behavioral tests, and the card work did not expand `ACTION_SPACE_SIZE` or change active tensor contracts. The full resolved `Mastemon (Tribal)` pool is larger: 93 unique cards across 55 decklists. Current pool coverage is 36 cards with production YAML; 57 cards remain without production YAML, and only `BT13-106` among those has an existing behavioral test file.

The remaining pool is mixed. Many cards are straightforward uses of existing patterns (`play_from_security`, DP modifiers, search/reveal, inherited keyword grants, Recovery, Retaliation, Barrier, Partition, Delay). Several high-frequency cards are blocked or risky without reusable substrate:

- CS cards such as `BT22-004`, `BT22-043`, `BT22-044`, `BT22-054`, and `BT22-093` need an event surface for effects that place CS Digimon cards into digivolution sources.
- `BT15-038` and `BT15-042` need a player-visible "trash top or bottom security as a cost" gate.
- `EX8-064` needs multi-card play from trash constrained by total play cost.
- `BT10-042` needs conditional attack and effect suppression keyed to opponent Digimon with Security Attack.
- `BT11-043` likely needs temporary name/original-name mutation in addition to color/base-DP changes.

## Goals / Non-Goals

**Goals:**

- Make the full resolved `Mastemon (Tribal)` pool authorable in Rust DSL without no-op stubs, hidden auto-selections, or raw-Rust card escapes.
- Close reusable DSL/engine substrate before authoring cards that depend on it.
- Preserve player-visible choices through existing action IDs and `PendingSelection` flows.
- Keep best-deck readiness intact while expanding coverage to lower-frequency and cross-archetype tech cards.
- Maintain gap trackers and readiness notes so unresolved substrate is capability-centric, not a pile of card-local TODOs.

**Non-Goals:**

- Do not rework the completed best-deck card YAML unless a regression is discovered.
- Do not complete unrelated archetypes just because their cards overlap the Mastemon pool.
- Do not change `ACTION_SPACE_SIZE`, active tensor layouts, or RL model compatibility as a side effect of this change.
- Do not author approximated YAML for cards whose printed choices or mutations are not faithfully expressible.

## Decisions

### Substrate before card batches

Implement the reusable blockers first, each with a focused failing Rust test under `code/digimon-engine/tests/`. Only after a blocker is proven should dependent cards be authored.

Alternative considered: batch all 57 cards immediately and leave comments for blocked clauses. That would produce a larger diff but would blur the difference between coverage, substrate blockers, and approximation risk.

### Add a source-placement observer event instead of card-local hooks

The CS package repeatedly asks "when effects place Digimon cards with the [CS] trait in this Digimon's digivolution cards." This should be modeled as a reusable effect event emitted by the source-placement helpers, with event payload for placed card, host permanent, owner, cause, and effect-created provenance. DSL predicates should be able to inspect the placed card trait and host/source relationship.

Alternative considered: wire each CS card as a local tail after the specific placement effects in Mastemon. That fails for cross-card observers and would miss placements created by other cards.

### Model security costs as costs, not ordinary steps

Top-or-bottom security trash on `BT15-038` and `BT15-042` is a cost that gates the effect. The DSL should expose a choice-shaped cost gate that offers top and bottom when legal, records whether the cost was paid, and skips the body if declined or unpayable. This should reuse the existing security-trash and result-gate style from `if_trash_top_security_cost`.

Alternative considered: use `select_effect_choice` followed by `trash_top_security` or `trash_bottom_security`. That is acceptable only when the trash is an effect body, not when it is the printed activation cost gating the rest of the effect.

### Generalize aggregate play-cost budget selection

`EX8-064` needs "play up to 10 play cost's total worth of [NSo] Digimon cards from trash." This should be a visible multi-pick selection over a zone-card source with a remaining play-cost budget, followed by batch play that respects the selected cards' original origins. The existing opponent-permanent play-cost-budget selector is useful precedent but not enough by itself.

Alternative considered: approximate by picking one card with play cost <= 10 or by playing in greedy order. Both hide legal player choices.

### Treat Venusmon locks as a modifier/filter capability

`BT10-042` has two unusual pieces: Security Attack -1 grants to opponent Digimon, and a lock preventing opponent Digimon with Security Attack from attacking Venusmon or activating `[When Attacking]` / `[When Digivolving]`. The implementation should prefer reusable conditional modifiers or timing filters over special-casing Venusmon.

Alternative considered: skip only the lock and author the Security Attack -1. That would be partial coverage, not full-pool readiness.

### Keep identity mutation narrow

`BT11-043` changes an opponent Digimon into a white Digimon with 3000 DP and original name `Sukamon` until the end of the opponent's turn. Existing modifier support appears to cover color and base-DP changes, but not temporary original-name mutation. Add only the minimum identity modifier needed to make rule predicates and display/debug state treat the permanent as having the temporary original name during the expiry window.

Alternative considered: only change color/DP and ignore the name. That breaks cards that care about names and is not faithful.

## Risks / Trade-offs

- Source-placement observer fan-out can easily double-trigger if both source helpers and caller-specific code emit events. Mitigation: centralize emission in placement helpers and add negative tests for ordinary digivolve/source trash paths.
- Security cost gates can accidentally run tails after a declined or impossible cost. Mitigation: mirror the successful-cost result-gate tests used by Mastemon best-deck substrate.
- Aggregate play-cost selection may need new pending-selection shape. Mitigation: first attempt to reuse existing selection ranges; if new action IDs are required, stop and split action/tensor contract work into a separate proposal.
- Venusmon's lock may interact with timing dispatch and action masks in subtle ways. Mitigation: add mask-level tests plus direct timing-dispatch tests proving only the gated permanent's matching timings are suppressed.
- Full-pool cards include unrelated tech cards from other archetypes. Mitigation: group by Mastemon pool frequency and shared substrate, then record out-of-scope residuals as follow-up only when they are low-frequency and unrelated to full-pool readiness.
