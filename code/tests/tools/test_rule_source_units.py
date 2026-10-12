"""Pure occurrence tests; no source authority or whole-card readiness."""
import copy
import json
from pathlib import Path

import pytest

F = Path(__file__).parents[1] / 'fixtures/card_data/rule_source_units'


def fixture(cid='BT25-025'):
    return json.loads((F / (cid + '.json')).read_text(encoding='utf8'))['observations'][0]


def derive(observation):
    from tools.clause_coverage.rule_source_units import derive_rule_occurrences
    return derive_rule_occurrences(observation)


@pytest.mark.parametrize('cid,category,value', [
    ('BT25-025','traits','Cyborg'), ('EX12-026','traits','Aquatic'),
    ('EX12-076','forms','Hybrid'), ('P-213','traits','Dragonkin'),
])
def test_actual_all_printings_exact_slices_and_full_source(cid,category,value):
    for o in json.loads((F/(cid+'.json')).read_text(encoding='utf8'))['observations']:
        before=copy.deepcopy(o);r=derive(o)
        assert o==before and r['source_observation']==o and r['source_observation'] is not o
        assert r['status']=='DERIVED_RULE_DIAGNOSTIC' and r['blockers']==[]
        assert len(r['occurrences'])==1
        x=r['occurrences'][0];text=o['extracted']['text_sections'][x['section_index']]['text']
        assert x['category']==category and x['value']==value
        assert x['whole_container']==text
        assert text[x['char_start']:x['char_end']]==x['line']
        assert text.encode()[x['utf8_byte_start']:x['utf8_byte_end']].decode()==x['line']
        assert value in r['projected_fields'][{'traits':'type_eng','forms':'form_eng'}[category]]
        assert 'ready' not in r and 'clauses' not in r and 'canonical' not in r


def replace_rule(o,line):
    e=o['extracted'];s=e['text_sections'][1]
    s['text']='\n'.join(s['text'].splitlines()[:-1]+[line])
    return o


@pytest.mark.parametrize('line',[
    '(Rule) Trait: Has [Cyborg] Type. Draw 1.',
    '(Rule) Trait: Has [Cyborg/Dragonkin] Type.',
    'During your turn (Rule) Trait: Has [Cyborg] Type.',
    '［Rule］ Trait: Has [Cyborg] Type.',
    '（Rule） Trait: Has [Cyborg] Type.',
    '＜Rule＞ Trait: Has [Cyborg] Type.',
    '(Rule) Trait: Has [Cyborg] Color.',
    '(Rule) Trait: Has [] Type.',
    '(Rule) You can include up to 50 copies of this card in your deck.',
])
def test_unknown_inline_unicode_trailing_and_non_grant_refused(line):
    r=derive(replace_rule(fixture(),line))
    assert r['status']=='BLOCKED_RULE_DIAGNOSTIC' and r['blockers']
    assert r['projected_fields'] is None
    assert r['source_observation']['extracted']['text_sections'][1]['text'].endswith(line)


@pytest.mark.parametrize('grants',[
    {'traits':[],'forms':[],'attributes':[]},
    {'traits':['Angel'],'forms':[],'attributes':[]},
    {'traits':['Cyborg','Cyborg'],'forms':[],'attributes':[]},
    {'traits':['Cyborg'],'forms':[]},
    {'traits':['Cyborg'],'forms':[],'attributes':[],'other':[]},
])
def test_grant_shape_missing_extra_or_wrong_cannot_pass(grants):
    o=fixture();o['extracted']['rule_grants']=grants
    assert derive(o)['status']=='BLOCKED_RULE_DIAGNOSTIC'


def test_repeated_occurrences_preserved_semantic_grant_deduplicated():
    o=fixture();e=o['extracted'];line=e['text_sections'][1]['text'].splitlines()[-1]
    e['text_sections'][1]['text']+='\n'+line
    r=derive(o);assert r['status']=='DERIVED_RULE_DIAGNOSTIC'
    assert [x['occurrence_index']for x in r['occurrences']]==[0,1]
    assert len({x['char_start']for x in r['occurrences']})==2
    assert r['source_grants']['traits']==['Cyborg']


def test_whole_ordinary_container_siblings_and_crlf_remain_unchanged():
    o=fixture();o['extracted']['text_sections'][1]['text']=o['extracted']['text_sections'][1]['text'].replace('\n','\r\n')
    before=copy.deepcopy(o);r=derive(o);assert r['source_observation']==before
    x=r['occurrences'][0];t=before['extracted']['text_sections'][1]['text']
    assert x['char_end']==len(t) and x['utf8_byte_end']==len(t.encode())


def test_attribute_derivation_is_synthetic_not_global_source_approval():
    o=replace_rule(fixture(),'(Rule) Trait: Has [Virus] attribute.')
    o['extracted']['rule_grants']={'traits':[],'forms':[],'attributes':['Virus']}
    r=derive(o);assert r['status']=='DERIVED_RULE_DIAGNOSTIC'
    assert r['occurrences'][0]['category']=='attributes'
    assert r['projected_fields']['attribute_eng']==['Vaccine','Virus']


def test_special_rule_without_word_rule_in_body_is_not_silently_empty():
    o=fixture();o['extracted']['text_sections']=[{'label':'DUAL Rule','text':'＜Arts Digivolve＞ (Instead of trashing after use...)'}]
    o['extracted']['rule_grants']={'traits':[],'forms':[],'attributes':[]}
    r=derive(o);assert r['status']=='BLOCKED_RULE_DIAGNOSTIC'


def test_no_rule_grants_is_explicit_diagnostic_only():
    o=fixture();e=o['extracted'];e['text_sections']=[];e['rule_grants']={'traits':[],'forms':[],'attributes':[]}
    r=derive(o);assert r['status']=='NO_RULE_GRANTS_DIAGNOSTIC' and not r['occurrences']


def test_nested_bracket_value_is_not_a_supported_single_trait():
    o=replace_rule(fixture(),'(Rule) Trait: Has [[Cyborg] Type.')
    o['extracted']['rule_grants']['traits']=['[Cyborg']
    assert derive(o)['status']=='BLOCKED_RULE_DIAGNOSTIC'


@pytest.mark.parametrize('value',[None,[],{}, {'extracted':None}])
def test_malformed_input_refused(value):
    from tools.clause_coverage.rule_source_units import RuleUnitError
    with pytest.raises(RuleUnitError):derive(value)

@pytest.mark.parametrize('location',['body','label'])
@pytest.mark.parametrize('marker',['SpecialRule','specialrule','\uff33\uff50\uff45\uff43\uff49\uff41\uff4c\uff32\uff55\uff4c\uff45'])
def test_concatenated_special_rule_refused_without_normalizing_source(location,marker):
    o=fixture();before=copy.deepcopy(o)
    if location=='body':o['extracted']['text_sections'][1]['text']+='\n['+marker+'] Draw 1.'
    else:o['extracted']['text_sections'].append({'label':marker,'text':'Draw 1.'})
    supplied=copy.deepcopy(o);r=derive(o)
    assert r['status']=='BLOCKED_RULE_DIAGNOSTIC' and r['projected_fields'] is None
    assert r['source_observation']==supplied and o==supplied
    assert r['occurrences'][0]['line'] in before['extracted']['text_sections'][1]['text']
