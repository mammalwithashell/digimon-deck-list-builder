"""Offline resolver fixtures. Synthetic inventories are not source approvals."""
import hashlib
import json
from copy import deepcopy
from pathlib import Path

import pytest
import yaml

F = Path(__file__).parents[1] / 'fixtures/card_data/focused_closure'


def raw(obj):
    return json.dumps(obj, ensure_ascii=False, sort_keys=True).encode()


def sha(b):
    return hashlib.sha256(b).hexdigest()


def scope(source, ids):
    refs = [{'observation_id': sha((source+c).encode()), 'gameplay_card_id': c,
             'printing_id': c} for c in ids]
    return dict(schema_version=1, source=source, scope='full-pool:test',
                source_locations=['fixture://test'], first_acquired_at='2026-01-01',
                parser_version='fixture-v1', source_schema_version='fixture-v1',
                observation_ids=[r['observation_id'] for r in refs], observation_refs=refs,
                complete=True, expected_count=len(ids), failed_ids=[])


def fixture():
    ids = ['BT1-001', 'BT1-002', 'BT1-003', 'BT1-004']
    s = dict(card=ids[0], clause=ids[0]+'#effect#0', seed=1,
             decks={'p0': {'rest': 'test', 'stack': [ids[1], ids[0]]},
                    'p1': {'rest': 'ST1', 'stack': []}},
             steps=[{'actor': 0, 'do': {'play': {'card': ids[1]}}}])
    docs = {'official.json': raw(scope('official', ids)),
            'secondary.json': raw(scope('digimoncard_io', ids)),
            'canonical.json': raw({'schema_version': 1, 'cards': {c: {'card_id': c} for c in ids}}),
            'runtime.json': raw({c: {'card_id': c, 'index': i+1} for i,c in enumerate(ids)}),
            'qa.json': raw({'version': 1, 'cards': {c: [] for c in ids}, 'qa': {}}),
            'witness.json': raw({'E': [ids[0]], 'Q': [ids[0]]}),
            'starter.json': raw({'starter_decks': [{'id': 'starter', 'name': 'Starter', 'set': 'ST1',
                               'main_deck': [ids[0]], 'egg_deck': []}]}),
            'exams/a/pool.json': raw({'decks': [{'name': 'test', 'cards': [ids[0],ids[1],ids[1]], 'eggs': [ids[2]]},
                                               {'name': 'ST1', 'cards': [ids[0]], 'eggs': []}]}),
            'exams/a/test.yaml': yaml.safe_dump(s).encode()}
    manifest = dict(schema_version=1, files={},
                    inventories={'official': 'official.json', 'secondary': 'secondary.json',
                                 'canonical': 'canonical.json', 'runtime': 'runtime.json', 'qa': 'qa.json'},
                    scope_witness='witness.json', starter='starter.json',
                    scenarios=['exams/a/test.yaml'], pools=['exams/a/pool.json'],
                    explicit_books={}, target_decks=[], implementation_files=[])
    return docs, manifest


def run(docs, manifest, **kwargs):
    from tools.card_data.focused_closure import resolve_closure
    m = deepcopy(manifest); m['files'] = {p: sha(b) for p,b in docs.items()}
    b = raw(m)
    return resolve_closure(original_inputs=docs, manifest_bytes=b, expected_manifest_sha256=sha(b), **kwargs)


def codes(result):
    return {b['code'] for b in result['blockers']}


def test_original_universe_order_and_no_admission():
    d,m=fixture(); r=run(d,m)
    assert r['status']=='OFFLINE_CLOSURE_ONLY_NOT_ADMISSION'
    assert len(r['inventory'])==4 and r['inventory']['BT1-004']['runtime']
    p0=r['scenarios']['exams/a/test.yaml']['seats']['p0']
    assert p0['main']==['BT1-001','BT1-002','BT1-002']
    assert p0['eggs']==['BT1-003']
    assert p0['ordered']==['BT1-002','BT1-003','BT1-001','BT1-002']
    assert r['E']==['BT1-001','BT1-002','BT1-003']
    assert 'IMPLEMENTATION_REFERENCE_ANALYSIS_REQUIRED' in codes(r)


def test_reverse_only_qa_fixed_point_and_full_question():
    d,m=fixture();q=json.loads(d['qa.json'])
    q['cards']['BT1-001']=['Q1'];q['cards']['BT1-004']=['Q1']
    q['qa']['Q1']={'q_id':'Q1','question':'whole question','answer':'whole answer','card_ids':['BT1-003']}
    d['qa.json']=raw(q);r=run(d,m)
    assert r['Q']==['BT1-001','BT1-002','BT1-003','BT1-004']
    assert r['qa']['Q1']['members']==['BT1-001','BT1-003','BT1-004']
    assert r['qa']['Q1']['row']==q['qa']['Q1']


def test_same_omission_from_supplied_views_refused():
    d,m=fixture()
    r=run(d,m, supplied_inventory=['BT1-001','BT1-002','BT1-003'])
    assert 'SUPPLIED_INVENTORY_MISMATCH' in codes(r)
    assert 'BT1-004' in r['inventory']


@pytest.mark.parametrize('change,code',[
    ('stack','INVALID_STACK'),('duplicate','DUPLICATE_DECK_ALIAS'),
    ('unknown_action','SCENARIO_SCHEMA'),('empty_steps','SCENARIO_SCHEMA'),
    ('unknown_seat','SCENARIO_SCHEMA'),('empty_deck','EMPTY_DECK'),
    ('unknown_rest','UNKNOWN_DECK'),('foreign_qa','QA_REFERENCE_MISSING'),
])
def test_blocking_conditions(change,code):
    d,m=fixture();s=yaml.safe_load(d['exams/a/test.yaml']);p=json.loads(d['exams/a/pool.json'])
    if change=='stack':s['decks']['p0']['stack']=['BT1-002']*3
    elif change=='duplicate':p['decks'].append(deepcopy(p['decks'][0]))
    elif change=='unknown_action':s['steps'][0]['do']={'create':{'card':'BT1-004'}}
    elif change=='empty_steps':s['steps']=[]
    elif change=='unknown_seat':s['decks']['p2']=s['decks']['p0']
    elif change=='empty_deck':p['decks'][0]['cards']=[];p['decks'][0]['eggs']=[]
    elif change=='unknown_rest':s['decks']['p0']['rest']='missing'
    elif change=='foreign_qa':
        q=json.loads(d['qa.json']);q['cards']['BT1-001']=['QNO'];d['qa.json']=raw(q)
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode();d['exams/a/pool.json']=raw(p)
    assert code in codes(run(d,m))


def test_witness_never_shrinks_for_narrative_only_member():
    d,m=fixture();d['witness.json']=raw({'E':['BT1-004'],'Q':['BT1-004']})
    r=run(d,m);assert 'BT1-004' in r['E']
    assert 'AUDITED_REFERENCE_UNCLASSIFIED' in codes(r)


def test_comments_do_not_authorize_new_cards():
    d,m=fixture();d['exams/a/test.yaml']+=b'\n# BT1-004\n'
    assert 'BT1-004' not in run(d,m)['E']


def test_changed_pinned_input_and_unknown_request_refused():
    from tools.card_data.focused_closure import resolve_closure
    d,m=fixture();m['files']={p:sha(b)for p,b in d.items()};b=raw(m)
    d['runtime.json']+=b' '
    with pytest.raises(ValueError):resolve_closure(original_inputs=d,manifest_bytes=b,expected_manifest_sha256=sha(b))
    with pytest.raises(TypeError):run(*fixture(),validated=True)


def test_actual_q6813_negative_retains_both_seats_and_original_bytes():
    d,m=fixture();s=(F/'EX12-046-qa-Q6813.yaml').read_bytes();p=(F/'toho_pool.json').read_bytes()
    d['exams/a/test.yaml']=s;d['exams/a/pool.json']=p;d['starter.json']=(F/'starter_decks.json').read_bytes()
    r=run(d,m)
    failures=[x for x in r['blockers'] if x['code']=='INVALID_STACK']
    assert len(failures)==1 and failures[0]['seat']=='p1' and failures[0]['missing']=={'BT15-037':1}
    assert len(r['scenarios']['exams/a/test.yaml']['seats'])==2
    assert r['input_sha256']['exams/a/test.yaml']==sha(s)


def test_starter_fallback_without_pool_and_overlay_priority():
    d,m=fixture();s=yaml.safe_load(d['exams/a/test.yaml'])
    for seat in s['decks'].values():seat.update(rest='sTaRtEr',stack=[])
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode()
    r=run(d,m);assert r['scenarios']['exams/a/test.yaml']['selected_book'] is None
    assert r['scenarios']['exams/a/test.yaml']['seats']['p1']['ordered']==['BT1-001']
    p=json.loads(d['exams/a/pool.json']);p['decks'].append({'name':'starter','cards':['BT1-003']})
    d['exams/a/pool.json']=raw(p)
    assert run(d,m)['scenarios']['exams/a/test.yaml']['seats']['p0']['ordered']==['BT1-003']


def test_no_starter_no_matching_pool_refuses_no_substitution():
    d,m=fixture();m['starter']=None
    s=yaml.safe_load(d['exams/a/test.yaml']);s['decks']['p1']['rest']='missing'
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode()
    assert 'UNKNOWN_DECK' in codes(run(d,m))


def test_ambiguous_books_preserve_actual_choice_but_block():
    d,m=fixture();d['exams/a/second.json']=d['exams/a/pool.json'];m['pools'].append('exams/a/second.json')
    r=run(d,m);assert 'AMBIGUOUS_BOOK' in codes(r)
    assert r['scenarios']['exams/a/test.yaml']['selected_book']=='exams/a/pool.json'
    m['explicit_books']={'exams/a/test.yaml':'exams/a/second.json'}
    assert 'AMBIGUOUS_BOOK' not in codes(run(d,m))


def test_remote_ambiguity_and_local_precedence():
    d,m=fixture();m['pools']=['exams/b/pool.json','exams/c/pool.json']
    d['exams/b/pool.json']=d['exams/c/pool.json']=d['exams/a/pool.json']
    assert 'AMBIGUOUS_BOOK' in codes(run(d,m))
    m['pools'].append('exams/a/pool.json')
    assert 'AMBIGUOUS_BOOK' not in codes(run(d,m))


def test_casefold_duplicate_alias_and_duplicate_json_key():
    d,m=fixture();p=json.loads(d['exams/a/pool.json']);p['decks'].append({'name':'TEST','cards':['BT1-004']})
    d['exams/a/pool.json']=raw(p);assert 'DUPLICATE_DECK_ALIAS' in codes(run(d,m))
    d,m=fixture();d['runtime.json']=b'{"BT1-001":{"card_id":"BT1-001"},"BT1-001":{"card_id":"BT1-001"}}'
    with pytest.raises(ValueError,match='duplicate'):run(d,m)


def test_declared_qa_without_reverse_link_expands_and_failed_card_blocks():
    d,m=fixture();s=yaml.safe_load(d['exams/a/test.yaml'])
    s['interaction']={'id':'qa:Q2','source':'qa','kind':'positive'}
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode();q=json.loads(d['qa.json'])
    q['qa']['Q2']={'q_id':'Q2','question':'Q','answer':'A','card_ids':['BT1-004']}
    q['failed']=['BT1-004'];d['qa.json']=raw(q)
    r=run(d,m);assert 'BT1-004' in r['Q'] and 'QA_UNAVAILABLE' in codes(r)


@pytest.mark.parametrize('field',['schema_version','complete','expected_count'])
def test_scope_types_do_not_coerce_boolean_or_integer(field):
    d,m=fixture();o=json.loads(d['official.json']);o[field]=1 if field=='complete' else True
    d['official.json']=raw(o)
    with pytest.raises(ValueError):run(d,m)


def test_whole_input_membership_pin_and_unknown_manifest_schema():
    from tools.card_data.focused_closure import resolve_closure
    d,m=fixture();m['files']={p:sha(b)for p,b in d.items()};b=raw(m);d.pop('official.json')
    with pytest.raises(ValueError,match='inventory'):resolve_closure(original_inputs=d,manifest_bytes=b,expected_manifest_sha256=sha(b))
    d,m=fixture();m['E']=['BT1-001']
    with pytest.raises(ValueError,match='unknown'):run(d,m)


def test_target_decks_and_all_identity_action_forms_retained():
    d,m=fixture();m['target_decks']=[{'pool':'exams/a/pool.json','name':'test'}]
    s=yaml.safe_load(d['exams/a/test.yaml'])
    s['steps']=[{'actor':1,'do':{'digivolve':{'from':'field.0','using':'BT1-004'}}},
                {'actor':0,'do':{'select':{'materials':['BT1-002','BT1-003']}}},
                {'actor':1,'do':{'select':{'cards':['BT1-004']}}}]
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode()
    assert run(d,m)['E']==['BT1-001','BT1-002','BT1-003','BT1-004']


def test_actual_fixture_hash_provenance():
    evidence=json.loads((F/'provenance.json').read_bytes())
    for path,row in evidence.items():assert sha((F/path).read_bytes())==row['sha256']


def test_yaml12_yes_selection_key_is_not_boolean():
    d,m=fixture();s=yaml.safe_load(d['exams/a/test.yaml']);s['steps']=[{'actor':0,'do':{'select':{'yes':True}}}]
    # serde_yml treats yes as a string key, whereas PyYAML's default YAML1.1
    # resolver turns it into True. Preserve the actual Rust wire grammar.
    d['exams/a/test.yaml']=yaml.safe_dump(s).replace("'yes':",'yes:').encode()
    assert 'SCENARIO_SCHEMA' not in codes(run(d,m))


def test_known_nonsemantic_native_metadata_is_retained_not_interpreted():
    d,m=fixture();st=json.loads(d['starter.json']);st['starter_decks'][0]['game_mode']='starter'
    d['starter.json']=raw(st);p=json.loads(d['exams/a/pool.json']);p['_comment']='BT1-004'
    d['exams/a/pool.json']=raw(p);r=run(d,m)
    assert not({'POOL_SCHEMA','STARTER_SCHEMA'}&codes(r)) and 'BT1-004' not in r['E']


def test_native_campaign_target_counts_are_not_a_supplied_E_list():
    d,m=fixture();d['targets.json']=raw({'scope':'fixture','cards':['BT1-004'],
        'decks':[{'archetype':'target','counts':{'BT1-004':4},'source':'fixture'}]})
    m['target_decks']=[{'pool':'targets.json','name':'target','format':'campaign-counts'}]
    r=run(d,m);assert 'BT1-004' in r['E']
    assert r['targets'][0]['counts']=={'BT1-004':4}
    assert r['original_runtime_indices']['BT1-004']=={'index':4}


@pytest.mark.parametrize('count',[True,0,-1,1.5])
def test_campaign_count_malformed_blocks(count):
    d,m=fixture();d['targets.json']=raw({'scope':'fixture','cards':['BT1-004'],
        'decks':[{'archetype':'target','counts':{'BT1-004':count}}]})
    m['target_decks']=[{'pool':'targets.json','name':'target','format':'campaign-counts'}]
    assert 'TARGET_SCHEMA' in codes(run(d,m))


def test_explicit_exam_root_prevents_cross_tree_book_selection():
    d,m=fixture();d['other/pool.json']=d['exams/a/pool.json'];m['pools'].append('other/pool.json')
    # Search from the correct Rust exams_root, not all trees in a multi-lane
    # bundle. Move the own book remotely so directory-priority cannot mask it.
    d['exams/b/pool.json']=d.pop('exams/a/pool.json')
    m['pools']=['other/pool.json','exams/b/pool.json'];m['exam_roots']=['exams','other']
    r=run(d,m);assert 'AMBIGUOUS_BOOK' not in codes(r)
    assert r['scenarios']['exams/a/test.yaml']['selected_book']=='exams/b/pool.json'


@pytest.mark.parametrize('field,value',[('card_count',True),('card_count',3),('qa_count',1)])
def test_qa_declared_totals_must_match_original_rows(field,value):
    d,m=fixture();q=json.loads(d['qa.json']);q[field]=value;d['qa.json']=raw(q)
    with pytest.raises(ValueError):run(d,m)


def test_failed_qa_only_member_and_historical_scope_blocker_retained():
    d,m=fixture();q=json.loads(d['qa.json']);q['failed']=['BT1-099'];d['qa.json']=raw(q)
    d['witness.json']=raw({'E':['BT1-001'],'Q':['BT1-001'],
                          'unresolved':[{'kind':'historical-question','id':'original'}]})
    r=run(d,m);assert r['inventory']['BT1-099']['qa_failed']
    assert r['retained_audited_scope']['unresolved'][0]['id']=='original'


@pytest.mark.parametrize('path',['C:/foreign.json','./foreign.json','a//foreign.json','a/./foreign.json','a/../foreign.json','/foreign.json','a\\foreign.json'])
def test_r1_noncanonical_paths_refuse(path):
    d,m=fixture();d[path]=b'{}'
    with pytest.raises(ValueError,match='path'):run(d,m)

@pytest.mark.parametrize('token',['010','012','1:20','1_0'])
def test_r1_native_string_numeric_forms_block(token):
    d,m=fixture();d['exams/a/test.yaml']=d['exams/a/test.yaml'].replace(b'seed: 1',('seed: '+token).encode())
    assert 'SCENARIO_SCHEMA' in codes(run(d,m))

@pytest.mark.parametrize('token,value',[('0b10',2),('0o10',8),('10',10)])
def test_r1_native_integer_forms(token,value):
    from tools.card_data.focused_closure import _yaml
    assert _yaml(('seed: '+token).encode())['seed']==value


def test_r1_qa_conflict_payload_retained():
    d,m=fixture();q=json.loads(d['qa.json'])
    q['cards']['BT1-001']=['Q1'];q['qa']['Q1']={'q_id':'Q1','question':'q','answer':'a','card_ids':['BT1-001']}
    q['conflicts']=[{'q_id':'Q1','card_id':'BT1-001','detail':'preserve original dispute'}]
    d['qa.json']=raw(q);r=run(d,m)
    assert 'QA_CONFLICT' in codes(r)
    assert r['qa_conflicts']==q['conflicts'] and 'Q1' in r['qa']


@pytest.mark.parametrize('conflicts',[{},[{}],[{'q_id':True,'card_id':'BT1-001'}],[{'q_id':'Q1','card_id':False}]])
def test_r1_malformed_qa_conflicts_refuse(conflicts):
    d,m=fixture();q=json.loads(d['qa.json']);q['conflicts']=conflicts;d['qa.json']=raw(q)
    with pytest.raises(ValueError):run(d,m)


def test_r1_unrelated_conflict_retained_without_false_scope_admission():
    d,m=fixture();q=json.loads(d['qa.json']);q['conflicts']=[{'q_id':'Q999','card_id':'BT1-099'}]
    d['qa.json']=raw(q);r=run(d,m)
    assert r['qa_conflicts']==q['conflicts'] and r['inventory']['BT1-099']['qa']
    assert 'BT1-099' not in r['Q'] and 'QA_CONFLICT' not in codes(r)


@pytest.mark.parametrize('root',['C:/exams','./exams','exams//a','exams/./a'])
def test_r1_exam_roots_same_canonical_path_contract(root):
    d,m=fixture();m['exam_roots']=[root]
    with pytest.raises(ValueError,match='path'):run(d,m)


@pytest.mark.parametrize('field,value,valid',[
 ('seed',2**64-1,True),('seed',2**64,False),('seed',-1,False),
 ('value',2**31-1,True),('value',-2**31,True),('value',2**31,False),('value',-2**31-1,False),
 ('ordinal',2**31-1,True),('ordinal',2**31,False),('ordinal',-1,False),
 ('count',65535,True),('count',65536,False),('count',-1,False),
 ('at',2**32,False),('at',-1,False),('at',1,True),
 ('ruling_at',2**32,False),('ruling_at',1,True)])
def test_r2_native_numeric_ranges(field,value,valid):
    d,m=fixture();s=yaml.safe_load(d['exams/a/test.yaml'])
    if field=='seed':s['seed']=value
    elif field in ('value','ordinal'):
        a={'value':value} if field=='value' else {'cards':['BT1-001'],'ordinal':value}
        s['steps']=[{'actor':0,'do':{'select':a}}]
    elif field=='count':s['steps'][0]['expect']={'count':value}
    elif field=='at':s['assert']=[{'at':value,'that':{'memory':0}}]
    else:s['expect_ruling']={'q_id':'Q1','assert':[{'at':value,'that':{'memory':0}}]}
    d['exams/a/test.yaml']=yaml.safe_dump(s).encode();r=run(d,m)
    assert ('SCENARIO_SCHEMA' not in codes(r))==valid
