"""Independent wire-boundary reds; original BT26 proposal stays frozen."""
from copy import deepcopy
from dataclasses import replace

import pytest

from test_bt26_kind_corrections import inputs
from tools.card_data.corrections import apply_correction, _coerce_face_correction
from tools.card_data.errors import ValidationError
from tools.card_data.models import CardFace, FaceKind


@pytest.mark.parametrize('field,value', [
    ('unexpected', 'discarded'), ('text_sections', []),
    ('face_id', True), ('colors', [True]),
    ('effect_text', True), ('inherited_text', 7), ('security_text', None),
    ('play_cost', True), ('level', False), ('dp', True),
])
def test_whole_face_unknown_fields_and_wrong_wire_types_refuse(field, value):
    card, observations, rows, _ = inputs()
    row = next(r for r in rows if r.field_path == 'faces')
    payload = deepcopy(row.value)
    payload[0][field] = value
    with pytest.raises((ValidationError, ValueError)):
        apply_correction(card, replace(row, value=payload), observations=observations)


def test_exact_existing_face_value_remains_supported():
    card, observations, rows, _ = inputs()
    row = next(r for r in rows if r.field_path == 'faces')
    fixed = apply_correction(card, row, observations=observations)
    assert fixed.faces[0].kind.value == 'option'
    assert fixed.faces[0].play_cost == 3


@pytest.mark.parametrize('kind,expected', [('Option', FaceKind.OPTION), ('Digi-Egg', FaceKind.DIGI_EGG), (FaceKind.OPTION, FaceKind.OPTION)])
def test_minimal_defaults_and_kind_normalization_remain(kind, expected):
    face, = _coerce_face_correction(({'face_id': 'primary', 'kind': kind, 'colors': ('Red',)},))
    assert face.kind is expected and face.effect_text == ''
    assert face.play_cost is face.level is face.dp is None


def test_zero_null_empty_collection_and_typed_face_controls():
    face = CardFace('primary', FaceKind.OPTION, ('Red',), play_cost=0, level=None, dp=0)
    assert _coerce_face_correction([face]) == (face,)
    assert _coerce_face_correction([face])[0] is face
    assert _coerce_face_correction([]) == _coerce_face_correction(()) == ()


@pytest.mark.parametrize('kind,expected', [('option', FaceKind.OPTION), ('Option', FaceKind.OPTION), ('Digi-Egg', FaceKind.DIGI_EGG)])
def test_typed_string_kind_returns_normalized_enum(kind, expected):
    face = replace(CardFace('primary', FaceKind.OPTION, ('Red',)), kind=kind)
    result, = _coerce_face_correction([face])
    assert result.kind is expected
    assert result is not face


@pytest.mark.parametrize('field,value', [('play_cost', True), ('effect_text', 7), ('colors', (True,)), ('face_id', ''), ('colors', ()), ('colors', ('',))])
def test_typed_face_does_not_bypass_wire_value_checks(field, value):
    face = CardFace('primary', FaceKind.OPTION, ('Red',))
    # Deliberately malformed object without invoking the model's own checks.
    object.__setattr__(face, field, value)
    with pytest.raises(ValidationError):
        _coerce_face_correction([face])


@pytest.mark.parametrize('payload', [
    {'kind': 'option', 'colors': ['Red']},
    {'face_id': 'primary', 'colors': ['Red']},
    {'face_id': 'primary', 'kind': 'option'},
    {'face_id': 'primary', 'kind': 'unknown', 'colors': ['Red']},
    {'face_id': 'primary', 'kind': 'option', 'colors': []},
    {'face_id': 'primary', 'kind': 'option', 'colors': ['Red'], 'dp': -1},
])
def test_required_fields_enum_and_existing_numeric_constraints_remain(payload):
    with pytest.raises(ValidationError):
        _coerce_face_correction([payload])
