## 1. Freeze the coordinated contract

- [x] 1.1 Inventory every capped battle/breeding source selector and security selector and freeze paging IDs, capacity, physical identity and generation metadata against exported layouts. See contract-inventory.md; this freezes the migration descriptor, not the active schema.
- [x] 1.2 Update ACTION_SPEC and TENSOR_SPEC with navigation semantics, action schema version, observation schema/hash changes and migration requirements. Rust schema two / 128-wide rows and computed hashes are verified; consumers remain under task 4.

## 2. Prove complete legal choices

- [x] 2.1 Add failing reachability tests at indices 11, 12 and deeper for material, own-source and count-capped selectors, including breeding; add security index-9/index-10/later boundaries.
- [x] 2.2 Add clone, compaction, source movement, stale generation, cancellation and impossible-prefix mask regressions.
- [x] 2.3 Implement a shared owned source chooser with navigation and pinned physical-card resolution; remove silent truncation paths.

- [x] 2.4 Implement typed owned security paging with permission-preserving metadata, physical identity, stale-generation rejection and no navigation RNG/movement.

## 3. Preserve actual cost payment

- [x] 3.1 Extend selected-source security placement with correlated completed movement receipts, pending replacement and abort/reset cleanup. Ten selected-source and twelve top-card controls pass; the existing typed finalizer/reset cleanup is reused.
- [x] 3.2 Verify BT18-042 captures the selected level before movement and deletes only after actual payment. Nineteen canonical card cases pass in both final full-card invocations, including deferred/canceled/redirected costs and simultaneous same-level deletion.

## 4. Synchronize consumers and migration

- [x] 4.1 Update Rust descriptors/constants, PyO3 exports, RL wrappers, frontend/DTO selection rendering and submissions together.
- [x] 4.2 Update recorder/replay, inference/export/model manifests and compatibility gates, rejecting old semantic versions even when shapes match. Thirteen current receipt/Link projection tests and all ten active replay goldens (635 actions) pass. Three affected recordings were regenerated with supported tooling after explicit action-schema and P-249 token-index diagnostics; originals and SHA records remain under qa/replay-goldens/retired/2026-10-25-owned-choice-p249. Compatible fresh exports pass the 123-check native qualification.
- [x] 4.3 Rebuild bindings and test navigation metadata, masks and recurrent inference through actual environment boundaries. Final receipt/Link/top-choice wheel passes 123 normal-stack native checks, including fresh recurrent actor/critic exports, legal masks and reset behavior; see final-link-pin-release-wheel-normal-stack-check.log.

## 5. Requalify the affected cards

- [x] 5.1 Author BT18-042 fully after its source-choice/payment primitives land; replace the ignored geometry witnesses with public action reachability tests for BT18-042, BT18-063, BT18-076 and BT18-037 and complete their behavioral reviews.
- [x] 5.2 Run contract, engine, RL and frontend checks; regenerate impact/lowering/admission artifacts and verify the committed wheel agrees. Final required 93-suite run and all 13 post-promotion Tier 0 checks pass; final-wheel public gate accepts all 20 IDs / 54 copies.
- [x] 5.3 Retrain or validate compatible model exports and record promotion evidence before clearing training blockers. Fresh compatible recurrent actor/critic ONNX exports pass masking/reset and metadata validation in the final 123-check native run. Four qualified ledger rows were promoted; no old model weights were relabeled and no trained-policy quality promotion is claimed.

## Current qualification checkpoint (2026-10-25)

Task 2 and the core consumer migration have native qualification evidence.
The latest replay projections and final binding rebuild are qualified under 4.2/4.3.
Nine active
capacity witnesses cover physical source positions 11/12/39/300 and security
positions 9/10/39, both actors and clones. Prefix/recovery controls cover live
movement, compaction, stale generations and impossible payment. All named
source-selector families use owned physical choices. This does not certify
independent clone completion for unrelated legacy raw FnOnce callbacks.

The final optimized PyO3 wheel passes 123 tests on the normal Windows Python
main-thread stack, including actual environment boundaries and real fresh
untrained recurrent SB3 checkpoints and actor/critic ONNX exports. Recurrent
reset and legal masks pass. Frontend, checked transport, privacy and desktop
library compilation are recorded in the consumer migration checkpoint.
Replay versioning and V4 fingerprints preserve archival protocol behavior.

Task 5.1 has 206 canonical card cases, nine active capacity witnesses and an
independent printed-text/rules review. Task 5.2 is complete: all 93 required
suites pass. Both complete card invocations pass 12729 tests with zero failures
and 35 existing ignores. DSL passes 1596 with one existing ignore; final engine
unit qualification passes 507 with zero failures. All 13 post-promotion Tier 0
checks pass, including impact/lowering/admission freshness.

Task 5.3 is complete through compatible export validation. The four former
blockers are promoted after qualification, and public mechanics, registration/
production-data and card-admissibility checks accept 20/20 IDs and 54/54 copies
of the byte-identical local DCGO list. See verification/final-exact-deck-admission.json
under the Light Hybrid QA folder. This is simulator training admission; no
production policy training, model-quality promotion or new live DCGO oracle
certification is claimed.

The following checkpoints are chronological historical records and may describe
older intermediate source snapshots; this current checkpoint takes precedence.

## Historical implementation checkpoint

P-249 publication, nine card tests and independent review are complete. The
current exact deck gate is 16/20 IDs, 46/54 copies. Four ledger blockers remain:
BT18-037, BT18-042, BT18-063 and BT18-076. Do not clear them from primitive
coverage alone. Current CLI pool includes 1181 implemented IDs; a fresh binding
wheel and actual environment agreement are still pending task 4.3/5.2.

The pure page kernel (five Cargo tests) and SourceChoicePin (six Cargo tests)
are installed for basic material and security selectors in this worktree.
Nine public owned-selector tests pass, including raw and DSL own/opponent
multi-source, count-capped battle/breeding sources, deep choices, navigation,
required-prefix clones, checked stale tokens and optional stale security.
Four card-page tensor tests, two face-down characteristic tests and two
controller-relative security eligibility tests also pass. The active Rust
action schema is two, pending rows 128, and lite/deck/full sizes 9434/9874/44506.
Other source selector families, mandatory invalidation, prefix compaction,
external checked submissions, replay and fresh-wheel verification remain open.
The 28-row allocation and inspection-row exception are recorded in
contract-inventory.md. Do not expose
private CardHandle values as policy features or wire card IDs.

Consumer checkpoint: 16 Python privacy/perspective tests and 11 frontend
selection tests pass, as does TypeScript checking. Typed page rows render
deep source/security choices and navigation; nonselecting seats and hidden
spectators cannot receive inspected security identities. PyO3 and desktop
DTO metadata threading is authored but not compiled yet. See
qa/archetype-qa/dsl/light-hybrid-training/consumer-migration-checkpoint.json.
The ACTION/TENSOR documents now reserve the complete coordinated allocation;
task 1.2 is complete for the active Rust descriptor; native consumer verification remains open.

Selected-source receipts are fully backed by typed SourceCard security-move
continuations, rather than the legacy raw placement path. Runtime RED exposed
unpaid synchronous placement and deferred Card-subject panics; the current ten
source, twelve top and one card-property tests pass. Existing selected-active-top
semantics intentionally place the whole permanent through its owned path.
The compiled pack format is 0.32.0; its four compatibility unit tests pass.

BT18-042's official observation/image say AS THE BOTTOM SECURITY CARD, without
"your". The stale API text was corrected in central data and its durable override;
use to_owner:true under CR3-1-3-8. It deletes opponent Digimon with the SAME level
as the paid card. bind_permanent_property now accepts a named physical card
and captures its printed level before movement, so level_eq_binding can compare
against that saved literal. Full card authoring/qualification remains pending.
Its second clause also requires review of the explicit top-security-to-hand
cost and optional/once-per-turn behavior; do not infer readiness from clause one.

The new canonical construct tests are under tests/dsl/constructs, with dedicated
root test wrappers. A Rust string replacement fragment was changed to an
identical concat! expression to avoid falsely crediting the of_permanent
predicate; global admission correctly stays 1062/4477, without an unrelated
promotion. Full tier-2 impact planning requests the broader suite; rerun it
once the coordinated source/security consumer migration is complete.

Fresh combined checkpoint: all 204 canonical deck-card cases and 22 shared
contract cases pass in verification/logs/combined-card-contract-rerun-fixed.log.
The first combined attempt exposed a stale 96-wide test stride; the corrected
test now uses PENDING_CHOICE_ROW_SIZE. No admission blockers were cleared.

The first rebuilt PyO3 diagnostic wheel now compiles and passes 96 native
Python checks (29 profile, 2 owned-page/token, 56 binding surface and 9 actual
DigimonEnv adapter cases). The unoptimized wheel needs a 32MB Python worker
stack on Windows; normal release/runtime verification remains required. This
wheel predates subsequent source-prefix/replay changes, so final rebuild and
qualification remain open. Global Python installation was left unchanged.
