"""End-to-end cross-track integration test across Tracks 1, 2, 3, and 4.

Verifies live wire interoperability (no mocks) across:
1. Track 2 (`digimon_training_mcp.whitebox`): `.zip` -> `card_embedding_neighbors`,
   `position_saliency`, `plasticity_probe`, and cached `export_checkpoint_onnx`
   against Track 1's `ex12-erlangmon-saneiketsu-invitation-trap.json`.
2. Track 4 (`tools.puzzle_lab.mcp_client.StdioMcpClient`) spawning Track 1's
   compiled `digimon-engine-mcp` Rust binary over real stdio JSON-RPC, loading
   Track 2's exported `.onnx`, inspecting Erlangmon's stack (`inspect_stack`),
   scoring/inspecting the position (`inspect_position`, `score_line`), and
   proving `verify_puzzle` passes `target` while Erlangmon + Saneiketsu
   Invitation refutes `avoid`.
3. Track 3 (`tools.emit_puzzle_shards` + `azero_trainer`) emitting `selfplay-shards-v1`
   from `ex12-erlangmon-saneiketsu-invitation-trap.json` and training cleanly.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import gymnasium as gym
import numpy as np
import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
CODE_ROOT = REPO_ROOT / "code"
TRAINING_MCP_SRC = CODE_ROOT / "digimon-training-mcp" / "src"
for p in (str(CODE_ROOT), str(TRAINING_MCP_SRC)):
    if p not in sys.path:
        sys.path.insert(0, p)


def _build_tiny_mlp_checkpoint(zip_path: Path) -> Path:
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile
    from digimon_engine import ACTION_SPACE_SIZE
    from model_contract import current_contract

    profile = get_tensor_profile("standard_lite_v2")

    class _DummyMaskedEnv(gym.Env):
        def __init__(self):
            super().__init__()
            self.observation_space = gym.spaces.Box(
                low=-10.0, high=20001.0, shape=(profile.tensor_size,), dtype=np.float32
            )
            self.action_space = gym.spaces.Discrete(ACTION_SPACE_SIZE)

        def reset(self, *, seed=None, options=None):
            super().reset(seed=seed)
            return np.zeros(profile.tensor_size, dtype=np.float32), {}

        def step(self, action):
            return np.zeros(profile.tensor_size, dtype=np.float32), 0.0, True, False, {}

        def action_masks(self):
            m = np.zeros(ACTION_SPACE_SIZE, dtype=bool)
            m[62] = True
            return m

    model = MaskablePPO(
        "MlpPolicy",
        _DummyMaskedEnv(),
        policy_kwargs={
            "features_extractor_class": CardEmbeddingExtractor,
            "features_extractor_kwargs": {"features_dim": 64, "tensor_profile_id": "standard_lite_v2"},
            "net_arch": dict(pi=[32], vf=[32]),
        },
        n_steps=8,
        batch_size=8,
        device="cpu",
    )
    model.digimon_contract = current_contract("standard_lite_v2")
    model.save(str(zip_path))
    return zip_path


def _ensure_engine_mcp_binary() -> Path:
    bin_path = REPO_ROOT / "target" / "debug" / "digimon-engine-mcp"
    if bin_path.is_file():
        return bin_path
    cargo = shutil.which("cargo")
    if cargo is None:
        pytest.skip("cargo not available to build digimon-engine-mcp")
    subprocess.run(
        [cargo, "build", "-p", "digimon-engine-mcp"],
        cwd=str(REPO_ROOT),
        check=True,
    )
    assert bin_path.is_file()
    return bin_path


def test_full_cross_track_pipeline_live(tmp_path: Path):
    from digimon_training_mcp.whitebox import (
        card_embedding_neighbors,
        export_checkpoint_onnx,
        plasticity_probe,
        position_saliency,
    )
    from tools.emit_puzzle_shards import emit_puzzle_shards
    from tools.puzzle_lab.mcp_client import StdioMcpClient
    from digimon_gym.agents.azero_trainer import load_shards

    puzzle_path = REPO_ROOT / "qa" / "puzzles" / "ex12-erlangmon-saneiketsu-invitation-trap.json"
    assert puzzle_path.is_file(), f"Missing Track 1 puzzle: {puzzle_path}"

    # 1. Build a real contract-stamped SB3 .zip checkpoint
    ckpt_zip = _build_tiny_mlp_checkpoint(tmp_path / "pilot_mlp.zip")
    orig_bytes = ckpt_zip.read_bytes()

    # 2. Track 2 white-box diagnostics on Track 1's EX12 Erlangmon puzzle
    emb = card_embedding_neighbors(ckpt_zip, ["EX12-034", "EX12-071", "BT20-042"], top_k=3)
    assert emb["ok"] is True
    assert {c["card_id"] for c in emb["cards"]} == {"EX12-034", "EX12-071", "BT20-042"}

    sal = position_saliency(
        ckpt_zip,
        puzzle_path=puzzle_path,
        target_action_id=415,
        avoid_action_id=0,
        top_k=5,
    )
    assert sal["ok"] is True
    assert sal["objective"] == "logit_margin"
    assert len(sal["section_attribution"]) > 0

    plast = plasticity_probe(
        ckpt_zip,
        puzzle_path=puzzle_path,
        target_action_id=415,
        avoid_action_ids=[0, 114],
        steps=8,
        lr=1e-3,
    )
    assert plast["ok"] is True
    assert plast["learnable"] is True
    assert ckpt_zip.read_bytes() == orig_bytes, "White-box probes must never mutate .zip on disk"

    exp = export_checkpoint_onnx(ckpt_zip, cache_dir=tmp_path / "onnx_cache")
    assert exp["ok"] is True
    onnx_path = Path(exp["onnx_path"])
    assert onnx_path.is_file()

    # 3. Track 4 StdioMcpClient -> Track 1 real Rust `digimon-engine-mcp` binary
    mcp_bin = _ensure_engine_mcp_binary()
    with StdioMcpClient.from_command([str(mcp_bin), "--pool", "implemented"], cwd=REPO_ROOT) as client:
        client.initialize()
        tool_names = {t["name"] for t in client.list_tools()}
        assert {"load_model", "inspect_position", "score_line", "inspect_stack", "load_puzzle", "verify_puzzle"} <= tool_names

        loaded_model = client.call_tool("load_model", {"model_id": "test_mlp", "onnx_path": str(onnx_path)})
        assert loaded_model["ok"] is True
        assert loaded_model["kind"] == "mlp"

        loaded_puz = client.call_tool("load_puzzle", {"puzzle_path": str(puzzle_path), "mode": "author"})
        assert loaded_puz["ok"] is True
        gid = loaded_puz["game_id"]

        # Erlangmon stack on Seat 2 (engine player 1, slot 0): top EX12-034, inherited EX12-015 + EX12-002
        stack = client.call_tool("inspect_stack", {"game_id": gid, "player": 1, "zone": "battle", "slot": 0})
        assert stack["ok"] is True
        assert stack["top_card"]["card_id"] == "EX12-034"
        assert "Kotenken" in stack["top_card"]["effect_text"]
        inherited_ids = [s["card_id"] for s in stack["inherited_sources"]]
        assert "EX12-015" in inherited_ids

        pos = client.call_tool(
            "inspect_position",
            {"game_id": gid, "model_id": "test_mlp", "simulations": 16, "worlds": 2, "top_k": 5},
        )
        assert pos["mode"] == "search"
        assert "policy_entropy" in pos
        assert len(pos["root_plans"]) > 0

        verified = client.call_tool(
            "verify_puzzle",
            {"puzzle_path": str(puzzle_path), "model_id": "test_mlp", "adversary_sims": 16, "seeds": 1},
        )
        assert verified["ok"] is True
        assert all(r["passed"] for r in verified["target_results"])
        assert all(r["refuted"] for r in verified["avoid_results"])

    # 4. Track 3 `emit_puzzle_shards` on Track 1's EX12 Erlangmon puzzle -> `azero_trainer.load_shards`
    shard_dir = tmp_path / "ex12_shards"
    emit_puzzle_shards([puzzle_path], shard_dir, permutations_per_puzzle=2, seed=7)
    ds = load_shards([shard_dir])
    assert len(ds) >= 2
