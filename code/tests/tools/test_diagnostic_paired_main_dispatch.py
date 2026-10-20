"""Paired main/face corrections retain whole original R4 source evidence."""
import copy
import json

import pytest

from tools.card_data.serialization import canonical_json_bytes as enc
from tools.clause_coverage import diagnostic_source_book as book
from test_diagnostic_source_book import files as base_files, inputs as base_inputs
from test_link_current_review import snapshot, F as R4, sha
from test_paired_main_card_composition import inputs as paired_inputs


@pytest.fixture(scope='module')
def expanded(base_files, snapshot):
    a = base_inputs(base_files)
    raw = (R4 / 'extraction.json').read_bytes()
    for role,row in json.loads(raw)['files'].items():
        dest = a['candidate_root'] / row['path']
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(snapshot.file(role))
    a.update(extraction_artifact=raw, expected_extraction_sha256=sha(raw))
    m = json.loads(a['input_manifest'])
    m.update(snapshot_sha256=sha(raw), evaluated_card_ids=['EX1-066'], authorizations={})
    source = paired_inputs()
    def artifact(name, raw):
        path = base_files / 'paired-main-dispatch' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if path.exists():
            assert path.read_bytes() == raw
        else:
            path.write_bytes(raw)
        return {'path':path.relative_to(base_files).as_posix(), 'sha256':sha(raw), 'bytes':len(raw)}
    for oid,raw in source['observations'].items():
        lane = json.loads(raw)['source']
        path = a['candidate_root'] / 'data/card_sources/observations' / lane / (oid + '.json')
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
    for url,raw in source['raw_pages'].items():
        m['raw_pages'][url] = artifact(sha(raw) + '.html', raw)
    for name,raw in source['image_artifacts'].items():
        m['images'][name] = artifact(sha(raw) + '.image', raw)
    sidecar = json.loads(source['correction_sidecar'])
    names = [row['artifact_name'] for row in sidecar['images']]
    m['authorizations']['EX1-066'] = {
        'paired_main_sidecar':artifact('sidecar.json', source['correction_sidecar']),
        'paired_main_image_1':artifact('main-image-1.png', source['correction_images'][names[0]]),
        'paired_main_image_2':artifact('main-image-2.png', source['correction_images'][names[1]])}
    a.update(input_manifest=enc(m), expected_input_manifest_sha256=sha(enc(m)))
    return a,snapshot


@pytest.fixture(scope='module')
def actual(expanded):
    return book.build_diagnostic_source_book(**expanded[0])


def test_actual_whole_r4_source_and_population_conserved(actual):
    row = actual['cards']['EX1-066']
    assert row['status'] == 'COMPOSED_DIAGNOSTIC', row['blockers']
    assert row['source_composition']['status'] == 'COMPOSED_PAIRED_MAIN_CARD'
    assert len(actual['cards']) == 4484 and len(row['original_observations']) == 14
    assert len(row['runtime_claimed_fields']) == 19
    assert set(row['runtime_unclaimed']) == {'rarity','card_name_jpn'}
    assert actual['full_span_count'] is None and actual['full_gating_interaction_count'] is None
    source = row['source_composition']
    assert row['conservation']['occurrences'] == len(source['inventory']) == len(source['consumption'])
    assert len([r for r in source['consumption'] if r['disposition'] == 'superseded-by-reviewed-correction']) == 7
    assert len(source['zones']['effect']['unit_indices']) == 1
    assert all(o['executable'] is False for o in row['structural_obligations'])
    assert any(o['kind'] == 'paired-main-face-consistency' for o in row['structural_obligations'])
    assert actual['cards']['BT1-001']['status'] == 'NOT_EVALUATED'
    assert actual['status'] == 'DIAGNOSTIC_ONLY_NOT_ADMISSION'


def dispatch(expanded, auth):
    a,snapshot = expanded
    root = a['input_root']
    manifest = json.loads(a['input_manifest'])
    source = book._source_inputs(snapshot, a['candidate_root'], 'EX1-066',
        {url:book._file(root,d) for url,d in manifest['raw_pages'].items()})
    return book._dispatch(snapshot, a['expected_extraction_sha256'], source,
        {name:book._file(root,d) for name,d in manifest['images'].items()}, auth, root)


@pytest.mark.parametrize('change', ['missing','extra','one_image','swapped_images',
    'sidecar_hash','image_hash','boolean_size','unsafe_path','old_sidecar'])
def test_exact_reviewed_sidecar_and_both_image_authorizations_required(expanded, change):
    a,_ = expanded
    auth = copy.deepcopy(json.loads(a['input_manifest'])['authorizations']['EX1-066'])
    if change == 'missing': auth = {}
    elif change == 'extra': auth['validated'] = True
    elif change == 'one_image': auth.pop('paired_main_image_2')
    elif change == 'swapped_images':
        auth['paired_main_image_1'],auth['paired_main_image_2'] = auth['paired_main_image_2'],auth['paired_main_image_1']
    elif change == 'sidecar_hash': auth['paired_main_sidecar']['sha256'] = '0'*64
    elif change == 'image_hash': auth['paired_main_image_1']['sha256'] = '0'*64
    elif change == 'boolean_size': auth['paired_main_image_1']['bytes'] = True
    elif change == 'unsafe_path': auth['paired_main_sidecar']['path'] = '../outside'
    else:
        from test_paired_main_card_composition import F
        raw = (F / 'sidecar-APPROVED.json').read_bytes()
        path = a['input_root'] / 'paired-main-dispatch' / 'historical-sidecar.json'
        path.write_bytes(raw)
        auth['paired_main_sidecar'] = {'path':path.relative_to(a['input_root']).as_posix(),
                                     'sha256':sha(raw), 'bytes':len(raw)}
    with pytest.raises((ValueError, TypeError, OSError)):
        dispatch(expanded, auth)


def test_dependency_inventory_includes_whole_paired_proof_and_composer():
    assert {'tools.clause_coverage.paired_main_correction_units',
            'tools.clause_coverage.paired_main_card_composition'} <= set(book.diagnostic_dependencies())


def test_unknown_sourceless_kind_remains_refused(actual):
    source = copy.deepcopy(actual['cards']['EX1-066']['source_composition'])
    index = source['zones']['effect']['unit_indices'][0]
    source['units'][index]['kind'] = 'unreviewed-image-correction'
    with pytest.raises(ValueError, match='unknown sourceless unit'):
        book._descriptors('EX1-066', source)
