"""Separate empty-producer capability; original A2 behavior stays unchanged."""
import gzip
import json
from pathlib import Path
import pytest

from tools.clause_coverage import runtime_projection_proof as p
from test_runtime_rule_actual_snapshot import snapshot,actual_inputs,EXTRACTION_SHA,BASE

F=BASE/'runtime_empty_condition'


def inputs(snapshot,cid):
    if cid in ('BT24-083','BT24-031'):return actual_inputs(snapshot,cid)
    a=actual_inputs(snapshot,'BT24-083');a.update(snapshot.card(cid))
    d=json.loads((F/'metadata.json').read_bytes())['cards'][cid]
    observations=json.loads(gzip.decompress((F/(cid+'-observations.json.gz')).read_bytes()))
    a['observations']={k:p.enc(v)for k,v in observations.items()}
    a['raw_pages']={u:gzip.decompress((F/name).read_bytes())for u,name in d['raw_files'].items()}
    repin(a);return a


def repin(a):
    keys=('canonical_card','runtime_record','registry_bytes','aggregate','observations',
          'correction_document','condition_selections','adjudication_ledger','raw_pages','image_artifacts',
          'old_input_revision','new_input_revision','expected_official_universe')
    a['runtime_binding']=p.enc({'schema_version':1,'input_sha256':p.input_hashes({k:a[k]for k in keys}),
                              'producer_sha256':p.producer_dependencies()})


@pytest.mark.parametrize('cid',['BT26-104','BT24-083'])
def test_actual_empty_runtime_requires_whole_source_composition(snapshot,cid):
    from tools.clause_coverage.runtime_empty_condition import compose_empty_condition_runtime_rule_card
    a=inputs(snapshot,cid);r=compose_empty_condition_runtime_rule_card(**a)
    assert r['status']=='EXACT_EMPTY_RUNTIME_SOURCE_COMPOSITION'
    assert r['original_runtime_bytes'].encode()==a['runtime_record']
    assert r['original_canonical_bytes'].encode()==a['canonical_card']
    assert r['source_composition']['status']=='COMPOSED_RULE_CARD'
    assert len(r['claimed_fields'])==19
    empty=r['empty_condition_evidence']
    assert empty['projection']=='' and not empty['a2_section_coverage_claimed']
    assert set(empty['candidate_values'])=={oid for oid,raw in a['observations'].items()if json.loads(raw)['source']=='official'}
    assert set(empty['candidate_values'].values())=={''}
    if cid=='BT26-104':
        assert len(r['source_composition']['retained_alternative_structures'])==2
        assert sum(s['label']=='Link Condition' for o in empty['official_sections'].values()for s in o)==2


@pytest.mark.parametrize('cid',['BT24-031','EX12-031'])
def test_first_batch_nonempty_and_correction_controls_refused(snapshot,cid):
    from tools.clause_coverage.runtime_empty_condition import compose_empty_condition_runtime_rule_card
    with pytest.raises(ValueError):compose_empty_condition_runtime_rule_card(**inputs(snapshot,cid))


def test_old_public_a2_wrapper_still_refuses_alternate_link_label(snapshot):
    from tools.clause_coverage.runtime_rule_composition import compose_runtime_rule_card
    with pytest.raises(ValueError,match='unsupported condition source section'):
        compose_runtime_rule_card(**inputs(snapshot,'BT26-104'))


@pytest.mark.parametrize('mutation',['missing_alternate','changed_section','nonempty_runtime','space_runtime',
    'source_revision','unknown_input','selection_wire','registry_bool','ledger_wire'])
def test_rebound_adversaries_do_not_become_empty_success(snapshot,mutation):
    from tools.clause_coverage.runtime_empty_condition import compose_empty_condition_runtime_rule_card
    a=inputs(snapshot,'BT26-104')
    if mutation=='missing_alternate':
        key=next(k for k,v in a['observations'].items()if json.loads(v)['source']=='official' and
                 any(s['label']=='Link Condition'for s in json.loads(v)['extracted']['text_sections']))
        a['observations'].pop(key)
    elif mutation=='changed_section':
        key=next(k for k,v in a['observations'].items()if json.loads(v)['source']=='official')
        d=json.loads(a['observations'][key]);d['extracted']['text_sections'].append({'label':'Special Play Condition','text':'[Assembly -2] invented'})
        a['observations'][key]=p.enc(d)
    elif mutation in ('nonempty_runtime','space_runtime'):
        d=json.loads(a['runtime_record']);d['xros_req']='route'if mutation=='nonempty_runtime'else' ';a['runtime_record']=p.enc(d)
    elif mutation=='source_revision':a['new_input_revision']='0'*64
    elif mutation=='unknown_input':a['validated']=True
    elif mutation=='selection_wire':
        d=json.loads(a['condition_selections']);d['schema_version']=True;a['condition_selections']=p.enc(d)
    elif mutation=='registry_bool':
        d=json.loads(a['registry_bytes']);d['padding_index']=False;a['registry_bytes']=p.enc(d)
    else:
        d=json.loads(a['adjudication_ledger']);d['adjudications'][0]['reason']=True;a['adjudication_ledger']=p.enc(d)
    repin(a)
    with pytest.raises(ValueError):compose_empty_condition_runtime_rule_card(**a)


@pytest.mark.parametrize('stale',[False,True])
def test_private_empty_predicate_rejects_target_selection_even_if_empty(snapshot,stale):
    from tools.clause_coverage.runtime_empty_condition import _empty_condition_evidence
    from tools.card_data.complete_candidate import _canonical_card_from_payload
    from tools.card_data.serialization import content_digest
    a=inputs(snapshot,'BT26-104');d=json.loads(a['canonical_card'])
    card=_canonical_card_from_payload('BT26-104',d,path=Path('<test>'))
    ids=sorted(k for k,v in a['observations'].items()if json.loads(v)['source']=='official')
    values={oid:''for oid in ids};ledger=json.loads(a['condition_selections'])
    ledger['selections'].append({'schema_version':1,'selection_id':'test-only-empty-selection',
        'selection_kind':'official_condition_projection','card_id':'BT26-104','field_path':'xros_req',
        'source_manifest_revision':'0'*64 if stale else a['new_input_revision'],
        'candidate_values':values,'candidate_set_digest':content_digest(values),'selected_observation_id':ids[0],
        'selected_value':'','selected_value_digest':content_digest(''),'reviewer':'test','reason':'test',
        'reviewed_at':'2026-10-12T00:00:00Z'})
    a['condition_selections']=p.enc(ledger)
    with pytest.raises(ValueError,match='target selections'):_empty_condition_evidence(card,a)


def test_mandatory_whole_source_blocker_retained(snapshot,monkeypatch):
    from tools.clause_coverage import runtime_empty_condition as m
    a=inputs(snapshot,'BT26-104');calls=[]
    def block(**kw):
        calls.append(kw)
        return {'status':'BLOCKED','blockers':[{'code':'UNREVIEWED_ALTERNATE'}],'inventory':['retained']}
    monkeypatch.setattr(m,'compose_rule_card',block)
    r=m.compose_empty_condition_runtime_rule_card(**a)
    assert r['status']=='BLOCKED' and r['source_composition']['inventory']==['retained']
    assert calls[0]['observations']==a['observations']
    assert calls[0]['canonical_card']==a['canonical_card']


def test_empty_output_requires_complete_replay(snapshot):
    from tools.clause_coverage.runtime_empty_condition import compose_empty_condition_runtime_rule_card,validate_empty_condition_runtime_rule_card
    a=inputs(snapshot,'BT26-104');r=compose_empty_condition_runtime_rule_card(**a)
    r['empty_condition_evidence']['a2_section_coverage_claimed']=True
    with pytest.raises(ValueError):validate_empty_condition_runtime_rule_card(r,**a)


def test_empty_producer_alone_cannot_replace_security_adjudication(snapshot,tmp_path):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    from tools.clause_coverage.runtime_empty_condition import compose_empty_condition_runtime_rule_card
    artifact=snapshot.metadata();artifact.pop('extraction_artifact_sha256');artifact.pop('limit')
    ledger=json.loads(snapshot.file('adjudications'));before=len(ledger['adjudications'])
    ledger['adjudications']=[r for r in ledger['adjudications']if not(r['kind']=='canonical_field_selection'
        and r['card_id']=='BT26-104' and r.get('field_path')=='security_effect_description_eng')]
    assert len(ledger['adjudications'])<before
    # Deliberately rebound test-only extraction, never a real reviewed artifact.
    for role,entry in artifact['files'].items():
        raw=p.enc(ledger)if role=='adjudications'else snapshot.file(role)
        path=tmp_path/entry['path'];path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
        entry.update(sha256=p._sha(raw),bytes=len(raw))
    raw=p.enc(artifact);rebound=load_runtime_snapshot(candidate_root=tmp_path,
        extraction_artifact=raw,expected_extraction_sha256=p._sha(raw))
    a=inputs(rebound,'BT26-104');a['expected_extraction_sha256']=p._sha(raw);repin(a)
    with pytest.raises(ValueError,match='provenance|adjudicat'):
        compose_empty_condition_runtime_rule_card(**a)


def test_private_empty_predicate_refuses_actual_nonempty_condition(snapshot):
    from tools.clause_coverage.runtime_empty_condition import _empty_condition_evidence
    from tools.card_data.complete_candidate import _canonical_card_from_payload
    a=inputs(snapshot,'BT24-031');d=json.loads(a['canonical_card'])
    card=_canonical_card_from_payload('BT24-031',d,path=Path('<test>'))
    with pytest.raises(ValueError):_empty_condition_evidence(card,a)
