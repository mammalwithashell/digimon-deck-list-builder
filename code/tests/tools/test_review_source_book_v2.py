"""Separate offline review loader; never promotes the source-book status."""
import json,hashlib
from pathlib import Path
import pytest
F=Path(__file__).parents[1]/'fixtures/card_data/source_book_v2_rust/fresh-four-review-book.json'
sha=lambda b:hashlib.sha256(b).hexdigest()

def test_actual_book_is_immutable_review_only():
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 raw=F.read_bytes();b=load_review_v2(raw,expected_sha256=sha(raw))
 assert b.card_count==4484 and len(b.clause_ids())==22 and not b.is_recordable
 assert b.card_clause_count('BT24-031')==3 and b.card_clause_count('BT1-001')is None
 assert b.card_clause_count('LM-063')is None
 with pytest.raises(ValueError):b.card_clause_count('MISSING-1')
 first=b.clause_ids()[0];binding=b.binding(first);assert binding.semantic_version==2 and binding.source_generation_sha256==b.source_generation_sha256
 doc=b.document();doc['clauses'][0]['text']='mutated copy';assert b.document()['clauses'][0]['text']!='mutated copy'
 with pytest.raises((AttributeError,TypeError)):binding.semantic_version=1
 with pytest.raises((AttributeError,TypeError)):b.source_generation_sha256='0'*64

@pytest.mark.parametrize('change',['wrong_pin','bool_version','unknown_version','body','generation','unknown_zero','omit_card','drop_clause','structural','duplicate_key','nonfinite'])
def test_malformed_or_mutated_book_refused(change):
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 raw=F.read_bytes();d=json.loads(raw)
 if change=='wrong_pin':
  with pytest.raises(ValueError):load_review_v2(raw,expected_sha256='0'*64)
  return
 if change=='bool_version':d['book_schema_version']=True
 elif change=='unknown_version':d['semantic_fingerprint_version']=99
 elif change=='body':d['clauses'][0]['text']='changed'
 elif change=='generation':d['source_generation_sha256']='0'*64
 elif change=='unknown_zero':d['cards']['BT1-001']['clause_count']=0
 elif change=='omit_card':d['cards'].pop('BT1-001')
 elif change=='drop_clause':d['clauses'].pop()
 elif change=='structural':d['cards']['EX12-031']['structural_obligations']=[]
 elif change=='duplicate_key':
  raw=b'{"book_schema_version":1,'+raw[1:]
 elif change=='nonfinite':raw=b'{"nonfinite":NaN,'+raw[1:]
 if change not in ('duplicate_key','nonfinite'):raw=json.dumps(d,ensure_ascii=False).encode('utf-8')
 with pytest.raises(ValueError):load_review_v2(raw,expected_sha256=sha(raw))

def test_public_api_does_not_take_admission_flags():
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 with pytest.raises(TypeError):load_review_v2(F.read_bytes(),expected_sha256=sha(F.read_bytes()),validated=True)

def test_all_shared_wire_vectors_and_refusals():
 from tools.clause_coverage.source_book_v2 import wire_bytes
 vectors=json.loads((F.parent.parent/'source_book_v2/semantic_vectors_v2.json').read_bytes())
 assert len(vectors['vectors'])==27
 for row in vectors['vectors']+vectors['scalar_vectors']:
  raw=wire_bytes(row['payload']);assert raw.hex()==row['utf8_hex'] and sha(raw)==row['sha256']
 for payload in vectors['rejected_payloads']:
  with pytest.raises(ValueError):wire_bytes(payload)

@pytest.mark.parametrize('raw',[b'[]',b'null',b'1',b'false',b'{}',b'{',b'{}{}',b'\xff',b'{"a":1,"a":2}',b'{"x":Infinity}'])
def test_malformed_envelopes_fail_with_value_error(raw):
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 with pytest.raises(ValueError):load_review_v2(raw,expected_sha256=sha(raw))

@pytest.mark.parametrize('pin',[None,True,7,'F'*64,'g'*64,'a'*63])
def test_strict_external_pin_type_and_format(pin):
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 with pytest.raises(ValueError):load_review_v2(F.read_bytes(),expected_sha256=pin)

def test_no_implicit_v1_or_unknown_envelope_fields():
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 for extra in ({'version':1,'clauses':[]},{'recordable':True},{'validated':True}):
  doc=json.loads(F.read_bytes());doc.update(extra);raw=json.dumps(doc).encode()
  with pytest.raises(ValueError):load_review_v2(raw,expected_sha256=sha(raw))

@pytest.mark.parametrize('value',[None,False,2,[],"invalid"])
def test_malformed_nested_diagnostic_refuses_consistently(value):
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 doc=json.loads(F.read_bytes());doc['diagnostic_evidence']=value
 raw=json.dumps(doc).encode()
 with pytest.raises(ValueError):load_review_v2(raw,expected_sha256=sha(raw))

@pytest.mark.parametrize('case',['count_bool','count_mismatch','drop_unknown','membership_type','blockers_type','duplicate_eval','foreign_eval'])
@pytest.mark.parametrize('entry',['reader','materializer'])
def test_inventory_schema_and_counts_recomputed(case,entry):
 from tools.clause_coverage.review_source_book_v2 import load_review_v2
 from tools.clause_coverage.source_book_v2 import _materialize
 d=json.loads(F.read_bytes());e=d['diagnostic_evidence']
 if case=='count_bool':d['membership_counts']['official']=e['membership_counts']['official']=True
 elif case=='count_mismatch':d['membership_counts']['official']=e['membership_counts']['official']=4473
 elif case=='drop_unknown':d['cards'].pop('BT1-001');e['cards'].pop('BT1-001')
 elif case=='membership_type':d['cards']['BT1-001']['membership']['official']=e['cards']['BT1-001']['membership']['official']='yes'
 elif case=='blockers_type':d['cards']['BT1-001']['blockers']=e['cards']['BT1-001']['blockers']='unknown'
 elif case=='duplicate_eval':e['evaluated_card_ids'].append(e['evaluated_card_ids'][0])
 else:e['evaluated_card_ids'].append('MISSING-999')
 raw=json.dumps(d,ensure_ascii=False).encode()
 with pytest.raises(ValueError):
  if entry=='reader':load_review_v2(raw,expected_sha256=sha(raw))
  else:_materialize(e,d['compiler_dependencies'])
