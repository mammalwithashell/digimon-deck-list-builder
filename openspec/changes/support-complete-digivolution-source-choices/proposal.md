## Why

BT18-042 requires a player to choose any eligible Digimon card from its digivolution sources, but the current selector exposes only twelve sources while game stacks are uncapped. A thirteen-source behavioral regression reproduces the missing legal choice, so the exact Light Hybrid deck cannot be certified through card authoring alone. BT18-063 and BT18-076 share that source gap. BT18-037 also loses an eligible security choice at index ten because its security selector is capped at ten cards, despite larger security stacks being legal.

## What Changes

- Add complete security-card paging with the same ownership, physical identity and generation guarantees; preserve only visibility permitted by the current effect.
- Add a complete, clone-safe source-choice protocol that can reach every eligible physical source without truncating stacks or auto-selecting cards.
- **BREAKING** Version action semantics and coordinate source choice metadata, observations, bindings, frontend dispatch, replay and model compatibility even if the action-vector size remains unchanged.
- Prefer owned paging within the existing action size, reusing infrastructure rather than reusing another choice kind with misleading semantics. Final IDs/page capacity must be specified and exported together before implementation.
- Make source-placement costs depend on a completed placement receipt, including interruption/cancellation, before enabling BT18-042's deletion.
- Keep BT18-042, BT18-063, BT18-076 and BT18-037 blocked until contract conformance, real card behavior and generated admission checks pass. This proposal changes no active constants, tensors or model metadata.

## Capabilities

### New Capabilities

- `complete-digivolution-source-choices`: Complete physical-source reachability, versioned paged interaction, cross-surface/model compatibility, and receipt-gated card promotion.

- `complete-security-card-choices`: Complete security-card reachability, permission-preserving paged interaction and coordinated compatibility.

### Modified Capabilities

None. The new capability coordinates existing action/observation interfaces; it does not claim an active migration has occurred.

## Impact

Rust selection/masks/decoders/resume state and constants; DSL material/source selectors and payment receipts; PyO3 exports; RL wrappers and layout metadata; desktop/web DTOs and frontend selectors; recorder/replay and DCGO action codegen if descriptors change; model manifests, ONNX export, checkpoint resume and inference compatibility; docs/ACTION_SPEC.md and docs/TENSOR_SPEC.md. Validation includes the source and security boundary witnesses, page boundaries, cloning, both seats, stale requests, interrupted placement costs and independent training-gate regeneration.
