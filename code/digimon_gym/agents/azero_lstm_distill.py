"""Sequence distillation from AlphaZero self-play shards into MaskableRecurrentPPO.

Consumes ``selfplay-shards-v1`` directories (chronological non-forced game
trajectories with ``game_id.npy`` and ``mover.npy``), groups rows into per-seat
episodes ``(game_id, mover)``, holds out WHOLE games for validation, and trains
a ``MaskableRecurrentActorCriticPolicy`` checkpoint with:

    loss = weighted_CE(normalized pi, masked_log_softmax(logits)) + value_coef * MSE(value, z)

Saved checkpoints are standard SB3 ``MaskableRecurrentPPO`` ``.zip`` files plus
a validated ``.meta.json`` sidecar, ready for:
  1. Warm-starting KL-anchored recurrent PPO (``pilot_training.py --lstm --init-from ...``)
  2. Exporting to native Rust ONNX (``code/tools/export_onnx.py --type lstm``)
  3. Direct evaluation via ``code/tools/anchored_eval_cli.py``
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import gymnasium as gym
import numpy as np
import torch
import torch.nn.functional as F

from model_contract import current_contract, validate_checkpoint, validate_contract

SCHEMA = "selfplay-shards-v1"


@dataclass
class SeatSequence:
    """Chronological non-forced trajectory for one seat in one game."""

    game_id: int
    mover: int
    obs: np.ndarray            # (T, obs_dim) f32
    offsets: np.ndarray        # (T + 1,) i64 local offsets into actions/visits
    actions: np.ndarray        # (nnz,) i32
    visits: np.ndarray         # (nnz,) f32
    policy_weight: np.ndarray  # (T,) f32
    z: np.ndarray              # (T,) f32
    terminated: np.ndarray     # (T,) bool

    @property
    def length(self) -> int:
        return int(self.obs.shape[0])


@dataclass
class SequenceDataset:
    sequences: list[SeatSequence]
    obs_dim: int
    action_dim: int
    observation_profile: str
    manifests: list[dict]
    exclusions: list[dict]

    @property
    def total_rows(self) -> int:
        return sum(s.length for s in self.sequences)


def load_sequence_shards(dirs: list[Path]) -> SequenceDataset:
    """Load validated selfplay-shards-v1 directories as chronological per-seat sequences."""
    sequences: list[SeatSequence] = []
    manifests: list[dict] = []
    exclusions: list[dict] = []
    obs_dim: Optional[int] = None
    action_dim: Optional[int] = None
    profile: Optional[str] = None
    game_base = 0

    for d in dirs:
        manifest_path = d / "manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("evaluation_only"):
            raise ValueError(f"{manifest_path}: policy-only evaluation cannot provide training targets")
        if manifest.get("schema") != SCHEMA:
            raise ValueError(f"{manifest_path}: schema {manifest.get('schema')!r} != {SCHEMA!r}")

        quarantine_path = d / "training_exclusions.json"
        quarantine = None
        excluded_ids: set[int] = set()
        seen_ids: set[int] = set()
        excluded_rows = 0
        if quarantine_path.exists():
            quarantine = json.loads(quarantine_path.read_text(encoding="utf-8"))
            entries = quarantine.get("games")
            expected_sha = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
            if (
                quarantine.get("schema") != "selfplay-training-exclusions-v1"
                or quarantine.get("manifest_sha256") != expected_sha
                or not isinstance(entries, list)
                or not entries
            ):
                raise ValueError(f"{quarantine_path}: invalid exclusion schema or manifest hash")
            for entry in entries:
                gid = entry.get("game_id") if isinstance(entry, dict) else None
                reason = entry.get("reason") if isinstance(entry, dict) else None
                if (
                    type(gid) is not int
                    or gid < 0
                    or not isinstance(reason, str)
                    or not reason.strip()
                    or gid in excluded_ids
                ):
                    raise ValueError(f"{quarantine_path}: invalid or duplicate game exclusion")
                excluded_ids.add(gid)

        contract = validate_contract(manifest.get("model_contract"), manifest["observation_profile"])
        if contract["observation_profile"] != manifest["observation_profile"]:
            raise ValueError(f"{manifest_path}: observation_profile differs from contract")
        if (
            contract["tensor_size"] != manifest["obs_dim"]
            or contract["action_space_size"] != manifest["action_dim"]
        ):
            raise ValueError(f"{manifest_path}: dimensions differ from model contract")
        if manifest.get("value_target") != "terminal_outcome":
            raise ValueError(f"{manifest_path}: value_target must be terminal_outcome")

        if obs_dim is None:
            obs_dim = int(manifest["obs_dim"])
            action_dim = int(manifest["action_dim"])
            profile = str(manifest["observation_profile"])
        else:
            if int(manifest["obs_dim"]) != obs_dim or int(manifest["action_dim"]) != action_dim:
                raise ValueError(f"{manifest_path}: dims disagree with earlier shard dirs")
            if manifest["observation_profile"] != profile:
                raise ValueError(
                    f"{manifest_path}: observation_profile {manifest['observation_profile']!r} != {profile!r}"
                )

        from digimon_gym.agents.belief_contract import require_same_information

        require_same_information(manifests + [manifest])
        manifests.append(manifest)

        # Accumulate rows per (rebased_game_id, mover) across shards in manifest order.
        by_key: dict[tuple[int, int], list[tuple[np.ndarray, np.ndarray, np.ndarray, float, float, bool]]] = {}
        max_game = -1

        for shard in manifest["shards"]:
            s = d / shard["dir"]
            obs = np.load(s / "obs.npy")
            off = np.load(s / "policy_offsets.npy")
            act = np.load(s / "policy_actions.npy")
            vis = np.load(s / "policy_visits.npy")
            z = np.load(s / "z.npy")
            terminated = np.load(s / "terminated.npy")
            movers = np.load(s / "mover.npy")
            games = np.load(s / "game_id.npy")
            rows = obs.shape[0]
            if (
                rows != shard["rows"]
                or off.shape != (rows + 1,)
                or z.shape != (rows,)
                or terminated.shape != (rows,)
                or movers.shape != (rows,)
                or games.shape != (rows,)
            ):
                raise ValueError(f"{s}: row bookkeeping inconsistent")
            if not np.all(np.isin(z, [-1.0, 0.0, 1.0])) or not np.all(np.isin(terminated, [0, 1])):
                raise ValueError(f"{s}: invalid terminal targets")
            if off[0] != 0 or off[-1] != len(act) or len(act) != len(vis) or not np.all(np.diff(off) > 0):
                raise ValueError(f"{s}: invalid sparse policy offsets")
            if (
                obs.shape[1] != obs_dim
                or not np.isfinite(obs).all()
                or not np.isfinite(vis).all()
                or np.any(vis < 0)
                or np.any(act < 0)
                or np.any(act >= action_dim)
            ):
                raise ValueError(f"{s}: invalid observations or policy targets")
            weight_path = s / "policy_weight.npy"
            if weight_path.exists():
                weight = np.load(weight_path)
                if weight.shape != (rows,) or not np.isfinite(weight).all() or np.any(weight < 0):
                    raise ValueError(f"{s}: invalid policy_weight array")
            else:
                weight = np.ones(rows, dtype=np.float32)

            max_game = max(max_game, int(games.max(initial=-1)))
            if excluded_ids:
                seen_ids.update(int(g) for g in np.unique(games))

            for row_idx in range(rows):
                raw_gid = int(games[row_idx])
                if raw_gid in excluded_ids:
                    excluded_rows += 1
                    continue
                gid = raw_gid + game_base
                mv = int(movers[row_idx])
                s_idx, e_idx = int(off[row_idx]), int(off[row_idx + 1])
                by_key.setdefault((gid, mv), []).append(
                    (
                        obs[row_idx].astype(np.float32, copy=True),
                        act[s_idx:e_idx].astype(np.int32, copy=True),
                        vis[s_idx:e_idx].astype(np.float32, copy=True),
                        float(weight[row_idx]),
                        float(z[row_idx]),
                        bool(terminated[row_idx]),
                    )
                )

        if excluded_ids - seen_ids:
            raise ValueError(
                f"{quarantine_path}: exclusion refers to absent game IDs {sorted(excluded_ids - seen_ids)}"
            )
        if quarantine is not None:
            exclusions.append(
                {
                    "directory": str(d),
                    "manifest_sha256": quarantine["manifest_sha256"],
                    "games": quarantine["games"],
                    "excluded_rows": excluded_rows,
                }
            )

        for (gid, mv), steps in by_key.items():
            obs_arr = np.stack([st[0] for st in steps], axis=0)
            act_parts = [st[1] for st in steps]
            vis_parts = [st[2] for st in steps]
            lengths = [len(a) for a in act_parts]
            local_offsets = np.concatenate(([0], np.cumsum(lengths, dtype=np.int64)))
            sequences.append(
                SeatSequence(
                    game_id=gid,
                    mover=mv,
                    obs=obs_arr,
                    offsets=local_offsets,
                    actions=np.concatenate(act_parts).astype(np.int32),
                    visits=np.concatenate(vis_parts).astype(np.float32),
                    policy_weight=np.asarray([st[3] for st in steps], dtype=np.float32),
                    z=np.asarray([st[4] for st in steps], dtype=np.float32),
                    terminated=np.asarray([st[5] for st in steps], dtype=bool),
                )
            )
        game_base += max_game + 1

    if obs_dim is None or action_dim is None or profile is None:
        raise ValueError("no shard directories given")
    return SequenceDataset(
        sequences=sequences,
        obs_dim=obs_dim,
        action_dim=action_dim,
        observation_profile=profile,
        manifests=manifests,
        exclusions=exclusions,
    )


def _collate_batch(
    seqs: list[SeatSequence], action_dim: int, device: torch.device
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    """Collate B variable-length seat trajectories into time-major tensors (T_max, B, ...)."""
    batch_size = len(seqs)
    max_t = max(s.length for s in seqs)
    obs_dim = seqs[0].obs.shape[1]

    obs = np.zeros((max_t, batch_size, obs_dim), dtype=np.float32)
    pi = np.zeros((max_t, batch_size, action_dim), dtype=np.float32)
    legal = np.zeros((max_t, batch_size, action_dim), dtype=bool)
    legal[:, :, 0] = True  # safe dummy legal slot on padded tail positions
    seq_mask = np.zeros((max_t, batch_size), dtype=bool)
    pol_weight = np.zeros((max_t, batch_size), dtype=np.float32)
    z = np.zeros((max_t, batch_size), dtype=np.float32)
    term = np.zeros((max_t, batch_size), dtype=bool)

    for b, seq in enumerate(seqs):
        t_len = seq.length
        obs[:t_len, b] = seq.obs
        seq_mask[:t_len, b] = True
        pol_weight[:t_len, b] = seq.policy_weight
        z[:t_len, b] = seq.z
        term[:t_len, b] = seq.terminated
        legal[:t_len, b, 0] = False
        for t in range(t_len):
            s_idx, e_idx = seq.offsets[t], seq.offsets[t + 1]
            acts = seq.actions[s_idx:e_idx]
            vis = seq.visits[s_idx:e_idx]
            legal[t, b, acts] = True
            total = float(vis.sum())
            if total > 0.0:
                pi[t, b, acts] = vis / total
            else:
                pi[t, b, acts] = 1.0 / len(acts)

    return (
        torch.as_tensor(obs, device=device),
        torch.as_tensor(pi, device=device),
        torch.as_tensor(legal, device=device),
        torch.as_tensor(seq_mask, device=device),
        torch.as_tensor(pol_weight, device=device),
        torch.as_tensor(z, device=device),
        torch.as_tensor(term, device=device),
    )


def _forward_sequences(policy, obs_tb: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
    """Run shared/split feature extractor + actor/critic LSTMs over (T, B, obs_dim)."""
    t_max, batch_size, obs_dim = obs_tb.shape
    flat_obs = obs_tb.reshape(t_max * batch_size, obs_dim)
    feats = policy.extract_features(flat_obs)
    if policy.share_features_extractor:
        pi_feats = vf_feats = feats.reshape(t_max, batch_size, -1)
    else:
        pi_feats = feats[0].reshape(t_max, batch_size, -1)
        vf_feats = feats[1].reshape(t_max, batch_size, -1)

    dev = obs_tb.device
    h0_pi = torch.zeros(
        (policy.lstm_actor.num_layers, batch_size, policy.lstm_actor.hidden_size),
        dtype=torch.float32,
        device=dev,
    )
    c0_pi = torch.zeros_like(h0_pi)
    lstm_pi, _ = policy.lstm_actor(pi_feats, (h0_pi, c0_pi))

    if policy.lstm_critic is not None:
        h0_vf = torch.zeros(
            (policy.lstm_critic.num_layers, batch_size, policy.lstm_critic.hidden_size),
            dtype=torch.float32,
            device=dev,
        )
        c0_vf = torch.zeros_like(h0_vf)
        lstm_vf, _ = policy.lstm_critic(vf_feats, (h0_vf, c0_vf))
    elif policy.shared_lstm:
        lstm_vf = lstm_pi.detach()
    else:
        lstm_vf = policy.critic(vf_feats)

    flat_pi = policy.mlp_extractor.forward_actor(lstm_pi.reshape(t_max * batch_size, -1))
    flat_vf = policy.mlp_extractor.forward_critic(lstm_vf.reshape(t_max * batch_size, -1))
    logits = policy.action_net(flat_pi).reshape(t_max, batch_size, -1)
    values = policy.value_net(flat_vf).reshape(t_max, batch_size)
    return logits, values


def _evaluate_sequences(
    policy,
    seqs: list[SeatSequence],
    action_dim: int,
    seq_batch_size: int,
    constant_z: float,
    value_coef: float,
) -> dict:
    was_training = policy.training
    policy.set_training_mode(False)
    dev = policy.device
    ce_sum = weight_sum = 0.0
    sq_err: list[float] = []
    base_err: list[float] = []

    with torch.no_grad():
        for start in range(0, len(seqs), seq_batch_size):
            batch = seqs[start : start + seq_batch_size]
            obs_t, pi_t, legal_t, seq_mask_t, weight_t, z_t, term_t = _collate_batch(
                batch, action_dim, dev
            )
            logits, values = _forward_sequences(policy, obs_t)
            values_clamped = values.clamp(-1.0, 1.0)
            masked_logits = logits.masked_fill(~legal_t, torch.finfo(logits.dtype).min)
            logp = torch.log_softmax(masked_logits, dim=-1)
            step_ce = -(pi_t * logp.masked_fill(~legal_t, 0.0)).sum(dim=-1)
            eff_w = weight_t * seq_mask_t.float()
            ce_sum += float((step_ce * eff_w).sum().item())
            weight_sum += float(eff_w.sum().item())

            val_mask = seq_mask_t & term_t
            if val_mask.any():
                preds = values_clamped[val_mask].cpu().numpy()
                targets = z_t[val_mask].cpu().numpy()
                sq_err.extend(((preds - targets) ** 2).tolist())
                base_err.extend(((constant_z - targets) ** 2).tolist())

    policy.set_training_mode(was_training)
    policy_ce = ce_sum / weight_sum if weight_sum > 0.0 else None
    value_mse = float(np.mean(sq_err)) if sq_err else None
    constant_mse = float(np.mean(base_err)) if base_err else None
    return {
        "policy_ce": policy_ce,
        "value_mse": value_mse,
        "constant_mse": constant_mse,
        "value_rows": len(sq_err),
        "value_beats_constant": (
            None if (value_mse is None or constant_mse is None) else value_mse < constant_mse
        ),
        "loss": (policy_ce or 0.0) + value_coef * (value_mse or 0.0),
    }


def _write_lstm_meta_sidecar(
    out_zip: Path,
    contract: dict,
    *,
    lstm_hidden_size: int,
    net_arch: list[int],
    features_dim: int,
    embedding_dim: int,
    information: dict,
) -> Path:
    from digimon_engine import REGISTRY_CAPACITY
    from digimon_gym.agents.pilot_training import current_action_space_structure

    meta = {
        **contract,
        "algorithm": "lstm",
        "action_space_structure": current_action_space_structure(),
        "card_registry_capacity": int(REGISTRY_CAPACITY),
        "embedding_dim": int(embedding_dim),
        "features_dim": int(features_dim),
        "lstm_hidden_size": int(lstm_hidden_size),
        "net_arch": list(net_arch),
        "value_semantics": "terminal_outcome",
        "skip_forced_steps": True,
        "information": information,
    }
    meta_path = out_zip.with_suffix(".meta.json")
    meta_path.write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return meta_path


def distill_lstm(
    shard_dirs: list[Path],
    out: Path,
    *,
    init_from: Optional[Path] = None,
    init_extractor_from: Optional[Path] = None,
    lstm_hidden_size: int = 256,
    net_arch: Optional[list[int]] = None,
    features_dim: int = 256,
    embedding_dim: int = 8,
    epochs: int = 6,
    seq_batch_size: int = 16,
    lr: float = 3e-4,
    value_coef: float = 1.0,
    max_grad_norm: float = 1.0,
    holdout_fraction: float = 0.15,
    weight_decay: float = 0.0,
    value_label_smoothing: float = 0.0,
    device: str = "cpu",
    seed: int = 0,
    export_onnx: Optional[Path] = None,
) -> dict:
    """Distill chronological self-play shard trajectories into a MaskableRecurrentPPO checkpoint."""
    from digimon_gym.agents.azero_trainer import split_games
    from digimon_gym.agents.belief_contract import require_same_information
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.agents.maskable_recurrent import (
        MaskableMlpLstmPolicy,
        MaskableRecurrentPPO,
    )
    from sb3_contrib import MaskablePPO

    torch.manual_seed(seed)
    np.random.seed(seed)
    ds = load_sequence_shards(shard_dirs)
    if not ds.sequences:
        raise ValueError("No trajectory sequences found in shards")

    arch = list(net_arch) if net_arch is not None else [256, 256]
    contract = current_contract(ds.observation_profile)
    information = require_same_information(ds.manifests)

    class _SpacesEnv(gym.Env):
        observation_space = gym.spaces.Box(
            -10.0, 20001.0, (contract["tensor_size"],), np.float32
        )
        action_space = gym.spaces.Discrete(contract["action_space_size"])

        def reset(self, *, seed=None, options=None):
            return np.zeros((contract["tensor_size"],), dtype=np.float32), {}

        def step(self, action):
            return np.zeros((contract["tensor_size"],), dtype=np.float32), 0.0, True, False, {}

    if init_from is not None and init_extractor_from is not None:
        raise ValueError("init_from and init_extractor_from are mutually exclusive")

    if init_from is not None:
        model = MaskableRecurrentPPO.load(str(init_from), device=device)
        validate_checkpoint(model, ds.observation_profile)
    else:
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy,
            _SpacesEnv(),
            n_steps=64,
            batch_size=64,
            learning_rate=lr,
            seed=seed,
            device=device,
            policy_kwargs=dict(
                lstm_hidden_size=lstm_hidden_size,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=arch, vf=arch),
                features_extractor_class=CardEmbeddingExtractor,
                features_extractor_kwargs=dict(
                    features_dim=features_dim,
                    embedding_dim=embedding_dim,
                    tensor_profile_id=ds.observation_profile,
                ),
            ),
        )
        model.digimon_contract = contract
        if init_extractor_from is not None:
            try:
                src = MaskablePPO.load(str(init_extractor_from), device="cpu")
            except Exception:
                src = MaskableRecurrentPPO.load(str(init_extractor_from), device="cpu")
            validate_checkpoint(src, ds.observation_profile)
            src_sd = src.policy.state_dict()
            dst_sd = model.policy.state_dict()
            transferred = {
                k: v
                for k, v in src_sd.items()
                if "features_extractor" in k and k in dst_sd and dst_sd[k].shape == v.shape
            }
            if not transferred:
                raise ValueError(
                    f"init_extractor_from={init_extractor_from}: no matching features_extractor weights"
                )
            dst_sd.update(transferred)
            model.policy.load_state_dict(dst_sd)
            del src

    policy = model.policy
    seq_game_ids = np.asarray([s.game_id for s in ds.sequences], dtype=np.int64)
    if holdout_fraction > 0.0 and len(np.unique(seq_game_ids)) >= 2:
        heldout_mask = split_games(seq_game_ids, holdout_fraction, seed + 7919)
    else:
        heldout_mask = np.zeros(len(ds.sequences), dtype=bool)

    fit_seqs = [s for s, is_h in zip(ds.sequences, heldout_mask) if not is_h]
    heldout_seqs = [s for s, is_h in zip(ds.sequences, heldout_mask) if is_h]
    if not fit_seqs:
        raise ValueError("No sequences in fit split")

    fit_term_z = np.concatenate([s.z[s.terminated] for s in fit_seqs if s.terminated.any()])
    if len(fit_term_z) == 0:
        raise ValueError("No completed-game value targets in fit split")
    constant_z = float(fit_term_z.mean())
    fit_pol_rows = sum(int((s.policy_weight > 0).sum()) for s in fit_seqs)
    if fit_pol_rows == 0:
        raise ValueError("No policy-target rows (policy_weight > 0) in fit split")

    initial_heldout = (
        _evaluate_sequences(
            policy, heldout_seqs, ds.action_dim, seq_batch_size, constant_z, value_coef
        )
        if heldout_seqs
        else None
    )

    if weight_decay < 0.0:
        raise ValueError(f"weight_decay must be >= 0.0, got {weight_decay}")
    if not (0.0 <= value_label_smoothing < 1.0):
        raise ValueError(f"value_label_smoothing must lie in [0, 1), got {value_label_smoothing}")
    if weight_decay > 0.0:
        optimizer = torch.optim.AdamW(policy.parameters(), lr=lr, weight_decay=weight_decay)
    else:
        optimizer = torch.optim.Adam(policy.parameters(), lr=lr)
    rng = np.random.default_rng(seed)
    dev = policy.device
    best = {"epoch": None, "loss": float("inf"), "state": None}
    history: list[dict] = []

    for epoch in range(epochs):
        policy.set_training_mode(True)
        order = rng.permutation(len(fit_seqs))
        # Sort within coarse buckets or shuffle directly; shuffling preserves unbiased gradients.
        pol_sum = val_sum = 0.0
        batches = 0
        for start in range(0, len(order), seq_batch_size):
            batch = [fit_seqs[int(i)] for i in order[start : start + seq_batch_size]]
            obs_t, pi_t, legal_t, seq_mask_t, weight_t, z_t, term_t = _collate_batch(
                batch, ds.action_dim, dev
            )
            if value_label_smoothing > 0.0:
                z_t = z_t * (1.0 - value_label_smoothing)
            logits, values = _forward_sequences(policy, obs_t)
            masked_logits = logits.masked_fill(~legal_t, torch.finfo(logits.dtype).min)
            logp = torch.log_softmax(masked_logits, dim=-1)
            step_ce = -(pi_t * logp.masked_fill(~legal_t, 0.0)).sum(dim=-1)
            eff_w = weight_t * seq_mask_t.float()
            w_total = eff_w.sum()
            policy_loss = (
                (step_ce * eff_w).sum() / w_total if float(w_total) > 0.0 else step_ce.sum() * 0.0
            )
            val_mask = seq_mask_t & term_t
            value_loss = (
                F.mse_loss(values[val_mask], z_t[val_mask])
                if val_mask.any()
                else values.sum() * 0.0
            )
            loss = policy_loss + value_coef * value_loss

            optimizer.zero_grad()
            loss.backward()
            if max_grad_norm > 0.0:
                torch.nn.utils.clip_grad_norm_(policy.parameters(), max_grad_norm)
            optimizer.step()

            pol_sum += float(policy_loss.detach().item())
            val_sum += float(value_loss.detach().item())
            batches += 1

        epoch_stats: dict = {
            "epoch": epoch,
            "policy_loss": pol_sum / max(batches, 1),
            "value_loss": val_sum / max(batches, 1),
        }
        if heldout_seqs:
            heldout_stats = _evaluate_sequences(
                policy, heldout_seqs, ds.action_dim, seq_batch_size, constant_z, value_coef
            )
            epoch_stats["heldout"] = heldout_stats
            if heldout_stats["loss"] < best["loss"]:
                best = {
                    "epoch": epoch,
                    "loss": heldout_stats["loss"],
                    "state": copy.deepcopy(policy.state_dict()),
                }
        history.append(epoch_stats)
        print(
            f"  [lstm_distill] epoch {epoch}: policy_loss={epoch_stats['policy_loss']:.4f} "
            f"value_loss={epoch_stats['value_loss']:.4f}"
            + (
                f" | heldout loss={epoch_stats['heldout']['loss']:.4f} "
                f"(ce={epoch_stats['heldout']['policy_ce']:.4f}, "
                f"mse={epoch_stats['heldout']['value_mse']:.4f} vs const {epoch_stats['heldout']['constant_mse']:.4f})"
                if "heldout" in epoch_stats
                else ""
            ),
            flush=True,
        )

    selection = None
    if best["state"] is not None:
        policy.load_state_dict(best["state"])
        selection = {
            "metric": "heldout policy_ce + value_coef * value_mse",
            "best_epoch": best["epoch"],
            "epochs_run": epochs,
            "initial": initial_heldout,
            "selected": history[best["epoch"]]["heldout"],
        }
        print(f"  [lstm_distill] restored best held-out epoch {best['epoch']} of {epochs}", flush=True)

    policy.set_training_mode(False)
    model.digimon_contract = contract
    model.digimon_value_semantics = "terminal_outcome"
    model.digimon_information = information
    model.digimon_skip_forced_steps = True

    out = Path(out)
    out.parent.mkdir(parents=True, exist_ok=True)
    model.save(str(out))
    actual_hidden = int(policy.lstm_actor.hidden_size)
    actual_features = int(policy.features_extractor.features_dim)
    actual_embed = int(getattr(policy.features_extractor, "embedding", None).embedding_dim) if hasattr(policy.features_extractor, "embedding") else embedding_dim
    meta_path = _write_lstm_meta_sidecar(
        out,
        contract,
        lstm_hidden_size=actual_hidden,
        net_arch=arch,
        features_dim=actual_features,
        embedding_dim=actual_embed,
        information=information,
    )

    if export_onnx is not None:
        repo_root = Path(__file__).resolve().parents[3]
        cmd = [
            sys.executable,
            str(repo_root / "code" / "tools" / "export_onnx.py"),
            "--type",
            "lstm",
            "--input",
            str(out),
            "--output",
            str(export_onnx),
            "--tensor-profile",
            ds.observation_profile,
        ]
        res = subprocess.run(cmd, capture_output=True, text=True)
        if res.returncode != 0:
            raise RuntimeError(f"export_onnx failed:\n{res.stdout}\n{res.stderr}")

    return {
        "schema": "lstm-distill-summary-v1",
        "sequences": len(ds.sequences),
        "fit_sequences": len(fit_seqs),
        "heldout_sequences": len(heldout_seqs),
        "total_rows": ds.total_rows,
        "excluded_games": sum(len(q["games"]) for q in ds.exclusions),
        "weight_decay": float(weight_decay),
        "value_label_smoothing": float(value_label_smoothing),
        "epochs": history,
        "selection": selection,
        "out": str(out),
        "meta_sidecar": str(meta_path),
        "export_onnx": str(export_onnx) if export_onnx else None,
        "model_contract": validate_checkpoint(model, ds.observation_profile),
    }


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--shards", type=Path, nargs="+", required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--init-from", type=Path, default=None)
    p.add_argument("--init-extractor-from", type=Path, default=None)
    p.add_argument("--lstm-hidden-size", type=int, default=256)
    p.add_argument("--net-arch", type=str, default="256,256")
    p.add_argument("--features-dim", type=int, default=256)
    p.add_argument("--embedding-dim", type=int, default=8)
    p.add_argument("--epochs", type=int, default=6)
    p.add_argument("--seq-batch-size", type=int, default=16)
    p.add_argument("--lr", type=float, default=3e-4)
    p.add_argument("--value-coef", type=float, default=1.0)
    p.add_argument("--max-grad-norm", type=float, default=1.0)
    p.add_argument("--holdout-fraction", type=float, default=0.15)
    p.add_argument("--weight-decay", type=float, default=0.0)
    p.add_argument("--value-label-smoothing", type=float, default=0.0)
    p.add_argument("--device", default="cpu")
    p.add_argument("--seed", type=int, default=0)
    p.add_argument("--export-onnx", type=Path, default=None)
    p.add_argument("--summary-out", type=Path, default=None)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    arch = [int(x) for x in args.net_arch.split(",") if x.strip()]
    summary = distill_lstm(
        shard_dirs=list(args.shards),
        out=args.out,
        init_from=args.init_from,
        init_extractor_from=args.init_extractor_from,
        lstm_hidden_size=args.lstm_hidden_size,
        net_arch=arch,
        features_dim=args.features_dim,
        embedding_dim=args.embedding_dim,
        epochs=args.epochs,
        seq_batch_size=args.seq_batch_size,
        lr=args.lr,
        value_coef=args.value_coef,
        max_grad_norm=args.max_grad_norm,
        holdout_fraction=args.holdout_fraction,
        weight_decay=args.weight_decay,
        value_label_smoothing=args.value_label_smoothing,
        device=args.device,
        seed=args.seed,
        export_onnx=args.export_onnx,
    )
    if args.summary_out is not None:
        args.summary_out.parent.mkdir(parents=True, exist_ok=True)
        args.summary_out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
