"""dcgo-harness plumbing for the stage executors: binary, argv, parsers, deck
books, and the prompt-mismatch router (design D4)."""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools.card_loop.stages import harness as H
from tools.card_loop.stages.testing import make_ctx

# ---------------------------------------------------------------- binary


def test_env_override_wins(tmp_path):
    assert H.resolve_harness_bin(tmp_path, env={"DCGO_HARNESS_BIN": "X:/h.exe"}) == "X:/h.exe"


def test_per_worktree_cargo_target_is_found(tmp_path):
    repo = tmp_path / "my-worktree"
    repo.mkdir()
    base = tmp_path / "cargo-target"
    exe = base / "my-worktree" / "debug" / H._exe("dcgo-harness")
    exe.parent.mkdir(parents=True)
    exe.write_text("")
    assert H.resolve_harness_bin(repo, cargo_target_base=str(base), env={}) == str(exe)


def test_in_tree_target_debug_is_found(tmp_path):
    exe = tmp_path / "target" / "debug" / H._exe("dcgo-harness")
    exe.parent.mkdir(parents=True)
    exe.write_text("")
    assert H.resolve_harness_bin(tmp_path, cargo_target_base=str(tmp_path / "none"), env={}) == str(exe)


def test_ctx_harness_bin_overrides_resolution(tmp_path):
    ctx = make_ctx(tmp_path, harness_bin="C:/x/dcgo-harness.exe")
    assert H.harness_bin(ctx) == "C:/x/dcgo-harness.exe"


# ---------------------------------------------------------------- argv


def test_sim_argv_omits_the_default_book_and_carries_inspect(tmp_path):
    ctx = make_ctx(tmp_path)
    argv = H.sim_argv(ctx, "qa/dcgo-exams/ST23/a.yaml", "data/starter_decks.json", inspect=4)
    assert argv[:3] == ["dcgo-harness", "--root", "H:/root"]
    assert "--decks" not in argv
    assert argv[-2:] == ["--inspect", "4"]
    argv = H.sim_argv(ctx, "s.yaml", "qa/dcgo-exams/ST23/pool.json")
    assert argv[argv.index("--decks") + 1] == "qa/dcgo-exams/ST23/pool.json"
    assert "--sim-only" in argv and "--oracle" not in argv


def test_oracle_argv_is_the_one_call_shape(tmp_path):
    (tmp_path / "data").mkdir()
    (tmp_path / "data" / "interaction_denominator.json").write_text("{}")
    ctx = make_ctx(tmp_path)
    argv = H.oracle_argv(ctx, "s.yaml", "qa/dcgo-exams/ST23/pool.json", clause_text_json="c.json")
    text = " ".join(argv)
    assert argv[:5] == ["dcgo-harness", "--root", "H:/root", "exam", "--oracle"]
    for part in ("--build P:/player", "--scenario s.yaml", "--cards-json data/cards.json",
                 "--decks qa/dcgo-exams/ST23/pool.json", "--verdicts --clause-text-json c.json",
                 "--backfill", "--interaction-denominator data/interaction_denominator.json",
                 "--oracle-timeout 300"):
        assert part in text, part


def test_verdict_triage_argv(tmp_path):
    ctx = make_ctx(tmp_path)
    assert H.verdict_triage_argv(ctx, "BT7-056#effect#0", "dcgo_quirk", "general_rule.pdf 16-36") == [
        "dcgo-harness", "verdict-triage", "--clause", "BT7-056#effect#0", "--triage", "dcgo_quirk",
        "--citation", "general_rule.pdf 16-36"]


# ---------------------------------------------------------------- deck books


def _book(path: Path, names):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"decks": [{"name": n, "cards": []} for n in names]}), encoding="utf-8")


def test_deck_books_follow_exam_sim_all_candidates(tmp_path):
    (tmp_path / "data").mkdir()
    (tmp_path / "data" / "starter_decks.json").write_text(
        json.dumps({"starter_decks": [{"id": "a"}, {"id": "b"}]}), encoding="utf-8")
    _book(tmp_path / "qa/dcgo-exams/AAA/other.json", ["a", "b"])
    _book(tmp_path / "qa/dcgo-exams/ST23/pool.json", ["a", "b", "c"])
    _book(tmp_path / "qa/dcgo-exams/ST23/narrow.json", ["a"])
    sc = tmp_path / "qa/dcgo-exams/ST23/x.yaml"
    sc.write_text("decks:\n  p0: {rest: a}\n  p1: {rest: b}\n", encoding="utf-8")
    assert H.deck_books_for(tmp_path, "qa/dcgo-exams/ST23/x.yaml") == [
        "qa/dcgo-exams/ST23/pool.json", "qa/dcgo-exams/AAA/other.json", "data/starter_decks.json"]


# ---------------------------------------------------------------- sim-only output


SIM_PASS = """exam: qa/dcgo-exams/ST23/x.yaml
  lowered 4 step(s): [Action(1)]
  note: step 2 expect.prompt 'SelectCountEffect' not asserted sim-side (kind EffectChoice has no unambiguous DCGO prompt mapping); DCGO will assert it strictly
  assert: 3 check(s) over 1 assertion block(s), 0 failed
exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0
exam: mode sim-only (no oracle: this can only re-check what a previous oracle run confirmed)
"""

SIM_FAIL = """exam: qa/dcgo-exams/ST23/x.yaml
  lowered 4 step(s): [Action(1)]
  ASSERT FAILED: at 3: p0.memory expected 2, got 1
  assert: 3 check(s) over 1 assertion block(s), 1 failed
  FAILED: 1 assertion(s) failed
exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 1
"""


def test_sim_pass_needs_exit_zero_and_failed_zero():
    r = H.parse_sim_output(0, SIM_PASS)
    assert r.passed and not r.failures
    assert r.assert_line.startswith("assert: 3 check(s)")
    assert any("SelectCountEffect" in n for n in r.notes)
    assert not H.parse_sim_output(1, SIM_PASS).passed


def test_sim_failure_lines_are_kept_verbatim():
    r = H.parse_sim_output(1, SIM_FAIL)
    assert not r.passed
    assert r.failures == ["ASSERT FAILED: at 3: p0.memory expected 2, got 1", "FAILED: 1 assertion(s) failed"]
    assert "p0.memory" in r.failure_text()


def test_a_harness_that_did_not_start_is_a_failure():
    r = H.parse_sim_output(None, "", "dcgo-harness could not be started: [WinError 2]")
    assert not r.passed and "WinError 2" in r.failure_text()


def test_inspect_payload_is_parsed_out_of_the_noise():
    out = SIM_PASS.replace("  assert:", json.dumps(
        {"snapshot": {"step": 2, "pending_kind": "OwnField", "pending_optional": False,
                      "pending_prompt": "pick", "candidates": [[100, "ST1-02"]]},
         "projection": {"turn": 3}, "complete": True, "steps_run": 4}, indent=2) + "\n  assert:", 1)
    v = H.parse_inspect(out)
    assert v["snapshot"]["pending_kind"] == "OwnField"
    assert H.parse_inspect("no json here") is None


# ---------------------------------------------------------------- oracle output


def _oracle_line(**kw):
    base = {"scenario": "qa/dcgo-exams/ST23/x.yaml", "clause": "ST23-04#effect#0",
            "ids": ["ST23-04#effect#0"], "verdict": "confirmed", "first_divergence": None,
            "divergence": None, "reason": None, "denominator": "12 rows", "job_id": "exam-x",
            "job_outcome": "completed", "sidecar": "s.jsonl", "backfilled": True,
            "backfill_note": "ok", "recorded": ["ST23-04#effect#0"], "refused": []}
    base.update(kw)
    return json.dumps(base)


def test_oracle_lines_skip_notes():
    out = "note: preflight GO\n" + _oracle_line() + "\nnot json {\n"
    rows = H.parse_oracle_output(0, out, "exam --oracle: scenarios 1 / confirmed 1")
    assert len(rows) == 1 and rows[0]["verdict"] == "confirmed"


def test_prompt_mismatch_needs_a_failed_job():
    msg = "DCGO job failed: prompt mismatch: step 2 expected prompt 'main_phase' but DCGO asked 'OptionalSkill'"
    pm = H.prompt_mismatch(json.loads(_oracle_line(verdict="unmeasured", job_outcome="failed", reason=msg)))
    assert pm == H.PromptMismatch(row=2, expected="main_phase", asked="OptionalSkill")
    assert H.prompt_mismatch(json.loads(_oracle_line(verdict="unmeasured", job_outcome="partial",
                                                     reason=msg))) is None
    assert H.prompt_mismatch(json.loads(_oracle_line(verdict="unmeasured", job_outcome="failed",
                                                     reason="DCGO job failed: bad deck"))) is None


# ---------------------------------------------------------------- the prompt router


def _snap(kind=None, optional=None, n=0):
    return {"step": 0, "pending_kind": kind, "pending_optional": optional,
            "pending_prompt": None, "candidates": [[i, None] for i in range(n)]}


MISSING_DECLINE = H.PromptMismatch(row=5, expected="main_phase", asked="OptionalSkill")


def test_missing_decline_is_an_engine_disagreement():
    # Our engine was at an action prompt (no decline offered), as the scenario
    # expected; DCGO offered an optional decline.
    r = H.decide_prompt_route(MISSING_DECLINE, _snap())
    assert r.route == "engines_disagree" and r.ours == H.ACTION


def test_both_engines_asked_the_same_unexpected_prompt_means_the_scenario_is_wrong():
    r = H.decide_prompt_route(MISSING_DECLINE, _snap("Replacement", True, 2))
    assert r.route == "scenario_wrong" and r.ours == "OptionalSkill"


def test_selection_kinds_map_onto_dcgo_prompts():
    pm = H.PromptMismatch(row=3, expected="SelectHandEffect", asked="SelectCardEffect")
    assert H.decide_prompt_route(pm, _snap("Hand", False, 3)).route == "engines_disagree"
    assert H.decide_prompt_route(pm, _snap("Trash", False, 3)).route == "scenario_wrong"
    assert H.decide_prompt_route(pm, _snap()).route == "scenario_wrong"   # ours asked nothing


def test_optional_gate_fold_counts_as_matching_optional_skill():
    pm = H.PromptMismatch(row=3, expected="OptionalSkill", asked="SelectPermanentEffect")
    assert H.decide_prompt_route(pm, _snap("OppField", True, 2)).route == "engines_disagree"


def test_trigger_order_maps_by_candidate_count():
    pm = H.PromptMismatch(row=3, expected="MultipleSkills", asked="OptionalSkill")
    assert H.decide_prompt_route(pm, _snap("TriggerOrder", False, 2)).route == "engines_disagree"
    assert H.decide_prompt_route(pm, _snap("TriggerOrder", True, 1)).route == "scenario_wrong"


def test_an_unmapped_kind_is_undetermined_not_guessed():
    pm = H.PromptMismatch(row=3, expected="SelectCountEffect", asked="generic_int")
    r = H.decide_prompt_route(pm, _snap("EffectChoice { labels: 2 }", False, 2))
    assert r.route == "undetermined"
    assert H.decide_prompt_route(pm, None).route == "undetermined"


def test_rows_map_back_to_scenario_steps_by_expected_prompt():
    steps = [{"expect": {"prompt": "breeding_action"}}, {"expect": {"prompt": "main_phase"}},
             {"do": {"select": {}}}, {"expect": {"prompt": "main_phase"}},
             {"expect": {"prompt": "SelectHandEffect"}}]
    assert H.scenario_step_for_row(steps, H.PromptMismatch(7, "SelectHandEffect", "x")) == (4, "expect")
    assert H.scenario_step_for_row(steps, H.PromptMismatch(4, "main_phase", "x")) == (3, "expect-nearest")
    assert H.scenario_step_for_row(steps, H.PromptMismatch(9, "OptionalSkill", "x")) == (4, "row")
