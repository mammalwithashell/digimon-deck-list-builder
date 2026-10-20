"""Proof-only correction binding: actual immutable input bytes and adversarial pins."""
import copy,gzip,hashlib,json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from test_clause_source_units import inputs as ordinary_inputs
F=Path(__file__).parents[1]/'fixtures/card_data/canonical_correction_units'
FIELD='inherited_effect_description_eng'
sha=lambda b:hashlib.sha256(b).hexdigest()

def input_pins(a):
 return {k:({x:sha(y) for x,y in sorted(v.items())} if isinstance(v,dict) else sha(v)) for k,v in a.items() if k in ('aggregate','scopes','observations','canonical_card','runtime_card','adjudication_ledger','correction_document','raw_pages')}

def inputs():
 old=ordinary_inputs();d=json.loads((F/'observations.json').read_bytes());image=next(F.glob('*.webp'))
 a={k:old[k] for k in ['aggregate','scopes','adjudication_ledger','correction_document','old_input_revision','new_input_revision','expected_official_universe']}
 a.update(observations={k:enc(v) for k,v in d['observations'].items()},canonical_card=(F/'canonical_card.json').read_bytes(),runtime_card=(F/'runtime_card.json').read_bytes(),raw_pages={u:gzip.decompress((F/n).read_bytes()) for u,n in d['raw_files'].items()},image_bytes=image.read_bytes())
 corr=next(r for r in json.loads(a['correction_document'])['corrections'] if r['card_id']=='EX12-031')
 from tools.card_data import models,normalization,serialization,source_manifest,complete_candidate,corrections,generate,adjudications,compatibility_corrections,compatibility_selections,errors
 from tools.clause_coverage import source_units,canonical_correction_units
 modules=(models,normalization,serialization,source_manifest,complete_candidate,corrections,generate,adjudications,compatibility_corrections,compatibility_selections,errors,source_units,canonical_correction_units)
 dependency_pins={m.__name__:sha(Path(m.__file__).read_bytes()) for m in modules}
 side={'dependency_sha256':dependency_pins,'schema_version':1,'kind':'reviewed-inherited-field-image-v1','card_id':'EX12-031','field_path':FIELD,'correction_id':corr['correction_id'],'source_revision':a['new_input_revision'],'input_sha256':input_pins(a),'correction_row_sha256':sha(enc(corr)),'evidence_observation_ids':corr['evidence_observation_ids'],'image':{'artifact_name':image.name,'sha256':sha(a['image_bytes']),'printing_id':'EX12-031','official_observation_id':'88940a47a03d7bb14e5cecd366f9c8fa48b8d0b308522b34da64eae9d684819d'}}
 a['sidecar']=enc(side);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar']);return a

def repin(a):
 d=json.loads(a['sidecar']);d['input_sha256']=input_pins(a);a['sidecar']=enc(d);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])

def bind(**a):
 from tools.clause_coverage.canonical_correction_units import bind_canonical_correction_unit
 return bind_canonical_correction_unit(**a)

def test_actual_sealed_field_and_image_proof_preserve_all_evidence():
 a=inputs();before=copy.deepcopy(a);r=bind(**a)
 assert a==before and r['status']=='PROOF_ONLY_BOUND'
 assert r['unit']['kind']=='reviewed-canonical-image-correction'
 assert r['unit']['authority']=='adjudicated' and r['unit']['value']==json.loads(a['canonical_card'])['fields'][FIELD]
 assert r['unit']['runtime_value']==json.loads(a['runtime_card'])[FIELD]
 assert len(r['source_observations'])==4 and len(r['original_sections'])==2
 assert all('from its digivolution cards' not in s['text'] for s in r['original_sections'])
 assert 'from its digivolution cards' in r['unit']['value']
 assert r['whole_card_status']=='NOT_COMPOSED' and r['remaining_integration_gate']
 assert r['input_sha256']==input_pins(a) and r['unit']['image']==json.loads(a['sidecar'])['image']
 assert 'zones' not in r and 'clauses' not in r

@pytest.mark.parametrize('key',['canonical_card','runtime_card','aggregate','adjudication_ledger','correction_document'])
def test_byte_pins_refuse_changed_even_equivalent_json(key):
 a=inputs();a[key]+=b'\n'
 with pytest.raises(ValueError):bind(**a)

@pytest.mark.parametrize('mutation',['wrong_pin','image','missing_observation','missing_raw','universe','stale','validated'])
def test_external_pin_and_complete_scope_refusals(mutation):
 a=inputs()
 if mutation=='wrong_pin':a['expected_reviewed_sidecar_sha256']='0'*64
 elif mutation=='image':a['image_bytes']+=b'x'
 elif mutation=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
 elif mutation=='missing_raw':a['raw_pages']={}
 elif mutation=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
 elif mutation=='stale':a['old_input_revision']='0'*64
 else:a['validated']=True
 if mutation not in ('wrong_pin','validated'):repin(a)
 with pytest.raises((ValueError,TypeError)):bind(**a)

@pytest.mark.parametrize('mutation',['kind','field','card','bool_schema','row_digest','evidence_missing','evidence_duplicate','foreign_primary','wrong_printing','reason_only','extra_key'])
def test_sidecar_semantics_cannot_be_self_rehashed(mutation):
 a=inputs();d=json.loads(a['sidecar'])
 if mutation=='kind':d['kind']='xros-image'
 elif mutation=='field':d['field_path']='effect_description_eng'
 elif mutation=='card':d['card_id']='EX12-026'
 elif mutation=='bool_schema':d['schema_version']=True
 elif mutation=='row_digest':d['correction_row_sha256']='0'*64
 elif mutation=='evidence_missing':d['evidence_observation_ids'].pop()
 elif mutation=='evidence_duplicate':d['evidence_observation_ids'].append(d['evidence_observation_ids'][0])
 elif mutation=='foreign_primary':d['image']['official_observation_id']='0'*64
 elif mutation=='wrong_printing':d['image']['printing_id']='EX12-031_P1'
 elif mutation=='reason_only':d.pop('image')
 else:d['approved']=True
 a['sidecar']=enc(d);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
 with pytest.raises(ValueError):bind(**a)

@pytest.mark.parametrize('mutation',['field','provenance','canonical_other_field','face','runtime_field','runtime_identity','source_scope','raw','correction_history'])
def test_exact_replay_refuses_forged_inputs_even_with_fresh_pins(mutation):
 a=inputs()
 if mutation in ('field','provenance','canonical_other_field','face'):
  d=json.loads(a['canonical_card'])
  if mutation=='field':d['fields'][FIELD]+=' Draw 1.'
  elif mutation=='provenance':d['provenance'][FIELD]['authority']='official'
  elif mutation=='canonical_other_field':d['fields']['level']=99
  else:d['faces'][0]['inherited_text']='Invented'
  a['canonical_card']=enc(d)
 elif mutation in ('runtime_field','runtime_identity'):
  d=json.loads(a['runtime_card']);d[FIELD if mutation=='runtime_field' else 'card_id']='Invented';a['runtime_card']=enc(d)
 elif mutation=='source_scope':
  d=json.loads(a['scopes']['official']);d['scope']='card:EX12-031';a['scopes']['official']=enc(d)
 elif mutation=='raw':a['raw_pages']={u:b+b'x' for u,b in a['raw_pages'].items()}
 else:
  d=json.loads(a['correction_document']);d['corrections'].append(copy.deepcopy(d['corrections'][0]));a['correction_document']=enc(d)
 repin(a)
 with pytest.raises(ValueError):bind(**a)

def test_output_is_recomputed_not_trusted():
 from tools.clause_coverage.canonical_correction_units import validate_correction_binding
 a=inputs();r=bind(**a);r['unit']['authority']='official'
 with pytest.raises(ValueError):validate_correction_binding(r,**a)

def test_proof_without_checker_dependency_pins_is_refused():
 a=inputs();d=json.loads(a['sidecar']);d.pop('dependency_sha256',None);a['sidecar']=enc(d);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
 with pytest.raises(ValueError):bind(**a)

def test_forged_dependency_rehash_cannot_change_producer():
 a=inputs();d=json.loads(a['sidecar']);d['dependency_sha256']['tools.card_data.generate']='0'*64;a['sidecar']=enc(d);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
 with pytest.raises(ValueError):bind(**a)


def test_actual_producer_lf_to_crlf_is_explicit_and_other_whitespace_refused():
 # Synthetic reviewed correction controls exercise the versioned producer path;
 # they are not approval of changed actual image/card data.
 from tools.card_data import generate
 a=inputs();doc=json.loads(a['correction_document']);row=next(r for r in doc['corrections'] if r['card_id']=='EX12-031');row['value']+='\nSynthetic reviewed line.'
 c=json.loads(a['canonical_card']);c['fields'][FIELD]=row['value'];r=json.loads(a['runtime_card']);r[FIELD]=generate._legacy_text(row['value'])
 a.update(canonical_card=enc(c),runtime_card=enc(r),correction_document=enc(doc));p=json.loads(a['sidecar']);p['correction_row_sha256']=sha(enc(row));a['sidecar']=enc(p);repin(a)
 bound=bind(**a);assert '\r\n' in bound['unit']['runtime_value'] and '\r\n' not in bound['unit']['value']
 r[FIELD]=row['value'];a['runtime_card']=enc(r);repin(a)
 with pytest.raises(ValueError):bind(**a)
