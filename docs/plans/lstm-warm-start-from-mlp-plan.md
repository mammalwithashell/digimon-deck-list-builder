# LSTM Training Plan — Warm-Starting from Existing MLP Models

**Status:** Research / planning only. No code changes proposed yet.
**Date:** 2026-06-28

---

## TL;DR — the premise needs adjusting

The framing of the request was *"add LSTM training, warm-starting from MLPs."* But the repo **already has full LSTM training**, has done so for some time, and the MLP and LSTM paths **already share the feature encoder**. Concretely:

- `pilot_training.py` accepts `--lstm`, instantiates a custom `MaskableRecurrentPPO` (LSTM hidden=256, separate actor/critic LSTMs), and ONNX export + inference both have an LSTM path (`tools/export_onnx.py`, `inference/onnx_policy.py`).
- There is already a warm-start flag — `--init-extractor-from <ckpt>` — that performs partial state-dict surgery, copying only `features_extractor.*` keys.
- That flag is, however, **algo-pinned**: it loads the source checkpoint with the *current* run's algo class, so as-shipped it does **MLP→MLP** (e.g. comparing `--net-arch` widths), not **MLP→LSTM**. Fixing that is a small, targeted change.

So the real plan is **not** "build LSTM from scratch." It's:

1. Unlock the existing warm-start primitive to cross MLP→LSTM.
2. Decide what (beyond the encoder) can be transferred, and what must re-learn.
3. Run a warm-started LSTM and prove it's actually better than fresh LSTM on the anchored eval frame.

If you were imagining building LSTM training from scratch, you can drop most of that scope. The architecture choices have already been made; this work is incremental.

---

## What exists today

### Training entrypoint
`code/digimon_gym/agents/pilot_training.py`

- Library: **`sb3_contrib.MaskablePPO`** for MLP runs (`pilot_training.py:39`).
- LSTM runs: **local `MaskableRecurrentPPO`** (subclass of `sb3_contrib.RecurrentPPO` with masking re-injected — `maskable_recurrent/maskable_recurrent_ppo.py:41`). This is a small in-tree fork, not a clean external dependency.
- `--lstm` flag toggles the path at `pilot_training.py:2871-2896`.

### Shared encoder (the load-bearing fact)
`code/digimon_gym/agents/features_extractor.py:29` — `CardEmbeddingExtractor`

- `nn.Embedding(20000, 16, padding_idx=0)` over card-ID slots, concatenated with scalar positions, then `Linear(combined_dim, 512) + ReLU`. Output `features_dim=512`.
- **Both MLP and LSTM policies instantiate this same class with the same kwargs** (`pilot_training.py:2841-2848`). This is what makes encoder warm-start trivial.

### What is *not* shared
- Policy class. MLP uses SB3's stock `ActorCriticPolicy` via `"MlpPolicy"`; LSTM uses local `MaskableRecurrentActorCriticPolicy` (`maskable_recurrent/policies.py:350`), which adds `lstm_actor` + `lstm_critic`.
- Head input width. MLP: `policy_net.0 = Linear(512 → 64)`. LSTM: `policy_net.0 = Linear(256 → 64)` because the LSTM output (hidden=256) replaces features. Same parameter *names*, incompatible *shapes*.
- LSTM-specific parameters obviously don't exist on the MLP side.

### Warm-start flags already in `TrainingConfig`
`code/digimon_gym/agents/training_config.py:91-100`, used at `pilot_training.py:2852-2943`

| Flag | What it does | Cross-arch? |
|---|---|---|
| `--resume-from` | Full SB3 `load()` + continued step counter | No (same algo class) |
| `--init-from` | Full SB3 `load()` + reset step counter | No (same algo class) |
| `--init-extractor-from` | Filtered state-dict copy of `features_extractor.*` keys only | No, as shipped — but the surgery template is right there at lines 2924-2943 |

### Env / observation surface
`code/digimon_gym/digimon_gym.py` + `docs/TENSOR_SPEC.md`

- Active default observation profile is **`standard_lite_deck_v2`** at **8850 floats** (not 1375 — that's the retired `standard_compact_v1`, dropped 2026-05-30).
- Per-seat perspective: agent gets `runner.get_board_tensor(1)`; opponent gets `get_board_tensor(2)`. Mirrored.
- **Opponent hand is hidden in the agent's obs.** Deck order is hidden. Face-down security identities are hidden. This is a genuine POMDP, not a Markov env with a flat tensor.
- Action space: **2192**, identical between MLP and LSTM. No re-shaping needed.

### Sequence handling
`code/digimon_gym/agents/maskable_recurrent/buffers.py:37`

- `MaskableRecurrentRolloutBuffer` extends SB3's `RecurrentRolloutBuffer`, sequences split at `episode_starts`, BPTT runs the full sequence length.
- No explicit BPTT-length knob. Effective BPTT = min(episode length, `n_steps`).
- LSTM run forces `batch_size = n_steps` (a `RecurrentPPO` constraint, `pilot_training.py:2877`). Default `n_steps=2048`.
- BO3 wrapper (`agents/match_env.py:11-15`): hidden state **carries across games within a match** via `reset_inner_only` (per CLAUDE.md rule 26).

### Inference + export
- `tools/export_onnx.py` already handles `--type lstm` (lines 206-251).
- `inference/onnx_policy.py:95-148` has an LSTM path; `reset()` is called at episode boundaries per CLAUDE.md rule 10.
- Opponent pool can already host LSTM opponents (`pilot_training.py:497-515`).

---

## Warm-start options, ranked

All four assume the MLP `.zip` lives at `models/<run>/final.zip` and a new LSTM run is being launched. None of these require building anything from scratch — they're variations on what the existing surgery template at `pilot_training.py:2924-2943` already does.

### Option A — Encoder transplant only (lowest risk, recommended starting point)

**What:** Lift just the `features_extractor.*` keys (embedding + projection, ~10M params) from the MLP into a freshly-initialized LSTM model. Heads + LSTM weights stay random.

**How:** Generalize `--init-extractor-from` so it loads the *source* checkpoint with `MaskablePPO.load` regardless of `use_lstm` for the current run. One-line fix at `pilot_training.py:2925`, or a new `--init-extractor-source-algo {mlp,lstm,auto}` flag if you want it explicit. Existing filter at lines 2932-2941 (`"features_extractor" in k` + shape match) already does the right thing.

**Trade-offs:** Safe. Heads have to relearn (LSTM-output-conditioned, so they couldn't have been transferred anyway). Embedding table starts already meaningful, which is the biggest single warm-start win — the embedding is 20000×16 = 320k params that took the MLP a long time to shape.

**Expected outcome:** Faster early learning vs cold LSTM, possibly equal or marginally better final policy. Worth doing first because the diff is tiny.

### Option B — Encoder + freeze for N steps, then unfreeze

**What:** Same transplant as A, but freeze `features_extractor` for the first ~200-500k steps so the LSTM and heads can adapt to fixed features before the extractor starts drifting. Then unfreeze.

**How:** After surgery, iterate `model.policy.features_extractor.parameters()` and set `requires_grad = False`. Hook a `BaseCallback.on_step` to flip it back on at a configurable step.

**Trade-offs:** Stabilizes early training (no embedding thrashing while LSTM is random). Costs a callback + freeze logic. Modest win on top of A.

### Option C — Distillation (MLP teacher → LSTM student)

**What:** Use the trained MLP as a frozen action-distribution teacher; train the LSTM with PPO **plus** a KL-to-teacher regularizer (decaying weight). The LSTM gets to imitate the MLP's reflex behavior while learning to use memory where it actually pays off.

**How:** Wrap or extend `MaskableRecurrentPPO.train()` to add a `kl_to_teacher` term. Teacher is a frozen `MaskablePPO` loaded at init. Schedule the coefficient from ~0.5 → 0 over the first ~1M steps. Teacher and student share the env's masked action space, so KL is well-defined per-step.

**Trade-offs:** More work than A/B. Real upside is that the MLP's *behavior* transfers, not just its features. Risk: if the MLP is itself weak (which the anchored eval frame can tell you — see docs/MODEL_EVALUATION.md), distillation just locks in its mistakes. Only do this if the MLP beats greedy comfortably on the anchored panel.

### Option D — Full re-architecture: shared encoder + dual heads

**What:** Refactor `features_extractor.py` so the encoder lives once and both an MLP head and an LSTM head can attach. Train them jointly or sequentially.

**Trade-offs:** Large change, dubious payoff given the existing structure already shares the encoder. Listed for completeness — **don't do this** unless the goal expands to "switchable inference mode."

### Recommendation

Start with **A**, measure on the anchored eval frame (greedy + frozen champions, seat-balanced — `tools/anchored_eval_cli.py`), then layer in **B** if early training is unstable. Only reach for **C** if the MLP itself is a strong reference and you want to lock in its tactics while LSTM learns memory.

---

## Observation-shape implications

This is **not** the bottleneck. The existing LSTM path already handles everything that flat-vs-sequence needs:

- Obs tensor stays the same shape (8850 floats, single timestep). The LSTM sees a sequence because the rollout buffer collects sequential timesteps from `episode_starts` markers — not because the obs itself becomes a sequence.
- `MaskableRecurrentRolloutBuffer` already pads, slices, and threads `lstm_states` correctly.
- Episode boundaries are well-defined: env `reset()`, plus `MatchEnv.reset_inner_only` between games within a BO3 match (state intentionally *not* reset there).
- No replay buffer to worry about — this is PPO, on-policy, rollout-based.

**The real question to validate** is whether the env's partial observability is large enough that recurrence pays off:

- Hidden from each seat: opponent hand, opponent deck order, opponent face-down security, your own face-down security (security checks reveal cards into the past), digivolution-stack contents under some encodings.
- Plausibly useful for LSTM memory: tracking what's *already been revealed* from the opponent's security stack, tracking opponent plays of cards that imply specific decklists (e.g. early reveal of a meta key card), tracking your own play sequence within a turn for combo timing.
- Counter-argument: a lot of partial-observability that matters in Digimon is *structured* (decklist priors, archetype identification) rather than *temporal*. A good encoder + opponent-modeling head might capture more than naive LSTM.

So the prior on LSTM helping is positive but not overwhelming. The anchored eval will tell you.

---

## Training plan (concrete)

### Phase 0 — Make `--init-extractor-from` cross-arch (1-day diff)

Edit `pilot_training.py:2925` (the line that picks `MaskableRecurrentPPO.load` vs `MaskablePPO.load`) so the *source* algo is detectable from the checkpoint or specifiable via a new flag. Update `TrainingConfig` to expose it. Existing filter + merge logic at 2932-2943 already does the safe thing (skip mismatched keys).

Add a guardrail test that asserts: after surgery, `features_extractor` parameter tensors are bitwise-equal to the source's, and **no other** parameter changed.

### Phase 1 — Smoke run

```
python -m digimon_gym.agents.pilot_training \
  --lstm \
  --init-extractor-from models/<best_mlp_run>/final.zip \
  --timesteps 50000 \
  --archetypes <one or two stable archetypes>
```

Just verify it loads, runs, and the run finishes without NaNs. Inspect TB scalars.

### Phase 2 — Real comparison

Two paired runs at the same seed/budget:

1. Fresh LSTM (control), `--timesteps 1000000`.
2. Warm-started LSTM via Option A, same budget.

Same `--match-format bo3` (default per CLAUDE.md rule 26). Same anchored-eval callback cadence (default 100k per CLAUDE.md rule 30). Same deck pool snapshot.

Compare via **anchored eval only** (`tools/anchored_eval_cli.py --deck-pool-snapshot <run>/deck_pool_snapshot.json --n 200+`). **Do not** trust the in-run eval win rate — it's degenerate against the training opponent (CLAUDE.md rule 30, `docs/MODEL_EVALUATION.md`).

### Hyperparameter starting points

Inherit from current LSTM defaults:

| Param | Default | Notes |
|---|---|---|
| `lstm_hidden_size` | 256 | Don't change for first run |
| `n_lstm_layers` | 1 | Don't change |
| `enable_critic_lstm` | True | Keep |
| `n_steps` | 2048 | Forces `batch_size = 2048` for LSTM (RecurrentPPO constraint) |
| `learning_rate` | whatever the current LSTM default is | Consider 0.5× for the warm-start run since the encoder is already trained |
| `ent_coef` | current default | Same |
| Net arch | `dict(pi=[64], vf=[64])` | LSTM default |

If memory pressure (LSTM + BPTT on `n_steps=2048` is expensive), drop `n_steps` to 1024 before reducing `lstm_hidden_size`.

### Eval cadence

- In-training anchored panel: every 100k steps (existing `anchored_eval_freq`). This catches collapse; **does not** decide promotion.
- Post-hoc anchored frame at end of run, `--n 400+` to beat deck-luck noise.
- Optional: exploiter probe (`code/digimon_gym/agents/exploiter.py`) if the warm-started LSTM clears greedy comfortably.

---

## Risks & open questions

These belong in a follow-up review before any code lands.

1. **Is the source MLP actually strong?** Anchored-eval the candidate MLP checkpoints against greedy + champion registry before treating any of them as a warm-start seed. A weak teacher poisons options C and may not meaningfully beat fresh LSTM in option A.
2. **Embedding-table covariate shift.** The MLP shaped its embedding to feed a 512-wide projection consumed by an MLP head. Whether those embeddings remain optimal once an LSTM sits between them and the heads is empirical. Option B (freeze-then-unfreeze) is the cheapest hedge.
3. **`MaskableRecurrentPPO` is a local fork of `sb3_contrib.RecurrentPPO`.** Pinning a `sb3_contrib` version and tracking upstream changes is on you — that's a maintenance question, not a warm-start one, but it's worth raising now if the team is going to lean harder on the LSTM path.
4. **Observation profile drift.** The retired `standard_compact_v1` (1375 floats) leaked opponent hand, which weakens the LSTM-helps argument. The active `standard_lite_deck_v2` (8850) does not — confirm any MLP you use as warm-start was trained on the *same* profile as the new LSTM run, otherwise the embedding table is shaped for a different observation contract and the transplant won't behave as expected. (The training contract validator at `pilot_training.py:324-362` should catch mismatch; double-check it covers profile identity, not just tensor size.)
5. **Cross-game state carryover (BO3).** LSTM hidden state intentionally persists across the three games of a match (`MatchEnv.reset_inner_only`). This is correct for Digimon (sideboarding info, opponent reads), but it does make warm-started runs more sensitive to how the MLP's representation generalizes across deck-pair compositions. Worth a sanity check that anchored eval also uses BO3.
6. **Decision needed: do we want a brand-new flag, or do we generalize the existing one?** A new `--warm-start-from-mlp` flag is more discoverable but adds surface area. Generalizing `--init-extractor-from` to accept a `--init-extractor-source-algo` is smaller. Pick before implementation.
7. **Distillation (Option C) requires teacher serving inside the train loop.** Forward-only inference adds GPU memory and step time. Quantify before committing.
8. **What is success?** Define the bar up front: "warm-started LSTM beats fresh LSTM on the anchored panel by ≥X% at the same step budget" or "warm-started LSTM reaches anchored win-rate Y in N steps fewer than fresh." Without this, the comparison is folklore.

---

## What this plan deliberately does *not* propose

- Building LSTM training from scratch (already exists).
- Switching frameworks (sb3-contrib is fine; the fork is small).
- Changing observation shape, action space, or buffer format (none required).
- Touching the Rust engine (orthogonal — this is pure Python/PyTorch).
- Self-play (retired per CLAUDE.md rule 30; use champion pool instead).

---

## Files most likely to change if this plan is approved

- `code/digimon_gym/agents/training_config.py` — new flag(s).
- `code/digimon_gym/agents/pilot_training.py` — lines ~2918-2943 (loader + surgery generalization), plus optional freeze callback for Option B.
- `code/tests/rl/` — new test for cross-arch encoder transplant invariants.
- `docs/TRAINING_RUNBOOK.md` — section on MLP→LSTM warm-start with the new flag.

Nothing else.
