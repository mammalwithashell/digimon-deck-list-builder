"""Pure, fail-closed source conservation diagnostics; never publishes approvals.

Serialized inputs use existing snapshot, canonical, runtime and clause-book
formats. A conserved text comparison alone is NOT a provenance proof. Current
clause books lack observation bindings, so those legs explicitly remain blocked.
"""
from __future__ import annotations
from collections import Counter, defaultdict
import hashlib
import html
import json
from pathlib import Path
import re
from urllib.parse import urlparse
from . import serialization, models, compatibility_selections, source_manifest
from .sources import official
from tools.clause_coverage import semantic_fingerprint
from .serialization import content_digest

VERSION = 'source-conservation-v1'
CONDITIONS = frozenset(('special play condition','special digivolution condition','special evolution condition','dna digivolution','digixros'))
NORMALIZATION = 'entities-once-ascii-formatting-whitespace-v1'
REQUIRED = frozenset(('snapshot','canonical','runtime','clauses','memberships','selections','corrections','source_manifest'))

def _sha(raw): return hashlib.sha256(raw).hexdigest()
def _fail(code, **details): return {'code':code, **details}
def _unique(pairs):
    result={}
    for key,value in pairs:
        if key in result: raise ValueError('duplicate JSON key: '+key)
        result[key]=value
    return result

def _normal(text):
    if not isinstance(text,str): raise ValueError('text must be a string')
    value=html.unescape(text)
    if re.search(r'&(?:#\w+|\w+);',value) or any(c.isspace() and c not in ' \t\r\n' for c in value):
        raise ValueError('unsupported entity or whitespace normalization')
    return re.sub(r'[ \t\r\n]+',' ',value).strip()

def rule_grant_check(sections, grants):
    """Independent completeness guard around the reviewed v5 Rule grammar."""
    failures=[];expected={'forms':[], 'traits':[], 'attributes':[]}
    grammars={'forms':official._RULE_FORM,'traits':official._RULE_TRAIT,'attributes':official._RULE_ATTRIBUTE}
    for index,section in enumerate(sections):
        for line in section['text'].splitlines():
            # Broad detection is only a refusal guard, never an extraction rule.
            if not re.search(r'\bRule\b', line, re.I): continue
            for category,pattern in grammars.items():
                marker={'forms':'Form','traits':'Type','attributes':'attribute'}[category]
                if not re.search(r'\b'+marker+r'\b',line,re.I): continue
                match=pattern.match(line)
                if match is None or line[match.end():].strip() not in ('','.'): 
                    failures.append(_fail('RULE_GRANT_UNSUPPORTED_SYNTAX',section_index=index,category=category,text=line));continue
                value=match.group(1).strip()
                if not value or '/' in value:
                    failures.append(_fail('RULE_GRANT_UNSUPPORTED_SYNTAX',section_index=index,category=category,text=line));continue
                if value not in expected[category]: expected[category].append(value)
    for category,values in expected.items():
        actual=grants.get(category)
        if not isinstance(actual,list) or Counter(actual)!=Counter(values):
            failures.append(_fail('RULE_FORM_PARSE_LOSS' if category=='forms' else 'RULE_GRANT_PARSE_LOSS',category=category,expected=values,actual=actual))
    return failures

def _reconstruct(clause):
    text=_normal(clause['text']);timings=clause.get('timings');keyword=clause.get('keyword');kind=clause.get('kind')
    if not isinstance(timings,list) or any(not isinstance(t,str) or any(c in t for c in '[]<>\uff1c\uff1e') for t in timings): raise ValueError('unsupported timing shape')
    if kind=='untimed' and not timings and keyword is None:return text
    if kind=='timing' and timings and keyword is None:return _normal(' '.join('['+t+']' for t in timings)+' '+text)
    # Keyword brackets and angle-printed timing reconstruction require a separate
    # reviewed inverse. Never guess punctuation or silently drop a keyword body.
    raise ValueError('unsupported lossless clause reconstruction')

def condition_check(sections, xros_req, clauses):
    """Compare complete units and occurrence multiplicity, not fuzzy keywords.

This comparator proves text conservation only. Report provenance is checked
separately and cannot be satisfied by coincidentally matching text.
"""
    failures=[]
    try:
        expected=[(s['label'],_normal(s['text'])) for s in sections if s['text'].strip()]
        projection=_normal('\n'.join('['+label+'] '+text for label,text in expected))
        if projection!=_normal(xros_req): failures.append(_fail('CONDITION_PROJECTION_LOSS',expected=projection,actual=xros_req))
        selected=[c for c in clauses if c.get('label','').lower() in CONDITIONS or c.get('label')=='xros_req']
        actual=[]
        for c in selected:
            if c.get('label')=='xros_req': raise ValueError('xros_req clause lacks original section identity')
            actual.append((c['label'],_reconstruct(c)))
        if actual!=expected:
            failures.append(_fail('CLAUSE_BODY_LOSS',expected=expected,actual=actual,clause_ids=[c['id'] for c in selected]))
    except (ValueError,KeyError,TypeError) as error:
        failures.append(_fail('CONDITION_NORMALIZATION_OR_RECONSTRUCTION_UNSUPPORTED',reason=str(error)))
    return failures

def build_conservation_report(*, inputs: dict[str,bytes], raw_pages: dict[str,bytes]):
    """Read exact serialized bytes and emit exhaustive diagnostics without I/O writes."""
    failures=[];rows=[]
    def fail(code,card_id=None,leg='integrity',**details):
        failures.append(_fail(code,card_id=card_id,leg=leg,**details))
    bindings={k:{'sha256':_sha(v),'bytes':len(v)} for k,v in sorted(inputs.items())}
    data={}
    for key in sorted(REQUIRED):
        try:data[key]=json.loads(inputs[key],object_pairs_hook=_unique)
        except (KeyError,ValueError,TypeError) as e:fail('INPUT_SCHEMA_INVALID',input=key,reason=str(e))
    memberships=data.get('memberships',{});universe=memberships.get('universe',[])
    if not isinstance(universe,list) or len(set(universe))!=len(universe) or any(not isinstance(c,str) or not models.CARD_ID_RE.fullmatch(c) for c in universe):
        fail('UNIVERSE_INVALID');universe=[]
    for key in ('focused','scenario_required'):
        ids=memberships.get(key)
        if not isinstance(ids,list) or len(set(ids))!=len(ids) or not set(ids)<=set(universe):fail('MEMBERSHIP_INVALID',membership=key)
    snapshot=data.get('snapshot',{});manifest=snapshot.get('manifest',{});observations=snapshot.get('observations',[]);by_card=defaultdict(list);by_id={}
    try:
        if manifest.get('complete') is not True or manifest.get('failed_ids')!=[]:fail('OFFICIAL_SCOPE_INCOMPLETE')
        for o in observations:
            oid=o['observation_id'];cid=o['extracted']['card_id'];by_card[cid].append(o)
            if oid in by_id:fail('OBSERVATION_DUPLICATE',cid,observation_id=oid)
            by_id[oid]=o
            if content_digest({k:o[k] for k in ('raw_payload_digest','parser_version','source_schema_version','extracted')},exclude_operational=True)!=oid:fail('OBSERVATION_IDENTITY_INVALID',cid,observation_id=oid)
            if o['source']!='official' or o['parser_version']!='official-html-v5' or o['source_schema_version']!='world-card-list-html-v1':fail('SOURCE_SCHEMA_UNREVIEWED',cid,observation_id=oid)
            origin=urlparse(o['source_url'])
            if origin.scheme!='https' or origin.netloc!='world.digimoncard.com' or origin.path!='/cards/':fail('SOURCE_ORIGIN_UNREVIEWED',cid,observation_id=oid)
            raw=raw_pages.get(o['source_url'])
            if raw is None or _sha(raw)!=o['raw_payload_digest']:fail('RAW_SOURCE_MISSING_OR_CHANGED',cid,observation_id=oid)
        refs=manifest.get('observation_refs',[])
        if len(refs)!=len(by_id) or {r['observation_id'] for r in refs}!=set(by_id) or set(manifest.get('observation_ids',[]))!=set(by_id):fail('PRINTING_CLOSURE_INVALID')
        for ref in refs:
            e=by_id.get(ref['observation_id'],{}).get('extracted',{})
            if ref['gameplay_card_id']!=e.get('card_id') or ref['printing_id']!=e.get('printing',{}).get('printing_id'):fail('PRINTING_REFERENCE_INVALID',ref['gameplay_card_id'])
        if set(by_card)!=set(universe) or manifest.get('expected_count')!=len(universe):fail('UNIVERSE_SNAPSHOT_MISMATCH')
    except (KeyError,TypeError,ValueError) as error:fail('OBSERVATION_SCHEMA_INVALID',reason=str(error))
    canonical=data.get('canonical',{});cards=canonical.get('cards',{});runtime=data.get('runtime',{});book=data.get('clauses',{});clauses=book.get('clauses',[])
    for name,keys in [('canonical',cards),('runtime',runtime),('clause_book',book.get('cards',[]))]:
        if set(keys)!=set(universe):fail('UNIVERSE_ARTIFACT_MISMATCH',artifact=name,missing=sorted(set(universe)-set(keys)),extra=sorted(set(keys)-set(universe)))
    if canonical.get('schema_version')!=1:fail('CANONICAL_SCHEMA_INVALID')
    if len({c.get('id') for c in clauses})!=len(clauses):fail('CLAUSE_ID_DUPLICATE')
    if any(c.get('card_id') not in universe for c in clauses):fail('CLAUSE_FOREIGN_CARD')
    try:
        fresh={c['id']:c for c in semantic_fingerprint.bind_clause_semantics(clauses)}
        for clause in clauses:
            if any(clause.get(k)!=fresh[clause['id']][k] for k in ('semantic_fingerprint_version','semantic_sha256','semantic_payload')):
                fail('CLAUSE_SEMANTIC_BINDING_MISMATCH',clause.get('card_id'),clause_id=clause.get('id'))
    except (ValueError,KeyError,TypeError) as error:fail('CLAUSE_SEMANTIC_BINDING_MISMATCH',reason=str(error))
    aggregate=data.get('source_manifest',{});revision=aggregate.get('manifest_revision');payload={k:v for k,v in aggregate.items() if k!='manifest_revision'}
    if content_digest(payload,exclude_operational=True)!=revision or canonical.get('source_manifest_digest')!=revision:fail('SOURCE_AGGREGATE_BINDING_INVALID')
    official_binding=aggregate.get('sources',{}).get('official',{})
    if set(official_binding.get('observation_ids',[]))!=set(by_id):fail('SOURCE_AGGREGATE_OFFICIAL_CLOSURE_INVALID')
    try:selections=compatibility_selections.selections_from_payload(data.get('selections',{}))
    except Exception as error:fail('SELECTION_SCHEMA_INVALID',reason=str(error));selections=()
    correction_payload=data.get('corrections',{})
    if (not isinstance(correction_payload,dict) or set(correction_payload)!={'schema_version','corrections'}
            or type(correction_payload.get('schema_version')) is not int or correction_payload['schema_version']!=1
            or not isinstance(correction_payload.get('corrections'),list)
            or any(not isinstance(c,dict) for c in correction_payload.get('corrections',[]))):
        fail('CORRECTION_SCHEMA_UNSUPPORTED',schema_version=correction_payload.get('schema_version') if isinstance(correction_payload,dict) else None)
        corrections=[]
    else:
        corrections=correction_payload['corrections']
    global_failures=list(failures)
    for cid in sorted(universe):
        start=len(failures);obs=by_card.get(cid,[]);card=cards.get(cid,{});record=runtime.get(cid,{});card_clauses=[c for c in clauses if c.get('card_id')==cid]
        row={'card_id':cid,'official_observation_ids':sorted(o['observation_id'] for o in obs),'condition_candidates':{},'condition_evidence':[],'rule_evidence':[]}
        if not obs:fail('OFFICIAL_EVIDENCE_MISSING',cid)
        if card.get('card_id')!=cid or record.get('card_id')!=cid:fail('CARD_IDENTITY_MISMATCH',cid)
        printing_ids={oid for p in card.get('printings',[]) for oid in p.get('observation_ids',[])}
        if not set(row['official_observation_ids'])<=printing_ids:fail('CANONICAL_PRINTING_EVIDENCE_MISSING',cid)
        provenance=card.get('provenance',{}).get('form_eng',{})
        own_ids=set(row['official_observation_ids'])
        consistent_printing_ids={oid for p in card.get('printings',[]) for oid in p.get('observation_ids',[])
            if oid in own_ids and p.get('printing_id')==by_id[oid]['extracted']['printing']['printing_id']}
        if (not provenance or provenance.get('selected_observation_id') not in provenance.get('observation_ids',[])
                or provenance.get('selected_observation_id') not in own_ids
                or not set(provenance.get('observation_ids',[]))<=own_ids
                or not set(provenance.get('observation_ids',[]))<=consistent_printing_ids):
            fail('CANONICAL_FORM_PROVENANCE_UNRESOLVED',cid)
        if any(c.get('card_id')==cid for c in corrections):fail('CORRECTION_UNSUPPORTED',cid)
        form_values=[]
        for o in obs:
            e=o['extracted'];sections=e.get('text_sections');grants=e.get('rule_grants')
            if not isinstance(sections,list) or not isinstance(grants,dict):fail('OFFICIAL_TEXT_SCHEMA_INVALID',cid);continue
            try:
                for f in rule_grant_check(sections,grants):fail(f.pop('code'),cid,'rule_text_to_grants',observation_id=o['observation_id'],**f)
                forms=list(dict.fromkeys([p.strip() for p in e.get('form','').split('/') if p.strip()]+grants.get('forms',[])))
                form_values.append(forms);row['rule_evidence'].append({'observation_id':o['observation_id'],'text_sections':sections,'rule_grants':grants,'expected_forms':forms})
                for field,expected in [('form_eng',forms),('type_eng',grants.get('traits',[])),('attribute_eng',grants.get('attributes',[]))]:
                    for target,value in [('CANONICAL',card.get('fields',{}).get(field)),('RUNTIME',record.get(field))]:
                        if field=='form_eng' and isinstance(value,list) and set(value)-set(expected):fail(target+'_FORM_UNSOURCED',cid,'grants_to_projection',expected=expected,actual=value,observation_id=o['observation_id'])
                        if not isinstance(value,list) or not set(expected)<=set(value):fail(target+'_FORM_LOSS' if field=='form_eng' else target+'_GRANT_LOSS',cid,'grants_to_projection',field=field,expected=expected,actual=value,observation_id=o['observation_id'])
                units=[s for s in sections if s['label'].lower() in CONDITIONS]
                value='\n'.join('['+s['label']+'] '+s['text'] for s in units if s['text'].strip());row['condition_candidates'][o['observation_id']]=value
            except (KeyError,TypeError,ValueError) as error:fail('OFFICIAL_TEXT_SCHEMA_INVALID',cid,reason=str(error))
        if form_values and any(v!=form_values[0] for v in form_values):fail('FORM_AUTHORITY_UNRESOLVED',cid)
        candidates=row['condition_candidates'];chosen=None
        matching=[s for s in selections if s.card_id==cid and s.source_manifest_revision==revision and dict(s.candidate_values)==candidates and s.candidate_set_digest==content_digest(candidates)]
        if not matching and any(s.card_id==cid for s in selections):fail('CONDITION_SELECTION_STALE',cid)
        if len(matching)==1:chosen=matching[0].selected_observation_id;row['selection_id']=matching[0].selection_id
        elif len(set(candidates.values()))==1 and not matching:chosen=next(iter(candidates),None)
        else:fail('CONDITION_AUTHORITY_UNRESOLVED',cid)
        if chosen:
            units=[s for s in by_id[chosen]['extracted']['text_sections'] if s['label'].lower() in CONDITIONS]
            row['condition_evidence']=[oid for oid,value in candidates.items() if value==candidates[chosen]]
            row['selected_conditions']=units;row['actual_xros_req']=record.get('xros_req');row['actual_condition_clauses']=[c for c in card_clauses if c.get('label','').lower() in CONDITIONS or c.get('label')=='xros_req']
            for f in condition_check(units,record.get('xros_req',''),card_clauses):fail(f.pop('code'),cid,'condition_conservation',**f)
            if any(s['text'].strip() for s in units):
                # Existing Clause serialization has no observation/source-input
                # binding. Do not invent a field or infer authority from text.
                fail('CLAUSE_SOURCE_PROVENANCE_MISSING',cid,'condition_provenance',clause_ids=[c['id'] for c in row['actual_condition_clauses']])
        row['failures']=failures[start:];row['global_failure_indices']=list(range(len(global_failures)));row['status']='BLOCKED' if row['failures'] or global_failures else 'CONSERVED';rows.append(row)
    dependencies={m.__name__:_sha(Path(m.__file__).read_bytes()) for m in (serialization,models,official,compatibility_selections,source_manifest,semantic_fingerprint)}
    return {'schema_version':1,'kind':'source_conservation','checker_version':VERSION,'checker_sha256':_sha(Path(__file__).read_bytes()),'dependency_sha256':dependencies,'normalization_policy':NORMALIZATION,'inputs':bindings,'input_revision':content_digest(bindings),'raw_page_sha256':{u:_sha(v) for u,v in sorted(raw_pages.items())},'memberships':memberships,'scope_count':len(universe),'rows':rows,'failures':failures,'status':'BLOCKED' if failures else 'CONSERVED','failure_counts':dict(sorted(Counter(f['code'] for f in failures).items())),'engine_execution_validated':False,'oracle_verdicts_modified':False}
