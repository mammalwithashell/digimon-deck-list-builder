"""Rebuild an SB3 ``MaskablePPO`` checkpoint from a task-0.2 policy+value ONNX export.

``azero_trainer.py`` warm-starts from an SB3 ``.zip``; ``export_onnx.py`` goes the
other way. When only the ONNX survives (the cloud box that held the ``.zip`` is
gone), this reconstructs the checkpoint: build a fresh ``MaskablePPO`` with the
``CardEmbeddingExtractor`` architecture implied by the ONNX initializer shapes,
copy every weight across, stamp the live model contract, and verify the torch
policy reproduces the ONNX ``logits`` / ``value`` on random and real observations
before saving. Verification failure refuses to write.

Only the MLP export shape is supported (shared features extractor, two-layer
tanh heads). The ONNX graph's embedded contract must match the installed Rust
binding's layout for the profile — this never re-targets a stale model.

CLI:
    python code/tools/sb3_from_onnx.py --onnx model.onnx --out parent.zip
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parents[2]
for entry in (REPO_ROOT / "code", REPO_ROOT / "code" / "digimon_gym" / "agents"):
    if str(entry) not in sys.path:
        sys.path.insert(0, str(entry))

ONNX_TO_SB3 = {
    "features_extractor.": "features_extractor.",
    "mlp_extractor_policy.": "mlp_extractor.policy_net.",
    "mlp_extractor_value.": "mlp_extractor.value_net.",
    "action_net.": "action_net.",
    "value_net.": "value_net.",
}


def onnx_weights(path: Path) -> tuple[dict[str, np.ndarray], dict]:
    import onnx
    from onnx import numpy_helper

    graph = onnx.load(str(path))
    weights = {t.name: numpy_helper.to_array(t) for t in graph.graph.initializer}
    props = {p.key: p.value for p in graph.metadata_props}
    return weights, props


def infer_architecture(weights: dict[str, np.ndarray]) -> dict:
    emb = weights["features_extractor.embedding.weight"]
    proj = weights["features_extractor.projection.0.weight"]
    pi_layers = sorted(
        int(k.split(".")[1]) for k in weights if k.startswith("mlp_extractor_policy.") and k.endswith(".weight")
    )
    vf_layers = sorted(
        int(k.split(".")[1]) for k in weights if k.startswith("mlp_extractor_value.") and k.endswith(".weight")
    )
    return {
        "vocab_size": int(emb.shape[0]),
        "embedding_dim": int(emb.shape[1]),
        "features_dim": int(proj.shape[0]),
        "pi": [int(weights[f"mlp_extractor_policy.{i}.weight"].shape[0]) for i in pi_layers],
        "vf": [int(weights[f"mlp_extractor_value.{i}.weight"].shape[0]) for i in vf_layers],
    }


def map_state_dict(weights: dict[str, np.ndarray], target_keys: list[str]) -> dict:
    import torch

    mapped = {}
    for name, array in weights.items():
        for prefix, replacement in ONNX_TO_SB3.items():
            if name.startswith(prefix):
                mapped[replacement + name[len(prefix):]] = torch.as_tensor(np.array(array))
                break
        else:
            raise ValueError(f"unmapped ONNX initializer {name!r}")
    # SB3 aliases pi_/vf_ extractors onto the shared one; feed them the same tensors.
    for key in target_keys:
        for alias in ("pi_features_extractor.", "vf_features_extractor."):
            if key.startswith(alias):
                mapped[key] = mapped["features_extractor." + key[len(alias):]]
    missing = sorted(set(target_keys) - set(mapped))
    extra = sorted(set(mapped) - set(target_keys))
    if missing or extra:
        raise ValueError(f"state-dict mismatch: missing {missing}, extra {extra}")
    return mapped


def build(onnx_path: Path, out: Path, verify_games: int, seed: int) -> dict:
    import torch
    from sb3_contrib import MaskablePPO

    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.digimon_gym import DigimonEnv
    from model_contract import current_contract, validate_contract

    weights, props = onnx_weights(onnx_path)
    embedded = json.loads(props["digimon_training_contract"]) if "digimon_training_contract" in props else None
    if embedded is None:
        meta = json.loads(Path(f"{onnx_path}.meta.json").read_text(encoding="utf-8"))
        embedded = {k: meta[k] for k in (
            "observation_profile", "tensor_version", "feature_schema_version", "tensor_size",
            "tensor_layout_hash", "action_space_size", "action_schema_version")}
    contract = validate_contract(embedded)
    profile = contract["observation_profile"]
    arch = infer_architecture(weights)

    env = DigimonEnv(tensor_profile=profile)
    if tuple(env.observation_space.shape) != (contract["tensor_size"],):
        raise ValueError("Live observation space does not match the ONNX contract")
    model = MaskablePPO(
        "MlpPolicy", env, device="cpu", n_steps=64, batch_size=64, seed=seed, verbose=0,
        policy_kwargs=dict(
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=arch["features_dim"], embedding_dim=arch["embedding_dim"],
                vocab_size=arch["vocab_size"], tensor_profile_id=profile),
            net_arch=dict(pi=arch["pi"], vf=arch["vf"]),
        ),
    )
    policy = model.policy
    state = map_state_dict(weights, list(policy.state_dict().keys()))
    for key in ("features_extractor.card_id_indices", "features_extractor.scalar_indices"):
        if not torch.equal(state[key].to(policy.state_dict()[key].dtype), policy.state_dict()[key]):
            raise ValueError(f"{key}: ONNX layout indices differ from the live profile")
    policy.load_state_dict(state)
    policy.eval()
    model.digimon_contract = current_contract(profile)
    value_semantics = props.get("digimon_value_semantics")
    if value_semantics:
        model.digimon_value_semantics = value_semantics

    report = verify(policy, onnx_path, env, verify_games, seed)
    out.parent.mkdir(parents=True, exist_ok=True)
    model.save(str(out))
    reloaded = MaskablePPO.load(str(out), device="cpu")
    from model_contract import validate_checkpoint
    validate_checkpoint(reloaded, profile)
    return {"onnx": str(onnx_path), "out": str(out), "architecture": arch, "contract": contract,
            "value_semantics": value_semantics, "verification": report}


def verify(policy, onnx_path: Path, env, games: int, seed: int) -> dict:
    import onnxruntime as ort
    import torch

    from digimon_gym.digimon_gym import greedy_policy

    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    rng = np.random.default_rng(seed)
    low = np.where(np.isfinite(env.observation_space.low), env.observation_space.low, -1.0)
    high = np.where(np.isfinite(env.observation_space.high), env.observation_space.high, 1.0)
    # Card-id slots span [0, vocab]; the embedding row space is [0, vocab) so keep
    # the random draw strictly inside it (the live tensor never emits the bound).
    random_obs = rng.uniform(low, np.where(high >= 1000, high - 1, high), size=(64, low.shape[0])).astype(np.float32)
    real_obs = []
    for game in range(games):
        obs, _ = env.reset(seed=seed + game)
        done = False
        steps = 0
        while not done and steps < 200:
            real_obs.append(np.asarray(obs, dtype=np.float32).copy())
            obs, _r, terminated, truncated, _ = env.step(greedy_policy(env))
            done = terminated or truncated
            steps += 1
    batches = {"random": random_obs, "real": np.stack(real_obs) if real_obs else random_obs[:0]}
    report = {}
    for name, obs in batches.items():
        if len(obs) == 0:
            continue
        onnx_logits, onnx_value = session.run(["logits", "value"], {"obs": obs})
        with torch.no_grad():
            obs_t = torch.as_tensor(obs)
            feats = policy.extract_features(obs_t)
            latent_pi, latent_vf = policy.mlp_extractor(feats)
            logits = policy.action_net(latent_pi).numpy()
            value = policy.value_net(latent_vf).numpy()
        report[name] = {
            "rows": int(len(obs)),
            "max_abs_logit_diff": float(np.max(np.abs(logits - onnx_logits))),
            "max_abs_value_diff": float(np.max(np.abs(value.reshape(-1) - np.asarray(onnx_value).reshape(-1)))),
            "argmax_agreement": float(np.mean(logits.argmax(-1) == onnx_logits.argmax(-1))),
        }
        if report[name]["max_abs_logit_diff"] > 1e-3 or report[name]["max_abs_value_diff"] > 1e-3:
            raise ValueError(f"reconstructed policy diverges from ONNX on {name} observations: {report[name]}")
    return report


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--onnx", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True, help="SB3 .zip to write")
    p.add_argument("--verify-games", type=int, default=2, help="greedy-vs-greedy games supplying real observations")
    p.add_argument("--seed", type=int, default=0)
    args = p.parse_args(argv)
    report = build(args.onnx, args.out, args.verify_games, args.seed)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
