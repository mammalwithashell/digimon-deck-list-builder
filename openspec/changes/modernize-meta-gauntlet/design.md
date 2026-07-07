# Design — modernize-meta-gauntlet

## Context

`MetaGauntlet` (`code/digimon_gym/agents/gauntlet.py`) samples opponent decks for PPO training, weighted by a Threat Index over DigiLab tournament stats. The v2 fixes (survivorship bias, implementation gating, alias canonicalization) are sound; this change addresses what v2 left on the table, verified in-code on 2026-07-06:

- TI = `share·α + conversion·β` (α=1, β=2, `gauntlet.py:655-657`): conversion (0.3–0.6 range) dominates share (0.02–0.2 range) ~10×; `digilab_win_rate` is loaded (`:246`) but unused; `confidence_min_appearances=5` admits 5-appearance noise; the sleeper rule can pin a 5% sampling floor on a fluke.
- Weights are format-blind: TI aggregates all DigiLab history. The BT25 retrospective (DigiLab, 2026-07) shows why this is wrong: family/archetype shares moved >2× in one rotation, and the top decks (Glowing Dawn, 3M Beelstarmon) were new-from-scratch archetypes.
- "Deck Pool Routing" is dead code: decks are sorted by source preference (`:610`) but `_compute_sampling_weights` splits archetype weight evenly per deck. The alias-merge (`:562`) extends decklists across aliases with no dedup, double-weighting duplicate lists.
- Sampling weights are static after `load()`; per-archetype agent win rates are already logged per eval window but never fed back.
- `GauntletWrapper` adds a flat +0.5 terminal bounty for beating TI>0.15 opponents (`:846-850`) — a second emphasis mechanism on top of sampling, and an opponent-dependent distortion of the terminal value scale (interacts with rule 26 BO3 calibration and any future value-net training on these games).
- `GeneralistDeckPool` has hash-verified snapshots; `MetaGauntlet` does not — a `deck_library.json` rebuild mid-run silently changes pool and weights on resume.
- Opponent strength is capped at greedy: `GauntletWrapper` picks the deck, `OpponentWrapper` picks the policy, independently. Meanwhile `add-determinized-search` Phase 3 produces search-distilled ONNX checkpoints, and the Rust selfplay driver (`selfplay/driver.rs:214`) samples deck pairs uniformly with no weight support.

Constraints: rule 30 (promotion only on the anchored frame), rule 26 (do not change per-game/per-match reward magnitudes without updating the BO3 spec), rule 20 (PyO3 player-ID convention), layout-hash cohort gating (no cross-profile warm starts).

## Goals / Non-Goals

**Goals:**
1. Statistically defensible, format-scoped opponent weighting driven by shrunk win rates.
2. Deck-pool hygiene: dedup, real source/recency weighting, hash-verified reproducibility.
3. Opponent strength beyond greedy via jointly-sampled (deck, pilot) pairs using existing frozen policies.
4. The selfplay driver consumes gauntlet-exported weighted pools, so AZ training data is generated on the live meta distribution.
5. A written, testable warm-start lineage policy for format rotations.

**Non-Goals:**
- Family-taxonomy ingest / `deck_library.json` schema changes (separate change; this change consumes whatever archetypes exist).
- Live search opponents inside PPO rollouts ("boss episodes") — deferred until leaf batching (4.1) lands and a cost/benefit spike justifies it.
- PvP opponent-deck priors (add-determinized-search Phase 6).
- Any new reward shaping. This change only *removes* a reward term.

## Decisions

**D1 — TI formula: `TI = share × exp(c·(wr_adj − 0.5))`, win-rate-based, multiplicative.**
`wr_adj = (wins + k·0.5)/(games + k)` with k=25 (empirical-Bayes shrinkage toward a 0.5 prior). Rationale: share is the base rate of *facing* the deck; win rate tilts it toward decks that win. Multiplicative keeps share primary (a 0-share deck should not be sampled heavily no matter its win rate on 8 games); `exp` keeps weights positive and lets c (default 3.0) tune the tilt. Alternatives considered: (a) keep additive with retuned β — rejected, still lets the strength term swamp share at any fixed β because the two quantities have different scales; (b) `share × wr` linear — rejected, too weak a tilt near 0.5 to matter. Conversion rate is dropped from TI but retained in telemetry. The sleeper rule keys on `wr_adj > threshold` with the same shrinkage (which inherently enforces a real confidence floor; `confidence_min_appearances` becomes redundant and is retired). DigiLab must supply win/game *counts*, not just rates, for shrinkage — `get_scoped_meta` already returns `times_played`; win counts derive from `win_rate × times_played` if raw counts are unavailable.

**D2 — Format scoping via `override_meta_shares`-style injection, not a library rebuild.**
A new `--gauntlet-window` (date range or format tag resolved to the format's date range) triggers a DigiLab scoped query at run start; the result overrides per-archetype `share`/`wr` before weight computation. Rationale: the hook already exists (`override_meta_shares`), it keeps `deck_library.json` as the deck-pool source while making the *statistics* fresh, and it needs no ingest changes. The scoped stats used are written into the gauntlet snapshot (D4) so the run is reproducible even after the window's stats drift upstream. Offline fallback: if DigiLab is unreachable, fail loudly unless `--gauntlet-window-optional` is set (silent fallback to all-history weights would reintroduce the bug this change exists to fix).

**D3 — Within-archetype deck weighting replaces the dead sort.**
Deck weight within an archetype = `source_pref × recency_decay(event_date)`, normalized; dedup by `stable_deck_id` at merge time (first occurrence under the highest-preference source wins, sources recorded). Rationale: makes the documented routing real; recency matters because lists optimize over a format's lifetime. Alternative: placement-weighted — deferred, placement data is sparse and top-cut-biased.

**D4 — MetaGauntlet snapshot mirrors `GeneralistDeckPool`'s.**
Snapshot content: schema version, window, per-archetype scoped stats, TI parameters (c, k, τ), deck records (deck_id, card_ids, source, weight), pilot bindings (D6), and the final per-deck sampling weights; hash-verified on restore; path + hash recorded in run metadata. Adaptive-sampling state (D5) is *not* in the snapshot — it is derived live and re-derives on resume from the eval log.

**D5 — Adaptive sampling as a post-multiplier, recomputed per eval window.**
`w_i ∝ TI_i × (1 − wr_agent(arch_i))^τ`, τ default 1.0, `wr_agent` from the existing per-archetype eval telemetry with the same k=25 shrinkage (an archetype with 3 eval games does not move its weight much). Floor each eligible archetype at `w_min` (default 0.5× its TI-proportional weight) so nothing is starved and the distribution never collapses onto one matchup. Applied in `GauntletWrapper` via a `reweight(wr_by_arch)` call from the training loop's eval callback. Rationale for post-multiplier over replacing TI: keeps the meta prior authoritative and the adaptation bounded; trivially disabled (τ=0) for A/B.

**D6 — Joint (deck, pilot) sampling; pilots are frozen ONNX policies bound to deck entries.**
Pool entries gain an optional `pilot` binding: `{model_path, tensor_profile, decks_trained_on}`. Binding rules: a pilot may only bind to decks it was trained on (exact `stable_deck_id` match, or same-archetype match when the pilot was pool-trained on the archetype); unbound decks fall back to greedy. Sampling draws the (deck, pilot) pair as a unit. Implementation: `GauntletWrapper` passes the pilot spec through `info`/options to `OpponentWrapper`, which already knows how to run frozen ONNX opponents (`DIGIMON_ONNX_OPPONENT` lever). Rationale: fixes the greedy ceiling at inference cost only, and prevents the incoherent case (champion trained on ST decks piloting Beelstarmon). Alternative — independent policy/deck sampling — rejected for exactly that incoherence.

**D7 — Selfplay driver takes optional per-deck weights; gauntlet exports them.**
`MetaGauntlet.export_selfplay_pool(path)` writes `[{name, cards, weight}]`. The driver's deck-pair loop replaces uniform `gen_range` with weighted sampling (alias method or cumulative-sum; deterministic under the existing master seed) when weights are present; `manifest.json` records the weights, source snapshot hash, and seed. Pair sampling stays independent per seat (mirror matches allowed), matching current semantics. Rationale: ~20-line Rust change; makes AZ generations train on the field-weighted distribution. `run_selfplay_generation.py` gains `--deck-pool <exported.json>` pass-through. Alternative — reweighting shards after generation — rejected: importance-weighting π/z rows is statistically messier than sampling correctly at the source.

**D8 — Warm-start lineage policy (doctrine, enforced by tooling).**
Per format: (1) fine-tune the previous format's promoted generalist on the new format's gauntlet-weighted distribution (PPO or AZ regime); (2) promotion requires passing the format-scoped anchored frame (field-weighted matchup matrix vs the format pool, seat-balanced, rule 30) — thresholds are pre-registered per format, never inherited; (3) promoted checkpoints freeze into the champion registry with format + window + snapshot-hash provenance; (4) every N=3 formats, run one equal-compute from-scratch training and A/B it against the lineage on the anchored frame (lineage-calcification check). New-card embedding rows may initialize from attribute-nearest-neighbor cards (export-path option, off by default). Cross-profile warm starts remain forbidden (layout-hash gate).

**D9 — Bounty removal sequencing.**
The bounty is removed in the same release that enables adaptive sampling (D5), not before — emphasis must move, not vanish. Because this changes episode reward composition, the change is flagged per rule 26; per-game/per-match magnitudes themselves are untouched (the bounty was additive on top of them). A `--legacy-bounty` escape hatch is kept for one release for A/B, then deleted.

## Risks / Trade-offs

- [Scoped DigiLab stats are thin early in a format] → shrinkage (D1) degrades gracefully toward share-only weighting; the window can include the prior format's last weeks via an explicit `--gauntlet-window` range.
- [Adaptive sampling chases eval noise] → shrinkage on `wr_agent`, per-eval-window cadence (not per-episode), weight floor, τ knob for A/B; telemetry logs the effective distribution each window.
- [Pilot/deck binding gaps: most decks have no trained pilot initially] → greedy fallback is explicit and logged; the by-pilot mix is a logged metric so "how much of training pressure is search-strength" is observable, not assumed.
- [Weighted driver sampling breaks selfplay determinism] → weights are part of the seeded RNG stream and the manifest; a golden test replays a manifest and asserts identical pair sequence.
- [Removing the bounty changes learning dynamics on in-flight recipes] → sequenced with D5, `--legacy-bounty` A/B window, anchored-frame comparison before the flag is deleted.
- [ONNX opponents with wrong tensor profile] → pilot bindings carry `tensor_profile`; mismatch with the env profile fails at wrapper construction, not mid-run.
- [Lineage calcification (inherited blind spots)] → D8's periodic rebase A/B plus the exploiter lower bound per promoted checkpoint.

## Migration Plan

1. Land D1–D4 (pure Python, gauntlet-internal) behind unchanged defaults except the TI formula; a `--ti-legacy` flag preserves the old formula for one comparison run.
2. Land D5 + D9 together; run one A/B (adaptive+no-bounty vs legacy) on the anchored frame before deleting legacy flags.
3. Land D6 with greedy-fallback-everywhere as the initial state; bind existing league specialists + AZ checkpoints as they are validated.
4. Land D7 (Rust + export + orchestrator pass-through); golden determinism test; then switch the standing AZ recipe to the exported pool.
5. D8 is documented in `docs/TRAINING_RUNBOOK.md` + `docs/MODEL_EVALUATION.md` and enforced by the promotion tooling's provenance checks.

Rollback: every step is flag-gated or additive; reverting to current behavior = legacy flags on / omit weights file.

## Open Questions

- Should the format window resolve from a format *tag* (needs the format-aware ingest change to land a tag→date-range map) or stay date-range-only until then? (Default: date-range-only now.)
- k and c defaults (25, 3.0) are priors, not measurements — calibrate on BT25 retrospective data by checking the induced distribution against the top-cut table.
- Does `get_scoped_meta` need a win/loss *count* column added to the DigiLab query, or is `win_rate × times_played` reconstruction acceptable long-term?
- Whether AZ-regime fine-tunes (D8) should reset the replay-buffer window across formats or blend the last generations of the prior format.
