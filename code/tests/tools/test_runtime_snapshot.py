"""Synthetic container boundary tests; fixtures do not approve real candidates."""
import hashlib
import json
from pathlib import Path
import pytest

def enc(x):return json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def sha(x):return hashlib.sha256(x).hexdigest()

def snapshot_inputs(tmp_path):
    from tools.card_data.serialization import canonical_json_bytes
    records={'BT1-001':{'card_id':'BT1-001','schema_version':1}}
    aggregate={'manifest_revision':'a'*64,'sources':{'official':{'gameplay_card_ids':['BT1-001']}}}
    payloads={'canonical':{'cards':records},'runtime':records,'registry':{'schema_version':1},
              'aggregate':aggregate,'official_scope':{},'secondary_scope':{},'corrections':{},
              'selections':{},'adjudications':{}}
    files={}
    for role,payload in payloads.items():
        raw=enc(payload);path=role+'.json';(tmp_path/path).write_bytes(raw)
        files[role]={'path':path,'sha256':sha(raw),'bytes':len(raw)}
    art={'schema_version':1,'kind':'exact-runtime-snapshot-extraction-v1','source_revision':'a'*64,
         'files':files,'official_card_ids':['BT1-001'],
         'canonical_record_sha256':{k:sha(canonical_json_bytes(v))for k,v in records.items()},
         'runtime_record_sha256':{k:sha(canonical_json_bytes(v))for k,v in records.items()}}
    return dict(candidate_root=tmp_path,extraction_artifact=enc(art),expected_extraction_sha256=sha(enc(art)))

def test_verified_snapshot_reads_each_file_once_and_keeps_immutable_bytes(tmp_path,monkeypatch):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    a=snapshot_inputs(tmp_path);counts={};read=Path.read_bytes
    def tracked(p):counts[str(p)]=counts.get(str(p),0)+1;return read(p)
    monkeypatch.setattr(Path,'read_bytes',tracked)
    s=load_runtime_snapshot(**a);first=s.card('BT1-001');second=s.card('BT1-001')
    assert first==second and len(counts)==9 and set(counts.values())=={1}
    first['canonical_card']=b'bad';assert s.card('BT1-001')==second
    (tmp_path/'runtime.json').write_bytes(b'changed')
    assert s.card('BT1-001')==second
    with pytest.raises(ValueError):load_runtime_snapshot(**a)

@pytest.mark.parametrize('change',['schema_bool','artifact_pin','file_hash','size_bool','traversal',
    'absolute','missing_role','extra_role','record_hash','foreign_record','missing_member',
    'universe','revision','unknown','duplicate_paths'])
def test_snapshot_refusals(tmp_path,change):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    a=snapshot_inputs(tmp_path);d=json.loads(a['extraction_artifact'])
    if change=='schema_bool':d['schema_version']=True
    elif change=='artifact_pin':a['expected_extraction_sha256']='0'*64
    elif change=='file_hash':d['files']['runtime']['sha256']='0'*64
    elif change=='size_bool':d['files']['runtime']['bytes']=True
    elif change=='traversal':d['files']['runtime']['path']='../runtime.json'
    elif change=='absolute':d['files']['runtime']['path']='C:/runtime.json'
    elif change=='missing_role':d['files'].pop('registry')
    elif change=='extra_role':d['files']['extra']=d['files']['runtime']
    elif change=='record_hash':d['runtime_record_sha256']['BT1-001']='0'*64
    elif change=='foreign_record':d['runtime_record_sha256']['BT1-999']=d['runtime_record_sha256'].pop('BT1-001')
    elif change=='missing_member':d['canonical_record_sha256']={}
    elif change=='universe':d['official_card_ids']=[]
    elif change=='revision':d['source_revision']='b'*64
    elif change=='unknown':d['approved']=True
    else:d['files']['runtime']['path']=d['files']['canonical']['path']
    a['extraction_artifact']=enc(d)
    if change!='artifact_pin':a['expected_extraction_sha256']=sha(a['extraction_artifact'])
    with pytest.raises(ValueError):load_runtime_snapshot(**a)

def test_snapshot_rejects_forged_handle(tmp_path):
    from tools.clause_coverage.runtime_snapshot import VerifiedRuntimeSnapshot
    with pytest.raises(TypeError):VerifiedRuntimeSnapshot()


@pytest.mark.parametrize('role',['canonical','runtime','registry','aggregate','official_scope',
    'secondary_scope','corrections','selections','adjudications'])
def test_duplicate_json_keys_rejected_in_every_full_file(tmp_path,role):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    a=snapshot_inputs(tmp_path);d=json.loads(a['extraction_artifact'])
    raw=b'{"key":1,"key":2}';(tmp_path/d['files'][role]['path']).write_bytes(raw)
    d['files'][role].update(sha256=sha(raw),bytes=len(raw));a['extraction_artifact']=enc(d)
    a['expected_extraction_sha256']=sha(a['extraction_artifact'])
    with pytest.raises(ValueError):load_runtime_snapshot(**a)


@pytest.mark.parametrize('role',['canonical','runtime'])
def test_record_key_must_equal_card_id_with_rebound_artifact(tmp_path,role):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    from tools.card_data.serialization import canonical_json_bytes
    a=snapshot_inputs(tmp_path);d=json.loads(a['extraction_artifact'])
    row={'card_id':'BT1-002','schema_version':1};doc={'BT1-001':row}
    raw=enc({'cards':doc}if role=='canonical'else doc)
    (tmp_path/d['files'][role]['path']).write_bytes(raw)
    d['files'][role].update(sha256=sha(raw),bytes=len(raw))
    d[role+'_record_sha256']['BT1-001']=sha(canonical_json_bytes(row))
    a['extraction_artifact']=enc(d);a['expected_extraction_sha256']=sha(a['extraction_artifact'])
    with pytest.raises(ValueError):load_runtime_snapshot(**a)


@pytest.mark.parametrize('path',['//server/share.json','runtime.json:stream','./runtime.json','a/../runtime.json','a\\runtime.json'])
def test_all_unsafe_path_forms_rejected(tmp_path,path):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    a=snapshot_inputs(tmp_path);d=json.loads(a['extraction_artifact']);d['files']['runtime']['path']=path
    a['extraction_artifact']=enc(d);a['expected_extraction_sha256']=sha(a['extraction_artifact'])
    with pytest.raises(ValueError):load_runtime_snapshot(**a)


def test_snapshot_metadata_is_defensive_and_handle_read_only(tmp_path):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    s=load_runtime_snapshot(**snapshot_inputs(tmp_path));d=s.metadata();d['files']['runtime']['sha256']='0'*64
    d['official_card_ids'].append('BT1-099')
    assert s.metadata()['files']['runtime']['sha256']!='0'*64
    assert s.metadata()['official_card_ids']==['BT1-001']
    with pytest.raises(TypeError):s._artifact=b'changed'
    with pytest.raises(TypeError):s._files['runtime']=b'changed'


def test_distinct_role_paths_cannot_alias_one_hardlinked_file(tmp_path):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    a=snapshot_inputs(tmp_path)
    (tmp_path/'secondary_scope.json').unlink()
    (tmp_path/'secondary_scope.json').hardlink_to(tmp_path/'official_scope.json')
    with pytest.raises(ValueError):load_runtime_snapshot(**a)
