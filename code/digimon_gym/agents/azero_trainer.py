"""AlphaZero policy+value trainer over self-play shards.

`add-determinized-search` task 3.2 (design D9/D10). Consumes the
``selfplay-shards-v1`` directories written by ``digimon-engine-cli
selfplay`` (obs dense, π sparse as offsets/actions/visits, z ±1/0) and
trains an SB3 ``MaskablePPO`` checkpoint's policy+value networks with the
AlphaZero loss:

    loss = CE(normalized π, masked log-softmax(logits)) + c · MSE(value, z)

The model stays an ordinary SB3 ``.zip`` on purpose: the whole downstream
toolchain — ``code/tools/export_onnx.py`` (task-0.2 value-head export),
anchored eval, the champion registry, winprob — consumes it unchanged, so
one ``--export-onnx`` hop produces the next generation's driver input.

Sliding-window replay buffer (D10): pass multiple ``--shards`` directories
(the last N generations); rows are pooled and shuffled together.

Shards and checkpoints must carry the same live seven-field model contract.
``terminated.npy`` distinguishes terminal draws from safety-cap truncations;
only completed rows contribute to value loss. ``calibrate_value`` fits the
critic-specific layers with a whole-game validation split while freezing the
actor and shared features. Its marker records terminal-outcome target semantics,
not a claim of statistically calibrated win probabilities.

Sparse-π detail worth knowing: the driver enumerates **every root-legal
action** in a row's sparse entries (0-visit edges included), so a row's
action list doubles as the legality mask at that decision — the masked
softmax needs no separate mask array.

CLI:
    python -m digimon_gym.agents.azero_trainer \
        --init-from models/starter_pool_single_v1/final.zip \
        --shards runs/selfplay/gen0 [runs/selfplay/gen-1 ...] \
        --out models/azero/gen1.zip \
        [--epochs 4] [--batch-size 512] [--lr 2e-4] [--value-coef 1.0] \
        [--export-onnx models/azero/gen1.onnx]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from model_contract import validate_checkpoint, validate_contract

SCHEMA = "selfplay-shards-v1"


@dataclass
class ShardDataset:
    """All rows from one or more shard directories, pooled."""

    obs: np.ndarray  # (N, obs_dim) f32
    offsets: np.ndarray  # (N+1,) i64 into actions/visits
    actions: np.ndarray  # (nnz,) i32
    visits: np.ndarray  # (nnz,) f32 raw root visit counts
    z: np.ndarray  # (N,) f32 in [-1, +1]
    action_dim: int
    observation_profile: str
    terminated: np.ndarray
    game_ids: np.ndarray
    # (N,) f32 per-row policy-loss weight: 1 for a full search, 0 for a cheap
    # value-only search (playout-cap randomization) or forced PendingSelection.
    policy_weight: np.ndarray
    manifests: list[dict] = field(default_factory=list)
    exclusions: list[dict] = field(default_factory=list)
    avoid_offsets: np.ndarray = field(default_factory=lambda: np.zeros(1, dtype=np.int64))
    avoid_actions: np.ndarray = field(default_factory=lambda: np.zeros(0, dtype=np.int32))
    forced_selection: np.ndarray = field(default_factory=lambda: np.zeros(0, dtype=np.uint8))
    chain_anchor_idx: np.ndarray = field(default_factory=lambda: np.full(0, -1, dtype=np.int64))
    # Own the temporary backing file for as long as the dataset is alive.
    _observation_storage: object | None = field(default=None, repr=False)

    def __len__(self) -> int:
        return self.obs.shape[0]


def _filter_shard_dataset_rows(ds: ShardDataset, keep_mask: np.ndarray) -> ShardDataset:
    """Filter a ShardDataset by a boolean row mask, rebasing sparse offsets and chain anchors."""
    keep = np.asarray(keep_mask, dtype=bool)
    n_orig = len(ds)
    if keep.shape != (n_orig,):
        raise ValueError(f"keep_mask shape {keep.shape} != ({n_orig},)")
    if keep.all():
        return ds

    old_to_new = np.full(n_orig, -1, dtype=np.int64)
    n_new = int(keep.sum())
    old_to_new[keep] = np.arange(n_new, dtype=np.int64)

    lengths = np.diff(ds.offsets)
    edges = np.repeat(keep, lengths)
    new_act = ds.actions[edges].astype(np.int32, copy=False)
    new_vis = ds.visits[edges].astype(np.float32, copy=False)
    new_off = np.concatenate(([0], np.cumsum(lengths[keep], dtype=np.int64)))

    av_lengths = np.diff(ds.avoid_offsets)
    av_edges = np.repeat(keep, av_lengths)
    new_av_act = ds.avoid_actions[av_edges].astype(np.int32, copy=False)
    new_av_off = np.concatenate(([0], np.cumsum(av_lengths[keep], dtype=np.int64)))

    anchor_kept = ds.chain_anchor_idx[keep]
    new_anchor = np.where(anchor_kept >= 0, old_to_new[np.maximum(anchor_kept, 0)], -1).astype(np.int64)

    obs_storage = tempfile.TemporaryFile(prefix="digimon-replay-sub-")
    if n_new > 0:
        obs_storage.write(np.asarray(ds.obs[keep], dtype=np.float32).tobytes(order="C"))
        obs_storage.flush()
        new_obs = np.memmap(obs_storage, dtype=np.float32, mode="r+", shape=(n_new, ds.obs.shape[1]))
    else:
        new_obs = np.empty((0, ds.obs.shape[1]), dtype=np.float32)

    return ShardDataset(
        obs=new_obs,
        offsets=new_off,
        actions=new_act,
        visits=new_vis,
        z=ds.z[keep].astype(np.float32, copy=False),
        action_dim=ds.action_dim,
        observation_profile=ds.observation_profile,
        terminated=ds.terminated[keep].astype(bool, copy=False),
        game_ids=ds.game_ids[keep].astype(np.int64, copy=False),
        policy_weight=ds.policy_weight[keep].astype(np.float32, copy=False),
        manifests=list(ds.manifests),
        exclusions=list(ds.exclusions),
        avoid_offsets=new_av_off,
        avoid_actions=new_av_act,
        forced_selection=ds.forced_selection[keep].astype(np.uint8, copy=False),
        chain_anchor_idx=new_anchor,
        _observation_storage=obs_storage,
    )


def _subsample_extra_dataset(
    extra_ds: ShardDataset,
    max_rows: int,
    seed: int,
) -> ShardDataset:
    """Deterministically subsample whole games from extra_ds down to <= max_rows."""
    if len(extra_ds) <= max_rows:
        return extra_ds
    rng = np.random.default_rng(seed)
    unique_games = np.unique(extra_ds.game_ids)
    perm_games = rng.permutation(unique_games)

    selected_games: list[int] = []
    selected_rows = 0
    for gid in perm_games.tolist():
        cnt = int(np.sum(extra_ds.game_ids == gid))
        if selected_rows + cnt <= max_rows:
            selected_games.append(int(gid))
            selected_rows += cnt

    if selected_games:
        keep_mask = np.isin(extra_ds.game_ids, selected_games)
    else:
        first_gid = int(perm_games[0])
        game_rows = np.flatnonzero(extra_ds.game_ids == first_gid)[:max_rows]
        keep_mask = np.zeros(len(extra_ds), dtype=bool)
        keep_mask[game_rows] = True

    return _filter_shard_dataset_rows(extra_ds, keep_mask)


def _concat_shard_datasets(primary_ds: ShardDataset, extra_ds: ShardDataset) -> ShardDataset:
    """Pool primary_ds with (already capped) extra_ds while preserving primary manifests."""
    if extra_ds.obs.shape[1] != primary_ds.obs.shape[1] or extra_ds.action_dim != primary_ds.action_dim:
        raise ValueError("extra_shards dimensions disagree with primary shards")
    if extra_ds.observation_profile != primary_ds.observation_profile:
        raise ValueError(
            f"extra_shards observation_profile {extra_ds.observation_profile!r} "
            f"!= primary {primary_ds.observation_profile!r}"
        )
    if len(extra_ds) == 0:
        return primary_ds

    n_primary = len(primary_ds)
    n_total = n_primary + len(extra_ds)
    obs_dim = primary_ds.obs.shape[1]

    obs_storage = tempfile.TemporaryFile(prefix="digimon-replay-pooled-")
    if n_primary > 0:
        obs_storage.write(np.asarray(primary_ds.obs, dtype=np.float32).tobytes(order="C"))
    obs_storage.write(np.asarray(extra_ds.obs, dtype=np.float32).tobytes(order="C"))
    obs_storage.flush()
    pooled_obs = np.memmap(obs_storage, dtype=np.float32, mode="r+", shape=(n_total, obs_dim))

    act_base = len(primary_ds.actions)
    pooled_offsets = np.concatenate(
        [primary_ds.offsets, extra_ds.offsets[1:].astype(np.int64) + act_base]
    )
    av_base = len(primary_ds.avoid_actions)
    pooled_av_offsets = np.concatenate(
        [primary_ds.avoid_offsets, extra_ds.avoid_offsets[1:].astype(np.int64) + av_base]
    )
    game_shift = (int(primary_ds.game_ids.max()) + 1) if n_primary > 0 else 0
    extra_anchor_rebased = np.where(
        extra_ds.chain_anchor_idx >= 0,
        extra_ds.chain_anchor_idx + n_primary,
        -1,
    ).astype(np.int64)

    return ShardDataset(
        obs=pooled_obs,
        offsets=pooled_offsets,
        actions=np.concatenate([primary_ds.actions, extra_ds.actions]).astype(np.int32),
        visits=np.concatenate([primary_ds.visits, extra_ds.visits]).astype(np.float32),
        z=np.concatenate([primary_ds.z, extra_ds.z]).astype(np.float32),
        action_dim=primary_ds.action_dim,
        observation_profile=primary_ds.observation_profile,
        terminated=np.concatenate([primary_ds.terminated, extra_ds.terminated]).astype(bool),
        game_ids=np.concatenate([primary_ds.game_ids, extra_ds.game_ids + game_shift]).astype(np.int64),
        policy_weight=np.concatenate([primary_ds.policy_weight, extra_ds.policy_weight]).astype(np.float32),
        manifests=list(primary_ds.manifests),
        exclusions=list(primary_ds.exclusions) + list(extra_ds.exclusions),
        avoid_offsets=pooled_av_offsets,
        avoid_actions=np.concatenate([primary_ds.avoid_actions, extra_ds.avoid_actions]).astype(np.int32),
        forced_selection=np.concatenate([primary_ds.forced_selection, extra_ds.forced_selection]).astype(np.uint8),
        chain_anchor_idx=np.concatenate([primary_ds.chain_anchor_idx, extra_anchor_rebased]).astype(np.int64),
        _observation_storage=obs_storage,
    )


def _load_shards_single_pool(dirs: list[Path], *, enforce_same_info: bool = True) -> ShardDataset:
    off_parts, act_parts, vis_parts, z_parts, end_parts, game_parts = [], [], [], [], [], []
    weight_parts = []
    av_off_parts, av_act_parts = [], []
    forced_parts, anchor_parts = [], []
    observation_storage = tempfile.TemporaryFile(prefix="digimon-replay-")
    observation_rows = 0
    manifests: list[dict] = []
    exclusions: list[dict] = []
    obs_dim = action_dim = None
    profile = None
    game_base = 0
    for d in dirs:
        manifest_path = d / "manifest.json"
        manifest = json.loads(manifest_path.read_text())
        if manifest.get("evaluation_only"):
            raise ValueError("policy-only evaluation cannot provide search training targets")
        quarantine_path = d / "training_exclusions.json"
        quarantine = None
        excluded_ids = set()
        seen_ids = set()
        excluded_rows = 0
        if quarantine_path.exists():
            quarantine = json.loads(quarantine_path.read_text())
            entries = quarantine.get("games")
            if (quarantine.get("schema") != "selfplay-training-exclusions-v1"
                    or quarantine.get("manifest_sha256") != hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                    or not isinstance(entries, list) or not entries):
                raise ValueError(f"{quarantine_path}: invalid exclusion schema or manifest hash")
            for entry in entries:
                if (not isinstance(entry, dict) or type(entry.get("game_id")) is not int
                        or entry["game_id"] < 0 or not isinstance(entry.get("reason"), str)
                        or not entry["reason"].strip() or entry["game_id"] in excluded_ids):
                    raise ValueError(f"{quarantine_path}: invalid or duplicate game exclusion")
                excluded_ids.add(entry["game_id"])
        if manifest.get("schema") != SCHEMA:
            raise ValueError(f"{manifest_path}: schema {manifest.get('schema')!r}, expected {SCHEMA!r}")
        contract = validate_contract(manifest.get("model_contract"), manifest["observation_profile"])
        if contract["observation_profile"] != manifest["observation_profile"]:
            raise ValueError(f"{manifest_path}: observation_profile differs from contract")
        if contract["tensor_size"] != manifest["obs_dim"] or contract["action_space_size"] != manifest["action_dim"]:
            raise ValueError(f"{manifest_path}: dimensions differ from model contract")
        if manifest.get("value_target") != "terminal_outcome":
            raise ValueError(f"{manifest_path}: value_target must be terminal_outcome")
        if obs_dim is None:
            obs_dim = int(manifest["obs_dim"])
            action_dim = int(manifest["action_dim"])
            profile = manifest["observation_profile"]
        else:
            if int(manifest["obs_dim"]) != obs_dim or int(manifest["action_dim"]) != action_dim:
                raise ValueError(f"{manifest_path}: dims disagree with earlier shard dirs")
            if manifest["observation_profile"] != profile:
                raise ValueError(
                    f"{manifest_path}: observation_profile {manifest['observation_profile']!r} "
                    f"!= {profile!r} — refusing to mix profiles in one buffer"
                )
        from digimon_gym.agents.belief_contract import information_contract, require_same_information
        if enforce_same_info:
            require_same_information(manifests + [manifest])
        else:
            information_contract(manifest)
        manifests.append(manifest)
        max_game = -1
        for shard in manifest["shards"]:
            s = d / shard["dir"]
            obs = np.load(s / "obs.npy")
            off = np.load(s / "policy_offsets.npy")
            act = np.load(s / "policy_actions.npy")
            vis = np.load(s / "policy_visits.npy")
            z = np.load(s / "z.npy")
            terminated = np.load(s / "terminated.npy")
            games = np.load(s / "game_id.npy")
            rows = obs.shape[0]
            if rows != shard["rows"] or off.shape[0] != rows + 1 or z.shape[0] != rows or terminated.shape != (rows,) or games.shape != (rows,):
                raise ValueError(f"{s}: row bookkeeping inconsistent")
            if not np.isfinite(z).all() or np.any(z < -1.0) or np.any(z > 1.0) or not np.all(np.isin(terminated, [0, 1])):
                raise ValueError(f"{s}: invalid terminal targets")
            if off[0] != 0 or off[-1] != len(act) or len(act) != len(vis) or not np.all(np.diff(off) > 0):
                raise ValueError(f"{s}: invalid sparse policy offsets")
            if obs.shape[1] != manifest["obs_dim"] or not np.isfinite(obs).all() or not np.isfinite(vis).all() or np.any(vis < 0) or np.any(act < 0) or np.any(act >= manifest["action_dim"]):
                raise ValueError(f"{s}: invalid observations or policy targets")
            weight_path = s / "policy_weight.npy"
            if weight_path.exists():
                weight = np.load(weight_path)
                if weight.shape != (rows,) or not np.isfinite(weight).all() or np.any(weight < 0):
                    raise ValueError(f"{s}: invalid policy_weight array")
            else:
                weight = np.ones(rows, np.float32)

            forced_path = s / "forced_selection.npy"
            if forced_path.exists():
                forced = np.load(forced_path)
                if forced.shape != (rows,) or not np.all(np.isin(forced, [0, 1])):
                    raise ValueError(f"{s}: invalid forced_selection array")
                forced = forced.astype(np.uint8, copy=False)
                weight = np.where(forced != 0, 0.0, weight).astype(np.float32, copy=False)
            else:
                forced = np.zeros(rows, dtype=np.uint8)

            anchor_path = s / "chain_anchor_idx.npy"
            if anchor_path.exists():
                anchor = np.load(anchor_path)
                if anchor.shape != (rows,) or not np.issubdtype(anchor.dtype, np.integer):
                    raise ValueError(f"{s}: invalid chain_anchor_idx array")
                anchor = anchor.astype(np.int64, copy=True)
                if np.any((anchor != -1) & ((anchor < 0) | (anchor >= rows))):
                    raise ValueError(f"{s}: chain_anchor_idx out of bounds")
            else:
                anchor = np.full(rows, -1, dtype=np.int64)

            av_off_path = s / "avoid_offsets.npy"
            av_act_path = s / "avoid_actions.npy"
            if av_off_path.exists() or av_act_path.exists():
                if not (av_off_path.exists() and av_act_path.exists()):
                    raise ValueError(f"{s}: avoid_offsets.npy and avoid_actions.npy must be provided together")
                av_off = np.load(av_off_path)
                av_act = np.load(av_act_path)
                if (
                    av_off.shape != (rows + 1,)
                    or av_off[0] != 0
                    or av_off[-1] != len(av_act)
                    or np.any(np.diff(av_off) < 0)
                    or (len(av_act) > 0 and (np.any(av_act < 0) or np.any(av_act >= manifest["action_dim"])))
                ):
                    raise ValueError(f"{s}: invalid sparse avoid targets")
            else:
                av_off = np.zeros(rows + 1, dtype=np.int64)
                av_act = np.zeros(0, dtype=np.int32)

            # Preserve rebasing across chunks even when their highest game ID
            # is excluded. Filtering is local to this manifest, before rebasing.
            max_game = max(max_game, int(games.max(initial=-1)))
            if excluded_ids:
                seen_ids.update(int(g) for g in np.unique(games))
                keep = ~np.isin(games, list(excluded_ids))
                excluded_rows += int((~keep).sum())
                old_to_new = np.full(rows, -1, dtype=np.int64)
                old_to_new[keep] = np.arange(int(keep.sum()), dtype=np.int64)
                lengths = np.diff(off)
                edges = np.repeat(keep, lengths)
                obs, z, terminated, games = obs[keep], z[keep], terminated[keep], games[keep]
                weight = weight[keep]
                forced = forced[keep]
                anchor_kept = anchor[keep]
                anchor = np.where(anchor_kept >= 0, old_to_new[np.maximum(anchor_kept, 0)], -1)
                act, vis = act[edges], vis[edges]
                off = np.concatenate(([0], np.cumsum(lengths[keep], dtype=np.int64)))

                av_lengths = np.diff(av_off)
                av_edges = np.repeat(keep, av_lengths)
                av_act = av_act[av_edges]
                av_off = np.concatenate(([0], np.cumsum(av_lengths[keep], dtype=np.int64)))

            # Rebase this shard's offsets onto the pooled nnz axis. The
            # first shard keeps its leading 0; later shards drop theirs
            # (it duplicates the previous shard's final offset).
            base = sum(a.shape[0] for a in act_parts)
            off_parts.append((off if not off_parts else off[1:]).astype(np.int64) + base)
            av_base = sum(a.shape[0] for a in av_act_parts)
            av_off_parts.append((av_off if not av_off_parts else av_off[1:]).astype(np.int64) + av_base)
            row_base = observation_rows
            # Stream each validated shard once. Retaining every dense array and
            # then concatenating it doubles peak RAM for multi-generation replay.
            # The mapping preserves row order/float bits and supports the same
            # shuffled minibatch indexing; the OS can evict unused disk pages.
            observation_storage.write(obs.astype(np.float32, copy=False).tobytes(order="C"))
            observation_rows += len(obs)
            act_parts.append(act)
            vis_parts.append(vis.astype(np.float32, copy=False))
            z_parts.append(z.astype(np.float32, copy=False))
            end_parts.append(terminated.astype(bool))
            game_parts.append(games.astype(np.int64) + game_base)
            weight_parts.append(weight.astype(np.float32, copy=False))
            av_act_parts.append(av_act.astype(np.int32, copy=False))
            forced_parts.append(forced.astype(np.uint8, copy=False))
            anchor_parts.append(np.where(anchor >= 0, anchor + row_base, -1).astype(np.int64))
        if excluded_ids - seen_ids:
            raise ValueError(f"{quarantine_path}: exclusion refers to absent game IDs {sorted(excluded_ids - seen_ids)}")
        if quarantine is not None:
            exclusions.append({"directory": str(d), "manifest_sha256": quarantine["manifest_sha256"],
                               "games": quarantine["games"], "excluded_rows": excluded_rows})
        game_base += max_game + 1
    if obs_dim is None:
        raise ValueError("no shard directories given")

    offsets = np.concatenate(off_parts)
    observation_storage.flush()
    observations = (np.memmap(observation_storage, dtype=np.float32, mode="r+",
                             shape=(observation_rows, obs_dim)) if observation_rows else
                    np.empty((0, obs_dim), dtype=np.float32))
    return ShardDataset(
        obs=observations,
        offsets=offsets,
        actions=np.concatenate(act_parts).astype(np.int32),
        visits=np.concatenate(vis_parts),
        z=np.concatenate(z_parts),
        action_dim=action_dim,
        observation_profile=profile,
        terminated=np.concatenate(end_parts),
        game_ids=np.concatenate(game_parts),
        policy_weight=np.concatenate(weight_parts),
        manifests=manifests,
        exclusions=exclusions,
        avoid_offsets=np.concatenate(av_off_parts),
        avoid_actions=np.concatenate(av_act_parts).astype(np.int32),
        forced_selection=np.concatenate(forced_parts).astype(np.uint8) if forced_parts else np.zeros(0, dtype=np.uint8),
        chain_anchor_idx=np.concatenate(anchor_parts).astype(np.int64) if anchor_parts else np.zeros(0, dtype=np.int64),
        _observation_storage=observation_storage,
    )


def load_shards(
    dirs: list[Path],
    extra_shards: list[Path] | None = None,
    *,
    max_extra_shard_share: float = 0.10,
    seed: int = 0,
) -> ShardDataset:
    """Load validated shards, honoring manifest-bound per-game quarantine files and optional capped extra_shards."""
    if not (0.0 < max_extra_shard_share < 1.0):
        raise ValueError(f"max_extra_shard_share must lie in (0, 1), got {max_extra_shard_share}")
    primary_ds = _load_shards_single_pool(list(dirs), enforce_same_info=True)
    extra_list = [Path(p) for p in (extra_shards or [])]
    if not extra_list:
        return primary_ds
    extra_ds = _load_shards_single_pool(extra_list, enforce_same_info=False)
    n_primary = len(primary_ds)
    if n_primary > 0 and len(extra_ds) > 0:
        max_rows = max(1, int(n_primary * max_extra_shard_share / (1.0 - max_extra_shard_share)))
        if len(extra_ds) > max_rows:
            extra_ds = _subsample_extra_dataset(extra_ds, max_rows=max_rows, seed=seed)
    return _concat_shard_datasets(primary_ds, extra_ds)


def _avoid_margin_loss(
    logits,
    pi_np: np.ndarray,
    ds: ShardDataset,
    idx: np.ndarray,
    avoid_margin: float,
):
    """Contrastive hinge loss `relu(margin - (logit_target - logit_avoid))` on rows with avoid actions."""
    import torch

    if len(ds.avoid_actions) == 0:
        return logits.sum() * 0.0

    row_indices: list[int] = []
    target_actions: list[int] = []
    avoid_acts_flat: list[int] = []
    edge_weights: list[float] = []
    for b_row, ds_i in enumerate(idx):
        av_s, av_e = int(ds.avoid_offsets[ds_i]), int(ds.avoid_offsets[ds_i + 1])
        if av_e <= av_s:
            continue
        w = float(ds.policy_weight[ds_i])
        if w <= 0.0:
            continue
        tgt_a = int(np.argmax(pi_np[b_row]))
        av_slice = [int(a) for a in ds.avoid_actions[av_s:av_e] if int(a) != tgt_a]
        if not av_slice:
            continue
        per_edge_w = w / len(av_slice)
        for av_a in av_slice:
            row_indices.append(b_row)
            target_actions.append(tgt_a)
            avoid_acts_flat.append(av_a)
            edge_weights.append(per_edge_w)

    if not row_indices:
        return logits.sum() * 0.0

    dev = logits.device
    r_t = torch.as_tensor(row_indices, dtype=torch.long, device=dev)
    tgt_t = torch.as_tensor(target_actions, dtype=torch.long, device=dev)
    av_t = torch.as_tensor(avoid_acts_flat, dtype=torch.long, device=dev)
    ew_t = torch.as_tensor(edge_weights, dtype=logits.dtype, device=dev)
    logit_target = logits[r_t, tgt_t]
    logit_avoid = logits[r_t, av_t]
    hinge = torch.relu(float(avoid_margin) - (logit_target - logit_avoid))
    w_sum = ew_t.sum()
    return (hinge * ew_t).sum() / w_sum if float(w_sum) > 0.0 else logits.sum() * 0.0


def _dense_targets(
    ds: ShardDataset, idx: np.ndarray
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Densify a batch: (π [B, A] normalized, legality mask [B, A], z [B]).

    π rows with zero total visits (possible only at degenerate sim budgets)
    fall back to uniform over the row's legal actions.
    """
    B, A = len(idx), ds.action_dim
    pi = np.zeros((B, A), dtype=np.float32)
    mask = np.zeros((B, A), dtype=bool)
    for row, i in enumerate(idx):
        s, e = ds.offsets[i], ds.offsets[i + 1]
        acts = ds.actions[s:e]
        vis = ds.visits[s:e]
        mask[row, acts] = True
        total = vis.sum()
        if total > 0:
            pi[row, acts] = vis / total
        else:
            pi[row, acts] = 1.0 / len(acts)
    return pi, mask, ds.z[idx]


def _forward(policy, obs_t):
    """Raw (logits, value) through the SB3 policy's own nets.

    Shared or split feature extractors are both supported. The value is the
    raw ``value_net`` output; callers apply the checkpoint's activation.
    """
    if policy.share_features_extractor:
        feats = policy.extract_features(obs_t)
        latent_pi, latent_vf = policy.mlp_extractor(feats)
    else:
        pi_f, vf_f = policy.extract_features(obs_t)
        latent_pi = policy.mlp_extractor.forward_actor(pi_f)
        latent_vf = policy.mlp_extractor.forward_critic(vf_f)
    return policy.action_net(latent_pi), policy.value_net(latent_vf).squeeze(-1)


def split_games(game_ids: np.ndarray, fraction: float, seed: int) -> np.ndarray:
    """Boolean row mask of a held-out set made of WHOLE games.

    Positions from one game are near-copies, so splitting by row would leak
    the validation set into training. ``game_ids`` are already rebased to be
    unique across shard directories by ``load_shards``.
    """
    if not 0.0 < fraction < 1.0:
        raise ValueError("holdout fraction must be in (0, 1)")
    groups = np.unique(game_ids)
    if len(groups) < 2:
        raise ValueError("a held-out split needs at least two games")
    rng = np.random.default_rng(seed)
    rng.shuffle(groups)
    count = min(len(groups) - 1, max(1, int(round(len(groups) * fraction))))
    return np.isin(game_ids, groups[:count])


def heldout_metrics(policy, ds: ShardDataset, rows: np.ndarray, activation: str,
                    constant: float, batch_size: int, value_coef: float) -> dict:
    """Held-out policy CE, value MSE, and the always-guess-the-mean baseline.

    ``constant`` is the mean terminal outcome of the FIT rows, so the baseline
    is the best a position-blind value head could do on this split. A value
    head whose MSE does not beat it has learned nothing that generalises.
    """
    import torch
    from digimon_gym.agents import value_activation

    dev = policy.device
    ce_sum = weight_sum = 0.0
    squared: list[float] = []
    baseline: list[float] = []
    predicted: list[float] = []
    was_training = policy.training
    policy.set_training_mode(False)
    with torch.no_grad():
        for start in range(0, len(rows), batch_size):
            idx = rows[start:start + batch_size]
            pi_np, mask_np, z_np = _dense_targets(ds, idx)
            logits, raw = _forward(policy, torch.as_tensor(ds.obs[idx], device=dev))
            values = value_activation.apply(activation, raw).clamp(-1.0, 1.0)
            mask_t = torch.as_tensor(mask_np, device=dev)
            logp = torch.log_softmax(logits.masked_fill(~mask_t, torch.finfo(logits.dtype).min), dim=-1)
            row_ce = -(torch.as_tensor(pi_np, device=dev) * logp.masked_fill(~mask_t, 0.0)).sum(dim=-1)
            weight = torch.as_tensor(ds.policy_weight[idx], device=dev)
            ce_sum += float((row_ce * weight).sum())
            weight_sum += float(weight.sum())
            terminal = ds.terminated[idx]
            z = z_np[terminal]
            predicted.extend(values.cpu().numpy()[terminal].tolist())
            squared.extend(((values.cpu().numpy()[terminal] - z) ** 2).tolist())
            baseline.extend(((constant - z) ** 2).tolist())
    policy.set_training_mode(was_training)
    policy_ce = ce_sum / weight_sum if weight_sum > 0 else None
    value_mse = float(np.mean(squared)) if squared else None
    report = {
        "policy_ce": policy_ce,
        "value_mse": value_mse,
        "constant_mse": float(np.mean(baseline)) if baseline else None,
        "value_rows": len(squared),
        # Search scores unvisited moves at Q=0, so the value LEVEL changes how
        # widely it explores; track it next to accuracy across generations.
        "mean_value": float(np.mean(predicted)) if predicted else None,
    }
    report["value_beats_constant"] = (None if value_mse is None
                                      else value_mse < report["constant_mse"])
    report["loss"] = ((policy_ce or 0.0) + value_coef * (value_mse or 0.0))
    return report


def train(
    init_from: Path,
    shard_dirs: list[Path],
    out: Path,
    epochs: int = 4,
    batch_size: int = 512,
    lr: float = 2e-4,
    value_coef: float = 1.0,
    device: str = "auto",
    seed: int = 0,
    export_onnx: Path | None = None,
    holdout_fraction: float = 0.0,
    value_activation_name: str | None = None,
    weight_decay: float = 0.0,
    value_label_smoothing: float = 0.0,
    avoid_margin_coef: float | None = None,
    avoid_margin: float = 1.0,
    consistency_coef: float = 0.25,
    extra_shards: list[Path] | None = None,
    max_extra_shard_share: float = 0.10,
) -> dict:
    """Run the AZ update and save the new checkpoint. Returns a summary dict.

    ``holdout_fraction`` > 0 sets aside that share of whole games: they reach
    neither loss, are scored after every epoch, and the epoch with the lowest
    held-out loss (policy CE + value_coef * value MSE) is the one saved
    (early stopping by restoration). 0 keeps the legacy fit-everything path.

    ``value_activation_name`` (``identity``/``tanh``) records the value-head
    activation on the checkpoint; ``None`` keeps whatever the init carries.
    """
    import copy

    import torch
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents import value_activation

    if weight_decay < 0.0:
        raise ValueError(f"weight_decay must be >= 0.0, got {weight_decay}")
    if not (0.0 <= value_label_smoothing < 1.0):
        raise ValueError(f"value_label_smoothing must lie in [0, 1), got {value_label_smoothing}")
    if avoid_margin_coef is not None and avoid_margin_coef < 0.0:
        raise ValueError(f"avoid_margin_coef must be >= 0.0, got {avoid_margin_coef}")
    if avoid_margin < 0.0:
        raise ValueError(f"avoid_margin must be >= 0.0, got {avoid_margin}")
    if consistency_coef < 0.0:
        raise ValueError(f"consistency_coef must be >= 0.0, got {consistency_coef}")
    if not (0.0 < max_extra_shard_share < 1.0):
        raise ValueError(f"max_extra_shard_share must lie in (0, 1), got {max_extra_shard_share}")

    torch.manual_seed(seed)
    primary_ds = _load_shards_single_pool(list(shard_dirs), enforce_same_info=True)
    extra_list = [Path(p) for p in (extra_shards or [])]
    extra_rows_loaded = 0
    extra_rows_used = 0
    if extra_list:
        raw_extra_ds = _load_shards_single_pool(extra_list, enforce_same_info=False)
        extra_rows_loaded = len(raw_extra_ds)
        if len(primary_ds) > 0 and extra_rows_loaded > 0:
            max_rows = max(1, int(len(primary_ds) * max_extra_shard_share / (1.0 - max_extra_shard_share)))
            if extra_rows_loaded > max_rows:
                raw_extra_ds = _subsample_extra_dataset(raw_extra_ds, max_rows=max_rows, seed=seed)
        extra_rows_used = len(raw_extra_ds)
        ds = _concat_shard_datasets(primary_ds, raw_extra_ds)
    else:
        ds = primary_ds

    eff_avoid_coef = (
        (0.2 if len(ds.avoid_actions) > 0 else 0.0)
        if avoid_margin_coef is None
        else float(avoid_margin_coef)
    )
    if not ds.terminated.any():
        raise ValueError("No completed-game value targets; refusing to label an untrained critic terminal_outcome")
    model = MaskablePPO.load(str(init_from), device=device)
    validate_checkpoint(model, ds.observation_profile)
    activation = value_activation.resolve(model, value_activation_name)
    policy = model.policy
    if int(np.prod(policy.observation_space.shape)) != ds.obs.shape[1]:
        raise ValueError(
            f"model obs dim {policy.observation_space.shape} != shard obs_dim {ds.obs.shape[1]} "
            f"(shard profile: {ds.observation_profile})"
        )
    if int(policy.action_space.n) != ds.action_dim:
        raise ValueError(f"model action dim {policy.action_space.n} != shard {ds.action_dim}")

    truncated = int((~ds.terminated).sum())
    if holdout_fraction:
        heldout = split_games(ds.game_ids, holdout_fraction, seed + 7919)
    else:
        heldout = np.zeros(len(ds), dtype=bool)
    fit_rows = np.flatnonzero(~heldout)
    heldout_rows = np.flatnonzero(heldout)
    if not ds.terminated[fit_rows].any():
        raise ValueError("No completed-game value targets in the fit split")
    constant = float(ds.z[fit_rows][ds.terminated[fit_rows]].mean())
    policy_rows = int((ds.policy_weight[fit_rows] > 0).sum())
    if policy_rows == 0:
        raise ValueError("No policy-target rows (every row has policy_weight 0); refusing to train the actor on nothing")
    print(
        f"azero_trainer: {len(ds)} rows from {len(shard_dirs)} dir(s)"
        + (f" (+{extra_rows_used}/{extra_rows_loaded} extra rows)" if extra_list else "")
        + f", obs_dim {ds.obs.shape[1]}, profile {ds.observation_profile}, "
        f"{truncated} truncated-game rows (z=0), "
        f"{len(ds) - policy_rows} cheap/forced value-only rows excluded from the policy loss, "
        f"{sum(q['excluded_rows'] for q in ds.exclusions)} quarantined rows excluded from both losses, "
        f"{len(heldout_rows)} held-out rows from {len(np.unique(ds.game_ids[heldout]))} whole games, "
        f"value activation {activation}",
        flush=True,
    )

    def score() -> dict | None:
        if not len(heldout_rows):
            return None
        return heldout_metrics(policy, ds, heldout_rows, activation, constant, batch_size, value_coef)

    initial = score()
    if initial is not None:
        print(f"  held-out before fitting: {initial}", flush=True)
    best = {"epoch": None, "loss": float("inf"), "state": None}

    policy.set_training_mode(True)
    if weight_decay > 0.0:
        optimizer = torch.optim.AdamW(policy.parameters(), lr=lr, weight_decay=weight_decay)
    else:
        optimizer = torch.optim.Adam(policy.parameters(), lr=lr)
    rng = np.random.default_rng(seed)
    dev = policy.device

    history: list[dict] = []
    for epoch in range(epochs):
        order = rng.permutation(fit_rows)
        pol_sum = val_sum = av_sum = cc_sum = 0.0
        batches = 0
        for start in range(0, len(order), batch_size):
            idx = order[start : start + batch_size]
            pi_np, mask_np, z_np = _dense_targets(ds, idx)
            obs_t = torch.as_tensor(ds.obs[idx], device=dev)
            pi_t = torch.as_tensor(pi_np, device=dev)
            mask_t = torch.as_tensor(mask_np, device=dev)
            z_t = torch.as_tensor(z_np, device=dev)
            if value_label_smoothing > 0.0:
                z_t = z_t * (1.0 - value_label_smoothing)

            logits, raw_values = _forward(policy, obs_t)
            values = value_activation.apply(activation, raw_values)

            # Masked log-softmax — same legality semantics as the driver's
            # evaluator (illegal mass exactly excluded from the CE).
            masked_logits = logits.masked_fill(~mask_t, torch.finfo(logits.dtype).min)
            logp = torch.log_softmax(masked_logits, dim=-1)
            # Cheap / forced-selection (value-only) rows carry policy_weight 0
            # and drop out of the actor loss; the mean runs over policy-target rows only.
            row_ce = -(pi_t * logp.masked_fill(~mask_t, 0.0)).sum(dim=-1)
            weight_t = torch.as_tensor(ds.policy_weight[idx], device=dev)
            weight_total = weight_t.sum()
            policy_loss = (row_ce * weight_t).sum() / weight_total if weight_total > 0 else row_ce.sum() * 0.0
            terminal = torch.as_tensor(ds.terminated[idx], device=dev)
            value_loss = torch.nn.functional.mse_loss(values[terminal], z_t[terminal]) if terminal.any() else values.sum() * 0.0
            loss = policy_loss + value_coef * value_loss

            if consistency_coef > 0.0 and len(ds.chain_anchor_idx) > 0:
                anchors = ds.chain_anchor_idx[idx]
                has_anchor = (anchors >= 0) & (anchors < len(ds))
                if np.any(has_anchor):
                    mid_pos = np.flatnonzero(has_anchor)
                    anchor_ds_idx = anchors[mid_pos]
                    with torch.no_grad():
                        obs_anchor_t = torch.as_tensor(ds.obs[anchor_ds_idx], device=dev)
                        _, raw_anchor = _forward(policy, obs_anchor_t)
                        v_anchor = value_activation.apply(activation, raw_anchor).detach()
                    mid_pos_t = torch.as_tensor(mid_pos, dtype=torch.long, device=dev)
                    v_mid = values[mid_pos_t]
                    consistency_loss = torch.mean((v_mid - v_anchor) ** 2)
                    loss = loss + float(consistency_coef) * consistency_loss
                else:
                    consistency_loss = values.sum() * 0.0
            else:
                consistency_loss = values.sum() * 0.0

            if eff_avoid_coef > 0.0:
                avoid_loss = _avoid_margin_loss(logits, pi_np, ds, idx, avoid_margin)
                loss = loss + eff_avoid_coef * avoid_loss
            else:
                avoid_loss = logits.sum() * 0.0

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            pol_sum += float(policy_loss.detach())
            val_sum += float(value_loss.detach())
            av_sum += float(avoid_loss.detach())
            cc_sum += float(consistency_loss.detach())
            batches += 1
        epoch_stats = {
            "epoch": epoch,
            "policy_loss": pol_sum / max(batches, 1),
            "value_loss": val_sum / max(batches, 1),
            "consistency_loss": cc_sum / max(batches, 1),
        }
        if eff_avoid_coef > 0.0:
            epoch_stats["avoid_loss"] = av_sum / max(batches, 1)
        heldout_stats = score()
        if heldout_stats is not None:
            epoch_stats["heldout"] = heldout_stats
            if heldout_stats["loss"] < best["loss"]:
                best = {"epoch": epoch, "loss": heldout_stats["loss"],
                        "state": copy.deepcopy(policy.state_dict())}
        history.append(epoch_stats)
        print(
            f"  epoch {epoch}: policy_loss {epoch_stats['policy_loss']:.4f} "
            f"value_loss {epoch_stats['value_loss']:.4f} "
            f"consistency_loss {epoch_stats['consistency_loss']:.4f}"
            + (f" avoid_loss {epoch_stats['avoid_loss']:.4f}" if "avoid_loss" in epoch_stats else "")
            + (f" | held-out value_mse {heldout_stats['value_mse']} vs constant "
               f"{heldout_stats['constant_mse']}, policy_ce {heldout_stats['policy_ce']}"
               if heldout_stats else ""),
            flush=True,
        )

    selection = None
    if best["state"] is not None:
        policy.load_state_dict(best["state"])
        selection = {"metric": "heldout policy_ce + value_coef * value_mse",
                     "best_epoch": best["epoch"], "epochs_run": epochs,
                     "initial": initial, "selected": history[best["epoch"]]["heldout"]}
        print(f"  restored best held-out epoch {best['epoch']} of {epochs}", flush=True)

    policy.set_training_mode(False)
    if value_coef > 0:
        model.digimon_value_semantics = "terminal_outcome"
    out.parent.mkdir(parents=True, exist_ok=True)
    from digimon_gym.agents.belief_contract import require_same_information
    model.digimon_information = require_same_information(ds.manifests)
    model.save(str(out))
    print(f"saved {out}", flush=True)

    if export_onnx is not None:
        repo_root = Path(__file__).resolve().parents[3]
        cmd = [
            sys.executable,
            str(repo_root / "code" / "tools" / "export_onnx.py"),
            "--type",
            "mlp",
            "--input",
            str(out),
            "--output",
            str(export_onnx),
            "--tensor-profile",
            ds.observation_profile,
        ]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(f"export_onnx failed:\n{result.stdout}\n{result.stderr}")
        print(f"exported {export_onnx}", flush=True)

    return {
        "information": getattr(model,"digimon_information", {"regime":"legacy_exact"}),
        "rows": len(ds),
        "extra_rows_loaded": int(extra_rows_loaded),
        "extra_rows_used": int(extra_rows_used),
        "forced_selection_rows": int((ds.forced_selection != 0).sum()),
        "consistency_coef": float(consistency_coef),
        "truncated_rows": truncated,
        "excluded_rows": sum(q["excluded_rows"] for q in ds.exclusions),
        "excluded_games": sum(len(q["games"]) for q in ds.exclusions),
        "training_exclusions": ds.exclusions,
        "epochs": history,
        "holdout_fraction": holdout_fraction,
        "heldout_rows": int(len(heldout_rows)),
        "heldout_games": int(len(np.unique(ds.game_ids[heldout]))),
        "selection": selection,
        "value_activation": activation,
        "out": str(out),
        "model_contract": validate_checkpoint(model, ds.observation_profile),
        "export_onnx": str(export_onnx) if export_onnx else None,
    }


def calibrate_value(init_from: Path, shard_dirs: list[Path], out: Path,
                    epochs: int = 8, batch_size: int = 256, lr: float = 1e-3,
                    device: str = "auto", seed: int = 0,
                    value_activation_name: str | None = None,
                    select_best: bool = False,
                    extra_shards: list[Path] | None = None,
                    max_extra_shard_share: float = 0.10) -> dict:
    """Refit only the critic on completed games, keeping the actor bitwise fixed.

    Split by complete game, never by individual decision. This changes the
    training target to terminal outcome; held-out error measures calibration
    quality rather than asserting that fitting alone proves calibration.
    ``select_best`` restores the critic from the epoch with the lowest
    held-out MSE instead of keeping the last one.
    """
    import copy

    import torch
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents import value_activation
    torch.manual_seed(seed)
    ds = load_shards(
        shard_dirs,
        extra_shards=extra_shards,
        max_extra_shard_share=max_extra_shard_share,
        seed=seed,
    )
    model = MaskablePPO.load(str(init_from), device=device)
    validate_checkpoint(model, ds.observation_profile)
    activation = value_activation.resolve(model, value_activation_name)
    policy = model.policy
    policy.set_training_mode(False)
    rng = np.random.default_rng(seed)
    groups = np.unique(ds.game_ids[ds.terminated])
    if len(groups) < 2:
        raise ValueError("Value calibration needs at least two completed games for a held-out split")
    rng.shuffle(groups)
    heldout_games = groups[:max(1, len(groups) // 5)]
    heldout = ds.terminated & np.isin(ds.game_ids, heldout_games)
    fit = np.flatnonzero(ds.terminated & ~heldout)
    dev = policy.device
    # Shared feature extraction stays frozen; only critic-specific layers
    # change, so policy priors are exactly preserved during calibration.
    params = list(policy.mlp_extractor.value_net.parameters()) + list(policy.value_net.parameters())
    optimizer = torch.optim.Adam(params, lr=lr)
    torch.nn.init.zeros_(policy.value_net.weight)
    torch.nn.init.zeros_(policy.value_net.bias)

    def predict(idx):
        obs = torch.as_tensor(ds.obs[idx], device=dev)
        with torch.no_grad():
            features = policy.extract_features(obs)
            if not policy.share_features_extractor:
                features = features[1]
        raw = policy.value_net(policy.mlp_extractor.forward_critic(features)).squeeze(-1)
        return value_activation.apply(activation, raw)

    def metrics(indices):
        squared = []
        with torch.no_grad():
            for start in range(0, len(indices), batch_size):
                idx = indices[start:start + batch_size]
                prediction = predict(idx).clamp(-1, 1)
                squared.extend(((prediction - torch.as_tensor(ds.z[idx], device=dev)) ** 2).cpu().tolist())
        return float(np.mean(squared))

    heldout_idx = np.flatnonzero(heldout)
    constant = float(ds.z[fit].mean())
    constant_mse = float(np.mean((constant - ds.z[heldout_idx]) ** 2))
    critic_params = lambda: {k: copy.deepcopy(v) for k, v in policy.state_dict().items()
                             if k.startswith(("mlp_extractor.value_net.", "value_net."))}
    best = {"epoch": None, "mse": float("inf"), "state": None}
    history = []
    for epoch in range(epochs):
        order = rng.permutation(fit)
        for start in range(0, len(order), batch_size):
            idx = order[start:start + batch_size]
            loss = torch.nn.functional.mse_loss(predict(idx), torch.as_tensor(ds.z[idx], device=dev))
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
        history.append({"epoch": epoch, "fit_mse": metrics(fit), "heldout_mse": metrics(heldout_idx),
                        "heldout_constant_mse": constant_mse})
        if history[-1]["heldout_mse"] < best["mse"]:
            best = {"epoch": epoch, "mse": history[-1]["heldout_mse"], "state": critic_params()}
        print(f"value calibration: {history[-1]}", flush=True)
    if select_best and best["state"] is not None:
        policy.load_state_dict(best["state"], strict=False)
    model.digimon_value_semantics = "terminal_outcome"
    out.parent.mkdir(parents=True, exist_ok=True)
    model.save(str(out))
    report = {"fit_rows": len(fit), "heldout_rows": int(heldout.sum()),
              "excluded_truncated_rows": int((~ds.terminated).sum()),
              "completed_games": len(groups), "history": history,
              "heldout_constant_mse": constant_mse, "value_activation": activation,
              "selected_epoch": best["epoch"] if select_best else epochs - 1,
              "value_semantics": "terminal_outcome", "actor_frozen": True}
    out.with_suffix(".calibration.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    return report


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--init-from", type=Path, required=True, help="SB3 .zip to warm-start from")
    p.add_argument(
        "--shards",
        type=Path,
        nargs="+",
        required=True,
        help="selfplay-shards-v1 directories (pass several for a sliding window)",
    )
    p.add_argument(
        "--extra-shards",
        type=Path,
        nargs="*",
        default=[],
        help="auxiliary puzzle/curriculum shard directories (capped by --max-extra-shard-share)",
    )
    p.add_argument(
        "--max-extra-shard-share",
        type=float,
        default=0.10,
        help="maximum row share in (0, 1) permitted for --extra-shards relative to total rows",
    )
    p.add_argument("--out", type=Path, required=True, help="output SB3 .zip")
    p.add_argument("--epochs", type=int, default=4)
    p.add_argument("--batch-size", type=int, default=512)
    p.add_argument("--lr", type=float, default=2e-4)
    p.add_argument("--value-coef", type=float, default=1.0)
    p.add_argument(
        "--consistency-coef",
        type=float,
        default=0.25,
        help="continuation-consistent critic loss coefficient on PendingSelection chains (0.0 disables)",
    )
    p.add_argument("--device", default="auto")
    p.add_argument("--seed", type=int, default=0)
    p.add_argument(
        "--export-onnx",
        type=Path,
        default=None,
        help="also export the task-0.2 policy+value ONNX for the next driver generation",
    )
    p.add_argument("--holdout-fraction", type=float, default=0.0,
                   help="share of WHOLE games held out from both losses; the best held-out epoch is saved")
    p.add_argument("--value-activation", choices=("identity", "tanh"), default=None,
                   help="value-head output activation (default: keep the init checkpoint's)")
    p.add_argument("--weight-decay", type=float, default=0.0,
                   help="AdamW weight decay (0.0 uses plain Adam)")
    p.add_argument("--value-label-smoothing", type=float, default=0.0,
                   help="shrink training terminal z targets by (1 - eps); held-out MSE uses raw z")
    p.add_argument("--avoid-margin-coef", type=float, default=None,
                   help="contrastive hinge weight on avoid_actions.npy (default: 0.2 when present, else 0.0)")
    p.add_argument("--avoid-margin", type=float, default=1.0,
                   help="logit margin for relu(margin - (logit_target - logit_avoid))")
    p.add_argument("--calibrate-value-only", action="store_true",
                   help="refit only the critic (actor bitwise frozen) on a whole-game split, keeping "
                        "the best held-out epoch; ignores --value-coef and --holdout-fraction")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.calibrate_value_only:
        calibrate_value(args.init_from, args.shards, args.out, epochs=args.epochs,
                        batch_size=args.batch_size, lr=args.lr, device=args.device, seed=args.seed,
                        value_activation_name=args.value_activation, select_best=True,
                        extra_shards=list(args.extra_shards),
                        max_extra_shard_share=args.max_extra_shard_share)
        if args.export_onnx is not None:
            from digimon_gym.tensor_profiles import get_tensor_profile
            from tools.export_onnx import export_mlp
            profile = load_shards(args.shards).observation_profile
            export_mlp(str(args.out), str(args.export_onnx), get_tensor_profile(profile))
        return 0
    train(
        init_from=args.init_from,
        shard_dirs=args.shards,
        out=args.out,
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        value_coef=args.value_coef,
        device=args.device,
        seed=args.seed,
        export_onnx=args.export_onnx,
        holdout_fraction=args.holdout_fraction,
        value_activation_name=args.value_activation,
        weight_decay=args.weight_decay,
        value_label_smoothing=args.value_label_smoothing,
        avoid_margin_coef=args.avoid_margin_coef,
        avoid_margin=args.avoid_margin,
        consistency_coef=args.consistency_coef,
        extra_shards=list(args.extra_shards),
        max_extra_shard_share=args.max_extra_shard_share,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
