"""Exam binding: scenarios <-> the clause_coverage denominator."""
import json
from pathlib import Path

import pytest

from tools.clause_coverage.exam_binding import bind, load_verdict_store


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


def test_load_verdict_store_reads_a_directory_of_per_card_files(tmp_path):
    """The fleet layout: one file per card, merged on read."""
    d = tmp_path / "exam-verdicts"
    d.mkdir()
    (d / "EX12-073.json").write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "EX12-073#effect#0": {
                        "clause_id": "EX12-073#effect#0",
                        "card_id": "EX12-073",
                        "verdict": "confirmed",
                        "text_sha256": "abc",
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    (d / "BT8-084.json").write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "BT8-084#effect#0": {
                        "clause_id": "BT8-084#effect#0",
                        "card_id": "BT8-084",
                        "verdict": "unreachable",
                        "text_sha256": "def",
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    store = load_verdict_store(d)

    assert set(store) == {"EX12-073#effect#0", "BT8-084#effect#0"}
    assert store["EX12-073#effect#0"]["verdict"] == "confirmed"


def test_load_verdict_store_missing_directory_is_empty(tmp_path):
    """A fresh checkout has no ledger; everything is honestly unmeasured."""
    assert load_verdict_store(tmp_path / "does-not-exist") == {}


def test_load_verdict_store_directory_raises_loudly_on_corrupt_file(tmp_path):
    """A corrupt verdict file must raise, never silently degrade to `{}`.

    This is deliberate and pre-existing, not a bug: returning `{}` for an
    unreadable file would silently downgrade already-confirmed clauses back
    to `unmeasured` with no indication anything went wrong -- exactly the
    silent wrongness this ledger exists to prevent. Do NOT "fix" this into a
    silent `{}` return; the loud failure is the contract.
    """
    d = tmp_path / "exam-verdicts"
    d.mkdir()
    (d / "EX12-073.json").write_text("{bad json", encoding="utf-8")

    with pytest.raises(json.JSONDecodeError):
        load_verdict_store(d)


def test_load_verdict_store_directory_rejects_a_row_filed_under_the_wrong_card(tmp_path):
    """A row's card_id must agree with the file that holds it.

    Mirrors the Rust `VerdictStore::load_dir` check in
    `code/tools/dcgo-harness/src/exam/verdict.rs`: a BT8-084 row hand-edited
    or badly merged into EX12-035.json would otherwise be silently accepted
    by the Python reader (via plain dict.update) while the Rust reader
    refuses it -- and `bind()` feeds this straight into the human-facing
    denominator report.
    """
    d = tmp_path / "exam-verdicts"
    d.mkdir()
    (d / "EX12-035.json").write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "BT8-084#effect#0": {
                        "clause_id": "BT8-084#effect#0",
                        "card_id": "BT8-084",
                        "verdict": "confirmed",
                        "text_sha256": "abc",
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError) as exc_info:
        load_verdict_store(d)

    message = str(exc_info.value)
    assert "EX12-035" in message, message
    assert "BT8-084" in message, message


def test_load_verdict_store_still_reads_a_single_file(tmp_path):
    """The single-file form stays supported: tests and fixtures use it."""
    p = tmp_path / "verdicts.json"
    p.write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "EX12-073#effect#0": {
                        "clause_id": "EX12-073#effect#0",
                        "card_id": "EX12-073",
                        "verdict": "confirmed",
                        "text_sha256": "abc",
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    assert set(load_verdict_store(p)) == {"EX12-073#effect#0"}
