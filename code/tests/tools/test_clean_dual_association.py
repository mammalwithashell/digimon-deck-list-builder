"""Actual BT25-085 source-only proof; no runtime/compositor approval."""
import copy
import gzip
import hashlib
import json
from pathlib import Path

import pytest

F = Path(__file__).parents[1] / 'fixtures/card_data/clean_dual_association'
enc = lambda value: json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2).encode()


def inputs():
    data = json.loads((F / 'inputs.json').read_bytes())
    proposal = (F / 'proposal.json').read_bytes()
    return dict(
        aggregate=gzip.decompress((F / 'aggregate.json.gz').read_bytes()),
        scopes={s: gzip.decompress((F / (s + '-scope.json.gz')).read_bytes())
                for s in ('official', 'digimoncard_io')},
        observations={k: enc(v) for k, v in data['observations'].items()},
        raw_pages={u: gzip.decompress((F / n).read_bytes()) for u, n in data['raw_files'].items()},
        images={p.name: p.read_bytes() for p in F.glob('*.webp')},
        correction_document=gzip.decompress((F / 'corrections.json.gz').read_bytes()),
        proposal=proposal, expected_proposal_sha256=hashlib.sha256(proposal).hexdigest(),
        expected_source_revision=json.loads(proposal)['source_revision'],
        expected_official_universe=data['expected_official_universe'])


def prove(value):
    from tools.clause_coverage.clean_dual_association import prove_clean_dual_association
    return prove_clean_dual_association(**value)


def test_actual_three_printings_preserve_every_source_and_face():
    args = inputs()
    before = copy.deepcopy(args)
    result = prove(args)
    assert args == before
    assert result['status'] == 'PROOF_ONLY_PENDING_INDEPENDENT_REVIEW'
    assert result['approved'] is False
    assert len(result['associations']) == 3
    assert len(result['source_observations']) == 6
    assert result['authority'] == 'proposed-clean-dual-image-association'
    assert all(not d['blockers'] for d in result['original_diagnostics'])
    assert result['runtime_condition_registry_validation'] == 'NOT_PERFORMED_REQUIRED_BEFORE_COMPOSITION'


@pytest.mark.parametrize('change', [
    'image', 'missing_image', 'swap_images', 'raw', 'missing_observation',
    'foreign_observation', 'scope', 'universe', 'revision', 'pin',
    'bool_version', 'unknown_field', 'omit_printing', 'duplicate_printing',
    'wrong_kind', 'wrong_face', 'sections', 'image_url', 'printing', 'status',
    'bool_scope', 'bool_expected', 'secondary_parser', 'aggregate_extra',
    'unknown_face_section', 'missing_raw', 'extra_image', 'duplicate_json',
    'unknown_correction_schema', 'bool_correction_schema', 'target_correction',
])
def test_refuse_incomplete_or_unbound_inputs(change):
    args = inputs()
    p = json.loads(args['proposal'])
    row = p['observations'][0]
    if change == 'image': args['images'][row['image_artifact']] += b'x'
    elif change == 'missing_image': args['images'].pop(row['image_artifact'])
    elif change == 'swap_images': args['images'][row['image_artifact']] = args['images'][p['observations'][1]['image_artifact']]
    elif change == 'raw': args['raw_pages'][row['source_url']] += b'x'
    elif change == 'missing_observation': args['observations'].pop(row['observation_id'])
    elif change == 'foreign_observation': args['observations']['0' * 64] = args['observations'][row['observation_id']]
    elif change in ('scope', 'bool_scope', 'bool_expected'):
        s = json.loads(args['scopes']['official'])
        s[{'scope': 'scope', 'bool_scope': 'schema_version', 'bool_expected': 'expected_count'}[change]] = 'partial' if change == 'scope' else True
        args['scopes']['official'] = enc(s)
    elif change == 'universe': args['expected_official_universe'] = args['expected_official_universe'][:-1]
    elif change == 'revision': args['expected_source_revision'] = '0' * 64
    elif change == 'pin': args['expected_proposal_sha256'] = '0' * 64
    elif change == 'bool_version': p['version'] = True
    elif change == 'unknown_field': p['validated'] = True
    elif change == 'omit_printing': p['observations'].pop()
    elif change == 'duplicate_printing': p['observations'].append(copy.deepcopy(row))
    elif change == 'wrong_kind': row['raw_kind'] = 'Digimon/Tamer'
    elif change == 'wrong_face': row['face_identity'].reverse()
    elif change == 'sections': row['sections_sha256'] = '0' * 64
    elif change == 'image_url': row['source_image_url'] = 'https://invalid/image'
    elif change == 'printing': row['printing_id'] = 'foreign'
    elif change == 'status': p['status'] = 'APPROVED'
    elif change == 'secondary_parser':
        s = json.loads(args['scopes']['digimoncard_io']); s['parser_version'] = 'foreign'; args['scopes']['digimoncard_io'] = enc(s)
    elif change == 'aggregate_extra':
        s = json.loads(args['aggregate']); s['extra'] = True; args['aggregate'] = enc(s)
    elif change == 'unknown_face_section':
        o = json.loads(args['observations'][row['observation_id']]); o['extracted']['faces'][1]['text_sections'].append({'label': 'Unknown', 'text': 'Keep me'}); args['observations'][row['observation_id']] = enc(o)
    elif change == 'missing_raw': args['raw_pages'].pop(row['source_url'])
    elif change == 'extra_image': args['images']['unowned.webp'] = b'not owned'
    elif change == 'duplicate_json': args['proposal'] = b'{"version":1,"version":1}'
    elif change in ('unknown_correction_schema', 'bool_correction_schema', 'target_correction'):
        s = json.loads(args['correction_document'])
        if change == 'target_correction': s['corrections'][0]['card_id'] = 'BT25-085'
        else: s['schema_version'] = True if change == 'bool_correction_schema' else 999
        args['correction_document'] = enc(s)
    if change != 'duplicate_json': args['proposal'] = enc(p)
    if change != 'pin': args['expected_proposal_sha256'] = hashlib.sha256(args['proposal']).hexdigest()
    with pytest.raises(ValueError): prove(args)


def test_existing_mismatch_helper_does_not_accept_clean_case():
    from tools.clause_coverage.dual_kind_association import prove_dual_kind_association
    args = inputs(); args.pop('correction_document')
    p = json.loads(args['proposal']); p['kind'] = 'dual-image-kind-association-proposal'
    args['proposal'] = enc(p); args['expected_proposal_sha256'] = hashlib.sha256(args['proposal']).hexdigest()
    with pytest.raises(ValueError, match='kind-association'):
        prove_dual_kind_association(**args)


def rebind(args, mutate):
    """Rebuild source identities, scopes and proposal after deliberate mutation."""
    from tools.card_data import models, serialization, source_manifest
    proposal = json.loads(args['proposal'])
    mapping = {}
    observations = {}
    for old, raw in args['observations'].items():
        row = json.loads(raw)
        mutate(row)
        typed = models.SourceObservation(**{**row, 'source': models.SourceName(row['source'])})
        new = serialization.observation_digest(typed)
        row['observation_id'] = new
        mapping[old] = new
        observations[new] = enc(row)
    args['observations'] = observations
    scopes = []
    for name, raw in args['scopes'].items():
        scope = json.loads(raw)
        scope['observation_ids'] = [mapping.get(v, v) for v in scope['observation_ids']]
        for ref in scope['observation_refs']:
            ref['observation_id'] = mapping.get(ref['observation_id'], ref['observation_id'])
        args['scopes'][name] = enc(scope)
        scopes.append(models.ScopeManifest(**{**scope, 'source': models.SourceName(name),
            'observation_refs': tuple(models.ScopeObservationRef(**v) for v in scope['observation_refs'])}))
    aggregate = source_manifest.build_full_pool_source_manifest(scopes).to_dict()
    args['aggregate'] = enc(aggregate)
    args['expected_source_revision'] = proposal['source_revision'] = aggregate['manifest_revision']
    for row in proposal['observations']:
        row['observation_id'] = mapping[row['observation_id']]
        obs = json.loads(observations[row['observation_id']])
        row['raw_sha256'] = obs['raw_payload_digest']
        row['faces_sha256'] = hashlib.sha256(enc(obs['extracted']['faces'])).hexdigest()
        row['sections_sha256'] = hashlib.sha256(enc(obs['extracted']['text_sections'])).hexdigest()
    args['proposal'] = enc(proposal)
    args['expected_proposal_sha256'] = hashlib.sha256(args['proposal']).hexdigest()


def test_freshly_bound_required_canonical_conflict_refuses():
    args = inputs()
    first = json.loads(args['proposal'])['observations'][0]['observation_id']
    def change(row):
        if row['observation_id'] == first:
            row['extracted']['name'] = 'Foreign name'
    rebind(args, change)
    with pytest.raises(ValueError, match='required canonical conflict'):
        prove(args)


def test_freshly_bound_hidden_popup_refuses():
    args = inputs()
    proposal = json.loads(args['proposal'])
    url = proposal['observations'][0]['source_url']
    printing = proposal['observations'][0]['printing_id']
    raw = args['raw_pages'][url]
    needle = ('id="' + printing + '"').encode()
    assert raw.count(needle) == 1
    args['raw_pages'][url] = raw.replace(needle, needle + b' hidden', 1)
    def change(row):
        if row['source'] == 'official' and row['source_url'] == url:
            row['raw_payload_digest'] = hashlib.sha256(args['raw_pages'][url]).hexdigest()
    rebind(args, change)
    with pytest.raises(ValueError, match='hidden or styled popup'):
        prove(args)
