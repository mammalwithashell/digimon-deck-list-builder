## Context

The policy is `obs(8850) → CardEmbeddingExtractor(→512) → heads → {2192 action logits, 1 value}`. The MLP league path never set `net_arch`, so it inherited the SB3 default `[64,64]` — the 512-dim representation is compressed to a 64-wide path before every decision. The extractor (learned per-card embeddings + projection, optionally warm-started from `card_embeddings.npy`) is the sophisticated part; the 64-wide heads are the suspected weak link.

This change was scoped after a long throughput investigation concluded that engine/IPC/batch levers don't move *training* (training is update-bound; the engine is a few %). Sample efficiency is the better axis, and head width is the cheapest first test. The `--net-arch` and `--init-extractor-from` knobs were implemented and manually verified this session (built `512→256→256`, warm-started 15 extractor tensors) but are uncommitted, and the comparison run was deferred — this proposal captures finishing it.

## Goals / Non-Goals

**Goals:**
- A configurable, tested `--net-arch` knob (default byte-identical to today) + extractor-only warm-start, threaded through the league driver.
- A *fair*, headroom-preserving, anchored-eval-judged comparison of `[64,64]` vs `[256,256]` that yields a clear verdict.
- Encode the eval discipline learned this session so the experiment can't silently ceiling.

**Non-Goals:**
- Architectural changes to the extractor (attention/set-encoder over card slots) — that's the higher-ceiling follow-up, out of scope here.
- Tuning `n_epochs`/other PPO hypers (a separate sample-efficiency arm; may be a fast-follow but not required for the verdict).
- Any change to production defaults or a full from-scratch curriculum re-run.

## Decisions

- **Extractor-only warm-start is mandatory for fairness.** You cannot `--init-from` a `[64,64]` seed into a `[256,256]` model (shape mismatch). Training both widths *from scratch* either ceilings (vs greedy) or floors (vs a strong opponent) in a short budget, and the ~9.8M shared extractor/projection params swamp the ~0.75M head difference. Transferring the seed's extractor and re-learning only the heads isolates the variable.
- **Champion-pool opponent + anchored-eval judge.** Greedy is non-discriminating — a fresh model beats it 100% within 1–2 PPO updates (observed three times this session). The opponent must have headroom (champion pool from `models/specialists/`), and the verdict comes from anchored eval vs greedy + champions, never the in-run win rate.
- **Both arms share the extractor warm-start + seed + hypers**; only `net_arch` differs. Equal step budget (~300–500k); rank by anchored champion win rate.
- **Default unchanged.** `net_arch=None` → SB3 `[64,64]`; the knob is opt-in.

## Risks / Trade-offs

- **Re-learning heads on a frozen-ish extractor may not reach the seed's tuned heads** in a short budget — both arms could land below the seed. Mitigation: compare *relative* to each other at equal steps, not to the 5M seed; extend budget if both are still climbing.
- **The extractor itself may be the real bottleneck**, not the heads — a null result (wider ≠ better) is a valid, useful outcome that redirects effort to the architectural follow-up.
- **Champion-pool training adds per-worker opponent inference**, which is where batch/thread pathologies bit the league (see `project_league_config_failures`); keep this comparison sequential / modest `n_envs` and judge by anchored eval, not fps.
