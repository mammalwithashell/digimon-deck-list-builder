import json
from pathlib import Path

from tools.rules import mechanics_readiness as mr

REPO = Path(__file__).resolve().parents[3]


def _ob(status, mechanisms, blocks=False, gap_id=None):
    return {
        "status": status,
        "mechanisms": list(mechanisms),
        "blocks_training": blocks,
        "gap_id": gap_id,
        "chapter": 15,
        "tests": [],
    }


def _ledger(obs):
    return {"source": {"rules_version": "4.3", "sha256": "x"}, "obligations": obs}


def test_ready_when_no_blocking_gaps():
    out = mr.build(
        _ledger(
            {
                "15-1": _ob("proven", ["effects-and-ordering"]),
                "15-2": _ob("gap", ["keywords"], blocks=False, gap_id="G-1"),
                "15-3": _ob("not-testable", ["battle"]),
            }
        ),
        "abc",
    )
    assert out["ready"] is True
    assert out["counts"] == {
        "proven": 1,
        "not_testable": 1,
        "gap_blocking": 0,
        "gap_nonblocking": 1,
        "total": 3,
    }
    assert out["blocking"] == []
    assert out["version"] == 1
    assert out["source"] == {"obligations_sha256": "abc", "rules_version": "4.3"}
    assert out["mechanics_set"] == list(mr.MECHANICS_SET)


def test_blocking_gap_sets_not_ready_and_lists_rule():
    out = mr.build(
        _ledger(
            {
                "15-1": _ob("proven", ["effects-and-ordering"]),
                "15-9": _ob("gap", ["blocking", "battle"], blocks=True, gap_id="G-BLOCK"),
            }
        ),
        "abc",
    )
    assert out["ready"] is False
    assert out["counts"]["gap_blocking"] == 1
    assert out["blocking"] == [
        {"rule": "15-9", "gap_id": "G-BLOCK", "mechanisms": ["blocking", "battle"]}
    ]


def test_zero_proven_is_not_ready():
    out = mr.build(
        _ledger(
            {
                "15-1": _ob("not-testable", ["keywords"]),
                "15-2": _ob("gap", ["keywords"]),
            }
        ),
        "abc",
    )
    assert out["counts"]["proven"] == 0
    assert out["counts"]["gap_blocking"] == 0
    assert out["ready"] is False


def test_only_mechanics_set_rules_counted():
    out = mr.build(
        _ledger(
            {
                "2-1": _ob("proven", ["card-info"]),
                "2-2": _ob("gap", ["card-info"], blocks=True, gap_id="G-X"),
                "2-3": _ob("not-testable", []),
                "7-1": _ob("proven", ["card-info", "link"]),
            }
        ),
        "abc",
    )
    assert out["counts"]["total"] == 1
    assert out["counts"]["proven"] == 1
    assert out["blocking"] == []
    assert out["ready"] is True


def _write_ledger(path, obs):
    path.write_text(json.dumps(_ledger(obs)), encoding="utf-8")


def test_source_sha_mismatch_fails_check(tmp_path):
    ledger = tmp_path / "cr_obligations.json"
    artifact = tmp_path / "mechanics_readiness.json"
    _write_ledger(ledger, {"15-1": _ob("proven", ["keywords"])})
    assert mr.main(["--obligations", str(ledger), "--output", str(artifact)]) == 0
    assert mr.main(["--obligations", str(ledger), "--output", str(artifact), "--check"]) == 0

    # Ledger changes -> committed artifact is stale.
    _write_ledger(
        ledger,
        {"15-1": _ob("proven", ["keywords"]), "15-2": _ob("gap", ["keywords"], True, "G-1")},
    )
    assert mr.main(["--obligations", str(ledger), "--output", str(artifact), "--check"]) == 1

    # Regenerate, then tamper with the recorded sha only.
    assert mr.main(["--obligations", str(ledger), "--output", str(artifact)]) == 0
    data = json.loads(artifact.read_text(encoding="utf-8"))
    data["source"]["obligations_sha256"] = "0" * 64
    artifact.write_text(json.dumps(data), encoding="utf-8")
    assert mr.main(["--obligations", str(ledger), "--output", str(artifact), "--check"]) == 1


def test_check_fails_when_artifact_missing(tmp_path):
    ledger = tmp_path / "cr_obligations.json"
    _write_ledger(ledger, {"15-1": _ob("proven", ["keywords"])})
    assert (
        mr.main(["--obligations", str(ledger), "--output", str(tmp_path / "nope.json"), "--check"])
        == 1
    )


def test_real_ledger_counts_are_consistent():
    ledger_path = REPO / "qa/rules/cr_obligations.json"
    ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
    out = mr.build(ledger, mr.ledger_sha256(ledger_path))
    c = out["counts"]
    assert c["total"] == c["proven"] + c["not_testable"] + c["gap_blocking"] + c["gap_nonblocking"]
    assert c["total"] > 0
    assert len(out["blocking"]) == c["gap_blocking"]
    assert out["ready"] == (c["gap_blocking"] == 0 and c["proven"] > 0)
    mset = set(mr.MECHANICS_SET)
    expected = sum(1 for o in ledger["obligations"].values() if mset & set(o["mechanisms"]))
    assert c["total"] == expected
