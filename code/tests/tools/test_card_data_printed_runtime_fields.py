"""Official HTML -> canonical -> runtime regressions for printed metadata loss."""

from dataclasses import replace
import hashlib
import json
from pathlib import Path

import pytest

from tools.card_data.compatibility_diff import diff_runtime_fields
from tools.card_data.errors import SourceSchemaError
from tools.card_data.generate import project_canonical_card
from tools.card_data.models import SourceName, SourceObservation
from tools.card_data.normalization import normalize_card
from tools.card_data.serialization import observation_digest
from tools.card_data.sources.official import OFFICIAL_PARSER_VERSION, parse_card_list


FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/official"
PROVENANCE = json.loads((FIXTURES / "runtime_surface_provenance.json").read_text())


def _observation(card_id, transform=lambda value: value):
    entry = next(row for row in PROVENANCE if row["card_id"] == card_id)
    payload = (FIXTURES / entry["fixture"]).read_bytes()
    assert hashlib.sha256(payload).hexdigest() == entry["fixture_sha256"]
    transformed = transform(payload.decode("utf-8"))
    (printing,) = parse_card_list(transformed,
        source_url=entry["source_url"], category_id=entry["category_id"])
    draft = SourceObservation(schema_version=1, observation_id="0" * 64,
        source=SourceName.OFFICIAL, source_record_key=f"fixture:{card_id}",
        source_url=entry["source_url"], raw_payload_digest=hashlib.sha256(transformed.encode("utf-8")).hexdigest(),
        parser_version=OFFICIAL_PARSER_VERSION, source_schema_version="world-card-list-html-v1",
        first_acquired_at="2026-10-12T00:00:00Z", extracted=printing.extracted)
    return replace(draft, observation_id=observation_digest(draft))


@pytest.mark.parametrize("card_id,expected", [
    ("BT21-009", "＜Raid＞ (When this Digimon attacks, you may change the attack target to 1 of your opponent's unsuspended Digimon with the highest DP.)"),
    ("BT24-032", "[When Linking] 1 of your opponent's Digimon gets -2000 DP for the turn."),
    ("EX11-027", "[All Turns] When this Digimon would leave the battle area, by placing 1 of its link cards as its bottom digivolution card, it doesn't leave."),
])
def test_official_link_box_roundtrip_keeps_effect_separate_from_inherited(card_id, expected):
    observation = _observation(card_id)
    card = normalize_card(card_id, [observation])
    row = project_canonical_card(card)
    assert row["link_effect_description_eng"] == expected
    assert row["inherited_effect_description_eng"] == ""
    assert card.faces[0].inherited_text == ""
    assert expected not in row["effect_description_eng"]
    assert "＜Link＞" not in row["link_effect_description_eng"]
    assert any("＜Link＞" in section["text"] for section in observation.extracted["text_sections"])
    assert card.provenance["link_effect_description_eng"].observation_ids == (observation.observation_id,)
    old = dict(row, link_effect_description_eng=None)
    (diff,) = diff_runtime_fields({card_id: old}, {card_id: row}, {card_id: card})
    assert diff.provenance_observation_ids == (observation.observation_id,)


@pytest.mark.parametrize("card_id", ["BT11-061", "EX11-027"])
def test_official_printed_deck_limit_roundtrip(card_id):
    observation = _observation(card_id)
    card = normalize_card(card_id, [observation])
    row = project_canonical_card(card)
    assert row["max_count_in_deck"] == 50
    assert "You can include up to 50 copies" in row["effect_description_eng"]
    assert card.provenance["max_count_in_deck"].observation_ids == (observation.observation_id,)


@pytest.mark.parametrize("replacement", [
    "[Rule] You can include up to 50 copies of cards with this card's card number in your deck.",
    "(Rule) You can include up to 50 copies of cards with this card's card number in your deck.",
])
def test_printed_deck_limit_accepts_both_rule_marker_spellings(replacement):
    original = "You can include up to 50 copies of cards with this card's card number in your deck."
    observation = _observation("BT11-061", lambda html: html.replace(original, replacement))
    assert project_canonical_card(normalize_card("BT11-061", [observation]))["max_count_in_deck"] == 50


@pytest.mark.parametrize("replacement", [
    "You may reveal up to 50 cards from your deck.",
    "Your opponent can include up to 50 copies of cards with this card's card number in their deck.",
    "If you have a Tamer, you can include up to 50 copies of cards with this card's card number in your deck.",
])
def test_non_deck_limit_sentences_are_not_structured_rules(replacement):
    original = "You can include up to 50 copies of cards with this card's card number in your deck."
    observation = _observation("BT11-061", lambda html: html.replace(original, replacement))
    assert "max_count_in_deck" not in normalize_card("BT11-061", [observation]).fields


def test_true_inherited_text_and_link_mentions_are_not_moved():
    observation = _observation("BT11-061")
    extracted = dict(observation.extracted)
    sections = [{"label": "Inherited Effect", "text": "[Your Turn] When this Digimon gets linked, gain 1 memory."}]
    extracted["text_sections"] = sections
    extracted["faces"] = [dict(extracted["faces"][0], text_sections=sections)]
    observation = replace(observation, extracted=extracted)
    card = normalize_card("BT11-061", [observation])
    assert card.fields["inherited_effect_description_eng"] == sections[0]["text"]
    assert "link_effect_description_eng" not in card.fields


def test_unrecognized_combined_link_header_fails_without_swallowing_effect_text():
    observation = _observation("BT21-009")
    extracted = dict(observation.extracted)
    extracted["text_sections"] = [{"label": "Inherited Effect",
        "text": "＜Link＞ [Appmon] trait: Cost 1 (Unknown reminder.) ＜Raid＞ (An effect.)"}]
    with pytest.raises(SourceSchemaError, match="unsupported combined official Link"):
        normalize_card("BT21-009", [replace(observation, extracted=extracted)])


def test_conflicting_printed_copy_limits_fail_closed():
    original = "You can include up to 50 copies of cards with this card's card number in your deck."
    observation = _observation("BT11-061", lambda html: html.replace(
        original, original + "<br>" + original.replace("50", "8")))
    with pytest.raises(SourceSchemaError, match="conflicting printed deck copy limits"):
        normalize_card("BT11-061", [observation])


def test_printed_copy_limit_is_parsed_not_hardcoded_to_fifty():
    original = "You can include up to 50 copies of cards with this card's card number in your deck."
    observation = _observation("BT11-061", lambda html: html.replace(original, original.replace("50", "8")))
    assert normalize_card("BT11-061", [observation]).fields["max_count_in_deck"] == 8



# Exact immutable v4 observations: provenance and content-addressed identities
# are retained; unlike synthetic mutations below these fixtures must hash.
def _p190_observations():
    rows = json.loads((FIXTURES / "p190_link_observations.json").read_text(encoding="utf-8"))
    observations = [SourceObservation(**dict(row, source=SourceName(row["source"]))) for row in rows]
    assert all(observation_digest(o) == o.observation_id for o in observations)
    return observations


@pytest.mark.parametrize("printing", ["P-190", "P-190_P1", "P-190_P2"])
def test_p190_official_mirrored_link_containers_project_one_trigger(printing):
    observation = next(o for o in _p190_observations() if o.extracted["printing"]["printing_id"] == printing)
    before = json.dumps(observation.extracted, sort_keys=True)
    card = normalize_card("P-190", [observation])
    row = project_canonical_card(card)
    assert row["link_effect_description_eng"] == "[When Linking] ＜Draw 1＞"
    assert row["inherited_effect_description_eng"] == ""
    assert card.faces[0].inherited_text == ""
    assert row["effect_description_eng"].startswith("[On Play] ＜Draw 1＞")
    assert "[When Linking]" not in row["effect_description_eng"]
    assert card.provenance["link_effect_description_eng"].observation_ids == (observation.observation_id,)
    assert json.dumps(observation.extracted, sort_keys=True) == before


def _p190_mutated_explicit(text):
    import copy
    observation = next(o for o in _p190_observations() if o.extracted["printing"]["printing_id"] == "P-190")
    extracted = copy.deepcopy(observation.extracted)
    for sections in [extracted["text_sections"], extracted["faces"][0]["text_sections"]]:
        for section in sections:
            if section["label"] == "Link Effect":
                section["text"] = text
    return replace(observation, extracted=extracted)


@pytest.mark.parametrize("text", ["[When Linking] ＜Draw 2＞", "[When Attacking] ＜Draw 1＞", ""])
def test_p190_conflicting_mirrored_link_containers_fail_closed(text):
    observation = _p190_mutated_explicit(text)
    with pytest.raises(SourceSchemaError, match="conflicting official Link effect containers"):
        normalize_card("P-190", [observation])


def test_p190_mirrored_link_equality_uses_existing_marker_normalization():
    observation = _p190_mutated_explicit("{Hand} ＜Draw 1＞")
    for sections in [observation.extracted["text_sections"], observation.extracted["faces"][0]["text_sections"]]:
        for section in sections:
            if section["label"] == "Inherited Effect":
                section["text"] = section["text"].replace("[When Linking]", "[Hand]")
    assert normalize_card("P-190", [observation]).fields["link_effect_description_eng"] == "[Hand] ＜Draw 1＞"


def test_link_repeated_lines_inside_one_container_are_not_deduplicated():
    observation = _p190_mutated_explicit("[When Linking] ＜Draw 1＞\n[When Linking] ＜Draw 1＞")
    extracted = dict(observation.extracted)
    sections = [s for s in extracted["text_sections"] if s["label"] != "Inherited Effect"]
    extracted["text_sections"] = sections
    extracted["faces"] = [dict(extracted["faces"][0], text_sections=sections)]
    card = normalize_card("P-190", [replace(observation, extracted=extracted)])
    assert card.fields["link_effect_description_eng"] == "[When Linking] ＜Draw 1＞\n[When Linking] ＜Draw 1＞"
