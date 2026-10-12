"""Diagnostic dispatch uses complete original Link and clean Dual inputs."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
from tools.clause_coverage import diagnostic_source_book as book
from test_diagnostic_source_book import files as base_files, inputs as base_inputs
from test_link_card_composition import inputs as link_inputs
from test_clean_dual_composition import inputs as dual_inputs

sha = lambda b: hashlib.sha256(b).hexdigest()

@pytest.fixture(scope='module')
def expanded(base_files):
    a = base_inputs(base_files)
    snapshot = load_runtime_snapshot(candidate_root=a['candidate_root'], extraction_artifact=a['extraction_artifact'],
        expected_extraction_sha256=a['expected_extraction_sha256'])
    m = json.loads(a['input_manifest'])
    m['evaluated_card_ids'] = ['BT21-071', 'BT21-074', 'BT25-085']
    m['authorizations'] = {}
    def artifact(name, raw):
        p = base_files / 'dispatch-extension' / name
        p.parent.mkdir(parents=True, exist_ok=True)
        if p.exists(): assert p.read_bytes() == raw
        else: p.write_bytes(raw)
        return {'path': p.relative_to(base_files).as_posix(), 'sha256': sha(raw), 'bytes': len(raw)}
    original = {cid: link_inputs(snapshot, cid) for cid in ('BT21-071', 'BT21-074')}
    original['BT25-085'] = dual_inputs()
    for cid, source in original.items():
        for oid, raw in source['observations'].items():
            lane = json.loads(raw)['source']
            p = a['candidate_root'] / 'data/card_sources/observations' / lane / (oid + '.json')
            p.parent.mkdir(parents=True, exist_ok=True)
            if p.exists(): assert p.read_bytes() == raw
            else: p.write_bytes(raw)
        for url, raw in source['raw_pages'].items():
            if url in m['raw_pages']:
                assert (base_files / m['raw_pages'][url]['path']).read_bytes() == raw
            else: m['raw_pages'][url] = artifact(sha(url.encode()) + '.html', raw)
        if cid.startswith('BT21'):
            m['authorizations'][cid] = {'link_yaml': artifact(cid + '.yaml', source['yaml_bytes'])}
        else:
            m['authorizations'][cid] = {key: artifact(key + '.json', source[key])
                for key in ('association_proposal', 'association_review', 'association_proof')}
            for name, raw in source['image_artifacts'].items():
                if name in m['images']: assert (base_files / m['images'][name]['path']).read_bytes() == raw
                else: m['images'][name] = artifact(sha(name.encode()) + '.image', raw)
    a['input_manifest'] = enc(m); a['expected_input_manifest_sha256'] = sha(a['input_manifest'])
    return a, snapshot

@pytest.fixture(scope='module')
def actual(expanded):
    return book.build_diagnostic_source_book(**expanded[0])


def test_original_link_and_clean_dual_compose_without_narrowing_inventory(actual):
    assert actual['status'] == 'DIAGNOSTIC_ONLY_NOT_ADMISSION'
    assert len(actual['cards']) == 4484
    assert actual['membership_counts'] == {'official': 4474, 'canonical': 4481, 'runtime': 4483}
    assert actual['full_span_count'] is None and actual['full_gating_interaction_count'] is None
    for cid in ('BT21-074', 'BT25-085'):
        r = actual['cards'][cid]
        assert r['status'] == 'COMPOSED_DIAGNOSTIC', r['blockers']
        assert not r['blockers'] and set(r['runtime_unclaimed']) == {'rarity', 'card_name_jpn'}
        assert r['gating_interaction_count'] is None and r['original_observations']
        assert len(r['source_composition']['inventory']) == len(r['source_composition']['consumption'])
        assert all(not o['executable'] for o in r['structural_obligations'])
    assert actual['cards']['BT1-001']['status'] == 'NOT_EVALUATED'
    assert actual['cards']['LM-063']['status'] == 'BLOCKED'
    assert actual['cards']['TEST-020']['membership']['runtime_only']


def test_original_scopemon_conflict_is_not_overridden_by_yaml(actual):
    r = actual['cards']['BT21-071']
    assert r['status'] == 'BLOCKED' and r['span_count'] is None and r['original_observations']
    assert any('required canonical source conflict' in b.get('reason', '') for b in r['blockers'])


def test_original_clean_dual_proof_and_link_boundaries_preserved(actual):
    d = actual['cards']['BT25-085']['source_composition']
    assert d['input_sha256']['association_proof'] == sha(dual_inputs()['association_proof'])
    assert d['association_resolution']['authority'] == 'reviewed-clean-source-image-association'
    r = actual['cards']['BT21-074']; source = r['source_composition']
    assert source['status'] == 'COMPOSED_LINK_CARD' and len(r['runtime_claimed_fields']) == 20
    assert source['zones']['inherited']['value'] == ''
    assert source['zones']['link']['value'].startswith('[When Linking]')
    assert source['native_compilation_verified'] is False
    assert len([o for o in source['structural_obligations'] if o['role'] == 'dp']) == 1
    assert all('+4000 DP' not in s['span']['text'] for s in r['span_descriptors'])


def dispatch(expanded, cid, auth):
    a, snapshot = expanded
    m = json.loads(a['input_manifest']); root = a['input_root']
    raw_pages = {url: book._file(root, d) for url, d in m['raw_pages'].items()}
    source = book._source_inputs(snapshot, a['candidate_root'], cid, raw_pages)
    images = {name: book._file(root, d) for name, d in m['images'].items()}
    return book._dispatch(snapshot, a['expected_extraction_sha256'], source, images, auth, root)


@pytest.mark.parametrize('cid, change', [
    ('BT21-074', 'missing'), ('BT21-074', 'extra'), ('BT21-074', 'hash'),
    ('BT21-074', 'bytes_bool'), ('BT21-074', 'path'), ('BT21-074', 'cost'),
    ('BT25-085', 'missing'), ('BT25-085', 'extra'), ('BT25-085', 'hash'),
    ('BT25-085', 'bytes_bool'), ('BT25-085', 'path'), ('BT25-085', 'proof_content')])
def test_dispatch_capability_bytes_and_proofs_are_required(expanded, cid, change):
    a, _ = expanded; m = json.loads(a['input_manifest']); auth = copy.deepcopy(m['authorizations'][cid])
    key = 'link_yaml' if cid.startswith('BT21') else 'association_proof'
    if change == 'missing': auth.pop(key)
    elif change == 'extra': auth['validated'] = True
    elif change == 'hash': auth[key]['sha256'] = '0' * 64
    elif change == 'bytes_bool': auth[key]['bytes'] = True
    elif change == 'path': auth[key]['path'] = '../outside'
    else:
        raw = book._file(a['input_root'], auth[key])
        if change == 'cost':
            assert b'kind: link_condition\n    cost: 3' in raw
            raw = raw.replace(b'kind: link_condition\n    cost: 3', b'kind: link_condition\n    cost: 4')
        else: raw += b' '
        path = a['input_root'] / ('dispatch-extension/mutated-' + change)
        path.write_bytes(raw)
        auth[key] = {'path': path.relative_to(a['input_root']).as_posix(), 'sha256': sha(raw), 'bytes': len(raw)}
    with pytest.raises((ValueError, TypeError, OSError)):
        dispatch(expanded, cid, auth)


def test_dispatch_dependency_inventory_includes_new_checks():
    assert {'tools.clause_coverage.link_card_composition', 'tools.clause_coverage.link_yaml_binding',
        'tools.clause_coverage.clean_dual_association'} <= set(book.diagnostic_dependencies())
