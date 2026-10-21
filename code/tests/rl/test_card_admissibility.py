"""Tests for digimon_gym/agents/card_admissibility.py — the light per-card gate.

Rules (Plan 2 Task 2, rulings R1' and R5): a card is admissible when it is
registered, tested, carries no failed ledger verdict, and has no exam row (clause
or interaction) that is `diverged` without a truncation reason or a cited
`dcgo_quirk` / `stale` triage. Fingerprint drift never voids a diverged row.
"""
import json
import os
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

import digimon_gym.agents.card_admissibility as ca
from digimon_gym.agents.card_admissibility import CardAdmissibility

REPO = Path(__file__).resolve().parents[3]

CARD = "BT1-001"
OTHER = "BT1-002"


def _clause(card_id, idx=0, verdict="confirmed", **extra):
    clause_id = f"{card_id}#effect#{idx}"
    row = {
        "clause_id": clause_id,
        "card_id": card_id,
        "verdict": verdict,
        "text_sha256": "a" * 64,
        "reason": extra.pop("reason", "DIVERGED at step 3" if verdict == "diverged" else ""),
    }
    row.update(extra)
    return clause_id, row


def _exam(*clauses, interactions=()):
    return {
        "version": 1,
        "last_updated": "2026-10-21",
        "clauses": dict(clauses),
        "interactions": dict(interactions),
    }


def _ledger(**statuses):
    return {
        "version": 1,
        "last_updated": "2026-10-21",
        "cards": {cid.replace("_", "-"): {"status": s} for cid, s in statuses.items()},
    }


def _load(
    *,
    registry=(CARD, OTHER),
    tested=(CARD, OTHER),
    dsl=None,
    rust=None,
    exams=(),
    official=None,
):
    return CardAdmissibility.load(
        registry_ids=set(registry),
        tested_ids=set(tested),
        verdicts_dsl=dsl or _ledger(),
        verdicts_rust=rust or _ledger(),
        official=official or {},
        exam_verdicts=list(exams),
    )


# ─── Brief-named rule tests ──────────────────────────────────────────


def test_unregistered_blocks():
    adm = _load(registry=(OTHER,))
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "unregistered"
    # An id no source has ever heard of is unregistered too.
    assert adm.reason("ZZ9-999") == "unregistered"
    assert adm.admissible(OTHER) is True
    assert adm.reason(OTHER) is None


def test_untested_blocks():
    adm = _load(tested=(OTHER,))
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "untested"


@pytest.mark.parametrize(
    "status",
    ["FAILED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS", " audited-drift ", "FAIL", "QA-FAIL", "qa-fail"],
)
@pytest.mark.parametrize("ledger", ["dsl", "rust"])
def test_failed_verdict_blocks(status, ledger):
    adm = _load(**{ledger: _ledger(BT1_001=status)})
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "failed_verdict"
    assert adm.admissible(OTHER) is True


@pytest.mark.parametrize("status", ["PARTIAL", "BLOCKED"])
@pytest.mark.parametrize("ledger", ["dsl", "rust"])
def test_partial_verdict_blocks(status, ledger):
    """Ruling R4-revised: a PARTIAL / BLOCKED card has an unimplemented printed
    clause -- the approximation training must not see -- so it is a failed verdict,
    exactly as the gauntlet's ledger gate rejects it today."""
    adm = _load(**{ledger: _ledger(BT1_001=status)})
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "failed_verdict"


def test_failed_statuses_are_the_ledger_gate_plus_explicit_failures():
    assert ca.NOT_READY_LEDGER_STATUSES == {"PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"}
    assert ca.EXPLICIT_FAILED_STATUSES == {"FAILED", "FAIL", "QA-FAIL"}
    assert ca.FAILED_VERDICT_STATUSES == ca.NOT_READY_LEDGER_STATUSES | ca.EXPLICIT_FAILED_STATUSES


def test_gauntlet_ledger_gate_matches_the_owned_constant():
    """This module owns the status set; the gauntlet's ledger gate must agree."""
    pytest.importorskip("digimon_engine")
    from digimon_gym.agents.gauntlet import _NOT_READY_DSL_STATUSES

    assert _NOT_READY_DSL_STATUSES == ca.NOT_READY_LEDGER_STATUSES


def test_load_and_decisions_need_no_engine_binding():
    """The pure path (`load` + the per-card decision) never imports the engine."""
    script = textwrap.dedent(
        """
        import sys
        sys.modules["digimon_engine"] = None  # any import of it now fails
        import digimon_gym.agents.card_admissibility as ca
        adm = ca.CardAdmissibility.load(
            registry_ids={"BT1-001"},
            tested_ids={"BT1-001"},
            verdicts_dsl={"cards": {"BT1-001": {"status": "PARTIAL"}}},
            verdicts_rust={},
            exam_verdicts=[],
        )
        assert adm.reason("BT1-001") == "failed_verdict"
        assert "digimon_gym.agents.gauntlet" not in sys.modules
        print("ok")
        """
    )
    env = {**os.environ, "PYTHONPATH": str(REPO / "code")}
    out = subprocess.run([sys.executable, "-c", script], env=env, capture_output=True, text=True)
    assert out.returncode == 0, out.stderr
    assert out.stdout.strip() == "ok"


@pytest.mark.parametrize("status", ["IMPLEMENTED", "AUDITED-OK", "PASS", "FIXED", None])
def test_passing_verdict_admits(status):
    adm = _load(dsl=_ledger(BT1_001=status), rust=_ledger(BT1_001=status))
    assert adm.admissible(CARD) is True
    assert adm.reason(CARD) is None


def test_stale_diverged_verdict_still_blocks():
    """Ruling R1' (supersedes the brief's `..._is_unmeasured_and_admits`): a
    diverged row whose text fingerprint no longer matches the printed text is
    still a recorded divergence, so it blocks until triaged. This pins that
    `official` (the text staleness is measured against) is NOT consulted: even
    with text that would mark the row stale, the decision is unchanged."""
    official = {CARD: {"effect": "new printed text that no longer hashes to aaaa…"}}
    adm = _load(exams=[_exam(_clause(CARD, verdict="diverged"))], official=official)
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "diverged_no_quirk"


def test_stale_triage_with_citation_admits():
    row = _clause(CARD, verdict="diverged", triage="stale", citation="re-exam: text errata 2026-10")
    assert _load(exams=[_exam(row)]).admissible(CARD) is True


@pytest.mark.parametrize("triage", ["stale", "dcgo_quirk"])
@pytest.mark.parametrize("citation", [None, "", "   "])
def test_triage_without_citation_blocks(triage, citation):
    extra = {"triage": triage}
    if citation is not None:
        extra["citation"] = citation
    adm = _load(exams=[_exam(_clause(CARD, verdict="diverged", **extra))])
    assert adm.reason(CARD) == "diverged_no_quirk"


def test_truncated_diverged_row_admits():
    row = _clause(
        CARD,
        verdict="diverged",
        reason="TRUNCATED, no divergence found (compared 12 of 13 ours / 12 dcgo steps)",
    )
    assert _load(exams=[_exam(row)]).admissible(CARD) is True


def test_diverged_without_quirk_blocks():
    adm = _load(exams=[_exam(_clause(CARD, verdict="diverged"))])
    assert adm.admissible(CARD) is False
    assert adm.reason(CARD) == "diverged_no_quirk"
    assert adm.admissible(OTHER) is True


def test_truncated_row_triaged_ours_wrong_blocks():
    """An explicit human `ours_wrong` triage outranks an automated
    "ran out of steps" truncation reason."""
    row = _clause(
        CARD,
        verdict="diverged",
        reason="TRUNCATED, no divergence found (compared 12 of 13 ours / 12 dcgo steps)",
        triage="ours_wrong",
        citation="G-456: our trigger fires late",
    )
    assert _load(exams=[_exam(row)]).reason(CARD) == "diverged_no_quirk"


def test_diverged_triaged_ours_wrong_blocks():
    row = _clause(CARD, verdict="diverged", triage="ours_wrong", citation="our bug, G-123")
    assert _load(exams=[_exam(row)]).reason(CARD) == "diverged_no_quirk"


def test_diverged_with_quirk_citation_admits():
    row = _clause(CARD, verdict="diverged", triage="dcgo_quirk", citation="DCGO BT1_001.cs:42")
    assert _load(exams=[_exam(row)]).admissible(CARD) is True


def test_unmeasured_clauses_admit():
    exam = _exam(
        _clause(CARD, 0, verdict="confirmed"),
        _clause(CARD, 1, verdict="unreachable", reason="needs an opponent Tamer"),
        _clause(CARD, 2, verdict="unavailable", reason=""),
        _clause(CARD, 3, verdict="unmeasured"),
        _clause(CARD, 4, verdict="confirmed", text_sha256="stale" * 8),
    )
    adm = _load(exams=[exam])
    assert adm.admissible(CARD) is True
    # A card with no exam file at all admits too.
    assert adm.admissible(OTHER) is True


# ─── Families, attribution, priority ─────────────────────────────────


def test_diverged_interaction_row_blocks_every_card_it_names():
    interaction = (
        "qa:Q123",
        {"card_ids": [CARD, OTHER], "verdict": "diverged", "reason": "DIVERGED at step 9"},
    )
    adm = _load(exams=[_exam(interactions=[interaction])])
    assert adm.reason(CARD) == "diverged_no_quirk"
    assert adm.reason(OTHER) == "diverged_no_quirk"


def test_quirk_interaction_row_admits():
    interaction = (
        "probe:BT1-001#effect#0:timing",
        {
            "card_ids": [CARD],
            "verdict": "diverged",
            "reason": "DIVERGED at step 9",
            "triage": "dcgo_quirk",
            "citation": "DCGO orders the trigger before the rule check",
        },
    )
    assert _load(exams=[_exam(interactions=[interaction])]).admissible(CARD) is True


def test_row_without_card_id_falls_back_to_file_card_and_clause_prefix():
    clause_id, row = _clause(CARD, verdict="diverged")
    del row["card_id"]
    assert _load(exams=[_exam((clause_id, row))]).reason(CARD) == "diverged_no_quirk"
    exam = _exam(interactions=[("qa:Q9", {"verdict": "diverged", "reason": "DIVERGED"})])
    exam["card_id"] = OTHER  # what from_repo stamps from the file name
    assert _load(exams=[exam]).reason(OTHER) == "diverged_no_quirk"


def test_reason_priority_and_all_reasons_recorded():
    adm = _load(
        registry=(OTHER,),
        tested=(OTHER,),
        dsl=_ledger(BT1_001="FAILED"),
        exams=[_exam(_clause(CARD, verdict="diverged"))],
    )
    assert adm.reason(CARD) == "unregistered"
    assert adm.reasons(CARD) == ["unregistered", "untested", "failed_verdict", "diverged_no_quirk"]

    adm = _load(tested=(OTHER,), dsl=_ledger(BT1_001="FAILED"))
    assert adm.reason(CARD) == "untested"
    adm = _load(rust=_ledger(BT1_001="FAILED"), exams=[_exam(_clause(CARD, verdict="diverged"))])
    assert adm.reason(CARD) == "failed_verdict"
    assert adm.reasons(OTHER) == []


# ─── The committed artifact (ruling R5) ──────────────────────────────


def test_build_artifact_shape_and_counts():
    adm = _load(
        registry=(CARD, OTHER, "BT1-003", "BT1-004"),
        tested=(CARD, OTHER, "BT1-004"),
        dsl=_ledger(BT1_004="AUDITED-DRIFT"),
        exams=[_exam(_clause(OTHER, verdict="diverged"))],
    )
    sources = {"tested_cards_sha256": "t", "validated_cards_dsl_sha256": "d",
               "validated_cards_sha256": "r", "exam_verdicts_sha256": "e"}
    art = adm.build_artifact(sources=sources, generated_at="2026-10-21T00:00:00+00:00")
    assert art["version"] == 1
    assert art["generated_at"] == "2026-10-21T00:00:00+00:00"
    assert art["sources"] == sources
    assert art["counts"] == {
        "registered": 4,
        "admissible": 1,
        "unregistered": 0,
        "untested": 1,
        "failed_verdict": 1,
        "diverged_no_quirk": 1,
    }
    assert list(art["cards"]) == sorted(art["cards"])
    assert art["cards"][CARD] == {"admissible": True, "reason": None, "reasons": []}
    assert art["cards"]["BT1-003"]["reason"] == "untested"
    assert art["cards"]["BT1-004"]["reason"] == "failed_verdict"
    assert art["cards"][OTHER]["reason"] == "diverged_no_quirk"

    # The artifact is also the run-time reader (training ships no qa/).
    back = CardAdmissibility.from_artifact_dict(art)
    for cid in (CARD, OTHER, "BT1-003", "BT1-004", "ZZ9-999"):
        assert back.admissible(cid) == adm.admissible(cid)
        assert back.reason(cid) == adm.reason(cid)


def _write_inputs(root: Path, *, dsl_status="IMPLEMENTED"):
    (root / "exam").mkdir(exist_ok=True)
    (root / "tested.json").write_text(
        json.dumps({"version": 1, "generated_at": "x", "card_count": 2, "card_ids": [CARD, OTHER]}),
        encoding="utf-8",
    )
    (root / "dsl.json").write_text(json.dumps(_ledger(BT1_001=dsl_status)), encoding="utf-8")
    (root / "rust.json").write_text(json.dumps(_ledger()), encoding="utf-8")
    (root / "exam" / f"{OTHER}.json").write_text(
        json.dumps(_exam(_clause(OTHER, verdict="confirmed"))), encoding="utf-8"
    )


def _argv(root: Path, *extra):
    return [
        "--tested", str(root / "tested.json"),
        "--dsl-ledger", str(root / "dsl.json"),
        "--rust-ledger", str(root / "rust.json"),
        "--exam-verdicts", str(root / "exam"),
        "--out", str(root / "card_admissibility.json"),
        *extra,
    ]


@pytest.fixture
def fake_registry(monkeypatch):
    monkeypatch.setattr(ca, "load_registry_ids", lambda: {CARD, OTHER, "BT1-003"})


def test_cli_writes_then_check_passes_and_detects_drift(tmp_path, fake_registry, capsys):
    _write_inputs(tmp_path)
    assert ca.main(_argv(tmp_path)) == 0
    out = capsys.readouterr().out
    assert "3 registered cards: 2 admissible / 0 unregistered / 1 untested / 0 failed_verdict / 0 diverged_no_quirk" in out
    art = json.loads((tmp_path / "card_admissibility.json").read_text(encoding="utf-8"))
    assert set(art["sources"]) == {
        "tested_cards_sha256", "validated_cards_dsl_sha256",
        "validated_cards_sha256", "exam_verdicts_sha256",
    }
    assert ca.main(_argv(tmp_path, "--check")) == 0

    # generated_at is outside the comparison.
    art["generated_at"] = "1999-01-01T00:00:00+00:00"
    (tmp_path / "card_admissibility.json").write_text(json.dumps(art), encoding="utf-8")
    assert ca.main(_argv(tmp_path, "--check")) == 0

    # A ledger change makes the committed artifact stale.
    _write_inputs(tmp_path, dsl_status="FAILED")
    assert ca.main(_argv(tmp_path, "--check")) == 1
    assert ca.main(_argv(tmp_path)) == 0
    assert ca.main(_argv(tmp_path, "--check")) == 0

    # An exam-store change (even one that keeps every decision) is caught by the sha.
    (tmp_path / "exam" / "BT1-003.json").write_text(json.dumps(_exam()), encoding="utf-8")
    assert ca.main(_argv(tmp_path, "--check")) == 1


def test_cli_check_detects_tampered_card(tmp_path, fake_registry):
    _write_inputs(tmp_path)
    assert ca.main(_argv(tmp_path)) == 0
    path = tmp_path / "card_admissibility.json"
    art = json.loads(path.read_text(encoding="utf-8"))
    art["cards"]["BT1-003"] = {"admissible": True, "reason": None, "reasons": []}
    path.write_text(json.dumps(art), encoding="utf-8")
    assert ca.main(_argv(tmp_path, "--check")) == 1


def test_cli_check_fails_when_artifact_missing(tmp_path, fake_registry):
    _write_inputs(tmp_path)
    assert ca.main(_argv(tmp_path, "--check")) == 1


def test_rewrite_without_changes_keeps_generated_at(tmp_path, fake_registry):
    _write_inputs(tmp_path)
    assert ca.main(_argv(tmp_path)) == 0
    path = tmp_path / "card_admissibility.json"
    before = path.read_bytes()
    assert ca.main(_argv(tmp_path)) == 0
    assert path.read_bytes() == before


# ─── The real repo ───────────────────────────────────────────────────


def test_real_repo_counts_add_up():
    pytest.importorskip("digimon_engine")
    adm = CardAdmissibility.from_repo()
    art = adm.build_artifact(sources={}, generated_at="x")
    c = art["counts"]
    assert c["registered"] == len(art["cards"]) > 0
    assert c["registered"] == (
        c["admissible"] + c["unregistered"] + c["untested"]
        + c["failed_verdict"] + c["diverged_no_quirk"]
    )
    assert c["unregistered"] == 0  # the artifact covers registered ids only
    tested = set(json.loads((REPO / "data/tested_cards.json").read_text(encoding="utf-8"))["card_ids"])
    for cid in tested & set(art["cards"]):
        assert art["cards"][cid]["reason"] != "untested", cid


def test_from_artifact_reads_committed_file_and_rejects_other_versions(tmp_path):
    adm = _load(tested=(OTHER,))
    art = adm.build_artifact(sources={}, generated_at="x")
    path = tmp_path / "card_admissibility.json"
    path.write_text(ca.serialise(art), encoding="utf-8")
    back = CardAdmissibility.from_artifact(path)
    assert back.reason(CARD) == "untested" and back.admissible(OTHER) is True
    art["version"] = 99
    with pytest.raises(ValueError, match="version"):
        CardAdmissibility.from_artifact_dict(art)


@pytest.mark.parametrize(
    "entry",
    [
        {"admissible": True, "reason": None, "reasons": ["untested"]},
        {"admissible": False, "reason": "untested", "reasons": []},
        {"admissible": False, "reason": "failed_verdict", "reasons": ["untested", "failed_verdict"]},
        {"admissible": False, "reason": "untested"},  # `reasons` is required
        {"admissible": False, "reason": "bogus", "reasons": ["bogus"]},
    ],
)
def test_from_artifact_rejects_inconsistent_entries(entry):
    art = _load().build_artifact(sources={}, generated_at="x")
    art["cards"][CARD] = entry
    with pytest.raises(ValueError, match=CARD):
        CardAdmissibility.from_artifact_dict(art)


def test_input_override_refuses_the_committed_artifact_path(tmp_path, fake_registry, capsys):
    _write_inputs(tmp_path)
    assert ca.main(["--tested", str(tmp_path / "tested.json")]) == 2
    assert "--out" in capsys.readouterr().err


def test_check_reports_missing_exam_store(tmp_path, fake_registry, capsys):
    _write_inputs(tmp_path)
    assert ca.main(_argv(tmp_path)) == 0
    (tmp_path / "exam" / f"{OTHER}.json").unlink()
    (tmp_path / "exam").rmdir()
    assert ca.main(_argv(tmp_path, "--check")) == 1
    assert "exam-verdict store" in capsys.readouterr().err


def test_check_reports_corrupt_artifact(tmp_path, fake_registry, capsys):
    _write_inputs(tmp_path)
    (tmp_path / "card_admissibility.json").write_text("{not json", encoding="utf-8")
    assert ca.main(_argv(tmp_path, "--check")) == 1
    assert "not valid JSON" in capsys.readouterr().err
