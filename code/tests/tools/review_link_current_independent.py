import json,copy
import pytest
from test_link_current_review import snapshot,inputs,rebind,REVIEW,MAIN
from tools.clause_coverage.link_card_composition import compose_link_card
from tools.card_data.errors import CardDataError

def test_exact_current_review_metadata_and_unmodified_history(snapshot):
 args=inputs(snapshot);r=compose_link_card(**args)
 row=[a for a in r['canonical_conflict_review'] if a['field_path']==MAIN]
 assert len(row)==1;row=row[0]
 assert row['status']=='current' and row['approval_required'] is True
 assert REVIEW in json.dumps(row)
 ledger=json.loads(args['adjudication_ledger'])
 approval=next(a for a in ledger['adjudications'] if a['adjudication_id']==REVIEW)
 assert row['candidate_digest']==approval['old_digest']
 assert row['selected_value_digest']==approval['new_digest']
 assert row['selected_value']==approval['new_value']
 assert row['selected_observation_id']==approval['selected_observation_id']
 assert set(row['candidate_observation_ids'])==set(approval['provenance_observation_ids'])==set(args['observations'])
 assert {oid for a in row['candidate_values'] for oid in a['observation_ids']}==set(args['observations'])
 assert row['old_input_revision']==approval['old_input_revision']==args['old_input_revision']
 assert row['new_input_revision']==approval['new_input_revision']==args['new_input_revision']
 assert json.loads(r['original_canonical_bytes'])==json.loads(args['canonical_card'])
 assert r['native_compilation_verified'] is False
 assert not r.get('runtime_admission',False)

@pytest.mark.parametrize('change',['old_revision','duplicate','decision'])
def test_separately_rebound_container_cannot_supply_stale_or_ambiguous_review(snapshot,tmp_path,change):
 def alter(doc):
  row=next(r for r in doc['adjudications'] if r['adjudication_id']==REVIEW)
  if change=='old_revision':row['old_input_revision']='f'*64
  elif change=='duplicate':doc['adjudications'].append(copy.deepcopy(row))
  else:row['decision']='rejected'
 changed=rebind(snapshot,tmp_path,'adjudications',alter)
 with pytest.raises((ValueError,CardDataError)):
  compose_link_card(**inputs(changed))

def test_fake_review_summary_cannot_replace_a1_binding(snapshot,tmp_path,monkeypatch):
 def alter(doc):doc['adjudications']=[r for r in doc['adjudications'] if r['adjudication_id']!=REVIEW]
 changed=rebind(snapshot,tmp_path,'adjudications',alter)
 from tools.card_data import normalization
 monkeypatch.setattr(normalization,'canonical_conflict_review_rows',lambda *a,**k:[{'field_path':MAIN,'approval_required':True,'status':'current'}])
 with pytest.raises((ValueError,CardDataError)):
  compose_link_card(**inputs(changed))
