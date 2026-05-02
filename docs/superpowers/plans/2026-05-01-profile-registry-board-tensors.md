# Profile Registry For Board Tensors Implementation Plan

> Historical note only. Do not execute this plan.

This file is retained only as an archived record of an earlier, superseded implementation direction. Its original steps described a single-file `code/digimon-engine/src/tensor_profile.rs` registry, `tensor_profile::standard_v1_positions()`, and layout constants owned by `tensor.rs`. That approach is obsolete and must not be copied into new work.

The active plan and design are:

- `docs/superpowers/plans/2026-05-01-profile-owned-tensor-layout.md`
- `docs/superpowers/specs/2026-05-01-profile-owned-tensor-layout-design.md`

The canonical implementation now lives under `code/digimon-engine/src/tensor_profiles/<game_mode>/<version>.rs`. The current Standard v1 profile is `code/digimon-engine/src/tensor_profiles/standard/v1.rs`; `code/digimon-engine/src/tensor.rs` is the Standard v1 writer and compatibility surface.

New Rust code should import profile metadata from `digimon_engine::tensor_profiles`. `digimon_engine::tensor_profile` exists only as a temporary compatibility alias for older callers.
