#!/usr/bin/env python
"""Deck-specialist league driver (add-deck-specialist-league).

Turns one generalist into six per-deck specialists via a round-based PFSP league:

  Round 0 (seed) — the generalist is registered as every deck's specialist, so
                   round 1's frozen pool is "the generalist piloting each deck"
                   (plus each deck's own copy = the mirror).
  Round k        — each deck's specialist trains, scoped to its deck, warm-started
                   from its OWN round-(k-1) checkpoint (round 1 from the generalist),
                   against a FROZEN pool emitted from the registry (latest snapshot
                   of every deck incl. its own). At the round barrier all six
                   snapshot back into the registry.

**Warm-start is decoupled from the promotion gate** (``--warmstart``, default
``accumulate``): a round always continues the deck's own latest round checkpoint,
so a deck that fails the gate ("kept") still compounds its experience across
rounds instead of re-rolling from the generalist. The gate governs only the
opponent pool + registry (keep-best), so the pool stays the *gated* champions
(best-known per deck) — accumulating the warm-start never injects in-progress
weights into anyone's opponent set. Pass ``--warmstart champion`` to reproduce the
legacy gate-coupled behavior (warm-start from the registry champion) for A/B
comparison. Non-promoting decks' accumulated checkpoints are retained on disk for
post-hoc anchored eval; the per-deck *deliverable* stays the gated champion.

Each specialist is the tested ``pilot_training`` CLI run as a subprocess (so deck
validation, reward-profile resolution, callbacks, and the in-training anchored
panel are reused verbatim), launched with ``--opponent league`` so each frozen
opponent plays its OWN deck with its OWN policy (coupled (policy, deck) sampling —
see ``league_opponent.py``). The registry + round pools are durable JSON
(``specialist_registry.py``), so a box restart resumes from the last barrier.

Topology is a dial: ``--topology sequential`` runs a round's six specialists one
at a time on one box (cheap); ``--topology parallel`` launches them concurrently
(fast). Both converge to identical barrier state (registry + snapshots).

Judge specialists with anchored eval (NOT the in-run win rate) + the per-round
matchup matrix — see ``docs/MODEL_EVALUATION.md`` and rule 30.

    python code/tools/train_specialist_league.py \
        --generalist models/starter_pool_v1/final.zip --rounds 3 --dry-run
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional

# Resolve the in-tree packages when run without an editable install.
_CODE = Path(__file__).resolve().parents[1]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))

from digimon_gym.agents.specialist_registry import (  # noqa: E402
    Specialist,
    SpecialistRegistry,
    slugify_deck,
)

# The six worldwide starter decks (canonical archetype keys in deck_library.json).
STARTER_ARCHETYPES: List[str] = [
    "ST-1 Gaia Red",
    "ST-2 Cocytus Blue",
    "ST-3 Heaven's Yellow",
    "ST-4 Giga Green",
    "ST-5 Machine Black",
    "ST-6 Venomous Violet",
]

PILOT_MODULE = "digimon_gym.agents.pilot_training"
#: Active tensor layout for the starter league (the lite-deck-v2 cohort). The
#: generalist + all specialists must share it; the registry gates on it.
DEFAULT_LAYOUT_HASH = "standard_lite_deck_v2"


@dataclass
class LeagueSpec:
    generalist: str
    decks: List[str] = field(default_factory=lambda: list(STARTER_ARCHETYPES))
    save_dir: str = "models/specialists"
    log_dir: str = "runs/specialists"
    registry_path: str = "models/specialists/registry.json"
    pool_dir: str = "models/specialists/_pools"
    rounds: int = 3
    steps_per_round: int = 1_000_000
    lr: float = 1e-4
    tensor_profile: str = "standard_lite_deck_v2"
    reward_profile: str = "starter1_6_flat"
    match_format: str = "bo3"
    layout_hash: str = DEFAULT_LAYOUT_HASH
    keep_last_per_deck: int = 2  # generous retention (avoid the keep_last=3 peak-loss lesson)
    include_mirror: bool = True
    opponent_pool_mode: str = "pfsp"
    topology: str = "sequential"  # "sequential" | "parallel"
    max_parallel: int = 0  # parallel topology: max specialists running at once
                           # (0 = all decks at once). On a 16-core box use 2 with
                           # n_envs=8 each (the loop is update-bound past ~8 cores).
    n_envs: int = 1
    vec_env_backend: str = "subproc"
    eval_freq: int = 50_000
    eval_episodes: int = 20
    batch_size: int = 64       # PPO minibatch; larger => fewer, bigger update steps (faster wall-clock)
    n_epochs: int = 10         # PPO epochs/update; lower => cheaper update, less sample reuse
    anchored_eval_freq: int = 100_000   # in-training anchored panel cadence (the trustworthy signal)
    anchored_eval_games: int = 24       # seat-balanced games/anchor/panel (raise to cut n=24 noise)
    eval_n: int = 0  # >0: after each round barrier, play the seat-balanced matchup matrix
    promote_min_wr: float = 0.0  # >0: gate promotion — a round-r specialist replaces
                                 # round-(r-1) only if it wins >= this share of a
                                 # seat-balanced mirror head-to-head (else keep the
                                 # prior, so a regressing round can't poison the pool)
    warmstart: str = "accumulate"  # "accumulate" (default) | "champion".
                                   # accumulate: each round continues the deck's OWN
                                   # latest round checkpoint (decoupled from the gate),
                                   # so a kept deck compounds experience instead of
                                   # re-rolling from the generalist. champion: legacy —
                                   # warm-start from the registry champion (gate-coupled).
    lr_schedule: str = "constant"  # "constant" | "linear" (within-round LR decay)
    # Weights & Biases (optional; flag-gated). When wandb=True each specialist
    # subprocess logs to W&B with sync_tensorboard, grouped under wandb_group so
    # all 6 decks × N rounds appear together; the per-run name is "<deck>_r<rnd>".
    wandb: bool = False
    wandb_project: str = "digimon-rl"
    wandb_group: str = "specialist-league"
    wandb_mode: str = "online"
    seed: int = 123
    eval_seed: int = 999
    python: str = sys.executable

    def slug(self, deck: str) -> str:
        return slugify_deck(deck)

    def run_dir(self, deck: str, rnd: int) -> Path:
        return Path(self.save_dir) / self.slug(deck) / f"r{rnd}"

    def final_zip(self, deck: str, rnd: int) -> Path:
        return self.run_dir(deck, rnd) / "final.zip"

    def pool_manifest(self, deck: str, rnd: int) -> Path:
        return Path(self.pool_dir) / f"r{rnd}_{self.slug(deck)}.json"

    def matrix_path(self, rnd: int) -> Path:
        return Path(self.save_dir) / "_matrix" / f"r{rnd}.json"


def _resolve_deck_cards(decks: List[str]) -> dict:
    """Resolve each archetype to its first implemented deck's card IDs (for the
    matchup matrix). Lazy + best-effort: returns {} if deck data is unavailable."""
    try:
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool
    except Exception:
        return {}
    out: dict = {}
    for deck in decks:
        try:
            pool = load_generalist_deck_pool(allowed_archetypes={deck})
            names = pool.archetype_names
            if not names:
                continue
            first = sorted(pool.archetypes[names[0]], key=lambda d: d.deck_id)[0]
            out[deck] = list(first.card_ids)
        except Exception:
            continue
    return out


def _candidate_loader():
    """Tolerant LSTM-or-MLP model loader (reuses ``anchored_eval_cli._load_candidate``).
    Lazy so dry-run / CI paths never import torch + SB3."""
    import importlib.util
    cli = Path(__file__).resolve().parent / "anchored_eval_cli.py"
    s = importlib.util.spec_from_file_location("_anchored_cli", cli)
    m = importlib.util.module_from_spec(s)
    sys.modules["_anchored_cli"] = m
    s.loader.exec_module(m)  # type: ignore
    return lambda wp: m._load_candidate(wp, "cpu")


def _policy_loader():
    """Opponent-seat policy loader (load weights as a callable(env)->action)."""
    from digimon_gym.agents.pilot_training import make_agent_opponent_fn
    return lambda wp, algo: make_agent_opponent_fn(wp, algorithm=algo)


def evaluate_round(spec: "LeagueSpec", reg: SpecialistRegistry, rnd: int) -> Optional[Path]:
    """Build + persist the seat-balanced deck matchup matrix for round ``rnd``
    (the league's standing metric — NOT the in-run win rate). Best-effort: skips
    if eval is off or models/decks can't be resolved."""
    if spec.eval_n <= 0:
        return None
    from digimon_gym.agents import league_eval
    load_candidate = _candidate_loader()
    deck_cards = _resolve_deck_cards(spec.decks)
    if not deck_cards:
        print(f"[round {rnd}] eval skipped: no resolvable deck cards")
        return None
    matrix = league_eval.build_matchup_matrix(
        reg, deck_cards, spec.layout_hash,
        model_loader=load_candidate,
        n=spec.eval_n, base_seed=spec.eval_seed, tensor_profile=spec.tensor_profile,
    )
    out = league_eval.write_matchup_matrix(spec.matrix_path(rnd), matrix, rnd)
    # Compact summary: each deck's mean win rate across opponents (excl. greedy).
    for ad, row in matrix.items():
        opps = [v["win_rate"] for k, v in row.items() if k != "greedy"]
        mean = sum(opps) / len(opps) if opps else 0.0
        floor = row.get("greedy", {}).get("win_rate", float("nan"))
        print(f"  [matrix] {spec.slug(ad):<6} mean-vs-field={mean:.2f} vs-greedy={floor:.2f}")
    print(f"[round {rnd}] matchup matrix -> {out}")
    return out


def _generalist_meta(generalist: str, spec: LeagueSpec) -> dict:
    """Read the generalist's ``.meta.json`` if present, else fall back to spec
    defaults so round-0 seeding still works."""
    meta_path = Path(generalist).with_suffix(".meta.json")
    if meta_path.is_file():
        try:
            return json.loads(meta_path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {
        "algorithm": "mlp",
        "observation_profile": spec.tensor_profile,
        "tensor_layout_hash": spec.layout_hash,
    }


def seed_registry(spec: LeagueSpec, created_at: str = "") -> SpecialistRegistry:
    """Round 0: register the generalist as every deck's specialist so round 1 has
    a non-empty pool (generalist-as-each-deck + the mirror)."""
    reg = SpecialistRegistry.load(spec.registry_path)
    meta = _generalist_meta(spec.generalist, spec)
    # Force the active layout so the round pools are emittable even if the
    # generalist meta predates layout tagging.
    meta = {**meta, "tensor_layout_hash": spec.layout_hash}
    for deck in spec.decks:
        if deck in reg.current:
            continue  # resume-safe: don't clobber an existing seed
        reg.set_current(
            Specialist.from_meta(
                deck=deck,
                weights_path=spec.generalist,
                meta=meta,
                round=0,
                source="generalist-seed",
                created_at=created_at,
            )
        )
    reg.save()
    return reg


def _latest_round_checkpoint(spec: "LeagueSpec", deck: str, rnd: int) -> Optional[Path]:
    """The deck's round-``rnd`` warm-start checkpoint: prefer ``final.zip``, else the
    highest-step ``*.zip`` in the round dir. ``None`` if the round produced nothing.

    There is no league-level disk pruning — each round writes to a distinct
    ``<deck>/r{rnd}/`` dir and ``final.zip`` is never deleted — so a kept deck's
    round chain stays intact for the next round's warm-start (and for post-hoc
    anchored eval)."""
    final = spec.final_zip(deck, rnd)
    if final.is_file():
        return final
    run_dir = spec.run_dir(deck, rnd)
    if run_dir.is_dir():
        def _step(p: Path) -> int:
            digits = "".join(ch for ch in p.stem if ch.isdigit())
            return int(digits) if digits else -1
        ckpts = sorted(run_dir.glob("*.zip"), key=_step)
        if ckpts:
            return ckpts[-1]
    return None


def _resolve_warmstart(
    spec: "LeagueSpec", reg: SpecialistRegistry, deck: str, rnd: int
) -> str:
    """Resolve the ``--init-from`` checkpoint for ``deck``'s round ``rnd``, decoupled
    from the promotion gate.

    ``accumulate`` (default): continue the deck's OWN training chain — round 1 →
    the generalist seed (no prior round); round k>1 → the deck's r{k-1} checkpoint
    on disk. Falls back to the registry champion (with a warning) if that
    checkpoint can't be found (e.g. a ``--from-round`` resume), so accumulation is
    best-effort and never aborts.

    ``champion`` (legacy): the registry champion for the deck — gate-coupled, the
    pre-decoupling behavior, kept for A/B comparison."""
    if spec.warmstart == "champion":
        return str(reg.get(deck).weights_path)
    # accumulate
    if rnd <= 1:
        return spec.generalist
    prior = _latest_round_checkpoint(spec, deck, rnd - 1)
    if prior is not None:
        return str(prior)
    fallback = str(reg.get(deck).weights_path)
    print(f"[warn] {spec.slug(deck)} r{rnd}: no r{rnd - 1} checkpoint on disk; "
          f"falling back to registry champion {fallback} "
          f"(accumulation is best-effort)", flush=True)
    return fallback


def build_specialist_argv(
    spec: LeagueSpec, deck: str, rnd: int, init_from: str, pool_manifest: Path
) -> List[str]:
    """Argv for one specialist's round: deck-scoped, warm-started, vs the frozen
    league pool. ``--opponent league`` pins the agent to ``deck`` and couples each
    opponent's (policy, deck) from the manifest."""
    name = f"{spec.slug(deck)}_r{rnd}"
    argv = [
        spec.python, "-m", PILOT_MODULE,
        "--opponent", "league",
        "--league-pool-manifest", str(pool_manifest),
        "--archetypes", deck,                      # pins agent deck (single archetype)
        "--init-from", init_from,
        "--timesteps", str(spec.steps_per_round),
        "--lr", repr(spec.lr),
        "--match-format", spec.match_format,
        "--eval-freq", str(spec.eval_freq),
        "--eval-episodes", str(spec.eval_episodes),
        "--anchored-eval-freq", str(spec.anchored_eval_freq),
        "--anchored-eval-games", str(spec.anchored_eval_games),
        "--curriculum-seed", str(spec.seed + rnd),
        "--eval-seed", str(spec.eval_seed),
        "--save-dir", str(spec.run_dir(deck, rnd).parent),
        "--log-dir", f"{spec.log_dir}/{name}",
        "--set", f"run_name=r{rnd}",
        "--set", f"tensor_profile={spec.tensor_profile}",
        "--set", f"reward_profile_override={spec.reward_profile}",
        "--set", f"opponent_pool_mode={spec.opponent_pool_mode}",
        "--set", f"batch_size={spec.batch_size}",
        "--set", f"n_epochs={spec.n_epochs}",
        "--lr-schedule", spec.lr_schedule,
    ]
    if spec.wandb:
        argv += [
            "--wandb",
            "--wandb-project", spec.wandb_project,
            "--wandb-group", spec.wandb_group,
            "--wandb-run-name", name,         # "<deck-slug>_r<rnd>"
            "--wandb-mode", spec.wandb_mode,
        ]
    if spec.n_envs > 1:
        argv += [
            "--set", f"n_envs={spec.n_envs}",
            "--set", f"vec_env_backend={spec.vec_env_backend}",
        ]
    return argv


def _run(argv: List[str], cwd: Path) -> None:
    env = dict(os.environ)
    env["PYTHONPATH"] = str((cwd / "code").resolve()) + os.pathsep + env.get("PYTHONPATH", "")
    print("\n$ " + " ".join(argv), flush=True)
    subprocess.run(argv, cwd=str(cwd), env=env, check=True)


def _run_parallel(argvs: List[List[str]], cwd: Path, max_parallel: int = 0) -> None:
    """Run specialist subprocesses with at most ``max_parallel`` concurrent
    (0 = all at once). Bounds oversubscription on a fixed-core box — e.g. 2 with
    ``--n-envs 8`` saturates 16 cores without contention (update-bound past ~8)."""
    env = dict(os.environ)
    env["PYTHONPATH"] = str((cwd / "code").resolve()) + os.pathsep + env.get("PYTHONPATH", "")
    cap = max_parallel if (max_parallel and max_parallel > 0) else len(argvs)
    pending = list(argvs)
    running: List[subprocess.Popen] = []
    failed: List = []

    def _fill() -> None:
        while pending and len(running) < cap:
            argv = pending.pop(0)
            print("\n$ (bg) " + " ".join(argv), flush=True)
            running.append(subprocess.Popen(argv, cwd=str(cwd), env=env))

    _fill()
    while running:
        proc = running.pop(0)          # wait on the oldest, then refill the slot
        if proc.wait() != 0:
            failed.append(proc.args)
        _fill()
    if failed:
        raise RuntimeError(f"{len(failed)} specialist run(s) failed this round")


def run_round(spec: LeagueSpec, reg: SpecialistRegistry, rnd: int, cwd: Path,
              dry_run: bool = False, created_at: str = "") -> None:
    """Train every deck's specialist for round ``rnd`` against frozen pools, then
    snapshot all six back into the registry (the round barrier)."""
    argvs: List[List[str]] = []
    for deck in spec.decks:
        pool_path = reg.write_round_pool(
            spec.pool_manifest(deck, rnd),
            for_deck=deck,
            tensor_layout_hash=spec.layout_hash,
            keep_last_per_deck=spec.keep_last_per_deck,
            include_mirror=spec.include_mirror,
        )
        # Warm-start is decoupled from the promotion gate: under "accumulate"
        # (default) each round continues the deck's OWN latest checkpoint (round 1
        # → the generalist), so a kept deck compounds experience instead of
        # re-rolling from the registry champion. "champion" reproduces the legacy
        # gate-coupled behavior. The pool/registry emission above is untouched.
        init_from = _resolve_warmstart(spec, reg, deck, rnd)
        argvs.append(build_specialist_argv(spec, deck, rnd, init_from, pool_path))

    if dry_run:
        for argv in argvs:
            print(" ".join(argv))
        return

    if spec.topology == "parallel":
        _run_parallel(argvs, cwd, max_parallel=spec.max_parallel)
    else:
        for argv in argvs:
            _run(argv, cwd)

    _barrier(spec, reg, rnd, created_at)
    evaluate_round(spec, reg, rnd)


def _barrier(spec: LeagueSpec, reg: SpecialistRegistry, rnd: int,
             created_at: str = "") -> None:
    """Round barrier: snapshot each deck's round-``rnd`` specialist into the
    registry. When ``promote_min_wr > 0`` each deck is **gated** on a seat-balanced
    mirror head-to-head vs its prior checkpoint — a round that doesn't clear the
    bar is rejected (the prior stays current), so a regressing round can't poison
    the pool. Promotion uses the anchored frame only (never the in-run win rate)."""
    meta = {**_generalist_meta(spec.generalist, spec),
            "tensor_layout_hash": spec.layout_hash}
    gate = spec.promote_min_wr > 0.0
    prev = {deck: reg.get(deck) for deck in spec.decks}  # round-(r-1), pre-update
    load_candidate = policy_loader = None
    deck_cards: dict = {}
    if gate:
        load_candidate = _candidate_loader()
        policy_loader = _policy_loader()
        deck_cards = _resolve_deck_cards(spec.decks)

    promoted = kept = 0
    for deck in spec.decks:
        final = spec.final_zip(deck, rnd)
        if not final.is_file():
            raise RuntimeError(f"specialist output missing at barrier: {final}")
        cand = Specialist.from_meta(
            deck=deck, weights_path=str(final), meta=meta,
            round=rnd, source=f"league-r{rnd}", created_at=created_at,
        )
        if gate and deck_cards.get(deck):
            from digimon_gym.agents import league_eval
            rec = league_eval.anchored_head_to_head(
                load_candidate(str(final)),
                policy_loader(prev[deck].weights_path, prev[deck].algorithm),
                deck_cards[deck], n=(spec.eval_n or 24),
                base_seed=spec.eval_seed, tensor_profile=spec.tensor_profile,
            )
            if league_eval.should_promote(rec, spec.promote_min_wr):
                reg.set_current(cand); promoted += 1
                print(f"  [promote] {spec.slug(deck):<6} r{rnd} vs r{prev[deck].round} "
                      f"WR={rec.win_rate:.2f} >= {spec.promote_min_wr:.2f}")
            else:
                kept += 1
                print(f"  [keep]    {spec.slug(deck):<6} r{rnd} vs r{prev[deck].round} "
                      f"WR={rec.win_rate:.2f} <  {spec.promote_min_wr:.2f} "
                      f"(kept r{prev[deck].round})")
        else:
            reg.set_current(cand); promoted += 1
    reg.save()
    print(f"[round {rnd}] barrier: {promoted} promoted, {kept} kept "
          f"-> {spec.registry_path}")


def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    p.add_argument("--generalist", required=True,
                   help="Warm-start seed (generalist final.zip).")
    p.add_argument("--decks", default=None,
                   help="Comma-separated archetype keys (default: the six starters).")
    p.add_argument("--rounds", type=int, default=3)
    p.add_argument("--steps-per-round", type=int, default=1_000_000)
    p.add_argument("--lr", type=float, default=1e-4)
    p.add_argument("--save-dir", default="models/specialists")
    p.add_argument("--log-dir", default="runs/specialists")
    p.add_argument("--registry", default=None, help="Registry JSON (default: <save-dir>/registry.json).")
    p.add_argument("--keep-last-per-deck", type=int, default=2)
    p.add_argument("--no-mirror", action="store_true", help="Exclude own-deck snapshots from pools.")
    p.add_argument("--topology", choices=["sequential", "parallel"], default="sequential")
    p.add_argument("--max-parallel", type=int, default=0,
                   help="parallel topology: max specialists running at once "
                        "(0 = all). On a 16-core box use 2 with --n-envs 8.")
    p.add_argument("--n-envs", type=int, default=1)
    p.add_argument("--vec-env-backend", default="subproc")
    p.add_argument("--tensor-profile", default="standard_lite_deck_v2")
    p.add_argument("--reward-profile", default="starter1_6_flat")
    p.add_argument("--match-format", default="bo3", choices=["bo3", "single"])
    p.add_argument("--layout-hash", default=DEFAULT_LAYOUT_HASH)
    p.add_argument("--opponent-pool-mode", default="pfsp", choices=["pfsp", "uniform"])
    p.add_argument("--eval-freq", type=int, default=50_000)
    p.add_argument("--eval-episodes", type=int, default=20)
    p.add_argument("--batch-size", type=int, default=64,
                   help="PPO minibatch size per specialist. Larger (e.g. 256/512) "
                        "means fewer, bigger gradient steps -> faster update phase.")
    p.add_argument("--n-epochs", type=int, default=10,
                   help="PPO epochs per update. Lower trades sample reuse for a "
                        "cheaper update phase.")
    p.add_argument("--anchored-eval-freq", type=int, default=100_000,
                   help="Steps between in-training anchored panels (the trustworthy "
                        "signal vs greedy + registered champions; rule 30).")
    p.add_argument("--anchored-eval-games", type=int, default=24,
                   help="Seat-balanced games per anchor per panel. Raise to cut the "
                        "n=24 noise on the anchored signal.")
    p.add_argument("--eval-n", type=int, default=0,
                   help="Seat-balanced games per matchup-matrix cell after each "
                        "round barrier (0 = skip). 24 gives ~+/-20%% CI per cell.")
    p.add_argument("--promote-min-wr", type=float, default=0.0,
                   help="Gate promotion: a round-r specialist replaces its prior "
                        "only if it wins >= this share of a seat-balanced mirror "
                        "head-to-head (0 = always promote). 0.55 is the registry rule.")
    p.add_argument("--warmstart", choices=["accumulate", "champion"], default="accumulate",
                   help="Per-round warm-start source, decoupled from the gate. "
                        "accumulate (default): continue the deck's OWN latest round "
                        "checkpoint (round 1 from the generalist), so a kept deck "
                        "compounds experience. champion: legacy — warm-start from the "
                        "registry champion (gate-coupled); pass this to reproduce the "
                        "pre-decoupling behavior for A/B comparison.")
    p.add_argument("--lr-schedule", choices=["constant", "linear"], default="constant",
                   help="Within-round LR schedule passed to each specialist run "
                        "(linear = decay to 0 over the round, avoids constant-LR drift).")
    p.add_argument("--wandb", action="store_true",
                   help="Log every specialist to Weights & Biases (sync_tensorboard), "
                        "grouped so all decks/rounds appear together. Each subprocess "
                        "reads WANDB_API_KEY from the environment.")
    p.add_argument("--wandb-project", default="digimon-rl", help="W&B project (default: digimon-rl).")
    p.add_argument("--wandb-group", default="specialist-league",
                   help="W&B group tying all specialists/rounds together "
                        "(default: specialist-league).")
    p.add_argument("--wandb-mode", choices=["online", "offline", "disabled"], default="online",
                   help="W&B mode for the specialist runs (default: online).")
    p.add_argument("--seed", type=int, default=123)
    p.add_argument("--eval-seed", type=int, default=999)
    p.add_argument("--from-round", type=int, default=1,
                   help="Resume: first round to train (registry must already hold "
                        "the prior round). Default 1 (after the round-0 seed).")
    p.add_argument("--cwd", default=None)
    p.add_argument("--dry-run", action="store_true",
                   help="Print each round's commands and exit without training.")
    args = p.parse_args()

    decks = ([d.strip() for d in args.decks.split(",") if d.strip()]
             if args.decks else list(STARTER_ARCHETYPES))
    save_dir = args.save_dir
    spec = LeagueSpec(
        generalist=args.generalist,
        decks=decks,
        save_dir=save_dir,
        log_dir=args.log_dir,
        registry_path=args.registry or str(Path(save_dir) / "registry.json"),
        pool_dir=str(Path(save_dir) / "_pools"),
        rounds=args.rounds,
        steps_per_round=args.steps_per_round,
        lr=args.lr,
        keep_last_per_deck=args.keep_last_per_deck,
        include_mirror=not args.no_mirror,
        topology=args.topology,
        max_parallel=args.max_parallel,
        n_envs=args.n_envs,
        vec_env_backend=args.vec_env_backend,
        tensor_profile=args.tensor_profile,
        reward_profile=args.reward_profile,
        match_format=args.match_format,
        layout_hash=args.layout_hash,
        opponent_pool_mode=args.opponent_pool_mode,
        eval_freq=args.eval_freq,
        eval_episodes=args.eval_episodes,
        batch_size=args.batch_size,
        n_epochs=args.n_epochs,
        anchored_eval_freq=args.anchored_eval_freq,
        anchored_eval_games=args.anchored_eval_games,
        eval_n=args.eval_n,
        promote_min_wr=args.promote_min_wr,
        warmstart=args.warmstart,
        lr_schedule=args.lr_schedule,
        wandb=args.wandb,
        wandb_project=args.wandb_project,
        wandb_group=args.wandb_group,
        wandb_mode=args.wandb_mode,
        seed=args.seed,
        eval_seed=args.eval_seed,
    )
    cwd = Path(args.cwd).resolve() if args.cwd else Path.cwd()

    if not args.dry_run and not Path(spec.generalist).is_file():
        p.error(f"generalist not found: {spec.generalist}")

    reg = seed_registry(spec)
    print(f"[seed] round 0: generalist registered for {len(spec.decks)} decks "
          f"-> {spec.registry_path}")

    for rnd in range(args.from_round, args.rounds + 1):
        print(f"\n=== Round {rnd}/{args.rounds} "
              f"({spec.topology}, {spec.steps_per_round:,} steps/specialist) ===")
        run_round(spec, reg, rnd, cwd, dry_run=args.dry_run)

    if args.dry_run:
        return
    print(f"\nDone. {args.rounds}-round league complete. Registry: {spec.registry_path}")
    print("Judge with anchored eval + the matchup matrix (NOT the in-run win rate).")


if __name__ == "__main__":
    main()
