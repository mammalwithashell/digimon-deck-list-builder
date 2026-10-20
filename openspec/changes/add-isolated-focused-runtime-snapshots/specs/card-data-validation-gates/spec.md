## MODIFIED Requirements

### Requirement: Generated artifacts carry and verify their input revisions

An isolated artifact SHALL bind the complete source, semantic, projection, runtime, registry, pack/release, consumer binary, code dependency, scenario/deck and clause/probe/QA/gating membership tuple. The dependency closure SHALL include text splitting and source-unit composition. Production in a fresh process SHALL verify loaded code identities; a disk-only rehash after importing older modules is insufficient. Unknown global denominator counts SHALL remain unknown; required focused unknowns SHALL block admission without dropping any original required member.

Generated card data, campaign manifests, pool artifacts, and card packs SHALL carry or be verifiably associated with schema versions and deterministic digests of their source inputs. Freshness checks SHALL compare those revisions and SHALL ignore wall-clock-only differences. A generated artifact from a different canonical or correction revision MUST fail validation even if its top-level card count is unchanged.

#### Scenario: Canonical value changes without count change

- **WHEN** a printed field is corrected but the canonical card count remains the same
- **THEN** the source/canonical digest changes
- **AND** stale generated card data, campaign output, and affected pack inputs fail freshness checks

#### Scenario: Timestamp alone does not cause drift

- **WHEN** generation reruns from identical committed inputs at a later wall-clock time
- **THEN** check mode reports no difference caused solely by execution time

#### Scenario: Cached module or pack differs despite unchanged text

- **WHEN** loaded producer code, pack or consumer identity differs from the sealed tuple
- **THEN** generation or dispatch fails before use
- **AND** unchanged card text does not promote historical verdicts

### Requirement: Gates reject incomplete publication transactions

The isolated marker SHALL be distinct from the global committed marker and written only after all artifact bytes are sealed. An isolated reader SHALL verify complete file membership and hashes around marker reads and reject incomplete, changed or mixed artifacts. This route SHALL NOT disable or replace the ordinary committed-revision protocol. Campaign activation additionally SHALL require STOP/drain, the common lock, exact predecessor identity and append-only idempotent transition preserving unresolved STARTED events, questions and cumulative usage.

Card-data check, engine pack construction, direct Python/Rust runtime card-data loaders, pull-request validation, and release validation SHALL use the committed-revision protocol before consuming fixed compatibility paths. They SHALL reject an incomplete transaction journal, verify declared file hashes, and reread the revision marker after loading so a concurrent change causes retry or failure. An incomplete transaction SHALL block consumption until recovery either restores the prior complete revision or finishes the staged revision. Fault-injection coverage SHALL exercise interruption after every publication step and races with readers.

#### Scenario: Journal indicates interrupted publication

- **WHEN** a transaction journal exists without a matching committed-revision marker
- **THEN** card-data check and pack/release validation fail with a recovery-required diagnostic
- **AND** they do not accept the mixed fixed-path artifacts

#### Scenario: Recovery restores a complete revision

- **WHEN** rollback recovery completes after an interrupted publication
- **THEN** every fixed compatibility path matches the restored committed-revision digest
- **AND** ordinary checks may proceed

#### Scenario: Runtime loader races with apply

- **WHEN** a direct runtime card-data loader reads while an apply transaction starts or commits
- **THEN** its before/after marker and hash validation either returns one complete revision or rejects/retries the read
- **AND** it never accepts a mixture of old and new files

#### Scenario: Isolated read races with incomplete sealing

- **WHEN** an isolated directory lacks its final marker or its marker/files change during reading
- **THEN** the reader refuses without ordinary-path fallback
- **AND** no execution generation or historical evidence is rewritten

### Requirement: Release builds verify embedded card-data consistency

Opted-in focused execution SHALL verify its exact pack bytes, release metadata and dispatch scope. It MUST NOT relax source-lane, canonical core, validation-debt or global release checks. The repair and focused branches SHALL receive reviewed isolated changes only; the later base checkout is a read-only human-merge compatibility reference whose stronger build contracts MUST be preserved by any future integration. Passing isolated checks SHALL NOT authorize a global release.

Before an engine or desktop release embeds card data, the release gate SHALL regenerate or check `cards.json`, official compatibility data, registration and validated candidate pools, campaign inputs, production DSL validation, and the deterministic card pack. A release MUST fail when embedded artifacts are stale, source-lane violations exist, duplicate IDs exist, or pack digests do not match their declared inputs.

#### Scenario: Desktop build would embed stale tested cards

- **WHEN** production registration changes but the committed compatibility pool is stale
- **THEN** the release card-data gate fails before desktop compilation embeds the stale artifact

#### Scenario: Pack is reproducible

- **WHEN** the production card pack is compiled twice from identical validated inputs
- **THEN** card ordering, manifest count, source digest, and serialized pack digest are identical

#### Scenario: Isolated pack proof cannot waive global release checks

- **WHEN** a scoped snapshot and pack pass their focused checks while global canonical or release validation is blocked
- **THEN** global publication and release remain blocked
- **AND** only explicitly reviewed in-scope dispatch may become eligible after its separate behavioral gates
