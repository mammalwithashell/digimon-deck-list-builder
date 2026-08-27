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
    DCGO_SECURITY_ABSENT,
    DCGO_SECURITY_PRESENT,
    DCGO_SECURITY_UNKNOWN,
    dcgo_security_verdict,
    default_dcgo_root,
    extract_card_clauses,
    is_ingestion_artifact,
)


def test_card_overrides_field_is_tagged_card_overrides_source():
    cards_index = {"X-001": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[On Play] Draw 1."}}
    overrides_index = {"X-001": {"effect_description_eng": "[On Play] Draw 2 instead."}}
    clauses = extract_card_clauses(
        "X-001",
        cards_index=cards_index,
        overrides_index=overrides_index,
        official_index={},
        dcgo_root=None,  # hermetic: never consult a real DCGO checkout
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
        "X-002",
        cards_index=cards_index,
        overrides_index=overrides_index,
        official_index={},
        dcgo_root=None,
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
        dcgo_root=None,
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
        dcgo_root=None,
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
    clauses = extract_card_clauses(
        "X-005", cards_index=cards_index, overrides_index={}, official_index={}, dcgo_root=None
    )
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


# ---------------------------------------------------------------------------
# DCGO as a SECOND source for the security zone
#
# The image-required fallback exists because absence in a lossy source is not
# evidence of absence of a clause (README "Why security gets its own
# fallback"). That principle is untouched -- what these tests add is a source
# that CAN produce positive evidence of absence: DCGO's card script. A script
# that exists and contains no `EffectTiming.SecuritySkill` block is DCGO
# asserting the card has no security effect (verified on EX12-020 Gasamon
# against the card face and cards.json as well).
# ---------------------------------------------------------------------------


def _fake_dcgo_script(root, card_id: str, body: str, *, colour: str = "Yellow") -> None:
    """Write `<root>/Assets/Scripts/CardEffect/<SET>/<COLOUR>/<ID_>.cs`.

    Mirrors the real layout, including the UNDERSCORED filename convention
    (`EX99-001` -> `EX99_001.cs`) and the fact that the colour subdirectory
    is not derivable from the card id.
    """
    set_name = card_id.split("-")[0]
    directory = root / "Assets" / "Scripts" / "CardEffect" / set_name / colour
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{card_id.replace('-', '_')}.cs").write_text(body, encoding="utf-8")


_NO_SECURITY_SCRIPT = """
namespace DCGO.CardEffects.EX99
{
    public class EX99_001 : CEntity_Effect
    {
        // [Your Turn] ... -- no security block anywhere in this file.
        if (timing == EffectTiming.None) { }
    }
}
"""

_HAS_SECURITY_SCRIPT = """
namespace DCGO.CardEffects.EX99
{
    public class EX99_002 : CEntity_Effect
    {
        #region Security
        if (timing == EffectTiming.SecuritySkill)
        {
            string EffectDescription() => "[Security] Draw 1.";
        }
        #endregion
    }
}
"""


def test_dcgo_script_without_a_security_block_suppresses_the_phantom_security_slot(tmp_path):
    """Positive evidence of absence: the script EXISTS and has no
    `SecuritySkill` timing, so the card genuinely prints no [Security] box."""
    cards_index = {
        "EX99-001": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[Main] Do a thing."}
    }
    _fake_dcgo_script(tmp_path, "EX99-001", _NO_SECURITY_SCRIPT)
    clauses = extract_card_clauses(
        "EX99-001",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        dcgo_root=tmp_path,
    )
    assert not any(c.zone == "security" for c in clauses)
    assert not any(c.source == "image-required" for c in clauses)
    # The rest of the card is untouched.
    assert [c.zone for c in clauses] == ["effect"]


def test_dcgo_script_with_a_security_block_keeps_the_image_required_slot(tmp_path):
    """DCGO says the card DOES have a security effect but cannot tell us its
    printed text -- still genuinely image-required."""
    cards_index = {
        "EX99-002": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[Main] Do a thing."}
    }
    _fake_dcgo_script(tmp_path, "EX99-002", _HAS_SECURITY_SCRIPT, colour="Red")
    clauses = extract_card_clauses(
        "EX99-002",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        dcgo_root=tmp_path,
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "image-required"


def test_a_card_dcgo_has_no_script_for_stays_image_required(tmp_path):
    """No script at all is NOT evidence of absence -- the README's principle
    stands untouched for these."""
    cards_index = {
        "EX99-003": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[Main] Do a thing."}
    }
    _fake_dcgo_script(tmp_path, "EX99-001", _NO_SECURITY_SCRIPT)  # a DIFFERENT card
    clauses = extract_card_clauses(
        "EX99-003",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        dcgo_root=tmp_path,
    )
    assert [c.source for c in clauses if c.zone == "security"] == ["image-required"]


def test_an_absent_dcgo_checkout_falls_back_to_todays_behaviour(tmp_path):
    """This package must stay usable where DCGO is absent (a worktree's
    ./DCGO placeholder, a CI checkout). Absent DCGO must NOT silently drop
    slots -- it must behave exactly as before."""
    cards_index = {
        "EX99-004": {"card_kind": CARD_KIND_DIGIMON, "effect_description_eng": "[Main] Do a thing."}
    }
    placeholder = tmp_path / "empty-placeholder"
    placeholder.mkdir()
    for root in (None, tmp_path / "does-not-exist", placeholder):
        clauses = extract_card_clauses(
            "EX99-004",
            cards_index=cards_index,
            overrides_index={},
            official_index={},
            dcgo_root=root,
        )
        assert [c.source for c in clauses if c.zone == "security"] == ["image-required"], root


def test_dcgo_absence_never_overrides_real_printed_security_text(tmp_path):
    """DCGO is only ever consulted as a fallback, never to contradict text we
    actually have."""
    cards_index = {
        "EX99-005": {
            "card_kind": CARD_KIND_DIGIMON,
            "effect_description_eng": "[Main] Do a thing.",
            "security_effect_description_eng": "[Security] Draw 1.",
        }
    }
    _fake_dcgo_script(tmp_path, "EX99-005", _NO_SECURITY_SCRIPT)
    clauses = extract_card_clauses(
        "EX99-005",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        dcgo_root=tmp_path,
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "cards_json"
    assert security[0].timings == ["Security"]


def test_image_cache_still_wins_over_dcgo_absence(tmp_path):
    cards_index = {
        "EX99-006": {"card_kind": CARD_KIND_OPTION, "effect_description_eng": "[Main] Do a thing."}
    }
    _fake_dcgo_script(tmp_path, "EX99-006", _NO_SECURITY_SCRIPT)
    clauses = extract_card_clauses(
        "EX99-006",
        cards_index=cards_index,
        overrides_index={},
        official_index={},
        image_cache={"EX99-006": {"security": "[Security] Place this card in the battle area."}},
        dcgo_root=tmp_path,
    )
    security = [c for c in clauses if c.zone == "security"]
    assert len(security) == 1
    assert security[0].source == "image-cache"


def test_dcgo_security_verdict_reports_its_three_states(tmp_path):
    _fake_dcgo_script(tmp_path, "EX99-001", _NO_SECURITY_SCRIPT)
    _fake_dcgo_script(tmp_path, "EX99-002", _HAS_SECURITY_SCRIPT, colour="Green")
    assert dcgo_security_verdict("EX99-001", tmp_path) == DCGO_SECURITY_ABSENT
    assert dcgo_security_verdict("EX99-002", tmp_path) == DCGO_SECURITY_PRESENT
    assert dcgo_security_verdict("EX99-999", tmp_path) == DCGO_SECURITY_UNKNOWN
    assert dcgo_security_verdict("EX99-001", None) == DCGO_SECURITY_UNKNOWN
    assert dcgo_security_verdict("EX99-001", tmp_path / "nope") == DCGO_SECURITY_UNKNOWN
    assert dcgo_security_verdict("noseparator", tmp_path) == DCGO_SECURITY_UNKNOWN


def test_default_dcgo_root_is_overridable_by_env(tmp_path, monkeypatch):
    monkeypatch.setenv("DIGIMON_DCGO_ROOT", str(tmp_path))
    assert default_dcgo_root() == tmp_path
    monkeypatch.setenv("DIGIMON_DCGO_ROOT", "")
    assert default_dcgo_root() is None


# ---------------------------------------------------------------------------
# Ingestion-artifact filtering
#
# Some clause texts are scrape residue, not card text. The filter is a
# hard-coded blocklist of exact artifact strings, deliberately NOT a
# heuristic -- every entry below is paired with a test that a REAL clause of
# similar shape survives.
# ---------------------------------------------------------------------------


def test_inherited_effect_field_label_is_dropped_and_renumbers_its_zone():
    """Observed on EX12-004 / BT25-001..006: the API prefixes the effect text
    with the literal box label "Inherited Effect", which the splitter emits
    as a content-free leading clause.

    Driven through two bundle sections rather than one splittable string so
    the assertion tests THIS module's drop-and-renumber behaviour and not
    `text_split`'s (evolving) clause-boundary policy: whatever the splitter
    decides, a dropped clause must not consume an index.
    """
    official_index = {
        "X-011": {
            "text_sections": [
                {"label": "Effect", "text": "Inherited Effect"},
                {"label": "Effect", "text": "[On Play] Draw 1."},
            ]
        }
    }
    clauses = extract_card_clauses(
        "X-011", cards_index={}, overrides_index={}, official_index=official_index, dcgo_root=None
    )
    effect = [c for c in clauses if c.zone == "effect"]
    # The drop renumbers the surviving siblings down by one: the [On Play]
    # clause was #effect#1 and becomes #effect#0.
    assert [(c.id, c.text) for c in effect] == [("X-011#effect#0", "Draw 1.")]


def test_the_label_blocklist_is_exact_match_only():
    """Unit-level, so it holds whatever clause boundaries `text_split`
    chooses: the bare label is residue, the same words leading real card
    text are not."""
    assert is_ingestion_artifact("Inherited Effect", kind="untimed", timings=[], keyword=None)
    assert is_ingestion_artifact("inherited effect", kind="untimed", timings=[], keyword=None)
    assert is_ingestion_artifact("Security Effect", kind="untimed", timings=[], keyword=None)
    assert not is_ingestion_artifact(
        "Inherited Effect [End of Your Turn] This Digimon may DNA digivolve.",
        kind="untimed",
        timings=[],
        keyword=None,
    )
    # "Effect" alone is deliberately NOT blocklisted -- it is a real word
    # that ends real clause fragments (ST1-15 splits a sentence into one).
    assert not is_ingestion_artifact("effect.", kind="untimed", timings=[], keyword=None)


def test_a_real_clause_that_merely_starts_with_the_label_text_survives():
    """EX12-001's whole printed text is one clause beginning "Inherited
    Effect [End of Your Turn] ..." -- the blocklist is EXACT-match, so real
    text is never eaten."""
    cards_index = {
        "X-012": {
            "card_kind": CARD_KIND_DIGIEGG,
            "effect_description_eng": (
                "Inherited Effect This Digimon and 1 of your other Digimon may DNA digivolve."
            ),
        }
    }
    clauses = extract_card_clauses(
        "X-012", cards_index=cards_index, overrides_index={}, official_index={}, dcgo_root=None
    )
    effect = [c for c in clauses if c.zone == "effect"]
    assert len(effect) == 1
    assert effect[0].text.startswith("Inherited Effect This Digimon")


def test_mediawiki_key_line_residue_is_dropped():
    """Observed on 33 cards pool-wide: `inherited_effect_description_eng` is
    literally `|applinkdp =` -- a MediaWiki template key that leaked through
    ingestion."""
    cards_index = {
        "X-013": {
            "card_kind": CARD_KIND_DIGIMON,
            "effect_description_eng": "[Main] Do a thing.",
            "inherited_effect_description_eng": "|applinkdp =",
        }
    }
    clauses = extract_card_clauses(
        "X-013", cards_index=cards_index, overrides_index={}, official_index={}, dcgo_root=None
    )
    assert not any(c.zone == "inherited" for c in clauses)
    # Dropping the ONLY clause in a zone renumbers nothing elsewhere.
    assert [c.id for c in clauses if c.zone == "effect"] == ["X-013#effect#0"]


def test_a_real_clause_containing_an_equals_sign_survives():
    cards_index = {
        "X-014": {
            "card_kind": CARD_KIND_DIGIMON,
            "effect_description_eng": "[Main] Treat this Digimon's DP = 5000 for the turn.",
        }
    }
    clauses = extract_card_clauses(
        "X-014", cards_index=cards_index, overrides_index={}, official_index={}, dcgo_root=None
    )
    assert [c.text for c in clauses if c.zone == "effect"] == [
        "Treat this Digimon's DP = 5000 for the turn."
    ]


def test_a_content_free_untimed_clause_is_dropped_but_an_empty_keyword_clause_survives():
    """A keyword clause legitimately carries EMPTY text -- all its content is
    in `keyword` (e.g. EX12-065's <Fortitude>). A blanket "empty text ->
    drop" rule would eat those, so the empty-text filter is guarded on
    untimed + keyword-less + timing-less."""
    official_index = {"X-015": {"text_sections": [{"label": "Effect", "text": "＜Fortitude＞"}]}}
    clauses = extract_card_clauses(
        "X-015", cards_index={}, overrides_index={}, official_index=official_index, dcgo_root=None
    )
    assert [(c.keyword, c.text) for c in clauses] == [("Fortitude", "")]

    assert is_ingestion_artifact("", kind="untimed", timings=[], keyword=None)
    assert not is_ingestion_artifact("", kind="keyword", timings=[], keyword="Fortitude")
    assert not is_ingestion_artifact("", kind="timing", timings=["On Play"], keyword=None)
