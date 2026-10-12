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
from tools.card_data.errors import CardDataError
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




def bind_canonical_field(*, aggregate: bytes, observations: dict[str, bytes],
                         canonical_field: bytes, adjudication_ledger: bytes,
                         old_input_revision: str, new_input_revision: str) -> dict:
    """A1 only: replay one selected text or trait-list field, never a whole book.

    Both revisions are caller pins. Current producer selections bind one source
    revision as both old/new; unequal revisions require a future producer path.
    All official AND secondary candidate observations come from that pinned
    aggregate's same-card membership. Raw-page extraction/approval authenticity
    remain the caller's independently reviewed source-input responsibility.
    """
    try:
        return _bind_canonical_field(aggregate, observations, canonical_field,
                                     adjudication_ledger, old_input_revision, new_input_revision)
    except SourceBindingError:
        raise
    except (KeyError, TypeError, ValueError, AttributeError, IndexError, CardDataError) as error:
        raise SourceBindingError('canonical field source binding refused: ' + str(error)) from error


def _bind_canonical_field(aggregate_raw, observation_bytes, field_raw, ledger_raw, old, new):
    from tools.card_data import adjudications, models
    _require(all(isinstance(x, bytes) for x in (aggregate_raw, field_raw, ledger_raw)), 'field inputs must be bytes')
    _require(isinstance(old, str) and isinstance(new, str)
             and re.fullmatch(r'[0-9a-f]{64}', old) and re.fullmatch(r'[0-9a-f]{64}', new)
             and old == new, 'explicit equal producer input revisions required')
    aggregate = json.loads(aggregate_raw, object_pairs_hook=_unique)
    _require(type(aggregate['schema_version']) is int and aggregate['schema_version'] == 1
             and aggregate['manifest_kind'] == 'full-pool-source-aggregate', 'unsupported aggregate schema')
    _require(aggregate['manifest_revision'] == new and serialization.content_digest(
        {k:v for k,v in aggregate.items() if k != 'manifest_revision'}, exclude_operational=True) == new,
        'aggregate differs from caller source revision')
    field = json.loads(field_raw, object_pairs_hook=_unique)
    _require(set(field) == {'card_id','field_path','value','provenance'}, 'unsupported field input envelope')
    cid, path = field['card_id'], field['field_path']
    ordinary = path in dict(FIELDS) and isinstance(field['value'], str)
    trait = path in {'form_eng', 'type_eng', 'attribute_eng'} and isinstance(field['value'], list) \
        and all(isinstance(value, str) for value in field['value'])
    _require(isinstance(cid, str) and CARD_ID_RE.fullmatch(cid)
             and (ordinary or trait), 'unsupported selected field')
    _require(set(aggregate['sources']) == {'official', 'digimoncard_io'}, 'complete candidate lanes required')
    all_refs = []; card_refs = {}
    for source, lane in aggregate['sources'].items():
        refs = lane['observation_refs']; ids = lane['observation_ids']
        _require(lane['source'] == source and lane['scope'].startswith('full-pool:')
                 and len(ids) == len(set(ids)) == len(refs) == lane['observation_count']
                 and {r['observation_id'] for r in refs} == set(ids), 'source candidate closure')
        _require(set(lane['gameplay_card_ids']) == {r['gameplay_card_id'] for r in refs}
                 and len(lane['gameplay_card_ids']) == lane['gameplay_card_count'], 'source card closure')
        for ref in refs:
            all_refs.append(ref)
            if ref['gameplay_card_id'] == cid:
                _require(ref['observation_id'] not in card_refs, 'duplicate candidate observation')
                card_refs[ref['observation_id']] = (source, ref)
    membership = aggregate['membership']; all_ids = [r['observation_id'] for r in all_refs]
    _require(len(all_ids) == len(set(all_ids)) == membership['observation_count']
             and set(membership['observation_ids']) == set(all_ids)
             and len(membership['observation_ids']) == len(all_ids)
             and set(membership['gameplay_card_ids']) == {r['gameplay_card_id'] for r in all_refs}
             and len(membership['gameplay_card_ids']) == membership['gameplay_card_count'], 'aggregate complete membership')
    _require(card_refs and set(observation_bytes) == set(card_refs), 'complete same-card candidates required')
    candidates = []
    for oid, raw in observation_bytes.items():
        _require(isinstance(raw, bytes), 'observation input must be bytes')
        value = json.loads(raw, object_pairs_hook=_unique); source, ref = card_refs[oid]
        _require(type(value['schema_version']) is int and value['schema_version'] == 1
                 and value['observation_id'] == oid and value['source'] == source
                 and value['extracted']['card_id'] == cid, 'candidate identity/card/source mismatch')
        lane = aggregate['sources'][source]
        _require(value['parser_version'] == lane['parser_version']
                 and value['source_schema_version'] == lane['source_schema_version'], 'candidate source schema mismatch')
        observation = models.SourceObservation(**{**value, 'source': models.SourceName(source)})
        _require(serialization.observation_digest(observation) == oid, 'candidate content identity mismatch')
        candidate = normalization.observation_candidate(observation)
        _require(candidate.printing_id == ref['printing_id'] and candidate.gameplay_card_id == cid,
                 'candidate printing identity mismatch')
        if path in candidate.fields:
            candidates.append(candidate)
    _require(candidates and any(c.authority is models.AuthorityClass.OFFICIAL for c in candidates),
             'official selected field evidence required')
    payload = json.loads(ledger_raw, object_pairs_hook=_unique)
    _require(set(payload) == {'schema_version','adjudications'} and type(payload['schema_version']) is int
             and payload['schema_version'] == 1 and isinstance(payload['adjudications'], list), 'unsupported adjudication ledger')
    parsed = []
    for row in payload['adjudications']:
        # The shared loader has historical coercions; reject ambiguous wire types
        # before using its approved typed constructor and matching implementation.
        _require(isinstance(row, dict) and type(row['schema_version']) is int and row['schema_version'] == 1,
                 'invalid adjudication row version')
        for name in ('adjudication_id','kind','card_id','old_input_revision','new_input_revision',
                     'old_digest','new_digest','reason','reviewer','reviewed_at'):
            _require(isinstance(row[name], str), 'invalid adjudication scalar')
        for name in ('provenance_observation_ids','affected_deck_ids'):
            _require(isinstance(row.get(name, []), list) and all(isinstance(v,str) for v in row.get(name, [])),
                     'invalid adjudication membership')
        for name in ('selected_observation_id','field_path'):
            _require(row.get(name) is None or isinstance(row[name], str), 'invalid adjudication field')
        parsed.append(adjudications._row(row, path=Path('<supplied-bytes>')))
    parsed = adjudications.validate_migration_adjudications(parsed)
    actual, provenance, conflicts = normalization._select_field(cid, path, candidates,
        admission=models.AdmissionStatus.ADMITTED, adjudications=parsed, field_selection_input_revision=new)
    _require(serialization.canonical_json_bytes(actual) == serialization.canonical_json_bytes(field['value'])
             and serialization.canonical_json_bytes(provenance) == serialization.canonical_json_bytes(field['provenance']),
             'canonical selected value/provenance differs from producer replay')
    _require(provenance.authority is models.AuthorityClass.OFFICIAL and not provenance.derived,
             'selected field lacks official authority')
    binding = normalization._canonical_conflict_binding(card_id=cid, field_path=path, candidates=candidates,
        selected_value=actual, selected_observation_id=provenance.selected_observation_id,
        old_input_revision=old, new_input_revision=new)
    approval = normalization._current_canonical_conflict_adjudication(binding, parsed)
    _require(conflicts and approval is not None and provenance.adjudicated is True,
             'current exact canonical field adjudication required')
    return {'version': 1, 'kind': 'canonical-field-source-binding', 'card_id': cid, 'field_path': path,
            'old_input_revision': old, 'new_input_revision': new,
            'selected_observation_id': provenance.selected_observation_id,
            'selected_value': actual, 'provenance': serialization.canonicalize(provenance),
            'candidate_values': serialization.canonicalize(binding.candidate_values),
            'candidate_digest': binding.candidate_digest, 'selected_value_digest': binding.selected_value_digest,
            'adjudication_id': approval.adjudication_id,
            'inputs': {'aggregate': _sha(aggregate_raw), 'canonical_field': _sha(field_raw),
                       'adjudication_ledger': _sha(ledger_raw),
                       'observations': {oid:_sha(raw) for oid,raw in sorted(observation_bytes.items())}},
            'dependencies': {m.__name__:_sha(Path(m.__file__).read_bytes())
                             for m in (adjudications, models, normalization, serialization)},
            'validator_sha256': _sha(Path(__file__).read_bytes())}


def bind_condition_units(*, aggregate, observations, canonical_card, runtime_projection,
                         selection_ledger, correction_document, image_artifacts, raw_pages,
                         old_input_revision, new_input_revision):
    """A2 condition units only. Does not assign whole-book clause IDs or verdicts."""
    try:
        return _bind_condition_units(aggregate, observations, canonical_card, runtime_projection,
            selection_ledger, correction_document, image_artifacts, raw_pages,
            old_input_revision, new_input_revision)
    except SourceBindingError:
        raise
    except (KeyError, TypeError, ValueError, AttributeError, IndexError, CardDataError) as error:
        raise SourceBindingError('condition source binding refused: ' + str(error)) from error


def _bind_condition_units(aggregate_raw, observation_bytes, card_raw, runtime_raw,
                          selections_raw, corrections_raw, images, raw_pages, old, new, *, _dual_dispatch=False):
    from tools.card_data import models, compatibility_conditions as conditions
    from tools.card_data import compatibility_selections as selections
    from tools.card_data import compatibility_corrections as corrections
    from tools.card_data.complete_candidate import _canonical_card_from_payload
    _require(all(isinstance(x,bytes)for x in (aggregate_raw,card_raw,runtime_raw,selections_raw,corrections_raw)),
             'condition inputs must be bytes')
    _require(isinstance(old,str) and isinstance(new,str) and re.fullmatch('[0-9a-f]{64}',new)
             and old==new,'explicit equal condition source revisions required')
    aggregate=json.loads(aggregate_raw,object_pairs_hook=_unique)
    _require(type(aggregate['schema_version'])is int and aggregate['schema_version']==1
             and aggregate['manifest_kind']=='full-pool-source-aggregate'
             and aggregate['manifest_revision']==new
             and serialization.content_digest({k:v for k,v in aggregate.items()if k!='manifest_revision'},exclude_operational=True)==new,
             'condition aggregate binding mismatch')
    record=json.loads(card_raw,object_pairs_hook=_unique);cid=record['card_id']
    _require(type(record['schema_version'])is int and record['schema_version']==1,'canonical card schema')
    card=_canonical_card_from_payload(cid,record,path=Path('<supplied-bytes>'))
    _require(set(aggregate['sources'])=={'official','digimoncard_io'},'complete condition source lanes required')
    member_refs={};all_refs=[]
    for source,lane in aggregate['sources'].items():
        refs=lane['observation_refs'];ids=lane['observation_ids']
        _require(lane['source']==source and lane['scope'].startswith('full-pool:')
                 and len(ids)==len(set(ids))==len(refs)==lane['observation_count']
                 and set(ids)=={r['observation_id']for r in refs}
                 and set(lane['gameplay_card_ids'])=={r['gameplay_card_id']for r in refs}
                 and len(lane['gameplay_card_ids'])==lane['gameplay_card_count'],'condition source closure')
        all_refs.extend(refs)
        for ref in refs:
            if ref['gameplay_card_id']==cid:
                _require(ref['observation_id']not in member_refs,'duplicate condition member')
                member_refs[ref['observation_id']]=(source,ref)
    membership=aggregate['membership'];all_ids=[r['observation_id']for r in all_refs]
    _require(len(all_ids)==len(set(all_ids))==membership['observation_count']
             and set(membership['observation_ids'])==set(all_ids) and len(membership['observation_ids'])==len(all_ids)
             and set(membership['gameplay_card_ids'])=={r['gameplay_card_id']for r in all_refs}
             and len(membership['gameplay_card_ids'])==membership['gameplay_card_count'],'condition aggregate closure')
    _require(member_refs and set(member_refs)==set(observation_bytes),'complete same-card condition membership required')
    typed={};printing_pairs=set()
    for oid,raw in observation_bytes.items():
        _require(isinstance(raw,bytes),'condition observation must be bytes')
        o=json.loads(raw,object_pairs_hook=_unique);source,ref=member_refs[oid];lane=aggregate['sources'][source]
        _require(type(o['schema_version'])is int and o['schema_version']==1 and o['observation_id']==oid
                 and o['source']==source and o['extracted']['card_id']==cid
                 and o['parser_version']==lane['parser_version'] and o['source_schema_version']==lane['source_schema_version'],
                 'condition observation identity/source/card mismatch')
        observation=models.SourceObservation(**{**o,'source':models.SourceName(source)})
        _require(serialization.observation_digest(observation)==oid,'condition observation hash mismatch')
        candidate=normalization.observation_candidate(observation)
        _require(candidate.printing_id==ref['printing_id'],'condition printing mismatch')
        printing_pairs.add((candidate.printing_id,oid));typed[oid]=observation
        if source=='official':
            # A2 accounts for ordinary boxes outside its condition scope, but
            # never treats unknown structural labels as absent conditions.
            allowed_labels = {label.lower() for label in LABELS} | set(conditions._CONDITION_LABELS)
            if _dual_dispatch:
                from .dual_source_units import derive_dual_occurrences
                diagnostic = derive_dual_occurrences(o)
                _require(all(b['code']=='TOP_LEVEL_KIND_DISAGREES_WITH_FACES'
                             for b in diagnostic['blockers']), 'unsupported Dual condition sibling')
                _require(o['extracted']['layout']=='dual', 'Dual dispatch requires two faces')
                allowed_labels |= {'dual effect','dual rule'}
            _require(all(isinstance(section,dict) and isinstance(section.get('text'),str)
                         and section.get('label','').lower() in allowed_labels
                         for section in o['extracted']['text_sections']),
                     'unsupported condition source section')
            origin=urlparse(o['source_url'])
            _require(origin.scheme=='https'and origin.netloc=='world.digimoncard.com'and origin.path=='/cards/', 'condition official origin')
            _require(o['parser_version']=='official-html-v5'and o['source_schema_version']=='world-card-list-html-v1', 'unsupported condition source parser')
            _require(_sha(raw_pages[o['source_url']])==o['raw_payload_digest'],'condition raw source changed/missing')
    canonical_pairs=[(p.printing_id,oid)for p in card.printings for oid in p.observation_ids]
    _require(len(canonical_pairs)==len(set(canonical_pairs))and set(canonical_pairs)==printing_pairs,
             'canonical condition membership must match aggregate exactly')
    selected=selections.selections_from_bytes(selections_raw)
    document=corrections.correction_document_from_bytes(corrections_raw)
    corrections.validate_image_artifacts(document,images)
    relevant_selections=[s for s in selected if s.card_id==cid]
    current_selections=[s for s in relevant_selections if s.source_manifest_revision==new]
    relevant_corrections=[c for c in document.compatibility_corrections if c.card_id==cid]
    current_corrections=[c for c in relevant_corrections if c.source_manifest_revision==new]
    _require(not relevant_selections or len(current_selections)==1,'condition selection stale/ambiguous')
    _require(not relevant_corrections or len(current_corrections)==1,'condition correction stale/ambiguous')
    _require(not(current_selections and current_corrections),'condition selection/correction overlap')
    values={oid:'\n'.join(f"[{s['label']}] {s['text']}".strip()for s in o.extracted['text_sections']
            if any(label in s['label'].lower()for label in conditions._CONDITION_LABELS))
            for oid,o in sorted(typed.items())if o.source is models.SourceName.OFFICIAL}
    if current_selections:
        _require(dict(current_selections[0].candidate_values)==values
                 and current_selections[0].candidate_set_digest==serialization.content_digest(values),
                 'condition selection candidate map changed')
    if current_corrections:
        correction=current_corrections[0]
        projection,evidence=corrections.apply_image_correction(card,typed,correction,
            source_manifest_revision=new,image_artifacts=images)
        units=[{'kind':'image-correction','field':'xros_req','text':projection,
                'correction_id':correction.correction_id,'candidate_values':dict(correction.candidate_values),
                'candidate_set_digest':correction.candidate_set_digest,
                'base_observation_id':correction.base_observation_id,'base_value':correction.base_value,
                'evidence_observation_ids':list(evidence),'image_artifact_path':correction.image_artifact_path,
                'image_sha256':correction.image_sha256,'reviewer':correction.reviewer,
                'reviewed_at':correction.reviewed_at,'reason':correction.reason}]
        selection_id=None
    else:
        projection,evidence=conditions.official_condition_projection(card,typed,selections=selected,source_manifest_revision=new)
        selected_id=current_selections[0].selected_observation_id if current_selections else sorted(evidence)[0]
        selection_id=current_selections[0].selection_id if current_selections else None
        units=[]
        for index,section in enumerate(typed[selected_id].extracted['text_sections']):
            if any(label in section['label'].lower()for label in conditions._CONDITION_LABELS):
                _require(section['label'].lower()in conditions._CONDITION_LABELS,'unsupported composite condition label')
                units.append({'kind':'official-section','field':'xros_req','observation_id':selected_id,
                    'printing_id':member_refs[selected_id][1]['printing_id'],'section_index':index,
                    'label':section['label'],'condition_type':section['label'].lower().replace(' ','_'),
                    'text':section['text']})
        _require('\n'.join(f"[{u['label']}] {u['text']}".strip()for u in units)==projection,
                 'condition unit multiplicity/reconstruction mismatch')
    runtime=json.loads(runtime_raw,object_pairs_hook=_unique)
    _require(set(runtime)=={'card_id','xros_req'} and runtime['card_id']==cid
             and isinstance(runtime['xros_req'],str)and runtime['xros_req']==projection,'runtime condition projection mismatch')
    for unit in units:
        unit['spans']=[{'kind':s.kind,'timings':s.timings,'keyword':s.keyword,'text':s.text}
                       for s in text_split.split_clauses(unit['text'])]
    return {'version':1,'kind':'condition-source-unit-binding','card_id':cid,'field':'xros_req',
            'old_input_revision':old,'new_input_revision':new,'projection':projection,
            'candidate_values':values,'evidence_observation_ids':list(evidence),'selection_id':selection_id,'units':units,
            'inputs':{'aggregate':_sha(aggregate_raw),'canonical_card':_sha(card_raw),'runtime_projection':_sha(runtime_raw),
                      'selection_ledger':_sha(selections_raw),'correction_document':_sha(corrections_raw),
                      'observations':{oid:_sha(raw)for oid,raw in sorted(observation_bytes.items())},
                      'raw_pages':{url:_sha(raw)for url,raw in sorted(raw_pages.items())},
                      'images':{path:_sha(raw)for path,raw in sorted(images.items())}},
            'validator_sha256':_sha(Path(__file__).read_bytes()),
            'dependencies':{m.__name__:_sha(Path(m.__file__).read_bytes())for m in
                            (models,normalization,serialization,conditions,selections,corrections,text_split)} |
                            ({'tools.clause_coverage.dual_source_units':_sha(Path(__file__).with_name('dual_source_units.py').read_bytes())} if _dual_dispatch else {})}


def validate_condition_units(binding, **inputs):
    """Recompute all units/spans from supplied sources; no metadata-only blessing."""
    expected=bind_condition_units(**inputs)
    _require(type(binding)is dict and serialization.canonical_json_bytes(binding)==serialization.canonical_json_bytes(expected),
             'condition units differ from complete producer replay')


def _bind_dual_condition_units(*, aggregate, observations, canonical_card, runtime_projection,
        selection_ledger, correction_document, image_artifacts, raw_pages,
        old_input_revision, new_input_revision):
    """Internal face-aware binding; recomputes complete sibling diagnostics.

    This does not discharge image/source kind disagreements or approve a card.
    The public ordinary A2 entrypoint retains its existing refusal behavior.
    """
    return _bind_condition_units(aggregate, observations, canonical_card, runtime_projection,
        selection_ledger, correction_document, image_artifacts, raw_pages,
        old_input_revision, new_input_revision, _dual_dispatch=True)
