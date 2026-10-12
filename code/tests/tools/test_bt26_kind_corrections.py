"""Frozen BT26-100 source mismatch: correction only, no source preference rule."""
import json
import hashlib
from dataclasses import replace
from pathlib import Path
import pytest

from tools.card_data.complete_candidate import _canonical_card_from_payload
from tools.card_data.compatibility_corrections import correction_document_from_bytes, require_document_extension
from tools.card_data.corrections import apply_correction
from tools.card_data.models import SourceObservation, SourceName, FaceKind
from tools.card_data.errors import ValidationError
from tools.card_data.generate import project_canonical_card
from tools.card_data.normalization import canonical_conflict_review_rows
from tools.card_data.serialization import observation_digest

F = Path(__file__).parents[1] / 'fixtures/card_data/bt26_kind_correction'


def inputs():
    payload = json.loads((F / 'canonical.json').read_bytes())
    card = _canonical_card_from_payload('BT26-100', payload, path=F / 'canonical.json')
    observations = {oid: SourceObservation(**{**d, 'source': SourceName(d['source'])})
                    for oid, d in json.loads((F / 'observations.json').read_bytes()).items()}
    document = correction_document_from_bytes((F / 'proposed-corrections.json').read_bytes())
    rows = [r for r in document.corrections if r.card_id == 'BT26-100']
    return card, observations, rows, document


def corrected():
    card, observations, rows, _ = inputs()
    for row in rows:
        card = apply_correction(card, row, observations=observations)
    return card


def test_printed_option_corrects_kind():
    assert corrected().fields['card_kind'] == 'Option'


def test_printed_option_corrects_face():
    assert corrected().faces[0].kind.value == 'option'


def test_history_and_all_adjacent_fields_preserved():
    original, observations, rows, doc = inputs()
    prior = correction_document_from_bytes((F / 'prior-corrections.json').read_bytes())
    require_document_extension(prior, doc)
    assert all(row in doc.corrections for row in prior.corrections)
    assert json.loads((F / 'proposed-corrections.json').read_bytes())['corrections'][:len(prior.corrections)] == json.loads((F / 'prior-corrections.json').read_bytes())['corrections']
    assert doc.compatibility_corrections == prior.compatibility_corrections
    assert len(rows) == 2 and {r.field_path for r in rows} == {'card_kind', 'faces'}
    fixed = corrected()
    assert fixed.faces == (replace(original.faces[0], kind=FaceKind.OPTION),)
    assert fixed.fields == {**original.fields, 'card_kind': 'Option'}
    assert fixed.printings == original.printings and fixed.conflicts == original.conflicts
    before, after = project_canonical_card(original), project_canonical_card(fixed)
    assert {k for k in before.keys() | after.keys() if before.get(k) != after.get(k)} == {'card_kind'}
    assert (before['card_kind'], after['card_kind']) == (1, 2)
    assert before['play_cost'] == after['play_cost'] == 3
    assert all(observation_digest(v) == oid for oid, v in observations.items())


def test_kind_only_is_insufficient_and_nested_wire_enum_is_refused():
    original, observations, rows, _ = inputs()
    kind = next(r for r in rows if r.field_path == 'card_kind')
    interim = apply_correction(original, kind, observations=observations)
    assert interim.faces[0].kind is FaceKind.TAMER
    with pytest.raises(ValidationError, match='type'):
        apply_correction(interim, replace(kind, field_path='faces.0.kind', value='option'), observations=observations)


@pytest.mark.parametrize('mutation', ['type', 'foreign_evidence', 'missing_evidence', 'unknown_path', 'no_op'])
def test_bad_corrections_refuse(mutation):
    original, observations, rows, _ = inputs()
    row = next(r for r in rows if r.field_path == 'card_kind')
    if mutation == 'type': row = replace(row, value=True)
    if mutation == 'foreign_evidence': row = replace(row, evidence_observation_ids=('0' * 64,))
    if mutation == 'missing_evidence': observations = {}
    if mutation == 'no_op': original = corrected()
    with pytest.raises((ValidationError, ValueError)):
        if mutation == 'unknown_path': row = replace(row, field_path='unknown_field')
        apply_correction(original, row, observations=observations)


def test_unknown_wire_fields_and_history_loss_refuse():
    payload = json.loads((F / 'proposed-corrections.json').read_bytes())
    payload['corrections'][-1]['invented'] = True
    with pytest.raises(ValidationError):
        correction_document_from_bytes(json.dumps(payload).encode())
    prior = correction_document_from_bytes((F / 'prior-corrections.json').read_bytes())
    _, _, _, proposed = inputs()
    with pytest.raises(ValidationError):
        require_document_extension(prior, replace(proposed, corrections=tuple(r for r in proposed.corrections if r.card_id == 'BT26-100')))


def test_post_correction_requires_exact_fresh_approval():
    _, observations, rows, _ = inputs()
    revision = json.loads((F / 'metadata.json').read_bytes())['source_revision']
    report = canonical_conflict_review_rows({'BT26-100': corrected()}, observations.values(), input_revision=revision)
    kind = next(r for r in report if r['field_path'] == 'card_kind')
    assert kind['status'] == 'missing' and kind['approval_required']
    assert kind['selected_value'] == 'Option'
    assert kind['selected_observation_id'] == rows[0].evidence_observation_ids[0]
    assert set(kind['candidate_observation_ids']) == set(observations)


def test_image_and_original_fixture_bytes_are_pinned():
    metadata = json.loads((F / 'metadata.json').read_bytes())
    for name, expected in metadata['files'].items():
        assert hashlib.sha256((F / name).read_bytes()).hexdigest() == expected
    assert metadata['image_sha256'] == 'b322655d3ee3e5c2cdf581a534223783191561e2f7c5c9f627882927378d250c'
