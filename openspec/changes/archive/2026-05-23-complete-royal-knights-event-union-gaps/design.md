## Context

`close-royal-knights-substrate-gaps` closed the first Royal Knights substrate slice and left the archetype with full YAML/test inventory but several cards still incomplete. The remaining gaps are not random card-local problems: they group around event context and union/source operations that multiple Royal Knights cards need.

Current readiness pressure from the Royal Knights audit is 31 complete, 33 partial, and 8 blocked cards. The most important remaining blockers include `BT13-019`, `BT20-021`, and `EX11-053`, with related partial-card pressure from `BT20-056`, `EX11-069`, and event-observer cards such as `BT15-084`, `BT20-060`, `BT23-035`, `BT23-047`, `BT8-090`, `BT9-092`, `BT13-095`, `BT21-086`, and `RB1-035`.

Printed text and rules remain authoritative. DCGO may be used as an implementation reference, but it must not override printed optionality or introduce hidden selections.

## Goals / Non-Goals

**Goals:**

- Close reusable event-context primitives for remaining Royal Knights observers.
- Close reusable union/source primitives for cross-zone selections, source-placement costs, source-play effects, and attach-self follow-ups.
- Complete the targeted Royal Knights blocked/partial cards whose only remaining blockers are those primitive families.
- Keep every gameplay choice represented through existing action masks or pending selections unless a separately planned action/tensor contract update is proven necessary.
- Update gap trackers so remaining Royal Knights gaps name current reusable primitives rather than stale card-local TODOs.

**Non-Goals:**

- Do not sweep every remaining Royal Knights aggregate/formula gap unless it is directly required by the scoped event or union cards.
- Do not solve the full security Option lifecycle surface unless a scoped card requires only a narrow security event payload.
- Do not change `ACTION_SPACE_SIZE`, tensor layout, or RL contracts as a side effect. If the scoped work truly needs that, stop and plan the contract change explicitly.
- Do not use raw Rust placeholders, no-op YAML, or auto-selected targets to claim card completion.

## Decisions

### Decision: Split event context and union/source flows into separate capabilities

The change creates `royal-knights-event-context-coverage` and `royal-knights-union-source-flows` as separate specs. They are implemented together because the remaining archetype work needs both, but they remain separate contracts so future archetype audits can reuse either one without inheriting the other.

Alternative considered: a single broad Royal Knights completion capability. That would be easier to write but would blur two distinct substrate families and make later gap tracking less precise.

### Decision: Normalize event payload access before adding more trigger predicates

Event predicates and effect bodies SHALL read from one normalized trigger context rather than each observer adding its own ad hoc fields. This context needs enough payload to answer scoped predicates such as the event target being the source permanent, a played Digimon's level, a same-level X digivolution, and security cards being removed, added, or trashed.

Alternative considered: lower each card to bespoke Rust checks. That would unblock individual cards quickly but would leave the DSL unable to express the same printed patterns on future cards.

### Decision: Model heterogeneous union choices as stable candidate descriptors

Union/source selections SHALL present candidates from different zones through one pending-selection shape with stable candidate identity, zone, carrier, and source-card handles. This is especially important for `BT13-019` trash-or-breeding-source play, `BT20-021` hand-or-trash source placement as cost, and `EX11-053` hand-or-source play.

Alternative considered: create separate selection prompts per zone. That is simpler, but it changes printed "choose from A or B" effects into serial choices and can distort optionality, name uniqueness, and action-mask legality.

### Decision: Treat source-placement costs as atomic gates

Effects that place a card as a digivolution source as a cost SHALL only continue after the selected cost is paid. If no legal cost candidate exists, the follow-up effect SHALL NOT be offered as payable. If the effect is optional, PASS remains legal according to printed text.

Alternative considered: resolve the effect body first and clean up the source placement afterward. That risks applying benefits after an unpayable cost and hides the true legal choice from action masks.

### Decision: Bind played or placed cards for follow-up clauses

Source-play and effect-play operations SHALL expose handles for cards they successfully played or placed so follow-up clauses such as attach-self, keyword grants, or Rush grants can target only the intended cards.

Alternative considered: re-query the board after the operation. That can over-target when multiple matching Digimon exist or when multiple cards entered play during the same effect chain.

## Risks / Trade-offs

- [Risk] Heterogeneous union selection may need action IDs not currently available. -> Mitigation: first prove whether existing pending-selection/action ranges can represent the choices; if not, pause and plan an action/tensor contract change.
- [Risk] Event predicates may overfire from incomplete event payload distinctions. -> Mitigation: add negative tests for unrelated played, removed, added, trashed, and digivolved cards.
- [Risk] Source-card handles can become stale after movement between zones. -> Mitigation: resolve candidates at prompt time and validate handles again at selection resolution.
- [Risk] The change could sprawl into unrelated aggregate/formula gaps. -> Mitigation: keep aggregate work out of scope unless it is required by a named event/union target card.
- [Risk] Tracker closeout can become stale if ignored tests are not reconciled. -> Mitigation: every re-enabled or still-ignored Royal Knights test must cite a current primitive and tracker entry.

## Migration Plan

1. Reconcile the targeted Royal Knights ignored tests, YAML gap markers, and tracker entries against current engine/DSL behavior.
2. Add failing behavioral tests for event-context primitives and union/source primitives before implementation.
3. Implement event-context payload and predicate support, then migrate the targeted observer cards that no longer need placeholders.
4. Implement union/source pending-selection and operation support, then migrate the targeted blocked union cards.
5. Re-run the targeted Rust engine card tests and the Royal Knights archetype QA commands.
6. Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and Royal Knights QA notes with closed primitives and any remaining blockers.

Rollback is ordinary source rollback for engine, DSL, tests, and card YAML. No data migration or external dependency is expected.

## Open Questions

- Can all scoped heterogeneous union choices reuse existing action and pending-selection ranges, or is an action/tensor contract proposal required?
- Does `EX11-053` need to play from any matching source under a controller's Digimon, or only from a source context tied to a specific Royal Knights carrier after printed-text review?
- Which security event payloads are already present but under-exposed to DSL predicates, and which require engine event expansion?
- Should the same event-context primitives be documented generically after archive, or remain Royal Knights-scoped until another archetype needs them?
