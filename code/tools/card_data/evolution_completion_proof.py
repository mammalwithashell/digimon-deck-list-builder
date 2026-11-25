"""Offline, prove-only completion of partial printed normal evolution routes.

No emitter, source correction, runtime cutover or approval policy is provided.
Every accepted printing needs an independently recorded rendered-image witness;
HTML omissions alone never establish a missing route component. The external
binding pins the complete pending evo-conflict denominator, not a selected list.
"""
from __future__ import annotations

from collections import defaultdict
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import parse_qs, urljoin, urlparse

from bs4 import BeautifulSoup
from . import models, normalization, serialization, source_manifest
from .errors import CardDataError
from .sources import official

PREDICATE_VERSION = 'render-witnessed-normal-evolution-completion-v1'
FIELD = 'evo_costs'
COLORS = {'Red', 'Blue', 'Yellow', 'Green', 'Black', 'Purple', 'White'}
HEX = re.compile(r'^[0-9a-f]{64}$')


class ProofError(ValueError):
    """Malformed/stale evidence or incomplete witness closure; no proof emitted."""


def require(condition, message):
    if not condition:
        raise ProofError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    require(isinstance(raw, bytes), 'bytes required')
    value = json.loads(raw, object_pairs_hook=pairs)
    require(isinstance(value, dict), 'object required')
    return value


def routes_valid(routes):
    return (isinstance(routes, list) and 1 <= len(routes) <= 2
            and all(isinstance(r, dict) and set(r) == {'slot', 'colors', 'cost', 'from_level'}
                    and type(r['slot']) is int and r['slot'] == i + 1
                    and isinstance(r['colors'], list) and bool(r['colors'])
                    and len(set(r['colors'])) == len(r['colors']) and set(r['colors']) <= COLORS
                    and type(r['cost']) is int and r['cost'] >= 0
                    and type(r['from_level']) is int and 1 <= r['from_level'] <= 7
                    for i, r in enumerate(routes)))


def hidden(node, boundary):
    for current in [node, *node.parents]:
        if current.has_attr('hidden') or current.get('style') or any(str(c).lower().startswith('cardfaq') for c in current.get('class', [])):
            return True
        if current is boundary:
            break
    return False


def raw_evidence(o, raw, selected, *, soup=None, source_blocks=None):
    """Check identity and present normal-badge components; never fill blanks."""
    e = o.extracted
    pid = e['printing']['printing_id']
    if soup is None:
        soup = BeautifulSoup(raw, 'html.parser')
    blocks = soup.find_all('div', class_='popupCol', id=pid)
    require(len(blocks) == 1 and len(soup.find_all(id=pid)) == 1, 'raw popup not unique')
    block = blocks[0]
    require(not hidden(block, None), 'hidden raw popup or ancestor')
    for selector, expected in [('li.cardNo', e['card_id']), ('.cardTitle', e['name']), ('li.cardType', 'Digimon')]:
        nodes = block.select(selector)
        require(len(nodes) == 1 and not hidden(nodes[0], block) and not nodes[0].find(True)
                and nodes[0].get_text(' ', strip=True) == expected, 'raw card/name/type identity')
    images = block.select('div.cardImg img')
    require(len(images) == 1 and not hidden(images[0], block)
            and urljoin(o.source_url, images[0].get('src', '')) == e['printing']['image_url'], 'raw image identity')
    require(not block.select('.dualCardCol, .cardDetailColInnerDual, .cardDetailColInner_dual'), 'dual source layout')
    source_blocks = [b for b in (source_blocks if source_blocks is not None else official._card_blocks(official._decode_payload(raw, source_url=o.source_url)))
                     if re.search(r'\bid=[\"\']' + re.escape(pid) + r'[\"\']', b)]
    require(len(source_blocks) == 1, 'raw printing list item not unique')
    parsed = official._parse_printing(source_blocks[0], source_url=o.source_url, category_id=e['printing']['category_id'])
    require(serialization.canonicalize(parsed.extracted) == serialization.canonicalize(e), 'raw extracted observation replay mismatch')
    cells, reasons, missing = [], [], []
    for dt in block.select('dt.cardInfoTit'):
        label = dt.get_text(' ', strip=True)
        if not label.startswith('Digivolve Cost'):
            continue
        match = re.fullmatch(r'Digivolve Cost ([12])', label)
        if not match:
            reasons.append('non_normal_route_label'); continue
        dd = dt.find_next_sibling('dd')
        if dt.find(True) or dd is None or dt.parent.name != 'dl' or dd.parent is not dt.parent or hidden(dt, block) or hidden(dd, block):
            reasons.append('hidden_or_missing_route_cell'); continue
        if any(hidden(n, block) for n in dd.find_all(True)):
            reasons.append('hidden_route_component'); continue
        colors = []
        body = BeautifulSoup(str(dd), 'html.parser').dd
        for node in list(body.find_all(True)):
            classes = node.get('class', [])
            if (node.name != 'span' or len(classes) != 1 or not classes[0].startswith('cardColor_')
                    or node.find(True) or node.get_text(' ', strip=True) not in COLORS
                    or classes[0] != 'cardColor_' + node.get_text(' ', strip=True).lower()):
                reasons.append('nonliteral_route_component'); break
            colors.append(node.get_text(' ', strip=True)); node.decompose()
        rest = body.get_text(' ', strip=True)
        if len(colors) > 1:
            # Color separators, if printed in the source, must stay before cost.
            rest = re.sub(r'^\s*/\s*', '', rest, count=len(colors)-1)
        normal = re.fullmatch(r'(?:(\d+)\s+)?from(?:\s+Lv\.(\d+))?', rest)
        if not normal:
            reasons.append('non_normal_or_ambiguous_route_body'); continue
        components = dict(colors=colors, cost=int(normal[1]) if normal[1] else None,
                          from_level=int(normal[2]) if normal[2] else None)
        slot = int(match[1]); cells.append({'slot': slot, 'components': components, 'raw_cell_sha256': sha(str(dd).encode())})
        target = next((r for r in selected if r['slot'] == slot), None)
        if target is None:
            reasons.append('extra_slot'); continue
        absent = [key for key, value in components.items() if value is None or value == []]
        if len(absent) == 3:
            reasons.append('all_components_missing')
        for key, value in components.items():
            if key not in absent and value != target[key]:
                reasons.append('present_component_disagrees:' + key)
        if absent:
            missing.append({'slot': slot, 'fields': absent})
    if [c['slot'] for c in cells] != [r['slot'] for r in selected]:
        reasons.append('whole_slot_missing_extra_or_reordered')
    structured = e.get('evolution_routes')
    if structured and structured != selected:
        reasons.append('complete_structured_routes_disagree')
    return dict(observation_id=o.observation_id, source_record_key=o.source_record_key,
                raw_payload_digest=o.raw_payload_digest, printing_id=pid,
                image_url=e['printing']['image_url'], raw_block_sha256=sha(str(block).encode()),
                raw_cells=cells, missing_components=missing), reasons


def prove(**kwargs):
    try:
        return _prove(**kwargs)
    except ProofError:
        raise
    except (ValueError, TypeError, KeyError, AttributeError, IndexError, CardDataError) as error:
        raise ProofError('evolution proof refused: ' + str(error)) from error


def _prove(*, report_bytes, official_bytes, secondary_bytes, aggregate_bytes,
           binding_bytes, witness_bytes, raw_pages, image_bytes):
    inputs = dict(report_bytes=report_bytes, official_bytes=official_bytes,
                  secondary_bytes=secondary_bytes, aggregate_bytes=aggregate_bytes)
    binding = load(binding_bytes)
    require(type(binding.get('schema_version')) is int and binding['schema_version'] == 1, 'binding schema')
    require(binding['input_sha256'] == {k: sha(v) for k, v in inputs.items()}, 'input byte binding mismatch')
    report, off, secondary, aggregate = map(load, inputs.values())
    for key in ('old_input_revision', 'new_input_revision', 'source_manifest_digest'):
        require(isinstance(report[key], str) and HEX.fullmatch(report[key]), 'report revision identity')
    all_candidates, official_cards = defaultdict(list), defaultdict(list)
    by_id, scopes, page_hashes = {}, [], {}
    for snap, source, parser, schema, scope in [
        (off, models.SourceName.OFFICIAL, 'official-html-v5', 'world-card-list-html-v1', 'full-pool:official-catalog'),
        (secondary, models.SourceName.DIGIMONCARD_IO, 'digimoncard-io-json-v1', 'digimoncard-io-public-api-v1', 'full-pool:digimoncard-io-catalog')]:
        m = snap['manifest']
        require(m['source'] == source.value and m['parser_version'] == parser and m['source_schema_version'] == schema
                and m['scope'] == scope and m['complete'] is True and m['failed_ids'] == [], 'source scope identity/completeness')
        typed_scope = models.ScopeManifest(**(m | {'source': source, 'observation_refs': tuple(models.ScopeObservationRef(**r) for r in m['observation_refs'])}))
        scopes.append(typed_scope); current = {}
        for value in snap['observations']:
            require(value['source'] == source.value and value['parser_version'] == parser and value['source_schema_version'] == schema, 'observation authority')
            o = models.SourceObservation(**(value | {'source': source}))
            require(o.observation_id not in by_id and serialization.observation_digest(o) == o.observation_id, 'observation identity')
            c = normalization.observation_candidate(o)
            current[o.observation_id] = c; by_id[o.observation_id] = o; all_candidates[c.gameplay_card_id].append(c)
            require(o.source_url in m['source_locations'], 'source URL outside manifest')
            if source is models.SourceName.OFFICIAL:
                e = o.extracted; u = urlparse(o.source_url); categories = parse_qs(u.query).get('category', [])
                require(u.scheme == 'https' and u.netloc == 'world.digimoncard.com' and u.path == '/cards/' and len(categories) == 1, 'official source URL')
                pid = e['printing']['printing_id']
                require(o.source_record_key == f'{categories[0]}:{c.gameplay_card_id}:{pid}'
                        and e['printing']['category_id'] == categories[0], 'source record key identity')
                image = urlparse(e['printing']['image_url'])
                require(image.scheme == 'https' and image.netloc == 'world.digimoncard.com'
                        and image.path == f'/images/cardlist/card/{pid}.png', 'official image URL identity')
                require(o.source_url in raw_pages and isinstance(raw_pages[o.source_url], bytes), 'missing raw page')
                if o.source_url not in page_hashes: page_hashes[o.source_url] = sha(raw_pages[o.source_url])
                require(page_hashes[o.source_url] == o.raw_payload_digest, 'raw page digest mismatch')
                official_cards[c.gameplay_card_id].append(o)
        require(len(current) == len(m['observation_ids']) and set(current) == set(m['observation_ids']), 'observation closure')
        require(type(m['expected_count']) is int and m['expected_count'] == len({c.gameplay_card_id for c in current.values()}), 'scope card count')
        for ref in typed_scope.observation_refs:
            c = current[ref.observation_id]
            require(ref.gameplay_card_id == c.gameplay_card_id and ref.printing_id == c.printing_id, 'scope ref identity')
    rebuilt = source_manifest.build_full_pool_source_manifest(scopes).to_dict()
    require(serialization.canonical_json_bytes(rebuilt) == serialization.canonical_json_bytes(aggregate)
            and aggregate['manifest_revision'] == report['source_manifest_digest'], 'aggregate replay mismatch')
    conflicts = [f for f in report['canonical_conflicts'] if f['field_path'] == FIELD and f.get('approval_required') is True and f.get('status') in ('missing', 'stale')]
    keys = sorted([[f['card_id'], FIELD] for f in conflicts])
    require(len({tuple(k) for k in keys}) == len(keys) and keys == binding['conflict_keys']
            and type(binding['conflict_count']) is int and len(keys) == binding['conflict_count'], 'complete conflict denominator')
    review = load(witness_bytes)
    require(type(review.get('schema_version')) is int and review['schema_version'] == 1
            and review['report_sha256'] == sha(report_bytes) and review['source_manifest_digest'] == report['source_manifest_digest']
            and review['new_input_revision'] == report['new_input_revision'], 'stale witness envelope')
    witnesses = {}
    for w in review['witnesses']:
        oid = w['observation_id']; require(oid not in witnesses and oid in by_id, 'duplicate/foreign witness')
        o = by_id[oid]; e = o.extracted
        require(o.source is models.SourceName.OFFICIAL and [e['card_id'], FIELD] in keys, 'witness outside official conflict scope')
        for k, value in dict(card_id=e['card_id'], printing_id=e['printing']['printing_id'], source_record_key=o.source_record_key,
                             raw_payload_digest=o.raw_payload_digest, image_url=e['printing']['image_url'], rendered_card_id=e['card_id']).items():
            require(w[k] == value, 'witness identity mismatch: ' + k)
        require(w['visually_checked'] is True and isinstance(w['reviewer'], str) and bool(w['reviewer'].strip()), 'rendered reviewer required')
        stamp = datetime.fromisoformat(w['reviewed_at'].replace('Z', '+00:00'))
        require(stamp.tzinfo is not None, 'review timestamp requires timezone')
        require(routes_valid(w['printed_routes']), 'malformed printed route witness')
        require(w['image_url'] in image_bytes and isinstance(image_bytes[w['image_url']], bytes)
                and sha(image_bytes[w['image_url']]) == w['image_sha256'], 'rendered image bytes mismatch')
        witnesses[oid] = w
    corrected = {c['card_id'] for key in ('corrections', 'compatibility_corrections') for c in report.get(key, [])}
    eligible, excluded, soups, source_blocks_by_url = [], [], {}, {}
    for f in sorted(conflicts, key=lambda x: x['card_id']):
        cid = f['card_id']; candidates = [c for c in all_candidates[cid] if FIELD in c.fields]
        actual = normalization._canonical_conflict_binding(card_id=cid, field_path=FIELD, candidates=candidates,
                    selected_value=f['selected_value'], selected_observation_id=f['selected_observation_id'],
                    old_input_revision=report['source_manifest_digest'], new_input_revision=report['source_manifest_digest'])
        for key, value in serialization.canonicalize(actual).items():
            require(f[key] == value, 'current candidate binding mismatch: ' + key)
        selected = [c for c in candidates if c.observation.observation_id == f['selected_observation_id']]
        require(len(selected) == 1 and selected[0].observation.source is models.SourceName.OFFICIAL
                and selected[0].fields[FIELD] == f['selected_value'], 'selected observation/value mismatch')
        template = dict(kind='canonical_field_selection', card_id=cid, field_path=FIELD,
            old_input_revision=actual.old_input_revision, new_input_revision=actual.new_input_revision,
            old_digest=actual.candidate_digest, new_digest=actual.selected_value_digest, new_value=actual.selected_value,
            selected_observation_id=actual.selected_observation_id, provenance_observation_ids=list(actual.candidate_observation_ids))
        require(f['adjudication_template'] == template, 'adjudication template mismatch')
        reasons, evidence = [], []
        if cid in corrected: reasons.append('card_has_correction')
        if cid.startswith('EX6-'): reasons.append('EX6_owned_separate_review')
        if not routes_valid(f['selected_value']): reasons.append('selected_not_complete_normal_routes')
        if not official_cards[cid]: reasons.append('no_official_printing')
        if not reasons:
            for o in official_cards[cid]:
                e = o.extracted
                if e.get('layout') != 'single' or e.get('card_type') != 'Digimon':
                    reasons.append('not_single_digimon'); continue
                if o.source_url not in soups:
                    soups[o.source_url] = BeautifulSoup(raw_pages[o.source_url], 'html.parser')
                    source_blocks_by_url[o.source_url] = official._card_blocks(official._decode_payload(raw_pages[o.source_url], source_url=o.source_url))
                row, failures = raw_evidence(o, raw_pages[o.source_url], f['selected_value'],
                                             soup=soups[o.source_url], source_blocks=source_blocks_by_url[o.source_url])
                evidence.append(row); reasons.extend(failures)
            if not any(row['missing_components'] for row in evidence): reasons.append('no_partial_component_gap')
        if not reasons:
            for row in evidence:
                oid = row['observation_id']; require(oid in witnesses, 'missing current printing witness')
                w = witnesses[oid]
                require(w['candidate_digest'] == actual.candidate_digest and w['selected_observation_id'] == actual.selected_observation_id, 'stale witness candidate/selection')
                require(w['printed_routes'] == actual.selected_value, 'printed route disagrees with selected value')
                row['rendered_witness'] = w
        result = dict(card_id=cid, field_path=FIELD, candidate_digest=actual.candidate_digest,
                      candidate_values=list(actual.candidate_values), candidate_observation_ids=list(actual.candidate_observation_ids),
                      selected_observation_id=actual.selected_observation_id, selected_value=actual.selected_value,
                      adjudication_template=template, official_evidence=evidence)
        if reasons: excluded.append(result | {'reasons': sorted(set(reasons))})
        else: eligible.append(result)
    return dict(status='PROOF_ONLY_NO_APPROVALS', predicate_version=PREDICATE_VERSION,
                predicate_sha256=sha(Path(__file__).read_bytes()),
                dependency_sha256={m.__name__: sha(Path(m.__file__).read_bytes()) for m in (models, normalization, serialization, source_manifest, official)},
                input_sha256=binding['input_sha256'], binding_sha256=sha(binding_bytes), witness_sha256=sha(witness_bytes),
                source_manifest_digest=report['source_manifest_digest'], new_input_revision=report['new_input_revision'],
                raw_page_sha256=page_hashes, total_conflicts=len(conflicts), eligible=eligible, excluded=excluded,
                membership_sha256=serialization.content_digest([r['card_id'] for r in eligible]))
