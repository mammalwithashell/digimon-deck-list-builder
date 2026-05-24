"""Tests for code/digimon_gym/agents/mulligan_log.py."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from digimon_gym.agents.mulligan_log import (
    _derive_lvl_counts,
    _derive_has_tamer,
    MulliganLogWriter,
    MulliganLogWrapper,
)
from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.agents.pilot_training import OpponentWrapper


def test_derive_lvl_counts_counts_each_level_bucket():
    # ST1-03 is a level-3 Digimon; ST1-01 is a level-2 (egg).
    # Pick known-existing ids from data/cards.json.
    counts = _derive_lvl_counts(["ST1-03", "ST1-03", "ST1-01", "ST1-03", "ST1-01"])
    # Only levels 3-7 are bucketed.
    assert counts["3"] == 3
    assert counts["4"] == 0
    assert counts["5"] == 0
    assert counts["6"] == 0
    assert counts["7"] == 0


def test_derive_lvl_counts_handles_unknown_ids():
    counts = _derive_lvl_counts(["NOT-A-REAL-CARD", "ST1-03"])
    assert counts["3"] == 1  # unknown id contributes 0 to every bucket


def test_derive_has_tamer_returns_false_when_no_tamer():
    # ST1-03 is a Digimon, not a Tamer.
    assert _derive_has_tamer(["ST1-03", "ST1-03"]) is False


def test_derive_has_tamer_returns_true_when_any_card_is_tamer():
    # Look up any Tamer-typed card from cards.json. If cards.json has no
    # tamer at all (extremely unlikely), skip rather than fail spuriously.
    from data_paths import CARDS_JSON
    cards = json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    # cards.json encodes type as card_kind int: 1 = Tamer
    tamer_ids = [cid for cid, c in cards.items() if c.get("card_kind") == 1]
    if not tamer_ids:
        pytest.skip("No Tamer cards in cards.json — cannot exercise has_tamer=True path")
    assert _derive_has_tamer([tamer_ids[0], "ST1-03"]) is True


def _read_jsonl(path: Path) -> list:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def test_writer_disabled_does_nothing(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=False,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0, "agent_archetype": None})
    assert not (tmp_path / "mulligan_log_env_000.jsonl").exists()


def test_writer_writes_header_then_record(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run", "started_at": "2026-05-23T00:00:00+00:00"},
    )
    writer.append({"action": 0, "agent_archetype": "Puppets"})
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    assert len(lines) == 2
    assert lines[0]["kind"] == "mulligan_log_header"
    assert lines[0]["schema_version"] == 1
    assert lines[0]["run_name"] == "test_run"
    assert lines[1]["action"] == 0
    assert lines[1]["agent_archetype"] == "Puppets"


def test_writer_writes_header_only_once_across_appends(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0})
    writer.append({"action": 1})
    writer.append({"action": 0})
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    assert len(lines) == 4
    assert lines[0]["kind"] == "mulligan_log_header"
    # All subsequent records are data rows, not headers
    assert all(line.get("kind") != "mulligan_log_header" for line in lines[1:])


def test_writer_failure_disables_for_rest_of_run(tmp_path, capsys, monkeypatch):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        env_index=0,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    # Force the file open to raise the first time it's attempted.
    original_open = Path.open

    def _exploding_open(self, *args, **kwargs):
        if self.name == "mulligan_log_env_000.jsonl":
            raise OSError("simulated disk-full")
        return original_open(self, *args, **kwargs)

    monkeypatch.setattr(Path, "open", _exploding_open)
    writer.append({"action": 0})
    # Disabled now; subsequent appends should be silent no-ops.
    writer.append({"action": 1})
    assert writer.enabled is False
    stderr = capsys.readouterr().err
    assert "mulligan_log" in stderr.lower()


def test_writer_env_index_in_filename(tmp_path):
    writer0 = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    writer3 = MulliganLogWriter(output_dir=tmp_path, env_index=3, enabled=True, run_metadata={"run_name": "t"})
    assert writer0.path == tmp_path / "mulligan_log_env_000.jsonl"
    assert writer3.path == tmp_path / "mulligan_log_env_003.jsonl"
    # Each writer writes its own header independently.
    writer0.append({"action": 0})
    writer3.append({"action": 1})
    lines0 = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    lines3 = _read_jsonl(tmp_path / "mulligan_log_env_003.jsonl")
    assert lines0[0]["kind"] == "mulligan_log_header"
    assert lines3[0]["kind"] == "mulligan_log_header"
    assert lines0[1]["action"] == 0
    assert lines3[1]["action"] == 1


def _drive_to_first_pilot_step(env):
    """Reset env and skip opponent's leading turns until pilot acts.

    OpponentWrapper already does this on reset, so this is a no-op here
    but documents the contract: after reset returns, the next step()
    submitted is the pilot's first decision (mulligan if first turn).
    """
    obs, info = env.reset(seed=1)
    return obs, info


def _build_wrapped_env(writer, record_actions: bool = False):
    inner = DigimonEnv(record_actions=record_actions)
    opp = OpponentWrapper(inner, opponent_fn=greedy_policy)
    wrapped = MulliganLogWrapper(opp, writer=writer, source="train", env_index=0)
    return wrapped, inner


def test_wrapper_captures_pilot_mulligan_keep(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    _drive_to_first_pilot_step(wrapped)
    # Pilot picks KEEP (action 0). We bypass policy here and submit directly.
    assert inner.runner.mulligan_current_player == 1
    wrapped.step(0)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    # 1 header + 1 record
    assert len(lines) == 2
    rec = lines[1]
    assert rec["action"] == 0
    assert rec["source"] == "train"
    assert rec["env_index"] == 0
    assert rec["game_index"] == 0
    assert rec["hand_size"] == 5
    assert isinstance(rec["hand_card_ids"], list) and len(rec["hand_card_ids"]) == 5
    assert "hand_lvl_counts" in rec
    assert "hand_has_tamer" in rec
    assert rec["schema_version"] == 1


def test_wrapper_captures_pilot_mulligan_mull_when_opp_first(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    # record_actions=True means get_recording() is available on the runner; after
    # the first step it exposes initial_state.first_player_id as the authoritative
    # source (independent of to_ui_json / currentPlayer), letting us cross-check
    # the wrapper's logged value against the recording's own copy.
    wrapped, inner = _build_wrapped_env(writer, record_actions=True)
    # Find a seed where P2 truly goes first.  initial_state is populated only
    # after the first step, so we use to_ui_json() for the seed oracle (it is
    # available immediately after reset).
    found_seed = None
    for s in range(40):
        wrapped.reset(seed=s)
        if inner.runner.to_ui_json().get("currentPlayer") == 2:
            found_seed = s
            break
    if found_seed is None:
        pytest.skip("no seed in 0..39 produced P2-goes-first; try a wider range")
    # Pilot picks MULL (action 1).  After this step, initial_state is populated
    # in the recording, giving us an authoritative first_player_id to compare.
    wrapped.step(1)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    rec = lines[-1]
    assert rec["action"] == 1
    assert rec["source"] == "train"
    assert rec["first_player_id"] == 2  # the bug we fixed: this would be 1 if we used current_player_id
    # Cross-check: wrapper's value must agree with the authoritative recording
    # source, proving the get_recording() override path is consistent.
    authoritative_fp = inner.runner.get_recording()["initial_state"]["first_player_id"]
    assert rec["first_player_id"] == authoritative_fp


def test_wrapper_disabled_writer_writes_nothing(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=False, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    _drive_to_first_pilot_step(wrapped)
    wrapped.step(0)
    assert not (tmp_path / "mulligan_log_env_000.jsonl").exists()


def test_wrapper_increments_game_index_across_resets(tmp_path):
    writer = MulliganLogWriter(output_dir=tmp_path, env_index=0, enabled=True, run_metadata={"run_name": "t"})
    wrapped, inner = _build_wrapped_env(writer)
    for _ in range(3):
        wrapped.reset(seed=1)
        wrapped.step(0)
    lines = _read_jsonl(tmp_path / "mulligan_log_env_000.jsonl")
    # 1 header + 3 records
    assert len(lines) == 4
    indices = [line["game_index"] for line in lines[1:]]
    assert indices == [0, 1, 2]
