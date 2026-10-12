import copy,gzip,json,hashlib
from pathlib import Path
import pytest
from test_dual_kind_association import inputs as association_inputs
F=Path(__file__).parents[1]/'fixtures/card_data/dual_card_composition'
BASE=F.parent/'source_binding/conditions'
enc=lambda x:json.dumps(x,ensure_ascii=False,sort_keys=True,indent=2).encode()
def inputs():
 a=association_inputs();pins=json.loads((F/'pins.json').read_bytes());runtime=gzip.decompress((F/'runtime-snapshot.json.gz').read_bytes());cid='BT26-033'
 images=dict(a['images']);document=json.loads((BASE/'card_corrections.json').read_bytes())
 for row in document.get('compatibility_corrections',[]):images[row['image_artifact_path']]=(BASE/Path(row['image_artifact_path']).name).read_bytes()
 return dict(aggregate=a['aggregate'],scopes=a['scopes'],observations=a['observations'],raw_pages=a['raw_pages'],expected_official_universe=a['expected_official_universe'],old_input_revision=a['expected_source_revision'],new_input_revision=a['expected_source_revision'],canonical_card=(F/'canonical.json').read_bytes(),runtime_projection=enc(json.loads(runtime)[cid]),adjudication_ledger=gzip.decompress((F/'reviewed-ledger.json.gz').read_bytes()),correction_document=(BASE/'card_corrections.json').read_bytes(),condition_selections=(BASE/'card_compatibility_selections.json').read_bytes(),image_artifacts=images,association_proposal=a['proposal'],association_pin=a['expected_proposal_sha256'],association_review=(F/'association-review.json').read_bytes(),review_pin=pins['review_sha256'],runtime_snapshot=runtime,runtime_snapshot_pin=pins['runtime_snapshot_sha256'],registry=gzip.decompress((F/'registry.json.gz').read_bytes()),registry_pin=pins['registry_sha256'])
def compose(a):
 from tools.clause_coverage.dual_card_composition import compose_dual_card
 return compose_dual_card(**a)
def test_actual_dual_complete_gameplay_with_unclaimed_metadata():
 a=inputs();before=copy.deepcopy(a);r=compose(a);assert a==before
 assert r['status']=='COMPOSED_DUAL_SOURCE' and r['unclaimed_runtime_fields']==['card_name_jpn','rarity']
 assert all(d['status']=='BLOCKED_DUAL_DIAGNOSTIC'for d in r['original_diagnostics'])
 assert len(r['units'])==4 and len(r['consumption'])==12
 assert r['units'][-1]['role']=='arts-reminder'and not r['units'][-1]['spans']
 assert r['condition_binding']['projection']==json.loads(a['runtime_projection'])['xros_req']
 assert r['runtime_source_record']==json.loads(a['runtime_projection'])
@pytest.mark.parametrize('change',['review_pin','review_status','proposal','raw','image','missing_obs','missing_face','reverse_face','condition','kind','nested_cost','nested_text','nested_keyword','extra_gameplay','registry','snapshot','private_callback','missing_review'])
def test_binding_and_gameplay_refusals(change):
 a=inputs();r=json.loads(a['runtime_projection'])
 if change=='review_pin':a['review_pin']='0'*64
 elif change=='review_status':d=json.loads(a['association_review']);d['status']='pending';a['association_review']=enc(d);a['review_pin']=hashlib.sha256(a['association_review']).hexdigest()
 elif change=='proposal':a['association_proposal']+=b' '
 elif change=='raw':k=next(iter(a['raw_pages']));a['raw_pages'][k]+=b'x'
 elif change=='image':a['image_artifacts']['BT26-033.webp']+=b'x'
 elif change=='missing_obs':a['observations'].pop(next(iter(a['observations'])))
 elif change in ('missing_face','reverse_face'):
  c=json.loads(a['canonical_card']);c['faces'].pop()if change=='missing_face'else c['faces'].reverse();a['canonical_card']=enc(c)
 elif change=='registry':a['registry']+=b' '
 elif change=='snapshot':a['runtime_snapshot']+=b' '
 elif change=='private_callback':a['_runtime_validator']=lambda **kw:None
 elif change=='missing_review':a.pop('association_review')
 else:
  if change=='condition':r['xros_req']=''
  elif change=='kind':r['card_kind']=0
  elif change=='nested_cost':r['dual']['option']['use_cost']=1
  elif change=='nested_text':r['dual']['option']['effect_text']=''
  elif change=='nested_keyword':r['dual']['option']['keywords']=[]
  elif change=='extra_gameplay':r['unreviewed_choice']=True
  snapshot=json.loads(a['runtime_snapshot']);snapshot['BT26-033']=r;a['runtime_snapshot']=enc(snapshot);a['runtime_snapshot_pin']=hashlib.sha256(a['runtime_snapshot']).hexdigest();a['runtime_projection']=enc(r)
 with pytest.raises((ValueError,TypeError)):compose(a)
def test_public_a2_still_refuses_dual_siblings():
 from tools.clause_coverage.source_binding import bind_condition_units
 a=inputs()
 with pytest.raises(ValueError):bind_condition_units(aggregate=a['aggregate'],observations=a['observations'],canonical_card=a['canonical_card'],runtime_projection=enc({'card_id':'BT26-033','xros_req':json.loads(a['runtime_projection'])['xros_req']}),selection_ledger=a['condition_selections'],correction_document=a['correction_document'],image_artifacts=a['image_artifacts'],raw_pages=a['raw_pages'],old_input_revision=a['old_input_revision'],new_input_revision=a['new_input_revision'])
@pytest.mark.parametrize('change',['review_checker','review_dependency','review_source','proposal_pin','missing_arts','top_cost','digimon_dp','use_requirement','unknown_dual_key','registry_index'])
def test_full_projection_and_review_ownership_controls(change):
 a=inputs()
 if change.startswith('review_'):
  d=json.loads(a['association_review'])
  if change=='review_checker':next(x for x in d['files']if Path(x['path']).name=='dual_kind_association.py')['sha256']='0'*64
  elif change=='review_dependency':d['dependencies'][next(iter(d['dependencies']))]='0'*64
  else:d['source_revision']='0'*64
  a['association_review']=enc(d);a['review_pin']=hashlib.sha256(a['association_review']).hexdigest()
 elif change=='proposal_pin':a['association_pin']='0'*64
 else:
  r=json.loads(a['runtime_projection'])
  if change=='missing_arts':r['dual']['digimon']['keywords']=[]
  elif change=='top_cost':r['play_cost']=3
  elif change=='digimon_dp':r['dual']['digimon']['dp']=14000
  elif change=='use_requirement':r['dual']['option']['effect_text']=r['dual']['option']['effect_text'].replace('Use Req.','Other Req.')
  elif change=='unknown_dual_key':r['dual']['automatic_choice']=True
  else:r['index']+=1
  d=json.loads(a['runtime_snapshot']);d['BT26-033']=r;a['runtime_snapshot']=enc(d);a['runtime_snapshot_pin']=hashlib.sha256(a['runtime_snapshot']).hexdigest();a['runtime_projection']=enc(r)
 with pytest.raises(ValueError):compose(a)

@pytest.mark.parametrize('change',['schema_bool','version_bool','padding_bool','count_float','capacity_float','missing_other_official','unknown_metadata'])
def test_full_registry_typed_envelope_and_official_membership(change):
 a=inputs();r=json.loads(a['registry'])
 if change=='schema_bool':r['schema_version']=True
 elif change=='version_bool':r['registry_version']=True
 elif change=='padding_bool':r['padding_index']=False
 elif change=='count_float':r['assignment_count']=float(r['assignment_count'])
 elif change=='capacity_float':r['capacity']=float(r['capacity'])
 elif change=='unknown_metadata':r['unreviewed_epoch']=1
 else:
  other=next(cid for cid in a['expected_official_universe']if cid!='BT26-033')
  del r['assignments'][other];r['assignment_count']-=1
 a['registry']=enc(r);a['registry_pin']=hashlib.sha256(a['registry']).hexdigest()
 with pytest.raises(ValueError):compose(a)
