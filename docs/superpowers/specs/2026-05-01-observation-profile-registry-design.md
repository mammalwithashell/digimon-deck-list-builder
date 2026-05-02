# Observation Profile Registry Design

**Goal:** Make RL observation tensors modular, versioned, and safe to compare by adding an engine-owned observation profile registry plus layout metadata plumbing through Rust, PyO3, Gym, feature extraction, and model artifacts.

## Decision: One Spec, Not Two

The profile registry and layout metadata plumbing live in one design spec.

They are one contract:

- A profile registry without layout metadata cannot safely build feature extractors.
- Layout metadata without profile selection cannot support tensor ablation experiments.
- Model artifacts need both the selected profile ID and the exact layout hash to prevent loading a checkpoint against the wrong observation shape.

Implementation can still split into multiple phases. The design stays unified so the invariants are visible in one place.

## Context

Before the profile registry work, the training stack assumed one global tensor:

- Rust exports `TENSOR_SIZE` from `code/digimon-engine/src/tensor.rs`.
- PyO3 exposes `TENSOR_SIZE` in `code/digimon-engine-py/src/lib.rs`.
- `DigimonEnv` imported `TENSOR_SIZE` and built a fixed `spaces.Box`.
- `RustHeadlessGame.get_board_tensor(player_id)` had no profile argument.
- `CardEmbeddingExtractor` imported card/scalar positions from `engine_py_legacy.engine.data.tensor_layout`.

That makes tensor experimentation risky:

- A model can be trained with one tensor shape and accidentally evaluated with another.
- Python feature extraction can silently disagree with the Rust tensor writer.
- Profiling alternate tensors requires changing globals rather than selecting a profile.
- Ablation experiments are hard to reproduce from model artifacts alone.

The v2 tensor design in `docs/superpowers/specs/2026-05-01-rl-observation-action-tensor-v2-design.md` was implemented on top of this profile system instead of replacing one hardcoded tensor with another.

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

The design sketched an engine-owned profile identifier family:

```rust
pub enum ObservationProfileId {
    StandardCompactV1,
    StandardLiteV2,
    StandardFullV2,
}
```

Expose stable string IDs for implemented/exported profiles:

| Profile | String ID | Purpose |
|---|---|---|
| `StandardCompactV1` | `standard_compact_v1` | Implemented 1375-float Standard compact baseline |
| `StandardLiteV2` | `standard_lite_v2` | Implemented fair-information Standard v2 profile with structured board, hand, known-zone, and pending-choice tables |
| `StandardFullV2` | `standard_full_v2` | Implemented opt-in Standard v2 experiment extending lite with `action_id_features[2168][16]` |

`standard_lite_v2` is implemented and is the default pilot observation profile. `standard_full_v2` is implemented as an opt-in experimental profile. `standard_compact_v1` remains the compact compatibility and baseline profile. `standard_v1` and `compact_v1` are legacy aliases only; new metadata writes `standard_compact_v1` for compact runs, `standard_lite_v2` for default v2 pilot runs, and `standard_full_v2` only for explicit full-profile experiments.

Additional profile ideas are intentionally opt-in or planned rather than defaults. `standard_full_v2` is implemented and exported as an opt-in action-id feature table experiment. Other examples remain planned, including `standard_pending_ablation_v2` for isolating pending-choice metadata value and `standard_omniscient_debug_v1` for test-only hidden-information debugging.

`standard_pending_ablation_v2` and `standard_lite_v2` would have different jobs if the ablation profile is added later:

- `standard_pending_ablation_v2` answers "do rich pending-choice rows help?" while keeping the rest of the observation close to the compact baseline.
- `standard_lite_v2` answers "does the practical v2 observation improve training?" and is the first profile intended for serious pilot runs.

If `standard_pending_ablation_v2` is implemented and exported later, use it for controlled comparison runs against `standard_compact_v1`, especially trigger-order and pending-selection scenarios. Do not pass this profile ID to today's CLI or registry. Keep `standard_lite_v2` as the implemented default for v2 pilot runs.

## Layout Metadata

Each profile exposes a complete layout description:

```rust
pub struct TensorProfile {
    pub id: &'static str,
    pub game_mode: &'static str,
    pub version: u32,
    pub tensor_version: u16,
    pub feature_schema_version: &'static str,
    pub layout_hash: &'static str,
    pub tensor_size: usize,
    pub field_slots: usize,
    pub slot_size: usize,
    pub max_sources: usize,
    pub slot_layout: TensorSlotLayout,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub sections: &'static [TensorSection],
}
```

`TensorSection` is a compact description for debugging and artifact manifests. It does not need to list every feature name, but it includes enough section metadata to audit shape:

```rust
pub struct TensorSection {
    pub id: &'static str,
    pub start: usize,
    pub len: usize,
    pub shape: &'static [usize],
    pub kind: TensorSectionKind,
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
pub const FEATURE_SCHEMA_VERSION: &str = "standard_lite_v2.1";
```

The constant lives next to that profile's section table and card/scalar position builder, for example in `tensor_profiles/standard/v2_lite.rs`. It is copied into layout metadata and returned through PyO3.

The version is profile-specific, not global. Implemented values:

| Profile | Initial feature schema version |
|---|---|
| `standard_compact_v1` | `standard_compact_v1.1` |
| `standard_lite_v2` | `standard_lite_v2.1` |
| `standard_full_v2` | `standard_full_v2.1` |

Future profiles define their own feature-schema versions when implemented; do not publish artifact metadata for a profile before it exists in the registry.

Bump the profile's feature-schema version in the same commit when:

- the meaning, scale, bucket boundaries, or normalization of an existing feature changes
- a reserved index becomes an assigned feature
- a field moves between card ID and scalar interpretation
- a row order or prompt-row alignment rule changes
- hidden-information policy changes for that profile

Shape and position changes already affect `layout_hash` through section metadata and position lists, but the schema version still bumps for human-readable artifact inspection. Do not bump it for pure refactors that leave tensor values, layout metadata, and feature meanings unchanged.

Mechanical enforcement:

- `TensorProfile` cannot be constructed without `feature_schema_version`.
- The layout hash builder serializes `feature_schema_version` into the canonical hash input.
- Layout tests assert the version is non-empty for every profile.
- Snapshot tests pin each implemented profile's `feature_schema_version` and `layout_hash`, so semantic edits require an intentional snapshot update.
- A unit test clones a layout with only `feature_schema_version` changed and verifies the hash changes.

## Layout Hash

`layout_hash` is deterministic and profile-specific.

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

Observation profile dispatch lives in `code/digimon-engine/src/observation.rs`. Profile layout metadata lives under `code/digimon-engine/src/tensor_profiles/`. Current modules include the implemented compact compatibility, lite v2, and full v2 profiles. Other planned experiment profiles are documented here but are not exported until implemented:

```text
src/
  observation.rs
  tensor_profiles/
    mod.rs
    standard/
      mod.rs
      v1.rs
      v2_lite.rs
      v2_full.rs
```

Core API:

```rust
pub fn default_observation_profile() -> ObservationProfileId;
pub fn parse_observation_profile(raw: &str) -> Result<ObservationProfileId, String>;
pub fn list_observation_profiles() -> Vec<&'static str>;
pub fn observation_layout(profile: ObservationProfileId) -> TensorProfile;
pub fn build_observation_tensor(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    profile: ObservationProfileId,
) -> Vec<f32>;
```

Keep `tensor.rs` as a compatibility layer:

- `TENSOR_SIZE` remains the `standard_compact_v1` compatibility size.
- `build_tensor(...)` delegates to `build_observation_tensor(..., StandardCompactV1)`.
- New code uses the observation profile API.

Now that `standard_lite_v2` is the default pilot observation, profile-specific code avoids relying on a global size and reads the selected layout's `tensor_size`.

## Runner Contract

`HeadlessRunner` stores an observation profile:

```rust
pub struct HeadlessRunner {
    observation_profile: ObservationProfileId,
    ...
}
```

Constructor options:

- Existing constructors keep default profile behavior.
- The Rust/PyO3 runner constructor accepts `observation_profile`.

Tensor calls use the stored profile:

```rust
runner.get_board_tensor(player_id) -> Vec<f32>
runner.observation_layout() -> TensorProfile
runner.observation_profile_id() -> &'static str
```

The method name `get_board_tensor` can remain for compatibility, but internally it means "observation tensor for this runner's profile."

## PyO3 Contract

Expose profile selection and layout metadata in `digimon_engine`.

Module-level functions:

```python
digimon_engine.list_observation_profiles() -> list[str]
digimon_engine.get_observation_layout(profile_id: str | None = None) -> dict
```

Module-level constants:

```python
digimon_engine.DEFAULT_OBSERVATION_PROFILE: str
```

`get_observation_layout` returns:

```python
{
    "profile_id": "standard_lite_v2",
    "tensor_version": 2,
    "feature_schema_version": "standard_lite_v2.1",
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

`RustHeadlessGame` constructor accepts a profile:

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

Keep module-level `TENSOR_SIZE` as the compact compatibility size for backward compatibility, but Python training code uses `get_observation_layout(profile_id)["tensor_size"]`.

## Gym Contract

`DigimonEnv` accepts a tensor profile:

```python
env = DigimonEnv(tensor_profile="standard_lite_v2")
```

Resolution order:

1. Explicit `tensor_profile` constructor argument.
2. `DIGIMON_TENSOR_PROFILE` environment variable.
3. `digimon_engine.DEFAULT_OBSERVATION_PROFILE`.

On initialization:

- fetch layout through `digimon_engine.get_observation_layout(profile_id)`
- set `self.tensor_profile`
- set `self.observation_layout`
- build `spaces.Box(shape=(layout["tensor_size"],), dtype=np.float32)`

On runner creation:

- pass `observation_profile=self.tensor_profile` to `RustHeadlessGame`
- normalize legacy aliases (`standard_v1`, `compact_v1`) to `standard_compact_v1` before backend-specific checks
- for Python legacy backend, only allow the normalized `standard_compact_v1` unless a Python-side profile exists
- fail fast if `DIGIMON_BACKEND != rust` and a normalized non-`standard_compact_v1` profile is requested

`reset()` and `step()` include profile metadata in `info`:

```python
info = {
    "action_mask": self.action_mask(),
    "tensor_profile": self.tensor_profile,
    "tensor_feature_schema_version": self.observation_layout["feature_schema_version"],
    "tensor_layout_hash": self.observation_layout["layout_hash"],
}
```

## Feature Extractor Contract

`CardEmbeddingExtractor` no longer imports layout positions from `engine_py_legacy`.

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

SB3 policy kwargs pass profile layout:

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

It asserts:

- observation space size equals layout tensor size
- card/scalar positions cover the tensor
- no position is out of range

For loaded checkpoints, current shipped validation records the selected profile metadata and checks tensor/profile shapes where those contracts are constructed or exported. Loader/evaluation rejection on layout-hash mismatch is future work for model-loading paths; until that exists, manifests are audit metadata rather than a universal pre-inference gate.

## Model Artifact Metadata

Every trained model artifact records:

```json
{
  "observation_profile": "standard_lite_v2",
  "tensor_version": 2,
  "feature_schema_version": "standard_lite_v2.1",
  "tensor_size": 8320,
  "tensor_layout_hash": "sha256:...",
  "action_space_size": 2168,
  "card_registry_capacity": 20000,
  "embedding_dim": 16
}
```

For SB3 `.zip` models, store this in a sidecar JSON next to the model and in any run manifest the training script already writes.

For ONNX export, include the same metadata in the ONNX sidecar manifest. If ONNX metadata fields are convenient, duplicate the profile ID and layout hash there too, but the sidecar remains the source of truth.

Current shipped model artifact checks:

- ONNX export metadata records profile ID, tensor size, layout hash, action space size, registry capacity, and embedding dim
- ONNX export validates the loaded SB3 checkpoint observation shape against the requested profile tensor size before tracing
- feature extractors and env construction validate tensor/profile shape contracts

Future loader/evaluation work should reject mismatched artifact metadata before inference, including profile ID, feature schema version, tensor size, layout hash, action space size, registry capacity, and embedding dim.

## Training CLI

The training CLI supports a tensor-profile flag:

```bash
python -m digimon_gym.agents.pilot_training --tensor-profile standard_lite_v2
```

Also support:

```bash
DIGIMON_TENSOR_PROFILE=standard_lite_v2 python -m digimon_gym.agents.pilot_training
```

Training logs print:

- profile ID
- feature schema version
- tensor size
- card slot count
- scalar slot count
- layout hash
- estimated observation buffer size for the configured rollout shape if available

This makes speed/sample-efficiency comparisons easier to interpret.

## Experiment Profiles

Implemented and planned profile set:

### `standard_compact_v1`

Compact `1375`-float tensor. Used as the baseline, fallback, and compatibility profile.

### `standard_pending_ablation_v2`

Purpose: isolate the value of rich pending-choice metadata without paying for the full v2 board/action table.

Suggested shape:

- compact/fair board state
- decision context
- `pending_choice_features`
- no full `action_id_features`

This planned profile keeps the non-pending board representation intentionally close to `standard_compact_v1`, except for fairness fixes needed by the Rust profile system. If it is implemented and exported later, its comparison target is:

```text
standard_compact_v1 vs standard_pending_ablation_v2
```

Use it only after future implementation when you want a narrow answer about pending-choice metadata, such as whether trigger-order rows improve simultaneous-effect sequencing. Do not use it to evaluate the full v2 board design, because it intentionally omits `permanent_slots[2][15][96]`, own-hand rows, and known-zone rows from `standard_lite_v2`.

### `standard_lite_v2`

Purpose: implemented default v2 pilot training profile.

Includes:

- fair-information player and board state
- unified `permanent_slots[2][15]`
- own hand rows
- known public zones
- decision context
- rich pending-choice rows

Excludes:

- full `action_id_features[2168][16]`

Its current compact comparison target is:

```text
standard_compact_v1 vs standard_lite_v2
```

This comparison measures practical end-to-end value. If `standard_pending_ablation_v2` is implemented and exported later, comparing `standard_pending_ablation_v2` against `standard_lite_v2` would separate "pending metadata helped" from "the structured v2 board helped."

### `standard_full_v2`

Purpose: implemented opt-in experiment to test whether full action-id metadata improves wall-clock learning enough to justify the speed cost.

Includes `standard_lite_v2` plus shallow `action_id_features[2168][16]`.

### `standard_omniscient_debug_v1`

Purpose: tests and controlled experiments only.

This profile may include hidden identities, but it must never be the default for pilot training.

## Comparison Metrics

Profile experiments compare both sample efficiency and wall-clock throughput:

- environment steps per second
- games per hour
- rollout-buffer memory footprint
- policy forward/backward time per update
- win rate against greedy after fixed environment steps
- win rate against greedy after fixed wall-clock time
- targeted trigger-order scenario accuracy
- training stability with fixed seed families

A profile that learns in fewer games can still be worse if steps/sec drops enough. The default profile optimizes "useful play strength per training hour," not just "best win rate per environment step."

## Tests

Tests cover:

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
- Python legacy backend rejects non-`standard_compact_v1` profiles
- `CardEmbeddingExtractor` uses layout-provided positions
- ONNX export rejects SB3 checkpoints whose observation shape does not match the requested tensor profile size
- layout-hash mismatch rejection remains future loader/evaluation work

## Migration Plan Sketch

1. Rust profile/layout structs with `standard_compact_v1`.
2. Make `tensor.rs` delegate to `standard_compact_v1` through the profile registry.
3. Export profile list and layout metadata through PyO3.
4. `observation_profile` on `RustHeadlessGame`.
5. `tensor_profile` on `DigimonEnv`.
6. Update `CardEmbeddingExtractor` to consume layout metadata from policy kwargs.
7. Model artifact metadata writing and loading checks.
8. Implement `standard_pending_ablation_v2` if the ablation is still useful.
9. Implement `standard_lite_v2`. Completed; it is now the default pilot profile.
10. Implement `standard_full_v2`. Completed as an opt-in experiment; keep `standard_lite_v2` as the default until profiling justifies changing it.

## Acceptance Criteria

The implemented profile registry contract is accepted when:

- profile selection and layout metadata are one contract
- `standard_compact_v1` remains available as a baseline
- Rust owns layout metadata for Rust-built tensors
- Python feature extraction stops depending on legacy tensor layout for Rust profiles
- model artifacts record profile ID, tensor size, and layout hash
- default and non-default profiles can be selected through `DigimonEnv` and training CLI
- ONNX export fails fast when a checkpoint observation shape does not match the requested tensor profile
- layout-hash mismatch rejection is implemented in future model-loading/evaluation paths
