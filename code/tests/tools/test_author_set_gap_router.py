"""Tests for the flag-for-human path (task 5.1 / 5.2)."""

import os

from tools.author_set.gap_router import (
    FlaggedKeyword,
    blocked_cards,
    cards_using_keyword,
    cards_using_subsystem_keyword,
    route_flagged_keyword,
)


def _c(cid, eff):
    return {"card_id": cid, "effect_description_eng": eff,
            "inherited_effect_description_eng": "", "security_effect_description_eng": ""}


def _write_dcgo(root, set_prefix, color, card_id, body):
    d = os.path.join(root, "Assets", "Scripts", "CardEffect", set_prefix.upper(), color)
    os.makedirs(d, exist_ok=True)
    path = os.path.join(d, f"{card_id.replace('-', '_')}.cs")
    with open(path, "w", encoding="utf-8") as f:
        f.write(body)
    return path


def test_cards_using_keyword_matches_normalized_base():
    cards = {
        "S-1": _c("S-1", "gains [Unchained]"),
        "S-2": _c("S-2", "has [App Fusion -4]"),
        "S-3": _c("S-3", "has [App Fusion -6]"),
        "S-4": _c("S-4", "no keyword here"),
    }
    assert cards_using_keyword("unchained", cards) == ["S-1"]
    # both numeric params collapse to the same base
    assert cards_using_keyword("app fusion", cards) == ["S-2", "S-3"]


def test_subsystem_keyword_uses_dcgo_not_text(tmp_path):
    """A card that only *mentions* [Link] in text but whose DCGO C# implements it
    with standard effects is NOT a Link user; only cards whose DCGO actually calls
    the Link machinery count. This is the BT25 Aegiomon over-flag fix: one Link
    payoff card must not poison its whole archetype slice."""
    root = str(tmp_path / "DCGO")
    # Marsmon-style god: text references [Link] options, DCGO is standard effects.
    _write_dcgo(root, "BT25", "Red", "BT25-020",
                "ActivateClass ac; SelectPermanentEffect spe; ChangeDigimonDP(p, 3000);")
    # Vulcanusmon-style payoff: DCGO actually links cards.
    _write_dcgo(root, "BT25", "Purple", "BT25-075",
                "if (cardSource.CanLink(false)) ILinkCard.LinkCard(...);")
    # Factorial-Area-style: grants Link +1 via ChangeLinkMaxStaticEffect.
    _write_dcgo(root, "BT25", "Purple", "BT25-102",
                "cardEffects.Add(CardEffectFactory.ChangeLinkMaxStaticEffect(1));")
    cards = {
        "BT25-020": _c("BT25-020", "When a [TS] card with [Link] is played ..."),
        "BT25-075": _c("BT25-075", "You may link up to 2 cards ..."),
        "BT25-102": _c("BT25-102", "your [TS] Digimon gain <Link +1>"),
        "BT25-033": _c("BT25-033", "no link here at all"),
    }
    got = cards_using_subsystem_keyword("link", cards, dcgo_root=root, set_prefix="BT25")
    assert got == ["BT25-075", "BT25-102"], got
    # The text-based scanner over-flags BT25-020 (mentions [Link]); the DCGO-based
    # one does not — that is the whole point of the fix.
    assert "BT25-020" in cards_using_keyword("link", cards)
    assert "BT25-020" not in got


def test_subsystem_keyword_falls_back_to_text_when_dcgo_absent(tmp_path):
    """When a card has no DCGO file (e.g. promo, or unreleased), we cannot verify
    usage — fall back to the conservative text mention rather than silently
    declaring it link-free and authoring on a missing primitive."""
    root = str(tmp_path / "DCGO")  # empty — no .cs files
    cards = {
        # printed text carries the bracketed [Link] keyword token (what the text
        # scanner keys on), e.g. a link-requirement card.
        "BT25-075": _c("BT25-075", "Link Requirements [Link] [Appmon] trait: Cost 4."),
        "BT25-033": _c("BT25-033", "no link here at all"),
    }
    got = cards_using_subsystem_keyword("link", cards, dcgo_root=root, set_prefix="BT25")
    assert got == ["BT25-075"], got


def test_subsystem_keyword_unknown_keyword_uses_text(tmp_path):
    """An unrecognized subsystem keyword (no DCGO marker set) degrades to the
    text-based detector rather than returning nothing."""
    root = str(tmp_path / "DCGO")
    cards = {"S-1": _c("S-1", "gains [Unchained]"), "S-2": _c("S-2", "nope")}
    got = cards_using_subsystem_keyword("unchained", cards, dcgo_root=root, set_prefix="S")
    assert got == ["S-1"], got


def test_blocked_cards_union():
    flagged = [
        FlaggedKeyword("unchained", "EX11", ["EX11-001", "EX11-002"]),
        FlaggedKeyword("petrification", "EX11", ["EX11-002", "EX11-009"]),
    ]
    assert blocked_cards(flagged) == {"EX11-001", "EX11-002", "EX11-009"}


def test_route_writes_gap_and_plan_then_idempotent(tmp_path):
    gaps = tmp_path / "RUST_ENGINE_GAPS.md"
    plans = tmp_path / "plans"
    fk = FlaggedKeyword("zephyrlock", "BT99", ["BT99-001", "BT99-050"])

    r1 = route_flagged_keyword(fk, gaps_path=str(gaps), plans_dir=str(plans))
    assert r1["gap_appended"] and r1["plan_written"]
    body = gaps.read_text(encoding="utf-8")
    assert "zephyrlock" in body
    assert "BT99-001" in body
    assert "EXCLUDED from mass-implementation" in body
    plan_text = (plans / "author-set-bt99-zephyrlock.md").read_text(encoding="utf-8")
    assert "Needed from you" in plan_text
    assert "BT99-050" in plan_text

    # second run must not duplicate the gap entry or rewrite the plan
    r2 = route_flagged_keyword(fk, gaps_path=str(gaps), plans_dir=str(plans))
    assert not r2["gap_appended"] and not r2["plan_written"]
    assert gaps.read_text(encoding="utf-8").count("zephyrlock`") == body.count("zephyrlock`")
