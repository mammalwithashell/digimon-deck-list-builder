## Context

A real thirteen-source witness exposes only twelve actions through select_material. The same helper backs BT18-063/BT18-076's inherited Tamer play; a Tamer at source index twelve is also affected. Source-count predicates iterate the complete stack and are not affected. Game source vectors have no gameplay cap. ACTION_SPEC requires coordinated changes for more source choices, and the model contract already carries action_schema_version independently of shape.

## Goals / Non-Goals

**Goals:** Every eligible physical source is reachable; choices survive cloning and effect interruption; masks remain authoritative; async clients cannot apply a stale page click to another card; incompatible models fail closed; BT18-042 pays its selected-source cost before deleting anything.

**Non-Goals:** No card-local picker, hidden selection, source-depth restriction, active contract bump in this proposal, or P-249 data-pipeline cutover. No inference certification from static construct coverage.

## Decisions

1. **Use owned paging, not a larger fixed stack cap.** A finite slot increase still truncates an uncapped vector. A cloneable source-choice frame owns the complete filtered physical-card candidates, selected prefix, carrier identities, page and prompt generation. Reuse existing paging infrastructure where its semantics fit, but give source rows a typed source operation rather than pretending they are counts/effect labels.
2. **Keep the action vector size if the coordinated descriptor proves the layout.** Proposed commit window is one exported source-window stride; existing navigation action IDs can be reused only in the new owned source-selection phase. The final descriptor specifies commit/navigation/decline IDs and page capacity before code changes. Every consumer uses that descriptor, not hand-built offsets. Source schema semantics change even when the vector shape stays 2192.
3. **Version all affected interfaces together.** Bump Rust action SCHEMA_VERSION and export it through PyO3, RL, frontend and model metadata. Declare typed source rows with owner, carrier identity, physical card, current source index, page generation and operation. Update active observation schema/layout hash for the added page semantics; update profile IDs/sizes if required by the chosen layout. Async API/desktop submissions validate expected prompt/page generation; internal serial RL steps act on the current owned page.
4. **Use physical identities for resolution.** Navigation changes only owned prompt state. Commit revalidates the pinned card and carrier and cannot bind a different card that moved into an old index. Material, own-source and count-capped material selectors share this mechanism across battle and breeding. Capture the actual selected level for BT18-042 before its move.
5. **Keep placement payment separate from selection.** Extend the selected-source security step with a completed movement receipt, reusing the top-card receipt machinery where suitable. Pending acceptance, cancellation, custom handling and non-security redirection cannot be inferred from attempted moves or final location alone. Only a paid receipt permits BT18-042's grouped deletion.
6. **Fail closed for old models and recordings.** Existing version-one weights are not relabelled or silently re-exported as version two. The complete training tuple is required for resume/export/inference. Legacy recording replay needs an explicit semantic adapter using recorded physical source metadata; raw aliased source action IDs are insufficient.

## Risks / Trade-offs

- More decisions for deep stacks → expose stable page/card metadata and include navigation in evaluation telemetry.
- Same-size action semantics drift → action_schema_version plus observation hash gates, including same-size mismatch tests.
- A card moves while a prompt is parked → physical identity and generation checks; invalid requests do not spend costs or OPT.
- Receipt lifetime leaks across clone/reset → keyed completion, abort/reset cleanup and replay equivalence tests.
- Only one selector is migrated → inventory all source-cap paths and prove all use the shared owned chooser.

## Migration Plan

Implement the new contract in one dedicated change with docs, constants/descriptors, bindings, RL, UI, recorder/replay and model metadata synchronized. Generate DCGO C# descriptors from the base repository if exports change. Run contract and source reachability suites, rebuild bindings, retrain or genuinely compatible re-export models, and then implement/re-audit affected cards. Keep card-ledger blockers until the new wheel and committed gate artifacts agree. Roll back the contract and compatible model set together; do not load new models against an old decoder.

## Open Questions

The exact navigation IDs, page capacity and observation layout must be frozen by task 1 against current exported metadata. Paging is the selected architecture; that final address/layout table is a prerequisite gate, not permission to modify active constants in the card-authoring change.

## Security-choice companion scope

The same dedicated action/tensor migration also repairs BT18-037's security
selector: indices 0..9 are addressable today, but index 10 is a real missing
eligible choice in a larger legal security stack. Use the owned paging
framework with a typed security zone/owner and physical-card identity instead
of pretending that security is a material pile. Reveal card metadata only to
seats and effects permitted to inspect that security. Navigation cannot
shuffle, recover, move cards or consume RNG; a stale generation after movement
or shuffling cannot bind the replacement at an old security index. Apply the
same atomic schema/hash, PyO3/RL/frontend/replay/model migration and fail-closed
compatibility requirements. Lobomon stays blocked until its public boundary
regression and the transport/environment checks pass.
