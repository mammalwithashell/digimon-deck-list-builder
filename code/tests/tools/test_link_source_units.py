"""Pure Link source segmentation; never whole-card admission."""
from copy import deepcopy
import importlib
import json
from pathlib import Path

import pytest

F = Path(__file__).parents[1] / 'fixtures/card_data/link_source_units'


def derive(value):
    return importlib.import_module('tools.clause_coverage.link_source_units').derive_link_occurrences(value)


def source(name='BT21-071'):
    return json.loads((F / (name + '.json')).read_bytes())


@pytest.mark.parametrize('name,cost,dp', [('BT21-071',2,3000),('BT21-071_P1',2,3000),('BT21-074',3,4000)])
def test_actual_layouts(name,cost,dp):
    raw=source(name);before=deepcopy(raw);result=derive(raw)
    assert result['status']=='DERIVED_LINK_DIAGNOSTIC'
    assert result['blockers']==[] and raw==before
    assert result['section_inventory']==raw['extracted']['text_sections']
    assert [o['role'] for o in result['occurrences']].count('condition')==1
    assert next(o for o in result['occurrences'] if o['role']=='condition')['value']=={'trait':'Appmon','cost':cost}
    assert next(o for o in result['occurrences'] if o['role']=='dp')['value']==dp
    for o in result['occurrences']:
        text=raw['extracted']['text_sections'][o['section_index']]['text']
        assert text[o['char_start']:o['char_end']]==o['text']
        assert text.encode()[o['utf8_start']:o['utf8_end']].decode()==o['text']
    assert derive(raw)==result


def test_repeated_combined_sections_and_empty_ordinary_preserved():
    raw=source();raw['extracted']['text_sections'].append(deepcopy(raw['extracted']['text_sections'][-1]))
    raw['extracted']['text_sections'].append({'label':'Effect','text':''})
    result=derive(raw)
    assert result['status']=='DERIVED_LINK_DIAGNOSTIC'
    assert len(result['section_inventory'])==6
    assert [o['role'] for o in result['occurrences']].count('effect')==2


def test_crlf_offsets_preserve_source():
    raw=source();raw['extracted']['text_sections'][-1]['text']=raw['extracted']['text_sections'][-1]['text'].replace('\n','\r\n')
    result=derive(raw);assert result['status']=='DERIVED_LINK_DIAGNOSTIC'
    effect=next(o for o in result['occurrences'] if o['role']=='effect')
    assert effect['text'].startswith('[When Linking]')
    assert effect['utf8_start']>effect['char_start']


@pytest.mark.parametrize('mutation',[
    'missing_dp','duplicate_dp','empty_effect','empty_condition','mixed','unknown_label',
    'ascii_link','unknown_trait','altered_reminder','extra_timing','inline_link','extra_condition',
    'foreign_source','boolean_text','unknown_special_rule',
])
def test_refuses_partial_or_unsupported_layout(mutation):
    raw=source();s=raw['extracted']['text_sections']
    if mutation=='missing_dp':s.pop(2)
    elif mutation=='duplicate_dp':s.append(deepcopy(s[2]))
    elif mutation=='empty_effect':s[-1]['text']=s[-1]['text'].split('\n')[0]+'\n[When Linking]'
    elif mutation=='empty_condition':s[-1]['text']='\n'+s[-1]['text'].split('\n')[1]
    elif mutation=='mixed':s.append({'label':'Link DP','text':'DP+3000'})
    elif mutation=='unknown_label':s[-1]['label']='Other Link'
    elif mutation=='ascii_link':s[-1]['text']=s[-1]['text'].replace('\uff1cLink\uff1e','<Link>')
    elif mutation=='unknown_trait':s[-1]['text']=s[-1]['text'].replace('[Appmon]','[Unknown]')
    elif mutation=='altered_reminder':s[-1]['text']=s[-1]['text'].replace('sideways','upright')
    elif mutation=='extra_timing':s[-1]['text']+=' [On Play] Draw 1.'
    elif mutation=='inline_link':s[1]['text']+=' '+s[-1]['text']
    elif mutation=='extra_condition':s[-1]['text']+='\n'+s[-1]['text'].split('\n')[0]
    elif mutation=='foreign_source':raw['source']='digimoncard_io'
    elif mutation=='boolean_text':s[-1]['text']=True
    elif mutation=='unknown_special_rule':s[2]['text']+=' Has the [Wizard] type.'
    result=derive(raw)
    assert result['status']=='BLOCKED' and result['blockers']
    assert result['section_inventory']==s


def test_no_link_is_not_an_empty_success():
    raw=source();raw['extracted']['text_sections']=[]
    assert derive(raw)['status']=='NOT_APPLICABLE'


def test_result_copies_source():
    raw=source();result=derive(raw);result['section_inventory'][0]['text']='changed'
    assert raw['extracted']['text_sections'][0]['text']!='changed'


@pytest.mark.parametrize('suffix',[' \uff1cOn Play\uff1e Draw 1.',' <On Play> Draw 1.'])
def test_unknown_angle_timing_does_not_hide_in_effect(suffix):
    raw=source();raw['extracted']['text_sections'][-1]['text']+=suffix
    assert derive(raw)['status']=='BLOCKED'


def test_different_combined_conditions_are_ambiguous():
    raw=source();extra=deepcopy(raw['extracted']['text_sections'][-1])
    extra['text']=extra['text'].replace('Cost 2','Cost 3')
    raw['extracted']['text_sections'].append(extra)
    assert derive(raw)['status']=='BLOCKED'


@pytest.mark.parametrize('extracted',[None,[],True,'bad'])
def test_malformed_extracted_envelope_is_retained_and_blocked(extracted):
    raw=source();raw['extracted']=extracted
    result=derive(raw)
    assert result['status']=='BLOCKED' and result['blockers']
    assert result['observation']==raw and result['section_inventory'] is None
