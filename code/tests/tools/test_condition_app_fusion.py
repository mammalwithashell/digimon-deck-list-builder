"""Pure App Fusion segmentation, not source approval or gameplay resolution."""
import copy
import json
from pathlib import Path

import pytest

from tools.clause_coverage.condition_spans import ConditionSpanError
from tools.clause_coverage.condition_dna_alias import split_dna_condition_units

FIXTURES = Path(__file__).parents[1] / 'fixtures/card_data/condition_app_fusion'


def split(units):
    from tools.clause_coverage.condition_app_fusion import split_app_fusion_condition_units
    return split_app_fusion_condition_units(units)


def unit(value, **changes):
    return {'kind':'official-section', 'field':'xros_req',
            'label':'Special Digivolution Condition',
            'condition_type':'special_digivolution_condition', 'text':value, **changes}


@pytest.mark.parametrize('cid', ['AD1-005','BT21-018','BT21-059','BT21-073'])
def test_actual_sources_preserve_distinct_app_fusion_and_whole_body(cid):
    f=json.loads((FIXTURES/f'{cid}.json').read_text(encoding='utf-8'))
    u=f['source_unit']; before=copy.deepcopy(u)
    assert f['observation']['extracted']['text_sections'][u['section_index']]['text']==u['text']
    assert split([u])==[{'unit_index':0,'source_unit':before,'spans':[{
        'span_index':0,'kind':'timing','timings':['App Fusion'],'keyword':None,
        'text':u['text'].removeprefix('[App Fusion]').lstrip()}]}]
    assert u==before


def test_marker_only_repeated_units_and_input_ownership():
    u=unit('[App Fusion]');result=split([u,u])
    assert [r['unit_index']for r in result]==[0,1]
    assert all(r['spans'][0]['text']=='' for r in result)
    result[0]['source_unit']['text']='changed'
    assert u['text']=='[App Fusion]' and result[1]['source_unit']['text']=='[App Fusion]'


@pytest.mark.parametrize('body', [
    '[A] & [B]: Cost 0',
    '[A] & [B] & [C] & [D] : Cost 12',
    '[A] & [B]: Cost 0\nIf 2 such cards are linked together, stack the link card on top and digivolve.',
    '[A] & [B]: Cost 0 (If 2 such cards are linked together, stack the link card on top and digivolve.)',
    '[A] & [B]: Cost 0 If 2 such cards are linked together, stack the link card on top and digivolve',
])
def test_supported_material_and_reminder_shapes_remain_opaque(body):
    assert split([unit('[App Fusion] '+body)])[0]['spans'][0]['text']==body


def test_mixed_approved_units_keep_occurrence_indices_and_whole_sources():
    units=[unit('[Digivolve] Cost 3'),unit('[App Fusion] [A] & [B]: Cost 0'),
           unit('[DNA Digivolution] Cost 0'),unit('DigiXros -2: [Material]',
             label='Special Play Condition',condition_type='special_play_condition')]
    before=copy.deepcopy(units);result=split(units)
    assert [r['unit_index']for r in result]==list(range(4))
    assert [r['source_unit']for r in result]==before and units==before
    for i in [0,2,3]:assert result[i]['spans']==split_dna_condition_units([units[i]])[0]['spans']
    assert split([])==[]


@pytest.mark.parametrize('text', [
    'Preamble [App Fusion] [A] & [B]: Cost 0',
    '[Main] [App Fusion] [A] & [B]: Cost 0',
    '[App Fusion] [A] & [B]: Cost 0\n[App Fusion]',
    '[App Fusion] [A] & [B]: Cost 0\n[Digivolve] Cost 1',
    '[App Fusion] [A] & [B]: Cost 0\n[Main] Draw 1.',
    '[App Fusion] [A] & [B]: Cost 0\nAssembly -2: [C]',
    '[App Fusion] [A] & [B]: Cost 0\n[Special Play Condition]',
    '[App Fusion] [A] & [B]: Cost 0 unknown trailing prose',
    '[App Fusion] [A] & [B]: Cost -1',
    '[App Fusion] [A] & [B]: Cost true',
    '[App Fusion] [A] & [B]: Cost 0.5',
    '[App Fusion] [A] & [B]',
    '[App Fusion] Cost 0',
    '[App Fusion] [A]: Cost 0',
    '[App Fusion] [Main] & [B]: Cost 0',
    '[App Fusion] [] & [B]: Cost 0',
    '[App Fusion] [Unknown Condition] & [B]: Cost 0',
    '[App Fusion] [main] & [B]: Cost 0',
    '[App Fusion] [<On Play>] & [B]: Cost 0',
    'App Fusion [A] & [B]: Cost 0',
])
def test_unsupported_unknown_nested_or_mixed_grammar_refused(text):
    with pytest.raises(ConditionSpanError):split([unit(text)])


@pytest.mark.parametrize('changes', [
    {'kind':'image-correction'}, {'field':'main'}, {'kind':'unknown'},
    {'label':'Special Play Condition','condition_type':'special_play_condition'},
    {'label':'Effect','condition_type':'effect'}, {'condition_type':'unknown'},
    {'label':None}, {'text':None}, {'text':''},
])
def test_metadata_boundaries(changes):
    with pytest.raises(ConditionSpanError):split([unit('[App Fusion] [A] & [B]: Cost 0',**changes)])


@pytest.mark.parametrize('units',[None,{},'text',[None],[1]])
def test_malformed_inputs(units):
    with pytest.raises(ConditionSpanError):split(units)
