"""Tests for `tools.clause_coverage.card_sources` against synthetic fixtures.

Isolated from real repo data (unlike `test_clause_coverage_extract.py`) so
these can exercise edge cases (override precedence, image-cache fallback,
non-Digimon inherited-zone skip, bundle-confirmed-absent security) without
depending on which real cards happen to have which data today.
"""

from __future__ import annotations

from tools.clause_coverage.card_sources import (
    CARD_KIND_DIGIEGG,
    CARD_KIND_DIGIMON,
    CARD_KIND_DUAL,
    CARD_KIND_OPTION,
    CARD_KIND_TAMER,
    extract_card_clauses,
)


def test_card_overrides_field_is_tagged_card_overrides_source():
    cards_index = {"X-001": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[On Play] Draw 1."}}
    overrides_index = {"X-001": {"effect_description_eng": "[On Play] Draw 2 instead."}}
    clauses = extract_card_clauses(
        "X-001", cards_index=cards_index, overrides_index=overrides_index, official_index={}
    )
    # +1: every card without a bundle always gets a security-zone slot
    # (here image-required, since no security text is given at all).
    effect_clauses = [c for c in clauses if c.zone == "effect"]
    assert len(effect_clauses) == 1
    assert effect_clauses[0].source == "card_overrides"
    assert effect_clauses[0].text == "Draw 2 instead."
    assert [c.source for c in clauses if c.zone == "security"] == ["image-required"]


def test_untouched_field_stays_cards_json_source_even_with_an_override_present():
    cards_index = {
        "X-002": {
            "card_kind": CARD_KIND_DIGIMON,
            "effect_description_eng": "[On Play] Draw 1.",
            "inherited_effect_description_eng": "[Your Turn] +1000 DP.",
        }
    }
    overrides_index = {"X-002": {"type_eng": ["VB"]}}  # touches an unrelated field
    clauses = extract_card_clauses(
        "X-002", cards_index=cards_index, overrides_index=overrides_index, official_index={}
    )
    sources = {c.zone: c.source for c in clauses}
    # security is present too (image-required, no security text given) --
    # only effect/inherited sourcing is under test here.
    assert sources["effect"] == "cards_json"
    assert sources["inherited"] == "cards_json"
    assert sources["security"] == "image-required"


def test_image_cache_backfills_missing_security_text():
    cards_index = {"X-003": {"card_kind": CARD_KIND_OPTION, "effect_description_eng": "[Main] Do a thing."}}
    image_cache = {"X-003": {"security": "[Security] Place this card in the battle area."}}
    clauses = extract_card_clauses(
        "X-003",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        image_cache=image_cache,
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "image-cache"
    assert security[0].timings == ["Security"]


def test_no_source_for_security_falls_back_to_image_required_with_custom_img_dir():
    cards_index = {"X-004": {"card_kind": CARD_KIND_OPTION, "effect_description_eng": "[Main] Do a thing."}}
    clauses = extract_card_clauses(
        "X-004",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        img_dir=r"C:\fake\image\dir",
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "image-required"
    assert security[0].text == ""
    assert security[0].image_path == r"C:\fake\image\dir\X-004.webp"


def test_tamer_kind_inherited_field_is_not_trusted():
    cards_index = {
        "X-005": {
            "card_kind": CARD_KIND_TAMER,
            "effect_description_eng": "[Your Turn] Do a thing.",
            # Observed real-data pattern: this field holds security text for
            # Tamer/Option kinds, not a genuine inherited effect.
            "inherited_effect_description_eng": "[Security] Play this card without paying the cost.",
        }
    }
    clauses = extract_card_clauses("X-005", cards_index=cards_index, overrides_index={}, official_index={})
    assert not any(c.zone == "inherited" for c in clauses)
    # And the security zone still correctly falls through to image-required
    # rather than silently adopting the untrusted inherited-field text.
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "image-required"


def test_digimon_and_digiegg_kinds_do_get_inherited_zone():
    for kind in (CARD_KIND_DIGIMON, CARD_KIND_DIGIEGG):
        cards_index = {
            "X-006": {
                "card_kind": kind,
                "effect_description_eng": "[On Play] Draw 1.",
                "inherited_effect_description_eng": "[Your Turn] +1000 DP.",
            }
        }
        clauses = extract_card_clauses(
            "X-006", cards_index=cards_index, overrides_index={}, official_index={}
        )
        inherited = [c for c in clauses if c.zone == "inherited"]
        assert len(inherited) == 1, f"kind={kind}"
        assert inherited[0].source == "cards_json"


def test_dual_card_option_face_effect_text_is_extracted():
    cards_index = {
        "X-007": {
            "card_kind": CARD_KIND_DUAL,
            "effect_description_eng": "[When Attacking] Do the digimon-face thing.",
            "dual": {
                "option": {"effect_text": "[Main] Do the option-face thing."},
            },
        }
    }
    clauses = extract_card_clauses("X-007", cards_index=cards_index, overrides_index={}, official_index={})
    effect_texts = [c.text for c in clauses if c.zone == "effect"]
    assert "Do the digimon-face thing." in effect_texts
    assert "Do the option-face thing." in effect_texts
    # Dual kind is excluded from the inherited zone, same as Tamer/Option.
    assert not any(c.zone == "inherited" for c in clauses)


def test_bundle_with_no_security_section_is_confirmed_absent_not_image_required():
    official_index = {
        "X-008": {
            "text_sections": [
                {"label": "Effect", "text": "[On Play] Draw 1."},
                # No "Security" section at all.
            ]
        }
    }
    clauses = extract_card_clauses(
        "X-008", cards_index={}, overrides_index={}, official_index=official_index
    )
    assert not any(c.zone == "security" for c in clauses)
    assert not any(c.source == "image-required" for c in clauses)


def test_bundle_security_section_is_used_when_present():
    official_index = {
        "X-009": {
            "text_sections": [
                {"label": "Effect", "text": "[On Play] Draw 1."},
                {"label": "Security", "text": "[Security] Place this card in the battle area."},
            ]
        }
    }
    clauses = extract_card_clauses(
        "X-009", cards_index={}, overrides_index={}, official_index=official_index
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "bundle"
    assert security[0].timings == ["Security"]


def test_clause_ids_are_stable_across_repeated_extraction():
    cards_index = {
        "X-010": {
            "card_kind": CARD_KIND_DIGIMON,
            "effect_description_eng": "[On Play] [When Attacking] Draw 1. [Security] Place this in play.",
        }
    }
    first = extract_card_clauses("X-010", cards_index=cards_index, overrides_index={}, official_index={})
    second = extract_card_clauses("X-010", cards_index=cards_index, overrides_index={}, official_index={})
    assert [c.id for c in first] == [c.id for c in second]
    assert [c.to_dict() for c in first] == [c.to_dict() for c in second]
