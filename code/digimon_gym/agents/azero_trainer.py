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
import json
import subprocess
import sys
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
    z: np.ndarray  # (N,) f32 in {-1, 0, +1}
    action_dim: int
    observation_profile: str
    terminated: np.ndarray
    game_ids: np.ndarray
    manifests: list[dict] = field(default_factory=list)

    def __len__(self) -> int:
        return self.obs.shape[0]


def load_shards(dirs: list[Path]) -> ShardDataset:
    """Load + concatenate shard dirs, validating cross-dir consistency."""
    obs_parts, off_parts, act_parts, vis_parts, z_parts, end_parts, game_parts = [], [], [], [], [], [], []
    manifests: list[dict] = []
    obs_dim = action_dim = None
    profile = None
    game_base = 0
    for d in dirs:
        manifest_path = d / "manifest.json"
        manifest = json.loads(manifest_path.read_text())
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
            if not np.all(np.isin(z, [-1., 0., 1.])) or not np.all(np.isin(terminated, [0, 1])):
                raise ValueError(f"{s}: invalid terminal targets")
            if off[0] != 0 or off[-1] != len(act) or len(act) != len(vis) or not np.all(np.diff(off) > 0):
                raise ValueError(f"{s}: invalid sparse policy offsets")
            if obs.shape[1] != manifest["obs_dim"] or not np.isfinite(obs).all() or not np.isfinite(vis).all() or np.any(vis < 0) or np.any(act < 0) or np.any(act >= manifest["action_dim"]):
                raise ValueError(f"{s}: invalid observations or policy targets")
            # Rebase this shard's offsets onto the pooled nnz axis. The
            # first shard keeps its leading 0; later shards drop theirs
            # (it duplicates the previous shard's final offset).
            base = sum(a.shape[0] for a in act_parts)
            off_parts.append((off if not off_parts else off[1:]).astype(np.int64) + base)
            obs_parts.append(obs.astype(np.float32, copy=False))
            act_parts.append(act)
            vis_parts.append(vis.astype(np.float32, copy=False))
            z_parts.append(z.astype(np.float32, copy=False))
            end_parts.append(terminated.astype(bool))
            game_parts.append(games.astype(np.int64) + game_base)
            max_game = max(max_game, int(games.max(initial=-1)))
        game_base += max_game + 1
    if obs_dim is None:
        raise ValueError("no shard directories given")

    offsets = np.concatenate(off_parts)
    return ShardDataset(
        obs=np.concatenate(obs_parts),
        offsets=offsets,
        actions=np.concatenate(act_parts).astype(np.int32),
        visits=np.concatenate(vis_parts),
        z=np.concatenate(z_parts),
        action_dim=action_dim,
        observation_profile=profile,
        terminated=np.concatenate(end_parts),
        game_ids=np.concatenate(game_parts),
        manifests=manifests,
    )


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
) -> dict:
    """Run the AZ update and save the new checkpoint. Returns a summary dict."""
    import torch
    from sb3_contrib import MaskablePPO

    torch.manual_seed(seed)
    ds = load_shards(shard_dirs)
    if not ds.terminated.any():
        raise ValueError("No completed-game value targets; refusing to label an untrained critic terminal_outcome")
    model = MaskablePPO.load(str(init_from), device=device)
    validate_checkpoint(model, ds.observation_profile)
    policy = model.policy
    if int(np.prod(policy.observation_space.shape)) != ds.obs.shape[1]:
        raise ValueError(
            f"model obs dim {policy.observation_space.shape} != shard obs_dim {ds.obs.shape[1]} "
            f"(shard profile: {ds.observation_profile})"
        )
    if int(policy.action_space.n) != ds.action_dim:
        raise ValueError(f"model action dim {policy.action_space.n} != shard {ds.action_dim}")

    truncated = int((~ds.terminated).sum())
    print(
        f"azero_trainer: {len(ds)} rows from {len(shard_dirs)} dir(s), "
        f"obs_dim {ds.obs.shape[1]}, profile {ds.observation_profile}, "
        f"{truncated} truncated-game rows (z=0)",
        flush=True,
    )

    policy.set_training_mode(True)
    optimizer = torch.optim.Adam(policy.parameters(), lr=lr)
    rng = np.random.default_rng(seed)
    dev = policy.device

    history: list[dict] = []
    for epoch in range(epochs):
        order = rng.permutation(len(ds))
        pol_sum = val_sum = 0.0
        batches = 0
        for start in range(0, len(order), batch_size):
            idx = order[start : start + batch_size]
            pi_np, mask_np, z_np = _dense_targets(ds, idx)
            obs_t = torch.as_tensor(ds.obs[idx], device=dev)
            pi_t = torch.as_tensor(pi_np, device=dev)
            mask_t = torch.as_tensor(mask_np, device=dev)
            z_t = torch.as_tensor(z_np, device=dev)

            # Forward through the SB3 policy's own nets (shared or split
            # feature extractors both supported).
            if policy.share_features_extractor:
                feats = policy.extract_features(obs_t)
                latent_pi, latent_vf = policy.mlp_extractor(feats)
            else:
                pi_f, vf_f = policy.extract_features(obs_t)
                latent_pi = policy.mlp_extractor.forward_actor(pi_f)
                latent_vf = policy.mlp_extractor.forward_critic(vf_f)
            logits = policy.action_net(latent_pi)
            values = policy.value_net(latent_vf).squeeze(-1)

            # Masked log-softmax — same legality semantics as the driver's
            # evaluator (illegal mass exactly excluded from the CE).
            masked_logits = logits.masked_fill(~mask_t, torch.finfo(logits.dtype).min)
            logp = torch.log_softmax(masked_logits, dim=-1)
            policy_loss = -(pi_t * logp.masked_fill(~mask_t, 0.0)).sum(dim=-1).mean()
            terminal = torch.as_tensor(ds.terminated[idx], device=dev)
            value_loss = torch.nn.functional.mse_loss(values[terminal], z_t[terminal]) if terminal.any() else values.sum() * 0.0
            loss = policy_loss + value_coef * value_loss

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            pol_sum += float(policy_loss.detach())
            val_sum += float(value_loss.detach())
            batches += 1
        epoch_stats = {
            "epoch": epoch,
            "policy_loss": pol_sum / max(batches, 1),
            "value_loss": val_sum / max(batches, 1),
        }
        history.append(epoch_stats)
        print(
            f"  epoch {epoch}: policy_loss {epoch_stats['policy_loss']:.4f} "
            f"value_loss {epoch_stats['value_loss']:.4f}",
            flush=True,
        )

    policy.set_training_mode(False)
    if value_coef > 0:
        model.digimon_value_semantics = "terminal_outcome"
    out.parent.mkdir(parents=True, exist_ok=True)
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
        "rows": len(ds),
        "truncated_rows": truncated,
        "epochs": history,
        "out": str(out),
        "model_contract": validate_checkpoint(model, ds.observation_profile),
        "export_onnx": str(export_onnx) if export_onnx else None,
    }


def calibrate_value(init_from: Path, shard_dirs: list[Path], out: Path,
                    epochs: int = 8, batch_size: int = 256, lr: float = 1e-3,
                    device: str = "auto", seed: int = 0) -> dict:
    """Refit only the critic on completed games, keeping the actor bitwise fixed.

    Split by complete game, never by individual decision. This changes the
    training target to terminal outcome; held-out error measures calibration
    quality rather than asserting that fitting alone proves calibration.
    """
    import torch
    from sb3_contrib import MaskablePPO
    torch.manual_seed(seed)
    ds = load_shards(shard_dirs)
    model = MaskablePPO.load(str(init_from), device=device)
    validate_checkpoint(model, ds.observation_profile)
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
        return policy.value_net(policy.mlp_extractor.forward_critic(features)).squeeze(-1)

    def metrics(indices):
        squared = []
        with torch.no_grad():
            for start in range(0, len(indices), batch_size):
                idx = indices[start:start + batch_size]
                prediction = predict(idx).clamp(-1, 1)
                squared.extend(((prediction - torch.as_tensor(ds.z[idx], device=dev)) ** 2).cpu().tolist())
        return float(np.mean(squared))

    history = []
    for epoch in range(epochs):
        order = rng.permutation(fit)
        for start in range(0, len(order), batch_size):
            idx = order[start:start + batch_size]
            loss = torch.nn.functional.mse_loss(predict(idx), torch.as_tensor(ds.z[idx], device=dev))
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
        history.append({"epoch": epoch, "fit_mse": metrics(fit), "heldout_mse": metrics(np.flatnonzero(heldout))})
        print(f"value calibration: {history[-1]}", flush=True)
    model.digimon_value_semantics = "terminal_outcome"
    out.parent.mkdir(parents=True, exist_ok=True)
    model.save(str(out))
    report = {"fit_rows": len(fit), "heldout_rows": int(heldout.sum()),
              "excluded_truncated_rows": int((~ds.terminated).sum()),
              "completed_games": len(groups), "history": history,
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
    p.add_argument("--out", type=Path, required=True, help="output SB3 .zip")
    p.add_argument("--epochs", type=int, default=4)
    p.add_argument("--batch-size", type=int, default=512)
    p.add_argument("--lr", type=float, default=2e-4)
    p.add_argument("--value-coef", type=float, default=1.0)
    p.add_argument("--device", default="auto")
    p.add_argument("--seed", type=int, default=0)
    p.add_argument(
        "--export-onnx",
        type=Path,
        default=None,
        help="also export the task-0.2 policy+value ONNX for the next driver generation",
    )
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
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
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
