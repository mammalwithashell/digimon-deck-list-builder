"""Fixture-driven contracts for the official Bandai source adapter."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

import pytest

from tools.card_data.errors import CompletenessError, SourceSchemaError
from tools.card_data.serialization import observation_digest
from tools.card_data.sources.official import (
    OFFICIAL_CATALOG_URL,
    OFFICIAL_PARSER_VERSION,
    OfficialSourceAdapter,
    build_scope_snapshot,
    discover_release_set,
    normalize_release_prefix,
    parse_card_list,
    parse_catalog,
    raw_payload_digest,
)


FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/official"


def _fixture(name: str) -> bytes:
    return (FIXTURES / name).read_bytes()


def _category_url(category_id: str) -> str:
    return f"{OFFICIAL_CATALOG_URL}?search=true&category={category_id}"


def test_catalog_discovers_normalized_single_and_range_release_prefixes() -> None:
    entries = parse_catalog(_fixture("catalog.html"))
    by_id = {entry.category_id: entry for entry in entries}

    assert by_id["522037"].release_prefixes == ("EX12",)
    assert by_id["522012"].release_prefixes == ("EX3",)
    assert by_id["522002"].release_prefixes == ("BT1", "BT2", "BT3")
    assert by_id["522901"].release_prefixes == ("P",)
    assert normalize_release_prefix("EX-03") == "EX3"
    assert normalize_release_prefix("P") == "P"


def test_ordinary_and_alternate_printings_are_not_collapsed() -> None:
    printings = parse_card_list(
        _fixture("ex03.html"),
        source_url=_category_url("522012"),
        category_id="522012",
    )

    assert [printing.card_id for printing in printings] == [
        "EX3-001",
        "EX3-010",
        "EX3-010",
    ]
    assert [printing.printing_id for printing in printings] == [
        "EX3-001",
        "EX3-010",
        "EX3-010_P1",
    ]
    assert printings[2].alternate_art is True
    assert printings[0].extracted["faces"][0]["kind"] == "digi_egg"
    assert printings[1].extracted["evolution_routes"] == [
        {"slot": 1, "colors": ["Red", "Purple"], "cost": 2, "from_level": 3}
    ]


def test_incomplete_alternate_art_route_is_retained_as_a_printing_gap() -> None:
    payload = _fixture("ex03.html").decode("utf-8")
    full_route = (
        '<span class="cardColor_red">Red</span><span '
        'class="cardColor_purple">Purple</span> 2 from Lv.3'
    )
    offset = payload.rfind(full_route)
    assert offset > 0
    payload = payload[:offset] + "2 from" + payload[offset + len(full_route) :]

    printings = parse_card_list(
        payload,
        source_url=_category_url("522012"),
        category_id="522012",
    )

    primary, alternate = printings[1:]
    assert primary.extracted["evolution_routes"]
    assert alternate.extracted["evolution_routes"] == []
    assert alternate.extracted["printing_field_gaps"][0]["field"] == "evolution_routes"


def test_alternate_art_route_missing_cost_is_a_printing_gap() -> None:
    payload = _fixture("ex03.html").decode("utf-8")
    full_route = (
        '<span class="cardColor_red">Red</span><span '
        'class="cardColor_purple">Purple</span> 2 from Lv.3'
    )
    offset = payload.rfind(full_route)
    assert offset > 0
    incomplete = (
        '<span class="cardColor_red">Red</span><span '
        'class="cardColor_purple">Purple</span> from Lv.3'
    )
    payload = payload[:offset] + incomplete + payload[offset + len(full_route) :]

    alternate = parse_card_list(
        payload,
        source_url=_category_url("522012"),
        category_id="522012",
    )[2]

    assert alternate.extracted["evolution_routes"] == []
    assert alternate.extracted["printing_field_gaps"][0]["source_values"] == [
        "Red Purple from Lv.3"
    ]


def test_alternate_art_missing_face_metadata_does_not_override_primary() -> None:
    payload = _fixture("ex03.html").decode("utf-8")
    marker = '<li class="cardType">Digimon</li>'
    offset = payload.rfind(marker)
    assert offset > 0
    payload = payload[:offset] + '<li class="cardType"></li>' + payload[offset + len(marker) :]

    alternate = parse_card_list(
        payload,
        source_url=_category_url("522012"),
        category_id="522012",
    )[2]

    assert alternate.extracted["faces"] == []
    assert alternate.extracted["printing_field_gaps"][-1]["field"] == "faces"


def test_rule_grants_special_route_and_qa_are_preserved() -> None:
    (printing,) = parse_card_list(
        _fixture("ex07_rule_grant.html"),
        source_url=_category_url("522024"),
        category_id="522024",
    )
    extracted = printing.extracted

    assert extracted["rule_grants"] == {"traits": ["Ice-Snow"], "attributes": [], "forms": []}
    assert extracted["special_evolution_routes"] == [
        "[Digivolve] Lv.4 w/[Ice-Snow] trait: Cost 3"
    ]
    assert extracted["qa"] == [
        {
            "number": "Q9999",
            "date": "Jun. 1, 2024",
            "question": "Does the Rule text add the trait?",
            "answer": "Yes, this card also has the [Ice-Snow] trait.",
        }
    ]

    (promo,) = parse_card_list(
        _fixture("promo_rule_attribute.html"),
        source_url=_category_url("522901"),
        category_id="522901",
    )
    assert promo.extracted["rule_grants"] == {"traits": [], "attributes": ["Free"], "forms": []}



def test_bracketed_p213_rule_grant_is_preserved() -> None:
    (printing,) = parse_card_list(
        _fixture("promo_bracketed_rule.html"),
        source_url=_category_url("522901"),
        category_id="522901",
    )
    assert printing.card_id == "P-213"
    assert printing.extracted["rule_grants"] == {
        "traits": ["Dragonkin"], "attributes": [], "forms": []
    }
    assert "[Rule] Trait: Has [Dragonkin] Type." in next(
        section["text"] for section in printing.extracted["text_sections"]
        if section["label"] == "Effect"
    )
    assert printing.extracted["special_evolution_routes"] == [
        "[Digivolve] [Aegiomon]: Cost 3"
    ]


@pytest.mark.parametrize("marker", ["[Rule]", "(Rule)", "Rule:"])
@pytest.mark.parametrize("suffix, target", [("Type", "traits"), ("attribute", "attributes")])
def test_rule_grant_delimiters_preserve_traits_and_attributes(marker, suffix, target) -> None:
    payload = _fixture("promo_bracketed_rule.html").decode("utf-8").replace(
        "[Rule] Trait: Has [Dragonkin] Type.",
        f"{marker} Trait: Has [Dragonkin] {suffix}.",
    )
    (printing,) = parse_card_list(payload, source_url=_category_url("522901"), category_id="522901")
    expected = {"traits": [], "attributes": [], "forms": []}
    expected[target] = ["Dragonkin"]
    assert printing.extracted["rule_grants"] == expected


@pytest.mark.parametrize("text", [
    "[On Play] This Digimon gains the [Dragonkin] trait.",
    "[Rule] Trait: Has [Dragonkin] in its text.",
    "[Rule] Trait: Has Dragonkin Type.",
    "SuperRule Trait: Has [Dragonkin] Type.",
    "[Rule) Trait: Has [Dragonkin] Type.",
    "(Rule] Trait: Has [Dragonkin] Type.",
])
def test_rule_grants_do_not_capture_non_grants_or_malformed_markers(text) -> None:
    payload = _fixture("promo_bracketed_rule.html").decode("utf-8").replace(
        "[Rule] Trait: Has [Dragonkin] Type.", text,
    )
    (printing,) = parse_card_list(payload, source_url=_category_url("522901"), category_id="522901")
    assert printing.extracted["rule_grants"] == {"traits": [], "attributes": [], "forms": []}


def test_known_dual_layout_retains_structural_faces() -> None:
    (printing,) = parse_card_list(
        _fixture("bt25_dual.html"),
        source_url=_category_url("522036"),
        category_id="522036",
    )
    extracted = printing.extracted

    assert extracted["card_id"] == "BT25-043"
    assert extracted["layout"] == "dual"
    # The official row has no Digimon-face play cost.  Keep the immutable
    # source-shaped observation faithful; normalization owns the lossy
    # top-level compatibility projection from the Option face's use cost.
    assert extracted["play_cost"] is None
    assert len(extracted["faces"]) == 2
    assert extracted["faces"][0]["name"] == "Habakirimon"
    assert extracted["faces"][0]["play_cost"] is None
    assert extracted["faces"][1] == {
        "face_id": "dual",
        "kind": "option",
        "name": "Habakiri",
        "colors": ["Yellow"],
        "use_cost": 6,
        "text_sections": [
            {
                "label": "DUAL Effect",
                "text": "Use Req. ([Glowing Dawn] trait).\n[Main] 1 Digimon gets -8000 DP.",
            },
            {"label": "DUAL Rule", "text": "Arts Digivolve."},
        ],
    }
    assert extracted["qa"][0]["number"] == "Q6312"


def test_appmon_grade_route_is_structural_without_inventing_a_level() -> None:
    payload = _fixture("bt25_dual.html").decode("utf-8").replace(
        '<span class="cardColor_yellow">Yellow</span> 4 from Lv.5',
        '<span class="cardColor_all">Multicolor</span> 2 from 並<br>Stnd.',
    )
    (printing,) = parse_card_list(
        payload,
        source_url=_category_url("522036"),
        category_id="522036",
    )

    assert printing.extracted["evolution_routes"] == []
    assert printing.extracted["grade_evolution_routes"] == [
        {
            "slot": 1,
            "colors": ["Multicolor"],
            "cost": 2,
            "from_grade": "Standard",
        }
    ]


def test_official_token_blocks_are_not_admitted_as_gameplay_cards() -> None:
    payload = _fixture("bt25_dual.html").decode("utf-8").replace(
        "BT25-043", "BT25-TOKEN"
    )

    assert parse_card_list(
        payload,
        source_url=_category_url("522036"),
        category_id="522036",
    ) == ()


def test_historical_official_digi_eg_spelling_normalizes_by_value() -> None:
    payload = _fixture("ex03.html").decode("utf-8").replace(
        '<li class="cardType">Digi-Egg</li>',
        '<li class="cardType">Digi-Eg</li>',
        1,
    )
    first = parse_card_list(
        payload,
        source_url=_category_url("522012"),
        category_id="522012",
    )[0]

    assert first.extracted["card_type"] == "Digi-Egg"
    assert first.extracted["faces"][0]["kind"] == "digi_egg"


def test_hybrid_tamer_route_is_structural_without_inventing_a_level() -> None:
    payload = _fixture("bt25_dual.html").decode("utf-8").replace(
        '<span class="cardColor_yellow">Yellow</span> 4 from Lv.5',
        '<span class="cardColor_yellow">Yellow</span> 2 from TAMER',
    )
    (printing,) = parse_card_list(
        payload,
        source_url=_category_url("522036"),
        category_id="522036",
    )

    assert printing.extracted["evolution_routes"] == []
    assert printing.extracted["tamer_evolution_routes"] == [
        {
            "slot": 1,
            "colors": ["Yellow"],
            "cost": 2,
            "from_kind": "Tamer",
        }
    ]


def test_all_color_level_route_expands_to_the_seven_gameplay_colors() -> None:
    payload = _fixture("bt25_dual.html").decode("utf-8").replace(
        '<span class="cardColor_yellow">Yellow</span> 4 from Lv.5',
        '<span class="cardColor_all">All</span> 0 from Lv.2',
    )
    (printing,) = parse_card_list(
        payload,
        source_url=_category_url("522036"),
        category_id="522036",
    )

    assert printing.extracted["evolution_routes"] == [
        {
            "slot": 1,
            "colors": ["Red", "Blue", "Yellow", "Green", "White", "Black", "Purple"],
            "cost": 0,
            "from_level": 2,
        }
    ]


def test_release_discovery_returns_distinct_ids_and_retains_printings() -> None:
    pages = {_category_url("522012"): _fixture("ex03.html")}
    result = discover_release_set(
        _fixture("catalog.html"),
        release_prefix="EX-03",
        fetch_page=pages.__getitem__,
    )

    assert result.scope == "release-set:EX3"
    assert result.complete is True
    assert result.gameplay_card_ids == ("EX3-001", "EX3-010")
    assert len(result.printings) == 3
    assert {printing.printing_id for printing in result.printings} == {
        "EX3-001",
        "EX3-010",
        "EX3-010_P1",
    }


def test_release_discovery_rejects_missing_or_empty_scope() -> None:
    with pytest.raises(CompletenessError):
        discover_release_set(
            _fixture("catalog.html"),
            release_prefix="BT99",
            fetch_page=lambda _url: b"",
        )

    with pytest.raises(SourceSchemaError):
        discover_release_set(
            _fixture("catalog.html"),
            release_prefix="EX12",
            fetch_page=lambda _url: b"<html><body>No cards</body></html>",
        )


def test_adapter_builds_digest_bound_complete_scope_manifest() -> None:
    payloads = {
        OFFICIAL_CATALOG_URL: _fixture("catalog.html"),
        _category_url("522012"): _fixture("ex03.html"),
    }
    adapter = OfficialSourceAdapter(payloads.__getitem__)
    snapshot = adapter.acquire_release_set(
        "EX3",
        first_acquired_at="2026-07-12T12:00:00Z",
    )

    assert snapshot.manifest.complete is True
    assert snapshot.manifest.expected_count == 2
    assert snapshot.manifest.source_locations == (_category_url("522012"),)
    assert snapshot.manifest.parser_version == OFFICIAL_PARSER_VERSION
    assert snapshot.manifest.first_acquired_at == "2026-07-12T12:00:00Z"
    assert len(snapshot.observations) == 3
    assert len(snapshot.manifest.observation_refs) == 3
    assert {ref.gameplay_card_id for ref in snapshot.manifest.observation_refs} == {
        "EX3-001",
        "EX3-010",
    }
    page_digest = raw_payload_digest(_fixture("ex03.html"))
    for observation in snapshot.observations:
        assert observation.raw_payload_digest == page_digest
        assert observation.observation_id == observation_digest(observation)


def test_identical_observation_reuses_identity_and_first_acquisition_time() -> None:
    payloads = {
        OFFICIAL_CATALOG_URL: _fixture("catalog.html"),
        _category_url("522012"): _fixture("ex03.html"),
    }
    adapter = OfficialSourceAdapter(payloads.__getitem__)
    first = adapter.acquire_release_set(
        "EX3",
        first_acquired_at="2026-07-12T12:00:00Z",
    )
    second = adapter.acquire_release_set(
        "EX3",
        first_acquired_at="2026-07-13T12:00:00Z",
        existing_observations={item.observation_id: item for item in first.observations},
    )

    assert second.manifest.observation_ids == first.manifest.observation_ids
    assert {item.first_acquired_at for item in second.observations} == {
        "2026-07-12T12:00:00Z"
    }
    assert second.manifest.first_acquired_at == "2026-07-12T12:00:00Z"


def test_parser_or_payload_change_creates_new_observation_identity() -> None:
    catalog = _fixture("catalog.html")
    discovery = discover_release_set(
        catalog,
        release_prefix="EX3",
        fetch_page=lambda _url: _fixture("ex03.html"),
    )
    first = build_scope_snapshot(
        discovery,
        catalog_payload=catalog,
        first_acquired_at="2026-07-12T12:00:00Z",
    )
    parser_changed = build_scope_snapshot(
        discovery,
        catalog_payload=catalog,
        first_acquired_at="2026-07-12T12:00:00Z",
        parser_version="official-html-v-next",
    )

    assert set(first.manifest.observation_ids).isdisjoint(
        parser_changed.manifest.observation_ids
    )

    changed_pages = tuple(
        replace(page, raw_payload=page.raw_payload + b"\n") for page in discovery.pages
    )
    payload_changed = build_scope_snapshot(
        replace(discovery, pages=changed_pages),
        catalog_payload=catalog,
        first_acquired_at="2026-07-12T12:00:00Z",
    )
    assert set(first.manifest.observation_ids).isdisjoint(
        payload_changed.manifest.observation_ids
    )


def test_japanese_only_god_grade_retains_structural_evolution_route() -> None:
    (printing,) = parse_card_list(
        _fixture("bt26_japanese_grade.html"),
        source_url=_category_url("522038"),
        category_id="522038",
    )
    assert printing.card_id == "BT26-086"
    assert printing.extracted["evolution_routes"] == []
    assert printing.extracted["grade_evolution_routes"] == [
        {"slot": 1, "colors": ["Multicolor"], "cost": 4, "from_grade": "God"}
    ]
