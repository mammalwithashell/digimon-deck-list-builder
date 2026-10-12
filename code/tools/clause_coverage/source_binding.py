"""Inactive, pure phase-one source traceability contract; never a verdict.

Only ordinary unconflicted official text boxes are supported. Conditions,
Link/Dual, selections and corrections require separately reviewed adapters.
Exact observation/raw identities bind supplied evidence; they do not independently
prove that an extractor interpreted a page or a printed card correctly.
"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlparse

from tools.card_data import normalization, serialization
from tools.card_data.models import CARD_ID_RE
from . import semantic_fingerprint, text_split

VERSION = 1
FIELDS = (
    ('effect_description_eng', 'effect'),
    ('inherited_effect_description_eng', 'inherited'),
    ('security_effect_description_eng', 'security'),
)
REQUIRED = frozenset(('snapshot', 'canonical', 'runtime', 'source_manifest',
                      'corrections', 'selections', 'adjudications'))
LABELS = {'Effect': 'effect_description_eng',
          'Inherited Effect': 'inherited_effect_description_eng',
          'Security': 'security_effect_description_eng',
          'Security Effect': 'security_effect_description_eng'}


class SourceBindingError(ValueError):
    """Inputs cannot establish the supported source binding; never fall back."""


def _require(condition, message):
    if not condition:
        raise SourceBindingError(message)


def _sha(raw):
    return hashlib.sha256(raw).hexdigest()


def _unique(pairs):
    result = {}
    for key, value in pairs:
        _require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def _empty_ledger(value, collection):
    _require(isinstance(value, dict) and set(value) == {'schema_version', collection}
             and type(value['schema_version']) is int and value['schema_version'] == 1
             and value[collection] == [], 'unsupported ' + collection + ' adapter/schema')


def derive_binding(*, inputs: dict[str, bytes], raw_pages: dict[str, bytes],
                   expected_universe: list[str]) -> dict:
    """Derive traceability from complete explicit inputs, with no writes.

    expected_universe is an independent caller pin, not inferred from paired
    artifacts. A caller adopting this contract must derive it from its reviewed
    complete source scope. This helper does not authorize that caller's scope.
    """
    try:
        return _derive(inputs, raw_pages, expected_universe)
    except SourceBindingError:
        raise
    except (KeyError, TypeError, ValueError, AttributeError, IndexError) as error:
        raise SourceBindingError('malformed/unsupported source inputs: ' + str(error)) from error


def _derive(inputs, raw_pages, universe):
    _require(set(inputs) == REQUIRED, 'complete exact input set required')
    _require(isinstance(universe, list) and bool(universe)
             and all(isinstance(c, str) and CARD_ID_RE.fullmatch(c) for c in universe)
             and len(set(universe)) == len(universe), 'invalid independent universe')
    _require(all(isinstance(v, bytes) for v in inputs.values()), 'inputs must be bytes')
    data = {k: json.loads(v, object_pairs_hook=_unique) for k, v in inputs.items()}
    for name in ('corrections', 'selections', 'adjudications'):
        _empty_ledger(data[name], name)
    snap = data['snapshot']; manifest = snap['manifest']; observations = snap['observations']
    _require(type(manifest['schema_version']) is int and manifest['schema_version'] == 1
             and manifest['source'] == 'official'
             and manifest['scope'] == 'full-pool:official-catalog'
             and manifest['parser_version'] == 'official-html-v5'
             and manifest['source_schema_version'] == 'world-card-list-html-v1',
             'unsupported official manifest header')
    locations = manifest['source_locations']
    _require(isinstance(locations, list) and len(set(locations)) == len(locations)
             and set(locations) == {o['source_url'] for o in observations},
             'manifest source location closure')
    _require(manifest['complete'] is True and manifest['failed_ids'] == []
             and type(manifest['expected_count']) is int
             and manifest['expected_count'] == len(universe), 'incomplete official scope')
    by_id = {}; by_card = {cid: [] for cid in universe}
    for o in observations:
        oid = o['observation_id']; e = o['extracted']; cid = e['card_id']
        _require(cid in by_card and oid not in by_id, 'foreign/duplicate observation')
        _require(o['source'] == 'official' and o['parser_version'] == 'official-html-v5'
                 and o['source_schema_version'] == 'world-card-list-html-v1', 'unsupported observation schema')
        identity = {k: o[k] for k in ('raw_payload_digest', 'parser_version', 'source_schema_version', 'extracted')}
        _require(serialization.content_digest(identity, exclude_operational=True) == oid,
                 'invalid observation identity')
        origin = urlparse(o['source_url'])
        _require(origin.scheme == 'https' and origin.netloc == 'world.digimoncard.com'
                 and origin.path == '/cards/', 'unsupported official origin')
        _require(_sha(raw_pages[o['source_url']]) == o['raw_payload_digest'], 'raw page changed')
        _require(isinstance(e['printing']['printing_id'], str) and e['printing']['printing_id'], 'missing printing')
        _require(e.get('layout') == 'single', 'unsupported explicit source layout')
        faces = e.get('faces', [])
        _require(isinstance(faces, list) and len(faces) <= 1, 'unsupported face adapter')
        if faces:
            _require(faces[0]['face_id'] == 'primary' and faces[0]['text_sections'] == e['text_sections'],
                     'inconsistent primary face sections')
        for section in e['text_sections']:
            _require(section['label'] in LABELS and isinstance(section['text'], str), 'unsupported source section')
            # Rule grants need a distinct structural adapter, even inside Effect.
            _require(not re.search(r'\bRule\b', section['text'], re.I), 'unsupported Rule adapter')
        by_id[oid] = o; by_card[cid].append(o)
    _require(all(by_card.values()), 'missing zero-clause or source card')
    refs = manifest['observation_refs']
    _require(len(refs) == len(by_id) and len({r['observation_id'] for r in refs}) == len(refs)
             and set(manifest['observation_ids']) == set(by_id)
             and len(manifest['observation_ids']) == len(by_id), 'official observation closure')
    for r in refs:
        e = by_id[r['observation_id']]['extracted']
        _require(r['gameplay_card_id'] == e['card_id']
                 and r['printing_id'] == e['printing']['printing_id'], 'printing reference mismatch')
    aggregate = data['source_manifest']; revision = aggregate['manifest_revision']
    _require(serialization.content_digest({k:v for k,v in aggregate.items() if k != 'manifest_revision'},
             exclude_operational=True) == revision, 'aggregate revision mismatch')
    official_ids = aggregate['sources']['official']['observation_ids']
    _require(len(official_ids) == len(by_id) and set(official_ids) == set(by_id), 'aggregate official closure')
    canonical = data['canonical']; runtime = data['runtime']
    _require(type(canonical['schema_version']) is int and canonical['schema_version'] == 1
             and canonical['source_manifest_digest'] == revision, 'canonical aggregate binding')
    _require(set(canonical['cards']) == set(universe) == set(runtime), 'full universe mismatch')
    cards = {}
    for cid in sorted(universe):
        card = canonical['cards'][cid]; record = runtime[cid]; own = by_card[cid]
        _require(card['card_id'] == record['card_id'] == cid, 'card identity mismatch')
        _require(not record.get('xros_req') and not record.get('dual')
                 and not record.get('link_effect_description_eng')
                 and not card['fields'].get('link_effect_description_eng')
                 and len(card.get('faces', [])) <= 1, 'unsupported structural projection')
        candidates = {o['observation_id']: normalization._normalize_candidate_fields(
            normalization._official_fields(o['extracted'])) for o in own}
        own_ids = set(candidates)
        printing_pairs = {(p['printing_id'], oid) for p in card['printings'] for oid in p['observation_ids']}
        _require(all((by_id[oid]['extracted']['printing']['printing_id'], oid) in printing_pairs
                     for oid in own_ids), 'canonical printing closure')
        _require(all(oid in own_ids and pid == by_id[oid]['extracted']['printing']['printing_id']
                     for pid, oid in printing_pairs), 'foreign canonical printing')
        kinds = {v['card_kind'] for v in candidates.values()}
        _require(len(kinds) == 1 and card['fields']['card_kind'] in kinds, 'card kind mismatch')
        kind = next(iter(kinds))
        runtime_kind = record.get('card_kind')
        # Existing runtime ABI uses integers; source-shaped test projections use
        # canonical names. Neither Boolean aliases nor unknown integers qualify.
        kind_names = {0:'Digimon', 1:'Tamer', 2:'Option', 3:'Digi-Egg', 4:'Dual'}
        if type(runtime_kind) is int:
            runtime_kind = kind_names.get(runtime_kind)
        _require(type(runtime_kind) is str and runtime_kind == kind, 'runtime card kind mismatch')
        _require(kind in ('Digimon', 'Digi-Egg', 'Tamer', 'Option'), 'unsupported card kind')
        units = []; clauses = []; zones = {zone: [] for _, zone in FIELDS}
        # Fixed field order is part of this contract; original source ordering
        # within a field is retained by section_index. Cross-field source order
        # is independently bound in the full observation inventory below.
        for field, zone in FIELDS:
            provenance = card['provenance'][field]; selected = provenance['selected_observation_id']
            _require(provenance['authority'] == 'official' and provenance.get('derived') is False
                     and provenance.get('adjudicated') is False, 'unsupported selected authority')
            pids = provenance['observation_ids']
            _require(isinstance(pids, list) and len(pids) == len(set(pids))
                     and set(pids) == own_ids and selected in own_ids, 'field provenance closure')
            value = card['fields'][field]
            _require(isinstance(value, str) and all(v[field] == value for v in candidates.values()),
                     'conflicting or changed official canonical field')
            _require(record.get(field) == value, 'runtime projection mismatch')
            o = by_id[selected]; e = o['extracted']; parts = []
            for index, section in enumerate(e['text_sections']):
                mapped = LABELS[section['label']]
                if kind == 'Digi-Egg' and mapped == 'effect_description_eng':
                    mapped = 'inherited_effect_description_eng'
                if mapped != field:
                    continue
                _require(not (field == 'inherited_effect_description_eng' and kind in ('Option','Tamer')),
                         'unsupported misplaced inherited section')
                rendered = normalization._render_effect_section(section['label'], section['text'])
                _require(rendered is not None, 'unsupported section projection')
                parts.append(rendered)
                unit_index = len(units)
                unit = {'field': field, 'zone': zone, 'observation_id': selected,
                        'printing_id': e['printing']['printing_id'], 'section_index': index,
                        'label': section['label'], 'text': section['text'], 'projected_text': rendered,
                        'clause_ids': []}
                for split_index, span in enumerate(text_split.split_clauses(rendered)):
                    ident = f'{cid}#{zone}#{len(zones[zone])}'
                    row = {'id': ident, 'card_id': cid, 'zone': zone, 'label': section['label'],
                           'kind': span.kind, 'timings': span.timings, 'keyword': span.keyword,
                           'text': span.text, 'source': 'official-source-binding',
                           'source_unit_index': unit_index, 'source_split_index': split_index}
                    clauses.append(row); zones[zone].append(ident); unit['clause_ids'].append(ident)
                units.append(unit)
            _require('\n'.join(parts) == value, 'unsupported lossless field reconstruction')
        cards[cid] = {'observations': [{'observation_id': o['observation_id'],
                     'printing_id': o['extracted']['printing']['printing_id'],
                     'source_url': o['source_url'], 'raw_payload_digest': o['raw_payload_digest'],
                     'text_sections': o['extracted']['text_sections']} for o in own],
                     'units': units, 'zones': zones,
                     'clauses': semantic_fingerprint.bind_clause_semantics(clauses)}
    return {'version': VERSION, 'kind': 'clause-source-traceability',
            'inputs': {k:{'sha256':_sha(v),'bytes':len(v)} for k,v in sorted(inputs.items())},
            'raw_pages': {k:_sha(v) for k,v in sorted(raw_pages.items())},
            'dependencies': {m.__name__:_sha(Path(m.__file__).read_bytes())
                             for m in (normalization, serialization, text_split, semantic_fingerprint)},
            'validator_sha256': _sha(Path(__file__).read_bytes()),
            'universe': sorted(universe), 'cards': cards}


def validate_binding(binding, *, inputs, raw_pages, expected_universe):
    """Re-derive from actual inputs; rehashing submitted metadata cannot bless it."""
    expected = derive_binding(inputs=inputs, raw_pages=raw_pages, expected_universe=expected_universe)
    _require(type(binding) is dict and serialization.canonical_json_bytes(binding) == serialization.canonical_json_bytes(expected),
             'source binding differs from complete fresh derivation')


