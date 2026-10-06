"""Stage prompt templates and the strict renderer (design D14, task 6.2)."""
from __future__ import annotations

import re

import pytest

from tools.card_loop.contracts import STAGES
from tools.card_loop.stages import prompts as P


def test_every_stage_has_a_versioned_template():
    assert set(P.available_stages()) == set(STAGES)
    for stage in STAGES:
        first = P.template_path(stage).read_text(encoding="utf-8").splitlines()[0]
        assert re.fullmatch(r"version: \d+", first), (stage, first)
        assert P.version(stage) == first.split(":", 1)[1].strip()


def test_render_substitutes_fields_and_drops_the_version_header():
    stage = "triage"
    fields = {name: f"<{name}>" for name in P.fields(stage)}
    text = P.render(stage, **fields)
    assert not text.startswith("version:")
    for name in P.fields(stage):
        assert f"<{name}>" in text
    assert "{" + P.fields(stage)[0] + "}" not in text


def test_render_is_strict_about_missing_fields():
    stage = "implement"
    fields = {name: "x" for name in P.fields(stage)}
    missing = sorted(fields)[0]
    del fields[missing]
    with pytest.raises(P.PromptFieldError, match=missing):
        P.render(stage, **fields)


def test_render_is_strict_about_unknown_fields():
    fields = {name: "x" for name in P.fields("review")}
    with pytest.raises(P.PromptFieldError, match="surprise"):
        P.render("review", surprise="y", **fields)


def test_render_refuses_an_unknown_stage():
    with pytest.raises(ValueError, match="unknown stage"):
        P.render("bake_bread")


def test_double_braces_are_literal_and_json_examples_are_not_fields(tmp_path):
    t = tmp_path / "x.md"
    t.write_text('version: 3\nA {{literal}} and {"json": 1} and {name}.\n', encoding="utf-8", newline="\n")
    assert P.parse_template(t.read_text(encoding="utf-8")).fields == ("name",)
    assert P.render_text(t.read_text(encoding="utf-8"), name="N") == 'A {literal} and {"json": 1} and N.\n'


def test_a_template_without_a_version_header_is_refused():
    with pytest.raises(P.PromptFieldError, match="version"):
        P.parse_template("no header\n{x}\n")


def test_values_are_inserted_verbatim_not_re_expanded():
    fields = {name: "x" for name in P.fields("review")}
    first = P.fields("review")[0]
    fields[first] = "{card_id} stays literal"
    assert "{card_id} stays literal" in P.render("review", **fields)


@pytest.mark.parametrize("stage", STAGES)
def test_templates_are_narrow_pointers_not_embedded_contracts(stage):
    text = P.template_path(stage).read_text(encoding="utf-8")
    # D14: point at the contracts, never paste them; never send a worker into
    # an orchestrator skill (those spawn their own sub-agents).
    for banned in ("/batch-implement", "/archetype-campaign", "/dcgo-exam", "/implement-",
                   "/readiness-batch", "/author-set", "Skill("):
        assert banned not in text, (stage, banned)
    assert len(text.splitlines()) < 90, f"{stage} template is not narrow"
    assert "Return" in text and "JSON" in text, stage


@pytest.mark.parametrize("stage", ("author_clause", "author_interaction"))
def test_authoring_templates_point_at_the_exam_mcp(stage):
    text = P.template_path(stage).read_text(encoding="utf-8")
    for tool in ("exam_authoring_guide", "exam_validate", "exam_probe", "exam_keyword_brief"):
        assert tool in text, (stage, tool)
    assert "docs/digimon-rules/" in text


@pytest.mark.parametrize("stage", ("implement", "fix_card", "fix_engine"))
def test_code_templates_point_at_the_dsl_test_api_and_no_approximations(stage):
    text = P.template_path(stage).read_text(encoding="utf-8")
    assert "docs/RUST_DSL_TEST_API.md" in text
    assert "approximation" in text.lower()


def test_author_interaction_frames_the_goal_as_finding_a_divergence():
    text = P.template_path("author_interaction").read_text(encoding="utf-8").lower()
    assert "divergence" in text and "find" in text


def test_every_template_keeps_scratch_out_of_the_worktree():
    # The first pilot's workers left scratch files (`scratch_q.py`, `.tmp_p.py`)
    # in their worktree; the merge skips them now, but the rule belongs in the prompt.
    from tools.card_loop.contracts import STAGES
    from tools.card_loop.stages import prompts
    for stage in STAGES:
        text = prompts.template_text(stage).lower()
        assert "scratch" in text and "temp" in text, stage
        assert prompts.version(stage) == "2", stage
