## Why

The deck-specialist league couples two decisions that should be independent: the
**promotion gate** and the **per-round warm-start source**. Today each round a
deck's specialist is warm-started from its **registry champion**. For a deck that
fails the 0.55 gate ("kept"), the registry entry stays the round-0 generalist, so
the next round restarts *from the generalist again* — discarding the ~500k steps
the kept round just trained.

Confirmed live in the current run: st-1 (Gaia Red) round 3 launches with
`--init-from .../starter_pool_single_v1/final.zip` (the generalist), not st-1's
round-2 checkpoint. So a non-promoting deck never accumulates across rounds — it
gets N *independent* 500k-from-generalist attempts. Consequences:

- **Wasted compute** — every non-promoting round throws away its 500k of training.
- **Deep specialization is untestable on hard decks** — Gaia Red can never train
  more than 500k-deep, so "would more *accumulated* steps help?" is structurally
  unanswerable (we wrongly concluded "more steps won't help" from 500k re-rolls).
- **Contradicts intent** — the driver docstring says specialists are
  "warm-started from their round-(k-1) checkpoint," i.e. accumulation *was* the
  intent; the registry-as-warm-start coupling silently broke it.

Neither standard design discards trained weights this way: AlphaGo Zero keeps a
continuously-trained network and gates only the self-play *generator*; AlphaZero
accumulates always. We want the same — keep the gate for pool hygiene, but stop
throwing away experience.

## What Changes

- **Decouple warm-start from the gate.** The per-round warm-start always uses the
  deck's **own latest round checkpoint** (round 1 → generalist; round k>1 →
  `<deck>/r{k-1}` final), regardless of whether the prior round promoted. Every
  deck compounds its experience across rounds.
- **The gate keeps controlling the pool + registry only** (keep-best, ≥0.55
  head-to-head — unchanged). A regressing round still cannot enter the opponent
  pool. The opponent pool remains the **gated champions** (best-known per deck),
  never in-progress checkpoints.
- **Add a `--warmstart {accumulate,champion}` flag**, default `accumulate`, so the
  old behavior is reproducible for comparison and the default is the fix.
- **Retain accumulated checkpoints** for every deck (even non-promoting) so they
  can be post-hoc anchored-eval'd; the per-deck *deliverable* stays the gated
  champion (proven ≥0.55), but the accumulated specialist is preserved, not lost.
- **Fix the docstring** to describe the actual (now-correct) accumulate behavior.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `deck-specialist-league`: the "Per-deck specialist training scoped and
  warm-started" requirement changes — warm-start is decoupled from promotion and
  always continues the deck's own latest checkpoint (gate governs the pool/registry
  only). Adds the `--warmstart` toggle and the retain-accumulated-checkpoints rule.

## Impact

- **Code:** `code/tools/train_specialist_league.py` — the round-loop warm-start
  resolution (`build_specialist_argv` / round driver), the `--warmstart` flag, the
  docstring. No change to the gate/registry/pool-emission or matchup-matrix logic.
- **No engine changes.** No PyO3, no card scripts, no tensor/action surface.
- **Behavior:** the **base spec for `deck-specialist-league` currently lives in the
  in-progress `add-deck-specialist-league` change** (not yet archived). This delta's
  MODIFIED header (`Per-deck specialist training scoped and warm-started`) is
  verified to match the base requirement exactly, and the ADDED requirement is new,
  so the delta patches cleanly. **Archive ordering constraint:** archive
  `add-deck-specialist-league` first (it establishes the base requirement in
  `openspec/specs/`), then archive this change so the MODIFIED delta applies on top.
- **Compatibility:** `--warmstart champion` reproduces today's behavior exactly;
  default flips to `accumulate`. Existing registries/matrices unaffected.
- **Cost:** none extra — same step budget, the steps just compound instead of
  resetting.
