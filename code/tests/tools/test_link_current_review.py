"""Original full R4 inputs exercise current versus retained historical review."""
import copy
import gzip
import hashlib
import json
from pathlib import Path

import pytest

from tools.card_data.errors import CardDataError
from tools.card_data.serialization import canonical_json_bytes as enc
from tools.clause_coverage.link_card_composition import compose_link_card
from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot

F = Path(__file__).parents[1] / 'fixtures/card_data/link_current_review'
sha = lambda raw: hashlib.sha256(raw).hexdigest()
MAIN = 'effect_description_eng'
REVIEW = 'focused99-bt21-071-main-12de907a3b103cafcea3'


@pytest.fixture(scope='module')
def snapshot(tmp_path_factory):
    root = tmp_path_factory.mktemp('original-r4-link')
    raw = (F / 'extraction.json').read_bytes()
    manifest = json.loads((F / 'manifest.json').read_bytes())
    assert sha(raw) == manifest['extraction_sha256']
    for role, row in json.loads(raw)['files'].items():
        fixture = manifest['containers'][role]
        compressed = (F.parent / fixture['file']).read_bytes()
        assert sha(compressed) == fixture['compressed_sha256']
        original = gzip.decompress(compressed)
        assert sha(original) == row['sha256'] and len(original) == row['bytes']
        dest = root / row['path']
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(original)
    return load_runtime_snapshot(candidate_root=root, extraction_artifact=raw,
                                 expected_extraction_sha256=sha(raw))


def inputs(snapshot, cid='BT21-071'):
    card = snapshot.card(cid)
    row = json.loads((F / 'manifest.json').read_bytes())['cards'][cid]
    def read(item, compressed=True):
        raw = (F / item['file']).read_bytes()
        raw = gzip.decompress(raw) if compressed else raw
        assert sha(raw) == item['sha256']
        return raw
    meta = snapshot.metadata()
    return dict(snapshot=snapshot, expected_extraction_sha256=meta['extraction_artifact_sha256'],
        runtime_record=card['runtime_record'], registry_bytes=card['registry_bytes'],
        canonical_card=card['canonical_card'], aggregate=snapshot.file('aggregate'),
        scopes={source:snapshot.file(role) for source,role in
                [('official','official_scope'),('digimoncard_io','secondary_scope')]},
        observations={oid:read(item) for oid,item in row['observations'].items()},
        raw_pages={url:read(item) for url,item in row['raw_pages'].items()},
        adjudication_ledger=snapshot.file('adjudications'),
        correction_document=snapshot.file('corrections'),
        condition_selections=snapshot.file('selections'),
        old_input_revision=meta['source_revision'], new_input_revision=meta['source_revision'],
        expected_official_universe=meta['official_card_ids'],
        yaml_bytes=read(row['yaml'], False), expected_yaml_sha256=row['yaml']['sha256'])


def rebind(snapshot, root, role, mutate):
    """Adversarial container pin; it supplies no review authority."""
    artifact = json.loads((F / 'extraction.json').read_bytes())
    for name,row in artifact['files'].items():
        raw = snapshot.file(name)
        if name == role:
            value = json.loads(raw)
            mutate(value)
            raw = enc(value)
            row.update(sha256=sha(raw), bytes=len(raw))
            if name in ('canonical','runtime'):
                records = value['cards'] if name == 'canonical' else value
                artifact[name + '_record_sha256'] = {cid:sha(enc(v)) for cid,v in records.items()}
        dest = root / row['path']
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(raw)
    raw = enc(artifact)
    return load_runtime_snapshot(candidate_root=root, extraction_artifact=raw,
                                 expected_extraction_sha256=sha(raw))


def test_current_exact_main_review_composes_with_all_alternatives_retained(snapshot):
    args = inputs(snapshot)
    result = compose_link_card(**args)
    assert result['status'] == 'COMPOSED_LINK_CARD'
    assert len(result['claimed_fields']) == 20
    canonical = json.loads(args['canonical_card'])
    assert canonical['provenance'][MAIN]['adjudicated'] is True
    assert any(c['field_path'] == MAIN for c in canonical['conflicts'])
    assert result['original_canonical_bytes'].encode() == args['canonical_card']
    assert result['original_runtime_bytes'].encode() == args['runtime_record']
    assert result['zones']['effect']['value'] == canonical['fields'][MAIN]
    assert 'as 1 of' in result['zones']['effect']['value']
    assert any('as any of' in row['text'] for row in result['inventory'])
    originals = [(oid,i,s['label'],s['text']) for oid,raw in args['observations'].items()
        if (o := json.loads(raw))['source'] == 'official'
        for i,s in enumerate(o['extracted']['text_sections'])]
    actual = [(r['observation_id'],r['section_index'],r['label'],r['text']) for r in result['inventory']]
    assert sorted(actual) == sorted(originals)
    assert len(result['consumption']) == len(originals)
    assert result['native_compilation_verified'] is False


def test_other_link_card_still_uses_original_full_r4_inputs(snapshot):
    assert compose_link_card(**inputs(snapshot, 'BT21-074'))['status'] == 'COMPOSED_LINK_CARD'


@pytest.mark.parametrize('change', ['missing','stale','wrong_value','wrong_digest',
    'foreign_card','foreign_field','candidate_membership','selected_observation'])
def test_new_container_pin_and_adjudicated_flag_cannot_approve_invalid_review(snapshot, tmp_path, change):
    def mutate(doc):
        row = next(r for r in doc['adjudications'] if r['adjudication_id'] == REVIEW)
        if change == 'missing': doc['adjudications'].remove(row)
        elif change == 'stale': row['new_input_revision'] = '0' * 64
        elif change == 'wrong_value': row['new_value'] += ' Draw 9.'
        elif change == 'wrong_digest': row['old_digest'] = '0' * 64
        elif change == 'foreign_card': row['card_id'] = 'BT21-074'
        elif change == 'foreign_field': row['field_path'] = 'inherited_effect_description_eng'
        elif change == 'candidate_membership': row['provenance_observation_ids'].pop()
        else: row['selected_observation_id'] = '0' * 64
    changed = rebind(snapshot, tmp_path, 'adjudications', mutate)
    with pytest.raises((ValueError, CardDataError)):
        compose_link_card(**inputs(changed))


@pytest.mark.parametrize('change', ['derived','authority','observation_membership'])
def test_forged_canonical_provenance_is_not_review(snapshot, tmp_path, change):
    def mutate(doc):
        p = doc['cards']['BT21-071']['provenance'][MAIN]
        if change == 'derived': p['derived'] = True
        elif change == 'authority': p['authority'] = 'digimoncard_io'
        else: p['observation_ids'].pop()
    changed = rebind(snapshot, tmp_path, 'canonical', mutate)
    with pytest.raises((ValueError, CardDataError)):
        compose_link_card(**inputs(changed))


@pytest.mark.parametrize('key', ['validated','approved','review_status','_runtime_validator'])
def test_public_flags_cannot_supply_review(snapshot, key):
    args = inputs(snapshot)
    args[key] = True
    with pytest.raises((ValueError, CardDataError)):
        compose_link_card(**args)
