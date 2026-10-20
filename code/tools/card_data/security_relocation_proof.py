"""Prove-only exact paired Security source-zone restoration; never emit approvals."""
from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path
from urllib.parse import urljoin

from bs4 import BeautifulSoup
from . import form_restoration_proof as base, normalization, adjudications
from .sources import official
from . import compatibility_corrections, compatibility_selections, corrections as correction_rows, errors
from .errors import CardDataError
from .serialization import canonical_json_bytes, content_digest, canonicalize
from .models import SourceObservation, SourceName

PREDICATE_VERSION = 'exact-security-zone-pair-v2'
INHERITED = 'inherited_effect_description_eng'
SECURITY = 'security_effect_description_eng'
TEXT_FIELDS = frozenset({INHERITED, SECURITY, 'effect_description_eng'})
ProofError = base.ProofError


def _runtime(value):
    # Producer's newline serialization only, never old/new equivalence.
    return value.replace('\r\n', '\n').replace('\n', '\r\n')


def _raw_reasons(observation, soup):
    e = observation['extracted']
    blocks = soup.find_all('div', class_='popupCol', id=e['printing']['printing_id'])
    if len(blocks) != 1:
        return ['ambiguous_raw_popup']
    block, reasons = blocks[0], []
    if block.has_attr('hidden') or block.get('style'):
        reasons.append('hidden_raw_popup')
    if [n.get_text(' ', strip=True) for n in block.select('li.cardNo')] != [e['card_id']]:
        reasons.append('raw_card_identity')
    types = block.select('li.cardType')
    if (len(types) != 1 or types[0].find(True) or types[0].has_attr('hidden')
            or types[0].get('style') or types[0].get_text(' ', strip=True) != e['card_type']):
        reasons.append('raw_type_not_literal')
    if [urljoin(observation['source_url'], n.get('src', '')) for n in block.select('div.cardImg img')] != [e['printing'].get('image_url')]:
        reasons.append('raw_image_identity')
    if block.select('.cardDetailColInnerDual, .cardDetailColInner_dual'):
        reasons.append('raw_dual_layout')
    sections = official._text_sections(str(block))
    if sections != e.get('text_sections'):
        reasons.append('raw_sections_disagree')
    labels = block.select('dt.cardInfoTitSmall')
    for label in labels:
        for parent in label.parents:
            if parent is block: break
            if parent.has_attr('hidden') or parent.get('style'):
                reasons.append('hidden_raw_section_parent')
    security = [n for n in labels if n.get_text(' ', strip=True) == '[Security Effect]']
    if len(security) != 1:
        reasons.append('raw_security_section_count')
    for label in security:
        if label.find(True) is not None:
            reasons.append('security_label_not_literal_text')
        column = label.find_parent('div', class_='cardInfoCol')
        if (label.parent.name != 'dl' or 'cardInfoBoxSmall' not in label.parent.get('class', [])
                or column is None or column.find_parent('div', class_='popupCol') is not block):
            reasons.append('security_not_in_printed_info_container')
        for ancestor in [label, *label.parents]:
            if any(str(name).lower().startswith('cardfaq') for name in ancestor.get('class', [])):
                reasons.append('security_section_in_faq')
            if ancestor is block:
                break
    for label in labels:
        value = label.find_next_sibling('dd')
        if value is None or any(n.has_attr('hidden') or n.get('style') for n in [label, value, *value.find_all(True)]):
            reasons.append('hidden_or_missing_raw_section')
    return reasons


def prove(*, report_bytes, snapshot_bytes, canonical_bytes, ledger_bytes,
          corrections_bytes, binding_bytes, raw_pages):
    """Classify full pinned input, retaining both rows and all excluded members."""
    inputs = dict(report_bytes=report_bytes, snapshot_bytes=snapshot_bytes,
                  canonical_bytes=canonical_bytes, ledger_bytes=ledger_bytes,
                  corrections_bytes=corrections_bytes)
    binding = base._load(binding_bytes)
    base._require(binding.get('input_sha256') == {k: base._sha(v) for k, v in inputs.items()},
                  'input binding mismatch')
    # Existing reviewed validator enforces official scope/ref/identity/raw closure.
    checked = base.prove(report_bytes=report_bytes, snapshot_bytes=snapshot_bytes, raw_pages=raw_pages)
    report, snapshot = base._load(report_bytes), base._load(snapshot_bytes)
    canonical, ledger, corrections = map(base._load, (canonical_bytes, ledger_bytes, corrections_bytes))
    base._require(isinstance(ledger, dict) and set(ledger) == {'schema_version', 'adjudications'}
                  and type(ledger.get('schema_version')) is int and ledger['schema_version'] == 1
                  and isinstance(ledger.get('adjudications'), list), 'invalid whole ledger')
    try:
        compatibility_corrections.correction_document_from_bytes(corrections_bytes)
    except CardDataError as error:
        raise ProofError('invalid complete correction document: ' + str(error)) from error
    rows = adjudications.validate_migration_adjudications(
        [adjudications._row(r, path=Path('<bound-ledger>')) for r in ledger['adjudications']])
    by_card = defaultdict(list)
    for o in snapshot['observations']:
        by_card[o['extracted']['card_id']].append(o)
    cards = canonical.get('cards', {})
    base._require(set(by_card) <= set(cards), 'canonical missing official membership')
    aggregate = canonical.get('source_manifest_digest')
    reported_aggregate = report.get('source_manifest_digest')
    base._require(isinstance(aggregate, str) and base.HEX.fullmatch(aggregate)
                  and isinstance(reported_aggregate, str) and base.HEX.fullmatch(reported_aggregate)
                  and aggregate == reported_aggregate, 'report/canonical aggregate revision mismatch')
    all_diffs = report['runtime_field_diffs']
    index, approved = {}, set()
    current = defaultdict(list)
    for row in rows:
        if (row.kind.value == 'runtime_field_cutover' and row.old_input_revision == report['old_input_revision']
                and row.new_input_revision == report['new_input_revision']):
            current[(row.card_id, row.field_path)].append(row)
    for d in all_diffs:
        key = d['card_id'], d['field_path']
        base._require(key not in index, 'duplicate runtime field diff')
        base._require(d.get('old_digest') == content_digest(d.get('old_value')) and d.get('new_digest') == content_digest(d.get('new_value')), 'runtime field digest mismatch')
        index[key] = d
        matches = current.get(key, [])
        base._require(len(matches) <= 1, 'duplicate current ledger field')
        for row in matches:
            base._require(row.old_digest == d['old_digest'] and row.new_digest == d['new_digest']
                          and row.old_value == d['old_value'] and row.new_value == d['new_value']
                          and bool(row.provenance_observation_ids)
                          and set(row.provenance_observation_ids) <= set(d['provenance_observation_ids']),
                          'current ledger mismatch')
            approved.add(key)
        if d.get('approved') is True:
            base._require(bool(d.get('approval_id')), 'unbound report approval')
            approved.add(key)
    base._require(set(current) <= set(index), 'current ledger target absent from report')
    counts = dict(runtime_count=len(index), remaining_runtime_count=len(index.keys()-approved),
                  remaining_text_count=sum(k[1] in TEXT_FIELDS for k in index.keys()-approved),
                  official_card_count=len(by_card))
    base._require(all(type(binding.get(k)) is int and binding[k] == v for k, v in counts.items()), 'denominator binding mismatch')
    corrected = {r['card_id'] for doc in (report, corrections)
                 for key in ('corrections', 'compatibility_corrections') for r in doc.get(key, [])}
    conflicts = {r['card_id'] for r in report.get('canonical_conflicts', []) if r.get('approval_required')}
    pair_cards = sorted({cid for cid, field in index if field in {INHERITED, SECURITY}})
    eligible, excluded, soups = [], [], {}
    for cid in pair_cards:
        pair = [index.get((cid, field)) for field in (INHERITED, SECURITY)]
        reasons, evidence = [], []
        if any(d is None for d in pair):
            reasons.append('missing_pair')
        else:
            inherit, security = pair
            old = inherit['old_value']
            if not (isinstance(old, str) and old.startswith('[Security]')
                    and inherit['new_value'] == '' and security['old_value'] == '' and security['new_value'] == old):
                reasons.append('not_exact_no_header_pair')
        if cid in corrected: reasons.append('card_has_correction')
        if cid in conflicts: reasons.append('required_canonical_conflict')
        if cid in base.KNOWN_SOURCE_GAPS: reasons.append('known_adjacent_source_gap')
        observed = by_card.get(cid, [])
        if not observed: reasons.append('no_official_printing')
        card = cards.get(cid, {})
        # Validate exact provenance against pinned complete canonical record.
        for d in pair:
            if d is None: continue
            provenance = card.get('provenance', {}).get(d['field_path'], {})
            ids = provenance.get('observation_ids')
            selected = provenance.get('selected_observation_id')
            official_ids = {o['observation_id'] for o in observed}
            base._require(isinstance(ids, list) and all(isinstance(x, str) for x in ids)
                          and len(ids) == len(set(ids)) and isinstance(selected, str)
                          and selected in ids and selected in official_ids,
                          'canonical selected provenance is not a same-card official member')
            base._require(set(d.get('provenance_observation_ids', [])) == set(provenance.get('observation_ids', [])), 'canonical diff provenance mismatch')
            base._require({o['observation_id'] for o in observed} <= set(d['provenance_observation_ids']), 'diff omits official printing provenance')
        if not reasons:
            faces = card.get('faces')
            if not isinstance(faces, list) or len(faces) != 1 or faces[0].get('kind') not in {'tamer', 'option'} or faces[0].get('face_id') != 'primary':
                reasons.append('canonical_not_plain_single_face')
            for o in observed:
                e = o['extracted']
                faces = e.get('faces')
                if (e.get('card_type') not in {'Tamer', 'Option'} or e.get('layout') != 'single'
                        or not isinstance(faces, list) or len(faces) != 1
                        or faces[0].get('kind') != e['card_type'].lower()):
                    reasons.append('not_plain_single_tamer_option')
                if e.get('printing_field_gaps'): reasons.append('printing_field_gap')
                secs = [s for s in e.get('text_sections', []) if s.get('label') == 'Security Effect']
                if len(secs) != 1 or _runtime(secs[0]['text']) != pair[1]['new_value']:
                    reasons.append('explicit_security_disagrees')
                fields = normalization._normalize_candidate_fields(normalization._official_fields(e))
                normalized_faces = canonicalize(normalization._parse_faces(
                    e.get('faces'), observation=SourceObservation(**(o | {'source': SourceName.OFFICIAL}))))
                if normalized_faces != card.get('faces'):
                    reasons.append('canonical_faces_disagree_with_official_replay')
                if len(normalized_faces) == 1:
                    for field, face_field in [('effect_description_eng', 'effect_text'),
                                              (INHERITED, 'inherited_text'), (SECURITY, 'security_text')]:
                        source_value = fields.get(field)
                        canonical_value = card.get('fields', {}).get(field)
                        face_value = normalized_faces[0].get(face_field)
                        if not all(isinstance(v, str) for v in (source_value, canonical_value, face_value)) or not (_runtime(source_value) == _runtime(canonical_value) == _runtime(face_value)):
                            reasons.append('source_canonical_face_field_inconsistency')
                if fields.get(INHERITED) != '' or _runtime(fields.get(SECURITY, '')) != pair[1]['new_value']:
                    reasons.append('official_pair_disagrees')
                if any(s.get('label') == 'Inherited Effect' and s.get('text', '').strip() not in {'', '-', 'ー', '－', '—'} for s in e.get('text_sections', [])):
                    reasons.append('official_inherited_present')
                url = o['source_url']
                if url not in soups: soups[url] = BeautifulSoup(raw_pages[url].decode('utf-8'), 'html.parser')
                reasons.extend(_raw_reasons(o, soups[url]))
                evidence.append({k:o[k] for k in ('observation_id','raw_payload_digest','source_url','source_record_key')} | {'printing_id':e['printing']['printing_id'],'text_sections':e['text_sections']})
            for d in pair:
                value = card.get('fields', {}).get(d['field_path'])
                if not isinstance(value, str) or _runtime(value) != d['new_value']:
                    reasons.append('canonical_pair_disagrees')
            known_ids = {oid for p in card.get('printings', []) for oid in p.get('observation_ids', [])}
            if not all(set(d['provenance_observation_ids']) <= known_ids for d in pair):
                reasons.append('canonical_printing_evidence_missing')
        if reasons:
            excluded.append(dict(card_id=cid, reasons=sorted(set(reasons)), diffs=[d for d in pair if d is not None]))
        else:
            overlaps = [(cid, field) in approved for field in (INHERITED, SECURITY)]
            base._require(all(overlaps) or not any(overlaps), 'partial existing pair requires reconciliation')
            eligible.append(dict(card_id=cid, diffs=pair, official_evidence=sorted(evidence,key=lambda e:e['observation_id']), overlap='complete_exact_pair' if all(overlaps) else 'none'))
    eligible_keys = {(r['card_id'], d['field_path']) for r in eligible for d in r['diffs']}
    excluded_keys = {(r['card_id'], d['field_path']) for r in excluded for d in r['diffs']}
    inventory = [dict(card_id=k[0], field_path=k[1], old_digest=d['old_digest'], new_digest=d['new_digest'],
                      existing_approved=k in approved, classification='eligible_pair' if k in eligible_keys else 'excluded_pair' if k in excluded_keys else 'outside_family')
                 for k, d in sorted(index.items())]
    dependencies = dict(checked['dependency_sha256'])
    for name, module in [('form_restoration_proof.py',base),('normalization.py',normalization),('sources/official.py',official),('compatibility_corrections.py',compatibility_corrections),
                         ('compatibility_selections.py',compatibility_selections),
                         ('corrections.py',correction_rows),('errors.py',errors)]:
        dependencies[name] = base._sha(Path(module.__file__).read_bytes())
    return dict(status='PROOF_ONLY_NO_APPROVALS', predicate_version=PREDICATE_VERSION,
                predicate_sha256=base._sha(Path(__file__).read_bytes()), dependency_sha256=dependencies,
                input_sha256=binding['input_sha256'],binding_sha256=base._sha(binding_bytes),
                old_input_revision=report['old_input_revision'],new_input_revision=report['new_input_revision'],
                source_manifest_revision=canonical['source_manifest_digest'],raw_page_sha256=checked['raw_page_sha256'],
                total_runtime_diffs=counts['runtime_count'],remaining_runtime_diffs=counts['remaining_runtime_count'],
                remaining_text_diffs=counts['remaining_text_count'],official_card_count=counts['official_card_count'],
                total_pair_cards=len(pair_cards),runtime_inventory=inventory,eligible=eligible,excluded=excluded,
                membership_sha256=content_digest([r['card_id'] for r in eligible]))


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    for key in ('report','snapshot','canonical','ledger','corrections','binding','cache-index','out'):
        parser.add_argument('--'+key,type=Path,required=True)
    args=parser.parse_args(argv)
    base._require(not args.out.exists(),'refusing to overwrite proof')
    import json
    pages={}
    for item in json.loads(args.cache_index.read_text(encoding='utf-8')):
        path=(args.cache_index.parent/item['file']).resolve()
        base._require(path.is_relative_to(args.cache_index.parent.resolve()),'cache path escapes inventory')
        base._require(item['url'] not in pages,'duplicate cache URL')
        pages[item['url']]=path.read_bytes()
    kwargs={key+'_bytes':getattr(args,key).read_bytes() for key in ('report','snapshot','canonical','ledger','corrections','binding')}
    result=prove(**kwargs,raw_pages=pages)
    with args.out.open('xb') as stream:stream.write(canonical_json_bytes(result))
    return 0

if __name__=='__main__':
    raise SystemExit(main())