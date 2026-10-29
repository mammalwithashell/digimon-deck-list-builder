## Why

Current determinized self-play uses the opponent's exact deck composition to generate search targets. Closed-decklist players lack that information, so stronger search targets can still teach assumptions that fail against unfamiliar decks; robustness needs fair-information training and independent evidence.

## What Changes

- Add explicit `closed_decklist` and `open_decklist` information regimes. Preserve exact-list search for the latter; require belief-based search for closed-list training and evaluation.
- Infer a distribution over legal opponent decklists from a versioned candidate population and viewer-entitled evidence, including public trash and board counts. Support unseen lists with a constrained fallback instead of consulting the actual opponent list.
- Materialize playable worlds from the inferred composition rather than re-dealing the true hidden-card pool. Track legitimate reveal/peek knowledge through moves, shuffles, and forgetting events.
- Train both seats with fair search targets and unchanged observation/action contracts, recording regime, prior, legality policy, and evidence provenance in artifacts and resumes.
- Evaluate policy-only deployment separately from search-assisted play on held-out decklists, long-game counting scenarios, fixed anchors, and budget-matched exploiters. Require independent evidence before model promotion.
- Clarify ownership of the overlapping inferred-prior work in `add-determinized-search`: this change owns closed-list priors and training; replay UI/classification presentation remains there.

## Capabilities

### New Capabilities

- `fair-opponent-beliefs`: Viewer-scoped knowledge, legal deck priors, constrained unknown-list fallback, and composition-independent world construction.
- `closed-decklist-selfplay`: Explicit information regimes, fair search targets, artifact provenance, and resume compatibility.
- `belief-robustness-evaluation`: Information-leak guards, held-out evaluation, counting probes, and exploiter-based promotion evidence.

### Modified Capabilities

None in the archived base specs. The active `add-determinized-search` opponent-deck-prior proposal requires a coordinated amendment: exact-list self-play is an explicit open-list regime, not a universal training requirement.

## Impact

- Rust: `code/digimon-engine/src/determinize/`, `search/`, `selfplay/`, viewer knowledge/event plumbing, and existing deck-legality services.
- CLI/training: `code/digimon-engine-cli/src/selfplay.rs`, `code/digimon_gym/agents/azero_trainer.py`, `code/tools/run_selfplay_generation.py`, anchored evaluation and `agents/exploiter.py`.
- Documentation: training runbook, model evaluation, artifact metadata, and the active determinized-search proposal/spec/tasks.
- No action-space or tensor-layout change is authorized. Any required observation expansion must be separately planned with synchronized contracts and model compatibility.
- This proposal does not assert improved win rate, calibrated beliefs, or certified low exploitability; those require the specified experiments.
