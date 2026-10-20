"""A condition report must retain raw disagreement around image corrections."""
from dataclasses import replace

import pytest

from test_card_data_compatibility_corrections import bound_case
from test_card_data_condition_review_report import snapshots
from tools.card_data.compatibility_corrections import CorrectionDocument
from tools.card_data.condition_review_report import build_condition_review_report
from tools.card_data.errors import ValidationError
from tools.card_data.models import ScopeObservationRef


def case():
    official, secondary, _ = snapshots()
    _, observations, correction, images = bound_case()
    rows = tuple(observations.values())
    official = replace(official, observations=rows, manifest=replace(
        official.manifest, observation_ids=tuple(observations),
        observation_refs=tuple(ScopeObservationRef(oid, 'EX3-010',
            row.extracted['printing']['printing_id']) for oid, row in observations.items()),
        parser_version=rows[0].parser_version, source_schema_version=rows[0].source_schema_version,
        expected_count=1))
    baseline = build_condition_review_report(official, secondary)
    correction = replace(correction, source_manifest_revision=baseline['aggregate_source_revision'])
    return official, secondary, correction, images, baseline


def test_reviewed_image_reminder_retains_all_raw_values_and_separate_authority():
    official, secondary, correction, images, baseline = case()
    report = build_condition_review_report(official, secondary,
        correction_document=CorrectionDocument(compatibility_corrections=(correction,)),
        correction_image_artifacts=images)
    row = report['cards'][0]
    assert baseline['unresolved_card_count'] == 1
    assert report['unresolved_card_count'] == 0
    assert row['status'] == 'reviewed_image_correction'
    assert row['correction_id'] == correction.correction_id
    assert row['corrected_value'] == correction.value
    assert row['image_sha256'] == correction.image_sha256
    assert row['value_groups'] == baseline['cards'][0]['value_groups']
    assert row['review_binding'] == baseline['cards'][0]['review_binding']
    assert row['source_issue'] == baseline['cards'][0]['source_issue']
    assert row['canonical_conflicts'] == baseline['cards'][0]['canonical_conflicts']
    assert report['canonical_required_conflict_count'] == baseline['canonical_required_conflict_count']
    assert report['runtime_cutover_approved'] is False
    assert report['training_readiness_assessed'] is False
    assert report['selections_created'] == 0


@pytest.mark.parametrize('mutation', ['stale', 'foreign-card', 'missing-image', 'changed-image', 'overlap'])
def test_report_rejects_invalid_or_ambiguous_correction_binding(mutation):
    from tools.card_data.compatibility_selections import CompatibilitySourceSelection
    official, secondary, correction, images, _ = case()
    selections = ()
    if mutation == 'stale': correction = replace(correction, source_manifest_revision='0' * 64)
    if mutation == 'foreign-card': correction = replace(correction, card_id='EX3-011')
    if mutation == 'missing-image': images = {}
    if mutation == 'changed-image': images = {correction.image_artifact_path: b'wrong'}
    if mutation == 'overlap':
        selections = (CompatibilitySourceSelection(schema_version=1, selection_id='overlap',
            selection_kind='official_condition_projection', card_id=correction.card_id,
            field_path='xros_req', source_manifest_revision=correction.source_manifest_revision,
            candidate_values=correction.candidate_values, candidate_set_digest=correction.candidate_set_digest,
            selected_observation_id=correction.base_observation_id, selected_value=correction.base_value,
            selected_value_digest=correction.base_value_digest, reviewer='test', reason='test',
            reviewed_at='2026-10-12'),)
    with pytest.raises(ValidationError):
        build_condition_review_report(official, secondary, selections=selections,
            correction_document=CorrectionDocument(compatibility_corrections=(correction,)),
            correction_image_artifacts=images)


def test_cli_loads_full_document_and_owned_image_bytes(tmp_path, monkeypatch):
    from tools.card_data import condition_review_report as module
    from tools.card_data.compatibility_corrections import write_correction_document
    official, secondary, correction, images, _ = case()
    tree = tmp_path / 'source'; tree.mkdir()
    root = tmp_path / 'reviewed-evidence'; root.mkdir()
    document_path = root / 'data/card_corrections.json'
    write_correction_document(document_path, CorrectionDocument(compatibility_corrections=(correction,)))
    for relative, raw in images.items():
        path = root / relative; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(raw)
    monkeypatch.setattr(module, 'load_review_snapshots', lambda _: (official, secondary))
    output = tmp_path / 'report.json'
    assert module.main(['--review-tree', str(tree), '--output', str(output),
        '--corrections', str(document_path), '--correction-evidence-root', str(root)]) == 0
    assert output.exists()
