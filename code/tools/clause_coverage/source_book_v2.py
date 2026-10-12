"""Opt-in review-only v2 source book. No consumer/admission or verdict migration.

Only build_source_book_v2 accepts authority inputs and reconstructs composers.
Private compilation and wire validation establish internal consistency only.
Original source evidence, raw/projected views and unknown membership survive.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from . import diagnostic_source_book as diagnostic
from .condition_app_fusion import split_app_fusion_condition_units
from .text_split import split_clauses


def _require(ok,message):
    if not ok:raise ValueError(message)


def _sha(raw):return hashlib.sha256(raw).hexdigest()


def _object_bytes(value):
    return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode('utf-8')


def wire_bytes(value):
    """Language-neutral arrays only; exact Unicode/CRLF, no coercion."""
    def check(v):
        if v is None or type(v)in (str,int):return
        if type(v)is list:
            for x in v:check(x)
            return
        raise ValueError('wire payload allows only arrays, strings, integers and null')
    check(value)
    return json.dumps(value,ensure_ascii=False,separators=(',',':'),allow_nan=False).encode('utf-8')


def compiler_dependencies():
    return {**diagnostic.diagnostic_dependencies(),__name__:_sha(Path(__file__).read_bytes())}


def _span(value):
    _require(isinstance(value,dict) and value.get('kind')in ('timing','keyword','untimed'),'unknown span kind')
    _require(type(value.get('timings'))is list and all(type(x)is str for x in value['timings']),'span timings')
    _require(value.get('keyword')is None or type(value['keyword'])is str,'span keyword')
    _require('keyword'in value and type(value.get('text'))is str,'complete span required')
    return [value['kind'],value['timings'],value['keyword'],value['text']]


def _split(text):
    _require(type(text)is str,'exact projected text required')
    return [[s.kind,list(s.timings),s.keyword,s.text]for s in split_clauses(text)]


def _condition(unit):
    rows=split_app_fusion_condition_units([unit])
    _require(len(rows)==1 and rows[0]['source_unit']==unit,'condition replay source differs')
    return [_span(s)for s in rows[0]['spans']]


def _reconcile(cid,row):
    source=row['source_composition']
    units,views,obligations,conservation=diagnostic._descriptors(cid,source)
    _require(units==row['source_unit_descriptors'] and sorted(views,key=_object_bytes)==sorted(row['span_descriptors'],key=_object_bytes)
             and obligations==row['structural_obligations'] and conservation==row['conservation'],
             'diagnostic descriptors differ from complete composition')
    _require(row['span_count']==len(views),'descriptor count differs')
    by_index={u['unit_index']:u for u in units}
    selected={};conditions=(source.get('condition_binding')or{}).get('units',[])
    condition_by_key={(u['observation_id'],u['section_index']):u for u in conditions}
    _require(len(condition_by_key)==len(conditions),'duplicate condition source')
    for unit in conditions:
        _require([_span(s)for s in unit['spans']]==_split(unit['text']),'condition raw local span replay differs')
    used=set()
    if source['status']=='COMPOSED_DUAL_SOURCE':
        _require(not source.get('zones'),'Dual cannot also emit ordinary zones')
        for i,wrapped in by_index.items():
            u=wrapped['source_unit'];role=u['role']
            _require(role in ('condition','effect','inherited','security','arts-reminder'),'unknown Dual role')
            if role=='arts-reminder':
                _require(not u['spans'],'structural reminder must not execute');used.add(i);continue
            if role=='condition':
                key=(u['observation_id'],u['section_index']);_require(key in condition_by_key,'missing Dual condition binding')
                expected=_condition(condition_by_key[key])
            else:expected=_split(u['text'])
            _require([_span(s)for s in u['spans']]==expected,'Dual span replay differs')
            selected[i]=(role,expected);used.add(i)
    else:
        _require(source['status']in ('COMPOSED_RULE_CARD','COMPOSED_CORRECTED_RULE_CARD'),'unsupported composed route')
        zones=source['zones'];_require(set(zones)<= {'effect','inherited','security','conditions'},'unknown source zone')
        for zone,z in zones.items():
            indices=z['unit_indices'];_require(type(indices)is list and all(type(i)is int and i in by_index for i in indices),'invalid zone unit indices')
            _require(len(indices)==len(set(indices)) and not used.intersection(indices),'duplicate source unit consumption')
            expected_rows=[]
            for i in indices:
                u=by_index[i]['source_unit']
                if zone=='conditions':
                    key=(u['observation_id'],u['section_index']);_require(key in condition_by_key,'missing condition source binding')
                    bound=condition_by_key[key]
                    _require(all(u.get(k)==v for k,v in bound.items()),'condition unit differs from exact bound source')
                    expected=_condition(bound)
                else:expected=_split(u['projected_text'])
                selected[i]=(zone,expected);used.add(i)
                expected_rows.extend((i,n,span)for n,span in enumerate(expected))
            actual=[]
            for s in z['spans']:
                _require(type(s.get('unit_index'))is int and type(s.get('span_index'))is int,'strict span indices required')
                actual.append((s['unit_index'],s['span_index'],_span(s)))
            _require(actual==expected_rows,'projected zone differs from reviewed segmentation replay')
    _require(used==set(by_index),'source unit missing from executable/structural/empty inventory')
    result=[]
    for i,(zone,spans)in sorted(selected.items()):
        u=by_index[i]
        for n,span in enumerate(spans):
            # Preserve every printed occurrence, but reconcile its redundant views.
            matches=[v for v in views if v['unit_key']==u['key'] and v['occurrence_span_index']==n]
            _require(matches,'missing diagnostic span view')
            result.append({'unit_key':deepcopy(u['key']),'face':u['face'],'zone':zone,'local_span_index':n,
                'span':deepcopy(span),'descriptor_views':deepcopy(matches)})
    return result


def _materialize(d,deps):
    _require(type(d.get('schema_version'))is int and d['schema_version']==1
             and d.get('kind')=='bounded-source-composition-diagnostic-v1'
             and d.get('status')=='DIAGNOSTIC_ONLY_NOT_ADMISSION','reviewed diagnostic shape required')
    _require(d['full_span_count']is None and d['full_gating_interaction_count']is None,'unexpected completeness claim')
    generation_payload=['digimon-source-generation',2,d['snapshot_sha256'],d['input_manifest_sha256'],d['source_revision'],
                        _sha(_object_bytes(d['dependency_sha256'])),_sha(_object_bytes(deps))]
    generation=_sha(wire_bytes(generation_payload))
    result={'book_schema_version':2,'semantic_fingerprint_version':2,'status':'REVIEW_ONLY_NOT_ADMISSION',
        'source_generation_sha256':generation,'source_generation_payload':generation_payload,
        'compiler_dependencies':deepcopy(deps),'membership_counts':deepcopy(d['membership_counts']),
        'full_clause_count':None,'full_gating_interaction_count':None,'cards':{},'clauses':[],
        'diagnostic_evidence':deepcopy(d)}
    for cid,row in sorted(d['cards'].items()):
        card={'status':row['status'],'membership':deepcopy(row['membership']),'blockers':deepcopy(row['blockers']),
              'clause_count':None,'clause_ids':[],'gating_interaction_count':None}
        result['cards'][cid]=card
        if row['status']!='COMPOSED_DIAGNOSTIC':
            _require(row['status']in ('BLOCKED','NOT_EVALUATED'),'unknown completeness status');continue
        _require(cid in d['evaluated_card_ids'] and not row['blockers'],'unselected or blocked success')
        occurrences=_reconcile(cid,row)
        closure=_sha(_object_bytes(row))
        siblings=[[o['unit_key'],o['face'],o['zone'],o['local_span_index'],o['span']]for o in occurrences]
        card.update(status='COMPILED_REVIEW_ONLY',clause_count=len(occurrences),source_closure_sha256=closure,
            structural_obligations=deepcopy(row['structural_obligations']),
            superseded_source_occurrences=sum(x['disposition']=='superseded-by-reviewed-correction'for x in row['source_composition']['consumption']))
        for o in occurrences:
            locator=[o['face'],o['unit_key'],o['zone'],o['local_span_index']]
            ident=cid+'#v2#'+_sha(wire_bytes(locator))
            payload=['digimon-clause-semantics',2,generation,cid,locator,o['span'],siblings,closure]
            span=o['span'];result['clauses'].append({'id':ident,'card_id':cid,'face':o['face'],'unit_key':o['unit_key'],
                'zone':o['zone'],'local_span_index':o['local_span_index'],'kind':span[0],'timings':span[1],
                'keyword':span[2],'text':span[3],'descriptor_views':o['descriptor_views'],
                'semantic_fingerprint_version':2,'semantic_payload':payload,'semantic_sha256':_sha(wire_bytes(payload))})
            card['clause_ids'].append(ident)
    ids=[r['id']for r in result['clauses']];_require(len(ids)==len(set(ids)),'duplicate v2 clause identity')
    return result


def _compile_replayed_diagnostic(d):
    """Internal only: tests may exercise historical diagnostic consistency."""
    return _materialize(d,compiler_dependencies())


def validate_wire_book_v2(book):
    """Reconstruct wire contents; does NOT establish external source authority."""
    _require(type(book.get('book_schema_version'))is int and book['book_schema_version']==2,'unknown book version')
    _require(type(book.get('semantic_fingerprint_version'))is int and book['semantic_fingerprint_version']==2,'unknown semantic version')
    expected=_materialize(book['diagnostic_evidence'],book['compiler_dependencies'])
    _require(_object_bytes(expected)==_object_bytes(book),'v2 book differs from reconstructed contents')


def _input_state(candidate_root,input_root,extraction_artifact,input_manifest):
    """Exact disk closure before/after composition, including owned observations."""
    candidate=Path(candidate_root).resolve(strict=True);root=Path(input_root).resolve(strict=True)
    extraction=json.loads(extraction_artifact);manifest=json.loads(input_manifest);state={}
    def capture(base,d,key):
        raw=diagnostic._file(base,d);state[key]=_sha(raw);return raw
    for role,desc in extraction['files'].items():capture(candidate,desc,'candidate:'+role)
    descriptors={**{'raw:'+k:v for k,v in manifest['raw_pages'].items()},**{'image:'+k:v for k,v in manifest['images'].items()},'qa':manifest['qa']}
    for cid,auth in manifest['authorizations'].items():descriptors.update({cid+':'+k:v for k,v in auth.items()})
    for key,desc in descriptors.items():capture(root,desc,key)
    aggregate=json.loads((candidate/extraction['files']['aggregate']['path']).read_bytes())
    for source,lane in aggregate['sources'].items():
        for ref in lane['observation_refs']:
            if ref['gameplay_card_id']not in manifest['evaluated_card_ids']:continue
            oid=ref['observation_id'];path=(candidate/'data/card_sources/observations'/source/(oid+'.json')).resolve(strict=True)
            _require(path.is_relative_to(candidate),'escaped observation path');state['observation:'+oid]=_sha(path.read_bytes())
    return state


def build_source_book_v2(*,candidate_root,input_root,extraction_artifact,expected_extraction_sha256,
                         input_manifest,expected_input_manifest_sha256):
    """Reconstruct all reviewed inputs internally; emit review-only v2 contents."""
    deps=compiler_dependencies()
    _require(type(extraction_artifact)is bytes and _sha(extraction_artifact)==expected_extraction_sha256,'extraction pin mismatch')
    _require(type(input_manifest)is bytes and _sha(input_manifest)==expected_input_manifest_sha256,'input manifest pin mismatch')
    before=_input_state(candidate_root,input_root,extraction_artifact,input_manifest)
    d=diagnostic.build_diagnostic_source_book(candidate_root=candidate_root,input_root=input_root,
        extraction_artifact=extraction_artifact,expected_extraction_sha256=expected_extraction_sha256,
        input_manifest=input_manifest,expected_input_manifest_sha256=expected_input_manifest_sha256)
    result=_materialize(d,deps)
    _require(before==_input_state(candidate_root,input_root,extraction_artifact,input_manifest),'source inputs changed during compilation')
    _require(deps==compiler_dependencies(),'compiler dependencies changed during compilation')
    return result
