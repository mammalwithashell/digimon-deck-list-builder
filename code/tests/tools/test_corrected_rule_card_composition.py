"""Corrected whole-card composition uses original records and explicit test-only pins."""
import copy,gzip,json,hashlib
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from test_canonical_correction_units import inputs as correction_inputs
from test_clause_source_units import inputs as ordinary_inputs
from test_runtime_projection_proof import inputs as runtime_inputs
from tools.clause_coverage import runtime_projection_proof as runtime

FIELD='inherited_effect_description_eng'
def checker_pins():
 from tools.clause_coverage.corrected_rule_card_composition import checker_dependencies
 return checker_dependencies()

def inputs():
 c=correction_inputs();o=ordinary_inputs();r=runtime_inputs()
 a={k:v for k,v in c.items() if k not in ('runtime_card','sidecar','expected_reviewed_sidecar_sha256','image_bytes')}
 a.update(runtime_record=c['runtime_card'],registry_bytes=r['registry_bytes'],condition_selections=o['condition_selections'],image_artifacts=o['image_artifacts'] if 'image_artifacts'in o else r['image_artifacts'],correction_sidecar=c['sidecar'],expected_reviewed_sidecar_sha256=c['expected_reviewed_sidecar_sha256'],correction_image_bytes=c['image_bytes'])
 a['projection_binding']=enc({'schema_version':1,'input_sha256':runtime.input_hashes(a),'producer_sha256':runtime.producer_dependencies(),'checker_sha256':checker_pins()})
 return a

def repin(a):
 a.pop('projection_binding',None)
 a['projection_binding']=enc({'schema_version':1,'input_sha256':runtime.input_hashes(a),'producer_sha256':runtime.producer_dependencies(),'checker_sha256':checker_pins()})

def compose(a):
 from tools.clause_coverage.corrected_rule_card_composition import compose_corrected_rule_card
 return compose_corrected_rule_card(**a)

def test_actual_correction_rule_conditions_and_runtime_all_retained():
 a=inputs();before=copy.deepcopy(a);r=compose(a)
 assert a==before and r['status']=='COMPOSED_CORRECTED_RULE_CARD'
 assert len(r['correction_binding']['source_observations'])==4
 assert len(r['runtime_projection']['claimed_fields'])==19
 assert r['runtime_projection']['original_runtime_bytes']==a['runtime_record'].decode()
 assert r['structural_obligations'] and all(not o['executable'] for o in r['structural_obligations'])
 assert r['zones']['conditions']['unit_indices']
 z=r['zones']['inherited'];assert 'from its digivolution cards'in z['value']
 assert len(z['unit_indices'])==1
 u=r['units'][z['unit_indices'][0]]
 assert u['kind']=='reviewed-canonical-image-correction' and u['authority']=='adjudicated'
 assert not {'section_index','observation_id','char_start','char_end'}&set(u)
 superseded=[x for x in r['consumption']if x['disposition']=='superseded-by-reviewed-correction']
 assert len(superseded)==2 and all(x['replacement_unit_indices']==z['unit_indices']for x in superseded)
 assert all('from its digivolution cards'not in x['text']for x in superseded)
 assert len(r['consumption'])==len(r['inventory']) and 'ready'not in r and 'clauses'not in r

@pytest.mark.parametrize('mutation',['image','sidecar','registry','runtime_other','runtime_inherited','missing_observation','missing_raw','old_revision','unknown_flag','canonical_other','universe','extra_correction'])
def test_rebound_inputs_cannot_bypass_source_or_runtime_guards(mutation):
 a=inputs()
 if mutation=='image':a['correction_image_bytes']+=b'x'
 elif mutation=='sidecar':a['expected_reviewed_sidecar_sha256']='0'*64
 elif mutation=='registry':
  d=json.loads(a['registry_bytes']);d['assignment_count']=True;a['registry_bytes']=enc(d)
 elif mutation.startswith('runtime'):
  d=json.loads(a['runtime_record']);d['play_cost'if mutation=='runtime_other'else FIELD]='invented';a['runtime_record']=enc(d)
 elif mutation=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
 elif mutation=='missing_raw':a['raw_pages']={}
 elif mutation=='old_revision':a['old_input_revision']='0'*64
 elif mutation=='unknown_flag':a['validated']=True
 elif mutation=='canonical_other':
  d=json.loads(a['canonical_card']);d['fields']['play_cost']=999;a['canonical_card']=enc(d)
 elif mutation=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
 else:
  d=json.loads(a['correction_document']);d['corrections'].append(copy.deepcopy(d['corrections'][0]));a['correction_document']=enc(d)
 repin(a)
 with pytest.raises((ValueError,TypeError)):compose(a)

def test_submitted_composition_cannot_self_certify():
 a=inputs();r=compose(a);r['zones']['inherited']['value']='forged'
 from tools.clause_coverage.corrected_rule_card_composition import validate_corrected_rule_card
 with pytest.raises(ValueError):validate_corrected_rule_card(r,**a)


@pytest.mark.parametrize('name',['_runtime_validator','_correction_binding','validated','context'])
def test_old_public_source_entrypoints_never_accept_private_injection(name):
 from tools.clause_coverage.source_units import compose_ordinary_card
 a=ordinary_inputs();a[name]=lambda **kwargs:None
 with pytest.raises((TypeError,ValueError)):compose_ordinary_card(**a)

@pytest.mark.parametrize('name',['_runtime_validator','_correction_binding','runtime_projection'])
def test_new_public_entrypoint_never_accepts_private_injection(name):
 a=inputs();a[name]={};repin(a)
 with pytest.raises((TypeError,ValueError)):compose(a)


def test_unhandled_rule_sibling_keeps_whole_corrected_card_blocked(monkeypatch):
 from tools.clause_coverage import rule_card_composition as rules
 original=rules.derive_rule_occurrences
 def blocked(raw):
  result=original(raw);result['blockers'].append({'code':'UNSUPPORTED_RULE_TEST_WITNESS'})
  return result
 monkeypatch.setattr(rules,'derive_rule_occurrences',blocked)
 r=compose(inputs())
 assert r['status']=='BLOCKED' and r['blockers']
 assert not r['units'] and not r['zones']
 assert all(x['disposition']=='blocked'for x in r['consumption'])
 assert len(r['correction_binding']['original_sections'])==2


def test_default_context_and_custom_runtime_gate_preserve_canonical_checks():
 from tools.clause_coverage.source_units import _validated_context,_validated_context_core
 a=ordinary_inputs();calls=[]
 def validator(**kwargs):
  calls.append(kwargs)
  assert kwargs['runtime']['card_id']==kwargs['card'].card_id
 assert _validated_context(**a)==_validated_context_core(**a,_runtime_validator=validator)
 assert len(calls)==1 and set(calls[0])=={'card','runtime','typed','kind'}
 d=json.loads(a['canonical_card']);d['fields']['effect_description_eng']='forged';a['canonical_card']=enc(d)
 with pytest.raises(ValueError):_validated_context_core(**a,_runtime_validator=validator)


def test_runtime_callback_cannot_return_success_flag():
 from tools.clause_coverage.source_units import _validated_context_core
 with pytest.raises(ValueError):_validated_context_core(**ordinary_inputs(),_runtime_validator=lambda **kwargs:True)


@pytest.mark.parametrize('marker',['<Link>','\uff1cLink\uff1e','\uff1clink\uff1e'])
def test_original_link_marker_unicode_bytes_remain_refused(marker):
 from tools.clause_coverage.source_units import _unsupported_ordinary_text
 assert _unsupported_ordinary_text(marker)

@pytest.mark.parametrize('field,value',[('play_cost',999),('card_kind',True),('index',-1),('form_eng',['invented']),('effect_description_eng','invented'),('security_effect_description_eng','invented')])
def test_full_runtime_claims_checked_even_with_test_only_reissued_sidecar(field,value):
 a=inputs();d=json.loads(a['runtime_record']);d[field]=value;a['runtime_record']=enc(d)
 side=json.loads(a['correction_sidecar']);side['input_sha256']['runtime_card']=hashlib.sha256(a['runtime_record']).hexdigest()
 a['correction_sidecar']=enc(side);a['expected_reviewed_sidecar_sha256']=hashlib.sha256(a['correction_sidecar']).hexdigest();repin(a)
 with pytest.raises(ValueError):compose(a)


def test_missing_checker_closure_pin_is_refused():
 a=inputs();b=json.loads(a['projection_binding']);b.pop('checker_sha256',None);a['projection_binding']=enc(b)
 with pytest.raises(ValueError):compose(a)

def test_checker_closure_covers_executed_runtime_and_source_helpers():
 from tools.clause_coverage.corrected_rule_card_composition import checker_dependencies
 deps=checker_dependencies()
 for name in ['corrected_rule_card_composition','runtime_projection_proof','canonical_correction_units','source_units','rule_card_composition','source_composition','rule_source_units','condition_app_fusion']:
  assert 'tools.clause_coverage.'+name in deps

@pytest.mark.parametrize('field,value',[('schema_version',True),('registry_version',True),('registry_version',1.0),('padding_index',False),('padding_index',0.0),('capacity',8192.0),('assignment_count',True)])
def test_all_numeric_registry_headers_are_strict_integers(field,value):
 a=inputs();d=json.loads(a['registry_bytes']);d[field]=value;a['registry_bytes']=enc(d);repin(a)
 with pytest.raises(ValueError):compose(a)

@pytest.mark.parametrize('suffix',['\n[Rule] This card is also treated as having the [Dragon] type.','\n< Link > Draw 1.','\n[SpecialRule] unsupported.','\n\uff1cLink\uff1e Draw 1.'])
def test_new_structural_corrected_text_keeps_whole_card_blocked(suffix):
 from tools.card_data import generate
 a=inputs();d=json.loads(a['correction_document']);r=next(x for x in d['corrections']if x['card_id']=='EX12-031');r['value']+=suffix
 c=json.loads(a['canonical_card']);c['fields'][FIELD]=r['value'];v=json.loads(a['runtime_record']);v[FIELD]=generate._legacy_text(r['value'])
 a.update(correction_document=enc(d),canonical_card=enc(c),runtime_record=enc(v))
 p=json.loads(a['correction_sidecar']);p['correction_row_sha256']=hashlib.sha256(enc(r)).hexdigest()
 for k,n in [('canonical_card','canonical_card'),('runtime_card','runtime_record'),('correction_document','correction_document')]:p['input_sha256'][k]=hashlib.sha256(a[n]).hexdigest()
 a['correction_sidecar']=enc(p);a['expected_reviewed_sidecar_sha256']=hashlib.sha256(a['correction_sidecar']).hexdigest();repin(a)
 out=compose(a)
 assert out['status']=='BLOCKED' and not out['units'] and not out['zones']
 assert any(b['code']=='UNSUPPORTED_CORRECTED_FIELD_STRUCTURE'for b in out['blockers'])
 assert len(out['correction_binding']['original_sections'])==2


def test_changed_checker_dependency_pin_refused():
 a=inputs();b=json.loads(a['projection_binding']);b['checker_sha256']['tools.clause_coverage.rule_card_composition']='0'*64;a['projection_binding']=enc(b)
 with pytest.raises(ValueError):compose(a)


def test_unknown_registry_metadata_refused():
 a=inputs();d=json.loads(a['registry_bytes']);d['unreviewed_metadata']='extra';a['registry_bytes']=enc(d);repin(a)
 with pytest.raises(ValueError):compose(a)
