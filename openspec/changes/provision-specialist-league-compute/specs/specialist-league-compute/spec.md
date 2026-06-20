## ADDED Requirements

### Requirement: Durable round and registry state

League round artifacts — the specialist registry, snapshots, and per-round results — SHALL be persisted to durable storage that survives box restarts and teardown. The league MUST NOT keep round-barrier state only on ephemeral container storage (`/app`).

#### Scenario: Round survives a box restart
- **WHEN** a box is restarted or destroyed after a round barrier
- **THEN** the registry and that round's snapshots remain available from durable storage, and the next round can resume without re-running completed rounds

### Requirement: Provisioning recipe with topology dial and quota guidance

The provisioning recipe SHALL support both a single-box-sequential topology and a fan-out-across-boxes topology, SHALL default to single-box-sequential, and SHALL use dedicated (CCX) vCPU for sustained rounds. It SHALL document the Hetzner new-account 8-dedicated-core quota and the support-bump path required before a wide parallel topology.

#### Scenario: Default single-box topology
- **WHEN** a league is provisioned without an explicit topology
- **THEN** it runs the round's specialists sequentially on one dedicated-vCPU box

#### Scenario: Quota wall is surfaced, not hit silently
- **WHEN** a parallel topology requests more dedicated cores than the project's quota allows
- **THEN** the recipe surfaces the quota limit and the documented bump path rather than failing opaquely

### Requirement: League image is concede-disabled and carries the driver

The training image used for the league SHALL be built from the concede-disabled engine (v0.35+) and SHALL carry (or mount) the standalone league orchestrator and specialist-registry tooling.

#### Scenario: Image runs the league with concede disabled
- **WHEN** the league image starts a round
- **THEN** the engine's action mask does not expose concede, and the orchestrator + registry tooling are available in the container

### Requirement: Teardown and budget are explicit orchestration steps

Each round's run procedure SHALL end with an explicit artifact-preservation-then-teardown step (download/snapshot to durable storage, then destroy or idle the box), and the league SHALL carry per-round and total budget estimates. Checkpoint retention SHALL be generous enough that the per-round best checkpoint is always recoverable.

#### Scenario: No idle box left billing after a round
- **WHEN** a round completes and its artifacts are persisted to durable storage
- **THEN** the procedure destroys or idles the compute, leaving no box billing for nothing

#### Scenario: Peak checkpoint is recoverable
- **WHEN** selecting the per-round best checkpoint for a deck
- **THEN** that checkpoint still exists (retention did not prune it), unlike the `keep_last=3` loss on the floor run

### Requirement: Per-deck specialist publishing, layout-gated and versioned

Publishing SHALL emit each specialist into the model manifest keyed by deck, tagged with its `tensor_layout_hash` and a version. A specialist whose layout hash does not match the app's active observation layout SHALL NOT be published as loadable.

#### Scenario: Specialists published keyed by deck
- **WHEN** a league round is published
- **THEN** the manifest gains one entry per deck (`deck → specialist model`) with layout-hash and version tags

#### Scenario: Layout-incompatible specialist is not published as loadable
- **WHEN** a specialist's `tensor_layout_hash` mismatches the app's active layout
- **THEN** it is rejected at publish time rather than shipped as a loadable model

### Requirement: Deck-keyed resolution at inference with generalist fallback

The in-app/desktop AI SHALL load the specialist matching the deck it is piloting, and SHALL fall back to the generalist when no specialist exists for that deck.

#### Scenario: AI loads the matching specialist
- **WHEN** the AI begins a game piloting deck `ST-2 Cocytus Blue` and a published specialist exists for it
- **THEN** it loads the `ST-2 Cocytus Blue` specialist

#### Scenario: Fallback when no specialist exists
- **WHEN** the AI pilots a deck with no published specialist
- **THEN** it loads the generalist model

### Requirement: League monitoring surfaces the matchup matrix

League runs SHALL expose per-specialist TensorBoard and mirror runs back for the inspection MCP, and SHALL surface the per-round deck-by-deck matchup matrix as the human-facing progress signal (not the in-run win rate).

#### Scenario: Matchup matrix is the dashboard signal
- **WHEN** an operator checks league progress
- **THEN** the per-round matchup matrix is available as the progress view, alongside per-specialist TensorBoard
