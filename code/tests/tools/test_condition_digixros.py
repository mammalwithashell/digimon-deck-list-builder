"""Segmentation only: these fixtures do not constitute A2 source approval."""
import copy
import json
from pathlib import Path

import pytest

from tools.clause_coverage.condition_digixros import split_digixros_condition_units as split
from tools.clause_coverage.condition_spans import ConditionSpanError, split_condition_units

FIXTURES = Path(__file__).parents[1] / 'fixtures/card_data/condition_digixros'


def unit(text, **changes):
    return {'kind': 'official-section', 'field': 'xros_req',
            'label': 'Special Play Condition', 'condition_type': 'special_play_condition',
            'text': text, **changes}


@pytest.mark.parametrize('cid', ['BT10-024', 'BT10-063', 'BT12-011', 'EX3-014'])
def test_actual_source_is_one_preserved_cost_route(cid):
    fixture = json.loads((FIXTURES / f'{cid}.json').read_text(encoding='utf-8'))
    original = fixture['source_unit']
    section = fixture['observation']['extracted']['text_sections'][original['section_index']]
    assert original['text'] == section['text']
    rows = split([original])
    assert rows == [{'unit_index': 0, 'source_unit': original, 'spans': [{
        'span_index': 0, 'kind': 'timing', 'timings': ['DigiXros'],
        'keyword': None, 'text': original['text'].removeprefix('DigiXros').lstrip()}]}]
    assert rows[0]['source_unit'] is not original
    # Both original section metadata and full reduction/material/reminder survive.
    if cid == 'BT10-063':
        assert rows[0]['spans'][0]['text'].count('[Monitamon]') == 3
    if cid == 'BT12-011':
        assert '＜Save＞' in rows[0]['spans'][0]['text']
    if cid == 'EX3-014':
        assert 'different names' in rows[0]['spans'][0]['text']
        assert '\nWhen you would play' in rows[0]['spans'][0]['text']


@pytest.mark.parametrize('text', [
    'DigiXros -1:[Shoutmon]×[Ballistamon]',
    'DigiXros-2: Lv.4 w/[Tyrannomon] in name',
    'DigiXros -1: ∞ Digimon cards with different card numbers',
])
def test_body_never_normalizes_multiplicity_or_materials(text):
    assert split([unit(text)])[0]['spans'][0]['text'] == text[len('DigiXros'):].lstrip()


def test_order_and_duplicate_units_and_input_immutability():
    u = unit('DigiXros -2: [Monitamon] × [Monitamon] × [Monitamon]')
    other = unit('[Digivolve] Cost 3', label='Special Digivolution Condition',
                 condition_type='special_digivolution_condition')
    original = [u, other, u]
    before = copy.deepcopy(original)
    rows = split(original)
    assert original == before
    assert [r['unit_index'] for r in rows] == [0, 1, 2]
    assert [r['source_unit'] for r in rows] == before
    assert rows[0]['source_unit'] is not rows[2]['source_unit']
    assert rows[1]['spans'] == split_condition_units([other])[0]['spans']


@pytest.mark.parametrize('text', [
    'DigiXros', 'DigiXros -2:', 'DigiXros -2 from ', 'DigiXros +2: card',
    'DigiXros -2junk: card', 'DigiXros -2.5: card', 'DigiXros --2: card',
    'DigiXros -0: card', 'DigiXros -2: [Main] Draw 1.',
    'DigiXros -2: ＜Main＞ Draw 1.', 'DigiXros -2: <On Play> Draw 1.',
    'DigiXros -2: 1 card with [On Play] in text',
    'DigiXros -2: card\n[On Play] Draw 1.',
    'DigiXros -2: card\nDigiXros -3: other',
    'DigiXros -2: [Material]\nDigiXros +2: [Other]',
    'DigiXros -2: [Material]\nDigiXros -X: [Other]',
    'DigiXros -2: [Material]\nAssembly -2: [Other]',
    'DigiXros -2: [Material]\nApp Fusion [Other]: Cost 0',
    'DigiXros -2: card\n[Digivolve] Cost 0',
    'DigiXros -2: card\n[Special Play Condition] Assembly -1: card',
    '[App Fusion] [Globemon] & [Charismon]: Cost 0',
    '[DNA Digivolution] Cost 0',
])
def test_unsupported_or_mixed_routes_refused(text):
    with pytest.raises(ConditionSpanError):
        split([unit(text)])


@pytest.mark.parametrize('changes', [
    {'label': 'DigiXros', 'condition_type': 'digixros'},
    {'label': 'Special Digivolution Condition', 'condition_type': 'special_digivolution_condition'},
    {'field': 'main'}, {'kind': 'image-correction'}, {'condition_type': 'unknown'},
])
def test_plain_alias_is_scoped_to_exact_supported_source_section(changes):
    with pytest.raises(ConditionSpanError):
        split([unit('DigiXros -2: card', **changes)])


@pytest.mark.parametrize('units', [None, {}, 'DigiXros', [None], [1]])
def test_bad_collections_refused(units):
    with pytest.raises(ConditionSpanError):
        split(units)


def test_existing_bracketed_conditions_and_empty_collection_unchanged():
    u = unit('[DigiXros] -2: card')
    assert split([u]) == split_condition_units([u])
    assert split([]) == []
