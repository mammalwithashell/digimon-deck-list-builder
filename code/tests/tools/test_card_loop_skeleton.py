"""Tests for the card-loop skeleton: CLI dispatch, shared contracts, config."""
import sys
import types

import pytest

from tools.card_loop import __main__ as cli
from tools.card_loop.config import LoopConfig, load_config
from tools.card_loop.contracts import (
    TaskPacket,
    WorkerResult,
    item_id,
    split_item_id,
)


def test_help_lists_every_command(capsys):
    assert cli.main(["--help"]) == 0
    out = capsys.readouterr().out
    for name in cli.COMMANDS:
        assert name in out


def test_unknown_command_is_a_usage_error():
    assert cli.main(["frobnicate"]) == 2


def test_unbuilt_module_reports_not_implemented(monkeypatch, capsys):
    monkeypatch.setitem(cli.COMMANDS, "ghost", ("tools.card_loop.no_such_module", "cli", ""))
    assert cli.main(["ghost"]) == cli.NOT_IMPLEMENTED
    assert "not implemented yet" in capsys.readouterr().err


def test_dispatch_passes_remaining_args(monkeypatch):
    seen = {}
    fake = types.ModuleType("tools.card_loop._fake_cmd")
    fake.run = lambda argv: seen.setdefault("argv", argv) and 7
    monkeypatch.setitem(sys.modules, "tools.card_loop._fake_cmd", fake)
    monkeypatch.setitem(cli.COMMANDS, "fake", ("tools.card_loop._fake_cmd", "run", ""))
    assert cli.main(["fake", "--x", "1"]) == 7
    assert seen["argv"] == ["--x", "1"]


def test_item_ids_round_trip():
    assert item_id("clause", "BT7-056#effect#0") == "clause:BT7-056#effect#0"
    assert split_item_id("interaction:qa:Q1601") == ("interaction", "qa:Q1601")
    with pytest.raises(ValueError):
        item_id("deck", "x")


def test_packet_rejects_unknown_stage_and_family():
    base = dict(stage="triage", family="codex", item="clause:X#effect#0", attempt_id="a1",
                prompt="p", prompt_version="1", schema_path="s.json", worktree="wt")
    TaskPacket(**base)
    with pytest.raises(ValueError):
        TaskPacket(**{**base, "stage": "vibes"})
    with pytest.raises(ValueError):
        TaskPacket(**{**base, "family": "gemini"})


def test_worker_result_rejects_unknown_status():
    WorkerResult(status="ok")
    with pytest.raises(ValueError):
        WorkerResult(status="maybe")


def test_config_defaults_match_the_design():
    cfg = LoopConfig()
    assert cfg.audit_rate == 0.05
    assert cfg.exploration_share == 0.2
    assert cfg.routes["review"] == "codex"


def test_config_toml_overrides_merge_nested_tables(tmp_path):
    p = tmp_path / "loop.toml"
    p.write_text('audit_rate = 0.1\n[routes]\ntriage = "codex"\n[models.codex]\neffort = "medium"\n',
                 encoding="utf-8")
    cfg = load_config(str(p))
    assert cfg.audit_rate == 0.1
    assert cfg.routes["triage"] == "codex"
    assert cfg.routes["implement"] == "claude"        # untouched default kept
    assert cfg.models["codex"] == {"model": None, "effort": "medium"}


def test_config_rejects_unknown_keys_and_bad_routes(tmp_path):
    p = tmp_path / "bad.toml"
    p.write_text("auidt_rate = 0.1\n", encoding="utf-8")
    with pytest.raises(ValueError):
        load_config(str(p))
    with pytest.raises(ValueError):
        LoopConfig(routes={"triage": "gemini"})
