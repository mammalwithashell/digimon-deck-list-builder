"""Actual sealed R3 inputs; orchestration must not trust submitted compositions."""
import copy,gzip,hashlib,json
from pathlib import Path
import pytest
F=Path(__file__).parents[1]/'fixtures/card_data/diagnostic_source_book'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
enc=lambda value:json.dumps(value,ensure_ascii=False,sort_keys=True,indent=2).encode('utf-8')

@pytest.fixture(scope='module')
def files(tmp_path_factory):
 root=tmp_path_factory.mktemp('diagnostic-source-book')
 for name,d in json.loads((F/'archive.json').read_bytes()).items():
  raw=gzip.decompress((F/d['file']).read_bytes());assert sha(raw)==d['sha256'] and len(raw)==d['bytes']
  path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 return root

def inputs(root):
 e=(F/'extraction.json').read_bytes();m=(F/'inputs.json').read_bytes()
 return dict(candidate_root=root/'candidate',input_root=root,extraction_artifact=e,expected_extraction_sha256=sha(e),input_manifest=m,expected_input_manifest_sha256=sha(m))

def build(a):
 from tools.clause_coverage.diagnostic_source_book import build_diagnostic_source_book
 return build_diagnostic_source_book(**a)

@pytest.fixture(scope='module')
def actual(files):return build(inputs(files))

def test_actual_four_compositions_and_whole_universe_inventory(actual):
 assert actual['status']=='DIAGNOSTIC_ONLY_NOT_ADMISSION'
 assert actual['membership_counts']=={'official':4474,'canonical':4481,'runtime':4483}
 assert actual['evaluated_card_ids']==['BT24-031','BT26-033','EX12-031','EX12-046']
 for cid in actual['evaluated_card_ids']:
  r=actual['cards'][cid];assert r['status']=='COMPOSED_DIAGNOSTIC' and r['gating_interaction_count'] is None
  assert r['source_unit_descriptors'] and r['span_descriptors']
  assert not r['blockers'] and set(r['runtime_unclaimed'])<={'rarity','card_name_jpn'}
 assert actual['cards']['BT1-001']['status']=='NOT_EVALUATED'
 assert actual['cards']['LM-063']['status']=='BLOCKED' and actual['cards']['RB1-10']['status']=='BLOCKED'
 assert actual['cards']['TEST-020']['membership']['runtime_only']
 assert actual['cards']['LM-063']['span_count'] is None
 assert actual['full_span_count'] is None and 'clauses'not in actual and 'ready'not in actual

def test_condition_dual_rule_and_corrected_sources_conserved(actual):
 r=actual['cards']['EX12-046'];assert any(s['zone']=='conditions' and 'Assembly'in s['span']['timings'] for s in r['span_descriptors'])
 d=actual['cards']['BT26-033'];assert any(o['purpose']=='arts-reminder'and not o['executable']for o in d['structural_obligations'])
 c=actual['cards']['EX12-031'];assert len([x for x in c['source_composition']['consumption']if x['disposition']=='superseded-by-reviewed-correction'])==2
 assert c['structural_obligations'] and all(not x['executable']for x in c['structural_obligations'])
 for cid in actual['evaluated_card_ids']:
  r=actual['cards'][cid];assert r['conservation']['occurrences']==len(r['source_composition']['inventory'])==len(r['source_composition']['consumption'])
  assert len({tuple(x['key'])for x in r['source_unit_descriptors']})==len(r['source_unit_descriptors'])
  assert all('clause_id'not in x for x in r['span_descriptors'])

@pytest.mark.parametrize('mutation',['manifest_pin','snapshot_pin','unknown_flag','duplicate_scope','private_composition','extra_manifest'])
def test_reject_unpinned_or_injected_inputs(files,mutation):
 a=inputs(files)
 if mutation=='manifest_pin':a['expected_input_manifest_sha256']='0'*64
 elif mutation=='snapshot_pin':a['expected_extraction_sha256']='0'*64
 elif mutation=='unknown_flag':a['validated']=True
 else:
  m=json.loads(a['input_manifest'])
  if mutation=='duplicate_scope':m['evaluated_card_ids'].append(m['evaluated_card_ids'][0])
  elif mutation=='private_composition':m['compositions']={'BT24-031':{'status':'COMPOSED'}}
  else:m['ready']=True
  a['input_manifest']=enc(m);a['expected_input_manifest_sha256']=sha(a['input_manifest'])
 with pytest.raises((ValueError,TypeError)):build(a)


def test_duplicate_span_occurrence_is_not_conserved(actual):
 from tools.clause_coverage.diagnostic_source_book import _descriptors
 s=copy.deepcopy(actual['cards']['EX12-046']['source_composition'])
 z=next(v for v in s['zones'].values()if v['spans']);z['spans'].append(copy.deepcopy(z['spans'][0]))
 with pytest.raises(ValueError):_descriptors('EX12-046',s)

@pytest.mark.parametrize('change',['drop_consumption','foreign_unit','changed_container'])
def test_broken_source_conservation_refused(actual,change):
 from tools.clause_coverage.diagnostic_source_book import _descriptors
 s=copy.deepcopy(actual['cards']['BT24-031']['source_composition'])
 if change=='drop_consumption':s['consumption'].pop()
 elif change=='foreign_unit':s['consumption'][0]['unit_indices']=[99999]
 else:s['consumption'][0]['text']='forged'
 with pytest.raises(ValueError):_descriptors('BT24-031',s)

def one(a,cid):
 m=json.loads(a['input_manifest']);m['evaluated_card_ids']=[cid];m['authorizations']={k:v for k,v in m['authorizations'].items()if k==cid}
 a['input_manifest']=enc(m);a['expected_input_manifest_sha256']=sha(a['input_manifest']);return a

def test_unclaimed_gameplay_field_remains_blocked(files,actual,monkeypatch):
 from tools.clause_coverage import diagnostic_source_book as m
 r=actual['cards']['BT24-031'];claims=[x for x in r['runtime_claimed_fields']if x!='play_cost']
 monkeypatch.setattr(m,'_dispatch',lambda *a:(copy.deepcopy(r['source_composition']),claims,{}))
 d=build(one(inputs(files),'BT24-031'))['cards']['BT24-031']
 assert d['status']=='BLOCKED'and d['span_count']is None
 assert {'code':'UNCLAIMED_GAMEPLAY_FIELD','field':'play_cost'}in d['blockers']
 assert d['runtime_unclaimed_classification']['play_cost']=='unproven-gameplay'

def test_wrapper_refusal_preserves_original_observations(files,monkeypatch):
 from tools.clause_coverage import diagnostic_source_book as m
 def refuse(*a):raise ValueError('unknown source route')
 monkeypatch.setattr(m,'_dispatch',refuse)
 d=build(one(inputs(files),'BT24-031'))['cards']['BT24-031']
 assert d['status']=='BLOCKED'and d['span_count']is None and d['original_observations']
 assert d['gating_interaction_count']is None and 'source_unit_descriptors'not in d

def test_dependency_closure_contains_all_real_dispatchers():
 from tools.clause_coverage.diagnostic_source_book import diagnostic_dependencies
 d=diagnostic_dependencies()
 assert {'tools.clause_coverage.diagnostic_source_book','tools.clause_coverage.dual_card_composition',
 'tools.clause_coverage.corrected_rule_card_composition','tools.clause_coverage.runtime_empty_condition',
 'tools.clause_coverage.runtime_snapshot','tools.card_data.normalization'}<=set(d)


@pytest.mark.parametrize('change',['missing_raw','raw_digest','escaped_path','qa_missing_ruling_members'])
def test_pinned_dependency_refusals(files,change):
 a=one(inputs(files),'BT24-031');m=json.loads(a['input_manifest'])
 if change=='missing_raw':m['raw_pages']={}
 elif change=='raw_digest':next(iter(m['raw_pages'].values()))['sha256']='0'*64
 elif change=='escaped_path':next(iter(m['raw_pages'].values()))['path']='../outside.html'
 else:
  q=json.loads((files/m['qa']['path']).read_bytes());next(iter(q['qa'].values())).pop('card_ids')
  raw=enc(q);path=files/'invalid-qa.json';path.write_bytes(raw);m['qa']={'path':path.name,'sha256':sha(raw),'bytes':len(raw)}
 a['input_manifest']=enc(m);a['expected_input_manifest_sha256']=sha(a['input_manifest'])
 with pytest.raises((ValueError,FileNotFoundError)):build(a)

def test_qa_only_membership_retained_and_unknown(files,monkeypatch):
 from tools.clause_coverage import diagnostic_source_book as m
 a=one(inputs(files),'BT24-031');d=json.loads(a['input_manifest']);q=json.loads((files/d['qa']['path']).read_bytes())
 q['qa']['QA-DIAGNOSTIC-ONLY']={'card_ids':['QAONLY-001','BT24-031']}
 raw=enc(q);path=files/'qa-only-test.json';path.write_bytes(raw);d['qa']={'path':path.name,'sha256':sha(raw),'bytes':len(raw)}
 a['input_manifest']=enc(d);a['expected_input_manifest_sha256']=sha(a['input_manifest'])
 def refuse(*a):raise ValueError('skip card composition in membership-only control')
 monkeypatch.setattr(m,'_dispatch',refuse)
 r=build(a)['cards']['QAONLY-001'];assert r['membership']['qa_only']and r['status']=='BLOCKED'
 assert r['span_count']is None and r['gating_interaction_count']is None

def test_input_manifest_cannot_silently_select_provisional_card(files):
 a=inputs(files);d=json.loads(a['input_manifest']);d['evaluated_card_ids']=['LM-063'];d['authorizations']={}
 a['input_manifest']=enc(d);a['expected_input_manifest_sha256']=sha(a['input_manifest'])
 with pytest.raises(ValueError):build(a)


@pytest.mark.parametrize('boundary',['after_dispatch','during_snapshot_load'])
def test_dependency_drift_refuses_entire_diagnostic(files,monkeypatch,boundary):
 from tools.clause_coverage import diagnostic_source_book as m
 initial=m.diagnostic_dependencies();changed={**initial,'tools.clause_coverage.runtime_projection_proof':'0'*64}
 if boundary=='after_dispatch':
  original=m._dispatch
  def changing(*args):
   result=original(*args);monkeypatch.setattr(m,'diagnostic_dependencies',lambda:changed);return result
  monkeypatch.setattr(m,'_dispatch',changing)
 else:
  original=m.load_runtime_snapshot
  def changing(**kwargs):
   monkeypatch.setattr(m,'diagnostic_dependencies',lambda:changed);return original(**kwargs)
  monkeypatch.setattr(m,'load_runtime_snapshot',changing)
 with pytest.raises(ValueError,match='dependenc|changed|stable'):build(one(inputs(files),'BT24-031'))
