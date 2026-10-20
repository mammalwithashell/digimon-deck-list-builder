"""Version2 compilation is opt-in and cannot certify submitted success flags."""
import copy,gzip,json,hashlib
from pathlib import Path
import pytest
F=Path(__file__).parents[1]/'fixtures/card_data/source_book_v2'

@pytest.fixture
def diagnostic():return json.loads(gzip.decompress((F/'historical-four-diagnostic.json.gz').read_bytes()))

def compile_private(d):
 from tools.clause_coverage.source_book_v2 import _compile_replayed_diagnostic
 return _compile_replayed_diagnostic(d)

def test_real_four_routes_and_full_unknown_inventory(diagnostic):
 b=compile_private(diagnostic)
 assert b['book_schema_version']==2 and b['semantic_fingerprint_version']==2
 assert len(b['cards'])==4484 and b['full_clause_count']is None
 assert b['full_gating_interaction_count']is None
 assert {c:len(b['cards'][c]['clause_ids'])for c in diagnostic['evaluated_card_ids']}=={'BT24-031':3,'BT26-033':9,'EX12-031':5,'EX12-046':5}
 assert b['cards']['BT1-001']['clause_count']is None
 assert b['cards']['LM-063']['status']=='BLOCKED'
 assert b['cards']['TEST-020']['membership']['test_only']
 assert all('#v2#'in r['id']for r in b['clauses'])
 assert b['diagnostic_evidence']==diagnostic
 assert all(x['executable']is False for x in b['cards']['BT26-033']['structural_obligations'])
 assert b['cards']['EX12-031']['superseded_source_occurrences']==2
 assert len(b['clauses'])==22

def test_assembly_uses_reviewed_projection_not_local_raw_span(diagnostic):
 b=compile_private(diagnostic)
 rows=[r for r in b['clauses']if r['card_id']=='EX12-046'and r['timings']==['Assembly']]
 assert len(rows)==1 and rows[0]['text']=='-2: Lv.4 or lower [TB] trait card'
 assert rows[0]['zone']=='conditions'
 assert len(rows[0]['descriptor_views'])==2
 assert any(v['span']['kind']=='untimed'for v in rows[0]['descriptor_views'])

def test_determinism_and_exact_bytes_preserved(diagnostic):
 from tools.clause_coverage.source_book_v2 import wire_bytes
 a=compile_private(diagnostic);b=compile_private(copy.deepcopy(diagnostic));assert wire_bytes(a['clauses'][0]['semantic_payload'])==wire_bytes(b['clauses'][0]['semantic_payload'])
 assert a==b
 assert diagnostic['cards']['EX12-031']['source_composition']==a['diagnostic_evidence']['cards']['EX12-031']['source_composition']

@pytest.mark.parametrize('value',[True,False,1.0,{},[False],['x',{'y':1}]])
def test_wire_rejects_noncontract_types(value):
 from tools.clause_coverage.source_book_v2 import wire_bytes
 with pytest.raises(ValueError):wire_bytes(value)

def test_wire_preserves_unicode_and_line_endings():
 from tools.clause_coverage.source_book_v2 import wire_bytes
 assert wire_bytes(['\uff1cRule\uff1e','a\r\nb'])==b'["\xef\xbc\x9cRule\xef\xbc\x9e","a\\r\\nb"]'

@pytest.mark.parametrize('change',['drop_projection','duplicate_projection','false_index','changed_projection','foreign_unit','lost_consumption','altered_descriptor','structural_executable'])
def test_reconciliation_refuses_lost_or_forged_views(diagnostic,change):
 r=diagnostic['cards']['EX12-046'];s=r['source_composition'];z=s['zones']['conditions']['spans']
 if change=='drop_projection':z.pop()
 elif change=='duplicate_projection':z.append(copy.deepcopy(z[0]))
 elif change=='false_index':z[0]['span_index']=False
 elif change=='changed_projection':z[-1]['text']='-0: free'
 elif change=='foreign_unit':z[0]['unit_index']=9000
 elif change=='lost_consumption':s['consumption'].pop()
 elif change=='altered_descriptor':r['span_descriptors'][0]['span']['text']='forged'
 else:
  rr=diagnostic['cards']['EX12-031'];rr['structural_obligations'][0]['executable']=True
 with pytest.raises((ValueError,KeyError,IndexError)):compile_private(diagnostic)

@pytest.mark.parametrize('change',['timing','source_generation','runtime_evidence','structural','alternate'])
def test_semantics_bind_full_card_runtime_and_source_closure(diagnostic,change):
 a=compile_private(diagnostic);d=copy.deepcopy(diagnostic)
 if change=='source_generation':d['snapshot_sha256']='0'*64
 elif change=='runtime_evidence':d['cards']['BT24-031']['composition_evidence']['test_runtime_binding']='changed'
 elif change=='alternate':d['cards']['BT24-031']['original_observations']['new-observation']={'retained':'changed'}
 elif change=='structural':
  d['cards']['BT24-031']['composition_evidence']['structural_binding']='changed'
 else:
  d['input_manifest_sha256']='1'*64
 b=compile_private(d);aa={r['id']:r['semantic_sha256']for r in a['clauses']if r['card_id']=='BT24-031'};bb={r['id']:r['semantic_sha256']for r in b['clauses']if r['card_id']=='BT24-031'}
 assert aa.keys()==bb.keys() and all(aa[k]!=bb[k]for k in aa)

def test_no_public_submitted_diagnostic_flag():
 from tools.clause_coverage.source_book_v2 import build_source_book_v2
 with pytest.raises(TypeError):build_source_book_v2(diagnostic={'status':'COMPOSED'},validated=True)

def test_reader_reconstructs_payload_and_rejects_foreign_hash(diagnostic):
 from tools.clause_coverage.source_book_v2 import validate_wire_book_v2
 b=compile_private(diagnostic);validate_wire_book_v2(b)
 b['clauses'][0]['text']='changed'
 with pytest.raises(ValueError):validate_wire_book_v2(b)


def refresh_views(cid,row):
 from tools.clause_coverage.diagnostic_source_book import _descriptors
 values=_descriptors(cid,row['source_composition'])
 for key,value in zip(['source_unit_descriptors','span_descriptors','structural_obligations','conservation'],values):row[key]=value
 row['span_count']=len(row['span_descriptors'])

def test_forged_local_condition_span_refused_even_when_views_self_rehashed(diagnostic):
 row=diagnostic['cards']['EX12-046'];u=next(u for u in row['source_composition']['units']if u.get('field')=='xros_req')
 u['spans'][0]['text']='forged local source';refresh_views('EX12-046',row)
 with pytest.raises(ValueError):compile_private(diagnostic)

def test_repeated_equal_ordinary_spans_stay_distinct(diagnostic):
 from tools.clause_coverage.text_split import split_clauses
 row=diagnostic['cards']['BT24-031'];source=row['source_composition'];unit=source['units'][0]
 text='[On Play] Draw 1.\n[On Play] Draw 1.'
 unit['projected_text']=text
 source['zones']['effect']['spans']=[{'unit_index':0,'span_index':n,'kind':s.kind,'timings':s.timings,'keyword':s.keyword,'text':s.text}for n,s in enumerate(split_clauses(text))]
 refresh_views('BT24-031',row);b=compile_private(diagnostic)
 rows=[r for r in b['clauses']if r['card_id']=='BT24-031'and r['zone']=='effect']
 assert len(rows)==2 and rows[0]['text']==rows[1]['text'] and rows[0]['id']!=rows[1]['id']

def test_marker_only_timing_is_not_dropped(diagnostic):
 row=diagnostic['cards']['BT24-031'];s=row['source_composition'];s['units'][0]['projected_text']='[On Play]'
 s['zones']['effect']['spans']=[{'unit_index':0,'span_index':0,'kind':'timing','timings':['On Play'],'keyword':None,'text':''}]
 refresh_views('BT24-031',row);b=compile_private(diagnostic)
 assert any(r['card_id']=='BT24-031'and r['timings']==['On Play']and r['text']==''for r in b['clauses'])

@pytest.mark.parametrize('boundary',['source','dependency'])
def test_entry_refuses_drift_after_internal_reconstruction(diagnostic,monkeypatch,boundary):
 from tools.clause_coverage import source_book_v2 as m
 n=[0]
 def state(*a):
  n[0]+=1;return {'x':'a'if n[0]==1 or boundary!='source'else'b'}
 monkeypatch.setattr(m,'_input_state',state)
 deps=m.compiler_dependencies()
 def build(**kw):
  if boundary=='dependency':monkeypatch.setattr(m,'compiler_dependencies',lambda:{**deps,'changed':'1'})
  return diagnostic
 monkeypatch.setattr(m.diagnostic,'build_diagnostic_source_book',build)
 with pytest.raises(ValueError,match='changed'):
  m.build_source_book_v2(candidate_root='unused',input_root='unused',extraction_artifact=b'{}',expected_extraction_sha256=hashlib.sha256(b'{}').hexdigest(),input_manifest=b'{}',expected_input_manifest_sha256=hashlib.sha256(b'{}').hexdigest())


@pytest.mark.parametrize('change',['bool_version','unsupported_version','deleted_clause','duplicate_clause','changed_hash','dropped_unknown','empty_count','structural_drop'])
def test_wire_reader_rejects_inconsistent_artifact(diagnostic,change):
 from tools.clause_coverage.source_book_v2 import validate_wire_book_v2
 b=compile_private(diagnostic)
 if change=='bool_version':b['book_schema_version']=True
 elif change=='unsupported_version':b['semantic_fingerprint_version']=99
 elif change=='deleted_clause':b['clauses'].pop()
 elif change=='duplicate_clause':b['clauses'].append(copy.deepcopy(b['clauses'][0]))
 elif change=='changed_hash':b['clauses'][0]['semantic_sha256']='0'*64
 elif change=='dropped_unknown':b['cards'].pop('BT1-001')
 elif change=='empty_count':b['cards']['BT1-001']['clause_count']=0
 else:b['cards']['EX12-031']['structural_obligations']=[]
 with pytest.raises(ValueError):validate_wire_book_v2(b)

def test_runtime_field_source_units_and_obligations_all_bound(diagnostic):
 b=compile_private(diagnostic)
 for cid in diagnostic['evaluated_card_ids']:
  expected=hashlib.sha256(json.dumps(diagnostic['cards'][cid],ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode('utf-8')).hexdigest()
  assert b['cards'][cid]['source_closure_sha256']==expected
  assert all(r['semantic_payload'][-1]==expected for r in b['clauses']if r['card_id']==cid)

def test_compiler_does_not_rewrite_input_or_legacy_evidence(diagnostic):
 before=copy.deepcopy(diagnostic);compile_private(diagnostic);assert before==diagnostic


def test_actual_bt24_101_cost_alternatives_each_survive(diagnostic):
 diagnostic['cards']['BT24-101']=json.loads(gzip.decompress((F/'historical-BT24-101-row.json.gz').read_bytes()))
 diagnostic['evaluated_card_ids']=sorted(diagnostic['evaluated_card_ids']+['BT24-101'])
 b=compile_private(diagnostic);rows=[r for r in b['clauses']if r['card_id']=='BT24-101'and r['zone']=='conditions']
 assert len(rows)==2 and rows[0]['id']!=rows[1]['id']
 assert rows[0]['timings']==rows[1]['timings']==['Digivolve']
 assert any('Cost 5'in r['text']for r in rows)


def test_explicit_empty_ordinary_unit_is_retained_without_a_clause(diagnostic):
 row=diagnostic['cards']['BT24-031'];s=row['source_composition'];s['units'][0]['projected_text']=''
 s['zones']['effect'].update(spans=[],value='',presence='empty-box')
 refresh_views('BT24-031',row);b=compile_private(diagnostic)
 assert not any(r['card_id']=='BT24-031'and r['zone']=='effect'for r in b['clauses'])
 assert b['diagnostic_evidence']['cards']['BT24-031']['source_unit_descriptors'][0]['source_unit']['projected_text']==''
 assert b['cards']['BT24-031']['clause_count']==2


def test_structural_only_composition_retains_obligation_and_known_zero(diagnostic):
 # A pure internal compiler control, derived from the real Dual Arts container;
 # no claim that this synthetic card is an authoritative printed source.
 cid='BT26-033';row=diagnostic['cards'][cid];s=row['source_composition']
 u=next(copy.deepcopy(u)for u in s['units']if u['role']=='arts-reminder');s['units']=[u]
 for c in s['consumption']:
  c['unit_indices']=[0]if(c['observation_id'],c['section_index'])==(u['observation_id'],u['section_index'])else[]
  c['disposition']='consumed'if c['unit_indices']else'retained-alternative'
 refresh_views(cid,row);b=compile_private(diagnostic)
 assert b['cards'][cid]['clause_count']==0 and not b['cards'][cid]['clause_ids']
 assert b['cards'][cid]['structural_obligations'] and all(not x['executable']for x in b['cards'][cid]['structural_obligations'])
 assert b['cards']['BT1-001']['clause_count']is None


def test_shared_wire_vectors_reconstruct_exact_utf8_and_digest(diagnostic):
 from tools.clause_coverage.source_book_v2 import _materialize,wire_bytes
 v=json.loads((F/'semantic_vectors_v2.json').read_bytes())
 diagnostic['cards']['BT24-101']=json.loads(gzip.decompress((F/'historical-BT24-101-row.json.gz').read_bytes()))
 diagnostic['evaluated_card_ids']=sorted(diagnostic['evaluated_card_ids']+['BT24-101'])
 b=_materialize(diagnostic,v['compiler_dependencies'])
 assert b['source_generation_payload']==v['source_generation_payload']
 assert b['source_generation_sha256']==v['source_generation_sha256']
 assert len(b['clauses'])==len(v['vectors'])
 for row,vector in zip(b['clauses'],v['vectors'],strict=True):
  assert row['id']==vector['id'] and row['semantic_payload']==vector['payload']
  raw=wire_bytes(vector['payload']);assert raw.hex()==vector['utf8_hex']
  assert hashlib.sha256(raw).hexdigest()==row['semantic_sha256']==vector['sha256']
 for vector in v['scalar_vectors']:
  raw=wire_bytes(vector['payload']);assert raw.hex()==vector['utf8_hex'] and hashlib.sha256(raw).hexdigest()==vector['sha256']
 for bad in v['rejected_payloads']:
  with pytest.raises(ValueError):wire_bytes(bad)


def test_fresh_four_card_public_api_once(tmp_path):
 import os
 from tools.clause_coverage.source_book_v2 import build_source_book_v2,validate_wire_book_v2,compiler_dependencies
 original=F.parent/'diagnostic_source_book'
 for name,d in json.loads((original/'archive.json').read_bytes()).items():
  raw=gzip.decompress((original/d['file']).read_bytes());assert hashlib.sha256(raw).hexdigest()==d['sha256'] and len(raw)==d['bytes']
  target=tmp_path/name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
 side=(original/'correction-sidecar-paired-main-context-proposed.json').read_bytes()
 assert hashlib.sha256(side).hexdigest()=='6d971ad54d370726461a03692e9921c1bb3499f72a05bd03cc483aed1bccdc6d'
 (tmp_path/'reviews/correction-sidecar-paired-main-context-proposed.json').write_bytes(side)
 extraction=(original/'extraction.json').read_bytes();manifest=(original/'inputs-paired-main-context-proposed.json').read_bytes();deps=compiler_dependencies()
 b=build_source_book_v2(candidate_root=tmp_path/'candidate',input_root=tmp_path,extraction_artifact=extraction,
  expected_extraction_sha256=hashlib.sha256(extraction).hexdigest(),input_manifest=manifest,expected_input_manifest_sha256=hashlib.sha256(manifest).hexdigest())
 validate_wire_book_v2(b);assert deps==compiler_dependencies()
 assert len(b['clauses'])==22 and len(b['cards'])==4484
 assert all(b['cards'][cid]['status']=='COMPILED_REVIEW_ONLY'for cid in ['BT24-031','BT26-033','EX12-031','EX12-046'])
 assert b['cards']['EX12-031']['superseded_source_occurrences']==2
 if os.environ.get('SOURCE_BOOK_V2_REVIEW_OUTPUT'):
  target=Path(os.environ['SOURCE_BOOK_V2_REVIEW_OUTPUT']);assert not target.exists(),'do not overwrite prior integration evidence'
  raw=json.dumps(b,ensure_ascii=False,sort_keys=True,indent=2).encode('utf-8');target.write_bytes(gzip.compress(raw,mtime=0))
