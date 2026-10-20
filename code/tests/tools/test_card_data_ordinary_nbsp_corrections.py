"""Actual existing correction accounting, not NBSP-family admission."""
import copy,gzip,json
from pathlib import Path
import pytest
from test_card_data_ordinary_nbsp_proof import bundle,args
from tools.card_data import models,source_manifest,normalization
from tools.card_data.serialization import canonical_json_bytes
F=Path(__file__).parents[1]/'fixtures/card_data/ordinary_nbsp_correction'
CID='EX12-031';FIELD='inherited_effect_description_eng'

def correction_bundle():
 b=bundle();x=json.loads(gzip.decompress((F/'EX12-031.json.gz').read_bytes()))
 b['canonical']['cards'][CID]=x['card'];b['report']['runtime_field_diffs'].append(x['diff'])
 for key,field in [('snapshot','official_observations'),('secondary','secondary_observations')]:
  b[key]['observations']+=x[field];m=b[key]['manifest'];m['expected_count']+=1
  for o in x[field]:
   m['observation_ids'].append(o['observation_id']);m['observation_refs'].append({'gameplay_card_id':CID,'printing_id':normalization.observation_candidate(models.SourceObservation(**(o|{'source':models.SourceName(o['source'])}))).printing_id,'observation_id':o['observation_id']})
   if o['source_url']not in m['source_locations']:m['source_locations'].append(o['source_url'])
 for r in x['raw_pages']:b['raw_pages'][r['url']]=gzip.decompress((F/r['file']).read_bytes())
 scopes=[]
 for key in ['snapshot','secondary']:
  m=b[key]['manifest'];scopes.append(models.ScopeManifest(**(m|{'source':models.SourceName(m['source']),'observation_refs':tuple(models.ScopeObservationRef(**r)for r in m['observation_refs'])})))
 b['aggregate']=source_manifest.build_full_pool_source_manifest(scopes).to_dict()
 for key in ['canonical','report']:b[key]['source_manifest_digest']=b['aggregate']['manifest_revision']
 b['canonical']['source_manifests']={k:v['scope_manifest_digest']for k,v in b['aggregate']['sources'].items()}
 return b

def invoke(b,mutate_binding=None):
 from tools.card_data.ordinary_nbsp_proof import prove
 a=args(b);binding=json.loads(a['binding_bytes']);binding.update(schema_version=2,approved_runtime_ledger_count=3,approved_correction_count=1,approved_overlap_count=0)
 binding['remaining_runtime_count']-=1;binding['remaining_text_count']-=1
 if mutate_binding:mutate_binding(binding)
 a['binding_bytes']=canonical_json_bytes(binding)
 return prove(**a)

def test_actual_correction_is_accounted_but_never_eligible():
 b=correction_bundle();before=copy.deepcopy(b);r=invoke(b);assert b==before
 assert r['counts']=={'runtime_count':10,'remaining_runtime_count':6,'remaining_text_count':6,'approved_runtime_ledger_count':3,'approved_correction_count':1,'approved_overlap_count':0}
 assert len(r['eligible'])==4
 row=next(x for x in r['excluded']if x['card_id']==CID)
 assert row['existing_approved']and 'card_has_correction'in row['reasons']
 assert r['correction_approvals'][0]['correction_id']=='focused-ex12-031-inherited-decode-source-zone-20261011'
 assert len(r['runtime_inventory'])==10

@pytest.mark.parametrize('change',['missing','foreign','missing_evidence','value','id','kind','canonical_value','canonical_provenance','derived','bool_adjudicated','duplicate','schema_bool','unknown_kind','binding_count','binding_schema','duplicate_evidence','report_provenance'])
def test_correction_refusal_boundaries(change):
 b=correction_bundle();d=b['report']['runtime_field_diffs'][-1];c=next(x for x in b['corrections']['corrections']if x['card_id']==CID);card=b['canonical']['cards'][CID]
 if change=='missing':b['corrections']['corrections'].remove(c)
 elif change=='foreign':c['evidence_observation_ids']=[b['snapshot']['observations'][0]['observation_id']]
 elif change=='missing_evidence':c['evidence_observation_ids']=['0'*64]
 elif change=='value':c['value']+='!'
 elif change=='id':c['correction_id']+='-changed'
 elif change=='kind':d['approval_kind']='adjudication'
 elif change=='unknown_kind':d['approval_kind']='unknown'
 elif change=='canonical_value':card['fields'][FIELD]+='!'
 elif change=='canonical_provenance':card['provenance'][FIELD]['selected_observation_id']='0'*64
 elif change=='derived':card['provenance'][FIELD]['derived']=True
 elif change=='bool_adjudicated':card['provenance'][FIELD]['adjudicated']=1
 elif change=='duplicate':b['corrections']['corrections'].append(copy.deepcopy(c))
 elif change=='schema_bool':c['schema_version']=True
 elif change=='duplicate_evidence':c['evidence_observation_ids']*=2
 elif change=='report_provenance':d['provenance_observation_ids']=[]
 tweak=(lambda x:x.update(approved_correction_count=0))if change=='binding_count'else(lambda x:x.update(schema_version=True))if change=='binding_schema'else None
 with pytest.raises(ValueError):invoke(b,tweak)


def test_legacy_binding_cannot_silently_change_correction_denominator():
 from tools.card_data.ordinary_nbsp_proof import prove
 with pytest.raises(ValueError,match='binding schema2'):prove(**args(correction_bundle()))

@pytest.mark.parametrize('field,value',[('approved_correction_count',True),('approved_runtime_ledger_count',False),('approved_overlap_count',1),('remaining_text_count',7)])
def test_schema2_explicit_counts_are_strict(field,value):
 with pytest.raises(ValueError):invoke(correction_bundle(),lambda b:b.update({field:value}))

def test_report_approval_boolean_is_not_integer():
 b=correction_bundle();b['report']['runtime_field_diffs'][-1]['approved']=1
 with pytest.raises(ValueError):invoke(b)
