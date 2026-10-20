"""Source-validated, opt-in ordinary-box composition; no book or verdict output.

The complete producer inputs are replayed on every call. Unsupported structural
families return an inventory and explicit blockers, never a partial composition.
Canonical/runtime bytes supplied here remain caller-owned; nothing is emitted.

Byte pairing and producer normalization do not independently authenticate parser
extraction or card images. Callers must provide a sealed, reviewed source
snapshot; this contract checks its bindings, not the truth of newly invented
source observations or reviewer identities.
"""
from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlparse

from tools.card_data import adjudications, models, normalization, serialization
from tools.card_data import complete_candidate, source_manifest
from tools.card_data.compatibility_corrections import correction_document_from_bytes
from tools.card_data.compatibility_selections import selections_from_bytes
from tools.card_data.errors import CardDataError
from .source_binding import bind_canonical_field, SourceBindingError
from .text_split import split_clauses


class SourceUnitError(ValueError):
    pass


_FIELDS = {'effect_description_eng':'effect',
           'inherited_effect_description_eng':'inherited',
           'security_effect_description_eng':'security'}
_LABELS = {'effect':'effect_description_eng',
           'inherited effect':'inherited_effect_description_eng',
           'inheritable effect':'inherited_effect_description_eng',
           'security effect':'security_effect_description_eng'}


def _require(value, message):
    if not value:
        raise SourceUnitError(message)


def _unique(pairs):
    result = {}
    for key,value in pairs:
        _require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def _json(raw):
    _require(isinstance(raw,bytes), 'source inputs must be bytes')
    return json.loads(raw,object_pairs_hook=_unique)


def _sha(raw):
    return hashlib.sha256(raw).hexdigest()


def _unsupported_ordinary_text(text):
    return bool(re.search(r'\bRule\b|[<＜]\s*Link\b[^>＞]*[>＞]',text,re.I))


def _partition_structure_blockers(blockers,selected_ids,security_binding,security_value):
    """Retain reviewed wrong-label alternatives; do not hide other structures."""
    active=[];alternatives=[]
    for blocker in blockers:
        oid=blocker.get('observation_id')
        reviewed_wrong_label=(
            oid not in selected_ids and blocker['code']=='UNSUPPORTED_SECTION'
            and blocker.get('label')=='Link Condition'
            and security_binding.get('adjudication_id')
            and any(oid in row['observation_ids']
                    for row in security_binding.get('candidate_values',[]))
            and blocker.get('text','').startswith('[Security]')
            and normalization._normalize_printed_effect_text(blocker['text'])==security_value
        )
        (alternatives if reviewed_wrong_label else active).append(blocker)
    return active,alternatives


def _ledger(raw):
    d = _json(raw)
    _require(set(d)=={'schema_version','adjudications'} and type(d['schema_version']) is int
             and d['schema_version']==1 and isinstance(d['adjudications'],list), 'invalid adjudication envelope')
    for row in d['adjudications']:
        _require(type(row.get('schema_version')) is int and row['schema_version']==1, 'adjudication version')
        for name in ('adjudication_id','kind','card_id','old_input_revision','new_input_revision',
                     'old_digest','new_digest','reason','reviewer','reviewed_at'):
            _require(isinstance(row.get(name),str), 'invalid adjudication scalar')
        for name in ('provenance_observation_ids','affected_deck_ids'):
            _require(isinstance(row.get(name,[]),list) and all(isinstance(v,str)for v in row.get(name,[])), 'invalid adjudication membership')
        for name in ('field_path','selected_observation_id'):
            _require(row.get(name) is None or isinstance(row[name],str), 'invalid adjudication field')
    return adjudications.validate_migration_adjudications(
        adjudications._row(row,path=Path('<supplied-bytes>')) for row in d['adjudications'])


def compose_ordinary_card(**inputs):
    """Replay sealed reviewed source bindings and compose zones, or block.

    Raw-page hash matching is not a fresh HTML parse or image-authenticity proof.
    No caller-supplied 'validated' flag or submitted output metadata is trusted.
    """
    try:
        return _compose(**inputs)
    except SourceUnitError:
        raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as error:
        raise SourceUnitError('ordinary source composition refused: '+str(error)) from error


def _compose(**inputs):
    return _compose_verified_ordinary(_validated_context(**inputs))


def _validated_context(*,aggregate,scopes,observations,canonical_card,runtime_projection,
             adjudication_ledger,raw_pages,correction_document,condition_selections,
             old_input_revision,new_input_revision,expected_official_universe):
    return _validated_context_core(**locals())


def _validated_context_core(*,aggregate,scopes,observations,canonical_card,runtime_projection,
             adjudication_ledger,raw_pages,correction_document,condition_selections,
             old_input_revision,new_input_revision,expected_official_universe,
             _runtime_validator=None,_correction_binding=None,_face_validator=None):
    _require(isinstance(new_input_revision,str) and re.fullmatch('[0-9a-f]{64}',new_input_revision)
             and old_input_revision==new_input_revision, 'explicit equal source revision pins required')
    a = _json(aggregate)
    _require(set(scopes)=={'official','digimoncard_io'}, 'both complete source scopes required')
    typed_scopes = []
    for source,raw in scopes.items():
        d = _json(raw)
        _require(type(d['schema_version']) is int and d['schema_version']==1
                 and type(d['complete']) is bool and d['complete']
                 and type(d['expected_count']) is int and d['source']==source, 'invalid source scope headers')
        if source=='official':
            _require(d['scope']=='full-pool:official-catalog' and d['parser_version']=='official-html-v5'
                     and d['source_schema_version']=='world-card-list-html-v1', 'unsupported official source scope')
        typed_scopes.append(complete_candidate._manifest_from_dict(d))
    official_locations = next(scope.source_locations for scope in typed_scopes
                              if scope.source is models.SourceName.OFFICIAL)
    rebuilt = source_manifest.build_full_pool_source_manifest(typed_scopes).to_dict()
    _require(serialization.canonical_json_bytes(a)==serialization.canonical_json_bytes(rebuilt)
             and a['manifest_revision']==new_input_revision, 'aggregate differs from complete producer scope replay')
    official_ids = a['sources']['official']['gameplay_card_ids']
    _require(isinstance(expected_official_universe,(list,tuple))
             and len(expected_official_universe)==len(set(expected_official_universe))
             and set(expected_official_universe)==set(official_ids), 'independent official universe mismatch')
    record = _json(canonical_card)
    _require(type(record['schema_version']) is int and record['schema_version']==1, 'canonical schema')
    cid = record['card_id']
    _require(cid in official_ids, 'card lacks pinned official membership')
    card = complete_candidate._canonical_card_from_payload(cid,record,path=Path('<supplied-bytes>'))
    _require(card.admission is models.AdmissionStatus.ADMITTED, 'ordinary composition requires admitted canonical source')
    refs = {r['observation_id']:(source,r) for source,lane in a['sources'].items()
            for r in lane['observation_refs'] if r['gameplay_card_id']==cid}
    _require(set(observations)==set(refs), 'complete same-card source membership required')
    typed = {}; candidates = []; inventory = []; blockers = []; source_evidence = []
    for oid,raw in sorted(observations.items()):
        d = _json(raw);source,ref = refs[oid];lane = a['sources'][source]
        _require(type(d['schema_version']) is int and d['schema_version']==1
                 and d['observation_id']==oid and d['source']==source
                 and d['parser_version']==lane['parser_version']
                 and d['source_schema_version']==lane['source_schema_version'], 'observation source identity mismatch')
        observation = models.SourceObservation(**{**d,'source':models.SourceName(source)})
        _require(serialization.observation_digest(observation)==oid, 'observation bytes changed')
        candidate = normalization.observation_candidate(observation)
        _require(candidate.gameplay_card_id==cid and candidate.printing_id==ref['printing_id'], 'foreign card/printing')
        typed[oid]=observation;candidates.append(candidate)
        if source!='official':
            continue
        e=d['extracted'];origin=urlparse(d['source_url'])
        _require(origin.scheme=='https' and origin.netloc=='world.digimoncard.com'
                 and origin.path=='/cards/' and d['source_url'] in official_locations
                 and _sha(raw_pages[d['source_url']])==d['raw_payload_digest'], 'official raw source pairing mismatch')
        sections=e['text_sections']
        _require(isinstance(sections,list), 'explicit official section inventory required')
        source_evidence.append({'observation_id':oid,'printing_id':ref['printing_id'],
            'text_sections':deepcopy(sections),'layout':e.get('layout'),
            'source_url':d['source_url'],'raw_payload_digest':d['raw_payload_digest']})
        if e.get('layout')!='single' or len(e.get('faces',[]))>1:
            blockers.append({'code':'UNSUPPORTED_LAYOUT','observation_id':oid})
        elif e.get('faces'):
            _require(e['faces'][0]['face_id']=='primary' and e['faces'][0]['text_sections']==sections,
                     'primary face/source section mismatch')
        for index,section in enumerate(sections):
            _require(isinstance(section,dict) and isinstance(section.get('label'),str)
                     and isinstance(section.get('text'),str), 'invalid official section')
            item={'observation_id':oid,'printing_id':ref['printing_id'],'section_index':index,
                  'label':section['label'],'text':section['text']}
            inventory.append(item)
            if section['label'].lower() not in _LABELS:
                blockers.append({'code':'UNSUPPORTED_SECTION',**item})
            elif _unsupported_ordinary_text(section['text']):
                blockers.append({'code':'UNSUPPORTED_RULE_OR_LINK',**item})
        if any(e.get('rule_grants',{}).values()):
            blockers.append({'code':'UNSUPPORTED_RULE_GRANTS','observation_id':oid})
    pairs=[(p.printing_id,oid)for p in card.printings for oid in p.observation_ids]
    _require(len(pairs)==len(set(pairs)) and set(pairs)=={(ref['printing_id'],oid)for oid,(_,ref)in refs.items()}, 'canonical printing closure mismatch')
    ledger=_ledger(adjudication_ledger)
    replayed_card=normalization.normalize_card(cid,typed.values(),adjudications=ledger,
        old_input_revision=old_input_revision,new_input_revision=new_input_revision,
        field_selection_input_revision=new_input_revision)
    if _face_validator is None:
        _require(serialization.canonical_json_bytes(card.faces)==serialization.canonical_json_bytes(replayed_card.faces),
                 'canonical faces differ from producer replay')
    else:
        _require(_face_validator(card=card,replayed_card=replayed_card,typed=typed) is None,
                 'private full-record face validator must fail closed or return None')
    corrections=correction_document_from_bytes(correction_document)
    selections=selections_from_bytes(condition_selections)
    if any(r.card_id==cid for r in (*corrections.corrections,*corrections.compatibility_corrections)):
        blockers.append({'code':'UNSUPPORTED_CORRECTION'})
    if any(r.card_id==cid for r in selections):
        blockers.append({'code':'UNSUPPORTED_CONDITION_SELECTION'})
    runtime=_json(runtime_projection)
    _require(runtime.get('card_id')==cid, 'runtime card mismatch')
    kinds=[c for c in candidates if 'card_kind'in c.fields]
    kind,kind_provenance,kind_conflicts=normalization._select_field(cid,'card_kind',kinds,
        admission=models.AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=new_input_revision)
    _require(serialization.canonical_json_bytes(kind_provenance)==serialization.canonical_json_bytes(record['provenance']['card_kind'])
             and record['fields']['card_kind']==kind, 'canonical kind differs from producer replay')
    runtime_kind=runtime.get('card_kind')
    if type(runtime_kind)is int:
        runtime_kind={0:'Digimon',1:'Tamer',2:'Option',3:'Digi-Egg',4:'Dual'}.get(runtime_kind)
    if _runtime_validator is None:
        _require(type(runtime_kind)is str and runtime_kind==kind, 'runtime kind differs from source')
    else:
        _require(_runtime_validator(card=card,runtime=runtime,typed=typed,kind=kind) is None,
                 'private runtime validator must fail closed or return None')
    if any(c.requires_adjudication(models.AdmissionStatus.ADMITTED)for c in kind_conflicts):
        blockers.append({'code':'UNSUPPORTED_KIND_ADJUDICATION'})
    if len(card.faces)>1 or runtime.get('dual') or runtime.get('xros_req') or runtime.get('link_effect_description_eng') or card.fields.get('link_effect_description_eng'):
        blockers.append({'code':'UNSUPPORTED_STRUCTURAL_PROJECTION'})
    selected_fields = {}
    for field in _FIELDS:
        eligible=[c for c in candidates if field in c.fields]
        value,provenance,conflicts=normalization._select_field(cid,field,eligible,
            admission=models.AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=new_input_revision)
        if _correction_binding is not None and field==_correction_binding['unit']['field_path']:
            unit=_correction_binding['unit']
            _require(value==_correction_binding['original_selected_value'] and
                     serialization.canonical_json_bytes(provenance)==serialization.canonical_json_bytes(_correction_binding['original_provenance'])
                     and record['fields'][field]==unit['value'] and runtime[field]==unit['value']
                     and record['provenance'][field]==unit['provenance'],
                     'corrected field must bind exact original and corrected producer values')
            selected_fields[field]=(unit['value'],card.provenance[field],{'correction_binding':_correction_binding})
            continue
        _require(isinstance(value,str) and record['fields'][field]==value
                 and (_runtime_validator is not None or runtime[field]==value)
                 and serialization.canonical_json_bytes(record['provenance'][field])==serialization.canonical_json_bytes(provenance),
                 'selected field/provenance/runtime differs from producer replay')
        _require(provenance.authority is models.AuthorityClass.OFFICIAL and not provenance.derived, 'ordinary field requires selected official authority')
        binding={'selected_observation_id':provenance.selected_observation_id,
                 'provenance':serialization.canonicalize(provenance),'adjudication_id':None}
        if any(c.requires_adjudication(models.AdmissionStatus.ADMITTED)for c in conflicts):
            binding=bind_canonical_field(aggregate=aggregate,observations=observations,
                canonical_field=serialization.canonical_json_bytes({'card_id':cid,'field_path':field,'value':value,'provenance':record['provenance'][field]}),
                adjudication_ledger=adjudication_ledger,old_input_revision=old_input_revision,new_input_revision=new_input_revision)
        selected_fields[field]=(value,provenance,binding)
    selected_ids={provenance.selected_observation_id for _,provenance,_ in selected_fields.values()}
    # Retained alternative printings remain in the evidence inventory. Only
    # producer-selected sources are composed; A1 binds the exact full candidate
    # set whenever an actual printed-field conflict requires adjudication.
    security_value,_,security_binding=selected_fields['security_effect_description_eng']
    blockers,unselected_blockers=_partition_structure_blockers(
        blockers,selected_ids,security_binding,security_value)
    for item in inventory:
        item['selection_role']='selected-field-source' if item['observation_id'] in selected_ids else 'retained-alternative'
    result={'version':1,'kind':'ordinary-source-unit-composition','card_id':cid,
            'source_revision':new_input_revision,'inventory':inventory,'source_evidence':source_evidence,
            'retained_alternative_structures':unselected_blockers,'blockers':blockers,
            'status':'BLOCKED' if blockers else 'COMPOSED_ORDINARY','units':[],'zones':{},
            'input_sha256':{'aggregate':_sha(aggregate),'canonical_card':_sha(canonical_card),
                'runtime_projection':_sha(runtime_projection),'adjudications':_sha(adjudication_ledger),
                'corrections':_sha(correction_document),'selections':_sha(condition_selections),
                'scopes':{s:_sha(v)for s,v in sorted(scopes.items())},
                'observations':{oid:_sha(v)for oid,v in sorted(observations.items())},
                'raw_pages':{url:_sha(v)for url,v in sorted(raw_pages.items())}}}
    return {'result':result,'selected_fields':selected_fields,'typed':typed,
            'kind':kind,'card':card,'runtime':runtime}


def _compose_verified_ordinary(context, *, delegated_indices=None):
    """Internal projection after full input replay and explicit dispatch."""
    result=deepcopy(context['result'])
    if result['blockers']:
        return result
    selected_fields=context['selected_fields'];typed=context['typed'];kind=context['kind']
    inventory=result['inventory']
    for field,zone in _FIELDS.items():
        value,provenance,binding=selected_fields[field]
        if 'correction_binding' in binding:
            # Only the dedicated corrected entrypoint can construct this private binding.
            # It installs a distinct adjudicated unit after ordinary/condition/Rule dispatch.
            result['zones'][zone]={'field':field,'value':value,'source_binding':binding,
                'unit_indices':[],'presence':'correction-pending','spans':[]}
            continue
        oid=provenance.selected_observation_id;source=typed[oid];sections=source.extracted['text_sections']
        projected=_ordinary_sections(sections,kind,
            delegated_indices=(delegated_indices or {}).get(oid,set()))
        pieces=[(index,text,transform)for index,mapped,text,transform in projected if mapped==field]
        _require('\n'.join(text for _,text,_ in pieces)==value, 'unsupported lossless ordinary projection')
        refs_out=[];spans=[]
        for index,text,transform in pieces:
            source_unit=next(x for x in inventory if x['observation_id']==oid and x['section_index']==index)
            unit_index=len(result['units']);result['units'].append({**deepcopy(source_unit),'field':field,
                'projected_text':text,'transformation':transform})
            refs_out.append(unit_index)
            for local_index,span in enumerate(split_clauses(text)):
                spans.append({'unit_index':unit_index,'span_index':local_index,'kind':span.kind,
                              'timings':list(span.timings),'keyword':span.keyword,'text':span.text})
        result['zones'][zone]={'field':field,'value':value,'source_binding':binding,'unit_indices':refs_out,
            'presence':'nonempty' if value else 'absent-box' if not pieces else
                'printed-empty-box' if all(not sections[index]['text'].strip()for index,_,_ in pieces)
                else 'projected-empty-box','spans':spans}
    return result


def _ordinary_sections(sections,kind, *, delegated_indices=()):
    """Trace only explicitly supported routes, using producer render helpers."""
    result=[]
    explicit_security=any('security'in s['label'].lower()for s in sections)
    for index,s in enumerate(sections):
        if index in delegated_indices:
            # This occurrence is accounted by the condition dispatcher, not
            # removed from the verified source inventory or observation.
            from tools.card_data.compatibility_conditions import _CONDITION_LABELS
            _require(s['label'].lower() in _CONDITION_LABELS,
                     'only recognized condition occurrences may be delegated')
            continue
        label=s['label'].lower();field=_LABELS[label]
        text=normalization._render_effect_section(s['label'],s['text'])
        _require(text is not None,'ordinary section cannot be structural')
        transform='producer-render'
        if field=='effect_description_eng' and kind=='Digi-Egg':
            field='inherited_effect_description_eng';transform='producer-egg-zone'
        elif field=='effect_description_eng' and not explicit_security:
            text,security=normalization._split_one_shot_security_lines(text)
            result.extend((index,'security_effect_description_eng',part,'producer-embedded-security')for part in security)
        if field=='inherited_effect_description_eng' and kind in ('Option','Tamer') and text:
            # The producer's whole-field migration is conservative: do not guess
            # at section-level relocation in this first composition capability.
            raise SourceUnitError('unsupported non-inheritable inherited relocation')
        result.append((index,field,text,transform))
    return result


def validate_ordinary_card(binding,**inputs):
    expected=compose_ordinary_card(**inputs)
    _require(serialization.canonical_json_bytes(binding)==serialization.canonical_json_bytes(expected),
             'ordinary composition differs from complete source replay')
