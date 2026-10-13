import copy,json
import pytest
from test_diagnostic_paired_main_dispatch import expanded,base_files,snapshot,dispatch
from tools.clause_coverage import diagnostic_source_book as book
from tools.card_data.serialization import canonical_json_bytes as enc
from test_link_current_review import sha

def auth(expanded):return copy.deepcopy(json.loads(expanded[0]['input_manifest'])['authorizations']['EX1-066'])
def replace(expanded,a,obj,label):
    root=expanded[0]['input_root']; raw=enc(obj);p=root/('external-'+label+'.json');p.write_bytes(raw)
    a['paired_main_sidecar']={'path':p.relative_to(root).as_posix(),'sha256':sha(raw),'bytes':len(raw)}

@pytest.mark.parametrize('change',['duplicate_name','foreign_printing','duplicate_printing','extra_image'])
def test_reviewed_association_mutations_refuse(expanded,change):
    a=auth(expanded);s=json.loads(book._file(expanded[0]['input_root'],a['paired_main_sidecar']))
    if change=='duplicate_name':s['images'][1]['artifact_name']=s['images'][0]['artifact_name']
    elif change=='foreign_printing':s['images'][0]['printing_id']='BT1-001'
    elif change=='duplicate_printing':s['images'][1]['printing_id']=s['images'][0]['printing_id']
    else:s['images'].append(copy.deepcopy(s['images'][0]))
    replace(expanded,a,s,change)
    with pytest.raises((ValueError,TypeError)):dispatch(expanded,a)

@pytest.mark.parametrize('body',[[],True,None])
def test_wrong_sidecar_wire_refuses_as_composition_error(expanded,body):
    a=auth(expanded);replace(expanded,a,body,'bad-'+str(type(body).__name__))
    with pytest.raises((ValueError,TypeError)):dispatch(expanded,a)

def test_self_asserted_context_authorization_refused(expanded):
    a=auth(expanded);a['context']={'status':'COMPOSED_PAIRED_MAIN_CARD'}
    with pytest.raises(ValueError):dispatch(expanded,a)

def test_missing_original_observation_reaches_real_revalidation(expanded):
    a,snapshot=expanded;root=a['input_root'];m=json.loads(a['input_manifest'])
    source=book._source_inputs(snapshot,a['candidate_root'],'EX1-066',{u:book._file(root,d)for u,d in m['raw_pages'].items()})
    source['observations'].pop(next(iter(source['observations'])))
    with pytest.raises(ValueError):book._dispatch(snapshot,a['expected_extraction_sha256'],source,{n:book._file(root,d)for n,d in m['images'].items()},auth(expanded),root)

def test_top_level_malformed_sidecar_retains_blocked_full_inventory(expanded):
    a,_=expanded;args=dict(a);manifest=json.loads(a['input_manifest']);bad=auth(expanded)
    replace(expanded,bad,[], 'top-level-array')
    manifest['authorizations']['EX1-066']=bad
    args['input_manifest']=enc(manifest);args['expected_input_manifest_sha256']=sha(args['input_manifest'])
    result=book.build_diagnostic_source_book(**args)
    assert result['status']=='DIAGNOSTIC_ONLY_NOT_ADMISSION'
    assert len(result['cards'])==4484
    assert result['cards']['EX1-066']['status']=='BLOCKED'
    assert any(b['code']=='COMPOSITION_REFUSED' for b in result['cards']['EX1-066']['blockers'])
    assert len(result['cards']['EX1-066']['original_observations'])==14
    assert result['cards']['BT1-001']['status']=='NOT_EVALUATED'
    assert result['full_span_count'] is None and result['full_gating_interaction_count'] is None
