"""Exam binding: scenarios <-> the clause_coverage denominator."""
import json
from pathlib import Path

import pytest

from tools.clause_coverage.exam_binding import bind


def _write_scenario(d: Path, name: str, card: str, clause: str) -> Path:
    p = d / f"{name}.yaml"
    p.write_text(
        f"card: {card}\nclause: {clause}\nseed: 1\n"
        "decks:\n  p0: {stack: [], rest: x}\n  p1: {stack: [], rest: x}\n"
        "steps:\n  - actor: 0\n    do: {pass: {}}\n",
        encoding="utf-8",
    )
    return p


def test_clause_with_no_scenario_is_unmeasured(tmp_path):
    result = bind(["EX12-073"], tmp_path, tmp_path / "verdicts.json")
    assert result["denominator"]["total_clauses"] > 0
    # Every clause is unmeasured: nothing has been authored.
    assert result["denominator"]["by_verdict"]["unmeasured"] == result["denominator"]["total_clauses"]
    assert result["unmeasured_clause_ids"]


def test_a_scenario_naming_an_unknown_clause_is_an_orphan_not_a_pass(tmp_path):
    # This is the invisible-sixth-class failure the whole binding exists to
    # prevent: a typo'd clause id would otherwise pass its own assertions while
    # covering nothing in the denominator.
    _write_scenario(tmp_path, "typo", "EX12-073", "EX12-073#effct#0")
    result = bind(["EX12-073"], tmp_path, tmp_path / "verdicts.json")
    assert result["orphan_scenarios"], "a scenario keyed to no real clause must be reported"
    assert "EX12-073#effct#0" in json.dumps(result["orphan_scenarios"])


def test_denominator_always_sums_to_total(tmp_path):
    result = bind(["EX12-073", "EX12-035"], tmp_path, tmp_path / "verdicts.json")
    by = result["denominator"]["by_verdict"]
    assert sum(by.values()) == result["denominator"]["total_clauses"]


def test_verdicts_file_absent_is_not_an_error(tmp_path):
    # First run on a fresh checkout must work, reporting everything unmeasured.
    result = bind(["EX12-073"], tmp_path, tmp_path / "does-not-exist.json")
    assert result["denominator"]["by_verdict"]["unmeasured"] > 0


# --- Fifth test (beyond the plan's four): a real stored verdict must surface. ---


def _first_clause(card_id: str) -> dict:
    """The real, extracted first `effect` clause of a card, so the test binds
    against the same ids/text the tool will."""
    from tools.clause_coverage.extract import run

    clauses = run([card_id], "test")["clauses"]
    return next(c for c in clauses if c["zone"] == "effect")


def _write_verdicts(path: Path, entries: list[dict]) -> None:
    path.write_text(
        json.dumps(
            {
                "version": 1,
                "last_updated": "2026-08-21T00:00:00Z",
                "clauses": {e["clause_id"]: e for e in entries},
            },
            indent=2,
        ),
        encoding="utf-8",
    )


def test_clause_with_a_scenario_and_a_stored_verdict_reports_that_verdict(tmp_path):
    from tools.clause_coverage.exam_binding import clause_text_sha256

    clause = _first_clause("EX12-073")
    scenario = _write_scenario(tmp_path, "on_play", "EX12-073", clause["id"])

    verdicts_path = tmp_path / "verdicts.json"
    _write_verdicts(
        verdicts_path,
        [
            {
                "clause_id": clause["id"],
                "card_id": "EX12-073",
                "verdict": "confirmed",
                "label": clause["label"],
                "text_sha256": clause_text_sha256(clause["text"]),
                "scenario_path": str(scenario),
                "reason": None,
                "dcgo_build": None,
                "job_id": None,
                "recorded_at": "2026-08-21T00:00:00Z",
            }
        ],
    )

    result = bind(["EX12-073"], tmp_path, verdicts_path)

    bound = next(
        c for c in result["cards"]["EX12-073"]["clauses"] if c["clause_id"] == clause["id"]
    )
    assert bound["verdict"] == "confirmed", bound
    assert bound["scenarios"] == [str(scenario)]
    assert clause["id"] not in result["unmeasured_clause_ids"]
    assert result["denominator"]["by_verdict"]["confirmed"] == 1
    # The denominator still holds: everything else is honestly unmeasured.
    assert sum(result["denominator"]["by_verdict"].values()) == result["denominator"]["total_clauses"]
    assert not result["orphan_scenarios"]


def test_a_stored_verdict_whose_clause_text_drifted_is_invalidated_not_trusted(tmp_path):
    # Clause ids are positional within a zone, so re-scraped/overridden text can
    # silently re-point an id at a DIFFERENT clause. A stale `confirmed` would
    # then vouch for a clause nobody examined -- it must degrade to `unmeasured`.
    clause = _first_clause("EX12-073")
    _write_scenario(tmp_path, "on_play", "EX12-073", clause["id"])

    verdicts_path = tmp_path / "verdicts.json"
    _write_verdicts(
        verdicts_path,
        [
            {
                "clause_id": clause["id"],
                "card_id": "EX12-073",
                "verdict": "confirmed",
                "text_sha256": "stale-sha-from-different-text",
                "recorded_at": "2026-08-21T00:00:00Z",
            }
        ],
    )

    result = bind(["EX12-073"], tmp_path, verdicts_path)
    bound = next(
        c for c in result["cards"]["EX12-073"]["clauses"] if c["clause_id"] == clause["id"]
    )
    assert bound["verdict"] == "unmeasured"
    assert bound["invalidated"] is True
    assert clause["id"] in result["invalidated_clause_ids"]
    assert result["denominator"]["by_verdict"]["confirmed"] == 0
