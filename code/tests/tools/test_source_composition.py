"""Whole-card composition over complete sealed inputs, never source approvals."""
import copy
import gzip
import json
from pathlib import Path

import pytest

from tools.card_data import adjudications, normalization, serialization
from tools.card_data.models import SourceObservation, SourceName, AdmissionStatus
from tools.clause_coverage.source_units import SourceUnitError, compose_ordinary_card
from test_clause_source_units import inputs as ordinary_inputs

F = Path(__file__).parents[1] / 'fixtures/card_data/source_composition'
BASE = F.parent / 'source_binding/conditions'
enc = serialization.canonical_json_bytes


def inputs(cid='EX12-046'):
    args=ordinary_inputs(); d=json.loads((F/f'{cid}.json').read_text(encoding='utf-8'))
    obs=d['observations'];card=copy.deepcopy(d['card'])
    candidates=[normalization.observation_candidate(SourceObservation(**{**o,'source':SourceName(o['source'])}))for o in obs.values()]
    ledger=tuple(adjudications._row(r,path=Path('<reviewed-fixture>'))for r in json.loads(args['adjudication_ledger'])['adjudications'])
    for field in ['card_kind','effect_description_eng','inherited_effect_description_eng','security_effect_description_eng']:
        value,prov,_=normalization._select_field(cid,field,[c for c in candidates if field in c.fields],admission=AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=d['source_revision'])
        card['fields'][field]=value;card['provenance'][field]=serialization.canonicalize(prov)
    runtime={'card_id':cid,**{f:card['fields'][f]for f in ['card_kind','effect_description_eng','inherited_effect_description_eng','security_effect_description_eng']},'xros_req':d['xros_req']}
    corrections=json.loads(args['correction_document'])
    images={r['image_artifact_path']:(BASE/(r['image_sha256']+'.webp')).read_bytes()for r in corrections['compatibility_corrections']}
    return {**args,'canonical_card':enc(card),'runtime_projection':enc(runtime),'observations':{oid:enc(o)for oid,o in obs.items()},'raw_pages':{u:gzip.decompress((F/n).read_bytes())for u,n in d['raw_files'].items()},'image_artifacts':images}


def compose(**args):
    from tools.clause_coverage.source_composition import compose_condition_card
    return compose_condition_card(**args)


@pytest.mark.parametrize('cid',['BT24-031','BT24-101','EX12-046','EX12-063'])
def test_actual_whole_card_ordinary_and_condition_boxes(cid):
    args=inputs(cid);before=copy.deepcopy(args);r=compose(**args)
    assert r['status']=='COMPOSED_CARD' and not r['blockers'] and args==before
    assert set(r['zones'])=={'effect','inherited','security','conditions'}
    assert r['zones']['conditions']['value']==json.loads(args['runtime_projection'])['xros_req']
    assert r['condition_binding']['projection']==r['zones']['conditions']['value']
    assert len(r['consumption'])==len(r['inventory'])
    assert all(x['disposition']in ('consumed','retained-alternative')for x in r['consumption'])
    assert len({(x['observation_id'],x['section_index'])for x in r['consumption']})==len(r['inventory'])
    assert r['zones']['security']['presence']=='absent-box'


def test_special_play_assembly_separate_from_digivolve():
    r=compose(**inputs('EX12-063'));s=r['zones']['conditions']['spans']
    assert [x['timings']for x in s]==[['Digivolve'],['Assembly']]
    assert 'Non-white' in s[0]['text'] and 'Puppet' in s[1]['text']
    assert all('Special Play Condition' not in x['text']for x in s)


def test_two_alternatives_preserved_inside_one_source_unit():
    r=compose(**inputs('BT24-101'));s=r['zones']['conditions']['spans']
    assert len(s)==2 and s[0]['unit_index']==s[1]['unit_index']
    assert [x['span_index']for x in s]==[0,1]
    assert 'TS' in s[0]['text'] and 'Aegiochusmon' in s[1]['text']


def test_ordinary_api_remains_blocked_with_original_inputs():
    args=inputs();args.pop('image_artifacts');r=compose_ordinary_card(**args)
    assert r['status']=='BLOCKED' and not r['units'] and not r['zones']


@pytest.mark.parametrize('mutation',['runtime_xros','missing_member','foreign_raw','missing_image','scope','kind','missing_xros','validated'])
def test_complete_input_boundaries(mutation):
    args=inputs()
    if mutation=='runtime_xros':
        r=json.loads(args['runtime_projection']);r['xros_req']='';args['runtime_projection']=enc(r)
    elif mutation=='missing_member':args['observations'].pop(next(iter(args['observations'])))
    elif mutation=='foreign_raw':args['raw_pages'][next(iter(args['raw_pages']))]=b'changed'
    elif mutation=='missing_image':args['image_artifacts']={}
    elif mutation=='scope':args['expected_official_universe']=args['expected_official_universe'][:-1]
    elif mutation=='kind':
        r=json.loads(args['runtime_projection']);r['card_kind']='Option';args['runtime_projection']=enc(r)
    elif mutation=='missing_xros':
        r=json.loads(args['runtime_projection']);r.pop('xros_req');args['runtime_projection']=enc(r)
    elif mutation=='validated':args['validated']=True
    with pytest.raises(SourceUnitError):compose(**args)


@pytest.mark.parametrize('field,value',[('dual',{'card_id':'foreign'}),('link_effect_description_eng','Draw 1.')])
def test_other_structures_remain_whole_card_blockers(field,value):
    args=inputs();r=json.loads(args['runtime_projection']);r[field]=value;args['runtime_projection']=enc(r)
    result=compose(**args)
    assert result['status']=='BLOCKED' and result['blockers']
    assert not result['zones'] and not result['units'] and result['inventory']


@pytest.mark.parametrize('mutation',['missing_unit','duplicate_unit','foreign_index','changed_text'])
def test_dispatch_conservation_refuses_incomplete_or_foreign_binding(monkeypatch,mutation):
    import tools.clause_coverage.source_composition as module
    real=module.bind_condition_units
    def faulty(**args):
        b=real(**args)
        if mutation=='missing_unit':b['units'].pop()
        if mutation=='duplicate_unit':b['units'].append(copy.deepcopy(b['units'][0]))
        if mutation=='foreign_index':b['units'][0]['section_index']=999
        if mutation=='changed_text':b['units'][0]['text']='[Digivolve] Cost 9'
        return b
    monkeypatch.setattr(module,'bind_condition_units',faulty)
    with pytest.raises(SourceUnitError):compose(**inputs())


@pytest.mark.parametrize('mutation',['condition_span','empty_zone','inventory','disposition'])
def test_submitted_composition_must_replay_all_inputs(mutation):
    from tools.clause_coverage.source_composition import validate_condition_card
    args=inputs();r=compose(**args)
    if mutation=='condition_span':r['zones']['conditions']['spans'].pop()
    if mutation=='empty_zone':r['zones'].pop('security')
    if mutation=='inventory':r['inventory'].pop()
    if mutation=='disposition':r['consumption'][0]['disposition']='ignored'
    with pytest.raises(SourceUnitError):validate_condition_card(r,**args)


def test_unsupported_condition_grammar_keeps_inventory_and_whole_card_blocker(monkeypatch):
    import tools.clause_coverage.source_composition as module
    from tools.clause_coverage.condition_spans import ConditionSpanError
    def unsupported(units):
        raise ConditionSpanError('unreviewed source grammar')
    monkeypatch.setattr(module,'split_app_fusion_condition_units',unsupported)
    result=compose(**inputs())
    assert result['status']=='BLOCKED' and not result['units'] and not result['zones']
    assert any(b['code']=='UNSUPPORTED_CONDITION_GRAMMAR'for b in result['blockers'])
    assert len(result['consumption'])==len(result['inventory'])
    assert all(r['disposition']=='blocked'for r in result['consumption'])


def test_unsupported_cards_account_every_retained_occurrence_as_blocked():
    args=inputs();r=json.loads(args['runtime_projection']);r['dual']={'unsupported':True}
    args['runtime_projection']=enc(r);result=compose(**args)
    assert result['inventory'] and len(result['consumption'])==len(result['inventory'])
    assert all(r['disposition']=='blocked'for r in result['consumption'])


@pytest.mark.parametrize('cid',['BT1-010','BT1-045','BT4-104','BT26-104'])
def test_ordinary_and_empty_cards_preserve_existing_zones_and_units(cid):
    args=ordinary_inputs(cid);old=compose_ordinary_card(**args)
    runtime=json.loads(args['runtime_projection']);runtime['xros_req']=''
    args['runtime_projection']=enc(runtime)
    result=compose(image_artifacts={},**args)
    assert result['status']=='COMPOSED_CARD'
    assert result['units']==old['units']
    assert {k:v for k,v in result['zones'].items()if k!='conditions'}==old['zones']
    assert result['zones']['conditions']['presence']=='absent-box'


def test_internal_dispatch_cannot_delegate_ordinary_box():
    from tools.clause_coverage.source_units import _ordinary_sections
    with pytest.raises(SourceUnitError):
        _ordinary_sections([{'label':'Effect','text':'[On Play] Draw 1.'}],
                           'Digimon',delegated_indices={0})
