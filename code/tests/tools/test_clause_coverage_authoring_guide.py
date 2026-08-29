"""The authoring guide is a projection of docs/DCGO_EXAM.md, not a second copy.

Two prose copies of one contract diverge within a release; a projection cannot.
"""

import pytest

from tools.clause_coverage.authoring_guide import TOPICS, build_guide

DOC = """
# DCGO Exam

## Scenario format

The six top-level keys are card, clause, seed, decks, steps, assert.
A clause id is `{card_id}#{zone}#{idx}`.

## Step vocabulary

`do:` is symbolic: hatch, pass, move, play, digivolve, attack, main, select.
`main: {on: field.0}` activates a permanent already in play.

## Prompt kinds

There are 13 prompt kinds plus two folds.

## Decks and stacking

`stack:` is a PREFIX and applies to the initial shuffle only.

## Assertions

`assert` is backfilled from the oracle, never hand-guessed.

## The five verdict classes

confirmed, diverged, unreachable, unavailable, unmeasured.
"""


def test_every_topic_is_populated():
    guide = build_guide(DOC)
    for topic in TOPICS:
        assert topic in guide["topics"], f"missing topic {topic}"
        assert guide["topics"][topic]["body"].strip(), f"topic {topic} is empty"


def test_build_is_deterministic():
    assert build_guide(DOC) == build_guide(DOC)


def test_a_missing_section_fails_loudly_rather_than_shipping_an_empty_topic():
    """An empty topic would answer an agent's question with silence."""
    with pytest.raises(ValueError) as e:
        build_guide("# DCGO Exam\n\nnothing here\n")
    assert "topic" in str(e.value).lower()


def test_the_real_doc_populates_every_topic():
    """Guards the anchors: a heading rename in DCGO_EXAM.md must fail here,
    not silently empty a topic the agent depends on."""
    from pathlib import Path

    doc = Path("docs/DCGO_EXAM.md").read_text(encoding="utf-8")
    guide = build_guide(doc)
    for topic in TOPICS:
        assert len(guide["topics"][topic]["body"]) > 50, f"{topic} looks unpopulated"
