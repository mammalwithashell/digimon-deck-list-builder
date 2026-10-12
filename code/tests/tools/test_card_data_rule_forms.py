"""Printed Rule Form grants survive official parsing and runtime projection."""
from dataclasses import replace
import hashlib
import json
from pathlib import Path

import pytest

from tools.card_data.compatibility_diff import diff_runtime_fields
from tools.card_data.generate import project_canonical_card
from tools.card_data.models import SourceName, SourceObservation
from tools.card_data.normalization import normalize_card
from tools.card_data.serialization import observation_digest
from tools.card_data.sources.official import OFFICIAL_PARSER_VERSION, parse_card_list

FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/official"
META = json.loads((FIXTURES / "ex12_076_rule_form.json").read_text())
PRINTED_RULE = "(Rule) Trait: Has [Hybrid] Form."


def observation(rule=PRINTED_RULE):
    raw = (FIXTURES / META["fixture"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == META["fixture_sha256"]
    payload = raw.decode("utf-8").replace(PRINTED_RULE, rule)
    printing, = parse_card_list(payload, source_url=META["source_url"], category_id=META["category_id"])
    draft = SourceObservation(schema_version=1, observation_id="0" * 64,
        source=SourceName.OFFICIAL, source_record_key="522037:EX12-076:EX12-076",
        source_url=META["source_url"], raw_payload_digest=hashlib.sha256(payload.encode()).hexdigest(),
        parser_version=OFFICIAL_PARSER_VERSION, source_schema_version="world-card-list-html-v1",
        first_acquired_at="2026-10-12T00:00:00Z", extracted=printing.extracted)
    return replace(draft, observation_id=observation_digest(draft))


def test_printed_hybrid_form_roundtrip_and_exact_provenance():
    source = observation()
    assert source.extracted["rule_grants"]["forms"] == ["Hybrid"]
    card = normalize_card("EX12-076", [source])
    runtime = project_canonical_card(card)
    assert runtime["form_eng"] == ["Mega", "Hybrid"]
    assert "Hybrid" not in runtime["type_eng"]
    assert PRINTED_RULE in runtime["effect_description_eng"]
    assert runtime["play_cost"] == source.extracted["play_cost"] == 16
    assert runtime["dp"] == source.extracted["dp"] == 16000
    assert card.provenance["form_eng"].observation_ids == (source.observation_id,)
    old = dict(runtime, form_eng=["Mega"])
    diff, = diff_runtime_fields({card.card_id: old}, {card.card_id: runtime}, {card.card_id: card})
    assert diff.field_path == "form_eng"
    assert diff.provenance_observation_ids == (source.observation_id,)
    assert not diff.approved


@pytest.mark.parametrize("marker", ["(Rule)", "[Rule]", "Rule:"])
def test_rule_form_markers_and_multiple_grants(marker):
    source = observation(f"{marker} Trait: Has [Hybrid] Form.<br>{marker} Trait: Has [Mega] Form."
        f"<br>{marker} Trait: Has [Hybrid] Form.<br>{marker} Trait: Has [Dragonkin] Type."
        f"<br>{marker} Trait: Has [Free] attribute.")
    assert source.extracted["rule_grants"] == {
        "forms": ["Hybrid", "Mega"], "traits": ["Dragonkin"], "attributes": ["Free"]}
    runtime = project_canonical_card(normalize_card("EX12-076", [source]))
    assert runtime["form_eng"] == ["Mega", "Hybrid"]
    assert "Dragonkin" in runtime["type_eng"]
    assert "Free" in runtime["attribute_eng"]


@pytest.mark.parametrize("text", [
    "[On Play] This Digimon gains the [Hybrid] Form.",
    "Trait: Has [Hybrid] Form.",
    "[Rule] Trait: Has Hybrid Form.",
    "[Rule] Trait: Has [Hybrid] Formless.",
    "SuperRule Trait: Has [Hybrid] Form.",
    "[Rule) Trait: Has [Hybrid] Form.",
    "(Rule] Trait: Has [Hybrid] Form.",
    "If you have a Tamer, (Rule) Trait: Has [Hybrid] Form.",
    "[On Play] Treat this card as: (Rule) Trait: Has [Hybrid] Form.",
])
def test_non_rules_and_conditional_prose_do_not_grant_forms(text):
    source = observation(text)
    assert source.extracted["rule_grants"].get("forms", []) == []
    assert project_canonical_card(normalize_card("EX12-076", [source]))["form_eng"] == ["Mega"]


def test_historical_v4_grants_without_forms_remain_loadable_without_fabrication():
    source = observation()
    extracted = dict(source.extracted, rule_grants={"traits": [], "attributes": []})
    historic = replace(source, parser_version="official-html-v4", extracted=extracted)
    historic = replace(historic, observation_id=observation_digest(historic))
    assert project_canonical_card(normalize_card("EX12-076", [historic]))["form_eng"] == ["Mega"]
    assert historic.observation_id != source.observation_id


def test_new_parser_version_does_not_reuse_historical_observation_identity():
    source = observation()
    assert OFFICIAL_PARSER_VERSION == "official-html-v5"
    old_version = replace(source, parser_version="official-html-v4")
    assert observation_digest(old_version) != source.observation_id


def test_conflicting_official_form_grants_require_adjudication():
    hybrid = observation()
    other = observation(PRINTED_RULE.replace("Hybrid", "Armor"))
    card = normalize_card("EX12-076", [hybrid, other])
    conflicts = [item for item in card.conflicts if item.field_path == "form_eng"]
    assert len(conflicts) == 1
    assert conflicts[0].requires_adjudication(card.admission)
    assert set(conflicts[0].candidate_observation_ids) == {hybrid.observation_id, other.observation_id}
    assert not card.provenance["form_eng"].adjudicated


def test_adding_form_grant_changes_only_structured_form_field():
    source = observation()
    legacy = replace(source, extracted=dict(source.extracted,
        rule_grants={"traits": [], "attributes": []}))
    legacy = replace(legacy, observation_id=observation_digest(legacy))
    old = project_canonical_card(normalize_card("EX12-076", [legacy]))
    new = project_canonical_card(normalize_card("EX12-076", [source]))
    assert {key for key in set(old) | set(new) if old.get(key) != new.get(key)} == {"form_eng"}
