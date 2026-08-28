"""A keyword's kind predicts the prompt shape; that is why this exists."""

from pathlib import Path

from tools.clause_coverage.keyword_brief import load_briefs, lookup

SEMANTICS = Path("docs/digimon-rules/keyword-semantics.md")
INDEX = Path("docs/digimon-rules/rules-index.json")


def test_opt_cost_keywords_expect_a_prompt():
    briefs = load_briefs(SEMANTICS, INDEX)
    evade = lookup(briefs, "Evade")
    assert evade is not None
    assert evade["kind"] == "Opt-cost→Mand"
    assert evade["rule"] == "16-21"
    assert evade["expects_prompt"] is True, "DCGO asks before an Opt-cost keyword resolves"


def test_mandatory_keywords_expect_no_prompt():
    briefs = load_briefs(SEMANTICS, INDEX)
    piercing = lookup(briefs, "Piercing")
    assert piercing is not None
    assert piercing["kind"] == "Mandatory"
    assert piercing["expects_prompt"] is False, (
        "an expect: row on a mandatory keyword desynchronizes the rest of the line"
    )


def test_lookup_tolerates_the_angle_brackets_cards_actually_print():
    briefs = load_briefs(SEMANTICS, INDEX)
    assert lookup(briefs, "<Evade>") == lookup(briefs, "evade")


def test_briefs_carry_pdf_pages_for_the_authoritative_text():
    briefs = load_briefs(SEMANTICS, INDEX)
    evade = lookup(briefs, "Evade")
    assert evade["pdf"] == "general_rule.pdf"
    assert evade["pages"], "a brief must point at the pages, not replace them"


def test_unknown_keyword_returns_none_rather_than_guessing():
    briefs = load_briefs(SEMANTICS, INDEX)
    assert lookup(briefs, "Telekinesis") is None


def test_every_table_row_parses():
    """A row the parser silently drops is a keyword an agent cannot look up."""
    briefs = load_briefs(SEMANTICS, INDEX)
    assert len(briefs) >= 35, f"only parsed {len(briefs)} keywords from the table"
