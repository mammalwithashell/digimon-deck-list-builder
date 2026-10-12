import copy,gzip,hashlib,json
from pathlib import Path
import pytest
F=Path(__file__).parents[1]/'fixtures/card_data/dual_kind_association'
enc=lambda v:json.dumps(v,ensure_ascii=False,sort_keys=True,indent=2).encode()
def inputs():
 d=json.loads((F/'inputs.json').read_bytes());p=(F/'proposal.json').read_bytes()
 return dict(aggregate=gzip.decompress((F/'aggregate.json.gz').read_bytes()),scopes={s:gzip.decompress((F/(s+'-scope.json.gz')).read_bytes())for s in ('official','digimoncard_io')},observations={k:enc(v)for k,v in d['observations'].items()},raw_pages={u:gzip.decompress((F/n).read_bytes())for u,n in d['raw_files'].items()},images={p.name:p.read_bytes()for p in F.glob('*.webp')},proposal=p,expected_proposal_sha256=hashlib.sha256(p).hexdigest(),expected_source_revision=json.loads(p)['source_revision'],expected_official_universe=d['expected_official_universe'])
def prove(a):
 from tools.clause_coverage.dual_kind_association import prove_dual_kind_association
 return prove_dual_kind_association(**a)
def test_actual_three_printings_proof_only_no_diagnostic_repair():
 a=inputs();before=copy.deepcopy(a);r=prove(a);assert a==before
 assert r['status']=='PROOF_ONLY_PENDING_INDEPENDENT_REVIEW' and len(r['associations'])==3
 assert all(d['status']=='BLOCKED_DUAL_DIAGNOSTIC' for d in r['original_diagnostics'])
 assert all(o['extracted']['card_type']=='Digimon/Tamer' for o in r['source_observations'])
 assert r['authority']=='proposed-image-derived-association' and not r['approved']
@pytest.mark.parametrize('change',['image','missing_image','swap_images','raw','missing_obs','foreign_obs','scope','universe','revision','pin','bool_version','unknown_field','omit_printing','duplicate_printing','wrong_kind','wrong_face','sections','image_url','printing','status'])
def test_invalid_association_or_full_input_refused(change):
 a=inputs();p=json.loads(a['proposal']);row=p['observations'][0]
 if change=='image':a['images'][row['image_artifact']]+=b'x'
 elif change=='missing_image':a['images'].pop(row['image_artifact'])
 elif change=='swap_images':a['images'][row['image_artifact']]=a['images'][p['observations'][1]['image_artifact']]
 elif change=='raw':a['raw_pages'][row['source_url']]+=b'x'
 elif change=='missing_obs':a['observations'].pop(row['observation_id'])
 elif change=='foreign_obs':a['observations']['0'*64]=a['observations'][row['observation_id']]
 elif change=='scope':s=json.loads(a['scopes']['official']);s['scope']='partial';a['scopes']['official']=enc(s)
 elif change=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
 elif change=='revision':a['expected_source_revision']='0'*64
 elif change=='pin':a['expected_proposal_sha256']='0'*64
 elif change=='bool_version':p['version']=True
 elif change=='unknown_field':p['approved']=True
 elif change=='omit_printing':p['observations'].pop()
 elif change=='duplicate_printing':p['observations'].append(copy.deepcopy(row))
 elif change=='wrong_kind':row['image_kind']='Digimon/Tamer'
 elif change=='wrong_face':row['face_identity'].reverse()
 elif change=='sections':row['sections_sha256']='0'*64
 elif change=='image_url':row['source_image_url']='https://invalid/image'
 elif change=='printing':row['printing_id']='foreign'
 elif change=='status':p['status']='APPROVED'
 a['proposal']=enc(p)
 if change!='pin':a['expected_proposal_sha256']=hashlib.sha256(a['proposal']).hexdigest()
 with pytest.raises(ValueError):prove(a)
@pytest.mark.parametrize('change',['bool_scope','bool_expected','secondary_parser','aggregate_extra','bad_face_bytes','missing_raw','unknown_sibling_in_proposal','missing_source_image','extra_image','duplicate_json'])
def test_additional_envelope_and_evidence_boundaries(change):
 a=inputs();p=json.loads(a['proposal']);row=p['observations'][0]
 if change in ('bool_scope','bool_expected'):
  s=json.loads(a['scopes']['official']);s['schema_version' if change=='bool_scope' else 'expected_count']=True;a['scopes']['official']=enc(s)
 elif change=='secondary_parser':s=json.loads(a['scopes']['digimoncard_io']);s['parser_version']='foreign';a['scopes']['digimoncard_io']=enc(s)
 elif change=='aggregate_extra':d=json.loads(a['aggregate']);d['extra']='unreviewed';a['aggregate']=enc(d)
 elif change=='bad_face_bytes':o=json.loads(a['observations'][row['observation_id']]);o['extracted']['faces'][1]['text_sections'].append({'label':'Unknown','text':'retain me'});a['observations'][row['observation_id']]=enc(o)
 elif change=='missing_raw':a['raw_pages'].pop(row['source_url'])
 elif change=='unknown_sibling_in_proposal':row['ignored_siblings']=[]
 elif change=='missing_source_image':row['source_image_url']=''
 elif change=='extra_image':a['images']['unowned.webp']=b'not owned'
 else:a['proposal']=b'{"version":1,"version":1}';a['expected_proposal_sha256']=hashlib.sha256(a['proposal']).hexdigest()
 if change!='duplicate_json':a['proposal']=enc(p);a['expected_proposal_sha256']=hashlib.sha256(a['proposal']).hexdigest()
 with pytest.raises(ValueError):prove(a)
def test_output_complete_source_copies_and_dependency_closure():
 from tools.clause_coverage import dual_kind_association as module
 a=inputs();r=prove(a)
 assert {o['observation_id']:o for o in r['source_observations']}=={k:json.loads(v)for k,v in a['observations'].items()}
 assert len(r['original_diagnostics'])==3 and all(len(d['occurrences'])==4 for d in r['original_diagnostics'])
 assert r['checker_sha256']==hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
 expected={'card_data/models.py','card_data/serialization.py','card_data/source_manifest.py','card_data/normalization.py','card_data/errors.py','card_data/sources/official.py','clause_coverage/dual_source_units.py'}
 assert set(r['dependency_sha256'])==expected
 for name,h in r['dependency_sha256'].items():assert hashlib.sha256((Path(module.__file__).parents[1]/name).read_bytes()).hexdigest()==h
 r['source_observations'][0]['extracted']['faces'].clear()
 assert all(len(json.loads(v)['extracted']['faces'])==2 for v in a['observations'].values())
