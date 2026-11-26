"""End-to-end 5-stage orchestrator for the `ex13-small3` Hybrid Search <-> LSTM PPO campaign.

Stages (idempotent, checkpointed in `<run_dir>/campaign_state.json`):
  0. Mixed teacher bootstrap (`greedy` + `development_v1` -> `Gen0` `[256, 256]` MLP)
  1. Macro-Substep MCTS search baseline (`Gens 1-3`, `--macro-substeps 16`)
  2. Recurrent sequence distillation (`Gen1-3` shards -> `lstm_distilled_r1`)
  3. KL-anchored clean-reward LSTM PPO (`terminal_strategic_v1` + PFSP vs `Gen1-3` ONNX -> `lstm_ppo_r1`)
  4. Paired Stage-4 arms:
       - Hybrid arm (`Gens 4-6`): 50% self-play + 50% `{lstm_ppo_r1 (policy_lstm, w=2), Gen2 (w=1), Gen3 (w=1)}`
       - Control arm (`Gens 4c-6c`): 50% self-play + 50% `{Gen1 (w=1), Gen2 (w=1), Gen3 (w=1)}`
  5. Held-out anchored evaluation (`heldout_snapshot.json` policy-only matrix + `heldout_decks.json` `320/4`
     searched head-to-heads + `search_exam` on `ex13-examon-training-before-wingdramon.json`)
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable, Optional

import numpy as np

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT / "code") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "code"))


@dataclass
class CampaignConfig:
    preset: str = "full"
    seed: int = 20261126
    procs: int = 2
    net_arch: tuple[int, ...] = (256, 256)
    features_dim: int = 256
    lstm_hidden_size: int = 256
    bootstrap_rounds: int = 64        # 64 * 3 = 192 teacher games
    search_generations_stage1: int = 3
    search_generations_stage4: int = 3
    search_games: int = 450
    chunk_games: int = 18
    sims: int = 320
    worlds: int = 4
    eval_batch_size: int = 4
    macro_substeps: int = 16
    reduced_sims: int = 64
    full_search_fraction: float = 0.25
    cheap_sims: int = 64
    search_epochs: int = 4
    search_batch_size: int = 256
    holdout_fraction: float = 0.15
    distill_epochs: int = 6
    distill_seq_batch_size: int = 16
    ppo_timesteps: int = 500_000
    ppo_n_envs: int = 2
    ppo_n_steps: int = 1024
    ppo_epochs: int = 4
    ppo_lr: float = 1e-4
    kl_coef: float = 0.03
    ppo_eval_freq: int = 100_000
    ppo_eval_episodes: int = 24
    ppo_checkpoint_every: int = 100_000
    gate_games_per_archetype: int = 20
    policy_eval_games_per_archetype: int = 120  # 120 * 3 = 360 games
    searched_eval_games: int = 180              # 90 seat-balanced pairs
    exam_seeds: int = 8


def make_config(args: argparse.Namespace) -> CampaignConfig:
    if args.preset == "smoke":
        cfg = CampaignConfig(
            preset="smoke",
            seed=args.seed,
            procs=min(args.procs, 2),
            net_arch=(64, 64),
            features_dim=64,
            lstm_hidden_size=64,
            bootstrap_rounds=8,          # 8 * 3 = 24 games (minimum valid 8-round block)
            search_generations_stage1=2,
            search_generations_stage4=1,
            search_games=4,
            chunk_games=2,
            sims=16,
            worlds=2,
            eval_batch_size=2,
            macro_substeps=8,
            reduced_sims=8,
            full_search_fraction=0.5,
            cheap_sims=8,
            search_epochs=1,
            search_batch_size=64,
            holdout_fraction=0.25,
            distill_epochs=2,
            distill_seq_batch_size=4,
            ppo_timesteps=256,
            ppo_n_envs=1,
            ppo_n_steps=64,
            ppo_epochs=2,
            ppo_lr=1e-4,
            kl_coef=0.03,
            ppo_eval_freq=128,
            ppo_eval_episodes=4,
            ppo_checkpoint_every=0,
            gate_games_per_archetype=2,
            policy_eval_games_per_archetype=4,
            searched_eval_games=4,
            exam_seeds=2,
        )
    else:
        cfg = CampaignConfig(preset="full", seed=args.seed, procs=args.procs)

    if args.bootstrap_rounds is not None:
        cfg.bootstrap_rounds = args.bootstrap_rounds
    if args.search_games is not None:
        cfg.search_games = args.search_games
    if args.chunk_games is not None:
        cfg.chunk_games = args.chunk_games
    if args.ppo_timesteps is not None:
        cfg.ppo_timesteps = args.ppo_timesteps
    if args.policy_eval_games_per_archetype is not None:
        cfg.policy_eval_games_per_archetype = args.policy_eval_games_per_archetype
    if args.searched_eval_games is not None:
        cfg.searched_eval_games = args.searched_eval_games
    return cfg


def atomic_write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + f".{os.getpid()}.tmp")
    tmp.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def update_progress(run_dir: Path, stage: str, status: str, **extra: Any) -> None:
    payload = {
        "updated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "stage": stage,
        "status": status,
        **extra,
    }
    atomic_write_json(run_dir / "campaign_progress.json", payload)
    print(f"[campaign] {stage}: {status} {extra}", flush=True)


def load_campaign_state(run_dir: Path) -> dict:
    p = run_dir / "campaign_state.json"
    if p.exists():
        return json.loads(p.read_text(encoding="utf-8"))
    return {"completed_stages": [], "artifacts": {}}


def save_campaign_state(run_dir: Path, state: dict) -> None:
    atomic_write_json(run_dir / "campaign_state.json", state)


def write_mlp_meta_sidecar(zip_path: Path, profile_id: str = "standard_lite_v2") -> Path:
    from digimon_engine import EMBEDDING_DIM, REGISTRY_CAPACITY
    from digimon_gym.agents.pilot_training import current_action_space_structure
    from model_contract import current_contract

    contract = current_contract(profile_id)
    meta = {
        **contract,
        "algorithm": "mlp",
        "action_space_structure": current_action_space_structure(),
        "card_registry_capacity": int(REGISTRY_CAPACITY),
        "embedding_dim": int(EMBEDDING_DIM),
        "value_semantics": "terminal_outcome",
    }
    meta_path = zip_path.with_suffix(".meta.json")
    atomic_write_json(meta_path, meta)
    return meta_path


class OnnxAgentAdapter:
    """Fast 1-thread ONNX Runtime adapter supporting both P1 (predict) and P2 (opponent_fn)."""

    def __init__(self, onnx_path: Path, *, is_lstm: bool = False, skip_forced: bool = False):
        import onnxruntime as ort
        from model_contract import validate_session

        so = ort.SessionOptions()
        so.intra_op_num_threads = 1
        so.inter_op_num_threads = 1
        self.session = ort.InferenceSession(
            str(onnx_path), sess_options=so, providers=["CPUExecutionProvider"]
        )
        self.contract = validate_session(self.session)
        self.is_lstm = is_lstm
        self.digimon_skip_forced_steps = bool(skip_forced)
        self.hidden_size = 256
        if is_lstm:
            for inp in self.session.get_inputs():
                if inp.name == "h_in":
                    self.hidden_size = int(inp.shape[-1])
                    break
        self._opp_h: Optional[np.ndarray] = None
        self._opp_c: Optional[np.ndarray] = None

    def predict(
        self,
        observation: np.ndarray,
        state: Optional[tuple[np.ndarray, np.ndarray]] = None,
        episode_start: Optional[np.ndarray] = None,
        deterministic: bool = True,
        action_masks: Optional[np.ndarray] = None,
    ):
        obs_batch = np.asarray(observation, dtype=np.float32).reshape(1, -1)
        mask = np.asarray(action_masks)
        if not self.is_lstm:
            logits = self.session.run(["logits"], {"obs": obs_batch})[0][0]
            masked = np.where(mask > 0.5, logits, -1e9)
            return int(np.argmax(masked)), None
        reset = state is None or (
            episode_start is not None and bool(np.asarray(episode_start).reshape(-1)[0])
        )
        if reset:
            h = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
            c = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
        else:
            h, c = state
        logits, h_out, c_out = self.session.run(
            ["logits", "h_out", "c_out"],
            {"obs": obs_batch, "h_in": h, "c_in": c},
        )
        masked = np.where(mask > 0.5, logits[0], -1e9)
        return int(np.argmax(masked)), (h_out, c_out)

    def as_opponent_fn(self) -> Callable:
        def _opp(env) -> int:
            mask = np.asarray(env.action_mask())
            if self.digimon_skip_forced_steps:
                valid = np.flatnonzero(mask > 0.5)
                if len(valid) == 1:
                    return int(valid[0])
            obs = np.asarray(env.runner.get_board_tensor(2), dtype=np.float32).reshape(1, -1)
            if not self.is_lstm:
                logits = self.session.run(["logits"], {"obs": obs})[0][0]
                masked = np.where(mask > 0.5, logits, -1e9)
                return int(np.argmax(masked))
            if self._opp_h is None or self._opp_c is None:
                self._opp_h = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
                self._opp_c = np.zeros((1, 1, self.hidden_size), dtype=np.float32)
            logits, self._opp_h, self._opp_c = self.session.run(
                ["logits", "h_out", "c_out"],
                {"obs": obs, "h_in": self._opp_h, "c_in": self._opp_c},
            )
            masked = np.where(mask > 0.5, logits[0], -1e9)
            return int(np.argmax(masked))

        def _reset():
            self._opp_h = None
            self._opp_c = None

        _opp.reset_state = _reset  # type: ignore[attr-defined]
        return _opp


def development_v1_opponent(env) -> int:
    decision = json.loads(env.runner.teacher_decision("development_v1"))
    return int(decision["action"])


def paired_bootstrap_ci(
    outcomes: list[float], seed: int = 2026, resamples: int = 10_000
) -> dict[str, float]:
    """Bootstrap 95% CI resampling seat-balanced (even, odd) game pairs."""
    arr = np.asarray(outcomes, dtype=np.float64)
    n = len(arr)
    if n == 0:
        return {"mean": 0.0, "ci95_low": 0.0, "ci95_high": 0.0}
    if n >= 4 and n % 2 == 0:
        pair_means = 0.5 * (arr[0::2] + arr[1::2])
    else:
        pair_means = arr
    rng = np.random.default_rng(seed)
    idx = rng.integers(0, len(pair_means), size=(resamples, len(pair_means)))
    means = pair_means[idx].mean(axis=1)
    return {
        "mean": float(arr.mean()),
        "ci95_low": float(np.percentile(means, 2.5)),
        "ci95_high": float(np.percentile(means, 97.5)),
    }


def evaluate_policy_matchup_detailed(
    agent_adapter,
    opponent_fn: Callable,
    deck_pool,
    games_per_archetype: int,
    base_seed: int,
    tensor_profile: str = "standard_lite_v2",
) -> dict:
    """Seat-balanced policy-only evaluation with per-archetype W-L-D, paired CI, and strategic telemetry."""
    from digimon_gym.agents.anchored_eval import _seat_balanced_seed, balanced_deck_pairs
    from digimon_gym.agents.pilot_training import (
        AutoAdvanceForcedWrapper,
        OpponentWrapper,
        _unwrap_to_digimon_env,
    )
    from digimon_gym.digimon_gym import DigimonEnv

    wins = losses = draws = 0
    game_scores: list[float] = []
    by_arch: dict[str, dict[str, int]] = {}
    by_matchup: dict[str, dict[str, int]] = {}
    turns: list[int] = []
    p1_digis: list[int] = []
    p1_dnas: list[int] = []
    p2_digis: list[int] = []

    pairs = list(balanced_deck_pairs(deck_pool, games_per_archetype, base_seed))
    for i, (first, second) in enumerate(pairs):
        seed = _seat_balanced_seed(base_seed, i)
        inner = DigimonEnv(
            deck1=first.card_ids,
            deck2=second.card_ids,
            tensor_profile=tensor_profile,
            record_actions=False,
        )
        env: Any = OpponentWrapper(inner, opponent_fn=opponent_fn)
        if getattr(agent_adapter, "digimon_skip_forced_steps", False):
            env = AutoAdvanceForcedWrapper(env)
        obs, _ = env.reset(seed=seed)
        state = None
        episode_start = np.array([True])
        steps = 0
        done = False
        while not done and steps < 4000:
            mask = _unwrap_to_digimon_env(env).action_mask()
            action, state = agent_adapter.predict(
                obs,
                state=state,
                episode_start=episode_start,
                deterministic=True,
                action_masks=mask,
            )
            obs, _r, terminated, truncated, _ = env.step(int(action))
            episode_start = np.array([False])
            steps += 1
            done = terminated or truncated
        if not inner.is_game_over:
            raise RuntimeError(f"Eval game hit step cap (seed {seed})")
        rl_state = inner._rl_state()
        turns.append(int(rl_state.get("turn_count", 0)))
        p1_digis.append(int(rl_state.get("p1_digivolutions", 0)))
        p1_dnas.append(int(rl_state.get("p1_dna_digivolutions", 0)))
        p2_digis.append(int(rl_state.get("p2_digivolutions", 0)))
        winner = inner.winner_id

        arch_rec = by_arch.setdefault(first.archetype_name, {"wins": 0, "losses": 0, "draws": 0, "games": 0})
        m_key = f"{first.archetype_name} vs {second.archetype_name}"
        m_rec = by_matchup.setdefault(m_key, {"wins": 0, "losses": 0, "draws": 0, "games": 0})
        arch_rec["games"] += 1
        m_rec["games"] += 1
        if winner == 1:
            wins += 1
            arch_rec["wins"] += 1
            m_rec["wins"] += 1
            game_scores.append(1.0)
        elif winner == 2:
            losses += 1
            arch_rec["losses"] += 1
            m_rec["losses"] += 1
            game_scores.append(0.0)
        else:
            draws += 1
            arch_rec["draws"] += 1
            m_rec["draws"] += 1
            game_scores.append(0.5)
        env.close()

    total = wins + losses + draws
    ci = paired_bootstrap_ci(game_scores, seed=base_seed + 17)
    for rec in by_arch.values():
        rec["win_rate"] = rec["wins"] / max(1, rec["games"])
    for rec in by_matchup.values():
        rec["win_rate"] = rec["wins"] / max(1, rec["games"])

    return {
        "games": total,
        "wins": wins,
        "losses": losses,
        "draws": draws,
        "win_rate": wins / max(1, total),
        "score_rate_with_half_draws": ci["mean"],
        "paired_bootstrap_ci95": [ci["ci95_low"], ci["ci95_high"]],
        "by_archetype": by_arch,
        "by_matchup": by_matchup,
        "telemetry": {
            "mean_turn_count": float(np.mean(turns)) if turns else 0.0,
            "median_turn_count": float(np.median(turns)) if turns else 0.0,
            "p90_turn_count": float(np.percentile(turns, 90)) if turns else 0.0,
            "mean_agent_digivolves": float(np.mean(p1_digis)) if p1_digis else 0.0,
            "mean_agent_dna_digivolves": float(np.mean(p1_dnas)) if p1_dnas else 0.0,
            "mean_opponent_digivolves": float(np.mean(p2_digis)) if p2_digis else 0.0,
        },
    }


def run_searched_head_to_head(
    *,
    cli_bin: Path,
    out_dir: Path,
    candidate_onnx: Path,
    opponent_id: str,
    opponent_onnx: Path,
    opponent_mode: str,
    heldout_decks_path: Path,
    games: int,
    procs: int,
    sims: int,
    worlds: int,
    eval_batch_size: int,
    macro_substeps: int,
    reduced_sims: int = 0,
    seed: int,
) -> dict:
    """Run paired seat-balanced 320/4 searched head-to-head games natively via digimon-engine-cli."""
    from tools.run_selfplay_generation import run_selfplay_stage
    from tools.search_opponents import SCHEMA as OPP_SCHEMA, checkpoint_entry

    if games % 2 != 0:
        raise ValueError("searched head-to-head games must be even for paired play/draw balance")

    if (out_dir / "summary.json").exists():
        return json.loads((out_dir / "summary.json").read_text(encoding="utf-8"))

    out_dir.mkdir(parents=True, exist_ok=True)
    pool_path = out_dir / "eval_opponent_pool.json"
    pool_doc = {
        "schema": OPP_SCHEMA,
        "selfplay_probability": 0.0,
        "opponents": [
            checkpoint_entry(opponent_id, opponent_onnx, weight=1.0, mode=opponent_mode)
        ],
    }
    atomic_write_json(pool_path, pool_doc)

    budget_flags = ["--root-noise-epsilon", "0.0"]
    if reduced_sims > 0:
        budget_flags += ["--reduced-sims", str(reduced_sims)]
    if macro_substeps > 0:
        budget_flags += ["--macro-substeps", str(macro_substeps)]

    chunk = max(2, min(16, games // max(1, procs)))
    if chunk % 2 != 0:
        chunk += 1

    t0 = time.monotonic()
    proc_dirs = run_selfplay_stage(
        cli_bin=str(cli_bin),
        gen_dir=out_dir,
        model_onnx=str(candidate_onnx),
        uniform=False,
        decks=heldout_decks_path,
        games=games,
        procs=procs,
        sims=sims,
        worlds=worlds,
        mode="pimc",
        seed_base=seed,
        temperature_decisions=0,
        decision_cap=600,
        shard_rows=100_000,
        cards_json=REPO_ROOT / "data" / "cards.json",
        pool="implemented",
        eval_batch_size=eval_batch_size,
        scheduling="dynamic",
        chunk_games=chunk,
        resume=(out_dir / "schedule.json").exists(),
        budget_flags=budget_flags,
        opponent_pool=pool_path,
    )
    elapsed = time.monotonic() - t0

    records = []
    deck_names = None
    game_decks = []
    truncated_total = 0
    for d in sorted(proc_dirs):
        manifest = json.loads((d / "manifest.json").read_text(encoding="utf-8"))
        deck_names = manifest["deck_names"]
        truncated_total += int(manifest["truncated_games"])
        for g_rec, pair in zip(manifest["opponent_games"], manifest["game_decks"]):
            records.append((g_rec["game_id"], g_rec, pair))

    records.sort(key=lambda x: x[0])
    wins = losses = draws = 0
    scores: list[float] = []
    by_arch: dict[str, dict[str, int]] = {}
    assert deck_names is not None
    for _, rec, pair in records:
        learner = int(rec["learner_seat"])
        winner = rec["winner"]
        arch = deck_names[pair[learner]].split(":sha256:", 1)[0]
        arch_stat = by_arch.setdefault(arch, {"wins": 0, "losses": 0, "draws": 0, "games": 0})
        arch_stat["games"] += 1
        if rec["truncated"] or winner is None:
            draws += 1
            arch_stat["draws"] += 1
            scores.append(0.5)
        elif int(winner) == learner:
            wins += 1
            arch_stat["wins"] += 1
            scores.append(1.0)
        else:
            losses += 1
            arch_stat["losses"] += 1
            scores.append(0.0)

    for st in by_arch.values():
        st["win_rate"] = st["wins"] / max(1, st["games"])
    ci = paired_bootstrap_ci(scores, seed=seed + 31)
    summary = {
        "candidate_onnx": str(candidate_onnx),
        "opponent_id": opponent_id,
        "opponent_onnx": str(opponent_onnx),
        "opponent_mode": opponent_mode,
        "sims": sims,
        "worlds": worlds,
        "macro_substeps": macro_substeps,
        "games": len(records),
        "truncated_games": truncated_total,
        "wins": wins,
        "losses": losses,
        "draws": draws,
        "win_rate": wins / max(1, len(records)),
        "score_rate_with_half_draws": ci["mean"],
        "paired_bootstrap_ci95": [ci["ci95_low"], ci["ci95_high"]],
        "by_archetype": by_arch,
        "elapsed_seconds": elapsed,
    }
    atomic_write_json(out_dir / "summary.json", summary)
    return summary


def run_search_exam(
    exam_bin: Path,
    exam_json: Path,
    model_onnx: Path,
    out_json: Path,
    *,
    seeds: int = 8,
    macro_substeps: int = 0,
) -> dict:
    out_json.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exam_bin),
        str(exam_json),
        "--model",
        str(model_onnx),
        "--seeds",
        str(seeds),
        "--macro-substeps",
        str(macro_substeps),
        "--json",
        str(out_json),
    ]
    env = dict(os.environ)
    env["DIGIMON_CARDS_JSON"] = str(REPO_ROOT / "data" / "cards.json")
    res = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if res.returncode != 0:
        raise RuntimeError(f"search_exam failed:\n{res.stdout}\n{res.stderr}")
    report = json.loads(out_json.read_text(encoding="utf-8"))
    report["macro_substeps"] = macro_substeps
    report["stdout"] = res.stdout
    atomic_write_json(out_json, report)
    return report


def run_campaign(
    run_dir: Path,
    pool_dir: Path,
    cli_bin: Path,
    exam_bin: Path,
    cfg: CampaignConfig,
) -> dict:
    import digimon_engine
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from digimon_gym.digimon_gym import greedy_policy
    from tools.prepare_small_hybrid_pool import prepare_pool

    run_dir.mkdir(parents=True, exist_ok=True)
    atomic_write_json(run_dir / "campaign_config.json", asdict(cfg))

    if not (pool_dir / "manifest.json").exists():
        prepare_pool(out_dir=pool_dir)

    implemented = set(digimon_engine.load_implemented_card_ids())
    heldout_pool = GeneralistDeckPool.from_snapshot(
        pool_dir / "heldout_snapshot.json", implemented_card_ids=implemented
    )
    train_pool = GeneralistDeckPool.from_snapshot(
        pool_dir / "train_snapshot.json", implemented_card_ids=implemented
    )
    decks_path = pool_dir / "decks.json"
    heldout_decks_path = pool_dir / "heldout_decks.json"
    exam_json = REPO_ROOT / "qa" / "search-exams" / "ex13-examon-training-before-wingdramon.json"

    # Enable deterministic MLP eval memoization for search workers (safe for frozen MLPs).
    os.environ.setdefault("DIGIMON_SEARCH_EVAL_CACHE_ENTRIES", "8192")
    os.environ["DIGIMON_ONNX_OPPONENT"] = "1"

    state = load_campaign_state(run_dir)
    completed = set(state.get("completed_stages", []))

    # =========================================================================
    # STAGE 0: Mixed Teacher Bootstrap -> Gen0 [256, 256] MLP
    # =========================================================================
    stage0_dir = run_dir / "stage0_bootstrap"
    gen0_zip = stage0_dir / "seed.zip"
    gen0_onnx = stage0_dir / "seed.onnx"
    if "stage0_bootstrap" not in completed:
        update_progress(run_dir, "stage0_bootstrap", "running")
        from tools.cloud_search.mixed_bootstrap import collect as collect_mixed
        from tools.english_meta_search_job import bootstrap as fit_bootstrap

        boot_seed = cfg.seed + 10_000
        if not (stage0_dir / "qualification.json").exists():
            if stage0_dir.exists():
                shutil.rmtree(stage0_dir)
            collect_mixed(
                pool_dir / "train_snapshot.json",
                stage0_dir,
                arm="mixed_v2",
                seed=boot_seed,
                rounds=cfg.bootstrap_rounds,
            )
        fit_bootstrap(
            stage0_dir,
            games=cfg.bootstrap_rounds * 3,
            seed=boot_seed,
            device="cpu",
            teacher_label="mixed_v2",
            net_arch=list(cfg.net_arch),
            features_dim=cfg.features_dim,
        )
        write_mlp_meta_sidecar(gen0_zip)

        gen0_adapter = OnnxAgentAdapter(gen0_onnx, is_lstm=False)
        gate0 = {
            "vs_greedy": evaluate_policy_matchup_detailed(
                gen0_adapter, greedy_policy, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 500
            ),
            "vs_development_v1": evaluate_policy_matchup_detailed(
                gen0_adapter,
                development_v1_opponent,
                heldout_pool,
                cfg.gate_games_per_archetype,
                cfg.seed + 600,
            ),
            "examon_exam_macro0": run_search_exam(
                exam_bin, exam_json, gen0_onnx, stage0_dir / "exam_gen0_macro0.json",
                seeds=cfg.exam_seeds, macro_substeps=0
            ),
            "examon_exam_macro16": run_search_exam(
                exam_bin, exam_json, gen0_onnx, stage0_dir / "exam_gen0_macro16.json",
                seeds=cfg.exam_seeds, macro_substeps=cfg.macro_substeps
            ),
        }
        atomic_write_json(stage0_dir / "stage0_gate.json", gate0)
        completed.add("stage0_bootstrap")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["gen0_zip"] = str(gen0_zip)
        state["artifacts"]["gen0_onnx"] = str(gen0_onnx)
        save_campaign_state(run_dir, state)
        update_progress(
            run_dir,
            "stage0_bootstrap",
            "completed",
            wr_vs_greedy=gate0["vs_greedy"]["win_rate"],
            wr_vs_development_v1=gate0["vs_development_v1"]["win_rate"],
        )

    # =========================================================================
    # STAGE 1: Macro-Substep Search Baseline (Gens 1..N)
    # =========================================================================
    stage1_dir = run_dir / "stage1_search"
    n_s1 = cfg.search_generations_stage1
    gen3_zip = stage1_dir / f"gen{n_s1 - 1}" / "model.zip"
    gen3_onnx = stage1_dir / f"gen{n_s1 - 1}" / "model.onnx"
    if "stage1_search_base" not in completed:
        update_progress(run_dir, "stage1_search_base", "running", target_generations=n_s1)
        from tools.run_selfplay_generation import load_state as load_sp_state, main as sp_main

        existing_sp = load_sp_state(stage1_dir) if stage1_dir.exists() else {"next_gen": 0}
        done_gens = int(existing_sp.get("next_gen", 0))
        rem_gens = n_s1 - done_gens
        if rem_gens > 0:
            partial_gen_dir = stage1_dir / f"gen{done_gens}"
            resume_flag = ["--resume-selfplay"] if (partial_gen_dir / "schedule.json").exists() else []
            rc = sp_main(
                [
                    "--run-dir", str(stage1_dir),
                    "--seed-model", str(gen0_zip),
                    "--seed-model-onnx", str(gen0_onnx),
                    "--decks", str(decks_path),
                    "--cli-bin", str(cli_bin),
                    "--generations", str(rem_gens),
                    "--games", str(cfg.search_games),
                    "--procs", str(cfg.procs),
                    "--chunk-games", str(cfg.chunk_games),
                    "--sims", str(cfg.sims),
                    "--worlds", str(cfg.worlds),
                    "--eval-batch-size", str(cfg.eval_batch_size),
                    "--macro-substeps", str(cfg.macro_substeps),
                    "--reduced-sims", str(cfg.reduced_sims),
                    "--full-search-fraction", str(cfg.full_search_fraction),
                    "--cheap-sims", str(cfg.cheap_sims),
                    "--window", "3",
                    "--epochs", str(cfg.search_epochs),
                    "--batch-size", str(cfg.search_batch_size),
                    "--lr", "2e-4",
                    "--value-coef", "1.0",
                    "--value-holdout-fraction", str(cfg.holdout_fraction),
                    "--value-activation", "tanh",
                    "--train-device", "cpu",
                    "--quarantine-truncation",
                    "--skip-eval",
                    "--seed", str(cfg.seed),
                    *resume_flag,
                ]
            )
            if rc != 0:
                raise RuntimeError(f"Stage 1 selfplay exited with code {rc}")

        for g in range(n_s1):
            write_mlp_meta_sidecar(stage1_dir / f"gen{g}" / "model.zip")

        gen0_opp = OnnxAgentAdapter(gen0_onnx, is_lstm=False).as_opponent_fn()
        gen3_adapter = OnnxAgentAdapter(gen3_onnx, is_lstm=False)
        gate1 = {
            "vs_greedy": evaluate_policy_matchup_detailed(
                gen3_adapter, greedy_policy, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 1500
            ),
            "vs_development_v1": evaluate_policy_matchup_detailed(
                gen3_adapter, development_v1_opponent, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 1600
            ),
            "vs_gen0": evaluate_policy_matchup_detailed(
                gen3_adapter, gen0_opp, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 1700
            ),
            "examon_exam_macro0": run_search_exam(
                exam_bin, exam_json, gen3_onnx, stage1_dir / "exam_gen3_macro0.json",
                seeds=cfg.exam_seeds, macro_substeps=0
            ),
            "examon_exam_macro16": run_search_exam(
                exam_bin, exam_json, gen3_onnx, stage1_dir / "exam_gen3_macro16.json",
                seeds=cfg.exam_seeds, macro_substeps=cfg.macro_substeps
            ),
        }
        atomic_write_json(stage1_dir / "stage1_gate.json", gate1)
        completed.add("stage1_search_base")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["gen3_zip"] = str(gen3_zip)
        state["artifacts"]["gen3_onnx"] = str(gen3_onnx)
        save_campaign_state(run_dir, state)
        update_progress(
            run_dir,
            "stage1_search_base",
            "completed",
            wr_vs_gen0=gate1["vs_gen0"]["win_rate"],
            wr_vs_development_v1=gate1["vs_development_v1"]["win_rate"],
        )

    # =========================================================================
    # STAGE 2: Sequence Distillation -> lstm_distilled_r1.{zip,onnx}
    # =========================================================================
    stage2_dir = run_dir / "stage2_distill"
    lstm_dist_zip = stage2_dir / "lstm_distilled_r1.zip"
    lstm_dist_onnx = stage2_dir / "lstm_distilled_r1.onnx"
    if "stage2_lstm_distill" not in completed:
        update_progress(run_dir, "stage2_lstm_distill", "running")
        from digimon_gym.agents.azero_lstm_distill import distill_lstm

        shard_dirs = []
        for g in range(n_s1):
            shard_dirs.extend(sorted((stage1_dir / f"gen{g}" / "shards").glob("proc*")))

        distill_summary = distill_lstm(
            shard_dirs=shard_dirs,
            out=lstm_dist_zip,
            init_extractor_from=gen3_zip,
            lstm_hidden_size=cfg.lstm_hidden_size,
            net_arch=list(cfg.net_arch),
            features_dim=cfg.features_dim,
            embedding_dim=8,
            epochs=cfg.distill_epochs,
            seq_batch_size=cfg.distill_seq_batch_size,
            lr=3e-4,
            value_coef=1.0,
            holdout_fraction=cfg.holdout_fraction,
            device="cpu",
            seed=cfg.seed + 200_000,
            export_onnx=lstm_dist_onnx,
        )
        atomic_write_json(stage2_dir / "distill_summary.json", distill_summary)

        lstm_dist_adapter = OnnxAgentAdapter(lstm_dist_onnx, is_lstm=True, skip_forced=True)
        gen3_opp = OnnxAgentAdapter(gen3_onnx, is_lstm=False).as_opponent_fn()
        gate2 = {
            "vs_greedy": evaluate_policy_matchup_detailed(
                lstm_dist_adapter, greedy_policy, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 2500
            ),
            "vs_development_v1": evaluate_policy_matchup_detailed(
                lstm_dist_adapter, development_v1_opponent, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 2600
            ),
            "vs_gen3": evaluate_policy_matchup_detailed(
                lstm_dist_adapter, gen3_opp, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 2700
            ),
        }
        atomic_write_json(stage2_dir / "stage2_gate.json", gate2)
        completed.add("stage2_lstm_distill")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["lstm_distilled_zip"] = str(lstm_dist_zip)
        state["artifacts"]["lstm_distilled_onnx"] = str(lstm_dist_onnx)
        save_campaign_state(run_dir, state)
        update_progress(
            run_dir,
            "stage2_lstm_distill",
            "completed",
            wr_vs_gen3=gate2["vs_gen3"]["win_rate"],
            wr_vs_greedy=gate2["vs_greedy"]["win_rate"],
        )

    # =========================================================================
    # STAGE 3: KL-Anchored Clean-Reward LSTM PPO -> lstm_ppo_r1.{zip,onnx}
    # =========================================================================
    stage3_dir = run_dir / "stage3_ppo"
    lstm_ppo_zip = stage3_dir / "lstm_ppo_r1.zip"
    lstm_ppo_onnx = stage3_dir / "lstm_ppo_r1.onnx"
    if "stage3_lstm_ppo" not in completed:
        update_progress(run_dir, "stage3_lstm_ppo", "running", timesteps=cfg.ppo_timesteps)
        from digimon_gym.agents.opponent_pool import OpponentEntry as PpoOpponentEntry, OpponentPool
        from digimon_gym.agents.pilot_training import train as ppo_train
        from digimon_gym.agents.training_config import TrainingConfig
        from digimon_gym.tensor_profiles import get_tensor_profile
        from tools.export_onnx import export_lstm

        stage3_dir.mkdir(parents=True, exist_ok=True)
        ppo_pool_path = stage3_dir / "ppo_opponent_pool.json"
        prior_wrs = [0.60, 0.50, 0.35]
        ppo_entries = []
        for g in range(n_s1):
            wr_est = prior_wrs[min(g, len(prior_wrs) - 1)]
            ppo_entries.append(
                PpoOpponentEntry(
                    name=f"gen{g + 1}",
                    weights_path=str((stage1_dir / f"gen{g}" / "model.zip").resolve()),
                    algorithm="mlp",
                    win_rate_vs_pool=wr_est,
                    games_played=100,
                )
            )
        ppo_pool = OpponentPool(manifest_path=ppo_pool_path, entries=ppo_entries)
        ppo_pool.save()

        ppo_cfg = TrainingConfig(
            algorithm="lstm",
            timesteps=cfg.ppo_timesteps,
            seed=cfg.seed + 300_000,
            n_envs=cfg.ppo_n_envs,
            vec_env_backend="subproc" if cfg.ppo_n_envs > 1 else "dummy",
            learning_rate=cfg.ppo_lr,
            lr_schedule="linear",
            n_steps=cfg.ppo_n_steps,
            batch_size=cfg.ppo_n_steps,
            n_epochs=cfg.ppo_epochs,
            gamma=0.995,
            gae_lambda=0.95,
            clip_range=0.2,
            ent_coef=0.005,
            vf_coef=0.5,
            max_grad_norm=0.5,
            lstm_hidden_size=cfg.lstm_hidden_size,
            net_arch=list(cfg.net_arch),
            opponent="pool",
            opponent_pool_manifest=str(ppo_pool_path),
            opponent_pool_mode="pfsp",
            eval_freq=cfg.ppo_eval_freq,
            eval_episodes=cfg.ppo_eval_episodes,
            anchored_eval_freq=0,
            checkpoint_every=cfg.ppo_checkpoint_every,
            init_from=str(lstm_dist_zip),
            kl_anchor_checkpoint=str(lstm_dist_zip),
            kl_coef=cfg.kl_coef,
            auto_advance_forced=True,
            generalist=True,
            curriculum_pool=str(pool_dir / "train_snapshot.json"),
            curriculum_seed=cfg.seed + 310_000,
            eval_seed=cfg.seed + 320_000,
            models_dir=str(stage3_dir / "models"),
            tensorboard_log=str(stage3_dir / "tb"),
            run_name="lstm_ppo_r1",
            tensor_profile="standard_lite_v2",
            record_games="off",
            mulligan_log="off",
            eval_game_log="on",
            match_format="single",
            reward_profile_override="terminal_strategic_v1",
        )
        ppo_train(
            cfg=ppo_cfg,
            generalist_deck_pool=train_pool,
            device="cpu",
            verbose=1,
        )
        final_zip = stage3_dir / "models" / "lstm_ppo_r1" / "final.zip"
        final_meta = stage3_dir / "models" / "lstm_ppo_r1" / "final.meta.json"
        shutil.copy2(final_zip, lstm_ppo_zip)
        shutil.copy2(final_meta, lstm_ppo_zip.with_suffix(".meta.json"))
        export_lstm(str(lstm_ppo_zip), str(lstm_ppo_onnx), get_tensor_profile("standard_lite_v2"))

        lstm_ppo_adapter = OnnxAgentAdapter(lstm_ppo_onnx, is_lstm=True, skip_forced=True)
        lstm_dist_opp = OnnxAgentAdapter(lstm_dist_onnx, is_lstm=True, skip_forced=True).as_opponent_fn()
        gen3_opp = OnnxAgentAdapter(gen3_onnx, is_lstm=False).as_opponent_fn()
        gate3 = {
            "vs_greedy": evaluate_policy_matchup_detailed(
                lstm_ppo_adapter, greedy_policy, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 3500
            ),
            "vs_development_v1": evaluate_policy_matchup_detailed(
                lstm_ppo_adapter, development_v1_opponent, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 3600
            ),
            "vs_lstm_distilled_r1": evaluate_policy_matchup_detailed(
                lstm_ppo_adapter, lstm_dist_opp, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 3700
            ),
            "vs_gen3": evaluate_policy_matchup_detailed(
                lstm_ppo_adapter, gen3_opp, heldout_pool, cfg.gate_games_per_archetype, cfg.seed + 3800
            ),
        }
        atomic_write_json(stage3_dir / "stage3_gate.json", gate3)
        completed.add("stage3_lstm_ppo")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["lstm_ppo_zip"] = str(lstm_ppo_zip)
        state["artifacts"]["lstm_ppo_onnx"] = str(lstm_ppo_onnx)
        save_campaign_state(run_dir, state)
        update_progress(
            run_dir,
            "stage3_lstm_ppo",
            "completed",
            wr_vs_distilled=gate3["vs_lstm_distilled_r1"]["win_rate"],
            wr_vs_gen3=gate3["vs_gen3"]["win_rate"],
        )

    # =========================================================================
    # STAGE 4: Hybrid Search Self-Play vs Pure-Search Control (Gens 4..6)
    # =========================================================================
    stage4_hybrid_dir = run_dir / "stage4_hybrid"
    stage4_control_dir = run_dir / "stage4_control"
    n_s4 = cfg.search_generations_stage4
    final_gen_idx = n_s1 + n_s4 - 1
    gen6_hybrid_zip = stage4_hybrid_dir / f"gen{final_gen_idx}" / "model.zip"
    gen6_hybrid_onnx = stage4_hybrid_dir / f"gen{final_gen_idx}" / "model.onnx"
    gen6_control_zip = stage4_control_dir / f"gen{final_gen_idx}" / "model.zip"
    gen6_control_onnx = stage4_control_dir / f"gen{final_gen_idx}" / "model.onnx"

    if "stage4_hybrid_and_control" not in completed:
        from digimon_gym.agents.belief_contract import bind_prepared_run
        from tools.run_selfplay_generation import (
            load_state as load_sp_state,
            main as sp_main,
            save_state as save_sp_state,
        )
        from tools.search_opponents import SCHEMA as OPP_SCHEMA, checkpoint_entry

        def _bootstrap_stage4_arm(
            arm_dir: Path,
            pool_opponents: Optional[list[dict]],
            history_window: int,
            arm_label: str,
        ) -> None:
            arm_dir.mkdir(parents=True, exist_ok=True)
            for g in range(n_s1):
                dst = arm_dir / f"gen{g}"
                src = (stage1_dir / f"gen{g}").resolve()
                if not dst.exists():
                    os.symlink(src, dst, target_is_directory=True)
            if not (arm_dir / "state.json").exists():
                save_sp_state(
                    arm_dir,
                    {
                        "next_gen": n_s1,
                        "prev": {
                            "model_zip": str(gen3_zip.resolve()),
                            "model_onnx": str(gen3_onnx.resolve()),
                        },
                    },
                )
            bind_prepared_run(arm_dir, "open_decklist", None)
            pool_flags: list[str] = []
            if pool_opponents:
                pool_src = arm_dir / "pool_source.json"
                atomic_write_json(
                    pool_src,
                    {
                        "schema": OPP_SCHEMA,
                        "selfplay_probability": 0.5,
                        "opponents": pool_opponents,
                    },
                )
                pool_flags = ["--opponent-pool", str(pool_src)]
            sp_state = load_sp_state(arm_dir)
            done_g = int(sp_state.get("next_gen", n_s1))
            rem_g = (n_s1 + n_s4) - done_g
            if rem_g > 0:
                update_progress(run_dir, f"stage4_{arm_label}", "running", remaining_generations=rem_g)
                partial_dir = arm_dir / f"gen{done_g}"
                resume_flag = ["--resume-selfplay"] if (partial_dir / "schedule.json").exists() else []
                rc = sp_main(
                    [
                        "--run-dir", str(arm_dir),
                        "--seed-model", str(gen0_zip),
                        "--seed-model-onnx", str(gen0_onnx),
                        *pool_flags,
                        "--history-window", str(history_window),
                        "--selfplay-probability", "0.5",
                        "--decks", str(decks_path),
                        "--cli-bin", str(cli_bin),
                        "--generations", str(rem_g),
                        "--games", str(cfg.search_games),
                        "--procs", str(cfg.procs),
                        "--chunk-games", str(cfg.chunk_games),
                        "--sims", str(cfg.sims),
                        "--worlds", str(cfg.worlds),
                        "--eval-batch-size", str(cfg.eval_batch_size),
                        "--macro-substeps", str(cfg.macro_substeps),
                        "--reduced-sims", str(cfg.reduced_sims),
                        "--full-search-fraction", str(cfg.full_search_fraction),
                        "--cheap-sims", str(cfg.cheap_sims),
                        "--window", "3",
                        "--epochs", str(cfg.search_epochs),
                        "--batch-size", str(cfg.search_batch_size),
                        "--lr", "1e-4",
                        "--value-coef", "1.0",
                        "--value-holdout-fraction", str(cfg.holdout_fraction),
                        "--value-activation", "tanh",
                        "--train-device", "cpu",
                        "--quarantine-truncation",
                        "--skip-eval",
                        "--seed", str(cfg.seed),
                        *resume_flag,
                    ]
                )
                if rc != 0:
                    raise RuntimeError(f"Stage 4 ({arm_label}) exited with code {rc}")
            for g in range(n_s1, n_s1 + n_s4):
                write_mlp_meta_sidecar(arm_dir / f"gen{g}" / "model.zip")

        # Hybrid pool: lstm_ppo_r1 (policy_lstm, w=2.0) + rolling 2-gen search history (w=1.0 each)
        hybrid_opponents = [
            checkpoint_entry("lstm-ppo-r1", lstm_ppo_onnx, weight=2.0, mode="policy_lstm"),
        ]

        _bootstrap_stage4_arm(stage4_hybrid_dir, hybrid_opponents, 2, "hybrid")
        # Control pool: pure rolling 3-gen historical search checkpoints (same seeds/budget, no LSTM)
        _bootstrap_stage4_arm(stage4_control_dir, None, 3, "control")

        completed.add("stage4_hybrid_and_control")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["gen6_hybrid_zip"] = str(gen6_hybrid_zip)
        state["artifacts"]["gen6_hybrid_onnx"] = str(gen6_hybrid_onnx)
        state["artifacts"]["gen6_control_zip"] = str(gen6_control_zip)
        state["artifacts"]["gen6_control_onnx"] = str(gen6_control_onnx)
        save_campaign_state(run_dir, state)
        update_progress(run_dir, "stage4_hybrid_and_control", "completed")

    # =========================================================================
    # STAGE 5: Held-Out Anchored Evaluation + 320/4 Searched Panel + Search Exams
    # =========================================================================
    stage5_dir = run_dir / "stage5_eval"
    final_eval_path = stage5_dir / "final_evaluation.json"
    if "stage5_evaluation" not in completed:
        update_progress(run_dir, "stage5_evaluation", "running")
        stage5_dir.mkdir(parents=True, exist_ok=True)

        adapters = {
            "Gen0": OnnxAgentAdapter(gen0_onnx, is_lstm=False),
            "Gen3": OnnxAgentAdapter(gen3_onnx, is_lstm=False),
            "lstm_distilled_r1": OnnxAgentAdapter(lstm_dist_onnx, is_lstm=True, skip_forced=True),
            "lstm_ppo_r1": OnnxAgentAdapter(lstm_ppo_onnx, is_lstm=True, skip_forced=True),
            "Gen6_control": OnnxAgentAdapter(gen6_control_onnx, is_lstm=False),
            "Gen6_hybrid": OnnxAgentAdapter(gen6_hybrid_onnx, is_lstm=False),
        }

        policy_only_vs_baselines: dict[str, dict] = {}
        for idx, (name, adapter) in enumerate(adapters.items()):
            update_progress(run_dir, "stage5_evaluation", "policy_baselines", candidate=name)
            policy_only_vs_baselines[name] = {
                "vs_greedy": evaluate_policy_matchup_detailed(
                    adapter,
                    greedy_policy,
                    heldout_pool,
                    cfg.policy_eval_games_per_archetype,
                    cfg.seed + 50_000 + idx * 1000,
                ),
                "vs_development_v1": evaluate_policy_matchup_detailed(
                    adapter,
                    development_v1_opponent,
                    heldout_pool,
                    cfg.policy_eval_games_per_archetype,
                    cfg.seed + 60_000 + idx * 1000,
                ),
            }

        head_to_head_specs = [
            ("Gen6_hybrid_vs_Gen6_control", "Gen6_hybrid", "Gen6_control", 71_000),
            ("Gen6_hybrid_vs_Gen3", "Gen6_hybrid", "Gen3", 72_000),
            ("Gen6_control_vs_Gen3", "Gen6_control", "Gen3", 73_000),
            ("Gen6_hybrid_vs_lstm_ppo_r1", "Gen6_hybrid", "lstm_ppo_r1", 74_000),
            ("lstm_ppo_r1_vs_Gen3", "lstm_ppo_r1", "Gen3", 75_000),
            ("lstm_ppo_r1_vs_lstm_distilled_r1", "lstm_ppo_r1", "lstm_distilled_r1", 76_000),
            ("Gen3_vs_Gen0", "Gen3", "Gen0", 77_000),
        ]
        policy_head_to_heads: dict[str, dict] = {}
        for label, cand_name, opp_name, seed_off in head_to_head_specs:
            update_progress(run_dir, "stage5_evaluation", "policy_h2h", matchup=label)
            policy_head_to_heads[label] = evaluate_policy_matchup_detailed(
                adapters[cand_name],
                adapters[opp_name].as_opponent_fn(),
                heldout_pool,
                cfg.policy_eval_games_per_archetype,
                cfg.seed + seed_off,
            )

        searched_specs = [
            ("Gen6_hybrid_vs_Gen6_control", gen6_hybrid_onnx, "gen6-control", gen6_control_onnx, "search", 81_000),
            ("Gen6_hybrid_vs_Gen3", gen6_hybrid_onnx, "gen3", gen3_onnx, "search", 82_000),
            ("Gen6_control_vs_Gen3", gen6_control_onnx, "gen3", gen3_onnx, "search", 83_000),
            ("Gen6_hybrid_vs_lstm_ppo_r1", gen6_hybrid_onnx, "lstm-ppo-r1", lstm_ppo_onnx, "policy_lstm", 84_000),
        ]
        searched_head_to_heads: dict[str, dict] = {}
        for label, cand_onnx, opp_id, opp_onnx, opp_mode, seed_off in searched_specs:
            update_progress(run_dir, "stage5_evaluation", "searched_h2h", matchup=label)
            searched_head_to_heads[label] = run_searched_head_to_head(
                cli_bin=cli_bin,
                out_dir=stage5_dir / "searched_h2h" / label,
                candidate_onnx=cand_onnx,
                opponent_id=opp_id,
                opponent_onnx=opp_onnx,
                opponent_mode=opp_mode,
                heldout_decks_path=heldout_decks_path,
                games=cfg.searched_eval_games,
                procs=cfg.procs,
                sims=cfg.sims,
                worlds=cfg.worlds,
                eval_batch_size=cfg.eval_batch_size,
                macro_substeps=cfg.macro_substeps,
                reduced_sims=cfg.reduced_sims,
                seed=cfg.seed + seed_off,
            )

        search_exams: dict[str, dict] = {}
        for name, onnx_path in (
            ("Gen0", gen0_onnx),
            ("Gen3", gen3_onnx),
            ("Gen6_control", gen6_control_onnx),
            ("Gen6_hybrid", gen6_hybrid_onnx),
        ):
            search_exams[name] = {
                "macro_0": run_search_exam(
                    exam_bin, exam_json, onnx_path, stage5_dir / "exams" / f"{name}_macro0.json",
                    seeds=cfg.exam_seeds, macro_substeps=0
                ),
                "macro_16": run_search_exam(
                    exam_bin, exam_json, onnx_path, stage5_dir / "exams" / f"{name}_macro16.json",
                    seeds=cfg.exam_seeds, macro_substeps=cfg.macro_substeps
                ),
            }

        final_report = {
            "schema": "hybrid-small-campaign-eval-v1",
            "preset": cfg.preset,
            "config": asdict(cfg),
            "pool_manifest": json.loads((pool_dir / "manifest.json").read_text(encoding="utf-8")),
            "policy_only_vs_baselines": policy_only_vs_baselines,
            "policy_only_head_to_heads": policy_head_to_heads,
            "searched_head_to_heads": searched_head_to_heads,
            "search_exams": search_exams,
        }
        atomic_write_json(final_eval_path, final_report)
        completed.add("stage5_evaluation")
        state["completed_stages"] = sorted(completed)
        state["artifacts"]["final_evaluation"] = str(final_eval_path)
        save_campaign_state(run_dir, state)
        update_progress(run_dir, "stage5_evaluation", "completed", report=str(final_eval_path))

    return json.loads(final_eval_path.read_text(encoding="utf-8"))


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--run-dir", type=Path, required=True)
    p.add_argument(
        "--pool-dir",
        type=Path,
        default=REPO_ROOT / "data" / "training_pools" / "ex13-small3",
    )
    p.add_argument(
        "--cli-bin",
        type=Path,
        default=REPO_ROOT / "target" / "release" / "digimon-engine-cli",
    )
    p.add_argument(
        "--exam-bin",
        type=Path,
        default=REPO_ROOT / "target" / "release" / "examples" / "search_exam",
    )
    p.add_argument("--preset", choices=("full", "smoke"), default="full")
    p.add_argument("--seed", type=int, default=20261126)
    p.add_argument("--procs", type=int, default=2)
    p.add_argument("--bootstrap-rounds", type=int, default=None)
    p.add_argument("--search-games", type=int, default=None)
    p.add_argument("--chunk-games", type=int, default=None)
    p.add_argument("--ppo-timesteps", type=int, default=None)
    p.add_argument("--policy-eval-games-per-archetype", type=int, default=None)
    p.add_argument("--searched-eval-games", type=int, default=None)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    cfg = make_config(args)
    run_campaign(args.run_dir, args.pool_dir, args.cli_bin, args.exam_bin, cfg)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
