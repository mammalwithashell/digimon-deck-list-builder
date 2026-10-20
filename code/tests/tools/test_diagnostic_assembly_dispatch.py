"""Assembly dispatch retains separate source and implementation evidence."""
import copy,json,hashlib
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
from tools.clause_coverage import diagnostic_source_book as book
from test_diagnostic_source_book import files as base_files,inputs as base_inputs
from test_assembly_card_composition import inputs as assembly_inputs

sha=lambda b:hashlib.sha256(b).hexdigest()
@pytest.fixture(scope='module')
def expanded(base_files):
 a=base_inputs(base_files)
 snapshot=load_runtime_snapshot(candidate_root=a['candidate_root'],extraction_artifact=a['extraction_artifact'],expected_extraction_sha256=a['expected_extraction_sha256'])
 m=json.loads(a['input_manifest']);m['evaluated_card_ids']=['BT24-081','BT26-073'];m['authorizations']={}
 def artifact(name,raw):
  p=base_files/'assembly-dispatch'/name;p.parent.mkdir(parents=True,exist_ok=True)
  if p.exists():assert p.read_bytes()==raw
  else:p.write_bytes(raw)
  return {'path':p.relative_to(base_files).as_posix(),'sha256':sha(raw),'bytes':len(raw)}
 for cid in m['evaluated_card_ids']:
  source=assembly_inputs(snapshot,cid)
  for oid,raw in source['observations'].items():
   lane=json.loads(raw)['source'];p=a['candidate_root']/'data/card_sources/observations'/lane/(oid+'.json');p.parent.mkdir(parents=True,exist_ok=True)
   if p.exists():assert p.read_bytes()==raw
   else:p.write_bytes(raw)
  for url,raw in source['raw_pages'].items():
   if url in m['raw_pages']:assert (base_files/m['raw_pages'][url]['path']).read_bytes()==raw
   else:m['raw_pages'][url]=artifact(sha(url.encode())+'.html',raw)
  for name,raw in source['image_artifacts'].items():
   if name in m['images']:assert (base_files/m['images'][name]['path']).read_bytes()==raw
   else:m['images'][name]=artifact(sha(name.encode())+'.image',raw)
  m['authorizations'][cid]={'assembly_yaml':artifact(cid+'.yaml',source['yaml_bytes'])}
 a['input_manifest']=enc(m);a['expected_input_manifest_sha256']=sha(a['input_manifest']);return a,snapshot

@pytest.fixture(scope='module')
def actual(expanded):return book.build_diagnostic_source_book(**expanded[0])

def test_source_claims_and_full_inventory_survive_implementation_blocker(actual):
 assert len(actual['cards'])==4484 and actual['membership_counts']=={'official':4474,'canonical':4481,'runtime':4483}
 assert actual['full_span_count'] is None and actual['full_gating_interaction_count'] is None
 for cid in ['BT24-081','BT26-073']:
  r=actual['cards'][cid]
  assert r.get('source_status')=='COMPOSED_RULE_CARD',r['blockers']
  assert len(r['runtime_claimed_fields'])==20 and 'digixros_costs'in r['runtime_claimed_fields']
  assert set(r['runtime_unclaimed'])=={'rarity','card_name_jpn'}
  assert r['source_unit_descriptors'] and r['span_descriptors'] and r['gating_interaction_count'] is None
  assert r['conservation']['occurrences']==len(r['source_composition']['inventory'])==len(r['source_composition']['consumption'])
  assert r['implementation_correspondence']['native_compilation_verified'] is False
 assert actual['cards']['BT1-001']['status']=='NOT_EVALUATED'
 assert actual['cards']['TEST-020']['membership']['runtime_only']

def test_named_kind_obligation_is_not_lost(actual):
 r=actual['cards']['BT24-081'];assert r['status']=='BLOCKED'
 assert r['implementation_correspondence']['status']=='BLOCKED'
 assert r['implementation_correspondence']['unresolved_obligations'][0]['code']=='NAMED_MATERIAL_KIND_CLOSURE_REQUIRED'
 assert any(b['code']=='IMPLEMENTATION_CORRESPONDENCE_BLOCKED'for b in r['blockers'])
 assert r['implementation_correspondence']==r['composition_evidence']['implementation_correspondence']

def test_static_recipe_is_not_native_behavior_or_readiness(actual):
 r=actual['cards']['BT26-073'];assert r['status']=='COMPOSED_DIAGNOSTIC',r['blockers']
 assert r['implementation_correspondence']['status']=='STATIC_RECIPE_ONLY'
 assert r['implementation_correspondence']['unresolved_obligations']==[]
 assert r['composition_evidence']['admission_performed'] is False
 assert actual['status']=='DIAGNOSTIC_ONLY_NOT_ADMISSION'

def dispatch(expanded,cid,auth):
 a,snapshot=expanded;m=json.loads(a['input_manifest']);root=a['input_root']
 source=book._source_inputs(snapshot,a['candidate_root'],cid,{u:book._file(root,d)for u,d in m['raw_pages'].items()})
 return book._dispatch(snapshot,a['expected_extraction_sha256'],source,{n:book._file(root,d)for n,d in m['images'].items()},auth,root)

@pytest.mark.parametrize('change',['missing','extra','foreign','hash','bytes_bool','path','cost'])
def test_exact_assembly_authorization_and_bytes_required(expanded,change):
 a,_=expanded;m=json.loads(a['input_manifest']);auth=copy.deepcopy(m['authorizations']['BT24-081'])
 if change=='missing':auth={}
 elif change=='extra':auth['link_yaml']=auth['assembly_yaml']
 elif change=='foreign':auth=copy.deepcopy(m['authorizations']['BT26-073'])
 elif change=='hash':auth['assembly_yaml']['sha256']='0'*64
 elif change=='bytes_bool':auth['assembly_yaml']['bytes']=True
 elif change=='path':auth['assembly_yaml']['path']='../outside'
 else:
  raw=book._file(a['input_root'],auth['assembly_yaml']);assert b'cost: 6'in raw;raw=raw.replace(b'cost: 6',b'cost: 7');p=a['input_root']/'assembly-dispatch/mutated.yaml';p.write_bytes(raw);auth['assembly_yaml']={'path':p.relative_to(a['input_root']).as_posix(),'sha256':sha(raw),'bytes':len(raw)}
 with pytest.raises((ValueError,TypeError,OSError)):dispatch(expanded,'BT24-081',auth)

def test_dependency_inventory_includes_assembly_recipe_and_producers():
 assert {'tools.clause_coverage.assembly_card_composition','tools.clause_coverage.assembly_recipe_binding','tools.ingest_cards','tools.xros_cost_parser'}<=set(book.diagnostic_dependencies())


def test_assembly_authorization_cannot_relabel_an_ordinary_card(expanded):
 a,_=expanded;auth=json.loads(a['input_manifest'])['authorizations']['BT24-081']
 with pytest.raises(ValueError,match='Assembly legacy field required'):
  dispatch(expanded,'BT24-031',auth)
