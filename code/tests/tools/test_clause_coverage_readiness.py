import json
import os

import pytest

from tools.clause_coverage.readiness import (
    build_readiness,
    card_status,
    clause_is_adjudicated,
    library_decklists,
    render,
    vanilla_confirmed,
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
    report = {"card_id": "V", "total_clauses": 0, "by_verdict": {}, "clauses": [],
              "vanilla_confirmed": True}
    assert card_status(report)["status"] == "ready"


def test_zero_clause_card_without_a_vanilla_confirmation_blocks():
    # Zero extracted clauses is only "prints nothing" when a second source agrees;
    # otherwise the extraction is unresolved and the card must not be admitted.
    report = {"card_id": "V", "total_clauses": 0, "by_verdict": {}, "clauses": []}
    status = card_status(report)
    assert status["status"] == "not_ready"
    assert status["blocking"] == ["V#extraction#unresolved"]


@pytest.mark.parametrize(
    "official, record, expected",
    [
        ({"colors": "Red", "text_sections": []}, {"card_kind": 0}, True),
        # failed official lookup (no colors) -- a skeleton, not "prints nothing"
        ({"text_sections": []}, {}, False),
        ({"colors": "", "text_sections": []}, {}, False),
        (None, {}, False),
        # the official entry looks vanilla but cards.json/overrides print text
        ({"colors": "Red"}, {"effect_description_eng": "[On Play] Draw 1."}, False),
        ({"colors": "Red"}, {"inherited_effect_description_eng": "[Your Turn] +1000 DP"}, False),
        ({"colors": "Red"}, {"security_effect_description_eng": "[Security] Play this."}, False),
        ({"colors": "Red"}, {"xros_req": "[DigiXros -2] A x B"}, False),
        ({"colors": "Red"}, {"dual": {"option": {"effect_text": "[Main] Delete 1."}}}, False),
        ({"colors": "Red"}, {"effect_description_eng": "   "}, True),
    ],
)
def test_vanilla_needs_a_real_official_entry_and_a_textless_card_record(official, record, expected):
    assert vanilla_confirmed(official, record) is expected


def test_failed_official_lookup_with_printed_text_is_not_ready(tmp_path):
    # LM-057's official-mirror entry is a failed-lookup skeleton (no colors, no
    # sections), so extraction yields zero clauses -- yet the card prints an
    # effect and a security effect. A true vanilla (ST1-02) stays ready.
    data = build_readiness(["LM-057", "ST1-02"], scenarios_dir=tmp_path / "none",
                           verdicts_dir=tmp_path / "none")
    assert data["cards"]["LM-057"]["status"] == "not_ready"
    assert data["cards"]["LM-057"]["blocking"] == ["LM-057#extraction#unresolved"]
    assert data["cards"]["ST1-02"]["status"] == "ready"


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


from tools.clause_coverage.readiness import main, rank_plan


def _cards(ready, not_ready):
    cards = {c: {"status": "ready", "total_clauses": 1, "by_verdict": {}, "blocking": []} for c in ready}
    cards.update({c: {"status": "not_ready", "total_clauses": 1, "by_verdict": {},
                      "blocking": [f"{c}#effect#0"]} for c in not_ready})
    return cards


def test_plan_prefers_the_card_that_completes_decklists():
    decks = [["R", "X"], ["R", "X"], ["R", "Y", "Z"]]
    plan = rank_plan(decks, _cards(["R"], ["X", "Y", "Z"]), lambda c: True, limit=3)
    assert plan["decklists_ready_now"] == 0
    assert [p["card_id"] for p in plan["picks"]][0] == "X"
    assert plan["picks"][0]["decklists_completed"] == 2


def test_plan_skips_decklists_the_other_gates_reject():
    decks = [["R", "X"], ["R", "UNREG"]]
    plan = rank_plan(decks, _cards(["R"], ["X", "UNREG"]), lambda c: c != "UNREG", limit=5)
    assert plan["decklists_considered"] == 1
    assert [p["card_id"] for p in plan["picks"]] == ["X"]


def test_plan_ties_break_by_card_id_for_determinism():
    decks = [["A1"], ["B1"]]
    plan = rank_plan(decks, _cards([], ["B1", "A1"]), lambda c: True, limit=2)
    assert [p["card_id"] for p in plan["picks"]] == ["A1", "B1"]


def test_plan_breaks_completion_ties_by_decklists_containing():
    # Spec §4.8: equal completions fall to how many decklists contain the card.
    decks = [["A", "B"], ["B", "C"], ["B", "D"]]
    plan = rank_plan(decks, _cards([], ["A", "B", "C", "D"]), lambda c: True, limit=1)
    assert plan["picks"][0]["card_id"] == "B"
    assert plan["picks"][0]["decklists_containing"] == 3


def test_plan_counts_ready_decklists_and_reports_blockers():
    decks = [["R"], ["R", "X"], ["R", "X", "X"]]
    plan = rank_plan(decks, _cards(["R"], ["X"]), lambda c: True, limit=5)
    assert plan["decklists_ready_now"] == 1
    assert plan["picks"] == [{"card_id": "X", "decklists_completed": 2,
                              "decklists_containing": 2, "blocking": ["X#effect#0"],
                              "archetypes": []}]


def test_plan_names_the_archetypes_a_pick_unblocks():
    # Spec §4.8: the plan reports each pick's archetype(s), so /readiness-batch
    # can group picks that share a scenario library entry.
    decks = [["R", "X"], ["X"], ["Y"], ["X", "UNREG"]]
    plan = rank_plan(decks, _cards(["R"], ["X", "Y", "UNREG"]), lambda c: c != "UNREG", limit=2,
                     archetypes=["Beta", "Alpha", "Gamma", "Delta"])
    assert {p["card_id"]: p["archetypes"] for p in plan["picks"]} == {
        "X": ["Alpha", "Beta"], "Y": ["Gamma"],
    }


def _cli_inputs(tmp_path, cards):
    lib = tmp_path / "lib.json"
    lib.write_text(json.dumps({"archetypes": {"X": {"decklists": [
        {"decklist": json.dumps(cards)},
    ]}}}), encoding="utf-8")
    tested = tmp_path / "tested.json"
    tested.write_text(json.dumps({"card_ids": cards}), encoding="utf-8")
    ledger = tmp_path / "ledger.json"
    ledger.write_text(json.dumps({"cards": {}}), encoding="utf-8")
    return [
        "--library", str(lib), "--verdicts", str(tmp_path / "verdicts"),
        "--scenarios", str(tmp_path / "scenarios"), "--out", str(tmp_path / "out.json"),
        "--tested", str(tested), "--ledger", str(ledger),
    ]


def test_cli_writes_then_check_passes_and_detects_drift(tmp_path, capsys):
    args = _cli_inputs(tmp_path, ["EX10-025", "EX10-025"])
    out = tmp_path / "out.json"
    assert main(["--check", *args]) == 1  # missing
    assert main(args) == 0
    first = out.read_bytes()
    assert b"\r\n" not in first
    data = json.loads(first)
    assert data["cards"]["EX10-025"]["status"] == "not_ready"  # empty verdict store
    assert data["decklists"] == {"oracle_ready": 0, "total": 1}
    assert main(["--check", *args]) == 0
    assert main(args) == 0 and out.read_bytes() == first  # byte-deterministic
    out.write_text(first.decode("utf-8").replace("not_ready", "ready", 1), encoding="utf-8")
    assert main(["--check", *args]) == 1
    assert "stale" in capsys.readouterr().err


def test_cli_plan_json_ranks_from_the_artifact(tmp_path, capsys):
    args = _cli_inputs(tmp_path, ["EX10-025"])
    assert main(args) == 0
    capsys.readouterr()
    assert main(["--plan", "--json", "--limit", "3", *args]) == 0
    plan = json.loads(capsys.readouterr().out)
    assert plan["decklists_considered"] == 1
    assert [p["card_id"] for p in plan["picks"]] == ["EX10-025"]
