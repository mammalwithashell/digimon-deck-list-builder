"""Exact R3 containers and records; no fixture producer-view rewrites.

The external extraction hash is root-reviewed extraction evidence. Passing these
tests does not confer source admission or replace independent candidate review.
"""
import gzip
import json
from pathlib import Path
import pytest

from tools.clause_coverage import runtime_projection_proof as p
from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
from tools.clause_coverage.runtime_rule_composition import compose_runtime_rule_card

BASE=Path(__file__).parents[1]/'fixtures/card_data'
F=BASE/'runtime_snapshot'
EXTRACTION_SHA='9770babb5cafac567fe65f37cec62a5718e9646fd9d158913db024cf42204211'


@pytest.fixture(scope='module')
def snapshot(tmp_path_factory):
    root=tmp_path_factory.mktemp('original-r3-snapshot');artifact=(F/'extraction.json').read_bytes()
    for role,entry in json.loads(artifact)['files'].items():
        target=root/entry['path'];target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(gzip.decompress((F/(role+'.json.gz')).read_bytes()))
    return load_runtime_snapshot(candidate_root=root,extraction_artifact=artifact,
                                 expected_extraction_sha256=EXTRACTION_SHA)


def actual_inputs(snapshot,cid):
    if cid in ('BT24-003','BT24-031','BT24-083','BT24-090'):
        source=BASE/'runtime_projection';m=json.loads((source/'metadata.json').read_bytes())
        d=m['cards'][cid];observations=json.loads(gzip.decompress((source/(cid+'-observations.json.gz')).read_bytes()))
    else:
        source=BASE/'rule_card_composition';d=json.loads((source/(cid+'.json')).read_bytes())
        observations=d['observations']
    corrections=json.loads(snapshot.file('corrections'));image_dir=BASE/'runtime_projection'
    images={r['image_artifact_path']:(image_dir/(r['image_sha256']+'.webp')).read_bytes()
            for r in corrections['compatibility_corrections']}
    record=snapshot.card(cid);meta=snapshot.metadata()
    source_inputs=dict(canonical_card=record['canonical_card'],aggregate=snapshot.file('aggregate'),
        scopes={'official':snapshot.file('official_scope'),'digimoncard_io':snapshot.file('secondary_scope')},
        observations={k:p.enc(v)for k,v in observations.items()},
        correction_document=snapshot.file('corrections'),condition_selections=snapshot.file('selections'),
        adjudication_ledger=snapshot.file('adjudications'),
        raw_pages={url:gzip.decompress((source/name).read_bytes())for url,name in d['raw_files'].items()},
        old_input_revision=meta['source_revision'],new_input_revision=meta['source_revision'],
        expected_official_universe=meta['official_card_ids'])
    proof_inputs={k:v for k,v in source_inputs.items()if k!='scopes'}
    proof_inputs.update(runtime_record=record['runtime_record'],registry_bytes=record['registry_bytes'],image_artifacts=images)
    binding=p.enc({'schema_version':1,'input_sha256':p.input_hashes(proof_inputs),
                   'producer_sha256':p.producer_dependencies()})
    return dict(snapshot=snapshot,expected_extraction_sha256=EXTRACTION_SHA,
        runtime_record=record['runtime_record'],registry_bytes=record['registry_bytes'],
        runtime_binding=binding,image_artifacts=images,**source_inputs)


@pytest.mark.parametrize('cid',['BT24-003','BT24-031','BT24-083','BT24-090'])
def test_four_original_runtime_kinds(snapshot,cid):
    a=actual_inputs(snapshot,cid);r=compose_runtime_rule_card(**a)
    assert r['status']=='EXACT_RUNTIME_SOURCE_COMPOSITION'
    assert r['original_canonical_bytes'].encode()==snapshot.card(cid)['canonical_card']
    assert r['original_runtime_bytes'].encode()==snapshot.card(cid)['runtime_record']
    assert len(r['claimed_fields'])==19 and {'rarity','card_name_jpn'}<=set(r['unclaimed_runtime_fields'])
    assert r['source_composition']['status']=='COMPOSED_RULE_CARD'
    assert len(r['snapshot']['canonical_record_sha256'])==4481
    assert len(r['snapshot']['runtime_record_sha256'])==4483
    assert len(r['snapshot']['official_card_ids'])==4474


@pytest.mark.parametrize('cid',['BT25-025','EX12-026','EX12-076','P-213'])
def test_followup_four_original_rule_records(snapshot,cid):
    r=compose_runtime_rule_card(**actual_inputs(snapshot,cid))
    assert r['status']=='EXACT_RUNTIME_SOURCE_COMPOSITION'
    assert r['source_composition']['structural_obligations']
    assert len(r['claimed_fields'])==19
