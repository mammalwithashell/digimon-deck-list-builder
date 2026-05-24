"""End-to-end integration test for the mulligan-log pipeline.

The per-component unit tests in ``test_mulligan_log.py`` cover the helpers,
writer, wrapper, and (separately) the ``--mulligan-log`` CLI flag /
``TrainingConfig`` field. None of them exercise the wiring through
``pilot_training.train()`` — this test fills that gap with a short
generalist run and asserts the JSONL file lands at the documented path
with a header + N records, every record carrying all schema fields.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest


REQUIRED_RECORD_FIELDS = (
    "action",
    "hand_card_ids",
    "hand_lvl_counts",
    "hand_has_tamer",
    "hand_size",
    "agent_archetype",
    "opp_archetype",
    "first_player_id",
    "source",
    "env_index",
    "game_index",
    "wall_time",
    "iso_time",
)


@pytest.mark.slow
def test_train_writes_mulligan_log_jsonl_end_to_end(tmp_path):
    """Drive ``train()`` for a short generalist run; assert JSONL appears.

    Budget tuned for ~50s wall time: 2000 timesteps × ~70 steps/game on
    one env produces ~28 mulligan records, well above the >=3 bar. PPO
    hyperparameters are shrunk (n_steps=128, batch_size=32) so the
    rollout/update cadence completes within the budget.

    Seeds ``seed=1`` and ``curriculum_seed=1`` were chosen empirically as a
    deck-pairing trajectory that does not hit the known Rust engine panic
    ``"nested deferred deletion not supported (single-outstanding
    invariant)"`` during the first 2000 steps. If this test starts failing
    with that panic, the underlying engine bug has shifted under us —
    rotate to a fresh stable seed pair, file the panic against the engine,
    and don't silently suppress it here.
    """
    from digimon_gym.agents.gauntlet import load_generalist_deck_pool
    from digimon_gym.agents.pilot_training import train
    from digimon_gym.agents.training_config import TrainingConfig

    pool = load_generalist_deck_pool()
    expected_archetypes = set(pool.archetype_names)
    assert expected_archetypes, "generalist deck pool is empty; cannot run test"

    run_name = "test_mulligan_e2e"
    cfg = TrainingConfig(
        algorithm="mlp",
        timesteps=2000,
        seed=1,
        n_envs=1,
        n_steps=128,
        batch_size=32,
        n_epochs=2,
        opponent="greedy",
        generalist=True,
        eval_freq=10_000_000,  # >> timesteps so no eval fires
        eval_episodes=1,
        checkpoint_every=0,    # no checkpointing
        models_dir=str(tmp_path),
        tensorboard_log=str(tmp_path / "tb"),
        run_name=run_name,
        mulligan_log="on",
        curriculum_seed=1,
        # BO3 makes action 93 (concede) legal during mulligan; this test
        # asserts the mulligan choice is in {0, 1}, so opt back to the
        # single-game mode where mulligan is still the only decision.
        match_format="single",
    )

    train(cfg=cfg, generalist_deck_pool=pool, verbose=0)

    jsonl_path = tmp_path / run_name / "mulligan_log_env_000.jsonl"
    assert jsonl_path.exists(), f"expected mulligan log at {jsonl_path}"

    lines = [
        json.loads(line)
        for line in jsonl_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    headers = [line for line in lines if line.get("kind") == "mulligan_log_header"]
    records = [line for line in lines if line.get("kind") != "mulligan_log_header"]

    assert len(headers) >= 1, "JSONL is missing the header line"
    header = headers[0]
    assert header["schema_version"] == 1
    assert header["run_name"] == run_name

    # BO3 match-training added action 93 (CONCEDE_GAME) as always-legal at
    # every agent decision point including mulligan. The mulligan log writer
    # records every action taken in the mulligan phase, including concedes
    # — those aren't real mulligan choices so filter them out before the
    # core assertions.
    mulligan_records = [r for r in records if r["action"] in (0, 1)]
    assert len(mulligan_records) >= 3, (
        f"expected >=3 mulligan records from 2000 timesteps of greedy "
        f"play; got {len(mulligan_records)} (total records {len(records)}). "
        f"file contents: {lines!r}"
    )

    for i, rec in enumerate(mulligan_records):
        missing = [k for k in REQUIRED_RECORD_FIELDS if k not in rec]
        assert not missing, f"record {i} missing fields {missing}: {rec!r}"
        assert rec["source"] == "train", f"record {i} source != 'train': {rec!r}"
        assert rec["action"] in (0, 1), f"record {i} action not in {{0,1}}: {rec!r}"
        assert rec["hand_size"] == 5, f"record {i} hand_size != 5: {rec!r}"
        assert isinstance(rec["hand_card_ids"], list) and len(rec["hand_card_ids"]) == 5, (
            f"record {i} hand_card_ids not a length-5 list: {rec!r}"
        )
        assert rec["env_index"] == 0, f"record {i} env_index != 0: {rec!r}"
        assert rec["agent_archetype"] in expected_archetypes, (
            f"record {i} agent_archetype {rec['agent_archetype']!r} not in "
            f"generalist pool {sorted(expected_archetypes)}"
        )
