"""Tests for code/digimon_gym/agents/mulligan_log.py."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from digimon_gym.agents.mulligan_log import (
    _derive_lvl_counts,
    _derive_has_tamer,
    MulliganLogWriter,
)


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
        enabled=False,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0, "agent_archetype": None})
    assert not (tmp_path / "mulligan_log.jsonl").exists()


def test_writer_writes_header_then_record(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        enabled=True,
        run_metadata={"run_name": "test_run", "started_at": "2026-05-23T00:00:00+00:00"},
    )
    writer.append({"action": 0, "agent_archetype": "Puppets"})
    lines = _read_jsonl(tmp_path / "mulligan_log.jsonl")
    assert len(lines) == 2
    assert lines[0]["kind"] == "mulligan_log_header"
    assert lines[0]["schema_version"] == 1
    assert lines[0]["run_name"] == "test_run"
    assert lines[1]["action"] == 0
    assert lines[1]["agent_archetype"] == "Puppets"


def test_writer_writes_header_only_once_across_appends(tmp_path):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    writer.append({"action": 0})
    writer.append({"action": 1})
    writer.append({"action": 0})
    lines = _read_jsonl(tmp_path / "mulligan_log.jsonl")
    assert len(lines) == 4
    assert lines[0]["kind"] == "mulligan_log_header"
    # All subsequent records are data rows, not headers
    assert all(line.get("kind") != "mulligan_log_header" for line in lines[1:])


def test_writer_failure_disables_for_rest_of_run(tmp_path, capsys, monkeypatch):
    writer = MulliganLogWriter(
        output_dir=tmp_path,
        enabled=True,
        run_metadata={"run_name": "test_run"},
    )
    # Force the file open to raise the first time it's attempted.
    original_open = Path.open

    def _exploding_open(self, *args, **kwargs):
        if self.name == "mulligan_log.jsonl":
            raise OSError("simulated disk-full")
        return original_open(self, *args, **kwargs)

    monkeypatch.setattr(Path, "open", _exploding_open)
    writer.append({"action": 0})
    # Disabled now; subsequent appends should be silent no-ops.
    writer.append({"action": 1})
    assert writer.enabled is False
    stderr = capsys.readouterr().err
    assert "mulligan_log" in stderr.lower()
