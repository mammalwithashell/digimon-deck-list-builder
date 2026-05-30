## Why

We have batch funnels that flag *which* games are suspicious — `dcgo-replay` produces a parity report keyed by card, the training eval log surfaces crashes / draws / anomalous games — but no interactive microscope for an agent to take **one** flagged game, step through it with full card and action-decoder context, move back and forth, and confirm + localize the actual engine bug. The replay substrate already exists (native `ReplayRunner`, the opaque-capable `dcgo-replay` path, the 26-tool engine MCP), but the two replay paths have diverged, backward stepping is a full rebuild, and there is no agent-facing "what did this step do, and is it correct?" surface. This change unifies the two paths into one steppable core and gives Claude a guided bug-hunting workflow on top of it.

## What Changes

- **Unify the two replay paths into a single steppable core.** Promote `ReplayRunner` (native `GameRecorder` JSON) and `dcgo-replay`'s `replay_recording` (DCGO JSONL, deterministic + opaque) into one `ReplaySession` parameterized over a `RecordingSource` adapter (`NativeAdapter`, `DcgoAdapter`). Per-step policy becomes a knob: **Trust** (self-play audit) vs **CheckThenApply** (differential, mask-membership + actor check). Divergences are recorded **non-fatally and pause** the session for inspection rather than aborting.
- **Move the DCGO `RecordingV1` parser into the engine** so the engine (and therefore the MCP, which depends only on the engine) can build a `DcgoAdapter` directly. `dcgo-replay` is reduced to a thin batch driver over `ReplaySession`; its `ReplayOutcome` / parity-report output is unchanged.
- **Snappy bidirectional stepping.** Arc-wrap the engine's shared immutable state (`card_data` / registries) so a checkpoint clones only the mutable game graph; add a checkpoint ring so `step_back` / `seek(n)` restore the nearest checkpoint and replay the remainder instead of rebuilding from scratch. For opaque games, the checkpoint stores the **reveal-cursor index** and re-attaches a fresh `RevealQueue` on restore (the `Box<dyn RevealSource>` is intentionally non-`Clone`). This also unblocks counterfactual A/B ("restore checkpoint, take a different action").
- **Fat per-step view.** A `replay_step_view(step_n)` that returns the decoded recorded action, the engine's full decoded legal-action set, any divergence, the events emitted by applying the action, a before/after state delta, and the card IDs in play — one object both bug-hunting modes read with different intent.
- **Partial-observability surfacing.** Expose the existing `is_opaque_placeholder` flag in the view layer so opaque-game inspection reads "hidden" instead of a bogus card ID.
- **New MCP tools** on `digimon-engine-mcp`: `load_recording` accepts DCGO JSONL (deterministic + opaque) in addition to native JSON; real `seek` / `step_forward` / `step_back` / `restore_checkpoint`; `replay_step_view`; cheap mechanical scanners `scan_divergences` (Mode 1) and `scan_fizzles` / `scan_panics` (Mode 2 leads). Register the engine MCP in `.mcp.json` (currently commented out).
- **New `/replay-bug-hunt` skill** with two playbooks: **Mode 1 (differential)** — load a DCGO recording (bot or opaque PvP), step to the first divergence, back-step to inspect the lead-up, localize to a card, confirm against DCGO C# + `general_rule.pdf`; **Mode 2 (judge)** — load a self-play / eval game, judge each effect ("did the removal fire on the right target?") against card text + rules PDF + DCGO C#, write a verdict.

## Capabilities

### New Capabilities

- `interactive-replay-stepper`: the unified `ReplaySession` core (adapter trait, Trust vs CheckThenApply policy, pausable non-fatal divergence, checkpoint ring + snapshot/restore, fat step view, partial-observability surfacing) and its `digimon-engine-mcp` exposure (DCGO + native loading, step/seek/back/restore, step-view, mechanical scanners).
- `replay-bug-hunt-skill`: the `/replay-bug-hunt` agent workflow — the Mode 1 differential playbook, the Mode 2 judge playbook, the oracle framing per recording source, and where confirmed findings are written.

### Modified Capabilities

- `recording-replay`: `ReplayRunner` is generalized into the adapter-driven `ReplaySession`; backward seek becomes checkpoint-based (snappy) rather than rebuild-and-rewalk; snapshot/restore and a DCGO adapter are added. Existing native-replay behavior is preserved through `NativeAdapter`.
- `engine-debug-mcp`: `load_recording` gains DCGO-JSONL ingestion; the stubbed `seek` is implemented and joined by `step_forward` / `step_back` / `restore_checkpoint` / `replay_step_view` / `scan_divergences` / `scan_fizzles` / `scan_panics`.

## Impact

- **Engine** (`code/digimon-engine/`): Arc-wrap shared immutable state for cheap `Game` snapshot/restore (the v1.5 prerequisite already noted in `DEBUG_MCP.md`); `runners/replay.rs` promoted to the `ReplaySession` core + `RecordingSource` adapters; DCGO `RecordingV1` parser moved in from the tool crate; `is_opaque_placeholder` surfaced in `view/`. Existing engine tests preserved via `NativeAdapter`.
- **MCP** (`code/digimon-engine-mcp/`): new + revised tools and registry handling for the persistent session cursor; `.mcp.json` registration.
- **Tool** (`code/tools/dcgo-replay/`): reduced to a batch driver over `ReplaySession`; parity-report output byte-stable (existing determinism test must continue to pass).
- **Skill** (`.claude/skills/replay-bug-hunt/`): new skill + playbooks; findings routed to existing trackers (`docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`).
- **Docs**: `docs/DEBUG_MCP.md` (tool surface, lift the "seek stubbed" / "no branching" v1 limitations), `docs/RUST_ENGINE_API.md` (snapshot/restore contract).
- **Dependency**: builds on `add-dcgo-recording-parity-harness` (the DCGO parser, opaque-opponent engine mode, and `ReplayOutcome` types). That change should land first; this one consumes and refactors its replay path.
- **Not changed**: the 2192-action layout, the DCGO JSONL schema, the native `GameRecorder` JSON schema, trained models.
