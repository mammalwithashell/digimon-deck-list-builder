"""Tests for `tools.clause_coverage.extract` against real repo card data.

The task's test requirements are stated in terms of real card IDs (EX12-073,
EX12-018) from the real VB deck (`qa/dcgo-harness/vb_pool.json`), so these
tests exercise the real source-priority resolution end to end against the
actual `data/cards.json` / `data/card_overrides.json` / `data/card_official.json`
rather than a synthetic fixture. Synthetic-fixture edge cases (image cache,
override precedence, non-Digimon inherited-zone skip) live in
`test_clause_coverage_card_sources.py`.
"""

from __future__ import annotations

from tools.clause_coverage.extract import run


def _clauses_for(card_id: str) -> list[dict]:
    result = run([card_id], f"test:{card_id}")
    assert result["cards"] == [card_id]
    return result["clauses"]


def test_ex12_073_four_clauses_with_security_image_required():
    """EX12-073 (no bundle, Option kind) has exactly 4 clauses: the 3 real
    effect-zone clauses on its printed text, plus 1 image-required security
    slot (cards.json's security_effect_description_eng is empty pool-wide,
    and its inherited_effect_description_eng -- which coincidentally holds
    the real security text under the wrong key -- is deliberately not
    trusted; see card_sources.py / README)."""
    clauses = _clauses_for("EX12-073")
    assert len(clauses) == 4

    by_zone: dict[str, list[dict]] = {}
    for c in clauses:
        by_zone.setdefault(c["zone"], []).append(c)

    assert "inherited" not in by_zone  # Option kind -- inherited zone deliberately skipped
    assert [c["timings"] for c in by_zone["effect"]] == [
        ["Use Req."],
        ["Main"],
        ["Main", "Delay"],
    ]

    assert len(by_zone["security"]) == 1
    security_clause = by_zone["security"][0]
    assert security_clause["id"] == "EX12-073#security#0"
    assert security_clause["source"] == "image-required"
    assert security_clause["text"] == ""
    assert security_clause["image_path"].endswith("EX12-073.webp")


def test_ex12_018_keywords_are_separate_clauses():
    """<Progress>, <Piercing>, <Security A. +1> are three distinct clauses,
    not merged into one."""
    clauses = _clauses_for("EX12-018")
    keyword_clauses = [c for c in clauses if c["kind"] == "keyword"]
    assert [c["keyword"] for c in keyword_clauses] == ["Progress", "Piercing", "Security A. +1"]
    assert len({c["id"] for c in keyword_clauses}) == 3


def test_ex12_018_compound_when_digivolving_when_attacking_is_one_clause():
    clauses = _clauses_for("EX12-018")
    compound = [
        c for c in clauses if c["timings"] == ["When Digivolving", "When Attacking", "Once Per Turn"]
    ]
    assert len(compound) == 1
    assert "digivolution cards" in compound[0]["text"]


def test_ex12_018_prefers_bundle_over_cards_json():
    """EX12-018 has a bundle (one of only 3 EX12 cards that do). The
    extractor must use it instead of cards.json's version of the same
    text. Distinguishing signal: cards.json's keyword lines carry an
    explanatory parenthetical reminder after each keyword (e.g. "(While
    attacking, your opponent's effects don't affect this Digimon.)"); the
    bundle's official-DB text does not -- so an empty body on the Progress
    clause proves the bundle path was taken, not cards.json's."""
    clauses = _clauses_for("EX12-018")
    progress = next(c for c in clauses if c["keyword"] == "Progress")
    assert progress["source"] == "bundle"
    assert progress["text"] == ""


def test_digimon_kind_card_gets_inherited_zone_clause():
    """Contrast with EX12-073 (Option): a real Digimon-kind card in the deck
    DOES get its inherited_effect_description_eng parsed. EX12-021's
    inherited text is "[When Attacking] [Once Per Turn] If your hand has 7
    or fewer cards, ＜Draw 1＞ (...)" -- one compound-timing clause plus one
    embedded keyword clause (the documented unconditional-keyword-splitting
    behavior, see text_split.py / README "Known limitations")."""
    clauses = _clauses_for("EX12-021")
    inherited = [c for c in clauses if c["zone"] == "inherited"]
    assert len(inherited) == 2
    assert all(c["source"] == "cards_json" for c in inherited)
    assert inherited[0]["timings"] == ["When Attacking", "Once Per Turn"]
    assert inherited[1]["keyword"] == "Draw 1"


def test_full_vb_deck_denominator_counts_are_internally_consistent():
    from pathlib import Path

    import json

    with open(Path("qa/dcgo-harness/vb_pool.json"), encoding="utf-8") as f:
        pool = json.load(f)
    deck_cards = pool["decks"][0]["cards"]
    distinct = sorted(set(deck_cards))
    # Derived, never pinned. This test asserts INTERNAL CONSISTENCY -- every
    # other assertion below relates the denominator to itself -- so the input
    # size must track whatever the pool currently holds. A hard-coded count
    # (it was `== 20`) broke the moment a2b0d9573 revised the decklist to 22
    # distinct IDs, and that failure said nothing about extractor correctness.
    # What IS worth guarding is that the pool is real and that dedup behaved.
    assert deck_cards, "vb_pool.json's first deck is empty"
    assert 0 < len(distinct) <= len(deck_cards)

    result = run(distinct, "test:vb-deck")
    denom = result["denominator"]
    assert denom["total_clauses"] == len(result["clauses"]) > 0
    assert sum(denom["by_zone"].values()) == denom["total_clauses"]
    assert sum(denom["by_source"].values()) == denom["total_clauses"]
    assert denom["image_required_count"] == denom["by_source"].get("image-required", 0)
    assert len(denom["image_required_clause_ids"]) == denom["image_required_count"]
