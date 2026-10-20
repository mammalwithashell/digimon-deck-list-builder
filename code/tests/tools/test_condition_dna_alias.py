"""DNA marker spelling tests, without source approval or clause-book output."""
import copy
import json
from pathlib import Path

import pytest

from tools.clause_coverage.condition_dna_alias import split_dna_condition_units as split
from tools.clause_coverage.condition_digixros import split_digixros_condition_units
from tools.clause_coverage.condition_spans import ConditionSpanError

FIXTURES = Path(__file__).parents[1] / 'fixtures/card_data/condition_dna_alias'


def unit(text, **changes):
    return {'kind':'official-section', 'field':'xros_req',
            'label':'Special Digivolution Condition',
            'condition_type':'special_digivolution_condition', 'text':text, **changes}


@pytest.mark.parametrize('cid', ['BT13-059', 'BT16-012', 'BT17-078', 'EX6-062'])
def test_actual_printed_alias_preserves_full_source_body(cid):
    f=json.loads((FIXTURES/f'{cid}.json').read_text(encoding='utf-8'))
    u=f['source_unit']; before=copy.deepcopy(u)
    assert f['observation']['extracted']['text_sections'][u['section_index']]['text']==u['text']
    result=split([u])
    assert result==[{'unit_index':0,'source_unit':before,'spans':[{
        'span_index':0,'kind':'timing','timings':['DNA Digivolve'],'keyword':None,
        'text':u['text'].removeprefix('[DNA Digivolution]').lstrip()}]}]
    assert u==before and result[0]['source_unit'] is not u


def test_marker_only_and_repeated_units_stay_distinct():
    u=unit('[DNA Digivolution]')
    r=split([u,u]); assert [x['unit_index']for x in r]==[0,1]
    assert all(x['spans'][0]['text']=='' for x in r)
    assert r[0]['source_unit'] is not r[1]['source_unit']


def test_other_approved_grammars_unchanged():
    units=[unit('[Digivolve] Cost 3'),unit('DigiXros -2: [Material]',
        label='Special Play Condition',condition_type='special_play_condition')]
    assert split(units)==split_digixros_condition_units(units)
    assert split([])==[]


@pytest.mark.parametrize('text', [
    'Preamble [DNA Digivolution] Cost 0',
    '[DNA Digivolution] 0 from red Lv.4 + blue Lv.4\n[Main] Draw 1.',
    '[DNA Digivolution] [Main] Draw 1.',
    '[DNA Digivolution] ＜On Play＞ Draw 1.',
    '[DNA Digivolution] Cost 0 [Digivolve] Cost 1',
    '[DNA Digivolution] Cost 0\n[DNA Digivolution] Cost 0',
    '[DNA Digivolution] Cost 0\n[App Fusion] card',
    '[DNA Digivolution] Cost 0\nAssembly -2: card',
    '[DNA Digivolution] Cost 0\nDigiXros +2: card',
    '[DNA Digivolution] Cost 0\n[Special Play Condition] card',
    '[DNA Digivolution] Cost 0\nDigivolve Lv.5: Cost 3',
    '[DNA Digivolution] Cost 0\nDNA Digivolve Cost 0',
    '[DNA Digivolution] Cost 0 Digivolve Lv.5: Cost 3',
    '[DNA Digivolution] Cost 0\nDigivolve unsuspended with the 2 specified Digimon stacked on top of each other.\nDigivolve Lv.5: Cost 3',
])
def test_unsupported_mixed_or_embedded_alias_refused(text):
    with pytest.raises(ConditionSpanError): split([unit(text)])


@pytest.mark.parametrize('changes', [
    {'kind':'image-correction'}, {'field':'main'},
    {'label':'Special Play Condition','condition_type':'special_play_condition'},
    {'label':'DNA Digivolution','condition_type':'dna_digivolution'},
    {'condition_type':'unknown'},
])
def test_alias_limited_to_explicit_official_digivolution_section(changes):
    with pytest.raises(ConditionSpanError): split([unit('[DNA Digivolution] Cost 0',**changes)])


@pytest.mark.parametrize('units',[None,{},'text',[None],[1]])
def test_malformed_collections_refused(units):
    with pytest.raises(ConditionSpanError):split(units)
