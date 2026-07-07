# Modernize Meta Gauntlet: format-scoped weighting, search-strength opponents, meta-weighted self-play, warm-start lineage

## Why

MetaGauntlet is the mechanism that points training at the real tournament meta, but its weighting is statistically miscalibrated (conversion-rate-dominated Threat Index with a 5-appearance confidence floor; `digilab_win_rate` loaded but never used), format-blind (TI aggregates all DigiLab history, so dead formats vote on the current training distribution — BT25 showed Time Strangers jumping 10.3%→22.7% across one rotation), and capped in strength (every sampled meta deck is piloted by greedy, so "threat-weighted" sampling delivers greedy-level pressure everywhere). Meanwhile the determinized-search stack (`add-determinized-search`, Phases 0–3 landed) produces search-distilled policies and a self-play driver that could consume the gauntlet's meta distribution directly — but the driver samples deck pairs uniformly and the gauntlet has no export for it. With sets rotating every ~2 months, the project also needs a stated warm-start policy so per-format retraining is days of fine-tuning, not weeks from scratch.

## What Changes

- **Threat Index overhaul**: replace `share·α + conversion·β` with a multiplicative form on empirical-Bayes-shrunk win rate (`TI = share × exp(c·(wr_adj − 0.5))`); apply the same shrinkage to the sleeper rule; retire raw conversion rate as a weighting input.
- **Format/date scoping**: gauntlet weights come from a DigiLab query scoped to a format window (`--gauntlet-window`), via the existing `override_meta_shares` hook + `digilab_client.get_scoped_meta`.
- **Deck-pool hygiene**: dedup decklists by `stable_deck_id` across sources/aliases; make within-archetype deck sampling actually weighted by source preference and recency (today's source-preference sort is dead code — sampling splits evenly).
- **Adaptive opponent sampling**: blend live per-archetype agent win rates (already logged per eval window) into sampling weights (`∝ TI × (1 − wr_agent)^τ`), recomputed per eval window.
- **Bounty retirement**: **BREAKING** (training-behavior): remove the flat terminal bounty bonus once adaptive sampling lands — emphasis moves entirely into the sampling distribution so terminal rewards stay pure win/loss for value-target hygiene.
- **Gauntlet snapshot**: MetaGauntlet gains the same hash-verified snapshot/restore treatment `GeneralistDeckPool` already has, so resumes survive `deck_library.json` rebuilds.
- **Joint (deck, pilot) opponents**: gauntlet entries can carry a pilot (greedy | frozen ONNX policy, e.g. AZ-generation checkpoints or league specialists); deck and pilot are sampled **jointly** so distilled-search pilots play the decks they were trained on.
- **Meta-weighted self-play**: `MetaGauntlet.export_selfplay_pool()` emits a weighted deck pool; the Rust selfplay driver accepts optional per-deck weights and samples pairs from them (today: uniform `gen_range`), with weights recorded in the generation manifest for reproducibility.
- **Warm-start lineage policy**: per-format training warm-starts from the previous format's promoted generalist; promotion criteria do not inherit (each format's checkpoint must pass its own field-weighted anchored frame); promoted checkpoints freeze into the champion registry; periodic equal-compute from-scratch rebase runs as an A/B hygiene check.

## Capabilities

### New Capabilities
- `meta-scoped-gauntlet`: statistically sound, format-windowed, reproducible opponent-deck weighting (TI formula, shrinkage, scoping, dedup, weighted deck routing, adaptive sampling, snapshot, bounty retirement)
- `search-strength-opponents`: joint (deck, pilot) opponent sampling so gauntlet decks are piloted by distilled-search policies where available instead of greedy everywhere
- `meta-weighted-selfplay`: gauntlet-exported weighted deck pools consumed by the AlphaZero self-play driver, so π/z training data is generated on the live meta distribution
- `format-warm-start-lineage`: the per-format retraining doctrine — warm-start lineage, non-inherited promotion gates, champion freezing, periodic rebase checks

### Modified Capabilities
<!-- No existing openspec/specs/ capability covers gauntlet or self-play training; the selfplay driver spec lives in the still-active add-determinized-search change and is extended here via the new meta-weighted-selfplay capability rather than a delta. -->

## Impact

- **Python (training)**: `code/digimon_gym/agents/gauntlet.py` (TI, scoping, dedup, weighted routing, adaptive weights, snapshot, bounty removal), `code/digimon_gym/agents/pilot_training.py` (flags, wiring, telemetry feedback loop), `code/server/digilab_client.py` (format-window query surface if the existing scoped query needs a format tag), `code/tools/run_selfplay_generation.py` (weighted-pool pass-through)
- **Rust (engine CLI)**: `code/digimon-engine/src/selfplay/driver.rs` + `digimon-engine-cli` `selfplay` subcommand (optional per-deck weights; manifest records weights + seed)
- **Data**: consumes format/date-scoped DigiLab stats; depends on `deck_library.json` re-ingest freshness (currently missing new-format archetypes — tracked separately as the family/format-aware ingest work)
- **Interactions**: composes with `add-determinized-search` Phase 3 artifacts (AZ checkpoints as pilots, selfplay driver); BO3 reward calibration (rule 26) constrains the bounty change; anchored-eval discipline (rule 30) governs the lineage promotion gates
- **Not in scope**: family taxonomy ingest (separate change), live search opponents inside PPO rollouts (boss episodes deferred), PvP deck-prior work (Phase 6 of add-determinized-search)
