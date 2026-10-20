import json
from pathlib import Path

import pytest

from tools.card_data.corrections import write_reviewed_corrections
from tools.card_data.errors import ValidationError


def test_legacy_writer_must_not_erase_v2_compatibility_history(tmp_path):
    path = tmp_path / 'card_corrections.json'
    raw = json.dumps({'schema_version': 2, 'corrections': [],
                      'compatibility_corrections': [{'sentinel': 'must not erase'}]}).encode()
    path.write_bytes(raw)
    with pytest.raises(ValidationError):
        write_reviewed_corrections(path, ())
    assert path.read_bytes() == raw


def test_image_correction_contract_is_separate_from_canonical_fields():
    from tools.card_data.compatibility_corrections import CorrectionDocument
    assert CorrectionDocument().compatibility_corrections == ()


FIXTURE = Path(__file__).parent / 'fixtures' / 'condition_image_correction'


def bound_case():
    import hashlib
    from tools.card_data.compatibility_corrections import CompatibilityCorrection
    from tools.card_data.live_reconcile import _observation_from_dict
    from tools.card_data.normalization import normalize_card
    from tools.card_data.serialization import content_digest
    raw = json.loads((FIXTURE / 'ex3-010-observations.json').read_text(encoding='utf-8'))
    observations = {r['observation_id']: _observation_from_dict(r) for r in raw}
    values = {oid: '\n'.join(f"[{r['label']}] {r['text']}" for r in o.extracted['text_sections']
                            if r['label'] == 'Special Digivolution Condition')
              for oid, o in observations.items()}
    oid = 'e74ff1f1cdca2a41815bb652167779f7b7201740b44ae6069c0a4790078e05eb'
    image = (FIXTURE / 'EX3-010.webp').read_bytes()
    digest = hashlib.sha256(image).hexdigest()
    value = values[oid] + '\nDigivolve unsuspended with the 2 specified Digimon stacked on top of each other.'
    correction = CompatibilityCorrection(
        schema_version=1, correction_id='ex3-010-dna-reminder',
        correction_kind='official_condition_image_correction', card_id='EX3-010', field_path='xros_req',
        source_manifest_revision='a' * 64, candidate_values=values,
        candidate_set_digest=content_digest(values), base_observation_id=oid,
        base_value=values[oid], base_value_digest=content_digest(values[oid]),
        value=value, value_digest=content_digest(value), evidence_observation_ids=tuple(sorted(values)),
        image_artifact_path=f'data/card_correction_evidence/{digest}.webp', image_sha256=digest,
        reason='Inspected image reminder omitted by all four official observations',
        reviewer='independent test review', reviewed_at='2026-10-12T00:00:00Z')
    return normalize_card('EX3-010', observations.values()), observations, correction, {correction.image_artifact_path: image}


def test_frozen_printed_reminder_projects_only_with_bound_image_correction():
    from tools.card_data.compatibility_conditions import official_condition_projection
    from tools.card_data.compatibility_corrections import apply_image_correction
    from tools.card_data.errors import SourceSchemaError
    card, obs, correction, images = bound_case()
    with pytest.raises(SourceSchemaError):
        official_condition_projection(card, obs)
    value, evidence = apply_image_correction(card, obs, correction,
        source_manifest_revision='a' * 64, image_artifacts=images)
    assert value == correction.value
    assert evidence == tuple(sorted(obs))
    assert all('stacked on top' not in v for v in correction.candidate_values.values())


@pytest.mark.parametrize('mutation', ['revision', 'missing-image', 'tampered-image', 'missing-observation', 'foreign-card'])
def test_projection_rejects_unbound_evidence(mutation):
    from dataclasses import replace
    from tools.card_data.compatibility_corrections import apply_image_correction
    card, obs, correction, images = bound_case()
    revision = 'a' * 64
    if mutation == 'revision': revision = 'b' * 64
    if mutation == 'missing-image': images = {}
    if mutation == 'tampered-image': images = {correction.image_artifact_path: b'changed'}
    if mutation == 'missing-observation': obs.pop(next(iter(obs)))
    if mutation == 'foreign-card': card = replace(card, card_id='EX3-011')
    with pytest.raises(ValidationError):
        apply_image_correction(card, obs, correction, source_manifest_revision=revision, image_artifacts=images)


@pytest.mark.parametrize('changes', [
    {'field_path': 'effect_description_eng'}, {'field_path': 'xros_req.0'},
    {'image_artifact_path': '../elsewhere.webp'}, {'candidate_set_digest': '0' * 64},
    {'evidence_observation_ids': ()}, {'value_digest': '0' * 64}, {'reviewer': ''},
])
def test_correction_schema_rejects_invalid_targets_and_bindings(changes):
    from dataclasses import replace
    _, _, correction, _ = bound_case()
    with pytest.raises(ValidationError):
        replace(correction, **changes)


def test_v2_whole_document_roundtrip_and_legacy_interfaces_fail_closed(tmp_path):
    from tools.card_data.compatibility_corrections import CorrectionDocument, write_correction_document, load_correction_document
    from tools.card_data.corrections import load_reviewed_corrections
    _, _, correction, _ = bound_case()
    document = CorrectionDocument(compatibility_corrections=(correction,))
    path = tmp_path / 'card_corrections.json'
    write_correction_document(path, document)
    before = path.read_bytes()
    assert load_correction_document(path) == document
    with pytest.raises(ValidationError): load_reviewed_corrections(path)
    with pytest.raises(ValidationError): write_reviewed_corrections(path, ())
    with pytest.raises(ValidationError): write_correction_document(path, CorrectionDocument())
    assert path.read_bytes() == before


def test_v1_document_is_compatible_without_creating_image_corrections(tmp_path):
    from tools.card_data.compatibility_corrections import load_correction_document
    path = tmp_path / 'card_corrections.json'
    write_reviewed_corrections(path, ())
    assert load_correction_document(path).compatibility_corrections == ()


def test_correction_document_rejects_unknown_and_duplicate_fields():
    from tools.card_data.compatibility_corrections import correction_document_from_bytes
    for raw in [b'{"schema_version":1,"corrections":[],"unknown":[]}',
                b'{"schema_version":1,"schema_version":1,"corrections":[]}']:
        with pytest.raises(ValidationError): correction_document_from_bytes(raw)


def test_official_candidate_seals_image_and_keeps_runtime_gate(tmp_path):
    from dataclasses import replace
    from test_card_data_assembly_projection import snapshot
    from tools.card_data.compatibility_corrections import CorrectionDocument
    from tools.card_data.generate import project_canonical_card
    from tools.card_data.models import ScopeObservationRef
    from tools.card_data.official_candidate import build_official_candidate, write_candidate
    from tools.card_data.publication import load_candidate_artifacts
    from tools.card_data.registry import StableCardRegistry
    from tools.card_data.serialization import content_digest
    card, obs, correction, images = bound_case()
    snap = snapshot(next(iter(obs.values())))
    snap = replace(snap, observations=tuple(obs.values()), manifest=replace(snap.manifest,
        observation_ids=tuple(obs), observation_refs=tuple(ScopeObservationRef(oid, card.card_id,
        o.extracted['printing']['printing_id']) for oid, o in obs.items())))
    correction = replace(correction, source_manifest_revision=content_digest(snap.manifest, exclude_operational=True))
    old = {card.card_id: project_canonical_card(card, legacy_bootstrap={'xros_req': correction.base_value})}
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={card.card_id: 1})
    result = build_official_candidate(snap, legacy_cards=old, registry=registry,
        correction_document=CorrectionDocument(compatibility_corrections=(correction,)),
        correction_image_artifacts=images)
    assert json.loads(result.artifacts['data/card_corrections.json'])['schema_version'] == 2
    assert result.artifacts[correction.image_artifact_path] == images[correction.image_artifact_path]
    assert json.loads(result.artifacts['data/cards.json'])[card.card_id]['xros_req'] == correction.value
    diff, = [r for r in result.report['runtime_field_diffs'] if r['field_path'] == 'xros_req']
    assert not diff['approved']
    assert sorted(diff['provenance_observation_ids']) == sorted(obs)
    assert result.report['compatibility_corrections'][0]['correction_id'] == correction.correction_id
    write_candidate(result, tmp_path / 'candidate')
    loaded = load_candidate_artifacts(tmp_path / 'candidate')
    assert loaded[correction.image_artifact_path] == images[correction.image_artifact_path]


def test_candidate_loader_rejects_unowned_image_even_with_resealed_inventory(tmp_path):
    from tools.card_data.compatibility_corrections import CorrectionDocument, correction_document_payload
    from tools.card_data.publication import candidate_inventory_payload, load_candidate_artifacts
    from tools.card_data.serialization import canonical_json_bytes
    _, _, correction, _ = bound_case()
    artifacts = {'data/card_corrections.json': canonical_json_bytes(
        correction_document_payload(CorrectionDocument(compatibility_corrections=(correction,))))}
    for relative, raw in artifacts.items():
        target = tmp_path / relative; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(raw)
    (tmp_path / '.card-data-candidate.json').write_bytes(canonical_json_bytes(candidate_inventory_payload(artifacts)))
    with pytest.raises(ValidationError): load_candidate_artifacts(tmp_path)


def test_selection_and_image_correction_overlap_rejected():
    from tools.card_data.compatibility_conditions import refresh_official_conditions
    from tools.card_data.compatibility_selections import CompatibilitySourceSelection
    card, obs, correction, images = bound_case()
    selection = CompatibilitySourceSelection(schema_version=1, selection_id='overlap',
        selection_kind='official_condition_projection', card_id=card.card_id, field_path='xros_req',
        source_manifest_revision=correction.source_manifest_revision, candidate_values=correction.candidate_values,
        candidate_set_digest=correction.candidate_set_digest, selected_observation_id=correction.base_observation_id,
        selected_value=correction.base_value, selected_value_digest=correction.base_value_digest,
        reviewer='reviewer', reason='reason', reviewed_at='2026-10-12')
    with pytest.raises(ValidationError):
        refresh_official_conditions({card.card_id: card}, obs, {}, selections=(selection,),
            source_manifest_revision=correction.source_manifest_revision,
            compatibility_corrections=(correction,), correction_image_artifacts=images)


def full_case(tmp_path):
    from dataclasses import replace
    from test_card_data_assembly_projection import snapshot
    from tools.card_data.complete_candidate import rebuild_source_plan_from_snapshots
    from tools.card_data.generate import project_canonical_card
    from tools.card_data.models import ScopeObservationRef, SourceName
    from tools.card_data.sources.official import OfficialScopeSnapshot
    from tools.card_data.registry import StableCardRegistry
    from tools.card_data.serialization import canonical_json_bytes, observation_digest
    from tools.card_data.source_manifest import build_full_pool_source_manifest
    card, obs, correction, images = bound_case()
    sources = tuple(obs.values())
    official = snapshot(sources[0])
    official = replace(official, observations=sources, manifest=replace(official.manifest,
        scope='full-pool:official-catalog', observation_ids=tuple(obs),
        observation_refs=tuple(ScopeObservationRef(oid, card.card_id, o.extracted['printing']['printing_id']) for oid, o in obs.items())))
    old = project_canonical_card(card, legacy_bootstrap={'xros_req': correction.base_value})
    secondary_obs = replace(sources[0], source=SourceName.DIGIMONCARD_IO,
        parser_version='secondary-fixture-v1', extracted=dict(old, card_id=card.card_id))
    secondary_obs = replace(secondary_obs, observation_id=observation_digest(secondary_obs))
    secondary = OfficialScopeSnapshot(replace(official.manifest, source=SourceName.DIGIMONCARD_IO,
        parser_version=secondary_obs.parser_version, scope='full-pool:digimoncard-io-catalog',
        observation_ids=(secondary_obs.observation_id,),
        observation_refs=(ScopeObservationRef(secondary_obs.observation_id, card.card_id, card.card_id),)), (secondary_obs,))
    revision = build_full_pool_source_manifest((official.manifest, secondary.manifest)).manifest_revision
    correction = replace(correction, source_manifest_revision=revision)
    (tmp_path / 'data').mkdir(exist_ok=True)
    (tmp_path / 'data/cards.json').write_bytes(canonical_json_bytes({card.card_id: old}))
    plan = rebuild_source_plan_from_snapshots(tmp_path, official, secondary)
    registry = StableCardRegistry(schema_version=1, registry_version=1, capacity=20000,
        padding_index=0, assignments={card.card_id: 1})
    return dict(repo_root=tmp_path, official=official, secondary=secondary, source_plan=plan,
        legacy_cards={card.card_id: old}, registry=registry), correction, images


def test_full_candidate_and_committed_reader_preserve_whole_correction(tmp_path, monkeypatch):
    from types import SimpleNamespace
    from tools.card_data.complete_candidate import build_complete_candidate
    from tools.card_data.compatibility_corrections import CorrectionDocument
    from tools.card_data.generate import load_committed_generation_stack
    kwargs, correction, images = full_case(tmp_path)
    result = build_complete_candidate(**kwargs,
        correction_document=CorrectionDocument(compatibility_corrections=(correction,)),
        correction_image_artifacts=images)
    assert result.artifacts[correction.image_artifact_path] == images[correction.image_artifact_path]
    assert json.loads(result.artifacts['data/card_corrections.json'])['schema_version'] == 2
    assert result.report['compatibility_corrections'][0]['status'] == 'applied'
    diff, = [d for d in result.report['runtime_field_diffs'] if d['field_path'] == 'xros_req']
    assert not diff['approved']
    monkeypatch.setattr('tools.card_data.generate.read_consistent_revision',
        lambda *a, **k: SimpleNamespace(values=result.artifacts, revision='fixture'))
    stack = load_committed_generation_stack(tmp_path, marker_path=tmp_path/'marker', journal_path=tmp_path/'journal')
    assert stack.generated_cards_bytes == stack.committed_cards_bytes
    assert json.loads(stack.generated_cards_bytes)['EX3-010']['xros_req'] == correction.value
    result.artifacts[correction.image_artifact_path] = b'tampered'
    with pytest.raises(ValidationError):
        load_committed_generation_stack(tmp_path, marker_path=tmp_path/'marker', journal_path=tmp_path/'journal')


@pytest.mark.parametrize('mutation', ['lost-projection', 'changed-source', 'unowned-image'])
def test_committed_reader_rejects_correction_binding_drift(tmp_path, monkeypatch, mutation):
    from types import SimpleNamespace
    from tools.card_data.complete_candidate import build_complete_candidate
    from tools.card_data.compatibility_corrections import CorrectionDocument
    from tools.card_data.generate import load_committed_generation_stack
    from tools.card_data.serialization import canonical_json_bytes
    kwargs, correction, images = full_case(tmp_path)
    result = build_complete_candidate(**kwargs,
        correction_document=CorrectionDocument(compatibility_corrections=(correction,)),
        correction_image_artifacts=images)
    artifacts = dict(result.artifacts)
    if mutation == 'lost-projection':
        path = 'data/card_compatibility_projection.json'
        payload = json.loads(artifacts[path])
        payload['compatibility_fields']['EX3-010']['xros_req'] = correction.base_value
        artifacts[path] = canonical_json_bytes(payload)
    elif mutation == 'changed-source':
        path = 'data/card_corrections.json'
        payload = json.loads(artifacts[path])
        payload['compatibility_corrections'][0]['source_manifest_revision'] = 'b' * 64
        artifacts[path] = canonical_json_bytes(payload)
    else:
        artifacts.pop(correction.image_artifact_path)
    monkeypatch.setattr('tools.card_data.generate.read_consistent_revision',
        lambda *a, **k: SimpleNamespace(values=artifacts, revision='fixture'))
    with pytest.raises(ValidationError):
        load_committed_generation_stack(tmp_path, marker_path=tmp_path/'marker', journal_path=tmp_path/'journal')


def test_duplicate_target_and_id_corrections_rejected():
    from dataclasses import replace
    from tools.card_data.compatibility_corrections import CorrectionDocument
    _, _, correction, _ = bound_case()
    for other in (correction, replace(correction, correction_id='other-review')):
        with pytest.raises(ValidationError):
            CorrectionDocument(compatibility_corrections=(correction, other))


def test_publication_plan_rejects_unowned_correction_image(tmp_path):
    from tools.card_data.compatibility_corrections import CorrectionDocument, correction_document_payload
    from tools.card_data.publication import build_publication_plan
    from tools.card_data.serialization import canonical_json_bytes
    _, _, correction, _ = bound_case()
    kwargs, _, _ = full_case(tmp_path)
    with pytest.raises(ValidationError):
        build_publication_plan(repo_root=tmp_path, source_plan=kwargs['source_plan'],
            marker_path=tmp_path/'marker', journal_path=tmp_path/'journal', artifact_bytes={'data/card_corrections.json': canonical_json_bytes(
            correction_document_payload(CorrectionDocument(compatibility_corrections=(correction,))))})


def test_publication_cannot_drop_v2_correction_history(tmp_path):
    from tools.card_data.compatibility_corrections import CorrectionDocument, write_correction_document
    from tools.card_data.publication import build_publication_plan
    from tools.card_data.serialization import canonical_json_bytes
    kwargs, correction, _ = full_case(tmp_path)
    write_correction_document(tmp_path/'data/card_corrections.json',
        CorrectionDocument(compatibility_corrections=(correction,)))
    with pytest.raises(ValidationError, match='preserve all existing entries'):
        build_publication_plan(repo_root=tmp_path, source_plan=kwargs['source_plan'],
            marker_path=tmp_path/'marker', journal_path=tmp_path/'journal',
            artifact_bytes={'data/card_corrections.json': canonical_json_bytes({'schema_version':1,'corrections':[]})})


def test_first_v1_to_v2_publication_preserves_all_four_canonical_rows(tmp_path):
    from tools.card_data.compatibility_corrections import (
        CorrectionDocument, correction_document_payload, load_correction_document,
        write_correction_document,
    )
    from tools.card_data.publication import build_publication_plan
    from tools.card_data.serialization import canonical_json_bytes
    kwargs, correction, images = full_case(tmp_path)
    prior = load_correction_document(FIXTURE/'prior-canonical-corrections.json')
    assert len(prior.corrections) == 4 and not prior.compatibility_corrections
    write_correction_document(tmp_path/'data/card_corrections.json', prior)
    proposed = CorrectionDocument(compatibility_corrections=(correction,))
    with pytest.raises(ValidationError, match='preserve all existing entries'):
        build_publication_plan(repo_root=tmp_path, source_plan=kwargs['source_plan'],
            marker_path=tmp_path/'marker', journal_path=tmp_path/'journal', artifact_bytes={
                **images, 'data/card_corrections.json': canonical_json_bytes(correction_document_payload(proposed))})
