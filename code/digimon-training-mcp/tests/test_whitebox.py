"""End-to-end tests for Phase 1C white-box `.zip` diagnostics (`whitebox.py` + `server.py`)."""

from __future__ import annotations

import hashlib
import io
import json
import shutil
import sys
from contextlib import redirect_stdout
from pathlib import Path
from typing import Any, Dict

import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")
pytest.importorskip("onnx")
pytest.importorskip("onnxruntime")
pytest.importorskip("digimon_engine")

import gymnasium as gym  # noqa: E402
import torch  # noqa: E402
import digimon_engine as de  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[3]
CODE_ROOT = REPO_ROOT / "code"
if str(CODE_ROOT) not in sys.path:
    sys.path.insert(0, str(CODE_ROOT))

from digimon_gym.agents.features_extractor import CardEmbeddingExtractor  # noqa: E402
from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO  # noqa: E402
from digimon_gym.agents.maskable_recurrent.policies import MaskableMlpLstmPolicy  # noqa: E402
from model_contract import current_contract, validate_onnx_file  # noqa: E402

from digimon_training_mcp.server import (  # noqa: E402
    TOOL_HANDLERS,
    ServerContext,
    build_server,
)
from digimon_training_mcp.whitebox import (  # noqa: E402
    card_embedding_neighbors,
    export_checkpoint_onnx,
    plasticity_probe,
    position_saliency,
)


EXAM_STAGED_REL = "qa/search-exams/ex13-imperialdramon-dna-paildramon-jamming-lethal.json"
EXAM_PREFIX_REL = "qa/search-exams/ex13-empty-board-choke-over-lv5-dump.json"
PUZZLE_REL = "qa/puzzles/3-muskets-der-blitz.json"


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _make_dummy_env(contract: Dict[str, Any]) -> gym.Env:
    tensor_size = int(contract["tensor_size"])
    action_size = int(contract["action_space_size"])

    class _DummySpacesEnv(gym.Env):
        observation_space = gym.spaces.Box(-10.0, 20001.0, (tensor_size,), np.float32)
        action_space = gym.spaces.Discrete(action_size)

        def reset(self, *, seed=None, options=None):
            return np.zeros((tensor_size,), dtype=np.float32), {}

        def step(self, action):
            return np.zeros((tensor_size,), dtype=np.float32), 0.0, True, False, {}

    return _DummySpacesEnv()


def _seed_meaningful_embeddings(policy: Any) -> None:
    """Plant deterministic structure in the embedding table so neighbor and
    card-ID saliency assertions are crisp and fast without needing rollouts."""
    reg = de.CardRegistry()
    emb = policy.features_extractor.embedding
    with torch.no_grad():
        gen = torch.Generator().manual_seed(2026)
        emb.weight.copy_(torch.randn(emb.weight.shape, generator=gen) * 0.05)
        emb.weight[0].zero_()

        idx_exveemon = reg.index_of("BT12-022")
        idx_stingmon = reg.index_of("BT12-050")
        idx_paildramon = reg.index_of("BT16-025")
        idx_slayerdramon = reg.index_of("EX13-024")

        base_vec = torch.zeros(emb.weight.shape[1], dtype=torch.float32)
        base_vec[0] = 5.0
        base_vec[1] = 3.0
        emb.weight[idx_exveemon].copy_(base_vec)
        # Make BT12-050 Stingmon the unmistakable nearest neighbor of BT12-022 ExVeemon.
        near_vec = base_vec.clone()
        near_vec[2] = 0.05
        emb.weight[idx_stingmon].copy_(near_vec)

        # Give BT16-025 Paildramon and EX13-024 Slayerdramon large norms so card slots
        # register strongly in gradient-times-embedding saliency.
        pail_vec = torch.full((emb.weight.shape[1],), 4.0, dtype=torch.float32)
        slayer_vec = torch.full((emb.weight.shape[1],), -3.5, dtype=torch.float32)
        emb.weight[idx_paildramon].copy_(pail_vec)
        emb.weight[idx_slayerdramon].copy_(slayer_vec)

        # Ensure projection weights on card-embedding columns are non-trivial.
        proj_linear = policy.features_extractor.projection[0]
        proj_linear.weight.add_(0.25)


@pytest.fixture(scope="module")
def built_checkpoints(tmp_path_factory: pytest.TempPathFactory) -> Dict[str, Path]:
    base_dir = tmp_path_factory.mktemp("whitebox_ckpts")
    contract = current_contract("standard_lite_v2")
    env = _make_dummy_env(contract)

    mlp = MaskablePPO(
        "MlpPolicy",
        env,
        n_steps=8,
        batch_size=8,
        seed=11,
        device="cpu",
        policy_kwargs=dict(
            net_arch=dict(pi=[32], vf=[32]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=32,
                embedding_dim=16,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    mlp.digimon_contract = contract
    _seed_meaningful_embeddings(mlp.policy)
    mlp_path = base_dir / "tiny_mlp.zip"
    mlp.save(str(mlp_path))

    lstm = MaskableRecurrentPPO(
        MaskableMlpLstmPolicy,
        env,
        n_steps=8,
        batch_size=8,
        seed=22,
        device="cpu",
        policy_kwargs=dict(
            lstm_hidden_size=32,
            n_lstm_layers=1,
            enable_critic_lstm=True,
            net_arch=dict(pi=[32], vf=[32]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=32,
                embedding_dim=16,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    lstm.digimon_contract = contract
    _seed_meaningful_embeddings(lstm.policy)
    lstm_path = base_dir / "tiny_lstm.zip"
    lstm.save(str(lstm_path))

    return {"mlp": mlp_path, "lstm": lstm_path}


# ─── 1. card_embedding_neighbors ──────────────────────────────────────


@pytest.mark.parametrize("kind", ["mlp", "lstm"])
def test_card_embedding_neighbors_end_to_end(built_checkpoints: Dict[str, Path], kind: str):
    ckpt = built_checkpoints[kind]
    res = card_embedding_neighbors(
        ckpt,
        card_ids=["BT12-022", "BT12-050", "BT16-025"],
        top_k=5,
    )
    assert res["ok"] is True, res
    assert len(res["cards"]) == 3

    exvee = res["cards"][0]
    assert exvee["card_id"] == "BT12-022"
    assert exvee["card_name"] == "ExVeemon"
    assert exvee["registry_index"] > 0
    assert exvee["l2_norm"] > 5.0
    assert len(exvee["neighbors"]) == 5

    # Planted nearest neighbor must be BT12-050 Stingmon, enriched with DB metadata.
    top_nb = exvee["neighbors"][0]
    assert top_nb["card_id"] == "BT12-050"
    assert top_nb["card_name"] == "Stingmon"
    assert top_nb["cosine_similarity"] > 0.99
    assert top_nb["level"] == 4
    assert isinstance(top_nb["colors"], list) and "Green" in top_nb["colors"]
    assert isinstance(top_nb["traits"], list) and len(top_nb["traits"]) > 0

    # Query card itself and padding index 0 are never in neighbors.
    nb_ids = {n["card_id"] for n in exvee["neighbors"]}
    assert "BT12-022" not in nb_ids

    # Pairwise cosine covers all 3 pairs.
    pw = res["pairwise_cosine"]
    assert set(pw.keys()) == {
        "BT12-022|BT12-050",
        "BT12-022|BT16-025",
        "BT12-050|BT16-025",
    }
    assert pw["BT12-022|BT12-050"] > 0.99


def test_card_embedding_neighbors_unknown_card_returns_error(built_checkpoints: Dict[str, Path]):
    res = card_embedding_neighbors(built_checkpoints["mlp"], card_ids=["NOT-A-CARD-999"])
    assert res["ok"] is False
    assert "NOT-A-CARD-999" in res["error"]


# ─── 2. position_saliency ─────────────────────────────────────────────


@pytest.mark.parametrize("kind", ["mlp", "lstm"])
def test_position_saliency_objectives_and_card_resolution(
    built_checkpoints: Dict[str, Path], kind: str
):
    ckpt = built_checkpoints[kind]
    exam_path = REPO_ROOT / EXAM_STAGED_REL

    # 1) logit_margin objective
    margin_res = position_saliency(
        ckpt,
        puzzle_path=exam_path,
        target_action_id=63,
        avoid_action_id=400,
        top_k=25,
    )
    assert margin_res["ok"] is True, margin_res
    assert margin_res["objective"] == "logit_margin"
    assert isinstance(margin_res["base_value"], float)
    assert isinstance(margin_res["margin"], float)

    sections = margin_res["section_attribution"]
    assert len(sections) == 8
    l1_vals = [s["l1_grad_x_input"] for s in sections]
    assert l1_vals == sorted(l1_vals, reverse=True)
    assert sum(l1_vals) > 0.0

    top_feats = margin_res["top_features"]
    assert len(top_feats) == 25
    for f in top_feats:
        assert set(f.keys()).issuperset({"index", "section", "grad_x_input", "grad", "obs_value"})
    # Occupied card-ID slots with high planted norms should resolve to real card IDs!
    resolved_cards = {f["card_id"] for f in top_feats if "card_id" in f}
    assert resolved_cards & {"BT16-025", "BT12-022", "BT12-050", "EX13-024"}

    # 2) target_logit objective
    tlogit_res = position_saliency(
        ckpt,
        puzzle_path=exam_path,
        target_action_id=63,
        top_k=10,
    )
    assert tlogit_res["ok"] is True, tlogit_res
    assert tlogit_res["objective"] == "target_logit"
    assert "margin" not in tlogit_res
    assert len(tlogit_res["top_features"]) == 10

    # 3) value objective
    val_res = position_saliency(
        ckpt,
        puzzle_path=exam_path,
        top_k=10,
    )
    assert val_res["ok"] is True, val_res
    assert val_res["objective"] == "value"
    assert "margin" not in val_res
    assert pytest.approx(val_res["base_value"], abs=1e-6) == margin_res["base_value"]


def test_position_saliency_lstm_prefix_actions_and_studio_puzzle_and_scenario_json(
    built_checkpoints: Dict[str, Path]
):
    lstm_ckpt = built_checkpoints["lstm"]

    # search-exam-v1 with replay prefix actions ([0, 0, 60]) warms LSTM (h, c)
    prefix_res = position_saliency(
        lstm_ckpt,
        puzzle_path=REPO_ROOT / EXAM_PREFIX_REL,
        target_action_id=62,
        top_k=8,
    )
    assert prefix_res["ok"] is True, prefix_res
    assert len(prefix_res["top_features"]) == 8

    # studio-puzzle-v1 file
    puzzle_res = position_saliency(
        lstm_ckpt,
        puzzle_path=REPO_ROOT / PUZZLE_REL,
        top_k=8,
    )
    assert puzzle_res["ok"] is True, puzzle_res
    assert puzzle_res["objective"] == "value"

    # Inline scenario_json string
    exam_doc = json.loads((REPO_ROOT / EXAM_STAGED_REL).read_text(encoding="utf-8"))
    inline_res = position_saliency(
        built_checkpoints["mlp"],
        scenario_json=json.dumps(exam_doc["setup"]["staged"]),
        target_action_id=63,
        avoid_action_id=400,
        top_k=5,
    )
    assert inline_res["ok"] is True, inline_res
    assert inline_res["objective"] == "logit_margin"


# ─── 3. plasticity_probe & Strict Read-Only Invariant ─────────────────


@pytest.mark.parametrize("kind", ["mlp", "lstm"])
def test_plasticity_probe_improves_and_never_mutates_checkpoint(
    built_checkpoints: Dict[str, Path], tmp_path: Path, kind: str
):
    # Copy checkpoint into isolated read-only-checked dir to verify mtime/sha256/siblings.
    ckpt_dir = tmp_path / f"models_{kind}"
    ckpt_dir.mkdir()
    ckpt_copy = ckpt_dir / f"agent_{kind}.zip"
    shutil.copy2(built_checkpoints[kind], ckpt_copy)

    before_sha = _sha256_file(ckpt_copy)
    before_mtime_ns = ckpt_copy.stat().st_mtime_ns
    before_files = sorted(p.name for p in ckpt_dir.iterdir())

    exam_path = REPO_ROOT / EXAM_STAGED_REL
    res = plasticity_probe(
        ckpt_copy,
        puzzle_path=exam_path,
        target_action_id=63,
        avoid_action_ids=[400, 114],
        steps=12,
        lr=1e-3,
    )
    assert res["ok"] is True, res
    assert res["loss_end"] < res["loss_start"]
    assert res["post"]["target_prob"] > res["pre"]["target_prob"]
    assert res["post"]["margin"] > res["pre"]["margin"]
    assert res["learnable"] is True
    assert isinstance(res["diagnosis"], str) and "learnable" in res["diagnosis"]

    # Disk checkpoint must be byte-for-byte & mtime untouched, with no sidecar files written.
    assert _sha256_file(ckpt_copy) == before_sha
    assert ckpt_copy.stat().st_mtime_ns == before_mtime_ns
    assert sorted(p.name for p in ckpt_dir.iterdir()) == before_files

    # Running again starts from the exact same `pre` metrics (proving in-memory isolation).
    res2 = plasticity_probe(
        ckpt_copy,
        puzzle_path=exam_path,
        target_action_id=63,
        avoid_action_ids=[400, 114],
        steps=2,
        lr=1e-3,
    )
    assert pytest.approx(res2["pre"]["target_prob"], abs=1e-6) == res["pre"]["target_prob"]
    assert pytest.approx(res2["pre"]["value"], abs=1e-6) == res["pre"]["value"]


def test_plasticity_probe_flags_observation_blindness_when_gradients_vanish(tmp_path: Path):
    """When features are dead (zeroed projection weights + negative biases behind ReLU),
    gradients are ~0 and plasticity_probe must report learnable=False + observation_blindness."""
    contract = current_contract("standard_lite_v2")
    env = _make_dummy_env(contract)
    blind_mlp = MaskablePPO(
        "MlpPolicy",
        env,
        n_steps=8,
        batch_size=8,
        device="cpu",
        policy_kwargs=dict(
            net_arch=dict(pi=[16], vf=[16]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=16,
                embedding_dim=8,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    blind_mlp.digimon_contract = contract
    blind_path = tmp_path / "blind.zip"
    blind_mlp.save(str(blind_path))

    res = plasticity_probe(
        blind_path,
        puzzle_path=REPO_ROOT / EXAM_STAGED_REL,
        target_action_id=63,
        avoid_action_ids=[400],
        steps=3,
        lr=1e-12,
    )
    assert res["ok"] is True, res
    assert res["learnable"] is False
    assert "observation_blindness" in res["diagnosis"]

    # Illegal target action is cleanly rejected.
    bad_res = plasticity_probe(
        blind_path,
        puzzle_path=REPO_ROOT / EXAM_STAGED_REL,
        target_action_id=2190,
        steps=5,
    )
    assert bad_res["ok"] is False
    assert "illegal" in bad_res["error"]


# ─── 4. export_checkpoint_onnx & Cache / Read-Only Guarantees ─────────


@pytest.mark.parametrize("kind", ["mlp", "lstm"])
def test_export_checkpoint_onnx_roundtrip_and_cache(
    built_checkpoints: Dict[str, Path], tmp_path: Path, kind: str
):
    ckpt_dir = tmp_path / f"readonly_{kind}"
    ckpt_dir.mkdir()
    ckpt_copy = ckpt_dir / f"{kind}.zip"
    shutil.copy2(built_checkpoints[kind], ckpt_copy)

    before_sha = _sha256_file(ckpt_copy)
    before_mtime_ns = ckpt_copy.stat().st_mtime_ns
    cache_root = tmp_path / "onnx_cache"

    # Capture stdout to prove export_checkpoint_onnx emits 0 bytes to stdout (MCP stdio safe).
    captured = io.StringIO()
    with redirect_stdout(captured):
        res = export_checkpoint_onnx(ckpt_copy, cache_dir=cache_root)
    assert captured.getvalue() == ""

    assert res["ok"] is True, res
    assert res["model_type"] == kind
    assert res["sha256"] == before_sha

    onnx_path = Path(res["onnx_path"])
    meta_path = Path(res["meta_path"])
    assert onnx_path.is_file()
    assert meta_path.is_file()
    assert onnx_path.parent == cache_root.resolve() / before_sha
    validate_onnx_file(onnx_path)

    if kind == "lstm":
        assert res["value_onnx_path"] is not None
        val_onnx_path = Path(res["value_onnx_path"])
        assert val_onnx_path.is_file()
        assert val_onnx_path.name == "policy.value.onnx"
    else:
        assert res["value_onnx_path"] is None

    # Checkpoint file and its directory remain 100% untouched!
    assert _sha256_file(ckpt_copy) == before_sha
    assert ckpt_copy.stat().st_mtime_ns == before_mtime_ns
    assert [p.name for p in ckpt_dir.iterdir()] == [f"{kind}.zip"]

    # Second call hits cache without rewriting policy.onnx.
    first_onnx_mtime = onnx_path.stat().st_mtime_ns
    res_cached = export_checkpoint_onnx(ckpt_copy, cache_dir=cache_root)
    assert res_cached["ok"] is True
    assert res_cached["onnx_path"] == str(onnx_path)
    assert onnx_path.stat().st_mtime_ns == first_onnx_mtime


# ─── 5. MCP Server Handlers, Relative Paths & Lazy-Import Fallback ────


@pytest.mark.asyncio
async def test_mcp_handlers_resolve_relative_paths(
    built_checkpoints: Dict[str, Path], tmp_path: Path
):
    models_dir = tmp_path / "models"
    models_dir.mkdir()
    rel_ckpt = models_dir / "run_alpha" / "step_000001000.zip"
    rel_ckpt.parent.mkdir(parents=True)
    shutil.copy2(built_checkpoints["mlp"], rel_ckpt)

    ctx = ServerContext(runs_dir=None, models_dir=models_dir, repo_root=REPO_ROOT)
    server = build_server(ctx)
    assert server.name == "digimon-training-mcp"

    r_emb = await TOOL_HANDLERS["card_embedding_neighbors"](
        ctx,
        {
            "checkpoint_path": "run_alpha/step_000001000.zip",
            "card_ids": ["BT12-022", "BT12-050"],
            "top_k": 3,
        },
    )
    assert r_emb["ok"] is True
    assert r_emb["cards"][0]["neighbors"][0]["card_id"] == "BT12-050"

    r_sal = await TOOL_HANDLERS["position_saliency"](
        ctx,
        {
            "checkpoint_path": "run_alpha/step_000001000.zip",
            "puzzle_path": EXAM_STAGED_REL,
            "target_action_id": 63,
            "avoid_action_id": 400,
            "top_k": 5,
        },
    )
    assert r_sal["ok"] is True
    assert r_sal["objective"] == "logit_margin"

    r_probe = await TOOL_HANDLERS["plasticity_probe"](
        ctx,
        {
            "checkpoint_path": "run_alpha/step_000001000.zip",
            "puzzle_path": EXAM_STAGED_REL,
            "target_action_id": 63,
            "steps": 4,
            "lr": 1e-3,
        },
    )
    assert r_probe["ok"] is True
    assert r_probe["loss_end"] < r_probe["loss_start"]

    r_onnx = await TOOL_HANDLERS["export_checkpoint_onnx"](
        ctx,
        {
            "checkpoint_path": "run_alpha/step_000001000.zip",
            "cache_dir": str(tmp_path / "mcp_onnx_cache"),
        },
    )
    assert r_onnx["ok"] is True
    assert Path(r_onnx["onnx_path"]).is_file()

    # Also verify full MCP CallToolRequest JSON-RPC dispatch via build_server(ctx).
    import mcp.types as mcp_types

    call_handler = server.request_handlers[mcp_types.CallToolRequest]
    mcp_res = await call_handler(
        mcp_types.CallToolRequest(
            method="tools/call",
            params=mcp_types.CallToolRequestParams(
                name="card_embedding_neighbors",
                arguments={
                    "checkpoint_path": "run_alpha/step_000001000.zip",
                    "card_ids": ["BT16-025"],
                    "top_k": 2,
                },
            ),
        )
    )
    mcp_payload = json.loads(mcp_res.root.content[0].text)
    assert mcp_payload["ok"] is True
    assert mcp_payload["cards"][0]["card_id"] == "BT16-025"


def test_lazy_import_graceful_error_when_torch_missing(
    monkeypatch: pytest.MonkeyPatch, built_checkpoints: Dict[str, Path]
):
    """When PyTorch is unavailable, white-box tools return structured {ok: False, error: ...}."""
    monkeypatch.setitem(sys.modules, "torch", None)

    r1 = card_embedding_neighbors(built_checkpoints["mlp"], ["BT16-025"])
    r2 = position_saliency(built_checkpoints["mlp"], puzzle_path=REPO_ROOT / EXAM_STAGED_REL)
    r3 = plasticity_probe(
        built_checkpoints["mlp"],
        puzzle_path=REPO_ROOT / EXAM_STAGED_REL,
        target_action_id=63,
    )
    for r in (r1, r2, r3):
        assert r["ok"] is False
        assert "missing whitebox dependency" in r["error"]

    monkeypatch.setitem(sys.modules, "onnx", None)
    r4 = export_checkpoint_onnx(built_checkpoints["mlp"])
    assert r4["ok"] is False
    assert "missing whitebox" in r4["error"]


def test_server_import_does_not_eagerly_load_heavy_ml_modules():
    """Importing `digimon_training_mcp.server` in a fresh Python process must not
    import `torch`, `sb3_contrib`, `onnx`, or `digimon_engine`."""
    import subprocess

    proc = subprocess.run(
        [
            sys.executable,
            "-c",
            (
                "import sys, digimon_training_mcp.server; "
                "loaded = [m for m in ('torch', 'sb3_contrib', 'onnx', 'onnxruntime', 'digimon_engine') if m in sys.modules]; "
                "assert not loaded, f'eagerly imported: {loaded}'"
            ),
        ],
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, proc.stderr

