# Frozen mixed versus development evaluation

Launched October 25, 2026 at 22:26 Central on the existing Hetzner host. This
is evaluation only; the completed seed comparison and original checkpoints are
unchanged. No search, fitting, automatic restart or promotion is enabled.

- Root: `/opt/digimon-search/head-to-head-20261025`.
- Source: `/opt/digimon-search/mixed-seed-comparison-20261025`.
- Service: `digimon-head-to-head.service`, `Restart=no`, three-hour limit.
- Native image: `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:76d7cf1e6fd0c25af41bbb4b0199c5da812eb0bac952d0db3dcbf57072ab8444`.
- Read-only Python overlay: commit `7d2d9c5b4`, runtime directory
  `/opt/digimon-search/head-to-head-runtime-7d2d9c5b4`.
- Controller SHA-256: `3bbe62057f5b49dd4d8c6a35d1758571d6b6a865346fec024f887a53ad687a08`.
- Mixed model SHA-256: `66a3c4eacc9a509777b6f4e91dda224826f59a342ab20000cb795051f778a0f8`.
- Development model SHA-256: `f2c78c4eda90f87c694668edb76ffddddb12fd508e35c1dd5b4490424153fee3`.
- Combined snapshot SHA-256: `3b3ac04a3bba707ab0e3a4f0142307a63c76d11c62c157ffa5afe2c93ae53977`.
- Schedule SHA-256: `8495ba88a786e018327f337ef66e568f583b55c83add9d819b4be678793bb1a4`.
- Seed: 91261025; 6,760 games, 40 per ordered deck cell across all 13 archetypes.

The two evaluation processes run concurrently in a network-disabled container
limited to two CPUs and 12 GiB. Each model is the player-1 candidate for 3,380
games. Every ordered deck cell has ten pairs with opposite opening players per
direction. Both sides use deterministic Torch inference with legal masks.
The source's held-out meta and same-list pet cohorts are retained separately in
the plan, while the combined schedule also covers meta/pet cross-pool games.

Nineteen focused tests passed. Native preflight verified both model contracts,
the frozen card database, image identity, and admission of all 13 lists. Four
Virus Busters versus Examon smoke games (both model roles and opening orders)
finished normally with telemetry enabled. An initial preparation-only attempt
found a missing nested evidence directory; the fix was committed and preflight
passed before any evaluation service was started. Its log is preserved.

Separate W&B sidecars publish to `digimon-rl`:
- `head-to-head-mixed-20261025`
- `head-to-head-development-20261025`

Local dashboard: `http://127.0.0.1:8502`, cache/config under
`runs/head-to-head-dashboard`. The completed original comparison remains at
port 8501. For this active evaluation, inspect `comparison_state.json`, both
`evaluations/<arm>/window-000001/combined_head_to_head-progress.json` files and
then `head_to_head_result.json`. The final result includes the full matchup
matrix and a four-game-block bootstrap interval; no result was available at
launch. Per-card reports retain both player roles and are uploaded separately.
