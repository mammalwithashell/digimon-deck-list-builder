"""Actual original source/legacy recipes; no admission or gameplay claim."""
import copy
import gzip
import hashlib
import json
from pathlib import Path

import pytest
from tools.card_data.serialization import canonical_json_bytes as enc

F = Path(__file__).parents[1] / 'fixtures/card_data/assembly_recipe_binding'
sha = lambda b: hashlib.sha256(b).hexdigest()


def inputs(cid='BT24-081'):
    data = json.loads((F / (cid + '.json')).read_bytes())
    aggregate = gzip.decompress((F / 'aggregate.json.gz').read_bytes())
    revision = json.loads(aggregate)['manifest_revision']
    corrections = (F / 'corrections.json').read_bytes()
    image = F.parent / 'source_binding/conditions/53865c5601afb4e96466c3acf50b8ee4c73930e7b36baca1211e75453494b9c0.webp'
    images = {r['image_artifact_path']: image.read_bytes()
              for r in json.loads(corrections)['compatibility_corrections']}
    runtime = data['runtime_record']
    a = dict(aggregate=aggregate,
        observations={Path(n).stem: (F / n).read_bytes() for n in data['observations']},
        canonical_card=enc(data['canonical_card']),
        runtime_projection=enc({'card_id': cid, 'xros_req': runtime['xros_req']}),
        selection_ledger=(F / 'selections.json').read_bytes(),
        correction_document=corrections, image_artifacts=images,
        raw_pages={url: gzip.decompress((F / n).read_bytes()) for url, n in data['raw_pages'].items()},
        old_input_revision=revision, new_input_revision=revision)
    legacy = enc(runtime['digixros_costs']); yaml = (F / (cid + '.yaml')).read_bytes()
    return dict(condition_inputs=a, legacy_field=legacy,
                expected_legacy_field_sha256=sha(legacy), yaml_bytes=yaml,
                expected_yaml_sha256=sha(yaml))


def bind(a):
    from tools.clause_coverage.assembly_recipe_binding import bind_assembly_recipe
    return bind_assembly_recipe(**a)


@pytest.mark.parametrize('cid,cost,count', [('BT24-081', 6, 2), ('BT26-073', 2, 1)])
def test_actual_complete_condition_and_legacy_recipe(cid, cost, count):
    a = inputs(cid); before = copy.deepcopy(a); r = bind(a)
    assert a == before
    assert r['status'] == 'BOUND_ASSEMBLY_RECIPE_DIAGNOSTIC'
    assert r['semantic_recipe']['total_reduction'] == cost
    assert len(r['semantic_recipe']['materials']) == count
    assert r['semantic_recipe']['source_zones'] == ['trash']
    assert r['semantic_recipe']['stack_order'] == 'left_to_right_top_to_bottom'
    assert r['legacy_value'] == json.loads(a['legacy_field'])
    assert r['condition_binding']['projection'] == json.loads(a['condition_inputs']['runtime_projection'])['xros_req']
    assert len(r['source_occurrences']) == 2
    assert all(x['condition_text'].startswith('Assembly -') for x in r['source_occurrences'])
    assert not r['source_authority'] and not r['native_compilation_verified']
    assert not r['training_source_complete'] and r['unreviewed_effect_paths']
    if cid == 'BT24-081':
        assert r['semantic_recipe']['materials'] == [{'name_is': 'Titamon', 'count': 1}, {'name_is': 'SkullBaluchimon', 'count': 1}]
        assert r['unresolved_obligations'] == [{'code': 'NAMED_MATERIAL_KIND_CLOSURE_REQUIRED', 'names': ['Titamon', 'SkullBaluchimon'], 'yaml_kind': 'digimon'}]
    else:
        assert r['unresolved_obligations'] == []
        assert r['semantic_recipe']['materials'][0]['predicate'] == {'all_of': [{'kind': 'digimon'}, {'level_lte': 4}, {'any_of': [{'in_text_contains': 'Chronomon'}, {'trait_has': 'TS'}]}]}
        assert len(r['condition_binding']['units']) == 2  # Aegiomon evolution retained.


@pytest.mark.parametrize('change', ['reduction', 'reduction_bool', 'name', 'count_bool', 'count', 'level', 'origin', 'unknown', 'missing', 'different_names'])
def test_rehashed_whole_legacy_value_tampering_refused(change):
    a = inputs(); value = json.loads(a['legacy_field'])
    if change == 'reduction': value[0]['reduce_cost_per_card'] = 12
    elif change == 'reduction_bool': value[0]['reduce_cost_per_card'] = True
    elif change == 'name': value[0]['elements'][0]['name_contains'] = 'Titamon X'
    elif change == 'count_bool': value[0]['elements'][0]['count'] = True
    elif change == 'count': value[0]['elements'][0]['count'] = 2
    elif change == 'level': value[0]['elements'][0]['level_max'] = 4
    elif change == 'origin': value[0]['source_zones'] = ['hand']
    elif change == 'unknown': value[0]['approved'] = True
    elif change == 'missing': value[0].pop('has_text')
    else: value[0]['different_names'] = True
    a['legacy_field'] = enc(value); a['expected_legacy_field_sha256'] = sha(a['legacy_field'])
    with pytest.raises(ValueError): bind(a)


@pytest.mark.parametrize('old,new', [('cost: 6', 'cost: 12'), ('name_is: "Titamon"', 'name_is: "Titamon X"'),
    ('zones: [trash]', 'zones: [hand]'), ('stack_under: true', 'stack_under: false'),
    ('cost: 6', 'cost: "6"'), ('cost: 6', 'cost: true'), ('kind: assembly', 'kind: digixros'),
    ('cost: 6', 'cost: 6\n    extra: true'), ('cost: 6', 'cost: 6\n    cost: 6'),
    ('name_is: "Titamon"', 'name_is: "SkullBaluchimon"')])
def test_rehashed_yaml_recipe_tampering_refused(old, new):
    a = inputs(); assert old.encode() in a['yaml_bytes']
    a['yaml_bytes'] = a['yaml_bytes'].replace(old.encode(), new.encode())
    a['expected_yaml_sha256'] = sha(a['yaml_bytes'])
    with pytest.raises(ValueError): bind(a)


@pytest.mark.parametrize('old,new', [('level_lte: 4', 'level_lte: 5'), ('any_of:', 'all_of:'),
    ('in_text_contains: "Chronomon"', 'name_is: "Chronomon"'), ('trait_has: TS', 'trait_has: Shaman'),
    ('kind: digimon', 'kind: tamer')])
def test_outer_level_and_kind_govern_both_branches(old, new):
    a = inputs('BT26-073'); a['yaml_bytes'] = a['yaml_bytes'].replace(old.encode(), new.encode())
    a['expected_yaml_sha256'] = sha(a['yaml_bytes'])
    with pytest.raises(ValueError): bind(a)


@pytest.mark.parametrize('change', ['missing_observation', 'raw_drift', 'revision', 'runtime_condition', 'injected_binding', 'legacy_pin', 'yaml_pin'])
def test_complete_input_bindings_required(change):
    a = inputs(); c = a['condition_inputs']
    if change == 'missing_observation': c['observations'].pop(next(iter(c['observations'])))
    elif change == 'raw_drift': c['raw_pages'] = {k: v + b' ' for k, v in c['raw_pages'].items()}
    elif change == 'revision': c['old_input_revision'] = '0' * 64
    elif change == 'runtime_condition': c['runtime_projection'] = enc({'card_id': 'BT24-081', 'xros_req': ''})
    elif change == 'injected_binding': c['validated'] = True
    elif change == 'legacy_pin': a['expected_legacy_field_sha256'] = '0' * 64
    else: a['expected_yaml_sha256'] = '0' * 64
    with pytest.raises((ValueError, TypeError)): bind(a)


def test_fixture_raw_pins_are_exact():
    p = json.loads((F / 'provenance.json').read_bytes())
    for name, row in p['files'].items():
        raw = (F / name).read_bytes()
        if row['compressed']: raw = gzip.decompress(raw)
        assert sha(raw) == row['sha256']


@pytest.mark.parametrize('text', [
    'Assembly -0: [A]×[B]', 'Assembly -65536: [A]×[B]',
    'Assembly -6: [A]×[B] extra', 'Assembly -6: [A]×[B]\nAssembly -1: [C]',
    'Assembly -6: [A]×[A]', 'Assembly -6: [A]×', 'Assembly -6: [A]×[B]×[C]',
    'Assembly -6: [ A]×[B]', 'Assembly -6: [A] x [B]', 'DigiXros -6: [A]×[B]',
    'Assembly -2: Lv.256 or lower Digimon card w/[Chronomon] in text or w/[TS] trait',
    'Assembly -2: Lv.4 or lower card w/[Chronomon] in text or w/[TS] trait',
    'Assembly -2: Lv.4 or lower Digimon card w/[Chronomon] in text and w/[TS] trait',
    'Assembly -2: Lv.4 or lower Digimon card w/[Chronomon] in name or w/[TS] trait',
    'Assembly -2: Lv.4 or lower Digimon card w/[Chronomon] in text or w/[TS] trait extra',
])
def test_unsupported_complete_recipe_grammar(text):
    from tools.clause_coverage.assembly_recipe_binding import _recipe
    with pytest.raises(ValueError): _recipe(text)


@pytest.mark.parametrize('cid,old,new', [
    ('BT24-081', 'stack_under: true', 'stack_under: "true"'),
    ('BT24-081', 'zones: [trash]', 'zones: [trash], count: 1'),
    ('BT26-073', 'level_lte: 4', 'level_lte: "4"'),
    ('BT26-073', 'stack_under: true', 'stack_under: true\n        count: 2'),
])
def test_native_scalar_and_implicit_material_count_contract(cid, old, new):
    a = inputs(cid); a['yaml_bytes'] = a['yaml_bytes'].replace(old.encode(), new.encode())
    a['expected_yaml_sha256'] = sha(a['yaml_bytes'])
    with pytest.raises(ValueError): bind(a)
