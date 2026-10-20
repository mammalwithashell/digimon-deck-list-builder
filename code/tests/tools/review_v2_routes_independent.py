import copy,gzip,hashlib,json
from pathlib import Path
import pytest
from tools.clause_coverage import source_book_v2 as b
from tools.clause_coverage.diagnostic_source_book import _descriptors
P=Path('C:/Users/james/.codex/worktrees/fe80/digimon-deck-list-builder-1/runs/card-loop/meta-six-20261010')
R=Path('D:/cl-data-repair')
F=R/'code/tests/fixtures/card_data/source_book_v2_composition_routes'
D=json.loads(gzip.decompress((F/'fresh-six-diagnostic.json.gz').read_bytes()))

def pins():
 f=json.loads((P/'source-book-v2-composition-routes-frozen-20261013.json').read_bytes())
 for row in f['owned_files']: assert hashlib.sha256((R/row['path']).read_bytes()).hexdigest()==row['sha256']
 for path,h in f['dependencies'].items(): assert hashlib.sha256((R/path).read_bytes()).hexdigest()==h
 return 11+len(f['dependencies'])

def refresh(cid,row):
 for k,v in zip(('source_unit_descriptors','span_descriptors','structural_obligations','conservation'),_descriptors(cid,row['source_composition'])): row[k]=v
 row['span_count']=len(row['span_descriptors'])

def test_exact_pin_and_blocked_inventory():
 assert pins()==93
 result=b._compile_replayed_diagnostic(D)
 assert len(result['cards'])==4484 and len(result['clauses'])==22
 assert result['diagnostic_evidence']==D
 assert result['cards']['BT24-081']['clause_count'] is None
 assert result['cards']['BT24-081']['clause_ids']==[]
 assert D['cards']['BT24-081']['span_count']==7
 assert result['full_clause_count'] is result['full_gating_interaction_count'] is None
 assert sum(x['card_id']=='EX1-066' for x in result['clauses'])==3
 assert not any(x['card_id']=='BT24-081' for x in result['clauses'])
 b.validate_wire_book_v2(result)

@pytest.mark.parametrize('mutation',['same_artifact','same_printing','evidence_extra','pin_boolean'])
def test_independent_paired_cross_association_refusals(mutation):
 row=copy.deepcopy(D['cards']['EX1-066']);s=row['source_composition'];v=s['correction_binding']
 if mutation=='same_artifact':v['images'][1]['artifact_name']=v['images'][0]['artifact_name']
 elif mutation=='same_printing':v['images'][1]['printing_id']=v['images'][0]['printing_id']
 elif mutation=='evidence_extra':v['corrections'][0]['evidence_observation_ids'].append('foreign')
 else:v['images'][0]['sha256']=True
 s['zones']['effect']['source_binding']=copy.deepcopy(v)
 with pytest.raises((ValueError,KeyError,TypeError)):
  refresh('EX1-066',row);b._reconcile('EX1-066',row)

def test_removed_link_condition_with_complete_views_refuses():
 row=copy.deepcopy(D['cards']['BT21-074']);s=row['source_composition']
 s['units'].pop();s['zones']['conditions']['unit_indices']=[];s['zones']['conditions']['spans']=[]
 for c in s['consumption']:
  if c['unit_indices']==[3]:c.update(unit_indices=[],disposition='retained-alternative')
 refresh('BT21-074',row)
 with pytest.raises(ValueError):b._reconcile('BT21-074',row)

def test_wire_unknown_counts_not_zero():
 result=b._compile_replayed_diagnostic(D);result['full_clause_count']=0
 with pytest.raises(ValueError):b.validate_wire_book_v2(result)
 assert pins()==93
