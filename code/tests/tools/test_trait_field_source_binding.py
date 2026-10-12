"""Actual ST1-02 trait binding and explicitly synthetic schema controls."""
import copy,gzip,json
from pathlib import Path
import pytest
from tools.card_data import models,normalization,adjudications,serialization
from tools.clause_coverage.source_binding import bind_canonical_field,SourceBindingError
F=Path(__file__).parents[1]/'fixtures/card_data/trait_field_source_binding'
enc=serialization.canonical_json_bytes

def typed(o):return models.SourceObservation(**{**o,'source':models.SourceName(o['source'])})
def fixture(field='attribute_eng'):
    aggregate=gzip.decompress((F/'aggregate.json.gz').read_bytes())
    ledger=gzip.decompress((F/'reviewed-ledger.json.gz').read_bytes())
    observations={k:v.encode('utf8')for k,v in json.loads((F/'observations.json').read_bytes()).items()}
    revision=json.loads(aggregate)['manifest_revision']
    if field!='attribute_eng':
        # Synthetic field-shape tests, never emitted approvals or source evidence.
        a=json.loads(aggregate);oldnew={};changed={}
        for oid,raw in observations.items():
            o=json.loads(raw);e=o['extracted']
            if o['source']=='official':e[field[:-4]]=e['attribute']
            else:e['canonical_fields'][field]=e['canonical_fields']['attribute_eng']
            new=serialization.observation_digest(typed(o));o['observation_id']=new;oldnew[oid]=new;changed[new]=enc(o)
        observations=changed
        for lane in a['sources'].values():
            lane['observation_ids']=[oldnew.get(k,k)for k in lane['observation_ids']]
            for ref in lane['observation_refs']:ref['observation_id']=oldnew.get(ref['observation_id'],ref['observation_id'])
        a['membership']['observation_ids']=[oldnew.get(k,k)for k in a['membership']['observation_ids']]
        a.pop('manifest_revision');revision=serialization.content_digest(a,exclude_operational=True);a['manifest_revision']=revision;aggregate=enc(a)
        rows=json.loads(ledger)['adjudications'];row=copy.deepcopy(next(r for r in rows if r['card_id']=='ST1-02'and r.get('field_path')=='attribute_eng'))
        candidates=[normalization.observation_candidate(typed(json.loads(b)))for b in observations.values()]
        selected=oldnew[row['selected_observation_id']]
        binding=normalization._canonical_conflict_binding(card_id='ST1-02',field_path=field,candidates=candidates,selected_value=['Vaccine'],selected_observation_id=selected,old_input_revision=revision,new_input_revision=revision)
        row.update(field_path=field,selected_observation_id=selected,old_input_revision=revision,new_input_revision=revision,old_digest=binding.candidate_digest,new_digest=binding.selected_value_digest,old_value=serialization.canonicalize(binding.candidate_values),new_value=['Vaccine'],provenance_observation_ids=list(binding.candidate_observation_ids))
        ledger=enc({'schema_version':1,'adjudications':[row]})
    rows=tuple(adjudications._row(r,path=Path('<fixture>'))for r in json.loads(ledger)['adjudications'])
    candidates=[normalization.observation_candidate(typed(json.loads(b)))for b in observations.values()]
    value,provenance,_=normalization._select_field('ST1-02',field,candidates,admission=models.AdmissionStatus.ADMITTED,adjudications=rows,field_selection_input_revision=revision)
    return dict(aggregate=aggregate,observations=observations,canonical_field=enc({'card_id':'ST1-02','field_path':field,'value':value,'provenance':serialization.canonicalize(provenance)}),adjudication_ledger=ledger,old_input_revision=revision,new_input_revision=revision)

@pytest.mark.parametrize('field',['attribute_eng','form_eng','type_eng'])
def test_trait_list_full_producer_replay(field):
    a=fixture(field);before=copy.deepcopy(a);r=bind_canonical_field(**a)
    assert r['selected_value']==['Vaccine'] and r['field_path']==field
    assert {oid for row in r['candidate_values'] for oid in row['observation_ids']} == set(a['observations'])
    assert len(a['observations']) == 8 and len(r['candidate_values']) == 2 and a == before

@pytest.mark.parametrize('value',[True,'Vaccine',None,[True],[1],{},['Vaccine','Vaccine'],['Bird'],[]])
@pytest.mark.parametrize('field',['attribute_eng','form_eng','type_eng'])
def test_trait_value_shape_or_replay_refuses(field,value):
    a=fixture(field);f=json.loads(a['canonical_field']);f['value']=value;a['canonical_field']=enc(f)
    with pytest.raises(SourceBindingError):bind_canonical_field(**a)

@pytest.mark.parametrize('change',['field','missing','foreign','provenance','stale','ledger_bool','ledger_missing','candidate_identity'])
def test_actual_membership_and_approval_refusals(change):
    a=fixture();f=json.loads(a['canonical_field'])
    if change=='field':f['field_path']='evo_costs'
    elif change=='missing':a['observations'].pop(next(iter(a['observations'])))
    elif change=='foreign':f['card_id']='ST1-03'
    elif change=='provenance':f['provenance']['selected_observation_id']='0'*64
    elif change=='stale':a['new_input_revision']='0'*64
    elif change=='candidate_identity':
        oid=next(iter(a['observations']));o=json.loads(a['observations'][oid]);o['extracted']['attribute']='Virus';a['observations'][oid]=enc(o)
    else:
        d=json.loads(a['adjudication_ledger'])
        if change=='ledger_bool':d['schema_version']=True
        else:d['adjudications']=[r for r in d['adjudications']if r['card_id']!='ST1-02']
        a['adjudication_ledger']=enc(d)
    a['canonical_field']=enc(f)
    with pytest.raises(SourceBindingError):bind_canonical_field(**a)
