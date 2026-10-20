"""Wrapper wiring tests. Real sealed-candidate acceptance is a separate gate."""
import json
import pytest
from test_runtime_snapshot import snapshot_inputs,enc,sha


def setup(tmp_path,monkeypatch,status='COMPOSED_RULE_CARD'):
    from tools.clause_coverage import runtime_rule_composition as m
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    snapshot=load_runtime_snapshot(**snapshot_inputs(tmp_path));record=snapshot.card('BT1-001')
    a=dict(snapshot=snapshot,expected_extraction_sha256=snapshot.metadata()['extraction_artifact_sha256'],
        runtime_record=record['runtime_record'],registry_bytes=record['registry_bytes'],runtime_binding=b'{}',
        image_artifacts={},canonical_card=record['canonical_card'],aggregate=snapshot.file('aggregate'),
        scopes={'official':snapshot.file('official_scope'),'digimoncard_io':snapshot.file('secondary_scope')},
        observations={},adjudication_ledger=snapshot.file('adjudications'),raw_pages={},
        correction_document=snapshot.file('corrections'),condition_selections=snapshot.file('selections'),
        old_input_revision='a'*64,new_input_revision='a'*64,expected_official_universe=['BT1-001'])
    calls=[]
    def proof(**kw):
        calls.append(('proof',kw))
        return {'derived_canonical_view':{'card_id':'BT1-001','card_kind':'Digimon'},
                'claimed_fields':list(range(19)),'unclaimed_runtime_fields':{'rarity':'R'},
                'field_transformations':{'card_kind':'generate._legacy_kind'}}
    def compose(**kw):
        calls.append(('source',kw));return {'status':status,'blockers':[]if status!='BLOCKED'else[{'code':'UNSUPPORTED'}]}
    monkeypatch.setattr(m,'prove_runtime_projection',proof)
    monkeypatch.setattr(m,'compose_rule_card',compose)
    return m,a,calls


def test_original_and_derived_views_separate(tmp_path,monkeypatch):
    m,a,calls=setup(tmp_path,monkeypatch);r=m.compose_runtime_rule_card(**a)
    assert [c[0]for c in calls]==['proof','source']
    assert calls[0][1]['canonical_card']==calls[1][1]['canonical_card']==a['canonical_card']
    assert calls[0][1]['runtime_record']==a['runtime_record']
    assert calls[1][1]['runtime_projection']!=a['runtime_record']
    assert r['original_runtime_bytes'].encode()==a['runtime_record']
    assert r['derived_canonical_view_sha256']==sha(calls[1][1]['runtime_projection'])
    assert len(r['claimed_fields'])==19 and r['unclaimed_runtime_fields']=={'rarity':'R'}
    assert 'ready'not in r and 'admitted'not in r


def test_source_blockers_never_convert_to_composed(tmp_path,monkeypatch):
    m,a,_=setup(tmp_path,monkeypatch,'BLOCKED');r=m.compose_runtime_rule_card(**a)
    assert r['status']=='BLOCKED' and r['source_composition']['blockers']==[{'code':'UNSUPPORTED'}]


@pytest.mark.parametrize('field',['canonical_card','runtime_record','registry_bytes','aggregate',
    'adjudication_ledger','correction_document','condition_selections','old_input_revision',
    'new_input_revision','expected_extraction_sha256','expected_official_universe','scopes',
    'runtime_projection','validated'])
def test_dispatch_inputs_must_match_verified_snapshot(tmp_path,monkeypatch,field):
    m,a,calls=setup(tmp_path,monkeypatch)
    if field=='expected_official_universe':a[field]=[]
    elif field=='scopes':a[field]={'official':b'bad','digimoncard_io':b'{}'}
    elif field.endswith('revision')or field=='expected_extraction_sha256':a[field]='b'*64
    else:a[field]=b'bad'
    with pytest.raises(ValueError):m.compose_runtime_rule_card(**a)
    assert not calls


def test_no_precomputed_proof_accepted(tmp_path,monkeypatch):
    m,a,_=setup(tmp_path,monkeypatch);a['runtime_proof']={'validated':True}
    with pytest.raises(ValueError):m.compose_runtime_rule_card(**a)


def test_complete_output_replay(tmp_path,monkeypatch):
    m,a,_=setup(tmp_path,monkeypatch);r=m.compose_runtime_rule_card(**a);r['status']='ready'
    with pytest.raises(ValueError):m.validate_runtime_rule_card(r,**a)


def test_source_exception_is_not_converted_to_success(tmp_path,monkeypatch):
    m,a,_=setup(tmp_path,monkeypatch)
    def refuse(**kw):raise ValueError('source refused')
    monkeypatch.setattr(m,'compose_rule_card',refuse)
    with pytest.raises(ValueError,match='source refused'):m.compose_runtime_rule_card(**a)


def test_runtime_refusal_never_dispatches_source(tmp_path,monkeypatch):
    m,a,calls=setup(tmp_path,monkeypatch)
    def refuse(**kw):raise ValueError('runtime refused')
    monkeypatch.setattr(m,'prove_runtime_projection',refuse)
    with pytest.raises(ValueError,match='runtime refused'):m.compose_runtime_rule_card(**a)
    assert not calls


def test_wrapper_dependency_closure_includes_public_validators():
    from tools.clause_coverage.runtime_rule_composition import wrapper_dependencies
    d=wrapper_dependencies()
    assert {'tools.clause_coverage.runtime_snapshot','tools.clause_coverage.runtime_projection_proof',
        'tools.clause_coverage.rule_card_composition','tools.clause_coverage.source_units',
        'tools.clause_coverage.source_binding','tools.card_data.generate','tools.card_data.registry',
        'tools.card_data.adjudications','tools.card_data.models'}<=set(d)
    assert all(len(h)==64 for h in d.values())
