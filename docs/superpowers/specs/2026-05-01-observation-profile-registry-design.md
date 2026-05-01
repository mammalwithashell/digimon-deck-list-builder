# Observation Profile Registry Design

**Goal:** Make RL observation tensors modular, versioned, and safe to compare by adding an engine-owned observation profile registry plus layout metadata plumbing through Rust, PyO3, Gym, feature extraction, and model artifacts.

## Decision: One Spec, Not Two

The profile registry and layout metadata plumbing should live in one design spec.

They are one contract:

- A profile registry without layout metadata cannot safely build feature extractors.
- Layout metadata without profile selection cannot support tensor ablation experiments.
- Model artifacts need both the selected profile ID and the exact layout hash to prevent loading a checkpoint against the wrong observation shape.

Implementation can still split into multiple phases. The design should stay unified so the invariants are visible in one place.

## Context

The current training stack assumes one global tensor:

- Rust exports `TENSOR_SIZE` from `code/digimon-engine/src/tensor.rs`.
- PyO3 exposes `TENSOR_SIZE` in `code/digimon-engine-py/src/lib.rs`.
- `DigimonEnv` imports `TENSOR_SIZE` and builds a fixed `spaces.Box`.
- `RustHeadlessGame.get_board_tensor(player_id)` has no profile argument.
- `CardEmbeddingExtractor` imports card/scalar positions from `engine_py_legacy.engine.data.tensor_layout`.

That makes tensor experimentation risky:

- A model can be trained with one tensor shape and accidentally evaluated with another.
- Python feature extraction can silently disagree with the Rust tensor writer.
- Profiling alternate tensors requires changing globals rather than selecting a profile.
- Ablation experiments are hard to reproduce from model artifacts alone.

The v2 tensor design in `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md` should be implemented on top of this profile system instead of replacing one hardcoded tensor with another.

## Design Principles

1. Observation profile selection is explicit and reproducible.
2. Rust owns tensor layout metadata for Rust-built tensors.
3. Python must not import tensor layout metadata from `engine_py_legacy` for Rust-backed training.
4. Model artifacts record the observation profile and layout hash.
5. Profile IDs are stable user-facing strings.
6. Layout hashes change whenever any index meaning, card/scalar position, tensor size, or feature interpretation changes.
7. Unknown profile IDs fail fast with a clear error.
8. Experiments can compare profiles without changing action-space constants or game rules.

## Profile Model

Add an engine-owned profile identifier:

```rust
pub enum ObservationProfileId {
    CompactV1,
    V2PendingAblation,
    V2Lite,
    V2Full,
    OmniscientDebug,
}
```

Expose stable string IDs:

| Profile | String ID | Purpose |
|---|---|---|
| `CompactV1` | `compact_v1` | Current 1375-float baseline |
| `V2PendingAblation` | `v2_pending_ablation` | Compact/fair board plus rich pending-choice metadata, used to isolate pending metadata value |
| `V2Lite` | `v2_lite` | First serious fair-information v2 profile with structured board, hand, known-zone, and pending-choice tables |
| `V2Full` | `v2_full` | Full v2 including `action_id_features[2168][16]` |
| `OmniscientDebug` | `omniscient_debug` | Test-only profile that may expose hidden identities |

`compact_v1` should remain the initial default until at least one v2 profile is implemented and verified.

`v2_pending_ablation` and `v2_lite` have different jobs:

- `v2_pending_ablation` answers "do rich pending-choice rows help?" while keeping the rest of the observation close to the compact baseline.
- `v2_lite` answers "does the practical v2 observation improve training?" and is the first profile intended for serious pilot runs.

Use `v2_pending_ablation` for controlled comparison runs against `compact_v1`, especially trigger-order and pending-selection scenarios. Do not treat it as the production stepping stone unless the ablation result justifies prioritizing pending metadata before the rest of the v2 board layout.

## Layout Metadata

Each profile exposes a complete layout description:

```rust
pub struct ObservationLayout {
    pub profile_id: &'static str,
    pub tensor_version: u16,
    pub feature_schema_version: &'static str,
    pub tensor_size: usize,
    pub card_id_positions: Vec<usize>,
    pub scalar_positions: Vec<usize>,
    pub layout_hash: String,
    pub schema: ObservationSchema,
}
```

`ObservationSchema` is a compact, serializable description for debugging and artifact manifests. It should not need to list every feature name in the first implementation, but it must include enough section metadata to audit shape:

```rust
pub struct ObservationSection {
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
    pub shape: Vec<usize>,
}

pub struct ObservationSchema {
    pub sections: Vec<ObservationSection>,
}
```

The layout metadata must satisfy:

- `feature_schema_version` is non-empty
- `card_id_positions.len() + scalar_positions.len() == tensor_size`
- the two position lists have no overlap
- the union covers every index from `0..tensor_size`
- every card ID position corresponds to an integer registry ID field
- every non-card scalar position is in `scalar_positions`

## Feature Schema Version

Each profile module owns a required feature-schema version constant:

```rust
pub const FEATURE_SCHEMA_VERSION: &str = "v2_lite.1";
```

The constant lives next to that profile's section table and card/scalar position builder, for example in `observation/v2_lite.rs`. It is copied into `ObservationLayout.feature_schema_version` and returned through PyO3.

The version is profile-specific, not global. Initial suggested values:

| Profile | Initial feature schema version |
|---|---|
| `compact_v1` | `compact_v1.1` |
| `v2_pending_ablation` | `v2_pending_ablation.1` |
| `v2_lite` | `v2_lite.1` |
| `v2_full` | `v2_full.1` |
| `omniscient_debug` | `omniscient_debug.1` |

Bump the profile's feature-schema version in the same commit when:

- the meaning, scale, bucket boundaries, or normalization of an existing feature changes
- a reserved index becomes an assigned feature
- a field moves between card ID and scalar interpretation
- a row order or prompt-row alignment rule changes
- hidden-information policy changes for that profile

Shape and position changes already affect `layout_hash` through section metadata and position lists, but the schema version should still bump for human-readable artifact inspection. Do not bump it for pure refactors that leave tensor values, layout metadata, and feature meanings unchanged.

Mechanical enforcement:

- `ObservationLayout` cannot be constructed without `feature_schema_version`.
- The layout hash builder serializes `feature_schema_version` into the canonical hash input.
- Layout tests assert the version is non-empty for every profile.
- Snapshot tests pin each implemented profile's `feature_schema_version` and `layout_hash`, so semantic edits require an intentional snapshot update.
- A unit test should clone a layout with only `feature_schema_version` changed and verify the hash changes.

## Layout Hash

`layout_hash` should be deterministic and profile-specific.

Hash inputs:

- profile ID
- tensor version
- feature-schema version string
- tensor size
- section names, offsets, sizes, and shapes
- card ID positions
- scalar positions
- action space size when action metadata is included

Use a stable hash such as SHA-256 over canonical JSON. The hash does not need to include runtime card data or deck contents.

If a feature's meaning changes while shape stays the same, bump the profile's `FEATURE_SCHEMA_VERSION` constant so the hash changes.

## Rust API

Add an observation module, likely under `code/digimon-engine/src/observation/`:

```text
observation/
  mod.rs
  profile.rs
  layout.rs
  compact_v1.rs
  v2_pending_ablation.rs
  v2_lite.rs
  v2_full.rs
```

Core API:

```rust
pub fn default_profile_id() -> ObservationProfileId;
pub fn parse_profile_id(raw: &str) -> Result<ObservationProfileId, ObservationProfileError>;
pub fn list_observation_profiles() -> Vec<&'static str>;
pub fn observation_layout(profile: ObservationProfileId) -> ObservationLayout;
pub fn build_observation_tensor(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    profile: ObservationProfileId,
) -> Vec<f32>;
```

Keep `tensor.rs` as a compatibility layer if needed:

- `TENSOR_SIZE` remains `compact_v1` while the default is `compact_v1`.
- `build_tensor(...)` delegates to `build_observation_tensor(..., CompactV1)`.
- New code should use the observation profile API.

Once a v2 profile becomes the default, `TENSOR_SIZE` can point to the selected default profile, but profile-specific code should avoid relying on a global size.

## Runner Contract

`HeadlessRunner` should store an observation profile:

```rust
pub struct HeadlessRunner {
    observation_profile: ObservationProfileId,
    ...
}
```

Constructor options:

- Existing constructors keep default profile behavior.
- Add a constructor variant or config struct that accepts `observation_profile`.

Tensor calls use the stored profile:

```rust
runner.get_board_tensor(player_id) -> Vec<f32>
runner.observation_layout() -> ObservationLayout
runner.observation_profile_id() -> &'static str
```

The method name `get_board_tensor` can remain for compatibility, but internally it should mean "observation tensor for this runner's profile."

## PyO3 Contract

Expose profile selection and layout metadata in `digimon_engine`.

Module-level functions:

```python
digimon_engine.list_observation_profiles() -> list[str]
digimon_engine.default_observation_profile() -> str
digimon_engine.get_observation_layout(profile_id: str | None = None) -> dict
```

`get_observation_layout` returns:

```python
{
    "profile_id": "v2_lite",
    "tensor_version": 2,
    "feature_schema_version": "v2_lite.1",
    "tensor_size": 8320,
    "card_id_positions": [...],
    "scalar_positions": [...],
    "layout_hash": "...",
    "sections": [
        {"name": "global_features", "offset": 0, "size": 64, "shape": [64]},
        ...
    ],
}
```

`RustHeadlessGame` constructor should accept a profile:

```python
RustHeadlessGame(
    deck1_ids,
    deck2_ids,
    verbose=False,
    record_actions=False,
    record_tensors=False,
    seed=None,
    observation_profile=None,
)
```

If `observation_profile is None`, use the engine default. If it is a string, validate with the Rust parser and fail fast if unknown.

Expose runner methods:

```python
runner.observation_profile_id -> str
runner.get_observation_layout() -> dict
```

Keep module-level `TENSOR_SIZE` temporarily as the default profile size for backward compatibility, but new Python training code should use `get_observation_layout(profile_id)["tensor_size"]`.

## Gym Contract

`DigimonEnv` should accept a tensor profile:

```python
env = DigimonEnv(tensor_profile="v2_lite")
```

Resolution order:

1. Explicit `tensor_profile` constructor argument.
2. `DIGIMON_TENSOR_PROFILE` environment variable.
3. `digimon_engine.default_observation_profile()`.

On initialization:

- fetch layout through `digimon_engine.get_observation_layout(profile_id)`
- set `self.tensor_profile`
- set `self.observation_layout`
- build `spaces.Box(shape=(layout["tensor_size"],), dtype=np.float32)`

On runner creation:

- pass `observation_profile=self.tensor_profile` to `RustHeadlessGame`
- for Python legacy backend, only allow `compact_v1` unless a Python-side profile exists
- fail fast if `DIGIMON_BACKEND != rust` and a non-`compact_v1` profile is requested

`reset()` and `step()` should include profile metadata in `info`:

```python
info = {
    "action_mask": self.action_mask(),
    "tensor_profile": self.tensor_profile,
    "tensor_feature_schema_version": self.observation_layout["feature_schema_version"],
    "tensor_layout_hash": self.observation_layout["layout_hash"],
}
```

## Feature Extractor Contract

`CardEmbeddingExtractor` should no longer import layout positions from `engine_py_legacy`.

Preferred shape:

```python
class CardEmbeddingExtractor(BaseFeaturesExtractor):
    def __init__(
        self,
        observation_space,
        features_dim=512,
        observation_layout=None,
        ...
    ):
        ...
```

SB3 policy kwargs should pass profile layout:

```python
policy_kwargs = {
    "features_extractor_class": CardEmbeddingExtractor,
    "features_extractor_kwargs": {
        "observation_layout": layout,
    },
}
```

The extractor reads:

- `layout["card_id_positions"]`
- `layout["scalar_positions"]`
- `layout["tensor_size"]`
- `layout["layout_hash"]`

It should assert:

- observation space size equals layout tensor size
- card/scalar positions cover the tensor
- no position is out of range

For loaded checkpoints, the model artifact manifest should provide the expected layout. If the current env layout hash differs, evaluation should fail before model inference.

## Model Artifact Metadata

Every trained model artifact should record:

```json
{
  "observation_profile": "v2_lite",
  "tensor_version": 2,
  "feature_schema_version": "v2_lite.1",
  "tensor_size": 8320,
  "tensor_layout_hash": "sha256:...",
  "action_space_size": 2168,
  "card_registry_capacity": 20000,
  "embedding_dim": 16
}
```

For SB3 `.zip` models, store this in a sidecar JSON next to the model and in any run manifest the training script already writes.

For ONNX export, include the same metadata in the ONNX sidecar manifest. If ONNX metadata fields are convenient, duplicate the profile ID and layout hash there too, but the sidecar remains the source of truth.

Model loading/evaluation should check:

- environment profile ID matches artifact profile ID
- feature schema version matches
- tensor size matches
- layout hash matches
- action space size matches
- registry capacity and embedding dim match

This prevents silent cross-profile evaluation bugs.

## Training CLI

Add a training flag:

```bash
python -m digimon_gym.agents.pilot_training --tensor-profile v2_lite
```

Also support:

```bash
DIGIMON_TENSOR_PROFILE=v2_lite python -m digimon_gym.agents.pilot_training
```

Training logs should print:

- profile ID
- feature schema version
- tensor size
- card slot count
- scalar slot count
- layout hash
- estimated observation buffer size for the configured rollout shape if available

This makes speed/sample-efficiency comparisons easier to interpret.

## Experiment Profiles

Initial profile set:

### `compact_v1`

Current `1375`-float tensor. Used as the baseline and fallback.

### `v2_pending_ablation`

Purpose: isolate the value of rich pending-choice metadata without paying for the full v2 board/action table.

Suggested shape:

- compact/fair board state
- decision context
- `pending_choice_features`
- no full `action_id_features`

This profile should keep the non-pending board representation intentionally close to `compact_v1`, except for fairness fixes needed by the Rust profile system. Its comparison target is:

```text
compact_v1 vs v2_pending_ablation
```

Use it when you want a narrow answer about pending-choice metadata, such as whether trigger-order rows improve simultaneous-effect sequencing. Do not use it to evaluate the full v2 board design, because it intentionally omits `permanent_slots[2][15][96]`, own-hand rows, and known-zone rows from `v2_lite`.

### `v2_lite`

Purpose: first serious v2 training profile.

Includes:

- fair-information player and board state
- unified `permanent_slots[2][15]`
- own hand rows
- known public zones
- decision context
- rich pending-choice rows

Excludes:

- full `action_id_features[2168][16]`

Its comparison targets are:

```text
compact_v1 vs v2_lite
v2_pending_ablation vs v2_lite
```

The first comparison measures practical end-to-end value. The second separates "pending metadata helped" from "the structured v2 board helped."

### `v2_full`

Purpose: test whether full action-id metadata improves wall-clock learning enough to justify the speed cost.

Includes `v2_lite` plus shallow `action_id_features[2168][16]`.

### `omniscient_debug`

Purpose: tests and controlled experiments only.

This profile may include hidden identities, but it must never be the default for pilot training.

## Comparison Metrics

Profile experiments should compare both sample efficiency and wall-clock throughput:

- environment steps per second
- games per hour
- rollout-buffer memory footprint
- policy forward/backward time per update
- win rate against greedy after fixed environment steps
- win rate against greedy after fixed wall-clock time
- targeted trigger-order scenario accuracy
- training stability with fixed seed families

A profile that learns in fewer games can still be worse if steps/sec drops enough. The default profile should optimize "useful play strength per training hour," not just "best win rate per environment step."

## Tests

Add tests for:

- profile parser accepts known IDs and rejects unknown IDs
- `list_observation_profiles()` includes every implemented profile
- every layout covers its tensor exactly once
- every layout has a non-empty `feature_schema_version`
- layout hash changes when a test-only schema string changes
- layout hash changes when only `feature_schema_version` changes
- layout snapshots pin `feature_schema_version` and `layout_hash`
- PyO3 `get_observation_layout()` matches Rust layout metadata
- `RustHeadlessGame(observation_profile=...)` returns tensors of the selected size
- `DigimonEnv(tensor_profile=...)` creates the matching observation space
- Python legacy backend rejects non-`compact_v1` profiles
- `CardEmbeddingExtractor` uses layout-provided positions
- model metadata validation rejects mismatched layout hashes

## Migration Plan Sketch

1. Add Rust profile/layout structs with `compact_v1`.
2. Make `tensor.rs` delegate to `compact_v1` through the profile registry.
3. Export profile list and layout metadata through PyO3.
4. Add `observation_profile` to `RustHeadlessGame`.
5. Add `tensor_profile` to `DigimonEnv`.
6. Update `CardEmbeddingExtractor` to consume layout metadata from policy kwargs.
7. Add model artifact metadata writing and loading checks.
8. Implement `v2_pending_ablation`.
9. Implement `v2_lite`.
10. Implement `v2_full` only after profiling justifies it.

## Acceptance Criteria

The design is ready to implement when reviewers agree that:

- profile selection and layout metadata are one contract
- `compact_v1` remains available as a baseline
- Rust owns layout metadata for Rust-built tensors
- Python feature extraction stops depending on legacy tensor layout for Rust profiles
- model artifacts record profile ID, tensor size, and layout hash
- non-default profiles can be selected through `DigimonEnv` and training CLI
- mismatched model/profile/layout combinations fail fast
