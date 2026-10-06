import json
import os

import pytest

from tools.clause_coverage.readiness import (
    build_readiness,
    card_status,
    clause_is_adjudicated,
    library_decklists,
    render,
)


def _clause(cid, verdict, **extra):
    row = {"clause_id": cid, "verdict": verdict, "reason": None, "triage": None, "citation": None}
    row.update(extra)
    return row


@pytest.mark.parametrize(
    "clause, expected",
    [
        (_clause("A#effect#0", "confirmed"), True),
        (_clause("A#effect#0", "unreachable", reason="background-process card"), True),
        (_clause("A#effect#0", "unreachable", reason="  "), False),
        (_clause("A#effect#0", "unavailable", reason="no DCGO script"), True),
        (_clause("A#effect#0", "unavailable"), False),
        (_clause("A#effect#0", "diverged", triage="dcgo_quirk", citation="G-EXAM-X"), True),
        (_clause("A#effect#0", "diverged", triage="dcgo_quirk"), False),
        (_clause("A#effect#0", "diverged", triage="ours_wrong", citation="15-8-3-2"), False),
        (_clause("A#effect#0", "diverged", triage="undetermined"), False),
        (_clause("A#effect#0", "diverged"), False),
        (_clause("A#effect#0", "unmeasured"), False),
    ],
)
def test_clause_rules(clause, expected):
    assert clause_is_adjudicated(clause) is expected


def test_card_ready_only_when_every_clause_is_adjudicated():
    report = {
        "card_id": "A",
        "total_clauses": 2,
        "by_verdict": {"confirmed": 1, "unmeasured": 1},
        "clauses": [_clause("A#effect#0", "confirmed"), _clause("A#inherited#0", "unmeasured")],
    }
    status = card_status(report)
    assert status["status"] == "not_ready"
    assert status["blocking"] == ["A#inherited#0"]


def test_zero_clause_card_is_ready():
    report = {"card_id": "V", "total_clauses": 0, "by_verdict": {}, "clauses": []}
    assert card_status(report)["status"] == "ready"


def test_build_readiness_is_deterministic_and_summarised(tmp_path):
    def fake_bind(card_ids, scenarios_dir, verdicts_path, *, source_desc=None):
        cards = {
            "B": {"card_id": "B", "total_clauses": 1, "by_verdict": {"confirmed": 1},
                  "clauses": [_clause("B#effect#0", "confirmed")]},
            "A": {"card_id": "A", "total_clauses": 1, "by_verdict": {"unmeasured": 1},
                  "clauses": [_clause("A#effect#0", "unmeasured")]},
        }
        return {"cards": {c: cards[c] for c in card_ids}}

    data = build_readiness(["B", "A", "B"], scenarios_dir=tmp_path, verdicts_dir=tmp_path,
                           bind_fn=fake_bind)
    assert list(data["cards"]) == ["A", "B"]
    assert data["summary"] == {"not_ready": 1, "ready": 1}
    assert render(data) == render(json.loads(render(data)))
    assert render(data).endswith("}\n")


def test_library_decklists_reads_json_and_card_ids(tmp_path):
    lib = tmp_path / "lib.json"
    lib.write_text(json.dumps({"archetypes": {
        "X": {"decklists": [{"decklist": json.dumps(["BT1-001", "BT1-001"])},
                            {"card_ids": ["BT1-002"]},
                            {"decklist": "not json"}]},
    }}), encoding="utf-8")
    assert library_decklists(lib) == [["BT1-001", "BT1-001"], ["BT1-002"]]


def test_library_decklists_drops_strings_the_gauntlet_parser_drops(tmp_path):
    # gauntlet.py parses decklists with the engine's parse_tts, which keeps only
    # card-ID-shaped strings (export headers etc. are dropped). The artifact must
    # cover exactly the ids the gate will look up.
    lib = tmp_path / "lib.json"
    lib.write_text(json.dumps({"archetypes": {
        "X": {"decklists": [{"decklist": json.dumps(["Exported from x", "BT1-001", 7, "P-103",
                                                      "bt1-001", "ST1-01"])},
                            {"decklist": json.dumps(["header only"])}]},
    }}), encoding="utf-8")
    assert library_decklists(lib) == [["BT1-001", "P-103", "ST1-01"]]


def test_build_readiness_extracts_with_dcgo_disabled_and_restores_the_env(tmp_path, monkeypatch):
    monkeypatch.setenv("DIGIMON_DCGO_ROOT", "/some/dcgo")
    seen = {}

    def fake_bind(card_ids, scenarios_dir, verdicts_path, *, source_desc=None):
        seen["env"] = os.environ.get("DIGIMON_DCGO_ROOT")
        return {"cards": {}}

    build_readiness([], scenarios_dir=tmp_path, verdicts_dir=tmp_path, bind_fn=fake_bind)
    assert seen["env"] == ""
    assert os.environ["DIGIMON_DCGO_ROOT"] == "/some/dcgo"


def test_real_bind_admits_adjudicated_clauses_and_drops_a_text_drifted_card(tmp_path):
    # End to end through the real exam_binding.bind: the verdict store's
    # text_sha256 invalidation (spec §4.7) must drop a card back to not_ready.
    from tools.clause_coverage.exam_binding import clause_text_sha256
    from tools.clause_coverage.extract import run as extract_run

    card = "EX10-025"
    clauses = extract_run([card], "test")["clauses"]
    assert clauses, "fixture card must print at least one clause"

    def write_store(sha_for):
        store = tmp_path / "verdicts"
        store.mkdir(exist_ok=True)
        rows = {
            c["id"]: {"clause_id": c["id"], "card_id": card, "verdict": "confirmed",
                      "text_sha256": sha_for(c)}
            for c in clauses
        }
        (store / f"{card}.json").write_text(json.dumps({"version": 1, "clauses": rows}),
                                            encoding="utf-8")
        return store

    ok = write_store(lambda c: clause_text_sha256(c["text"]))
    data = build_readiness([card], scenarios_dir=tmp_path / "none", verdicts_dir=ok)
    assert data["cards"][card]["status"] == "ready"

    drifted = write_store(lambda c: "0" * 64)
    data = build_readiness([card], scenarios_dir=tmp_path / "none", verdicts_dir=drifted)
    assert data["cards"][card]["status"] == "not_ready"
    assert data["cards"][card]["blocking"] == sorted(c["id"] for c in clauses)
