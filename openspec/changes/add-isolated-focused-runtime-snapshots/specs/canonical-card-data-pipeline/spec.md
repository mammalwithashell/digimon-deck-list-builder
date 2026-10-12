## MODIFIED Requirements

### Requirement: Synchronization plan, apply, and check modes are scoped and non-destructive

An isolated focused runtime snapshot SHALL use a distinct immutable artifact type and completion marker. It MUST NOT be accepted as a global publication candidate, committed revision marker, fixed-path compatibility artifact, or apply input. Adding an isolated consumer MUST preserve the complete existing journal/hash/marker-before-and-after protocol for ordinary fixed paths; explicit scoped routing SHALL NOT fall back to an ordinary path after scoped validation fails.

The pipeline SHALL provide separate plan, apply, and check behavior. Plan SHALL stage and report a deterministic diff without changing committed artifacts and SHALL return a token binding requested scope, base committed revision, staged/diff digests, and reviewed adjudications. Apply SHALL acquire a repository-scoped exclusive writer lock, recheck the base revision immediately before publication, publish only after the entire requested scope passes validation, and preserve every out-of-scope source observation. Multi-file publication SHALL use a transaction journal with prior/new digests and rollback locations, per-file atomic replacement, and a committed-revision marker written last. Stale plans or concurrent writers MUST fail before replacement. Caught publication failures SHALL roll back; interrupted publication SHALL be detected and recovered or rolled back before later consumers proceed. Every fixed-path reader SHALL reject an incomplete journal, verify file hashes against a marker read before loading, and reread that marker afterward so a concurrent revision change causes retry or failure. Check SHALL be network-free and fail when fixed compatibility paths differ from the committed revision. Failed requested IDs, unexpected shrinkage, and unacknowledged removals MUST prevent publication.

#### Scenario: Plan does not mutate committed card data

- **WHEN** a set sync is run in plan mode and upstream additions or changes are found
- **THEN** the staged diff reports them
- **AND** no committed source, canonical, correction, or generated artifact changes

#### Scenario: Reviewed plan becomes stale before apply

- **WHEN** the committed revision changes after a plan token is created but before its apply begins publication
- **THEN** apply fails the base-revision compare-and-swap before replacing any fixed path
- **AND** the operator must create and review a new plan

#### Scenario: Two writers attempt apply concurrently

- **WHEN** two apply processes target the same repository
- **THEN** the exclusive writer lock permits at most one publication transaction
- **AND** the other process waits or fails without allocating indices or replacing files

#### Scenario: Set-scoped apply preserves other sets

- **WHEN** an apply succeeds for one release set
- **THEN** only that source scope's observation membership is replaced
- **AND** affected canonical IDs are recomputed from all retained observations across every scope
- **AND** every out-of-scope observation remains present and unchanged

#### Scenario: One gameplay ID appears in multiple product scopes

- **WHEN** a set-scoped apply replaces one product's observations for a gameplay ID that also has retained observations in another product scope
- **THEN** the other product observations remain attached to that canonical ID
- **AND** canonical recomputation does not discard them

#### Scenario: Requested fetch fails before publication

- **WHEN** one or more requested card observations fail to fetch, parse, normalize, or validate
- **THEN** apply exits unsuccessfully
- **AND** all previously committed artifacts remain unchanged

#### Scenario: Publication is interrupted after some replacements

- **WHEN** fault injection or process interruption occurs after one or more fixed paths are replaced but before the committed-revision marker is written
- **THEN** the incomplete journal causes subsequent sync, check, build, and release operations to refuse the mixed revision
- **AND** recovery restores the prior revision or completes the staged revision before those operations proceed

#### Scenario: Reader races with publication

- **WHEN** a runtime, build, or check reader begins while apply is replacing fixed paths
- **THEN** the journal/hash/marker protocol prevents it from accepting mixed-revision content
- **AND** it retries or fails cleanly rather than caching an uncommitted candidate revision

#### Scenario: Partial ID rendering cannot truncate a global artifact

- **WHEN** bundles or official records are rendered for an explicit subset of card IDs
- **THEN** the subset output is merged into its declared scope or written to a separate scoped destination
- **AND** the global official mirror is not replaced by that subset

#### Scenario: Offline check detects a hand edit

- **WHEN** a generated artifact is manually changed without updating its canonical inputs
- **THEN** network-free check mode fails and identifies the stale artifact

#### Scenario: Scoped artifact cannot become a global revision

- **WHEN** plan, apply, check or a default fixed-path reader receives an isolated snapshot marker or scoped runtime payload
- **THEN** it rejects the artifact type without changing committed state
- **AND** an invalid explicit isolated handle fails without fallback

### Requirement: Compatibility artifacts are deterministic products of one canonical revision

Every isolated snapshot SHALL additionally bind a prospective projection epoch: exact producer seed and freshly loaded implementation/dependency identities, complete original source/canonical/correction/selection/registry inputs, any explicitly owned compatibility-projection sidecar, and expected output bytes. Historical R3 candidate or proof bytes MUST retain their original epoch and MUST NOT be relabeled current. A changed projection implementation SHALL require fresh replay, full inventory and field-diff accounting, and current exact approvals even if source revisions or output counts are unchanged. The immutable isolated producer SHALL NOT use generated cards.json as authority or use runtime metadata to fabricate bootstrap provenance.

The pipeline SHALL deterministically generate `data/cards.json`, `data/card_official.json`, card bundles, and maintained lexicons from one canonical revision plus reviewed corrections and stable global-index assignments. A versioned compatibility-projection sidecar MAY carry only legacy ABI fields absent from the canonical schema and explicit non-gameplay fixture rows; it MUST bind its canonical, source-manifest, correction, registry, projection, and expected-output revisions and MUST NOT hide canonical real-card values. After producer cutover, generated `cards.json` SHALL be an output under comparison and MUST NOT be used as a regeneration or candidate-assembly input. Existing `cards.json` field names, value shapes, global indices, derived `norm_id`, and derived `card_index` semantics SHALL remain compatible during this change. Before replacing the legacy producer, shadow generation SHALL produce a field-by-field diff for every runtime-relevant value and MUST block cutover until each change is corrected or matched by a durable migration-adjudication entry recording card/field, old/new values or digests, source provenance, exact old/new input revisions, reviewer, reason, and review date. An input or value change SHALL stale the approval. Generated artifacts SHALL carry or be associated with schema and source-manifest revisions, use stable ordering, and derive timestamps from content-stable committed inputs rather than the current wall clock.

#### Scenario: Same inputs produce identical output

- **WHEN** generation runs twice from identical source observations, scope manifests, corrections, schema versions, and global-index registry
- **THEN** every generated compatibility artifact is byte-for-byte identical

#### Scenario: Existing cards.json contract is preserved

- **WHEN** canonical generation replaces the legacy cards.json producer
- **THEN** golden compatibility tests confirm existing keys, numeric/list/null shapes, card IDs, and stable index assignments remain compatible

#### Scenario: Post-cutover generation is independent of cards.json

- **WHEN** a committed `cards.json` row is hand-edited or resealed without changing its marker-owned canonical and compatibility-projection inputs
- **THEN** network-free generation independently reproduces the expected compatibility bytes
- **AND** freshness validation rejects the edited output

#### Scenario: Runtime field changes during shadow generation

- **WHEN** the canonical producer changes a color, kind, cost, trait, form, attribute, evolution route, or printed text value relative to the current generated runtime record
- **THEN** the cutover report identifies the exact field, old/new values, and source provenance
- **AND** producer cutover remains blocked until the change is corrected or matched by a current digest-bound migration-adjudication entry

#### Scenario: Engine build remains offline

- **WHEN** the engine, desktop application, hosted API, deck builder, or training runtime builds or starts
- **THEN** it consumes only committed generated artifacts
- **AND** it performs no live card-data network request

#### Scenario: Same source with a changed projection implementation

- **WHEN** producer or dependency bytes change while source observation revisions remain equal
- **THEN** a new projection epoch and fresh normal-producer equality check are required
- **AND** prior proofs remain historical and cannot automatically authorize the new output
