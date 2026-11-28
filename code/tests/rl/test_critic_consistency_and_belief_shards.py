"""Tests for Track B:
1. Forced `PendingSelection` value rows (`forced_selection.npy`) & chain anchors (`chain_anchor_idx.npy`)
   emitted by `emit_puzzle_shards.py` on multi-step selection puzzles (`ex13-imperialdramon` & `ex12-erlangmon`).
2. Continuation-consistent critic loss (`--consistency-coef`) & forced-selection handling in both
   `azero_trainer.py` (MLP) and `azero_lstm_distill.py` (LSTM), including quarantine/concat rebasing.
3. Deterministic `--extra-shards` + `--max-extra-shard-share 0.10` cap in both trainers and
   `specialist_league.py` command generation.
4. Multi-world Belief Puzzle shard emission (`--belief-worlds W`) producing Bayes-averaged
   `policy_visits.npy` and continuous expected outcome `z.npy` (`z_bar` strictly in `(-1, +1)`).
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import gymnasium as gym
import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")

import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CODE_DIR = _REPO_ROOT / "code"
_TOOLS_DIR = _CODE_DIR / "tools"
for _p in (str(_CODE_DIR), str(_TOOLS_DIR)):
    if _p not in sys.path:
        sys.path.insert(0, _p)

from digimon_gym.agents import azero_lstm_distill, azero_trainer, value_activation  # noqa: E402
from digimon_gym.agents.features_extractor import CardEmbeddingExtractor  # noqa: E402
from digimon_gym.agents.maskable_recurrent import (  # noqa: E402
    MaskableMlpLstmPolicy,
    MaskableRecurrentPPO,
)
from emit_puzzle_shards import emit_puzzle_shards, main as emit_cli_main  # noqa: E402
from model_contract import current_contract  # noqa: E402
from tools.cloud_search import specialist_league as league  # noqa: E402

IMPERIALDRAMON_EXAM = (
    _REPO_ROOT / "qa" / "search-exams" / "ex13-imperialdramon-dna-paildramon-jamming-lethal.json"
)
ERLANGMON_PUZZLE = (
    _REPO_ROOT / "qa" / "puzzles" / "ex12-erlangmon-saneiketsu-invitation-trap.json"
)


class _ContractSpacesEnv(gym.Env):
    def __init__(self, profile: str = "standard_lite_v2"):
        super().__init__()
        contract = current_contract(profile)
        self.observation_space = gym.spaces.Box(
            -10.0, 20001.0, (contract["tensor_size"],), np.float32
        )
        self.action_space = gym.spaces.Discrete(contract["action_space_size"])

    def reset(self, *, seed=None, options=None):
        return np.zeros(self.observation_space.shape, dtype=np.float32), {}

    def step(self, action):
        return np.zeros(self.observation_space.shape, dtype=np.float32), 0.0, True, False, {}


def _write_chain_shard_dir(
    root: Path,
    *,
    games: int = 8,
    include_forced: bool = True,
    mid_terminated: int = 0,
    profile_id: str = "standard_lite_v2",
    seed: int = 0,
) -> Path:
    """Write a synthetic `selfplay-shards-v1` directory with tracked `PendingSelection` chains.

    Per game (4 rows when `include_forced=True`, 3 rows when `False`):
      - step 0: Main root (`forced=0`, `anchor=-1`)
      - step 1: PendingSelection multi-choice (`forced=0`, `anchor=post_idx`)
      - [step 2: PendingSelection choiceless (`forced=1`, `policy_weight=0.0`, `anchor=post_idx`)]
      - post_idx: Post-resolution Main state (`forced=0`, `anchor=-1`, `terminated=1`)
    """
    contract = current_contract(profile_id)
    obs_dim = int(contract["tensor_size"])
    action_dim = int(contract["action_space_size"])
    rng = np.random.default_rng(seed)

    # Pick non-card scalar coordinates near the end of standard_lite_v2 so CardEmbeddingExtractor
    # passes them directly into its scalar stream without integer card-ID clamping.
    c_sign = obs_dim - 4
    c_mid = obs_dim - 3
    c_forced = obs_dim - 2
    c_anc = obs_dim - 1

    obs_rows: list[np.ndarray] = []
    offsets: list[int] = [0]
    actions: list[int] = []
    visits: list[float] = []
    weights: list[float] = []
    z_list: list[float] = []
    term_list: list[int] = []
    movers: list[int] = []
    game_ids: list[int] = []
    forced_list: list[int] = []
    anchor_list: list[int] = []

    for g in range(games):
        sign = 1.0 if (g % 2 == 0) else -1.0
        base = len(obs_rows)
        post_local = 3 if include_forced else 2

        # Row 0: Root Main
        r0 = rng.normal(0.0, 0.01, size=(obs_dim,)).astype(np.float32)
        r0[c_sign] = sign
        obs_rows.append(r0)
        actions.extend([1, 3, 62])
        visits.extend([90.0, 5.0, 5.0] if sign > 0 else [5.0, 90.0, 5.0])
        offsets.append(len(actions))
        weights.append(1.0)
        z_list.append(sign)
        term_list.append(1)
        movers.append(0)
        game_ids.append(g)
        forced_list.append(0)
        anchor_list.append(-1)

        # Row 1: Mid-chain PendingSelection (choice)
        r1 = np.zeros(obs_dim, dtype=np.float32)
        r1[c_sign] = sign
        r1[c_mid] = 3.0 * sign
        obs_rows.append(r1)
        actions.extend([30, 31])
        visits.extend([80.0, 20.0] if sign > 0 else [20.0, 80.0])
        offsets.append(len(actions))
        weights.append(1.0)
        z_list.append(sign if mid_terminated else 0.0)
        term_list.append(int(mid_terminated))
        movers.append(0)
        game_ids.append(g)
        forced_list.append(0)
        anchor_list.append(base + post_local)

        if include_forced:
            # Row 2: Mid-chain forced PendingSelection (1 legal choice, policy_weight=0)
            r2 = np.zeros(obs_dim, dtype=np.float32)
            r2[c_sign] = sign
            r2[c_forced] = 4.0 * sign
            obs_rows.append(r2)
            actions.extend([30])
            visits.extend([100.0])
            offsets.append(len(actions))
            weights.append(0.0)
            z_list.append(sign)
            term_list.append(1)
            movers.append(0)
            game_ids.append(g)
            forced_list.append(1)
            anchor_list.append(base + post_local)

        # Post-resolution anchor row
        r_post = np.zeros(obs_dim, dtype=np.float32)
        r_post[c_sign] = sign
        r_post[c_anc] = 3.0 * sign
        obs_rows.append(r_post)
        actions.extend([114, 62])
        visits.extend([95.0, 5.0] if sign > 0 else [5.0, 95.0])
        offsets.append(len(actions))
        weights.append(1.0)
        z_list.append(sign)
        term_list.append(1)
        movers.append(0)
        game_ids.append(g)
        forced_list.append(0)
        anchor_list.append(-1)

    shard = root / "shard-000"
    shard.mkdir(parents=True, exist_ok=True)
    n_rows = len(obs_rows)
    np.save(shard / "obs.npy", np.stack(obs_rows, axis=0).astype(np.float32))
    np.save(shard / "policy_offsets.npy", np.asarray(offsets, dtype=np.int64))
    np.save(shard / "policy_actions.npy", np.asarray(actions, dtype=np.int32))
    np.save(shard / "policy_visits.npy", np.asarray(visits, dtype=np.float32))
    np.save(shard / "policy_weight.npy", np.asarray(weights, dtype=np.float32))
    np.save(shard / "z.npy", np.asarray(z_list, dtype=np.float32))
    np.save(shard / "terminated.npy", np.asarray(term_list, dtype=np.uint8))
    np.save(shard / "mover.npy", np.asarray(movers, dtype=np.uint8))
    np.save(shard / "game_id.npy", np.asarray(game_ids, dtype=np.int32))
    np.save(shard / "forced_selection.npy", np.asarray(forced_list, dtype=np.uint8))
    np.save(shard / "chain_anchor_idx.npy", np.asarray(anchor_list, dtype=np.int64))

    manifest = {
        "schema": "selfplay-shards-v1",
        "observation_profile": profile_id,
        "model_contract": contract,
        "obs_dim": obs_dim,
        "action_dim": action_dim,
        "value_target": "terminal_outcome",
        "information": {
            "regime": "open_decklist",
            "knowledge_schema": 2,
            "prior_hash": None,
            "legality_hash": None,
            "prior_provenance": None,
        },
        "games": games,
        "truncated_games": 0,
        "shards": [{"dir": "shard-000", "rows": n_rows}],
    }
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return root


def _make_mlp_seed(path: Path, seed: int = 0) -> Path:
    env = _ContractSpacesEnv("standard_lite_v2")
    model = MaskablePPO(
        "MlpPolicy",
        env,
        n_steps=8,
        batch_size=8,
        seed=seed,
        device="cpu",
        verbose=0,
        policy_kwargs=dict(
            net_arch=dict(pi=[64], vf=[64]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=64,
                embedding_dim=8,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    model.digimon_contract = current_contract("standard_lite_v2")
    model.save(str(path))
    return path


def _make_lstm_seed(path: Path, seed: int = 0) -> Path:
    env = _ContractSpacesEnv("standard_lite_v2")
    model = MaskableRecurrentPPO(
        MaskableMlpLstmPolicy,
        env,
        n_steps=16,
        batch_size=16,
        seed=seed,
        device="cpu",
        verbose=0,
        policy_kwargs=dict(
            lstm_hidden_size=32,
            n_lstm_layers=1,
            enable_critic_lstm=True,
            net_arch=dict(pi=[32], vf=[32]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=32,
                embedding_dim=4,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    model.digimon_contract = current_contract("standard_lite_v2")
    model.save(str(path))
    return path


def test_emit_puzzle_shards_forced_selection_and_chain_anchors_on_imperialdramon_and_erlangmon(
    tmp_path: Path,
):
    imp_dir = tmp_path / "imperialdramon_shards"
    imp_manifest = emit_puzzle_shards(
        [IMPERIALDRAMON_EXAM],
        imp_dir,
        permutations_per_puzzle=2,
        observation_profile="standard_lite_v2",
        seed=5,
    )
    s0_imp = imp_dir / "shard-000"
    assert (s0_imp / "forced_selection.npy").is_file()
    assert (s0_imp / "chain_anchor_idx.npy").is_file()
    assert imp_manifest["games"] == 3
    assert imp_manifest["rows"] == 15  # 5 steps per DNA Paildramon trajectory
    assert imp_manifest["forced_selection_rows"] == 3
    assert imp_manifest["chain_anchored_rows"] == 6

    imp_ds = azero_trainer.load_shards([imp_dir])
    # Canonical Imperialdramon trajectory (rows 0..4):
    #   row 0: Main root (act 63) -> forced=0, anchor=-1
    #   row 1: SelectMaterial 1st (act 0, 2 legal) -> forced=0, anchor=3
    #   row 2: SelectMaterial 2nd (act 1, 1 legal) -> forced=1, weight=0.0, anchor=3
    #   row 3: Post-DNA Main (act 114, 1st swing reveals security) -> forced=0, anchor=-1
    #   row 4: Main 2nd swing (act 114) -> forced=0, anchor=-1
    assert imp_ds.forced_selection[:5].tolist() == [0, 0, 1, 0, 0]
    assert imp_ds.policy_weight[:5].tolist() == [1.0, 1.0, 0.0, 1.0, 1.0]
    assert imp_ds.chain_anchor_idx[:5].tolist() == [-1, 3, 3, -1, -1]
    assert imp_ds.chain_anchor_idx[5:10].tolist() == [-1, 8, 8, -1, -1]
    assert imp_ds.chain_anchor_idx[10:15].tolist() == [-1, 13, 13, -1, -1]

    # LSTM sequence loader filters forced_selection==1 by default and rebases chain_anchor_idx
    # from shard row 3 -> local sequence step 2!
    imp_seq_default = azero_lstm_distill.load_sequence_shards([imp_dir])
    assert imp_seq_default.total_rows == 12
    assert imp_seq_default.sequences[0].length == 4
    assert imp_seq_default.sequences[0].forced_selection.tolist() == [0, 0, 0, 0]
    assert imp_seq_default.sequences[0].chain_anchor_idx.tolist() == [-1, 2, -1, -1]

    imp_seq_with_forced = azero_lstm_distill.load_sequence_shards(
        [imp_dir], include_forced_selection=True
    )
    assert imp_seq_with_forced.total_rows == 15
    assert imp_seq_with_forced.sequences[0].chain_anchor_idx.tolist() == [-1, 3, 3, -1, -1]

    # EX12 Erlangmon puzzle: 3 consecutive PendingSelection steps (rows 1, 2, 3) anchor to Main row 4.
    erl_dir = tmp_path / "erlangmon_shards"
    erl_manifest = emit_puzzle_shards(
        [ERLANGMON_PUZZLE],
        erl_dir,
        permutations_per_puzzle=0,
        observation_profile="standard_lite_v2",
        seed=5,
    )
    assert erl_manifest["rows"] == 6
    erl_ds = azero_trainer.load_shards([erl_dir])
    assert erl_ds.forced_selection.tolist() == [0, 0, 0, 0, 0, 0]
    assert erl_ds.chain_anchor_idx.tolist() == [-1, 4, 4, 4, -1, -1]

    # Concatenating multiple shard dirs + manifest-bound game exclusions rebases chain_anchor_idx cleanly.
    excl_path = imp_dir / "training_exclusions.json"
    excl_path.write_text(
        json.dumps(
            {
                "schema": "selfplay-training-exclusions-v1",
                "manifest_sha256": hashlib.sha256(
                    (imp_dir / "manifest.json").read_bytes()
                ).hexdigest(),
                "games": [{"game_id": 1, "reason": "test middle game exclusion rebasing"}],
            }
        ),
        encoding="utf-8",
    )
    pooled = azero_trainer.load_shards([imp_dir, erl_dir])
    assert len(pooled) == 10 + 6  # game 1 (5 rows) excluded from imp_dir
    assert pooled.chain_anchor_idx[:5].tolist() == [-1, 3, 3, -1, -1]
    assert pooled.chain_anchor_idx[5:10].tolist() == [-1, 8, 8, -1, -1]
    assert pooled.chain_anchor_idx[10:16].tolist() == [-1, 14, 14, 14, -1, -1]


def test_critic_consistency_loss_and_forced_selection_value_in_mlp_and_lstm(tmp_path: Path):
    shard_dir = _write_chain_shard_dir(
        tmp_path / "chain_shards",
        games=12,
        include_forced=True,
        mid_terminated=0,
        seed=17,
    )
    ds = azero_trainer.load_shards([shard_dir])
    mid_choice_rows = np.flatnonzero((ds.chain_anchor_idx >= 0) & (ds.forced_selection == 0))
    forced_rows = np.flatnonzero(ds.forced_selection == 1)
    assert len(mid_choice_rows) == 12
    assert len(forced_rows) == 12
    assert np.all(ds.policy_weight[forced_rows] == 0.0)

    # ── 1. MLP Trainer: consistency_coef shrinks |V(s_mid) - V(s_anchor)| & forced rows fit V ──
    mlp_seed = _make_mlp_seed(tmp_path / "mlp_seed.zip", seed=3)

    out_no_cc = tmp_path / "mlp_no_cc.zip"
    out_with_cc = tmp_path / "mlp_with_cc.zip"

    sum_no_cc = azero_trainer.train(
        init_from=mlp_seed,
        shard_dirs=[shard_dir],
        out=out_no_cc,
        epochs=15,
        batch_size=16,
        lr=3e-3,
        consistency_coef=0.0,
        value_activation_name="tanh",
        device="cpu",
        seed=0,
    )
    sum_with_cc = azero_trainer.train(
        init_from=mlp_seed,
        shard_dirs=[shard_dir],
        out=out_with_cc,
        epochs=15,
        batch_size=16,
        lr=3e-3,
        consistency_coef=1.0,
        value_activation_name="tanh",
        device="cpu",
        seed=0,
    )
    assert sum_no_cc["epochs"][0]["consistency_loss"] == pytest.approx(0.0)
    assert sum_with_cc["epochs"][0]["consistency_loss"] > 0.0
    assert sum_with_cc["epochs"][-1]["consistency_loss"] < sum_with_cc["epochs"][0]["consistency_loss"]

    m_no_cc = MaskablePPO.load(str(out_no_cc), device="cpu")
    m_with_cc = MaskablePPO.load(str(out_with_cc), device="cpu")

    def _mlp_vals(model: MaskablePPO, indices: np.ndarray) -> np.ndarray:
        model.policy.set_training_mode(False)
        with torch.no_grad():
            _, raw = azero_trainer._forward(model.policy, torch.as_tensor(ds.obs[indices]))
            return value_activation.apply("tanh", raw).cpu().numpy()

    anc_for_mid = ds.chain_anchor_idx[mid_choice_rows]
    gap_mlp_no_cc = float(np.mean(np.abs(_mlp_vals(m_no_cc, mid_choice_rows) - _mlp_vals(m_no_cc, anc_for_mid))))
    gap_mlp_with_cc = float(np.mean(np.abs(_mlp_vals(m_with_cc, mid_choice_rows) - _mlp_vals(m_with_cc, anc_for_mid))))
    assert gap_mlp_with_cc < 0.4 * gap_mlp_no_cc

    # Forced selection rows (policy_weight == 0, terminated == 1) actively train the MLP value head
    # while leaving the actor's action_net gradient on forced-only features untouched.
    forced_preds = _mlp_vals(m_no_cc, forced_rows)
    assert float(np.mean(forced_preds * ds.z[forced_rows])) > 0.4

    # ── 2. LSTM Distill Trainer: forced rows filtered by default & consistency_coef shrinks gap ──
    lstm_seed = _make_lstm_seed(tmp_path / "lstm_seed.zip", seed=3)
    lstm_no_cc = tmp_path / "lstm_no_cc.zip"
    lstm_with_cc = tmp_path / "lstm_with_cc.zip"

    lsum_no_cc = azero_lstm_distill.distill_lstm(
        shard_dirs=[shard_dir],
        out=lstm_no_cc,
        init_from=lstm_seed,
        epochs=15,
        seq_batch_size=4,
        lr=4e-3,
        consistency_coef=0.0,
        holdout_fraction=0.0,
        device="cpu",
        seed=0,
    )
    lsum_with_cc = azero_lstm_distill.distill_lstm(
        shard_dirs=[shard_dir],
        out=lstm_with_cc,
        init_from=lstm_seed,
        epochs=15,
        seq_batch_size=4,
        lr=4e-3,
        consistency_coef=1.0,
        holdout_fraction=0.0,
        device="cpu",
        seed=0,
    )
    # 12 games * 3 non-forced steps = 36 rows (12 forced_selection==1 rows filtered out by default)
    assert lsum_with_cc["total_rows"] == 36
    assert lsum_no_cc["epochs"][0]["consistency_loss"] == pytest.approx(0.0)
    assert lsum_with_cc["epochs"][0]["consistency_loss"] > 0.0
    assert lsum_with_cc["epochs"][-1]["consistency_loss"] < lsum_with_cc["epochs"][0]["consistency_loss"]

    seq_ds = azero_lstm_distill.load_sequence_shards([shard_dir])
    lm_no_cc = MaskableRecurrentPPO.load(str(lstm_no_cc), device="cpu")
    lm_with_cc = MaskableRecurrentPPO.load(str(lstm_with_cc), device="cpu")

    def _lstm_seq_gap(model: MaskableRecurrentPPO) -> float:
        model.policy.set_training_mode(False)
        obs_t, *_ = azero_lstm_distill._collate_batch(
            seq_ds.sequences, seq_ds.action_dim, torch.device("cpu")
        )
        with torch.no_grad():
            _, vals = azero_lstm_distill._forward_sequences(model.policy, obs_t)
            vals_np = vals.cpu().numpy()
        diffs = []
        for b, seq in enumerate(seq_ds.sequences):
            for t in range(seq.length):
                anc_t = int(seq.chain_anchor_idx[t])
                if anc_t >= 0:
                    diffs.append(abs(float(vals_np[t, b]) - float(vals_np[anc_t, b])))
        return float(np.mean(diffs))

    gap_lstm_no_cc = _lstm_seq_gap(lm_no_cc)
    gap_lstm_with_cc = _lstm_seq_gap(lm_with_cc)
    assert gap_lstm_with_cc < 0.4 * gap_lstm_no_cc


def test_extra_shards_deterministic_cap_and_specialist_league_commands(tmp_path: Path):
    # Primary: 6 games * 3 rows = 18 rows -> at max_share=0.10, max_Rows = int(18 * 0.1 / 0.9) = 2?
    # Use 9 games * 4 rows = 36 primary rows -> max_Rows = int(36 * 0.1 / 0.9) = 4 rows (1 whole 4-row game!).
    primary_dir = _write_chain_shard_dir(
        tmp_path / "primary",
        games=9,
        include_forced=True,
        mid_terminated=1,
        seed=10,
    )
    extra_dir = _write_chain_shard_dir(
        tmp_path / "extra",
        games=6,  # 24 rows (far above max_Rows = 4)
        include_forced=True,
        mid_terminated=1,
        seed=99,
    )

    # Deterministic load_shards cap + valid rebased chain_anchor_idx
    capped_a = azero_trainer.load_shards(
        [primary_dir], extra_shards=[extra_dir], max_extra_shard_share=0.10, seed=42
    )
    capped_b = azero_trainer.load_shards(
        [primary_dir], extra_shards=[extra_dir], max_extra_shard_share=0.10, seed=42
    )
    assert len(capped_a) == 36 + 4  # 4 extra rows / 40 total = 10.0%
    np.testing.assert_array_equal(np.asarray(capped_a.obs), np.asarray(capped_b.obs))
    np.testing.assert_array_equal(capped_a.chain_anchor_idx, capped_b.chain_anchor_idx)
    assert np.all(
        (capped_a.chain_anchor_idx == -1)
        | ((capped_a.chain_anchor_idx >= 0) & (capped_a.chain_anchor_idx < len(capped_a)))
    )
    # Extra game's chain anchors sit in [36, 40)
    assert capped_a.chain_anchor_idx[36:40].tolist() == [-1, 39, 39, -1]

    mlp_seed = _make_mlp_seed(tmp_path / "seed_mlp.zip", seed=1)
    mlp_sum = azero_trainer.train(
        init_from=mlp_seed,
        shard_dirs=[primary_dir],
        extra_shards=[extra_dir],
        max_extra_shard_share=0.10,
        out=tmp_path / "capped_mlp.zip",
        epochs=1,
        batch_size=16,
        device="cpu",
        seed=42,
    )
    assert mlp_sum["extra_rows_loaded"] == 24
    assert mlp_sum["extra_rows_used"] == 4
    assert mlp_sum["rows"] == 40

    # LSTM distill: 9 games * 3 non-forced rows = 27 primary rows -> max_Rows = int(27 * 0.1 / 0.9) = 3 rows (1 sequence)
    lstm_seed = _make_lstm_seed(tmp_path / "seed_lstm.zip", seed=1)
    lstm_sum = azero_lstm_distill.distill_lstm(
        shard_dirs=[primary_dir],
        extra_shards=[extra_dir],
        max_extra_shard_share=0.10,
        out=tmp_path / "capped_lstm.zip",
        init_from=lstm_seed,
        epochs=1,
        seq_batch_size=4,
        holdout_fraction=0.0,
        device="cpu",
        seed=42,
    )
    assert lstm_sum["extra_rows_loaded"] == 18  # 6 sequences * 3 non-forced rows
    assert lstm_sum["extra_rows_used"] == 3
    assert lstm_sum["total_rows"] == 27 + 3

    # specialist_league.generation_command: unchanged when absent, wired when present on disk or in plan
    base_plan = dict(
        cli_bin="/opt/cli",
        cards_json="/opt/cards.json",
        games=100,
        workers=4,
        seed=7,
        replay_window=3,
        training=dict(
            trainer_kind="mlp",
            epochs=2,
            lr=1e-4,
            batch_size=256,
            value_holdout_fraction=0.2,
            value_activation="tanh",
            train_device="cpu",
        ),
    )
    cmd_clean = league.generation_command(tmp_path, base_plan, "Examon", tmp_path / "pool.json", 0)
    assert "--extra-shards" not in cmd_clean
    assert "--max-extra-shard-share" not in cmd_clean

    # Auto-discovered under root / 'extra_shards' / slug(arch)
    examon_extra = tmp_path / "extra_shards" / "examon"
    examon_extra.mkdir(parents=True)
    cmd_disk = league.generation_command(tmp_path, base_plan, "Examon", tmp_path / "pool.json", 0)
    assert "--extra-shards" in cmd_disk
    ex_idx = cmd_disk.index("--extra-shards")
    assert cmd_disk[ex_idx + 1] == str(examon_extra)
    assert "--max-extra-shard-share" in cmd_disk
    assert cmd_disk[cmd_disk.index("--max-extra-shard-share") + 1] == "0.1"

    # Explicit plan['training']['extra_shards'] + custom share
    plan_explicit = json.loads(json.dumps(base_plan))
    plan_explicit["training"]["extra_shards"] = {"Royal Knights": [str(extra_dir)]}
    plan_explicit["training"]["max_extra_shard_share"] = 0.08
    cmd_rk = league.generation_command(
        tmp_path, plan_explicit, "Royal Knights", tmp_path / "pool.json", 1
    )
    assert cmd_rk[cmd_rk.index("--extra-shards") + 1] == str(extra_dir)
    assert cmd_rk[cmd_rk.index("--max-extra-shard-share") + 1] == "0.08"


def test_multi_world_belief_puzzle_shards_emit_continuous_z_bar_and_bayes_visits(tmp_path: Path):
    # Imperfect-information Belief Puzzle:
    # Seat 1 has unsuspended ST1-03 Agumon facing 0 opponent security, unsuspended BT19-009 Growlmon
    # on Seat 2's field, and 1 hidden card in Seat 2's hand.
    # - 60% of `prior.candidates` hold BT19-011 WarGrowlmon ACE (<Blast Digivolve> in CounterTiming ->
    #   deletes ST1-03, gains 1 memory, swings for game -> outcome -1.0).
    # - 40% of `prior.candidates` hold vanilla ST1-03 Agumon (no Counter -> ST1-03 wins -> outcome +1.0).
    belief_puzzle_path = tmp_path / "belief_wargrowlmon_ace.json"
    belief_doc = {
        "schema": "studio-puzzle-v1",
        "id": "belief-wargrowlmon-ace-60-40",
        "title": "60% WarGrowlmon ACE hand trap vs 40% vanilla hand",
        "decks": {
            "1": ["ST1-01"] * 4 + ["ST1-03"] * 50,
            "2": ["ST1-01"] * 4 + ["BT19-009"] * 1 + ["ST1-03"] * 49,
        },
        "seed": 0,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 0, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "hand": [],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": [],
                    "trash": [],
                },
                "2": {
                    "hand": ["ST1-03"],
                    "field": [{"stack": ["BT19-009"], "is_suspended": False, "turn_played": 1}],
                    "security": [],
                    "trash": [],
                },
            },
            "prefix_actions": [],
        },
        "prior": {
            "format_id": "standard",
            "candidates": [
                # Incompatible candidate missing public field card BT19-009 must be filtered out!
                {"cards": ["ST1-01"] * 4 + ["ST1-07"] * 50, "weight": 99.0},
                {"cards": ["ST1-01"] * 4 + ["BT19-009"] * 1 + ["BT19-011"] * 49, "weight": 0.6},
                {"cards": ["ST1-01"] * 4 + ["BT19-009"] * 1 + ["ST1-03"] * 49, "weight": 0.4},
            ],
        },
        "refutation_triggers": [
            {"phase": "CounterTiming", "accept_optional": True},
        ],
        "lines": [
            {"id": "swing-into-possible-ace", "role": "target", "actions": [114]},
        ],
        "rl_goal": {"mode": "playout_to_win"},
    }
    belief_puzzle_path.write_text(json.dumps(belief_doc), encoding="utf-8")

    belief_shard_dir = tmp_path / "belief_shards"
    rc = emit_cli_main(
        [
            "--inputs",
            str(belief_puzzle_path),
            "--out",
            str(belief_shard_dir),
            "--permutations-per-puzzle",
            "0",
            "--belief-worlds",
            "20",
            "--seed",
            "7",
        ]
    )
    assert rc == 0

    z_arr = np.load(belief_shard_dir / "shard-000" / "z.npy")
    assert len(z_arr) == 1
    z_bar = float(z_arr[0])
    assert -1.0 < z_bar < 1.0
    assert z_bar == pytest.approx(-0.2, abs=0.15)  # 0.4 * (+1) + 0.6 * (-1) = -0.2

    # Multi-line Belief Puzzle: safe bypass (Gaia Force deletes BT19-009 first -> wins 100% of worlds)
    # vs hasty swing (action 114 -> refuted in 60% ACE worlds, wins in 40% vanilla worlds).
    two_line_doc = json.loads(json.dumps(belief_doc))
    two_line_doc["id"] = "belief-gaia-bypass-vs-ace"
    two_line_doc["decks"]["1"] = ["ST1-01"] * 4 + ["ST1-03"] * 46 + ["ST1-16"] * 4
    two_line_doc["setup"]["state"]["memory"] = 8
    two_line_doc["setup"]["zones"]["1"]["hand"] = ["ST1-16"]
    two_line_doc["lines"] = [
        {"id": "delete-growlmon-then-swing", "role": "target", "actions": [0, 100, 114]},
        {"id": "hasty-swing-into-ace", "role": "avoid", "actions": [114]},
    ]
    two_line_dir = tmp_path / "belief_two_line_shards"
    emit_puzzle_shards(
        [two_line_doc],
        two_line_dir,
        permutations_per_puzzle=0,
        belief_worlds=20,
        seed=7,
    )
    ds_two = azero_trainer.load_shards([two_line_dir])
    seq_two = azero_lstm_distill.load_sequence_shards([two_line_dir])
    assert len(ds_two) >= 1
    assert seq_two.total_rows >= 1

    r0_acts = ds_two.actions[ds_two.offsets[0] : ds_two.offsets[1]]
    r0_vis = ds_two.visits[ds_two.offsets[0] : ds_two.offsets[1]]
    vis_by_act = {int(a): float(v) for a, v in zip(r0_acts.tolist(), r0_vis.tolist())}
    # In 60% ACE worlds action 0 strictly wins (100 visits); in 40% vanilla worlds 0 and 114 tie (50/50)
    # -> Bayes-averaged visits ~80 on action 0 vs ~20 on action 114, and z_bar ~ 0.76 strictly in (-1, +1).
    assert vis_by_act[0] > vis_by_act[114]
    assert vis_by_act[114] > 0.0
    assert -1.0 < float(ds_two.z[0]) < 1.0
    assert 114 in set(ds_two.avoid_actions[ds_two.avoid_offsets[0] : ds_two.avoid_offsets[1]].tolist())
