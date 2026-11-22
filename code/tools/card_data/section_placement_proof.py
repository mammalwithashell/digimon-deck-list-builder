"""Prove-only exact two-field section placement; no approvals or runtime cutover.

A declared review scope is pinned alongside the complete pending text inventory.
An empty field is never approved alone: one literal body must be conserved across
an exact paired swap, and every official image must independently locate it.
"""
from __future__ import annotations
from collections import defaultdict
from datetime import datetime
from pathlib import Path
import re
from urllib.parse import parse_qs, urljoin, urlparse
from bs4 import BeautifulSoup
from . import evolution_completion_proof as identity
from . import models, normalization, serialization, source_manifest, complete_candidate, generate
from .sources import official
from .errors import CardDataError

ProofError=identity.ProofError
require,load,sha,hidden=identity.require,identity.load,identity.sha,identity.hidden
HEX=identity.HEX
PREDICATE_VERSION='exact-rendered-section-placement-v1'
FIELDS={'effect_description_eng':('Effect','main_box'),'inherited_effect_description_eng':('Inherited Effect','inherited_box'),'security_effect_description_eng':('Security Effect','security_box')}


def source_context(off,secondary,aggregate,report,raw_pages):
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
    return all_candidates, official_cards, by_id, page_hashes


def raw_sections(o, raw, soup):
    e=o.extracted;pid=e['printing']['printing_id']
    blocks=soup.find_all('div',class_='popupCol',id=pid)
    require(len(blocks)==1 and len(soup.find_all(id=pid))==1,'raw popup identity/uniqueness')
    block=blocks[0];require(not hidden(block,None),'hidden/FAQ popup ancestor')
    for selector,value in [('li.cardNo',e['card_id']),('.cardTitle',e['name']),('li.cardType',e['card_type'])]:
        nodes=block.select(selector)
        require(len(nodes)==1 and not nodes[0].find(True) and not hidden(nodes[0],block)
                and nodes[0].get_text(' ',strip=True)==value,'raw card/name/type identity')
    images=block.select('div.cardImg img')
    require(len(images)==1 and not hidden(images[0],block) and urljoin(o.source_url,images[0].get('src',''))==e['printing']['image_url'],'raw image identity')
    require(not block.select('.dualCardCol, .cardDetailColInnerDual, .cardDetailColInner_dual'),'dual unsupported')
    source_blocks=[b for b in official._card_blocks(official._decode_payload(raw,source_url=o.source_url)) if re.search(r'\bid=["\']'+re.escape(pid)+r'["\']',b)]
    require(len(source_blocks)==1,'source list item identity')
    parsed=official._parse_printing(source_blocks[0],source_url=o.source_url,category_id=e['printing']['category_id'])
    require(serialization.canonicalize(parsed.extracted)==serialization.canonicalize(e),'raw observation replay')
    values={f:'' for f in FIELDS};evidence=[];seen=set()
    for dt in block.select('dt.cardInfoTitSmall'):
        label=dt.get_text(' ',strip=True).strip('[]')
        if label not in {v[0] for v in FIELDS.values()}:continue
        field=next(f for f,v in FIELDS.items() if v[0]==label)
        require(field not in seen,'duplicate printed section label');seen.add(field)
        dd=dt.find_next_sibling('dd');column=dt.find_parent('div',class_='cardInfoCol')
        require(not dt.find(True) and dt.parent.name=='dl' and 'cardInfoBoxSmall' in dt.parent.get('class',[])
                and column is not None and column.find_parent('div',class_='popupCol') is block
                and dd is not None and dd.parent is dt.parent,'section container identity')
        require(not hidden(dt,block) and not hidden(dd,block) and not any(hidden(n,block) for n in dd.find_all(True)),'hidden/FAQ printed section')
        # Reuse only the producer's HTML decoding and documented icon/dash ABI.
        # No whitespace, punctuation, reminder or literal equivalence transform.
        pairs=official._paired_dl(str(dt)+str(dd),'cardInfoTitSmall')
        require(len(pairs)==1,'literal raw section extraction')
        value=normalization._normalize_printed_effect_text(pairs[0][1]);values[field]=value
        evidence.append(dict(field_path=field,label=label,body=value,raw_body_sha256=sha(str(dd).encode())))
    return values,dict(observation_id=o.observation_id,source_record_key=o.source_record_key,
        raw_payload_digest=o.raw_payload_digest,printing_id=pid,image_url=e['printing']['image_url'],
        raw_block_sha256=sha(str(block).encode()),sections=evidence)


def prove(**kwargs):
    try:return _prove(**kwargs)
    except ProofError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as error:
        raise ProofError('section proof refused: '+str(error)) from error


def _prove(*,report_bytes,official_bytes,secondary_bytes,aggregate_bytes,canonical_bytes,
           binding_bytes,witness_bytes,raw_pages,image_bytes):
    inputs=dict(report_bytes=report_bytes,official_bytes=official_bytes,secondary_bytes=secondary_bytes,
                aggregate_bytes=aggregate_bytes,canonical_bytes=canonical_bytes)
    binding=load(binding_bytes);require(type(binding.get('schema_version')) is int and binding['schema_version']==1,'binding schema')
    require(binding['input_sha256']=={k:sha(v) for k,v in inputs.items()},'input byte binding')
    report,off,secondary,aggregate,canonical=map(load,inputs.values())
    for k in ('old_input_revision','new_input_revision','source_manifest_digest'):
        require(isinstance(report[k],str) and HEX.fullmatch(report[k]),'report revision identity')
    candidates,official_cards,by_id,page_hashes=source_context(off,secondary,aggregate,report,raw_pages)
    require(canonical['source_manifest_digest']==report['source_manifest_digest']
            and serialization.content_digest(canonical['cards'])==report['candidate_canonical_revision'],'canonical/report revision mismatch')
    pending=[f for f in report['canonical_conflicts'] if f['field_path'] in FIELDS and f.get('approval_required') is True and f.get('status') in ('missing','stale')]
    keys=sorted([[f['card_id'],f['field_path']] for f in pending]);require(keys==binding['pending_text_keys'] and len({tuple(k) for k in keys})==len(keys),'full pending text denominator')
    index={(f['card_id'],f['field_path']):f for f in pending};plans=binding['plans'];require(isinstance(plans,list) and bool(plans),'review scope required')
    require(len({p['card_id'] for p in plans})==len(plans),'duplicate scope card')
    for plan in plans:
        require(isinstance(plan['fields'],list) and len(plan['fields'])==2 and len(set(plan['fields']))==2
                and set(plan['fields'])<=set(FIELDS) and all((plan['card_id'],f) in index for f in plan['fields']),'invalid paired scope')
    review=load(witness_bytes)
    require(type(review.get('schema_version')) is int and review['schema_version']==1 and review['report_sha256']==sha(report_bytes)
            and review['source_manifest_digest']==report['source_manifest_digest'] and review['new_input_revision']==report['new_input_revision'],'stale witness envelope')
    witnesses={};scope={p['card_id']:p['fields'] for p in plans}
    for w in review['witnesses']:
        oid=w['observation_id'];require(oid in by_id and oid not in witnesses,'foreign/duplicate witness')
        o=by_id[oid];e=o.extracted;require(o.source is models.SourceName.OFFICIAL and e['card_id'] in scope,'witness outside source scope')
        for k,v in dict(card_id=e['card_id'],printing_id=e['printing']['printing_id'],source_record_key=o.source_record_key,
                raw_payload_digest=o.raw_payload_digest,image_url=e['printing']['image_url'],rendered_card_id=e['card_id']).items():
            require(w[k]==v,'rendered witness identity: '+k)
        require(w['visually_checked'] is True and isinstance(w['reviewer'],str) and bool(w['reviewer'].strip()),'independent rendering review required')
        require(datetime.fromisoformat(w['reviewed_at'].replace('Z','+00:00')).tzinfo is not None,'review timestamp timezone')
        require(set(w['fields'])==set(FIELDS) and all(isinstance(v,str) for v in w['fields'].values()),'literal complete zone witness required')
        locations=w['section_locations']
        require(set(locations)==set(FIELDS) and locations['effect_description_eng']=='main_box'
                and locations['inherited_effect_description_eng']=='inherited_box'
                and locations['security_effect_description_eng'] in {'security_box','security_trigger_in_main_box'},'wrong rendered section location')
        labels=w['rendered_section_labels'];require(set(labels)==set(FIELDS),'rendered section labels required')
        if w['fields']['inherited_effect_description_eng']:
            require(labels['inherited_effect_description_eng']=='Inherited Effect','wrong inherited printed label')
        if w['fields']['security_effect_description_eng'] and locations['security_effect_description_eng']=='security_box':
            require(labels['security_effect_description_eng']=='Security Effect','wrong security printed label')
        require(w['image_url'] in image_bytes and isinstance(image_bytes[w['image_url']],bytes)
                and sha(image_bytes[w['image_url']])==w['image_sha256'],'changed/missing rendered image')
        for field in scope[e['card_id']]:
            f=index[e['card_id'],field]
            require(w['field_bindings'][field]==dict(candidate_digest=f['candidate_digest'],selected_observation_id=f['selected_observation_id']),'stale rendered candidate/selection')
        witnesses[oid]=w
    eligible=[];excluded=[];soups={};selected_keys=set();corrected={x['card_id'] for k in ('corrections','compatibility_corrections') for x in report.get(k,[])}
    for plan in plans:
        cid=plan['card_id'];fields=plan['fields'];card=complete_candidate._canonical_card_from_payload(cid,canonical['cards'][cid],path=Path('<bound-canonical>'))
        require(card.admission is models.AdmissionStatus.ADMITTED and len(card.faces)==1,'single admitted canonical card required')
        runtime=generate.project_canonical_card(card);templates=[];selected={};reasons=[];evidence=[]
        for field in FIELDS:require(runtime[field]==generate._legacy_text(card.fields.get(field,'')),'runtime zone mapping mismatch')
        for field in fields:
            f=index[cid,field];cs=[c for c in candidates[cid] if field in c.fields]
            cb=normalization._canonical_conflict_binding(card_id=cid,field_path=field,candidates=cs,selected_value=f['selected_value'],selected_observation_id=f['selected_observation_id'],old_input_revision=report['source_manifest_digest'],new_input_revision=report['source_manifest_digest'])
            for k,v in serialization.canonicalize(cb).items():require(f[k]==v,'candidate multiset/binding mismatch: '+k)
            own=[c for c in cs if c.observation.observation_id==f['selected_observation_id']]
            require(len(own)==1 and own[0].observation.source is models.SourceName.OFFICIAL and own[0].fields[field]==f['selected_value'],'selected official field mismatch')
            canonical_matches=(card.fields[field]==f['selected_value'] and card.provenance[field].selected_observation_id==f['selected_observation_id']
                    and set(card.provenance[field].observation_ids)==set(cb.candidate_observation_ids))
            if cid in corrected and not canonical_matches:reasons.append('corrected_canonical_differs_from_source_selection')
            else:require(canonical_matches,'canonical field/provenance mismatch')
            template=dict(kind='canonical_field_selection',card_id=cid,field_path=field,old_input_revision=cb.old_input_revision,new_input_revision=cb.new_input_revision,old_digest=cb.candidate_digest,new_digest=cb.selected_value_digest,new_value=cb.selected_value,selected_observation_id=cb.selected_observation_id,provenance_observation_ids=list(cb.candidate_observation_ids))
            require(f['adjudication_template']==template,'template mismatch');templates.append(template);selected[field]=f['selected_value'];selected_keys.add((cid,field))
            require(runtime[field]==generate._legacy_text(card.fields[field]),'runtime zone mapping mismatch')
        values=[selected[f] for f in fields]
        if not all(isinstance(v,str) for v in values) or sum(v!='' for v in values)!=1:reasons.append('not_exact_one_body_one_blank_pair')
        if cid in corrected:reasons.append('card_has_correction')
        if cid.startswith('EX6-'):reasons.append('EX6_separate_review')
        require(bool(official_cards[cid]),'no official printing')
        for o in official_cards[cid]:
            require(o.observation_id in witnesses,'missing current official printing witness')
            w=witnesses[o.observation_id]
            if w['section_locations']['security_effect_description_eng']=='security_trigger_in_main_box':reasons.append('upper_one_shot_security_requires_separate_proof')
            if any(w['fields'][f]!=selected[f] for f in fields):reasons.append('rendered_literal_or_placement_disagrees')
            if o.source_url not in soups:soups[o.source_url]=BeautifulSoup(raw_pages[o.source_url],'html.parser')
            raw,ev=raw_sections(o,raw_pages[o.source_url],soups[o.source_url]);ev['rendered_witness']=w;evidence.append(ev)
            c=normalization.observation_candidate(o)
            if any(raw[f]!=c.fields.get(f,'') for f in fields):reasons.append('implicit_or_cross_zone_projection')
            pair=[raw[f] for f in fields]
            if pair!=values and pair!=list(reversed(values)):reasons.append('not_literal_paired_relocation')
        if not any([normalization.observation_candidate(o).fields.get(f,'') for f in fields]!=values for o in official_cards[cid]):reasons.append('no_official_section_misplacement')
        row=dict(card_id=cid,fields=fields,selected_values=selected,adjudication_templates=templates,
                 candidate_bindings=[index[cid,f] for f in fields],runtime_mapping={f:runtime[f] for f in FIELDS},official_evidence=evidence)
        if reasons:excluded.append(row|{'reasons':sorted(set(reasons))})
        else:eligible.append(row)
    return dict(status='PROOF_ONLY_NO_APPROVALS',predicate_version=PREDICATE_VERSION,predicate_sha256=sha(Path(__file__).read_bytes()),
        dependency_sha256={m.__name__:sha(Path(m.__file__).read_bytes()) for m in (identity,models,normalization,serialization,source_manifest,official,complete_candidate,generate)},
        input_sha256=binding['input_sha256'],binding_sha256=sha(binding_bytes),witness_sha256=sha(witness_bytes),
        source_manifest_digest=report['source_manifest_digest'],new_input_revision=report['new_input_revision'],raw_page_sha256=page_hashes,
        pending_text_count=len(pending),reviewed_card_count=len(plans),outside_family=[f for f in pending if (f['card_id'],f['field_path']) not in selected_keys],
        eligible=eligible,excluded=excluded,membership_sha256=serialization.content_digest([(r['card_id'],r['fields']) for r in eligible]))
