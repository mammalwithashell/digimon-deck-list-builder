"""Exact reviewed selection of official compatibility-only condition sources."""
from dataclasses import replace
import json

import pytest

from test_card_data_assembly_projection import source, snapshot
from tools.card_data.errors import AdjudicationError, SourceSchemaError, ValidationError
from tools.card_data.normalization import normalize_card
from tools.card_data.serialization import content_digest, observation_digest, canonicalize
from tools.card_data.compatibility_conditions import official_condition_projection


def conflict():
    first = source("EX12-046")
    other = replace(first, extracted=dict(first.extracted, text_sections=[
        dict(row, text=row["text"].replace("Assembly -2", "Assembly -3"))
        for row in first.extracted["text_sections"]]))
    other = replace(other, observation_id=observation_digest(other))
    observations = {first.observation_id: first, other.observation_id: other}
    card = normalize_card("EX12-046", list(observations.values()))
    with pytest.raises(SourceSchemaError) as caught:
        official_condition_projection(card, observations)
    values = caught.value.details["candidate_conditions"]
    payload = dict(schema_version=1, selection_id="fixture-condition-selection",
        selection_kind="official_condition_projection", card_id=card.card_id, field_path="xros_req",
        source_manifest_revision="a" * 64, candidate_values=values,
        candidate_set_digest=content_digest(values), selected_observation_id=first.observation_id,
        selected_value=values[first.observation_id], selected_value_digest=content_digest(values[first.observation_id]),
        reviewer="independent fixture review", reason="Fixture primary image supports this condition", reviewed_at="2026-10-12")
    return card, observations, payload


def selection(payload):
    from tools.card_data.compatibility_selections import CompatibilitySourceSelection
    return CompatibilitySourceSelection(**payload)


def test_exact_selection_resolves_only_bound_official_condition():
    card, observations, payload = conflict()
    value, evidence = official_condition_projection(card, observations,
        selections=(selection(payload),), source_manifest_revision="a" * 64)
    assert value == payload["selected_value"]
    assert evidence == (payload["selected_observation_id"],)


@pytest.mark.parametrize("change", ["revision", "candidate_added", "candidate_removed", "candidate_value"])
def test_stale_selection_cannot_resolve_changed_source_boundary(change):
    card, observations, payload = conflict()
    if change == "revision":
        payload["source_manifest_revision"] = "b" * 64
    else:
        values = dict(payload["candidate_values"])
        unselected = next(oid for oid in values if oid != payload["selected_observation_id"])
        if change == "candidate_added": values["b" * 64] = "Extra observation"
        elif change == "candidate_removed": del values[unselected]
        else: values[unselected] += " altered"
        payload.update(candidate_values=values, candidate_set_digest=content_digest(values))
    with pytest.raises(SourceSchemaError):
        official_condition_projection(card, observations,
            selections=(selection(payload),), source_manifest_revision="a" * 64)


@pytest.mark.parametrize("mutation", [
    {"schema_version": 2}, {"field_path": "effect_description_eng"},
    {"selected_observation_id": "b" * 64}, {"selected_value": "Unobserved text"},
    {"selected_value_digest": "b" * 64}, {"candidate_set_digest": "b" * 64},
    {"reviewer": ""}, {"reason": ""}, {"reviewed_at": ""}, {"reviewed_at": "not a date"},
])
def test_invalid_selection_rows_fail_closed(mutation):
    _, _, payload = conflict()
    with pytest.raises(AdjudicationError): selection(dict(payload, **mutation))


def test_duplicate_current_selection_is_not_silently_chosen():
    card, observations, payload = conflict()
    second = dict(payload, selection_id="second-review")
    with pytest.raises(SourceSchemaError):
        official_condition_projection(card, observations,
            selections=(selection(payload), selection(second)), source_manifest_revision="a" * 64)


def test_versioned_ledger_roundtrip_and_duplicate_identity_rejection(tmp_path):
    from tools.card_data.compatibility_selections import load_compatibility_selections, selection_ledger_payload
    _, _, payload = conflict()
    row = selection(payload)
    path = tmp_path / "selections.json"
    path.write_text(json.dumps(canonicalize(selection_ledger_payload((row,)))))
    assert load_compatibility_selections(path) == (row,)
    path.write_text(json.dumps(dict(schema_version=1, selections=[payload, payload])))
    with pytest.raises(AdjudicationError): load_compatibility_selections(path)


def test_official_candidate_seals_selection_and_binds_runtime_revision(tmp_path):
    from tools.card_data.official_candidate import build_official_candidate, write_candidate
    from tools.card_data.registry import StableCardRegistry
    from tools.card_data.models import ScopeObservationRef
    from tools.card_data.generate import project_canonical_card
    card, observations, payload = conflict()
    sources = tuple(observations.values())
    snap = snapshot(sources[0])
    snap = replace(snap, observations=sources, manifest=replace(snap.manifest,
        observation_ids=tuple(observations), observation_refs=tuple(
            ScopeObservationRef(oid, card.card_id, card.card_id) for oid in observations)))
    payload["source_manifest_revision"] = content_digest(snap.manifest, exclude_operational=True)
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={card.card_id: 1})
    old = {card.card_id: project_canonical_card(card, legacy_bootstrap={"xros_req": "old condition"})}
    first = build_official_candidate(snap, legacy_cards=old, registry=registry,
        compatibility_selections=(selection(payload),))
    ledger_path = "data/card_compatibility_selections.json"
    assert json.loads(first.artifacts[ledger_path])["selections"][0]["selection_id"] == payload["selection_id"]
    generated = json.loads(first.artifacts["data/cards.json"])[card.card_id]
    assert generated["xros_req"] == payload["selected_value"]
    diff, = [d for d in first.report["runtime_field_diffs"] if d["field_path"] == "xros_req"]
    assert diff["provenance_observation_ids"] == [payload["selected_observation_id"]]
    assert not diff["approved"]  # source selection never grants runtime cutover approval
    other_id = next(oid for oid in observations if oid != payload["selected_observation_id"])
    second_payload = dict(payload, selection_id="other-reviewed-selection", selected_observation_id=other_id,
        selected_value=payload["candidate_values"][other_id],
        selected_value_digest=content_digest(payload["candidate_values"][other_id]))
    second = build_official_candidate(snap, legacy_cards=old, registry=registry,
        compatibility_selections=(selection(second_payload),))
    assert first.report["candidate_canonical_revision"] == second.report["candidate_canonical_revision"]
    assert first.report["new_input_revision"] != second.report["new_input_revision"]
    write_candidate(first, tmp_path / "candidate")
    assert (tmp_path / "candidate" / ledger_path).is_file()
    inventory = json.loads((tmp_path / "candidate/.card-data-candidate.json").read_text())
    assert ledger_path in inventory["artifact_digests"]


def test_committed_ledger_reader_uses_marker_bytes_and_preserves_old_absent_stack():
    from tools.card_data.complete_candidate import _compatibility_selections_from_committed_artifacts
    from tools.card_data.serialization import canonical_json_bytes
    from tools.card_data.compatibility_selections import selection_ledger_payload, SELECTION_LEDGER_PATH
    _, _, payload = conflict()
    row = selection(payload)
    artifacts = {SELECTION_LEDGER_PATH: canonical_json_bytes(selection_ledger_payload((row,)))}
    assert _compatibility_selections_from_committed_artifacts(artifacts) == (row,)
    assert _compatibility_selections_from_committed_artifacts({}) == ()
    with pytest.raises(AdjudicationError):
        _compatibility_selections_from_committed_artifacts({SELECTION_LEDGER_PATH: b'{}'})


def test_selection_ledger_extension_preserves_history():
    from tools.card_data.complete_candidate import _require_staged_ledger_extension
    _, _, payload = conflict()
    old = selection(payload)
    new = selection(dict(payload, selection_id="new-review"))
    assert _require_staged_ledger_extension((old,), (old, new), identity_field="selection_id",
        description="compatibility source selection ledger") == (old, new)
    with pytest.raises(ValidationError, match="append-only"):
        _require_staged_ledger_extension((old,), (new,), identity_field="selection_id",
            description="compatibility source selection ledger")


def test_full_candidate_binds_selection_ledger_and_selected_evidence(tmp_path):
    from tools.card_data.complete_candidate import build_complete_candidate, rebuild_source_plan_from_snapshots, write_complete_candidate
    from tools.card_data.models import ScopeObservationRef, SourceName
    from tools.card_data.sources.official import OfficialScopeSnapshot
    from tools.card_data.generate import project_canonical_card
    from tools.card_data.registry import StableCardRegistry
    from tools.card_data.serialization import canonical_json_bytes
    from tools.card_data.source_manifest import build_full_pool_source_manifest
    card, observations, payload = conflict()
    sources = tuple(observations.values())
    official = snapshot(sources[0])
    official = replace(official, observations=sources, manifest=replace(official.manifest,
        scope="full-pool:official-catalog", observation_ids=tuple(observations),
        observation_refs=tuple(ScopeObservationRef(oid, card.card_id, card.card_id) for oid in observations)))
    old = project_canonical_card(card, legacy_bootstrap={"xros_req": "old condition"})
    secondary_obs = replace(sources[0], source=SourceName.DIGIMONCARD_IO,
        parser_version="secondary-fixture-v1", extracted=dict(old, card_id=card.card_id))
    secondary_obs = replace(secondary_obs, observation_id=observation_digest(secondary_obs))
    secondary = OfficialScopeSnapshot(replace(official.manifest, source=SourceName.DIGIMONCARD_IO,
        parser_version=secondary_obs.parser_version, scope="full-pool:digimoncard-io-catalog",
        observation_ids=(secondary_obs.observation_id,),
        observation_refs=(ScopeObservationRef(secondary_obs.observation_id, card.card_id, card.card_id),)), (secondary_obs,))
    aggregate = build_full_pool_source_manifest((official.manifest, secondary.manifest))
    payload["source_manifest_revision"] = aggregate.manifest_revision
    (tmp_path / "data").mkdir()
    (tmp_path / "data/cards.json").write_bytes(canonical_json_bytes({card.card_id: old}))
    plan = rebuild_source_plan_from_snapshots(tmp_path, official, secondary)
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={card.card_id: 1})
    result = build_complete_candidate(repo_root=tmp_path, official=official, secondary=secondary,
        source_plan=plan, legacy_cards={card.card_id: old}, registry=registry,
        compatibility_selections=(selection(payload),))
    ledger_path = "data/card_compatibility_selections.json"
    assert json.loads(result.artifacts[ledger_path])["selections"][0]["selection_id"] == payload["selection_id"]
    assert result.report["compatibility_selection_ledger_revision"] == content_digest(json.loads(result.artifacts[ledger_path]))
    row = json.loads(result.artifacts["data/cards.json"])[card.card_id]
    assert row["xros_req"] == payload["selected_value"]
    diff, = [d for d in result.report["runtime_field_diffs"] if d["field_path"] == "xros_req"]
    assert diff["provenance_observation_ids"] == [payload["selected_observation_id"]]
    assert not diff["approved"]
    assert result.report["status"] == "blocked"
    write_complete_candidate(result, tmp_path / "candidate")
    inventory = json.loads((tmp_path / "candidate/.card-data-candidate.json").read_text())
    assert ledger_path in inventory["artifact_digests"]


@pytest.mark.parametrize("raw", [
    b'{"schema_version":1,"schema_version":1,"selections":[]}',
    b'{"schema_version":1,"selections":[],"unknown":true}',
    b'{"schema_version":true,"selections":[]}',
])
def test_ambiguous_or_unknown_ledger_schema_is_rejected(raw):
    from tools.card_data.compatibility_selections import selections_from_bytes
    with pytest.raises(AdjudicationError): selections_from_bytes(raw)
