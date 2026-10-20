"""Opt-in whole-card composition; regenerated sidecars below are test-only."""
import copy,gzip,json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from test_paired_main_correction_units import inputs as proof_inputs,sha
from tools.clause_coverage import runtime_projection_proof as rp
F=Path(__file__).parents[1]/'fixtures/card_data/paired_main_composition'

def inputs():
    c=proof_inputs()
    a={k:v for k,v in c.items() if k not in ('runtime_card','sidecar','expected_reviewed_sidecar_sha256','images')}
    a.update(runtime_record=c['runtime_card'],registry_bytes=gzip.decompress((F/'registry.json.gz').read_bytes()),
        condition_selections=gzip.decompress((F/'selections.json.gz').read_bytes()),
        image_artifacts={r['image_artifact_path']:(F/(r['image_sha256']+'.webp')).read_bytes()
            for r in json.loads(c['correction_document'])['compatibility_corrections']},
        correction_sidecar=c['sidecar'],expected_reviewed_sidecar_sha256=c['expected_reviewed_sidecar_sha256'],correction_images=c['images'])
    repin(a);return a

def repin(a):
    from tools.clause_coverage.paired_main_card_composition import checker_dependencies
    a.pop('projection_binding',None)
    a['projection_binding']=enc({'schema_version':1,'input_sha256':rp.input_hashes(a),
        'producer_sha256':rp.producer_dependencies(),'checker_sha256':checker_dependencies()})

def repin_correction(a):
    from tools.clause_coverage.paired_main_correction_units import _pins,_dependencies
    p=json.loads(a['correction_sidecar'])
    args={**a,'runtime_card':a['runtime_record']};p['input_sha256']=_pins(args);p['dependency_sha256']=_dependencies()
    rows=[r for r in json.loads(a['correction_document'])['corrections'] if r['card_id']=='EX1-066']
    p['corrections']={r['field_path']:{'correction_id':r['correction_id'],'sha256':sha(enc(r))}for r in rows}
    a['correction_sidecar']=enc(p);a['expected_reviewed_sidecar_sha256']=sha(a['correction_sidecar']);repin(a)

def compose(a):
    from tools.clause_coverage.paired_main_card_composition import compose_paired_main_card
    return compose_paired_main_card(**a)

def test_actual_pair_single_execution_and_all_source_containers_retained():
    a=inputs();before=copy.deepcopy(a);r=compose(a)
    assert a==before and r['status']=='COMPOSED_PAIRED_MAIN_CARD'
    assert len(r['correction_binding']['source_observations'])==14
    assert len(r['runtime_projection']['claimed_fields'])==19
    assert r['runtime_projection']['original_runtime_bytes']==a['runtime_record'].decode()
    zone=r['zones']['effect'];assert len(zone['unit_indices'])==1 and len(zone['spans'])==2
    u=r['units'][zone['unit_indices'][0]];assert u['authority']=='adjudicated'
    assert not {'observation_id','section_index','char_start','char_end'}&set(u)
    rows=[r for r in r['consumption'] if r['disposition']=='superseded-by-reviewed-correction']
    assert len(rows)==7 and all(not r['unit_indices'] and r['replacement_unit_indices']==zone['unit_indices']for r in rows)
    assert all('with a digivolution card' in r['text'] or 'digivolution' in r['text']for r in rows)
    assert len(r['consumption'])==len(r['inventory'])
    assert r['zones']['security']['value']==json.loads(a['canonical_card'])['fields']['security_effect_description_eng']
    assert all(r['units'][i].get('authority')!='adjudicated'for i in r['zones']['security']['unit_indices'])
    assert r['zones']['inherited']['value']==''
    assert len(r['rule_derivations'])==7
    assert any(o['kind']=='paired-main-face-consistency' and o['executable'] is False for o in r['structural_obligations'])
    assert 'ready'not in r and 'clauses'not in r

@pytest.mark.parametrize('name',['_face_validator','_runtime_validator','_correction_binding','validated','context','proof','runtime_projection'])
def test_public_wrapper_refuses_callback_or_prevalidated_injection(name):
    a=inputs();a[name]=True;repin(a)
    with pytest.raises((TypeError,ValueError)):compose(a)

@pytest.mark.parametrize('field,value',[('play_cost',99),('card_kind',True),('security_effect_description_eng','forged'),('index',-1),('norm_id',True),('xros_req','[Assembly -1]')])
def test_exact_runtime_nineteen_claims_refuse_even_reissued_test_pins(field,value):
    a=inputs();d=json.loads(a['runtime_record']);d[field]=value;a['runtime_record']=enc(d);repin_correction(a)
    with pytest.raises(ValueError):compose(a)

@pytest.mark.parametrize('field',['schema_version','registry_version','capacity','padding_index','assignment_count'])
def test_registry_headers_refuse_bool(field):
    a=inputs();d=json.loads(a['registry_bytes']);d[field]=True;a['registry_bytes']=enc(d);repin(a)
    with pytest.raises(ValueError):compose(a)

def test_old_approved_sidecar_is_not_silently_rebound_after_shared_epoch_changes():
    a=inputs();a['correction_sidecar']=(F/'sidecar-APPROVED.json').read_bytes();a['expected_reviewed_sidecar_sha256']=sha(a['correction_sidecar']);repin(a)
    with pytest.raises(ValueError,match='dependency'):compose(a)

def test_default_public_context_still_refuses_corrected_faces():
    from tools.clause_coverage.source_units import _validated_context
    a=inputs();keys={'aggregate','scopes','observations','canonical_card','adjudication_ledger','raw_pages','correction_document','condition_selections','old_input_revision','new_input_revision','expected_official_universe'}
    c=json.loads(a['canonical_card']);d={k:a[k]for k in keys};d['runtime_projection']=enc({'card_id':c['card_id'],**c['fields'],'xros_req':''})
    with pytest.raises(ValueError,match='faces'): _validated_context(**d)

def test_old_public_api_refuses_face_callback():
    from test_clause_source_units import inputs as ordinary_inputs
    from tools.clause_coverage.source_units import compose_ordinary_card
    with pytest.raises((ValueError,TypeError)):compose_ordinary_card(**ordinary_inputs(),_face_validator=lambda **kw:None)

def test_private_default_and_full_validator_leave_ordinary_output_identical():
    from test_clause_source_units import inputs as ordinary_inputs
    from tools.clause_coverage.source_units import _validated_context,_validated_context_core
    a=ordinary_inputs();seen=[]
    def validate(**kw):
        assert set(kw)=={'card','replayed_card','typed'}
        assert enc(kw['card'].faces)==enc(kw['replayed_card'].faces);seen.append(True)
    assert _validated_context(**a)==_validated_context_core(**a,_face_validator=validate)
    assert seen==[True]
    with pytest.raises(ValueError):_validated_context_core(**a,_face_validator=lambda **kw:True)

def test_rule_unknown_sibling_is_not_discharged(monkeypatch):
    from tools.clause_coverage import rule_card_composition as rules
    original=rules.derive_rule_occurrences
    def blocked(raw):
        r=original(raw);r['blockers'].append({'code':'UNSUPPORTED_RULE_TEST_WITNESS'});return r
    monkeypatch.setattr(rules,'derive_rule_occurrences',blocked)
    r=compose(inputs());assert r['status']=='BLOCKED' and r['blockers'] and not r['units']
    assert len(r['correction_binding']['original_sections'])==7
    assert len(r['consumption'])==len(r['inventory'])

@pytest.mark.parametrize('extra',[' [Rule] Also treated as Test.',' <Link> bogus',' [Assembly -1] bogus'])
def test_new_corrected_structural_text_cannot_be_treated_as_ordinary(extra):
    from tools.card_data.generate import _legacy_text
    a=inputs();d=json.loads(a['correction_document'])
    rows=[r for r in d['corrections']if r['card_id']=='EX1-066']
    for row in rows:row['value']+=extra
    a['correction_document']=enc(d);c=json.loads(a['canonical_card']);c['fields']['effect_description_eng']=rows[0]['value'];c['faces'][0]['effect_text']=rows[0]['value'];a['canonical_card']=enc(c)
    r=json.loads(a['runtime_record']);r['effect_description_eng']=_legacy_text(rows[0]['value']);a['runtime_record']=enc(r);repin_correction(a)
    result=compose(a);assert result['status']=='BLOCKED' and not result['units']
    assert any(b['code']=='UNSUPPORTED_CORRECTED_FIELD_STRUCTURE'for b in result['blockers'])

def test_result_revalidation_cannot_accept_second_execution():
    from tools.clause_coverage.paired_main_card_composition import validate_paired_main_card
    a=inputs();r=compose(a);r['units'].append(copy.deepcopy(r['units'][-1]))
    with pytest.raises(ValueError):validate_paired_main_card(r,**a)

@pytest.mark.parametrize('field,value',[('digixros_costs',[]),('gameplay_metadata',{'hidden':True})])
def test_unclaimed_gameplay_cannot_be_reported_as_composed(field,value):
    a=inputs();d=json.loads(a['runtime_record']);d[field]=value;a['runtime_record']=enc(d);repin_correction(a)
    with pytest.raises(ValueError,match='unclaimed gameplay'):compose(a)

def test_composition_dependency_drift_is_rejected(monkeypatch):
    from tools.clause_coverage import paired_main_card_composition as mod
    a=inputs();original=mod.checker_dependencies;calls=[]
    def drift():
        d=original();calls.append(True)
        if len(calls)>1:d['drift']='0'*64
        return d
    monkeypatch.setattr(mod,'checker_dependencies',drift)
    with pytest.raises(ValueError,match='drift'):compose(a)
