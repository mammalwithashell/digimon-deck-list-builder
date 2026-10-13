"""Independent whole-card checks beyond the paired correction proof."""
import copy
import json

import pytest

from test_paired_main_card_composition import inputs, compose, repin_correction, repin, F, enc
from tools.clause_coverage.paired_main_card_composition import validate_paired_main_card


def test_full_runtime_partition_and_single_corrected_effect():
    args = inputs()
    result = compose(args)
    projection = result['runtime_projection']
    original = json.loads(args['runtime_record'])
    assert set(projection['claimed_fields']).isdisjoint(projection['unclaimed_runtime_fields'])
    assert {**projection['projected_claimed_values'], **projection['unclaimed_runtime_fields']} == original
    assert set(projection['unclaimed_runtime_fields']) == {'rarity', 'card_name_jpn'}
    assert len(result['units']) == 2
    effect = result['zones']['effect']['unit_indices']
    security = result['zones']['security']['unit_indices']
    assert len(effect) == len(security) == 1 and effect != security
    assert result['units'][effect[0]]['authority'] == 'adjudicated'
    assert 'observation_id' in result['units'][security[0]]
    assert all(o['executable'] is False for o in result['structural_obligations'])


@pytest.mark.parametrize('change', ['unselected_official', 'secondary', 'face_security',
                                 'swapped_registry_indices', 'third_image'])
def test_self_issued_test_pins_do_not_hide_scope_or_sibling_drift(change):
    args = inputs()
    if change in ('unselected_official', 'secondary'):
        evidence = set(json.loads(args['correction_sidecar'])['evidence_observation_ids'])
        key = next(oid for oid,raw in args['observations'].items()
                   if (json.loads(raw)['source'] == 'digimoncard_io' if change == 'secondary'
                       else json.loads(raw)['source'] == 'official' and oid not in evidence))
        args['observations'].pop(key)
    elif change == 'face_security':
        raw = json.loads(args['canonical_card'])
        raw['faces'][0]['security_text'] += ' Draw 9.'
        args['canonical_card'] = enc(raw)
    elif change == 'swapped_registry_indices':
        raw = json.loads(args['registry_bytes'])
        values = raw['assignments']
        values['EX1-066'], values['EX12-031'] = values['EX12-031'], values['EX1-066']
        args['registry_bytes'] = enc(raw)
    else:
        args['correction_images']['unreviewed.png'] = b'foreign image'
    repin_correction(args)
    with pytest.raises(ValueError):
        compose(args)


@pytest.mark.parametrize('change', ['face_execution', 'second_main'])
def test_result_flags_cannot_grant_additional_execution(change):
    args = inputs()
    result = compose(args)
    if change == 'face_execution':
        mirror = next(o for o in result['structural_obligations'] if o['kind'] == 'paired-main-face-consistency')
        mirror['executable'] = True
    else:
        result['zones']['effect']['unit_indices'].append(result['zones']['security']['unit_indices'][0])
    with pytest.raises(ValueError):
        validate_paired_main_card(result, **args)


def test_proposed_sidecar_refresh_changes_only_the_explicit_core_dependency():
    args = inputs()
    old = json.loads((F / 'sidecar-APPROVED.json').read_bytes())
    new = json.loads(args['correction_sidecar'])
    changed = [key for key in old['dependency_sha256'] if old['dependency_sha256'][key] != new['dependency_sha256'][key]]
    assert changed == ['tools.clause_coverage.source_units']
    old['dependency_sha256'] = new['dependency_sha256']
    assert old == new
