"""Keep failing exam reports actionable without changing admission or verdicts."""

from subprocess import CompletedProcess
from unittest.mock import patch

from tools.exam_sim_all import run_one


def test_ruling_contradiction_is_reported_as_a_failure():
    output = "  RULING qa:Q3399 CONTRADICTED: at 22: p1.security expected 3 but our engine has 5\n"
    with patch("tools.exam_sim_all.subprocess.run", return_value=CompletedProcess([], 1, output, "")):
        ok, errors = run_one("harness", "scenario.yaml", "data/starter_decks.json")
    assert not ok
    assert errors == [output.strip()]


def test_unclassified_harness_crash_reports_stderr_and_exit_code():
    with patch("tools.exam_sim_all.subprocess.run", return_value=CompletedProcess([], 101, "", "thread panicked")):
        ok, errors = run_one("harness", "scenario.yaml", "data/starter_decks.json")
    assert not ok
    assert errors == ["harness exited 101: thread panicked"]


def test_zero_exit_without_success_summary_is_still_a_failure():
    with patch("tools.exam_sim_all.subprocess.run", return_value=CompletedProcess([], 0, "unexpected output", "")):
        ok, errors = run_one("harness", "scenario.yaml", "data/starter_decks.json")
    assert not ok
    assert errors == ["harness exited 0: unexpected output"]
