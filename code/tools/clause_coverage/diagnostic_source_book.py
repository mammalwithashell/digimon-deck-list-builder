"""Bounded source-composition inventory, never a clause book or admission artifact.

Input pins must come from reviewed configuration. Replaying hash-pinned source
bytes proves consistency, not the truth of newly invented source observations.
No submitted composition, success flag, legacy clause ID or verdict is accepted.
"""
from copy import deepcopy
import ast
from pathlib import Path, PurePosixPath

from tools.card_data.serialization import canonical_json_bytes as enc
from .runtime_snapshot import load_runtime_snapshot
from . import runtime_projection_proof as projection
from .runtime_rule_composition import compose_runtime_rule_card, wrapper_dependencies
from .runtime_empty_condition import compose_empty_condition_runtime_rule_card
from .dual_card_composition import compose_dual_card
from .corrected_rule_card_composition import compose_corrected_rule_card, checker_dependencies
from .link_card_composition import compose_link_card
from .assembly_card_composition import compose_assembly_card
from .paired_main_card_composition import compose_paired_main_card, checker_dependencies as paired_dependencies

_json = projection._json
_sha = projection._sha
_require = projection._require
_METADATA = {'rarity', 'card_name_jpn'}


def diagnostic_dependencies():
    """Hash the actual static local import closure, including all dispatchers."""
    root = Path(__file__).resolve().parents[2]
    pending, result = [__name__], {}
    while pending:
        name = pending.pop()
        if name in result or not (name == 'tools' or name.startswith('tools.')):
            continue
        path = root / Path(*name.split('.'))
        path = path.with_suffix('.py') if path.with_suffix('.py').is_file() else path / '__init__.py'
        if not path.is_file():
            continue
        raw = path.read_bytes()
        result[name] = _sha(raw)
        package = name if path.name == '__init__.py' else name.rpartition('.')[0]
        pieces = name.split('.')
        pending.extend('.'.join(pieces[:i]) for i in range(1, len(pieces)))
        for node in ast.walk(ast.parse(raw)):
            if isinstance(node, ast.Import):
                pending.extend(alias.name for alias in node.names)
            elif isinstance(node, ast.ImportFrom):
                base = node.module or ''
                if node.level:
                    prefix = package.split('.')[:len(package.split('.')) - node.level + 1]
                    base = '.'.join(prefix + ([base] if base else []))
                pending.append(base)
                pending.extend(base + '.' + alias.name for alias in node.names if alias.name != '*')
    return dict(sorted(result.items()))


def _file(root, descriptor):
    _require(isinstance(descriptor, dict) and set(descriptor) == {'path', 'sha256', 'bytes'},
             'exact file descriptor required')
    name = descriptor['path']
    _require(isinstance(name, str) and name and '\\' not in name and ':' not in name
             and not PurePosixPath(name).is_absolute()
             and all(p not in ('', '.', '..') for p in name.split('/')), 'unsafe input path')
    path = (root / name).resolve(strict=True)
    _require(path.is_relative_to(root), 'input escaped pinned root')
    raw = path.read_bytes()
    _require(type(descriptor['bytes']) is int and len(raw) == descriptor['bytes']
             and _sha(raw) == descriptor['sha256'], 'input file bytes changed: ' + name)
    return raw


def _source_inputs(snapshot, candidate_root, cid, raw_pages):
    meta = snapshot.metadata()
    aggregate = _json(snapshot.file('aggregate'))
    observations = {}
    for source, lane in aggregate['sources'].items():
        for ref in lane['observation_refs']:
            if ref['gameplay_card_id'] != cid:
                continue
            oid = ref['observation_id']
            _require(isinstance(oid, str) and len(oid) == 64
                     and all(c in '0123456789abcdef' for c in oid), 'invalid observation identity')
            path = (candidate_root / 'data/card_sources/observations' / source / (oid + '.json')).resolve(strict=True)
            _require(path.is_relative_to(candidate_root), 'escaped observation path')
            _require(oid not in observations, 'duplicate observation identity')
            observations[oid] = path.read_bytes()
    required_urls = {_json(raw)['source_url'] for raw in observations.values()
                     if _json(raw)['source'] == 'official'}
    _require(required_urls <= set(raw_pages), 'missing same-card official raw page')
    record = snapshot.card(cid)
    return dict(aggregate=snapshot.file('aggregate'),
        scopes={'official': snapshot.file('official_scope'), 'digimoncard_io': snapshot.file('secondary_scope')},
        observations=observations, canonical_card=record['canonical_card'],
        adjudication_ledger=snapshot.file('adjudications'),
        raw_pages={url: raw_pages[url] for url in sorted(required_urls)},
        correction_document=snapshot.file('corrections'), condition_selections=snapshot.file('selections'),
        old_input_revision=meta['source_revision'], new_input_revision=meta['source_revision'],
        expected_official_universe=meta['official_card_ids'])


def _dispatch(snapshot, pin, inputs, images, authorization, input_root):
    _require(isinstance(authorization, dict), 'capability authorization object required')
    cid = _json(inputs['canonical_card'])['card_id']
    record = snapshot.card(cid)
    card = _json(record['canonical_card'])
    corrections = [r for r in _json(inputs['correction_document'])['corrections'] if r['card_id'] == cid]
    if len(card['faces']) > 1:
        required = {'association_proposal', 'association_review'}
        _require(required <= set(authorization), 'Dual review inputs required')
        proposal = _file(input_root, authorization['association_proposal'])
        proposal_object = _json(proposal)
        _require(isinstance(proposal_object, dict), 'Dual proposal object required')
        clean = proposal_object.get('kind') == 'clean-dual-image-association-proposal'
        if clean:
            required.add('association_proof')
        _require(set(authorization) == required, 'exact Dual review inputs required')
        proof = _file(input_root, authorization['association_proof']) if clean else None
        result = compose_dual_card(**inputs, runtime_projection=record['runtime_record'], image_artifacts=images,
            association_proposal=proposal, association_proof=proof,
            association_pin=authorization['association_proposal']['sha256'],
            association_review=_file(input_root, authorization['association_review']),
            review_pin=authorization['association_review']['sha256'],
            runtime_snapshot=snapshot.file('runtime'), runtime_snapshot_pin=_sha(snapshot.file('runtime')),
            registry=record['registry_bytes'], registry_pin=_sha(record['registry_bytes']))
        return result, result['claimed_runtime_fields'], result
    if corrections:
        paired = {'paired_main_sidecar', 'paired_main_image_1', 'paired_main_image_2'}
        if set(authorization) & paired:
            _require(set(authorization) == paired, 'exact paired-main review inputs required')
            sidecar = _file(input_root, authorization['paired_main_sidecar'])
            sidecar_object = _json(sidecar)
            _require(isinstance(sidecar_object, dict), 'paired-main sidecar object required')
            associations = sidecar_object.get('images')
            _require(isinstance(associations, list) and len(associations) == 2
                and all(isinstance(row, dict) and isinstance(row.get('artifact_name'), str)
                        and row['artifact_name'] for row in associations)
                and len({row['artifact_name'] for row in associations}) == 2,
                'two distinct reviewed image associations required')
            a = dict(inputs, runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'],
                image_artifacts=images, correction_sidecar=sidecar,
                expected_reviewed_sidecar_sha256=authorization['paired_main_sidecar']['sha256'],
                correction_images={row['artifact_name']:_file(input_root, authorization[f'paired_main_image_{i+1}'])
                                   for i,row in enumerate(associations)})
            a['projection_binding'] = enc({'schema_version': 1, 'input_sha256': projection.input_hashes(a),
                'producer_sha256': projection.producer_dependencies(), 'checker_sha256': paired_dependencies()})
            result = compose_paired_main_card(**a)
            return result, result['runtime_projection']['claimed_fields'], result
        _require(set(authorization) == {'correction_sidecar', 'correction_image'}, 'correction review inputs required')
        a = dict(inputs, runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'],
            image_artifacts=images, correction_sidecar=_file(input_root, authorization['correction_sidecar']),
            expected_reviewed_sidecar_sha256=authorization['correction_sidecar']['sha256'],
            correction_image_bytes=_file(input_root, authorization['correction_image']))
        a['projection_binding'] = enc({'schema_version': 1, 'input_sha256': projection.input_hashes(a),
            'producer_sha256': projection.producer_dependencies(), 'checker_sha256': checker_dependencies()})
        result = compose_corrected_rule_card(**a)
        return result, result['runtime_projection']['claimed_fields'], result
    if (card['fields'].get('link_effect_description_eng')
            or _json(record['runtime_record']).get('link_effect_description_eng')):
        _require(set(authorization) == {'link_yaml'}, 'exact Link YAML input required')
        result = compose_link_card(**inputs, snapshot=snapshot, expected_extraction_sha256=pin,
            runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'],
            yaml_bytes=_file(input_root, authorization['link_yaml']),
            expected_yaml_sha256=authorization['link_yaml']['sha256'])
        return result, result['claimed_fields'], result
    if 'digixros_costs' in _json(record['runtime_record']) or 'assembly_yaml' in authorization:
        _require(set(authorization) == {'assembly_yaml'}, 'exact Assembly YAML authorization required')
        proof_inputs = {k: v for k, v in inputs.items() if k != 'scopes'}
        proof_inputs.update(runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'], image_artifacts=images)
        binding = enc({'schema_version': 1, 'input_sha256': projection.input_hashes(proof_inputs),
                       'producer_sha256': projection.producer_dependencies()})
        result = compose_assembly_card(**inputs, snapshot=snapshot, expected_extraction_sha256=pin,
            runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'],
            runtime_binding=binding, image_artifacts=images,
            yaml_bytes=_file(input_root, authorization['assembly_yaml']),
            expected_yaml_sha256=authorization['assembly_yaml']['sha256'])
        return result['source_composition'], result['claimed_fields'], result
    _require(not authorization, 'unexpected capability authorization')
    proof_inputs = {k: v for k, v in inputs.items() if k != 'scopes'}
    proof_inputs.update(runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'], image_artifacts=images)
    binding = enc({'schema_version': 1, 'input_sha256': projection.input_hashes(proof_inputs),
                   'producer_sha256': projection.producer_dependencies()})
    composer = (compose_empty_condition_runtime_rule_card
                if _json(record['runtime_record']).get('xros_req') == '' else compose_runtime_rule_card)
    result = composer(**inputs, snapshot=snapshot, expected_extraction_sha256=pin,
        runtime_record=record['runtime_record'], registry_bytes=record['registry_bytes'],
        runtime_binding=binding, image_artifacts=images)
    return result['source_composition'], result['claimed_fields'], result


def _descriptors(cid, source):
    """Verify occurrence coverage and preserve original units, spans and roles."""
    inventory, consumption, units = source['inventory'], source['consumption'], source['units']
    identity = lambda row: (row['observation_id'], row['section_index'])
    by_key = {identity(row): row for row in inventory}
    _require(len(by_key) == len(inventory) and len(consumption) == len(inventory)
             and {identity(row) for row in consumption} == set(by_key), 'source occurrence conservation failed')
    seen = set()
    for row in consumption:
        _require(all(row[k] == value for k, value in by_key[identity(row)].items()), 'source container changed')
        for index in row.get('unit_indices', []) + row.get('replacement_unit_indices', []):
            _require(type(index) is int and 0 <= index < len(units), 'foreign unit reference')
            seen.add(index)
    _require(seen == set(range(len(units))), 'unaccounted composed unit')
    unit_rows, spans = [], []
    obligations = deepcopy(source.get('structural_obligations', []))
    for index, unit in enumerate(units):
        if 'observation_id' in unit:
            key = [cid, 'official-occurrence', unit['observation_id'], unit['section_index'], index]
            _require((unit['observation_id'], unit['section_index']) in by_key, 'foreign unit source')
        else:
            _require(unit.get('kind') in {'reviewed-canonical-image-correction',
                                         'reviewed-paired-main-image-correction'}, 'unknown sourceless unit')
            key = [cid, 'reviewed-correction', unit['sidecar_sha256'], index]
        face = unit.get('face_id', 'primary')
        unit_rows.append({'key': key, 'unit_index': index, 'face': face, 'source_unit': deepcopy(unit)})
        if unit.get('role') == 'arts-reminder':
            _require(not unit['spans'], 'structural Arts reminder must not execute')
            obligations.append({'purpose': 'arts-reminder', 'executable': False, 'unit_key': key,
                                'source_unit': deepcopy(unit)})
        for local, span in enumerate(unit.get('spans', [])):
            spans.append({'unit_key': key, 'face': face, 'zone': unit.get('role'),
                          'occurrence_span_index': local, 'span': deepcopy(span)})
    for zone, row in source.get('zones', {}).items():
        for span in row.get('spans', []):
            index = span['unit_index']
            _require(type(index) is int and 0 <= index < len(units), 'span references missing unit')
            spans.append({'unit_key': unit_rows[index]['key'], 'face': unit_rows[index]['face'], 'zone': zone,
                          'occurrence_span_index': span['span_index'], 'span': deepcopy(span)})
    span_groups = {}
    for row in spans:
        index = row['occurrence_span_index']
        _require(type(index) is int and index >= 0, 'invalid local span index')
        key = (tuple(row['unit_key']), row['zone'])
        span_groups.setdefault(key, []).append(index)
    _require(all(indices == list(range(len(indices))) for indices in span_groups.values()),
             'duplicate, missing or reordered local span occurrence')
    _require(all(o.get('executable') is False for o in obligations), 'structural obligation became executable')
    return unit_rows, spans, obligations, {'occurrences': len(inventory), 'units': len(units),
        'spans': len(spans), 'structural_obligations': len(obligations)}


def build_diagnostic_source_book(*, candidate_root, input_root, extraction_artifact,
        expected_extraction_sha256, input_manifest, expected_input_manifest_sha256):
    """Evaluate only the explicitly pinned subset; retain every universe member."""
    dependencies = diagnostic_dependencies()
    _require(_sha(input_manifest) == expected_input_manifest_sha256, 'diagnostic input manifest changed')
    manifest = _json(input_manifest)
    _require(set(manifest) == {'schema_version', 'kind', 'snapshot_sha256', 'evaluated_card_ids',
        'raw_pages', 'images', 'authorizations', 'qa'}, 'unknown or missing diagnostic manifest field')
    _require(type(manifest['schema_version']) is int and manifest['schema_version'] == 1
             and manifest['kind'] == 'source-book-diagnostic-inputs-v1'
             and manifest['snapshot_sha256'] == expected_extraction_sha256, 'unsupported diagnostic input binding')
    selected = manifest['evaluated_card_ids']
    _require(isinstance(selected, list) and selected and all(isinstance(cid, str) for cid in selected)
             and selected == sorted(set(selected)), 'exact sorted unique evaluation subset required')
    root, candidate = Path(input_root).resolve(strict=True), Path(candidate_root).resolve(strict=True)
    snapshot = load_runtime_snapshot(candidate_root=candidate, extraction_artifact=extraction_artifact,
                                     expected_extraction_sha256=expected_extraction_sha256)
    meta = snapshot.metadata()
    official = set(meta['official_card_ids'])
    canonical, runtime = set(meta['canonical_record_sha256']), set(meta['runtime_record_sha256'])
    _require(set(selected) <= official, 'subset must have official source membership')
    _require(isinstance(manifest['authorizations'], dict)
             and set(manifest['authorizations']) <= set(selected), 'foreign capability authorization')
    raw_pages = {url: _file(root, d) for url, d in manifest['raw_pages'].items()}
    images = {key: _file(root, d) for key, d in manifest['images'].items()}
    qa = _json(_file(root, manifest['qa']))
    _require(isinstance(qa.get('cards'), dict) and isinstance(qa.get('qa'), dict), 'complete QA container required')
    qa_ids = set(qa['cards'])
    for ruling in qa['qa'].values():
        _require(isinstance(ruling, dict) and isinstance(ruling.get('card_ids'), list)
                 and all(isinstance(cid, str) for cid in ruling['card_ids']), 'invalid QA membership')
        qa_ids.update(ruling['card_ids'])
    result = {'schema_version': 1, 'kind': 'bounded-source-composition-diagnostic-v1',
        'status': 'DIAGNOSTIC_ONLY_NOT_ADMISSION', 'evaluated_card_ids': selected,
        'snapshot_sha256': expected_extraction_sha256, 'input_manifest_sha256': expected_input_manifest_sha256,
        'membership_counts': {'official': len(official), 'canonical': len(canonical), 'runtime': len(runtime)},
        'source_revision': meta['source_revision'], 'full_span_count': None, 'full_gating_interaction_count': None,
        'cards': {}, 'dependency_sha256': dependencies,
        'checker_sha256': _sha(Path(__file__).read_bytes()),
        'limit': 'Diagnostic inventory only. Unknown counts are not zero. No clause IDs, verdicts, denominator or admission.'}
    for cid in sorted(official | canonical | runtime | qa_ids):
        member = {'official': cid in official, 'canonical': cid in canonical, 'runtime': cid in runtime,
            'provisional': cid in canonical and cid not in official,
            'runtime_only': cid in runtime and cid not in canonical,
            'test_only': cid.startswith('TEST-') and cid not in official,
            'qa': cid in qa_ids, 'qa_only': cid in qa_ids and cid not in official | canonical | runtime}
        blockers = [] if cid in official else [{'code': 'NO_OFFICIAL_SOURCE_ADMISSION'}]
        if cid not in canonical or cid not in runtime:
            blockers.append({'code': 'MISSING_CANONICAL_OR_RUNTIME_RECORD'})
        result['cards'][cid] = {'membership': member, 'status': 'BLOCKED' if blockers else 'NOT_EVALUATED',
            'blockers': blockers, 'span_count': None, 'gating_interaction_count': None}
    for cid in selected:
        row = result['cards'][cid]
        inputs = _source_inputs(snapshot, candidate, cid, raw_pages)
        row['original_observations'] = {oid: _json(raw) for oid, raw in sorted(inputs['observations'].items())}
        try:
            source, claimed, evidence = _dispatch(snapshot, expected_extraction_sha256, inputs, images,
                                                 manifest['authorizations'].get(cid, {}), root)
            row['source_composition'] = source
            row['composition_evidence'] = evidence
            runtime_record = _json(snapshot.card(cid)['runtime_record'])
            _require(isinstance(claimed, list) and all(isinstance(k, str) for k in claimed)
                     and len(set(claimed)) == len(claimed) and set(claimed) <= set(runtime_record), 'invalid runtime claims')
            unclaimed = {k: v for k, v in runtime_record.items() if k not in claimed}
            row['runtime_claimed_fields'] = claimed
            row['runtime_unclaimed'] = unclaimed
            row['runtime_unclaimed_classification'] = {k: 'non-gameplay-metadata' if k in _METADATA else 'unproven-gameplay'
                                                       for k in unclaimed}
            row['blockers'] = deepcopy(source.get('blockers', []))
            row['blockers'].extend({'code': 'UNCLAIMED_GAMEPLAY_FIELD', 'field': k} for k in unclaimed if k not in _METADATA)
            if source['status'] not in {'COMPOSED_RULE_CARD', 'COMPOSED_CORRECTED_RULE_CARD',
                                      'COMPOSED_DUAL_SOURCE', 'COMPOSED_LINK_CARD', 'COMPOSED_PAIRED_MAIN_CARD'}:
                row['blockers'].append({'code': 'SOURCE_COMPOSITION_BLOCKED'})
            if row['blockers']:
                row['status'] = 'BLOCKED'
                continue
            units, spans, obligations, conservation = _descriptors(cid, source)
            row.update(status='COMPOSED_DIAGNOSTIC', source_unit_descriptors=units, span_descriptors=spans,
                structural_obligations=obligations, conservation=conservation, span_count=len(spans))
            if 'implementation_correspondence' in evidence:
                row['source_status'] = source['status']
                implementation = deepcopy(evidence['implementation_correspondence'])
                row['implementation_correspondence'] = implementation
                if implementation['status'] == 'BLOCKED' or implementation['unresolved_obligations']:
                    row['status'] = 'BLOCKED'
                    row['blockers'].append({'code': 'IMPLEMENTATION_CORRESPONDENCE_BLOCKED',
                        'unresolved_obligations': deepcopy(implementation['unresolved_obligations'])})
        except (ValueError, TypeError, KeyError, IndexError, OSError) as error:
            row['status'] = 'BLOCKED'
            row['blockers'].append({'code': 'COMPOSITION_REFUSED', 'reason': str(error), 'error_type': type(error).__name__})
    _require(diagnostic_dependencies() == dependencies, 'source diagnostic dependencies changed during build')
    return result
