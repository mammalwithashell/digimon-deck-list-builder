"""Independent controls over the new EX1 paired proof. Never admission."""
import copy
import json
import pytest
from test_paired_main_correction_units import inputs, repin, bind, enc, sha
from tools.clause_coverage.paired_main_correction_units import validate_paired_main_correction_binding


def test_exact_sections_and_unclaimed_fields_remain_visible():
    a=inputs(); r=bind(a)
    assert len({(s['observation_id'],s['section_index']) for s in r['original_sections']})==7
    runtime=json.loads(a['runtime_card'])
    assert set(r['runtime_claims']).isdisjoint(r['unclaimed_runtime_fields'])
    assert {**r['runtime_claims'],**r['unclaimed_runtime_fields']}==runtime
    assert r['unit']['field_path']=='effect_description_eng'
    assert r['face_mirror_obligation']['field_path']=='faces.0.effect_text'
    assert r['face_mirror_obligation']['executable'] is False
    assert len(r['source_observations'])==14


@pytest.mark.parametrize('mutation',['face_kind','extra_face','missing_secondary','reordered_image_bytes','extra_image','missing_effect_evidence'])
def test_retained_scope_and_face_identity_refuse_self_repin(mutation):
    a=inputs()
    if mutation in ('face_kind','extra_face'):
        d=json.loads(a['canonical_card'])
        if mutation=='face_kind':d['faces'][0]['kind']='option'
        else:d['faces'].append(copy.deepcopy(d['faces'][0]))
        a['canonical_card']=enc(d)
    elif mutation=='missing_secondary':
        key=next(k for k,v in a['observations'].items() if json.loads(v)['source']=='digimoncard_io')
        del a['observations'][key]
    elif mutation=='reordered_image_bytes':
        a['images']['primary.png'],a['images']['primary-p1.png']=a['images']['primary-p1.png'],a['images']['primary.png']
    elif mutation=='extra_image':a['images']['unreviewed.png']=b'unreviewed'
    else:
        d=json.loads(a['sidecar']);d['evidence_observation_ids']=['0'*64,'1'*64]
        a['sidecar']=enc(d);a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
    repin(a)
    with pytest.raises(ValueError):bind(a)


def test_duplicate_json_key_is_refused_even_when_rehashed():
    a=inputs()
    raw=a['sidecar'];a['sidecar']=b'{"schema_version":1,'+raw[1:]
    a['expected_reviewed_sidecar_sha256']=sha(a['sidecar'])
    with pytest.raises(ValueError):bind(a)


def test_forged_binding_cannot_promote_face_to_second_executable_unit():
    a=inputs();r=bind(a);r['face_mirror_obligation']['executable']=True
    with pytest.raises(ValueError):validate_paired_main_correction_binding(r,**a)
