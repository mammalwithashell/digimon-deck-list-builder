"""Pure segmentation tests; fixture authority is validated independently by A2."""
import copy
import pytest

from tools.clause_coverage.text_split import split_clauses
from test_clause_source_binding import condition_fixture
from tools.clause_coverage.source_binding import bind_condition_units, validate_condition_units


@pytest.fixture(scope='module')
def actual():
    result = {}
    for cid in ('EX12-046', 'EX12-063', 'EX12-076', 'EX3-010'):
        args = condition_fixture(cid)
        binding = bind_condition_units(**args)
        validate_condition_units(binding, **args)
        result[cid] = binding['units']
    return result


def split(units):
    from tools.clause_coverage.condition_spans import split_condition_units
    return split_condition_units(units)


def unit(text, label='Special Digivolution Condition'):
    # Synthetic segmentation input, not a source approval or identity fixture.
    return {'kind': 'official-section', 'field': 'xros_req', 'label': label,
            'condition_type': label.lower().replace(' ', '_'), 'text': text}


@pytest.mark.parametrize('cid', ['EX12-046', 'EX12-063', 'EX12-076'])
def test_real_separate_assembly(actual, cid):
    before = copy.deepcopy(actual[cid])
    rows = split(actual[cid])
    assert [r['unit_index'] for r in rows] == [0, 1]
    assert rows[0]['spans'][0]['timings'] == ['Digivolve']
    assert rows[1]['spans'] == [{'span_index': 0, 'kind': 'timing',
        'timings': ['Assembly'], 'keyword': None,
        'text': before[1]['text'].removeprefix('Assembly').lstrip()}]
    assert [r['source_unit'] for r in rows] == before == actual[cid]
    rows[0]['source_unit']['text'] = 'mutated output'
    assert actual[cid] == before


def test_actual_restrictions_preserved(actual):
    nonwhite = split(actual['EX12-063'])[0]['spans'][0]['text']
    assert nonwhite == 'Non-white Lv.4 w/[Puppet]/[Shambala] trait: Cost 3'
    susanoo = split(actual['EX12-076'])
    assert '[TS]' in susanoo[0]['spans'][0]['text']
    assert susanoo[1]['spans'][0]['text'] == '-9: 8 [Hybrid]/[Shambala] trait cards w/different names'
    assert '[TS]' not in susanoo[1]['spans'][0]['text']


def test_real_correction_remains_one_image_unit(actual):
    rows = split(actual['EX3-010'])
    assert len(rows) == 1
    assert rows[0]['source_unit'] == actual['EX3-010'][0]
    assert rows[0]['source_unit']['kind'] == 'image-correction'
    assert rows[0]['spans'] == [{'span_index': 0, 'kind': 'timing',
        'timings': ['DNA Digivolve'], 'keyword': None,
        'text': '0 from red Lv.4 + purple Lv.4\nDigivolve unsuspended with the 2 specified Digimon stacked on top of each other.'}]


def test_two_printed_jupiter_cost_alternatives():
    # BT24-101 printed lines; exact strings also present in official observations.
    # Official v5 observation 2bcdc6d9c9214bba3f56078831ff3bd32ebb51be7d653d9364edf960bc0fb40a.
    text = '[Digivolve] Lv.5 w/[TS] trait: Cost 5\n[Digivolve] Lv.5 w/[Aegiochusmon] in name: Cost 1 for each of your security cards'
    rows = split([unit(text)])
    assert [s['span_index'] for s in rows[0]['spans']] == [0, 1]
    assert all(s['timings'] == ['Digivolve'] for s in rows[0]['spans'])
    assert [s['text'] for s in rows[0]['spans']] == [line.removeprefix('[Digivolve] ') for line in text.splitlines()]


def test_repeated_units_and_marker_only_spans_are_not_deduplicated():
    u = unit('[Digivolve]')
    rows = split([u, u])
    assert [r['unit_index'] for r in rows] == [0, 1]
    assert all(r['spans'] == [{'span_index': 0, 'kind': 'timing',
        'timings': ['Digivolve'], 'keyword': None, 'text': ''}] for r in rows)
    assert rows[0]['source_unit'] is not rows[1]['source_unit']


@pytest.mark.parametrize('change', [
    {'kind': 'unknown'}, {'field': 'main'}, {'label': 'Effect'},
    {'condition_type': 'unknown'}, {'label': 'Future Condition'},
    {'text': ''}, {'text': ' \n '}, {'text': None}, {'text': 1},
])
def test_bad_unit_refused(change):
    from tools.clause_coverage.condition_spans import ConditionSpanError
    with pytest.raises(ConditionSpanError):
        split([{**unit('[Digivolve] Cost 3'), **change}])


@pytest.mark.parametrize('value', [None, {}, 'text', [None], [1]])
def test_bad_collection_refused(value):
    from tools.clause_coverage.condition_spans import ConditionSpanError
    with pytest.raises(ConditionSpanError):
        split(value)


def test_no_condition_units_is_valid():
    assert split([]) == []


def test_inline_assembly_not_globally_rewritten():
    from tools.clause_coverage.condition_spans import ConditionSpanError
    text = '[Main] This Digimon gains Assembly -2 until end of turn.'
    original = split_clauses(text)
    with pytest.raises(ConditionSpanError):
        split([unit(text, 'Effect')])
    assert split_clauses(text) == original
    assert len(original) == 1 and original[0].timings == ['Main']
    assert 'Assembly -2' in original[0].text


@pytest.mark.parametrize('text', [
    '[Special Play Condition] Assembly -2: card',
    '[Special Digivolution Condition] [DNA Digivolve] Cost 0\n[Special Play Condition] Assembly -2: card',
    '[Special Digivolution Condition] [Digivolve] Cost 3\n[Special Digivolution Condition] [Digivolve] Cost 4',
    '[Special Digivolution Condition] [Unknown Condition] text',
    '[Special Digivolution Condition] [Main] text',
    '[Special Digivolution Condition] [Digivolve] Cost 3\n[Future Header] text',
    '[Special Digivolution Condition] [Digivolve] Cost 3\n[Main] Draw 1.',
    '[Special Digivolution Condition] plain text',
    '[Special Digivolution Condition]',
])
def test_unsupported_image_segmentation_refused(actual, text):
    from tools.clause_coverage.condition_spans import ConditionSpanError
    u = {**actual['EX3-010'][0], 'text': text}
    with pytest.raises(ConditionSpanError):
        split([u])


def test_image_segmentation_does_not_hardcode_card_or_correction(actual):
    u = {**actual['EX3-010'][0], 'correction_id': 'another-reviewed-correction'}
    assert split([u])[0]['spans'] == split(actual['EX3-010'])[0]['spans']


def test_mislabeled_main_is_not_accepted_as_special_play():
    from tools.clause_coverage.condition_spans import ConditionSpanError
    with pytest.raises(ConditionSpanError):
        split([unit('[Main] This Digimon gains Assembly -2.', 'Special Play Condition')])


def test_inline_assembly_in_condition_is_not_rewritten():
    text = '[Digivolve] Lv.5 with Assembly in its text: Cost 3'
    assert split([unit(text)])[0]['spans'][0]['text'] == 'Lv.5 with Assembly in its text: Cost 3'


@pytest.mark.parametrize('text,expected', [
    ('[Digivolve]\n[Digivolve]', [('Digivolve', ''), ('Digivolve', '')]),
    ('[Digivolve]\n[DNA Digivolve] Cost 0', [('Digivolve', ''), ('DNA Digivolve', 'Cost 0')]),
    ('[Digivolve] [Digivolve] Cost 3', [('Digivolve', ''), ('Digivolve', 'Cost 3')]),
    ('＜Digivolve＞\n[DNA Digivolve]', [('Digivolve', ''), ('DNA Digivolve', '')]),
])
def test_each_cost_marker_is_a_distinct_occurrence(text, expected):
    spans = split([unit(text)])[0]['spans']
    assert [s['span_index'] for s in spans] == list(range(len(expected)))
    assert [(s['timings'][0], s['text']) for s in spans] == expected
    assert all(len(s['timings']) == 1 for s in spans)


@pytest.mark.parametrize('text,label', [
    ('Arbitrary unsupported prose.', 'Special Digivolution Condition'),
    ('Fusion -2: new unsupported grammar', 'Special Play Condition'),
    ('Unclassified preface. [Digivolve] Cost 3', 'Special Digivolution Condition'),
    ('Assembly -2: card', 'Special Digivolution Condition'),
    ('<Future Mechanic> condition', 'Special Play Condition'),
])
def test_unknown_condition_grammar_refused(text, label):
    from tools.clause_coverage.condition_spans import ConditionSpanError
    with pytest.raises(ConditionSpanError):
        split([unit(text, label)])
