## Context

The Xros Heart audit found that the Rust engine can express several nearby pieces, but not the central transaction that makes the archetype work. Production YAML currently covers only a thin slice of Xros Heart behavior, while example cards rely on comments or raw-Rust placeholders for DigiXros cost reduction, extra material zones, and Material Save.

DCGO models this as a cast-time DigiXros flow: `SelectDigiXrosClass` owns selected materials, extra trash/Tamer source counts, pre-attached materials, recipe validation, cost reduction, and source attachment. Taiki-style cards mutate that pending transaction before cost is fixed; Superior Mode-style cards select and pre-attach a Shoutmon, reduce cost, and unlock trash materials before payment.

Rust has reusable pending-selection infrastructure, source snapshots for deletion, and before-pay-cost hooks, so the design should add a Rust-native transaction around those seams instead of porting DCGO's object model directly.

## Goals / Non-Goals

**Goals:**

- Represent DigiXros play as a first-class Rust engine transaction with explicit material origins, recipe matching, cost math, source attachment, and context flags.
- Surface every player-visible choice through existing pending-selection/action-mask machinery.
- Let effects modify a pending DigiXros transaction before cost is paid, covering Taiki/Kiriha/Nene and Superior Mode-style cards.
- Fix `<Material Save X>` to trigger from deletion/removal timing and select printed recipe materials under a Tamer.
- Provide DSL vocabulary that can author the initial Xros Heart acceptance pool without raw Rust.
- Keep `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, and frontend action constants stable.

**Non-Goals:**

- Implementing every Xros Heart card in this proposal. This change defines the engine/DSL substrate and the first acceptance fixtures.
- Reworking normal digivolution, DNA, or Blast evolution flows except where shared helpers are naturally reused.
- Adding hidden auto-picks, UI-only choices, or card-specific approximations.
- Porting DCGO code verbatim; DCGO is a behavioral reference and shape guide.

## Decisions

### D1 - Introduce `DigiXrosTransaction`

Add a transaction value owned by the play-from-hand path while resolving a DigiXros alt path. It records the hand card being played, controller, recipe elements, selected material origins, pre-attached material origins, temporary material-zone allowances, cost delta per selected material, and final `digixros_count`.

Rationale: Xros Heart cards need multiple independent hooks to mutate one pending play. A transaction gives those hooks a shared, typed target and prevents cost calculation, material removal, and attachment from drifting into per-card ad hoc code. Alternative considered: compile each Xros Heart card as custom Rust. That would unblock individual cards but repeat the same transaction rules and violate the reusable-gap policy.

### D2 - Reuse existing pending selections and action IDs

Material selection, pre-attachment choices, Tamer picks, and Material Save picks SHALL use existing pending-selection rows where possible, especially union-zone and count-capped selectors. If a selection needs a new internal `SelectionKind`, it must still lower to existing action IDs or existing selection action ranges; this proposal does not authorize action-space expansion.

Rationale: RL legality is already tied to pending selections and action masks. Keeping choices inside that contract preserves the no-approximations policy without invalidating trained action/tensor consumers. Alternative considered: introduce dedicated DigiXros action IDs. That is out of scope because it would require `ACTION_SPEC.md`, tensor metadata, PyO3, frontend constants, and model compatibility work.

### D3 - Treat cast-time modifiers as transaction hooks

Before-pay-cost / when-would-play handlers that apply to the card being played can inspect and mutate the pending transaction. The hook API should support adding material zones/counts, adding pre-attached materials, adding one-shot cost deltas, and declining the modifier before it changes the transaction.

Rationale: DCGO's `UntilCalculateFixedCostEffect` maps cleanly to Rust's existing before-pay-cost seams, but the mutation target must be a typed transaction, not global game state. Alternative considered: model these as normal cost reducers only. That cannot express "select this Shoutmon as a source, then reduce cost and unlock trash."

### D4 - Commit materials only after cost payment succeeds

The transaction SHALL collect material choices and compute final cost before payment, but it SHALL move/attach selected materials only after the play cost is successfully paid and the permanent is created. Preattached material choices are committed in the same attachment pass.

Rationale: cost failure must not consume hand/trash/Tamer/field materials. This also aligns with the existing play action shape: identify legal play, pay cost, create permanent, then apply side effects. Alternative considered: remove selected materials eagerly during selection. That creates rollback complexity and incorrect visible state if payment fails.

### D5 - Material Save derives from deletion snapshots and recipe filters

`<Material Save X>` should run during deletion/removal replacement timing over the permanent's pre-removal source snapshot. It selects up to X source cards that satisfy the card's printed DigiXros recipe filters and places them under a chosen Tamer. If no Tamer or eligible source exists, no selection is offered.

Rationale: current Rust keyword treatment as `[Main]` is semantically wrong. Deletion snapshots already expose the source list before trash, which is exactly the data Material Save needs. Alternative considered: implement Material Save as an OnDeletion trigger after trash. That loses replacement-window semantics and can observe an already-mutated stack.

### D6 - DSL authoring mirrors reusable concepts, not card names

The YAML surface should declare `kind: digixros` alt paths, recipe material filters, material zone allowances, transaction modifiers, pre-attach selectors, and keyword options. The schema should avoid Xros Heart-specific keys except in card data predicates such as traits/names.

Rationale: Blue Flare and later DigiXros cards share the same concepts. Generic schema keeps the DSL useful beyond this archetype. Alternative considered: add one-off `xros_heart_*` fields. That would be easier to write initially but brittle and hard to reuse.

## Risks / Trade-offs

- **Pending-selection composition may not cover every material-origin shape** -> Start with BT10-009, BT10-087, BT12-112, and BT10-013 tests; if a missing selector shape appears, add the narrowest internal selector while preserving action IDs.
- **Before-pay-cost ordering is subtle** -> Write tests that prove transaction modifiers run before fixed-cost calculation and before any material movement.
- **Material Save is replacement-timing sensitive** -> Anchor implementation to the existing deletion snapshot/replacement framework and add tests for decline, no Tamer, and recipe-filtered source selection.
- **DigiXros and normal play share code paths** -> Keep the transaction optional and entered only through a matched DigiXros alt path; normal play cost behavior must remain covered by existing tests.
- **DSL schema can grow too broad** -> Validate only the acceptance-pool fields first; document unsupported fields as gaps rather than accepting no-op YAML.

## Migration Plan

1. Add failing Rust behavioral tests for the four acceptance cards and focused unit tests for transaction selection/cost/attachment.
2. Introduce `DigiXrosTransaction` and wire a single BT10-009-style play path through it.
3. Add transaction hook support for under-Tamer access and pre-attach/cost/trash-access modifiers.
4. Replace keyword `MaterialSave` handling with deletion/removal timing backed by snapshots.
5. Add DSL schema/lowering for the accepted transaction fields and migrate example Xros Heart specs into production YAML where tests pass.
6. Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md` as each reusable primitive closes.

Rollback: each slice is independently revertible. The transaction is opt-in for DigiXros alt paths, so reverting a later hook slice should leave normal play/digivolution unaffected.

## Open Questions

- Should recipe matching support duplicate same-name requirements as distinct slots in the first slice, or can the initial acceptance pool avoid that until a card requires it?
- Should the transaction expose a public `EffectContext` API only, or should some helpers stay internal to DSL lowering at first?
- Which Xros Heart cards beyond BT10-009, BT10-087, BT12-112, and BT10-013 should be the second acceptance batch after the substrate lands?
