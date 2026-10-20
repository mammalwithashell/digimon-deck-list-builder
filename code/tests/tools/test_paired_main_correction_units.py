"""Actual R4 original-byte correction proof; self-repins below are adversarial only."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc

F=Path(__file__).parents[1]/'fixtures/card_data/paired_main_correction'
sha=lambda b:hashlib.sha256(b).hexdigest()

def inputs():
    from tools.clause_coverage import paired_main_correction_units as mod
    m=json.loads((F/'inputs.json').read_bytes())
    def read(x):
        b=gzip.decompress((F/x['file']).read_bytes());assert sha(b)==x['sha256'];return b
    a={k:(read(v) if isinstance(v,dict) and 'file' in v else
          {n:read(d) for n,d in v.items()} if isinstance(v,dict) else v) for k,v in m['inputs'].items()}
    a['images']={n:(F/n).read_bytes() for n in m['images']}
    rows=[r for r in json.loads(a['correction_document'])['corrections'] if r['card_id']=='EX1-066']
    p={'schema_version':1,'kind':'reviewed-paired-main-image-v1','card_id':'EX1-066',
       'source_revision':a['new_input_revision'],'input_sha256':mod._pins(a),'dependency_sha256':mod._dependencies(),
       'corrections':{r['field_path']:{'correction_id':r['correction_id'],'sha256':sha(enc(r))} for r in rows},
       'evidence_observation_ids':rows[0]['evidence_observation_ids'],'images':[]}
    for n,printing in [('primary.png','EX1-066'),('primary-p1.png','EX1-066_P1')]:
        oid=next(o for o in p['evidence_observation_ids'] if json.loads(a['observations'][o])['extracted']['printing']['printing_id']==printing)
        p['images'].append({'artifact_name':n,'sha256':sha(a['images'][n]),'printing_id':printing,'official_observation_id':oid})
    a['sidecar']=enc(p);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar']);return a

def repin(a):
    from tools.clause_coverage import paired_main_correction_units as mod
    p=json.loads(a['sidecar']);p['input_sha256']=mod._pins(a)
    a['sidecar']=enc(p);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])

def bind(a):
    from tools.clause_coverage.paired_main_correction_units import bind_paired_main_correction_unit
    return bind_paired_main_correction_unit(**a)

def test_actual_fourteen_sources_two_rows_one_nonexecuted_value():
    a=inputs();before=copy.deepcopy(a);r=bind(a)
    assert a==before and r['status']=='PROOF_ONLY_BOUND' and r['whole_card_status']=='NOT_COMPOSED'
    assert len(r['source_observations'])==14 and len(r['original_sections'])==7
    assert len(r['corrections'])==2 and len(r['images'])==2
    assert r['unit']['value']==json.loads(a['canonical_card'])['faces'][0]['effect_text']
    assert r['face_mirror_obligation']['executable'] is False
    assert len(r['runtime_claims'])==15 and r['unclaimed_runtime_fields']
    assert r['unchanged_security'] and 'zones' not in r and 'clauses' not in r

@pytest.mark.parametrize('key',['aggregate','canonical_card','runtime_card','adjudication_ledger','correction_document'])
def test_equivalent_byte_changes_refuse(key):
    a=inputs();a[key]+=b'\n'
    with pytest.raises(ValueError):bind(a)

@pytest.mark.parametrize('change',['missing_observation','missing_raw','image','universe','revision','private_flag'])
def test_scope_and_external_authority_refuse(change):
    a=inputs()
    if change=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
    elif change=='missing_raw':a['raw_pages']={}
    elif change=='image':a['images']['primary.png']+=b'x'
    elif change=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
    elif change=='revision':a['old_input_revision']='0'*64
    else:a['validated']=True
    repin(a)
    with pytest.raises((ValueError,TypeError)):bind(a)

@pytest.mark.parametrize('change',['bool_schema','missing_pair','wrong_field','duplicate_image','wrong_printing','foreign_image','missing_images','dependency','row_hash','evidence'])
def test_self_rehashed_sidecar_refusals(change):
    a=inputs();p=json.loads(a['sidecar'])
    if change=='bool_schema':p['schema_version']=True
    elif change=='missing_pair':p['corrections'].pop('faces.0.effect_text')
    elif change=='wrong_field':p['corrections']['faces.1.effect_text']=p['corrections'].pop('faces.0.effect_text')
    elif change=='duplicate_image':p['images'][1]=p['images'][0]
    elif change=='wrong_printing':p['images'][0]['printing_id']='EX1-066_P2'
    elif change=='foreign_image':p['images'][0]['official_observation_id']='0'*64
    elif change=='missing_images':p.pop('images')
    elif change=='dependency':p['dependency_sha256']={}
    elif change=='row_hash':p['corrections']['effect_description_eng']['sha256']='0'*64
    else:p['evidence_observation_ids'].pop()
    a['sidecar']=enc(p);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
    with pytest.raises(ValueError):bind(a)

@pytest.mark.parametrize('change',['security','face_security','face_main','cost','provenance','runtime_security','runtime_cost','runtime_bool_cost','runtime_main','missing_row','third_row','duplicate_row','pair_divergence'])
def test_full_producer_replay_and_sibling_equality(change):
    a=inputs()
    if change.startswith('runtime_'):
        d=json.loads(a['runtime_card']);field={'runtime_security':'security_effect_description_eng','runtime_cost':'play_cost','runtime_bool_cost':'play_cost','runtime_main':'effect_description_eng'}[change]
        d[field]=True if change=='runtime_bool_cost' else 99 if change=='runtime_cost' else 'Changed';a['runtime_card']=enc(d)
    elif change in ('missing_row','third_row','duplicate_row','pair_divergence'):
        d=json.loads(a['correction_document']);rows=[r for r in d['corrections'] if r['card_id']=='EX1-066']
        if change=='missing_row':d['corrections'].remove(rows[1])
        elif change=='pair_divergence':rows[1]['value']+=' Draw 1.'
        else:
            x=copy.deepcopy(rows[0])
            if change=='third_row':x.update(correction_id='third',field_path='security_effect_description_eng')
            d['corrections'].append(x)
        a['correction_document']=enc(d)
    else:
        d=json.loads(a['canonical_card'])
        if change=='security':d['fields']['security_effect_description_eng']='Changed'
        elif change=='face_security':d['faces'][0]['security_text']='Changed'
        elif change=='face_main':d['faces'][0]['effect_text']='Changed'
        elif change=='cost':d['fields']['play_cost']=99
        else:d['provenance']['effect_description_eng']['authority']='official'
        a['canonical_card']=enc(d)
    repin(a)
    with pytest.raises(ValueError):bind(a)

def test_pair_divergence_refuses_even_when_row_digest_is_rebound():
    a=inputs();d=json.loads(a['correction_document']);r=next(r for r in d['corrections'] if r['card_id']=='EX1-066' and r['field_path']=='faces.0.effect_text')
    r['value']+=' Draw 1.';a['correction_document']=enc(d)
    p=json.loads(a['sidecar']);p['corrections'][r['field_path']]['sha256']=sha(enc(r));a['sidecar']=enc(p);repin(a)
    with pytest.raises(ValueError,match='paired values'):bind(a)

def test_source_content_identity_cannot_be_rebound_to_changed_containers():
    a=inputs();oid=next(o for o,b in a['observations'].items() if json.loads(b)['source']=='official')
    d=json.loads(a['observations'][oid]);d['extracted']['text_sections'].reverse();a['observations'][oid]=enc(d);repin(a)
    with pytest.raises(ValueError,match='content identity'):bind(a)

def test_runtime_newline_projection_is_exact_not_whitespace_normalized():
    a=inputs();d=json.loads(a['runtime_card']);assert '\r\n' in d['effect_description_eng']
    d['effect_description_eng']=d['effect_description_eng'].replace('\r\n','\n');a['runtime_card']=enc(d);repin(a)
    with pytest.raises(ValueError,match='runtime printed field'):bind(a)

def test_dependency_drift_during_proof_refuses(monkeypatch):
    from tools.clause_coverage import paired_main_correction_units as mod
    a=inputs();original=mod._dependencies;calls=[]
    def drift():
        d=original();calls.append(1)
        if len(calls)>1:d['changed']='0'*64
        return d
    monkeypatch.setattr(mod,'_dependencies',drift)
    with pytest.raises(ValueError,match='during proof'):bind(a)

def test_output_revalidation_refuses_authority_forgery():
    from tools.clause_coverage.paired_main_correction_units import validate_paired_main_correction_binding
    a=inputs();r=bind(a);r['unit']['authority']='official'
    with pytest.raises(ValueError,match='differs from exact replay'):validate_paired_main_correction_binding(r,**a)

@pytest.mark.parametrize('value',[None,[],True,'bad'])
def test_sidecar_wrong_envelope_is_clear_refusal(value):
    a=inputs();a['sidecar']=enc(value);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
    with pytest.raises(ValueError):bind(a)

@pytest.mark.parametrize('part',['face','provenance','printing','missing_default'])
def test_exact_raw_canonical_record_cannot_hide_ignored_fields(part):
    a=inputs();d=json.loads(a['canonical_card'])
    if part=='face':d['faces'][0]['unreviewed_metadata']='x'
    elif part=='provenance':d['provenance']['effect_description_eng']['unreviewed_metadata']='x'
    elif part=='printing':d['printings'][0]['unreviewed_metadata']='x'
    else:d['faces'][0].pop('inherited_text')
    a['canonical_card']=enc(d);repin(a)
    with pytest.raises(ValueError,match='complete canonical record'):bind(a)
