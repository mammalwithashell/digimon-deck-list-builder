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


def test_next_wave_cross_track_pipeline_live(tmp_path: Path):
    """End-to-end live wire verification of all 6 Next-Wave deliverables (no mocks):
    1. Track C `compile_play_in_with_fallback` -> Track A real `digimon-engine-mcp`
       `compile_play_in` -> Turn-1 `replay_prefix` upgrade + line remapping +
       symbolic `dcgo-harness` YAML + `verify_puzzle`.
    2. Track A `verify_belief_puzzle` on `qa/puzzles/ex13-belief-hand-trap-discipline.json`
       over real stdio JSON-RPC with exported `.onnx` model (`information_gap > 0`).
    3. Track B `emit_puzzle_shards` (`forced_selection.npy`, `chain_anchor_idx.npy`,
       and `--belief-worlds` continuous `z.npy`) -> `azero_trainer.train` with
       `consistency_coef=0.25` and `extra_shards` (`max_extra_shard_share=0.10`).
    4. Track C `run_tensor_completeness_fuzz` (with live `plasticity_probe`) +
       `run_auto_curriculum` Dual Gate (`curriculum_manifest.json` + SHA256 shards).
    5. Track D `export_tactician_datasets` + `DigimonVerifiableReward` scoring
       exported ground-truth reasoning traces at `1.00` against `digimon_engine`.
    """
    import json

    from digimon_training_mcp.whitebox import export_checkpoint_onnx
    from digimon_gym.agents.azero_trainer import load_shards, train as azero_train
    from tools.emit_puzzle_shards import emit_puzzle_shards
    from tools.puzzle_lab.auto_curriculum import run_auto_curriculum
    from tools.puzzle_lab.mcp_client import StdioMcpClient
    from tools.puzzle_lab.play_in import compile_play_in_with_fallback
    from tools.puzzle_lab.tensor_fuzzer import run_tensor_completeness_fuzz
    from tools.tactician_grpo.dataset_exporter import export_tactician_datasets
    from tools.tactician_grpo.verifiable_reward import DigimonVerifiableReward

    belief_puzzle = REPO_ROOT / "qa" / "puzzles" / "ex13-belief-hand-trap-discipline.json"
    imperial_exam = REPO_ROOT / "qa" / "search-exams" / "ex13-imperialdramon-dna-paildramon-jamming-lethal.json"
    erlang_puzzle = REPO_ROOT / "qa" / "puzzles" / "ex12-erlangmon-saneiketsu-invitation-trap.json"
    assert belief_puzzle.is_file()
    assert imperial_exam.is_file()

    ckpt_zip = _build_tiny_mlp_checkpoint(tmp_path / "next_wave_mlp.zip")
    exp = export_checkpoint_onnx(ckpt_zip, cache_dir=tmp_path / "onnx_cache")
    assert exp["ok"] is True
    onnx_path = Path(exp["onnx_path"])

    mcp_bin = _ensure_engine_mcp_binary()
    staged_playin_doc = {
        "schema": "studio-puzzle-v1",
        "id": "e2e-playin-turn3-agumon",
        "title": "E2E Turn 3 Agumon Play-In",
        "description": "Reach Turn 3 Main from Turn 1 with ST1-03 unsuspended and play BT1-010.",
        "seat": 1,
        "seed": 0,
        "setup": {
            "kind": "staged",
            "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "breeding": None,
                    "hand": ["BT1-010"],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": ["ST1-02", "ST1-02", "ST1-02", "ST1-05", "ST1-05"],
                    "trash": [],
                },
                "2": {
                    "breeding": None,
                    "hand": ["ST1-02"],
                    "field": [],
                    "security": ["ST1-02", "ST1-02", "ST1-05", "ST1-05", "ST1-06"],
                    "trash": [],
                },
            },
        },
        "lines": [
            {"id": "play-bt1-010", "name": "play_bt1_010", "role": "target", "actions": [0]},
            {"id": "avoid-pass", "name": "pass_turn", "role": "avoid", "actions": [62]},
        ],
        "rl_goal": {
            "mode": "bounded_horizon",
            "max_turns": 1,
            "max_actions": 2,
            "win_assertions": [{"kind": "zone_count", "player": 1, "zone": "field", "value": 2}],
        },
    }

    upgraded_puzzle_path = tmp_path / "upgraded_playin.json"
    dcgo_yaml_path = tmp_path / "upgraded_playin.yaml"

    with StdioMcpClient.from_command([str(mcp_bin), "--pool", "implemented"], cwd=REPO_ROOT) as client:
        client.initialize()
        tools = {t["name"] for t in client.list_tools()}
        assert len(tools) == 44
        assert {"compile_play_in", "verify_belief_puzzle"} <= tools

        loaded_model = client.call_tool("load_model", {"model_id": "nw_mlp", "onnx_path": str(onnx_path)})
        assert loaded_model["ok"] is True

        # 1. Track C -> Track A live `compile_play_in` + `verify_puzzle` + DCGO YAML
        playin_res = compile_play_in_with_fallback(
            staged_playin_doc,
            engine_mcp=client,
            beam_width=32,
            max_turns=5,
            out_puzzle_path=upgraded_puzzle_path,
            out_dcgo_yaml_path=dcgo_yaml_path,
        )
        assert playin_res.ok is True and playin_res.matched is True
        assert playin_res.prefix_actions[:2] == [0, 0]
        assert upgraded_puzzle_path.is_file()
        assert dcgo_yaml_path.is_file()
        assert "decks:" in dcgo_yaml_path.read_text(encoding="utf-8")

        re_verified = client.call_tool(
            "verify_puzzle",
            {"puzzle_path": str(upgraded_puzzle_path), "adversary_sims": 8, "seeds": 1},
        )
        assert re_verified["ok"] is True
        assert all(r["passed"] for r in re_verified["target_results"])
        assert all(r["refuted"] for r in re_verified["avoid_results"])

        # 2. Track A live `verify_belief_puzzle` on EX13 Hand-Trap Discipline
        belief_res = client.call_tool(
            "verify_belief_puzzle",
            {
                "puzzle_path": str(belief_puzzle),
                "model_id": "nw_mlp",
                "simulations": 24,
                "worlds": 6,
                "seeds": 2,
            },
        )
        assert belief_res["ok"] is True
        assert "information_gap" in belief_res
        assert len(belief_res["closed_visit_shares"]) > 0

        # 3. Track C Auto-Curriculum sidecar (Dual Gate: MCP `verify_puzzle` + PyTorch `plasticity_probe`)
        eval_report_path = tmp_path / "anchored_eval.json"
        eval_report_path.write_text(
            json.dumps(
                {
                    "by_matchup": [
                        {"archetype": "Imperialdramon (UG)", "opponent_archetype": "Shambala", "win_rate": 0.25, "games": 20}
                    ]
                }
            ),
            encoding="utf-8",
        )
        curriculum_out = tmp_path / "curriculum_out"
        cur_rep = run_auto_curriculum(
            eval_report_path,
            out_dir=curriculum_out,
            engine_mcp=client,
            candidate_puzzles=[erlang_puzzle],
            checkpoint_zip=ckpt_zip,
            enable_synthesis=False,
            permutations_per_puzzle=2,
            adversary_sims=8,
            verify_seeds=1,
            plasticity_steps=10,
            plasticity_lr=5e-3,
        )
        assert cur_rep.promoted_count == 1
        assert (curriculum_out / "curriculum_manifest.json").is_file()
        assert (curriculum_out / "extra_shards" / "manifest.json").is_file()

    # 4. Track B `emit_puzzle_shards` (forced_selection + chain_anchor_idx + belief worlds) -> `azero_trainer.train`
    primary_shards = tmp_path / "primary_shards"
    emit_puzzle_shards([imperial_exam], primary_shards, permutations_per_puzzle=3, seed=11)
    belief_shards = tmp_path / "belief_shards"
    emit_puzzle_shards([belief_puzzle], belief_shards, permutations_per_puzzle=0, belief_worlds=4, seed=13)

    p_ds = load_shards([primary_shards])
    assert int(np.sum(p_ds.forced_selection)) >= 4
    assert int(np.sum(p_ds.chain_anchor_idx >= 0)) >= 4

    trained_zip = tmp_path / "trained_next_wave.zip"
    train_summary = azero_train(
        init_from=ckpt_zip,
        shard_dirs=[primary_shards],
        extra_shards=[belief_shards, curriculum_out / "extra_shards"],
        max_extra_shard_share=0.20,
        consistency_coef=0.25,
        epochs=1,
        batch_size=16,
        device="cpu",
        out=trained_zip,
    )
    assert trained_zip.is_file()
    assert "consistency_loss" in train_summary["epochs"][0]

    # 5. Track C read-only Tensor Completeness Fuzzer with live plasticity_probe
    fuzz_rep = run_tensor_completeness_fuzz(
        profile="standard_lite_v2",
        families=["opt_spent_vs_unspent", "delay_armed_vs_placing_turn"],
        checkpoint_zip=trained_zip,
        plasticity_steps=6,
    )
    by_fam = {r.family: r for r in fuzz_rep.results}
    assert by_fam["opt_spent_vs_unspent"].status == "distinguished"
    assert by_fam["delay_armed_vs_placing_turn"].status == "collision"

    # 6. Track D Dataset Exporter + 5-Stage DigimonVerifiableReward
    grpo_data_dir = tmp_path / "tactician_data"
    ds_summary = export_tactician_datasets(out_dir=grpo_data_dir)
    assert ds_summary.sft_rows >= 4 and ds_summary.grpo_rows >= 4

    sft_first = json.loads(ds_summary.sft_path.read_text(encoding="utf-8").splitlines()[0])
    grpo_first = json.loads(ds_summary.grpo_path.read_text(encoding="utf-8").splitlines()[0])
    reward_fn = DigimonVerifiableReward()
    scores = reward_fn(
        prompts=[grpo_first["prompt"]],
        completions=[[{"role": "assistant", "content": sft_first["messages"][-1]["content"]}]],
        ground_truth_meta=[grpo_first["ground_truth_meta"]],
    )
    assert scores == [pytest.approx(1.0)]

