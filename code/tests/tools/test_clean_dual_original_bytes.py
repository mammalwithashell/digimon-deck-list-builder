"""Keep original observation bytes at the reviewed Dual composition boundary."""
import copy
import hashlib
import json
from pathlib import Path

import pytest

from test_clean_dual_composition import inputs, compose

F = Path(__file__).parents[1] / 'fixtures/card_data/clean_dual_original_bytes'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def original_inputs():
    args = inputs()
    pins = json.loads((F / 'pins.json').read_bytes())['files']
    for name, digest in pins.items():
        assert sha((F / name).read_bytes()) == digest
    for oid, old in args['observations'].items():
        raw = (F / (oid + '.json')).read_bytes()
        assert raw != old and json.loads(raw) == json.loads(old)
        args['observations'][oid] = raw
    return args


def reviewed_inputs():
    args = original_inputs()
    args['association_proof'] = (F / 'association-proof.json').read_bytes()
    args['association_review'] = (F / 'association-review.json').read_bytes()
    args['review_pin'] = sha(args['association_review'])
    return args


def test_reserialized_fixture_proof_cannot_authorize_original_observation_bytes():
    with pytest.raises(ValueError, match='complete clean proof differs'):
        compose(original_inputs())


def test_rebind_changes_only_six_observation_byte_pins():
    old = json.loads(inputs()['association_proof'])
    new = json.loads((F / 'association-proof.json').read_bytes())
    old_pins = old['input_sha256'].pop('observations')
    new_pins = new['input_sha256'].pop('observations')
    assert old == new
    assert len(old_pins) == len(new_pins) == 6 and old_pins.keys() == new_pins.keys()
    assert all(old_pins[k] != new_pins[k] for k in old_pins)
    assert new_pins == {k: sha(v) for k, v in original_inputs()['observations'].items()}


def test_independently_reviewed_original_bytes_compose_with_complete_records():
    args = reviewed_inputs()
    before = copy.deepcopy(args)
    result = compose(args)
    assert args == before
    assert result['status'] == 'COMPOSED_DUAL_SOURCE'
    assert result['runtime_source_record'] == json.loads(args['runtime_projection'])
    assert result['unclaimed_runtime_fields'] == ['card_name_jpn', 'rarity']
    assert result['input_sha256']['observations'] == {k: sha(v) for k, v in args['observations'].items()}
    assert len(result['units']) == 4 and len(result['consumption']) == 12


def test_post_review_whitespace_change_still_refuses():
    args = reviewed_inputs()
    oid = next(iter(args['observations']))
    args['observations'][oid] += b'\n'
    with pytest.raises(ValueError, match='complete clean proof differs'):
        compose(args)
