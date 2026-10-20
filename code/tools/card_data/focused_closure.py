"""Pure offline deck/scenario/QA closure, never runtime admission.

The externally reviewed manifest pins ORIGINAL inputs and directory membership.
A caller-computed digest is not authority. Source scopes establish retained
membership only: this module does not approve printed data, implement effects,
compile DSL, validate assertions, or create a complete behavioral denominator.
The original audited E/Q witness cannot be narrowed by this first-slice parser.
"""
from __future__ import annotations

from collections import Counter
from copy import deepcopy
import hashlib
import json
from pathlib import PurePosixPath
import re

import yaml

from .live_reconcile import _manifest_from_dict
from .source_manifest import build_full_pool_source_manifest


def _require(ok, message):
    if not ok:
        raise ValueError(message)


def _sha(value):
    return hashlib.sha256(value).hexdigest()


def _pairs(pairs):
    out = {}
    for k, v in pairs:
        _require(k not in out, f'duplicate key: {k}')
        out[k] = v
    return out


def _json(raw):
    def bad(value):
        raise ValueError(f'nonfinite JSON: {value}')
    return json.loads(raw.decode('utf-8'), object_pairs_hook=_pairs, parse_constant=bad)


class _Yaml(yaml.SafeLoader):
    pass


# serde_yml follows YAML1.2: yes/no/on/off are strings, notably `select: yes`.
# Never mutate SafeLoader's shared resolver registry in the hosting process.
_Yaml.yaml_implicit_resolvers = {
    key: [(tag, pattern) for tag,pattern in values if tag not in ('tag:yaml.org,2002:bool','tag:yaml.org,2002:int')]
    for key,values in yaml.SafeLoader.yaml_implicit_resolvers.items()
}
_Yaml.add_implicit_resolver('tag:yaml.org,2002:bool',
                            re.compile(r'^(?:true|True|TRUE|false|False|FALSE)$'), list('tTfF'))


_NATIVE_INTEGER = re.compile(r'^[+-]?(?:0|[1-9][0-9]*|0b[01]+|0o[0-7]+|0x[0-9a-fA-F]+)$')
_Yaml.add_implicit_resolver('tag:yaml.org,2002:int', _NATIVE_INTEGER, list('-+0123456789'))


def _native_integer(loader, node):
    text = loader.construct_scalar(node)
    _require(bool(_NATIVE_INTEGER.fullmatch(text)), 'unsupported YAML integer')
    sign = -1 if text.startswith('-') else 1
    value = text.lstrip('+-')
    base = {'0b':2,'0o':8,'0x':16}.get(value[:2],10)
    return sign * int(value[2:] if base != 10 else value, base)


_Yaml.add_constructor('tag:yaml.org,2002:int', _native_integer)


def _relative_path(value, *, root=False):
    _require(type(value) is str and bool(value) and ':' not in value and '\\' not in value
             and not PurePosixPath(value).is_absolute()
             and (value == '.' and root or all(part not in ('','.', '..') for part in value.split('/'))),
             'normalized relative input path required')


def _mapping(loader, node):
    # No merge keys/aliases may silently overwrite card/deck identities.
    return _pairs([(loader.construct_object(k), loader.construct_object(v)) for k,v in node.value])


_Yaml.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _mapping)


def _yaml(raw):
    _require(not any(isinstance(e, yaml.events.AliasEvent) for e in yaml.parse(raw)),
             'YAML aliases are unsupported')
    return yaml.load(raw.decode('utf-8'), Loader=_Yaml)


def _obj(value, required=(), allowed=None):
    _require(type(value) is dict and all(type(k) is str for k in value), 'object required')
    _require(set(required) <= set(value), 'missing fields: '+str(set(required)-set(value)))
    if allowed is not None:
        _require(set(value) <= set(allowed), 'unknown fields: '+str(set(value)-set(allowed)))
    return value


def _strings(value, *, unique=False):
    _require(type(value) is list and all(type(x) is str and x for x in value), 'string list required')
    _require(not unique or len(set(value)) == len(value), 'duplicate list identity')
    return value


def _ids(value, *, unique=False):
    result = _strings(value, unique=unique)
    _require(all(re.fullmatch(r'[A-Z][A-Z0-9]*-\d+', c) for c in result), 'card identity required')
    return result


def _integer(value, minimum=0, maximum=None):
    _require(type(value) is int and value >= minimum and (maximum is None or value <= maximum), 'strict nonnegative integer required')


def _native_scope(value):
    _obj(value)
    _require(type(value.get('schema_version')) is int and value['schema_version'] == 1,
             'source scope version')
    _require(type(value.get('complete')) is bool, 'source complete Boolean required')
    _integer(value.get('expected_count'))
    return _manifest_from_dict(value)


def _inventory(docs, roles):
    _obj(roles, ['official','secondary','canonical','runtime','qa'],
         ['official','secondary','canonical','runtime','qa'])
    aggregate = build_full_pool_source_manifest(
        [_native_scope(docs[roles['official']]), _native_scope(docs[roles['secondary']])])
    sets = {('official' if s.source.value == 'official' else 'secondary'): set(s.gameplay_card_ids)
            for s in aggregate.sources}
    canonical = _obj(docs[roles['canonical']], ['schema_version','cards'])
    _require(type(canonical['schema_version']) is int and canonical['schema_version'] == 1, 'canonical version')
    for name, records in [('canonical',canonical['cards']), ('runtime',docs[roles['runtime']])]:
        _obj(records)
        _ids(list(records), unique=True)
        for cid,row in records.items():
            _require(type(row) is dict and row.get('card_id') == cid, name+' card identity differs')
        sets[name] = set(records)
    qa = _obj(docs[roles['qa']], ['version','cards','qa'],
              ['version','source','card_count','qa_count','cards','qa','failed','conflicts'])
    _require(type(qa['version']) is int and qa['version'] == 1, 'QA version')
    _obj(qa['cards']); _obj(qa['qa'])
    for total,rows in [('card_count','cards'),('qa_count','qa')]:
        if total in qa:
            _integer(qa[total]);_require(qa[total]==len(qa[rows]),'QA declared count differs')
    sets['qa'] = set(_ids(list(qa['cards']), unique=True))
    for cid,qids in qa['cards'].items():
        _strings(qids, unique=True)
    for qid,row in qa['qa'].items():
        _obj(row, ['q_id','question','answer','card_ids'], ['q_id','question','answer','card_ids','date'])
        _require(row['q_id'] == qid and all(type(row[k]) is str for k in ['question','answer']), 'QA row identity/text')
        sets['qa'].update(_ids(row['card_ids'], unique=True))
    conflicts = qa.get('conflicts',[])
    _require(type(conflicts) is list, 'QA conflict list required')
    for conflict in conflicts:
        _obj(conflict, ['q_id','card_id'])
        _require(type(conflict['q_id']) is str and bool(conflict['q_id']), 'QA conflict identity')
        sets['qa'].update(_ids([conflict['card_id']]))
    sets['qa_failed'] = set(_ids(qa.get('failed',[]), unique=True))
    universe = set().union(*sets.values())
    inventory = {c: {axis:c in members for axis,members in sets.items()} for c in sorted(universe)}
    return inventory, qa, aggregate.manifest_revision


def _pool(doc):
    _obj(doc, ['decks'], ['decks','_comment','source','event_date','placement']); _require(type(doc['decks']) is list, 'deck list required')
    aliases = {}
    for d in doc['decks']:
        _obj(d, ['name','cards'], ['name','cards','eggs','source','event_date','placement','note'])
        _require(type(d['name']) is str and d['name'], 'deck name required')
        key=d['name'].lower()
        _require(key not in aliases, 'DUPLICATE_DECK_ALIAS')
        aliases[key]={'main':list(_ids(d['cards'])), 'eggs':list(_ids(d.get('eggs',[])))}
    return aliases


def _starters(doc):
    _obj(doc, ['starter_decks'], ['starter_decks','version','_generated_by'])
    _require(type(doc['starter_decks']) is list, 'starter list required')
    aliases={}
    for d in doc['starter_decks']:
        _obj(d, allowed=['id','name','set','main_deck','egg_deck','game_mode'])
        entry={'main':list(_ids(d.get('main_deck',[]))), 'eggs':list(_ids(d.get('egg_deck',[])))}
        local=set()
        for field in ('id','name','set'):
            alias=d.get(field,'');_require(type(alias) is str, 'starter alias type')
            if not alias: continue
            key=alias.lower()
            if key in local: continue  # Same row can intentionally repeat its name.
            _require(key not in aliases, 'DUPLICATE_DECK_ALIAS')
            local.add(key);aliases[key]=entry
    return aliases


def _campaign_counts(doc, name):
    _obj(doc,['scope','decks','cards'],['scope','decks','cards'])
    _require(type(doc['scope']) is str and type(doc['decks']) is list,'campaign pool schema')
    _ids(doc['cards'],unique=True)
    names=set();chosen=None
    for d in doc['decks']:
        _obj(d,['archetype','counts'],['archetype','counts','event_date','source','source_url','author','deck_id'])
        _require(type(d['archetype']) is str and d['archetype'],'campaign archetype required')
        key=d['archetype'].lower();_require(key not in names,'duplicate target archetype');names.add(key)
        _obj(d['counts']);_ids(list(d['counts']),unique=True)
        _require(bool(d['counts']),'empty target deck')
        for count in d['counts'].values():_integer(count,1)
        if key==name.lower():chosen=deepcopy(d['counts'])
    _require(chosen is not None,'unknown target archetype')
    return chosen


def _action_refs(action):
    """Parse identity-bearing action fields; no free-text/card-ID scanning."""
    _obj(action);_require(len(action)==1, 'one action required')
    verb,args=next(iter(action.items()));_obj(args)
    fields={'hatch':set(), 'pass':set(), 'move':{'from'}, 'play':{'card','from'},
            'digivolve':{'from','using'}, 'dna':{'card','from','materials'},
            'attack':{'attacker','target'}, 'main':{'on'}, 'link':{'card','from'},
            'select':{'cards','materials','targets','value','choice','yes','decline','ordinal',
                      'trigger','trigger_not','sim_only','dcgo_only'}}
    _require(verb in fields and set(args)<=fields[verb], 'unknown action grammar')
    refs=[]
    if verb in ('play','link','dna'): refs+=_ids([args.get('card')])
    if verb=='digivolve': refs+=_ids([args.get('using')])
    required={'digivolve':['from'], 'attack':['attacker','target'], 'main':['on']}.get(verb,[])
    for key in required: _require(type(args.get(key)) is str and args[key], 'action field required')
    for key in ('from','on','attacker','target'):
        if key in args: _require(type(args[key]) is str and args[key], 'slot string required')
    if verb=='dna':
        materials=_strings(args.get('materials'))
        _require(len(materials)==2 and all(re.fullmatch(r'(own\.)?field\.\d+', s) for s in materials), 'DNA materials require two own slots')
    if verb=='select':
        forms=set(args)&{'cards','materials','targets','value','choice','yes','decline'}
        _require(len(forms)==1, 'select requires one form')
        if 'cards' in args:
            refs+=_ids(args['cards']);_require(bool(args['cards']), 'empty card selection')
        if 'materials' in args:
            # Current Rust Materials are identity picks (not DNA slot arguments).
            refs+=_ids(args['materials']);_require(bool(args['materials']), 'empty materials')
        if 'targets' in args:_strings(args['targets'])
        for key in ('yes','decline','sim_only','dcgo_only'):
            if key in args:_require(type(args[key]) is bool, 'select Boolean required')
        if 'value' in args:_integer(args['value'], -(2**31), 2**31-1)
        if 'ordinal' in args:
            _integer(args['ordinal'], 0, 2**31-1)
            _require('cards' in args and len(args['cards'])==1, 'ordinal requires one card')
        _require(not(args.get('sim_only') and args.get('dcgo_only')), 'selection cannot be both wire-only')
        if 'choice' in args:_require(type(args['choice']) is str and args.get('sim_only') is True, 'choice requires sim_only')
    return refs


def _scenario(doc):
    _obj(doc, ['card','clause','seed','decks','steps'],
         ['card','clause','seed','decks','steps','covers','interaction','assert','expect_ruling','meta'])
    _ids([doc['card']]);_integer(doc['seed'], 0, 2**64-1)
    def clause(value):
        _require(type(value) is str and re.fullmatch(r'[A-Z][A-Z0-9]*-\d+#(?:effect|inherited|security|link)#\d+',value), 'clause identity required')
        return value.split('#')[0]
    _require(clause(doc['clause'])==doc['card'], 'foreign primary clause')
    refs=[doc['card']]
    covers=doc.get('covers',[doc['clause']]);_strings(covers,unique=True)
    _require(doc['clause'] in covers, 'covers omits primary')
    evidence=[clause(c) for c in covers]
    _obj(doc['decks'], ['p0','p1'], ['p0','p1'])
    for seat in doc['decks'].values():
        _obj(seat,['rest'],['rest','stack']);_require(type(seat['rest']) is str and seat['rest'], 'rest name required')
        refs+=_ids(seat.get('stack',[]))
    _require(type(doc['steps']) is list and doc['steps'], 'scenario needs steps')
    for step in doc['steps']:
        _obj(step,['actor','do'],['actor','do','expect'])
        _require(type(step['actor']) is int and step['actor'] in (0,1), 'actor must be0/1')
        refs+=_action_refs(step['do'])
        if step.get('expect') is not None:
            expect=_obj(step['expect'], allowed=['prompt','count','candidates'])
            if expect.get('count') is not None:_integer(expect['count'], 0, 2**16-1)
            if expect.get('prompt') is not None:_require(type(expect['prompt']) is str, 'prompt string required')
            if 'candidates' in expect:_strings(expect['candidates'])
    def assertions(rows):
        _require(type(rows) is list, 'assertion list required')
        for row in rows:
            _obj(row,['at','that'],['at','that']);_integer(row['at'],0,2**32-1);_obj(row['that'])
            _require(row['at']<=len(doc['steps']), 'assertion step outside scenario')
    assertions(doc.get('assert',[]))
    if doc.get('expect_ruling') is not None:
        ruling=_obj(doc['expect_ruling'],['q_id','assert'],['q_id','assert'])
        _require(type(ruling['q_id']) is str and bool(ruling['q_id']), 'ruling question required')
        assertions(ruling['assert'])
    # Wire shapes/ranges only; assertion bodies are not evaluated or admitted.
    return refs,evidence


def resolve_closure(*, original_inputs, manifest_bytes, expected_manifest_sha256, supplied_inventory=None):
    """Resolve pinned original membership, ordered decks and shared QA.

    Bad pins/envelopes raise ValueError. Resolvable content failures are retained
    as blockers. The output is ALWAYS offline and incomplete for implementation
    closure, even if all supported deck/QA checks pass.
    """
    _require(type(manifest_bytes) is bytes and type(expected_manifest_sha256) is str and
             re.fullmatch(r'[0-9a-f]{64}',expected_manifest_sha256) and _sha(manifest_bytes)==expected_manifest_sha256,
             'external manifest byte pin mismatch')
    manifest=_obj(_json(manifest_bytes), ['schema_version','files','inventories','scope_witness','starter','scenarios','pools','explicit_books','target_decks','implementation_files'],
                  ['schema_version','files','inventories','scope_witness','starter','scenarios','pools','explicit_books','target_decks','implementation_files','exam_roots'])
    _require(type(manifest['schema_version']) is int and manifest['schema_version']==1,'unknown resolver schema')
    pins=_obj(manifest['files']);_obj(original_inputs)
    _require(set(pins)==set(original_inputs),'original pinned file inventory differs')
    for path,pin in pins.items():
        _relative_path(path)
        _require(type(original_inputs[path]) is bytes and type(pin) is str and _sha(original_inputs[path])==pin,'original byte pin mismatch: '+path)
    for k in ('scenarios','pools','implementation_files'):_strings(manifest[k],unique=True)
    roots=_strings(manifest.get('exam_roots',['.']),unique=True)
    _require(bool(roots), 'exam root required')
    for root in roots:
        _relative_path(root, root=True)
    _obj(manifest['explicit_books']);_require(type(manifest['target_decks']) is list,'target deck list required')
    _require(set(manifest['explicit_books'])<=set(manifest['scenarios']), 'explicit book names unknown scenario')
    used=set(manifest['inventories'].values())|{manifest['scope_witness']}|set(manifest['scenarios'])|set(manifest['pools'])|set(manifest['implementation_files'])
    if manifest['starter'] is not None:used.add(manifest['starter'])
    _require(used<=set(pins),'manifest references unpinned file')
    docs={p:_json(original_inputs[p]) for p in set(manifest['inventories'].values())|{manifest['scope_witness']}}
    inventory,qa,source_revision=_inventory(docs,manifest['inventories'])
    witness=_obj(docs[manifest['scope_witness']],['E','Q'])
    witness_e=set(_ids(witness['E'],unique=True));witness_q=set(_ids(witness['Q'],unique=True))
    blockers=[];edges=[];e=set();q=set();results={};requested_questions=set();targets=[]
    def block(code,**details):blockers.append(dict(code=code,**details))
    def add(ids,reason,path,target):
        for cid in ids:
            target.add(cid);edges.append(dict(card_id=cid,reason=reason,input=path,sha256=pins[path]))
    pools={}
    for p in manifest['pools']:
        try:pools[p]=_pool(_json(original_inputs[p]))
        except (ValueError,TypeError,KeyError) as ex:block('DUPLICATE_DECK_ALIAS' if 'DUPLICATE_DECK_ALIAS' in str(ex) else 'POOL_SCHEMA',input=p,detail=str(ex))
    starter={}
    if manifest['starter'] is not None:
        try:starter=_starters(_json(original_inputs[manifest['starter']]))
        except (ValueError,TypeError,KeyError) as ex:block('DUPLICATE_DECK_ALIAS' if 'DUPLICATE_DECK_ALIAS' in str(ex) else 'STARTER_SCHEMA',detail=str(ex))
    for target in manifest['target_decks']:
        _obj(target,['pool','name'],['pool','name','format']);p=target['pool'];name=target['name']
        _require(p in pins and type(name) is str,'target pool/name invalid')
        try:
            form=target.get('format','harness-pool')
            _require(form in ('harness-pool','campaign-counts'),'unknown target format')
            if form=='campaign-counts':
                counts=_campaign_counts(_json(original_inputs[p]),name)
                targets.append(dict(input=p,name=name,counts=counts))
                add(counts,'target_deck',p,e)
            else:
                _require(p in manifest['pools'],'target harness pool not in catalog')
                entry=pools.get(p,{}).get(name.lower())
                if entry is None:block('UNKNOWN_TARGET_DECK',input=p,name=name)
                else:
                    targets.append(dict(input=p,name=name,**deepcopy(entry)))
                    add(entry['main']+entry['eggs'],'target_deck',p,e)
        except (ValueError,TypeError,KeyError) as ex:block('TARGET_SCHEMA',input=p,detail=str(ex))
    for path in manifest['scenarios']:
        try:doc=_yaml(original_inputs[path]);refs,covered=_scenario(doc)
        except (ValueError,TypeError,KeyError,yaml.YAMLError) as ex:
            block('SCENARIO_SCHEMA',input=path,detail=str(ex));continue
        add(refs,'scenario_identity',path,e);add(covered,'covered_clause',path,q)
        if doc.get('interaction') is not None:
            try:
                inter=_obj(doc['interaction'],['id','source','kind'],['id','source','kind'])
                _require(inter['source'] in ('qa','probe') and inter['kind'] in ('positive','negative'), 'unknown interaction schema')
                _require(type(inter['id']) is str and inter['id'].startswith(inter['source']+':'),'interaction identity/source')
                if inter['source']=='qa':requested_questions.add(inter['id'][3:])
            except (ValueError,TypeError) as ex:block('SCENARIO_SCHEMA',input=path,detail=str(ex))
        rests=[s['rest'].lower() for s in doc['decks'].values()]
        matching_roots=[r for r in roots if PurePosixPath(path).is_relative_to(PurePosixPath(r))]
        _require(len(matching_roots)==1,'scenario must belong to exactly one exam root')
        root=PurePosixPath(matching_roots[0])
        matches=sorted((p for p,v in pools.items() if PurePosixPath(p).is_relative_to(root) and all(n in v for n in rests)),
                       key=lambda p:(PurePosixPath(p).parent!=PurePosixPath(path).parent,p))
        chosen=manifest['explicit_books'].get(path)
        if chosen is not None:_require(chosen in manifest['pools'],'explicit book must be pinned pool')
        elif matches:
            chosen=matches[0]
            # Preserve actual first-match rule, but ambiguous ownership is not approved.
            beside=[p for p in matches if PurePosixPath(p).parent==PurePosixPath(path).parent]
            same_rank=beside if beside else matches
            if len(same_rank)>1:block('AMBIGUOUS_BOOK',input=path,candidates=matches,selected_by_rust=chosen)
        aliases=deepcopy(starter);aliases.update(pools.get(chosen,{}))
        results[path]={'selected_book':chosen,'matching_books':matches,'seats':{},'raw_sha256':pins[path]}
        for name,seat in doc['decks'].items():
            entry=aliases.get(seat['rest'].lower())
            if entry is None:block('UNKNOWN_DECK',input=path,seat=name,rest=seat['rest']);continue
            source=chosen if chosen is not None and seat['rest'].lower() in pools.get(chosen,{}) else manifest['starter']
            add(entry['main']+entry['eggs'],'scenario_rest:'+name,source,e)
            stack=seat.get('stack',[]);remaining=entry['main']+entry['eggs'];missing=Counter(stack)-Counter(remaining)
            result=deepcopy(entry);result['stack']=list(stack);result['ordered']=None;results[path]['seats'][name]=result
            if not remaining:block('EMPTY_DECK',input=path,seat=name)
            if missing:block('INVALID_STACK',input=path,seat=name,missing=dict(missing));continue
            for cid in stack:remaining.remove(cid)
            result['ordered']=remaining+list(reversed(stack))
    classified=e|q
    add(sorted(witness_e),'audited_scope_witness',manifest['scope_witness'],e)
    add(sorted(witness_q),'audited_scope_witness',manifest['scope_witness'],q)
    for cid in sorted((witness_e|witness_q)-classified):block('AUDITED_REFERENCE_UNCLASSIFIED',card_id=cid)
    q|=e
    members={qid:set(row['card_ids']) for qid,row in qa['qa'].items()}
    for cid,qids in qa['cards'].items():
        for qid in qids:
            if qid not in members:block('QA_REFERENCE_MISSING',card_id=cid,question_id=qid)
            else:members[qid].add(cid)
    for conflict in qa.get('conflicts',[]):
        qid = conflict['q_id']
        if qid in members: members[qid].add(conflict['card_id'])
    selected={};changed=True
    for qid in sorted(requested_questions):
        if qid not in members:block('QA_REFERENCE_MISSING',question_id=qid)
        else:
            selected[qid]={'row':deepcopy(qa['qa'][qid]),'members':sorted(members[qid])}
            add(sorted(members[qid]),'explicit_qa:'+qid,manifest['inventories']['qa'],q)
    while changed:
        changed=False
        for qid,ids in members.items():
            if ids&q and qid not in selected:
                selected[qid]={'row':deepcopy(qa['qa'][qid]),'members':sorted(ids)}
                add(sorted(ids),'shared_qa:'+qid,manifest['inventories']['qa'],q);changed=True
    for conflict in qa.get('conflicts',[]):
        if conflict['q_id'] in selected or conflict['card_id'] in e|q:
            block('QA_CONFLICT', conflict=deepcopy(conflict))
    for cid in sorted((e|q)-set(inventory)):block('MEMBERSHIP_MISSING',card_id=cid)
    for cid in sorted((e|q)&set(qa.get('failed',[]))):block('QA_UNAVAILABLE',card_id=cid)
    if supplied_inventory is not None:
        _ids(supplied_inventory,unique=True)
        if set(supplied_inventory)!=set(inventory):block('SUPPLIED_INVENTORY_MISMATCH',missing=sorted(set(inventory)-set(supplied_inventory)),extra=sorted(set(supplied_inventory)-set(inventory)))
    # This first slice deliberately has no authority to infer DSL dynamic paths.
    block('IMPLEMENTATION_REFERENCE_ANALYSIS_REQUIRED',card_ids=sorted(e),inputs=list(manifest['implementation_files']))
    return {'schema_version':1,'status':'OFFLINE_CLOSURE_ONLY_NOT_ADMISSION',
            'manifest_sha256':expected_manifest_sha256,'input_sha256':deepcopy(pins),
            'source_revision':source_revision,'inventory':inventory,
            'retained_audited_scope':{'E':sorted(witness_e),'Q':sorted(witness_q),
                                      'unresolved':deepcopy(witness.get('unresolved',[]))},
            'original_runtime_indices':{c:{k:deepcopy(v) for k,v in row.items() if k in ('index','card_index','norm_id')}
                                        for c,row in docs[manifest['inventories']['runtime']].items()},
            'E':sorted(e),'Q':sorted(q),
            'scenarios':results,'targets':targets,'qa':selected,'qa_conflicts':deepcopy(qa.get('conflicts',[])),'reason_edges':edges,'blockers':blockers,
            'full_clause_count':None,'full_gating_interaction_count':None,
            'unimplemented':['DSL-generated references','source validity','runtime consumed fields',
                             'assertion semantics','complete clause and interaction denominator','runtime admission']}
