"""Tests for the ``run_summary`` tool — spec §run_summary scenarios + panic bucketing."""

from __future__ import annotations

from digimon_training_mcp.panic_families import (
    PANIC_LINE_RE,
    PanicFamily,
    load_panic_families,
    match_family,
)
from digimon_training_mcp.summary import (
    count_panics,
    fallback_parse_eval_console,
    parse_header,
    read_eval_sidecar,
    run_summary,
    tail_console,
)


def test_parse_header_extracts_known_fields(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    header = parse_header(run_dir / "console.log")
    assert header["algorithm"] == "MaskablePPO"
    assert header["opponent"] == "greedy"
    assert header["total_steps"] == "1,000,000"
    assert header["tensorboard"] == "runs/test_run"
    assert header["tensor_profile"] == "standard_lite_v2"
    assert "gauntlet" in header


def test_read_eval_sidecar_tail(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    rows = read_eval_sidecar(run_dir / "evals.jsonl", tail=3)
    assert rows is not None
    assert len(rows) == 3
    assert rows[-1]["step"] == 50_000
    assert rows[0]["step"] == 30_000


def test_read_eval_sidecar_returns_none_when_missing(tmp_path):
    assert read_eval_sidecar(tmp_path / "missing.jsonl", tail=5) is None


def test_fallback_parse_eval_console(synthetic_repo):
    """When the sidecar is absent, regex-parse [Eval @ N steps] lines."""
    run_dir = synthetic_repo["active_run_dir"]
    rows = fallback_parse_eval_console(run_dir / "console.log", tail=3)
    assert len(rows) == 3
    assert rows[0]["step"] == 10_000
    # Wins are reported as fractions
    assert 0.0 <= rows[0]["win_rate"] <= 1.0
    # Fields absent in the console line are explicit null
    assert rows[0]["draw_rate"] is None


def test_count_panics_buckets_by_family(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    families = load_panic_families(synthetic_repo["repo_root"])
    result = count_panics(run_dir / "console.log", families)
    assert result["total"] == 4  # 2 outer-tail + 1 reentrant + 1 unmatched
    assert result["by_family"].get("G-DSL-OUTER-TAIL-NESTED-PARK") == 2
    assert result["by_family"].get("G-OPTION-PLAY-REENTRANT") == 1
    assert result["by_family"].get("other") == 1


def test_count_panics_no_families_rolls_everything_under_other(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    result = count_panics(run_dir / "console.log", [])
    assert result["total"] == 4
    assert result["by_family"].get("other") == 4


def test_tail_console_returns_last_n_lines(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    tail = tail_console(run_dir / "console.log", n=5)
    assert len(tail) == 5
    # Each line is newline-stripped
    assert all("\n" not in line for line in tail)


def test_run_summary_composes_all_sections(synthetic_repo):
    run_dir = synthetic_repo["active_run_dir"]
    families = load_panic_families(synthetic_repo["repo_root"])
    summary = run_summary(run_dir, tail_evals=3, families=families)
    assert summary["ok"] is True
    assert summary["header"]["algorithm"] == "MaskablePPO"
    assert len(summary["evals"]) == 3
    assert summary["evals_source"] == "sidecar"
    assert summary["panics"]["total"] == 4
    assert isinstance(summary["recent_console_tail"], list)


def test_run_summary_falls_back_to_console_when_no_sidecar(synthetic_repo):
    """A run with no evals.jsonl falls through to regex-parsing the console."""
    stale = synthetic_repo["stale_run_dir"]
    summary = run_summary(stale, tail_evals=3, families=[])
    assert summary["ok"] is True
    assert summary["evals_source"] == "console"
    # stale_run's console has no eval lines
    assert summary["evals"] == []


def test_panic_line_regex_matches_real_format():
    """Verify the regex against the literal format in training_recording.py:194-202."""
    sample = "[recorder env=0 game=42 step=128] ENGINE PANIC #1: PanicException: dsl_outer_tail overwrite: card=BT24-016"
    m = PANIC_LINE_RE.match(sample)
    assert m is not None
    assert m.group("env") == "0"
    assert m.group("game") == "42"
    assert m.group("step") == "128"
    assert m.group("count") == "1"
    assert m.group("exc_type") == "PanicException"
    assert "dsl_outer_tail overwrite" in m.group("msg")


def test_load_panic_families_handles_missing_file(tmp_path):
    """Missing panic-families.json is non-fatal."""
    assert load_panic_families(tmp_path) == []
    assert load_panic_families(None) == []


def test_match_family_first_match_wins():
    families = [
        PanicFamily(family_id="A", description="a", pattern=__import__("re").compile("foo"), status="open"),
        PanicFamily(family_id="B", description="b", pattern=__import__("re").compile("foo|bar"), status="open"),
    ]
    assert match_family("foo and bar", families) == "A"
    assert match_family("just bar", families) == "B"
    assert match_family("nothing here", families) is None
