"""Real official Assembly conditions must survive existing-card projection."""
from dataclasses import replace
import hashlib
import json
from pathlib import Path

import pytest

from tools.card_data.compatibility_diff import diff_runtime_fields
from tools.card_data.errors import SourceSchemaError
from tools.card_data.generate import project_canonical_card
from tools.card_data.models import (SourceName, SourceObservation, ScopeManifest,
    ScopeObservationRef, MigrationAdjudication, AdjudicationKind, ReviewedCorrection, ModelValidationError)
from tools.card_data.normalization import normalize_card
from tools.card_data.official_candidate import build_official_candidate, _source_bootstrap
from tools.card_data.registry import StableCardRegistry
from tools.card_data.serialization import observation_digest, content_digest
from tools.card_data.sources.official import OFFICIAL_PARSER_VERSION, OfficialScopeSnapshot, parse_card_list

FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/official"
META = json.loads((FIXTURES / "assembly_provenance.json").read_text())
EXPECTED = {
    "EX12-046": "Assembly -2: Lv.4 or lower [TB] trait card",
    "EX12-063": "Assembly -2: Lv.4 or lower [Puppet]/[TB] trait card",
    "EX12-076": "Assembly -9: 8 [Hybrid]/[Shambala] trait cards w/different names",
}


def source(card_id):
    meta = next(row for row in META if row["card_id"] == card_id)
    raw = (FIXTURES / meta["fixture"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == meta["fixture_sha256"]
    printing, = parse_card_list(raw, source_url=meta["source_url"], category_id=meta["category_id"])
    draft = SourceObservation(schema_version=1, observation_id="0" * 64,
        source=SourceName.OFFICIAL, source_record_key=f"{meta['category_id']}:{card_id}:{card_id}",
        source_url=meta["source_url"], raw_payload_digest=hashlib.sha256(raw).hexdigest(),
        parser_version=OFFICIAL_PARSER_VERSION, source_schema_version="world-card-list-html-v1",
        first_acquired_at="2026-10-12T00:00:00Z", extracted=printing.extracted)
    return replace(draft, observation_id=observation_digest(draft))


def snapshot(observation):
    cid = observation.extracted["card_id"]
    manifest = ScopeManifest(schema_version=1, source=SourceName.OFFICIAL, scope="full-pool",
        source_locations=(observation.source_url,), first_acquired_at=observation.first_acquired_at,
        parser_version=observation.parser_version, source_schema_version=observation.source_schema_version,
        observation_ids=(observation.observation_id,),
        observation_refs=(ScopeObservationRef(observation.observation_id, cid, cid),),
        complete=True, expected_count=1)
    return OfficialScopeSnapshot(manifest, (observation,))


def candidate(cid, existing):
    observation = source(cid)
    card = normalize_card(cid, [observation])
    old_route = next(row["text"] for row in observation.extracted["text_sections"]
        if row["label"] == "Special Digivolution Condition")
    legacy = project_canonical_card(card, legacy_bootstrap={
        "xros_req": old_route, "card_name_jpn": "unchanged Japanese name", "rarity": 2})
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={cid: 1} if existing else {})
    result = build_official_candidate(snapshot(observation),
        legacy_cards={cid: legacy} if existing else {}, registry=registry)
    return result, json.loads(result.artifacts["data/cards.json"])[cid], observation, old_route


@pytest.mark.parametrize("cid", list(EXPECTED))
def test_existing_card_preserves_assembly_and_special_route_without_replacing_other_bootstrap(cid):
    result, row, observation, old_route = candidate(cid, True)
    assert EXPECTED[cid] in row["xros_req"]
    assert old_route in row["xros_req"]
    assert row["card_name_jpn"] == "unchanged Japanese name"
    assert row["rarity"] == 2
    diffs = [d for d in result.report["runtime_field_diffs"] if d["field_path"] == "xros_req"]
    assert len(diffs) == 1
    assert diffs[0]["provenance_observation_ids"] == [observation.observation_id]
    assert not diffs[0]["approved"]


@pytest.mark.parametrize("cid", list(EXPECTED))
def test_newly_admitted_card_already_preserves_both_condition_sections(cid):
    result, row, observation, old_route = candidate(cid, False)
    assert EXPECTED[cid] in row["xros_req"]
    assert old_route in row["xros_req"]


def test_xros_removal_is_visible_to_runtime_cutover_gate():
    diffs = diff_runtime_fields({"EX12-046": {"xros_req": EXPECTED["EX12-046"]}},
        {"EX12-046": {"xros_req": ""}}, {})
    assert len(diffs) == 1
    assert diffs[0].field_path == "xros_req"
    assert diffs[0].provenance_observation_ids == ()
    assert not diffs[0].approved


def test_xros_whole_runtime_field_can_be_exactly_reviewed_without_canonical_schema_expansion():
    review = MigrationAdjudication(schema_version=1, adjudication_id="fixture-xros",
        kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER, card_id="EX12-046", field_path="xros_req",
        old_input_revision="a" * 64, new_input_revision="b" * 64,
        old_digest=content_digest(""), new_digest=content_digest(EXPECTED["EX12-046"]),
        provenance_observation_ids=("c" * 64,), reason="Reviewed official condition",
        reviewer="fixture", reviewed_at="2026-10-12")
    assert review.field_path == "xros_req"


@pytest.mark.parametrize("path", ["xros_req", "xros_req.condition"])
def test_xros_is_not_a_canonical_correction_target(path):
    with pytest.raises(ModelValidationError, match="unsupported canonical field root"):
        ReviewedCorrection(schema_version=1, correction_id="not-canonical", card_id="EX12-046",
            field_path=path, value="anything", evidence_observation_ids=("a" * 64,),
            reason="Negative fixture", reviewer="fixture", reviewed_at="2026-10-12")


def test_disagreeing_official_conditions_cannot_be_selected_by_card_name_provenance():
    first = source("EX12-046")
    extracted = dict(first.extracted)
    extracted["text_sections"] = [dict(row, text=row["text"].replace("Assembly -2", "Assembly -3"))
        for row in extracted["text_sections"]]
    other = replace(first, extracted=extracted)
    other = replace(other, observation_id=observation_digest(other))
    card = normalize_card("EX12-046", [first, other])
    with pytest.raises(SourceSchemaError, match="condition"):
        _source_bootstrap(card, {first.observation_id: first, other.observation_id: other})


@pytest.mark.parametrize("kind,path", [("canonical_field_selection", "xros_req"),
    ("runtime_field_cutover", "xros_req.condition")])
def test_xros_schema_exception_does_not_admit_other_selection_paths(kind, path):
    with pytest.raises(ModelValidationError, match="unsupported canonical field root"):
        MigrationAdjudication(schema_version=1, adjudication_id="not-xros-whole",
            kind=AdjudicationKind(kind), card_id="EX12-046", field_path=path,
            old_input_revision="a" * 64, new_input_revision="b" * 64,
            old_digest="c" * 64, new_digest="d" * 64,
            reason="Negative fixture", reviewer="fixture", reviewed_at="2026-10-12")


def test_exact_xros_cutover_requires_current_revision_value_and_actual_evidence():
    from tools.card_data.compatibility_diff import approve_runtime_diffs
    from tools.card_data.compatibility_conditions import refresh_official_conditions
    obs = source("EX12-046")
    foreign = source("EX12-063")
    card = normalize_card("EX12-046", [obs])
    values, evidence = refresh_official_conditions({card.card_id: card}, {obs.observation_id: obs}, {})
    diff, = diff_runtime_fields({card.card_id: {"xros_req": ""}}, values, {card.card_id: card},
        compatibility_field_provenance=evidence)
    approval = MigrationAdjudication(schema_version=1, adjudication_id="xros-current",
        kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER, card_id=card.card_id, field_path="xros_req",
        old_input_revision="a" * 64, new_input_revision="b" * 64,
        old_digest=diff.old_digest, new_digest=diff.new_digest,
        provenance_observation_ids=(obs.observation_id,), reason="Exact fixture review",
        reviewer="fixture", reviewed_at="2026-10-12")
    for mutation in ({}, {"new_input_revision": "c" * 64}, {"new_digest": "d" * 64},
                     {"provenance_observation_ids": (foreign.observation_id,)}):
        checked, = approve_runtime_diffs([diff], corrections=[],
            adjudications=[replace(approval, **mutation)],
            observations={obs.observation_id: obs, foreign.observation_id: foreign},
            old_input_revision="a" * 64, new_input_revision="b" * 64)
        assert checked.approved is (not mutation)


@pytest.mark.parametrize("failure", ["missing", "foreign", "malformed", "secondary_only", "bad_identity"])
def test_condition_projection_refuses_invalid_or_nonofficial_evidence(failure):
    from tools.card_data.compatibility_conditions import official_condition_projection
    obs = source("EX12-046")
    card = normalize_card("EX12-046", [obs])
    observations = {obs.observation_id: obs}
    if failure == "missing":
        observations = {}
    elif failure == "foreign":
        changed = replace(obs, extracted=dict(obs.extracted, card_id="EX12-063"))
    elif failure == "malformed":
        changed = replace(obs, extracted=dict(obs.extracted, text_sections=None))
    elif failure == "secondary_only":
        changed = replace(obs, source=SourceName.DIGIMONCARD_IO)
    else:
        changed = replace(obs, raw_payload_digest="a" * 64)
    if failure != "missing":
        if failure != "bad_identity":
            changed = replace(changed, observation_id=observation_digest(changed))
            card = replace(card, printings=(replace(card.printings[0],
                observation_ids=(changed.observation_id,)),))
        observations = {changed.observation_id: changed}
    with pytest.raises(SourceSchemaError):
        official_condition_projection(card, observations)


def test_preflight_reports_all_condition_disagreements_in_one_pass():
    from tools.card_data.compatibility_conditions import refresh_official_conditions
    cards, observations = {}, {}
    for cid in ("EX12-046", "EX12-063"):
        obs = source(cid)
        changed = replace(obs, extracted=dict(obs.extracted, text_sections=[
            dict(row, text=row["text"].replace("Assembly -2", "Assembly -3"))
            for row in obs.extracted["text_sections"]]))
        changed = replace(changed, observation_id=observation_digest(changed))
        observations.update({obs.observation_id: obs, changed.observation_id: changed})
        cards[cid] = normalize_card(cid, [obs, changed])
    with pytest.raises(SourceSchemaError) as caught:
        refresh_official_conditions(cards, observations, {})
    conflicts = caught.value.details["condition_conflicts"]
    assert {row["card_id"] for row in conflicts} == set(cards)
    for conflict in conflicts:
        groups = conflict["details"]["candidate_conditions"]
        assert len(groups) == 2
        assert all(oid in observations for oid in groups)
        assert any("Assembly -2" in value for value in groups.values())
        assert any("Assembly -3" in value for value in groups.values())


def test_refresh_preserves_unrelated_bootstrap_fields_and_does_not_merge_secondary_conditions():
    from tools.card_data.compatibility_conditions import refresh_official_conditions
    obs = source("EX12-046")
    secondary = replace(obs, source=SourceName.DIGIMONCARD_IO, extracted=dict(obs.extracted,
        text_sections=[{"label": "Special Play Condition", "text": "Invented secondary condition"}]))
    secondary = replace(secondary, observation_id=observation_digest(secondary))
    card = normalize_card("EX12-046", [obs])
    card = replace(card, printings=(replace(card.printings[0],
        observation_ids=(obs.observation_id, secondary.observation_id)),))
    old = {card.card_id: {"card_name_jpn": "unchanged", "rarity": 2,
        "dna_costs": [{"memory_cost": 0}], "digixros_costs": [{"different_levels": True}]}}
    values, evidence = refresh_official_conditions({card.card_id: card},
        {obs.observation_id: obs, secondary.observation_id: secondary}, old)
    assert all(values[card.card_id][key] == value for key, value in old[card.card_id].items())
    assert "Invented" not in values[card.card_id]["xros_req"]
    assert evidence[card.card_id]["xros_req"] == (obs.observation_id,)
    assert "xros_req" not in old[card.card_id]


@pytest.mark.parametrize("cid", list(EXPECTED))
def test_projected_assembly_is_preserved_in_compatibility_clause_denominator(cid):
    from tools.clause_coverage.card_sources import extract_card_clauses
    result, row, observation, _ = candidate(cid, True)
    clauses = extract_card_clauses(cid, cards_index={cid: row}, overrides_index={},
        official_index={}, dcgo_root=None)
    assert any(EXPECTED[cid] in clause.text for clause in clauses)


def test_full_candidate_builder_refreshes_existing_condition_and_preserves_review_gate(tmp_path):
    from tools.card_data.complete_candidate import build_complete_candidate, rebuild_source_plan_from_snapshots
    from tools.card_data.serialization import canonical_json_bytes
    cid = "EX12-046"
    obs = source(cid)
    official = snapshot(obs)
    official = replace(official, manifest=replace(official.manifest, scope="full-pool:official-catalog"))
    card = normalize_card(cid, [obs])
    old = project_canonical_card(card, legacy_bootstrap={"xros_req": "old route"})
    secondary_obs = replace(obs, source=SourceName.DIGIMONCARD_IO,
        parser_version="secondary-fixture-v1", extracted=dict(old, card_id=cid))
    secondary_obs = replace(secondary_obs, observation_id=observation_digest(secondary_obs))
    secondary = OfficialScopeSnapshot(replace(official.manifest, source=SourceName.DIGIMONCARD_IO,
        parser_version=secondary_obs.parser_version, scope="full-pool:digimoncard-io-catalog",
        observation_ids=(secondary_obs.observation_id,),
        observation_refs=(ScopeObservationRef(secondary_obs.observation_id, cid, cid),)), (secondary_obs,))
    (tmp_path / "data").mkdir()
    (tmp_path / "data/cards.json").write_bytes(canonical_json_bytes({cid: old}))
    plan = rebuild_source_plan_from_snapshots(tmp_path, official, secondary)
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={cid: 1})
    result = build_complete_candidate(repo_root=tmp_path, official=official, secondary=secondary,
        source_plan=plan, legacy_cards={cid: old}, registry=registry)
    row = json.loads(result.artifacts["data/cards.json"])[cid]
    assert EXPECTED[cid] in row["xros_req"]
    diff, = [d for d in result.report["runtime_field_diffs"] if d["field_path"] == "xros_req"]
    assert diff["provenance_observation_ids"] == [obs.observation_id]
    assert not diff["approved"]
    assert result.report["status"] == "blocked"


def test_already_correct_conditions_do_not_create_an_extra_runtime_change():
    first, row, obs, _ = candidate("EX12-046", True)
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={"EX12-046": 1})
    second = build_official_candidate(snapshot(obs), legacy_cards={"EX12-046": row}, registry=registry)
    assert not any(diff["field_path"] == "xros_req" for diff in second.report["runtime_field_diffs"])


def test_unrelated_same_card_observation_does_not_enter_condition_evidence():
    from tools.card_data.compatibility_conditions import official_condition_projection
    obs = source("EX12-046")
    unrelated = replace(obs, extracted=dict(obs.extracted, text_sections=[
        {"label": "Special Play Condition", "text": "Unrelated evidence must not be chosen"}]))
    unrelated = replace(unrelated, observation_id=observation_digest(unrelated))
    card = normalize_card("EX12-046", [obs])
    value, evidence = official_condition_projection(card,
        {obs.observation_id: obs, unrelated.observation_id: unrelated})
    assert evidence == (obs.observation_id,)
    assert "Unrelated" not in value


def test_dual_faces_are_not_rewritten_by_condition_projection():
    from tools.card_data.compatibility_conditions import refresh_official_conditions
    payload = (FIXTURES / "bt25_dual.html").read_bytes()
    printing, = parse_card_list(payload, source_url="https://example.test/dual", category_id="522036")
    draft = source("EX12-046")
    obs = replace(draft, extracted=printing.extracted, raw_payload_digest=hashlib.sha256(payload).hexdigest())
    obs = replace(obs, observation_id=observation_digest(obs))
    card = normalize_card("BT25-043", [obs])
    before = project_canonical_card(card)
    values, evidence = refresh_official_conditions({card.card_id: card}, {obs.observation_id: obs}, {})
    after = project_canonical_card(card, legacy_bootstrap=values[card.card_id])
    assert before["dual"] == after["dual"]
    assert before["effect_description_eng"] == after["effect_description_eng"]
    assert evidence[card.card_id]["xros_req"] == (obs.observation_id,)
