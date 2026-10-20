"""Pure face inventory: actual source gaps must remain visible."""
import copy
import json
from pathlib import Path

import pytest

FIXTURE = Path(__file__).parents[1] / 'fixtures/card_data/dual_source_units/BT26-033.json'


def observations():
    return json.loads(FIXTURE.read_text(encoding='utf8'))['observations']


def derive(o):
    from tools.clause_coverage.dual_source_units import derive_dual_occurrences
    return derive_dual_occurrences(o)


def test_all_actual_printings_preserve_faces_global_inventory_and_kind_disagreement():
    for o in observations():
        before = copy.deepcopy(o)
        result = derive(o)
        assert o == before == result['source_observation']
        assert result['status'] == 'BLOCKED_DUAL_DIAGNOSTIC'
        assert [b['code'] for b in result['blockers']] == ['TOP_LEVEL_KIND_DISAGREES_WITH_FACES']
        assert len(result['occurrences']) == 4
        assert [u['section_index'] for u in result['occurrences']] == list(range(4))
        assert [u['face_id'] for u in result['occurrences']] == ['primary', 'primary', 'dual', 'dual']
        assert [u['role'] for u in result['occurrences']] == ['condition', 'effect', 'effect', 'arts-reminder']
        for u in result['occurrences']:
            section = o['extracted']['text_sections'][u['section_index']]
            assert u['whole_container'] == section['text'] == u['text']
            assert u['char_start'] == u['utf8_byte_start'] == 0
            assert u['char_end'] == len(u['text'])
            assert u['utf8_byte_end'] == len(u['text'].encode('utf8'))
        for face in result['face_inventory']:
            assert face['zones']['inherited']['presence'] == 'absent-box'
            assert face['zones']['security']['presence'] == 'absent-box'
        assert 'clauses' not in result and 'runtime' not in result


@pytest.mark.parametrize('mutation', ['drop_face', 'swap_faces', 'duplicate_face', 'extra_face',
    'wrong_face_kind', 'wrong_face_id', 'drop_global', 'drop_local', 'change_sibling',
    'swap_sections', 'move_cross_face', 'unknown_label', 'unknown_rule', 'inherited', 'security'])
def test_structural_mutations_block_without_losing_original(mutation):
    o = observations()[0]; e = o['extracted']; faces = e['faces']; sections = e['text_sections']
    e['card_type'] = 'Digimon/Option'  # Synthetic control, never source repair.
    if mutation == 'drop_face': faces.pop()
    elif mutation == 'swap_faces': faces.reverse()
    elif mutation == 'duplicate_face': faces[1] = copy.deepcopy(faces[0])
    elif mutation == 'extra_face': faces.append(copy.deepcopy(faces[1]))
    elif mutation == 'wrong_face_kind': faces[1]['kind'] = 'tamer'
    elif mutation == 'wrong_face_id': faces[1]['face_id'] = 'other'
    elif mutation == 'drop_global': sections.pop()
    elif mutation == 'drop_local': faces[1]['text_sections'].pop()
    elif mutation == 'change_sibling': faces[0]['text_sections'][1]['text'] += ' Draw 1.'
    elif mutation == 'swap_sections': sections.reverse()
    elif mutation == 'move_cross_face': faces[1]['text_sections'].insert(0, faces[0]['text_sections'].pop())
    else:
        replacement = {'label': 'Unknown', 'text': 'Retain this source'}
        if mutation == 'unknown_rule': replacement = {'label': 'DUAL Rule', 'text': '(Rule) Has [Cyborg] Type.'}
        if mutation in ('inherited', 'security'):
            replacement = {'label': mutation.capitalize() + ' Effect', 'text': 'Draw 1.'}
        faces[1]['text_sections'].append(copy.deepcopy(replacement)); sections.append(replacement)
    before = copy.deepcopy(o); result = derive(o)
    assert result['status'] == 'BLOCKED_DUAL_DIAGNOSTIC' and result['blockers']
    assert result['source_observation'] == before == o


def test_repeated_source_occurrence_is_not_deduplicated():
    o = observations()[0]; e = o['extracted']; e['card_type'] = 'Digimon/Option'
    repeated = copy.deepcopy(e['faces'][0]['text_sections'][1])
    e['faces'][0]['text_sections'].append(copy.deepcopy(repeated)); e['text_sections'].insert(2, repeated)
    r = derive(o)
    assert r['status'] == 'DERIVED_DUAL_DIAGNOSTIC'
    assert len(r['occurrences']) == 5
    assert [u['section_index'] for u in r['occurrences']] == list(range(5))


def test_unsupported_layout_and_nonempty_grants_remain_blockers():
    o = observations()[0]; o['extracted']['layout'] = 'single'
    assert derive(o)['status'] == 'BLOCKED_DUAL_DIAGNOSTIC'
    o = observations()[0]; o['extracted']['rule_grants']['forms'] = ['Hybrid']
    assert any(b['code'] == 'UNSUPPORTED_DUAL_RULE_GRANTS' for b in derive(o)['blockers'])


@pytest.mark.parametrize('mutation', ['missing_both_arts', 'duplicate_arts', 'empty_primary', 'empty_dual'])
def test_required_face_content_cannot_disappear_from_both_views(mutation):
    o = observations()[0]; e = o['extracted']; e['card_type'] = 'Digimon/Option'
    if mutation == 'missing_both_arts': e['faces'][1]['text_sections'].pop()
    elif mutation == 'duplicate_arts': e['faces'][1]['text_sections'].append(copy.deepcopy(e['faces'][1]['text_sections'][-1]))
    elif mutation == 'empty_primary': e['faces'][0]['text_sections'] = []
    else: e['faces'][1]['text_sections'] = []
    e['text_sections'] = [copy.deepcopy(s) for f in e['faces'] for s in f['text_sections']]
    assert derive(o)['status'] == 'BLOCKED_DUAL_DIAGNOSTIC'


def test_repeated_empty_effect_box_does_not_erase_nonempty_presence():
    o = observations()[0]; e = o['extracted']; e['card_type'] = 'Digimon/Option'
    e['faces'][0]['text_sections'].append({'label': 'Effect', 'text': ''})
    e['text_sections'].insert(2, {'label': 'Effect', 'text': ''})
    r = derive(o)
    assert r['status'] == 'DERIVED_DUAL_DIAGNOSTIC'
    assert r['face_inventory'][0]['zones']['effect']['presence'] == 'nonempty'


@pytest.mark.parametrize('o', [None, [], {}, {'extracted': None}])
def test_malformed_input_refused(o):
    from tools.clause_coverage.dual_source_units import DualUnitError
    with pytest.raises(DualUnitError): derive(o)
