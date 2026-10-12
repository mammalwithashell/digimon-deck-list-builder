## ADDED Requirements

### Requirement: Scoped execution preserves complete source membership
The snapshot producer SHALL independently reconstruct complete official, secondary, canonical, runtime and QA inventories and retain evaluated, blocked and unknown members with reasons. An executable subset MUST NOT redefine global membership, retire unrelated cards, or convert unknown denominators to zero.

#### Scenario: Executable and book files omit the same card
- **WHEN** a caller removes an outside-pool inventory member from both supplied artifacts
- **THEN** validation rejects the mismatch against independently derived original membership
- **AND** no executable handle is created

### Requirement: Execution and evidence closure are explicit and reproducible
The system SHALL compute executable E and evidence Q to a fixed point from exact target/opponent decks, full named rest decks, setup/stack/card references, enabled implementation-generated objects, pool variants and complete shared QA associations. Every edge SHALL bind its reason and source digest. Unresolved indirect references SHALL block the affected capability.

#### Scenario: Scenario header omits a card in its named rest deck
- **WHEN** the real deck resolver includes that card in the scenario remainder
- **THEN** the card is added to E and requires source and implementation validation

#### Scenario: Shared question is only visible in the reverse association
- **WHEN** the denominator's card-to-question mapping associates an E member with a shared question
- **THEN** Q retains every member and complete question/answer even if the row's own card list is incomplete
- **AND** required executable auxiliaries are expanded or block dispatch

### Requirement: Runtime records are freshly derived and exactly reviewed
Admitted records SHALL be generated from complete immutable authority inputs using canonical normalization, typed reviewed corrections, explicit source selections, stable registry ownership and the normal forward projection. Every consumed field and derived conversion SHALL have current review evidence. Runtime-row self-comparison, copied candidate rows, unknown-field defaults and successful global-failure catching MUST NOT establish approval.

#### Scenario: Blocked global candidate contains a plausible runtime row
- **WHEN** that row is supplied without source-derived regeneration and complete consumed-field proof
- **THEN** isolated admission refuses it

#### Scenario: Exact admitted record uses a corrected Dual or Rule field
- **WHEN** a fresh projection and its whole source/face/condition containers pass current independent review
- **THEN** the record can become source-ready with original conflicting observations preserved
- **AND** source readiness does not imply behavioral readiness

### Requirement: Sealed scoped artifacts cannot impersonate publication
The producer SHALL seal a distinct immutable schema and marker binding all input, code, graph, registry and pack identities. It SHALL verify completion atomically and readers SHALL reject concurrent changes. Global publication, default runtime paths and default pool contracts SHALL remain unchanged and reject the scoped artifact as a global revision.

#### Scenario: Same counts but changed correction or producer
- **WHEN** a correction, producer, pack or registry differs from the sealed dependency identity
- **THEN** loading refuses before execution even if card counts and filenames match

#### Scenario: Scoped snapshot passed to global apply
- **WHEN** an operator supplies the scoped artifact as a publication candidate
- **THEN** apply refuses without replacing any fixed-path file

### Requirement: Isolated consumers fail before outside-scope execution
An opted-in consumer SHALL load one explicitly selected verified snapshot with no fallback. It SHALL preserve global card indices and observation/action contracts and reject all outside-E decks, lookups, generated objects, variants and swap choices before their execution. Only separately validated token/template capabilities SHALL create non-real game objects. Registered DSL code alone SHALL NOT grant scope admission.

#### Scenario: A legal-looking action or setup introduces an unreviewed card
- **WHEN** its card ID is absent from E or its required token capability is absent
- **THEN** the isolated run fails with the source and ID identified before the operation is applied
- **AND** it does not substitute a dummy or global record

#### Scenario: Snapshot profile omitted
- **WHEN** an ordinary existing consumer is run without isolated opt-in
- **THEN** its existing default data, build, publication and pool behavior remains unchanged

#### Scenario: New snapshot differs only in same-length metadata
- **WHEN** a process loads verified snapshots A, B and A again, differing only in same-length text, evolution costs, Dual data, aliases or normalized IDs
- **THEN** its effective card store corresponds to the new exact snapshot
- **AND** every cache is keyed by the exact snapshot/content and consumer identity or bypassed for the isolated handle
- **AND** it cannot inherit incompatible data, effects, registries or enriched rows through a length-only cache identity

### Requirement: Required evaluation and denominator membership is exact
The snapshot SHALL declare required evaluation IDs from E and every Q member whose semantics the campaign requires, exact clause IDs, full QA member sets and reverse associations, probe families and versions with all required positive/negative cases, structural obligations, and promoted gating IDs. These sets SHALL equal independently reconstructed requirements, not merely another supplied book. Required unknown, blocked, missing or unclaimed gameplay semantics SHALL refuse measurement activation. An evidence-only role SHALL have explicit validated semantics or expand into required evaluation; otherwise it SHALL block. Known focused totals and unknown outside-required global totals SHALL remain distinct.

#### Scenario: Both books omit the same auxiliary probe
- **WHEN** the supplied clause and interaction books both omit a required auxiliary member or negative probe while retaining its global inventory entry
- **THEN** independent requirement reconstruction rejects activation
- **AND** retaining the inventory row alone does not satisfy the required denominator

### Requirement: Campaign evidence is bound to an append-only generation
Activation SHALL require coherent source-book, clause, gating-interaction, scenario and verdict contracts for the exact required denominator. Current evidence SHALL bind source generation, semantic schema/hash, runtime snapshot, engine binary and pack, consumer/dependency identity, exact scenario/deck inputs and complete interaction identity. All five existing oracle verdict classes and historical results SHALL remain available. A change in any binding component SHALL make effective evidence unmeasured until new measurement; a history mapping MUST NOT copy terminal successes into a new generation.

#### Scenario: A corrected source changes one clause fingerprint
- **WHEN** a new campaign generation is prepared
- **THEN** the prior generation and all verdicts remain intact
- **AND** the affected clause and interactions start unmeasured until current evidence is recorded

#### Scenario: Global denominator has not been evaluated
- **WHEN** the focused book is complete but other inventory members remain unknown
- **THEN** reports show the known full focused denominator and unknown global denominator separately
- **AND** neither is described as global accuracy

#### Scenario: Visible text unchanged but executable identity changes
- **WHEN** the pack, binary or a relevant consumer dependency changes while the visible card text is identical
- **THEN** evidence bound to the prior identity cannot count as current confirmation

### Requirement: Generation activation is drained and crash-safe
The controller SHALL place STOP, verify actual workers and claims have drained, acquire the existing shared coordination lock, and compare the exact predecessor tip before activating a successor. Successor sealing and activation SHALL be atomic or recoverably staged and idempotent. Unresolved STARTED work, ancestor questions, cumulative usage and all prior evidence SHALL remain retained and explicitly accounted for. No worker SHALL dispatch against a mixed or unactivated generation.

#### Scenario: Crash after successor sealing before activation
- **WHEN** the controller stops between writing the sealed successor and selecting it as active
- **THEN** resume identifies the exact predecessor and pending successor under the lock
- **AND** it either completes the same idempotent transition or retains the predecessor without duplicated accounting or dispatched work

#### Scenario: Active worker or predecessor changes during preparation
- **WHEN** a claim is still active or the predecessor tip differs at activation
- **THEN** activation refuses and STOP remains in place until reconciliation and drain complete

### Requirement: Training admission is bounded and behaviorally demonstrated
An opted-in Pilot or Architect run SHALL pin the snapshot, exact allowed decks/opponents/variants/swaps, observation/action contracts and pack in run/model metadata. Admission SHALL require current implementation, legal player-visible choices, current passing behavioral/oracle evidence, independent review and no active relevant blocker. The route SHALL reject incompatible resume/inference and SHALL NOT widen default pools or claim arbitrary-meta readiness.

#### Scenario: Architect proposes a registered card outside the reviewed closure
- **WHEN** the proposal would introduce that card
- **THEN** the bounded action pool excludes it and execution refuses an injected action

#### Scenario: Only source proofs and smoke games pass
- **WHEN** required behavior or gating-interaction measurements are incomplete
- **THEN** neither deck is reported training ready

#### Scenario: Exact bounded configuration completes all gates
- **WHEN** source, implementation, choice, behavioral/oracle and independent-review gates pass for the full required closure
- **THEN** that pinned configuration can be admitted to training
- **AND** broader deck or policy-strength claims require separate evidence
