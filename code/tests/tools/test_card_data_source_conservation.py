"""Real source records plus adversarial conservation tests; never approve data."""
import copy,gzip,json
from pathlib import Path
import pytest
from tools.card_data.source_conservation import rule_grant_check,condition_check,build_conservation_report
from tools.card_data.serialization import canonical_json_bytes, content_digest
F=Path(__file__).parents[1]/'fixtures/card_data/source_conservation'
def fixture():
    names=['snapshot','canonical','runtime','clauses','memberships','selections','corrections','source_manifest']
    return {n:(F/(n+'.json')).read_bytes() for n in names},{json.loads((F/'provenance.json').read_bytes())['raw_url']:gzip.decompress((F/'official-ex12.html.gz').read_bytes())}
def values(inputs):return {k:json.loads(v) for k,v in inputs.items()}
def pack(values):return {k:canonical_json_bytes(v) for k,v in values.items()}
def conditions():
    i,_=fixture();v=values(i);o=next(x for x in v['snapshot']['observations'] if x['extracted']['card_id']=='EX12-076');s=[x for x in o['extracted']['text_sections'] if 'Condition' in x['label']];cl=[x for x in v['clauses']['clauses'] if x['card_id']=='EX12-076' and 'Condition' in x['label']];return s,'\n'.join('['+x['label']+'] '+x['text'] for x in s),cl

def test_exact_real_condition_shapes_conserve_bodies():
    s,x,c=conditions();assert condition_check(s,x,c)==[]
@pytest.mark.parametrize('mutation',['missing_projection','missing_clause','only_keyword','number','sign','names','duplicate','unrelated_label'])
def test_complete_restrictions_and_multiplicity_are_required(mutation):
    s,x,c=conditions()
    if mutation=='missing_projection':x=x.split('\n')[0]
    elif mutation=='missing_clause':c=c[:1]
    elif mutation=='only_keyword':c[-1]['text']='Assembly'
    elif mutation=='number':c[-1]['text']=c[-1]['text'].replace('8 ','7 ')
    elif mutation=='sign':c[-1]['text']=c[-1]['text'].replace('-9','+9')
    elif mutation=='names':c[-1]['text']=c[-1]['text'].replace(' w/different names','')
    elif mutation=='duplicate':s.append(copy.deepcopy(s[-1]));x+='\n['+s[-1]['label']+'] '+s[-1]['text']
    elif mutation=='unrelated_label':c[-1]['label']='Effect'
    assert condition_check(s,x,c)
@pytest.mark.parametrize('text',['(Rule) Trait: Has [Hybrid] Form.','[Rule] Trait: Has [Hybrid] Form.','Rule: Trait: Has [Hybrid] Form.'])
def test_rule_requires_observed_form_grant(text):
    assert rule_grant_check([dict(label='Effect',text=text)],dict(forms=[],traits=[],attributes=[]))
    assert rule_grant_check([dict(label='Effect',text=text)],dict(forms=['Hybrid'],traits=[],attributes=[]))==[]
@pytest.mark.parametrize('text',['(Rule) Trait: Has [Hybrid] Form except on Tuesdays.','\uff1cRule\uff1e Trait: Has [Hybrid] Form.','(Rule) Trait: Has [Hybrid] Form and [Rookie] Form.'])
def test_unsupported_rule_grammar_blocks(text):
    assert rule_grant_check([dict(label='Effect',text=text)],dict(forms=['Hybrid'],traits=[],attributes=[]))
def test_real_three_card_baseline_blocked_and_exhaustive():
    i,raw=fixture();r=build_conservation_report(inputs=i,raw_pages=raw)
    assert r['status']=='BLOCKED';assert {x['card_id'] for x in r['rows']}=={'EX12-046','EX12-063','EX12-076'}
    codes={x['code'] for x in r['failures']};assert 'CONDITION_NORMALIZATION_OR_RECONSTRUCTION_UNSUPPORTED' in codes and 'CANONICAL_FORM_LOSS' in codes and 'RUNTIME_FORM_LOSS' in codes and 'CLAUSE_SOURCE_PROVENANCE_MISSING' in codes
@pytest.mark.parametrize('mutation',['raw','observation_identity','missing_printing','scope','focused','duplicate_clause','canonical_provenance'])
def test_integrity_refusals_are_explicit(mutation):
    i,raw=fixture();v=values(i)
    if mutation=='raw':raw[next(iter(raw))]+=b'corrupt'
    elif mutation=='observation_identity':v['snapshot']['observations'][0]['observation_id']='0'*64
    elif mutation=='missing_printing':v['snapshot']['observations'].pop()
    elif mutation=='scope':v['memberships']['universe'].pop()
    elif mutation=='focused':v['memberships']['focused'].append('BT1-001')
    elif mutation=='duplicate_clause':v['clauses']['clauses'].append(v['clauses']['clauses'][0])
    elif mutation=='canonical_provenance':v['canonical']['cards']['EX12-076']['provenance'].pop('form_eng')
    r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert r['status']=='BLOCKED' and any(x['leg']=='integrity' for x in r['failures'])
def test_deterministic_and_no_mutation():
    i,raw=fixture();before=copy.deepcopy((i,raw));a=build_conservation_report(inputs=i,raw_pages=raw);b=build_conservation_report(inputs=i,raw_pages=raw);assert a==b and (i,raw)==before;assert a['inputs']['runtime']['sha256'];assert a['checker_sha256'];assert 'adjudications' not in a


def test_inline_unparsed_rule_marker_is_not_absence():
    assert rule_grant_check([dict(label='Effect',text='Other sentence. (Rule) Trait: Has [Hybrid] Form.')],dict(forms=[],traits=[],attributes=[]))

def test_actual_clause_binding_change_is_reported():
    i,raw=fixture();v=values(i);v['clauses']['clauses'][0]['text']+=' changed';r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert any(x['code']=='CLAUSE_SEMANTIC_BINDING_MISMATCH' for x in r['failures'])

def test_repeated_complete_condition_positive_control():
    s,x,c=conditions();s.append(copy.deepcopy(s[-1]));c.append(copy.deepcopy(c[-1]));c[-1]['id']+='-repeat';x+='\n['+s[-1]['label']+'] '+s[-1]['text'];assert condition_check(s,x,c)==[]

def test_plain_no_condition_no_grant_control():
    assert rule_grant_check([dict(label='Effect',text='Draw 1.')],dict(forms=[],traits=[],attributes=[]))==[]
    assert condition_check([], '', [])==[]

@pytest.mark.parametrize('mutation',['canonical','runtime','wrong_category'])
def test_real_hybrid_projection_loss_is_precise(mutation):
    i,raw=fixture();v=values(i)
    if mutation=='canonical':v['canonical']['cards']['EX12-076']['fields']['form_eng']=['Mega']
    elif mutation=='runtime':v['runtime']['EX12-076']['form_eng']=['Mega']
    else:v['runtime']['EX12-076']['form_eng']=['Mega'];v['runtime']['EX12-076']['type_eng'].append('Hybrid')
    r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert any(x['code']==('CANONICAL_FORM_LOSS' if mutation=='canonical' else 'RUNTIME_FORM_LOSS') and x['card_id']=='EX12-076' for x in r['failures'])

@pytest.mark.parametrize('mutation',['stale_selection','missing_provenance','malformed_selection','different_raw_same_revision'])
def test_authority_inputs_cannot_be_replaced_by_reported_revision(mutation):
    i,raw=fixture();v=values(i)
    if mutation=='stale_selection':
        from tools.card_data.serialization import content_digest
        obs=[o for o in v['snapshot']['observations'] if o['extracted']['card_id']=='EX12-076'];candidates={o['observation_id']:'\n'.join('['+s['label']+'] '+s['text'] for s in o['extracted']['text_sections'] if 'Condition' in s['label']) for o in obs};oid=next(iter(candidates))
        v['selections']['selections']=[dict(schema_version=1,selection_id='synthetic-stale-selection',selection_kind='official_condition_projection',card_id='EX12-076',field_path='xros_req',source_manifest_revision='0'*64,candidate_values=candidates,candidate_set_digest=content_digest(candidates),selected_observation_id=oid,selected_value=candidates[oid],selected_value_digest=content_digest(candidates[oid]),reviewer='synthetic',reason='synthetic stale-boundary test',reviewed_at='2026-10-12T00:00:00Z')]
    elif mutation=='missing_provenance':v['canonical']['cards']['EX12-076']['provenance']['form_eng']['observation_ids']=[]
    elif mutation=='malformed_selection':v['selections']['selections']=[{}]
    else:v['source_manifest']['membership']['gameplay_card_count']+=1
    r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    expected={'stale_selection':'CONDITION_SELECTION_STALE','missing_provenance':'CANONICAL_FORM_PROVENANCE_UNRESOLVED','malformed_selection':'SELECTION_SCHEMA_INVALID','different_raw_same_revision':'SOURCE_AGGREGATE_BINDING_INVALID'}[mutation]
    assert any(x['code']==expected for x in r['failures'])


def test_source_url_metadata_cannot_redirect_official_authority():
    i,raw=fixture();v=values(i);old=next(iter(raw));url='https://example.invalid/cards/'
    for o in v['snapshot']['observations']:o['source_url']=url
    raw[url]=raw.pop(old);r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert any(f['code']=='SOURCE_ORIGIN_UNREVIEWED' for f in r['failures'])

def test_extra_runtime_form_is_not_conservation():
    i,raw=fixture();v=values(i);v['runtime']['EX12-046']['form_eng'].append('Hybrid')
    r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert any(f['code']=='RUNTIME_FORM_UNSOURCED' and f['card_id']=='EX12-046' for f in r['failures'])


# Exact independent-review regressions, preserved as repository tests.
@pytest.mark.parametrize('text',['\uff08Rule\uff09 Trait: Has [Hybrid] Form.','\uff1cRule\uff1e Trait: Has [Hybrid] Form.','\uff3bRule\uff3d Trait: Has [Hybrid] Form.','Other. Rule: Trait: Has [Hybrid] Form.'])
def test_unicode_or_inline_unparsed_rule_refuses(text):
 assert rule_grant_check([dict(label='Effect',text=text)],dict(forms=[],traits=[],attributes=[]))

def test_cross_card_form_provenance_rejected():
 i,raw=fixture();v=values(i);foreign=next(o['observation_id']for o in v['snapshot']['observations']if o['extracted']['card_id']=='EX12-063')
 v['canonical']['cards']['EX12-046']['provenance']['form_eng'].update(observation_ids=[foreign],selected_observation_id=foreign)
 r=build_conservation_report(inputs=pack(v),raw_pages=raw)
 assert any(f['code']=='CANONICAL_FORM_PROVENANCE_UNRESOLVED' and f['card_id']=='EX12-046'for f in r['failures'])

def test_new_correction_collection_not_silently_ignored():
 i,raw=fixture();v=values(i);v['corrections']={'schema_version':2,'corrections':[],'compatibility_corrections':[{'card_id':'EX12-046','field_path':'xros_req','value':'different'}]}
 r=build_conservation_report(inputs=pack(v),raw_pages=raw)
 assert any('CORRECTION' in f['code']for f in r['failures'])

def test_global_invalidity_cannot_label_clean_row_conserved():
 # Synthetic ordinary-card fixture, no operational source approvals; integrity
 # failures deliberately remain. Row status must not hide those failures.
 i,raw=fixture();v=values(i);cid='EX12-046';members=[]
 for o in v['snapshot']['observations']:
  if o['extracted']['card_id']!=cid:continue
  o['extracted']['text_sections']=[];o['extracted']['rule_grants']={'forms':[],'traits':[],'attributes':[]}
  o['observation_id']=content_digest({k:o[k]for k in ['raw_payload_digest','parser_version','source_schema_version','extracted']},exclude_operational=True);members.append(o)
 v['snapshot']['observations']=members
 ids=[o['observation_id']for o in members];card=v['canonical']['cards'][cid]
 card['printings']=[{'observation_ids':ids}];card['provenance']['form_eng'].update(observation_ids=ids,selected_observation_id=ids[0]);card['fields']['form_eng']=['Ultimate']
 v['runtime'][cid]['form_eng']=['Ultimate'];v['runtime'][cid]['xros_req']=''
 v['canonical']['cards']={cid:card};v['runtime']={cid:v['runtime'][cid]};v['clauses']['cards']=[cid];v['clauses']['clauses']=[];v['memberships']={'universe':[cid],'focused':[cid],'scenario_required':[cid]}
 r=build_conservation_report(inputs=pack(v),raw_pages=raw)
 assert r['status']=='BLOCKED'
 assert r['rows'][0]['status']=='BLOCKED',r['rows'][0]



@pytest.mark.parametrize('wrong_printing',[False,True])
def test_own_card_provenance_requires_exact_printing_binding(wrong_printing):
    i,raw=fixture();v=values(i);cid='EX12-046';obs=[o for o in v['snapshot']['observations'] if o['extracted']['card_id']==cid];ids=[o['observation_id'] for o in obs];card=v['canonical']['cards'][cid]
    card['provenance']['form_eng'].update(observation_ids=ids,selected_observation_id=ids[0])
    card['printings']=[dict(printing_id='unrelated' if wrong_printing else o['extracted']['printing']['printing_id'],observation_ids=[o['observation_id']]) for o in obs]
    r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    failures=[f for f in r['failures'] if f['code']=='CANONICAL_FORM_PROVENANCE_UNRESOLVED' and f['card_id']==cid]
    assert bool(failures)==wrong_printing

@pytest.mark.parametrize('payload',[{'schema_version':2,'corrections':[],'compatibility_corrections':[]},{'schema_version':1,'corrections':[],'unknown':[]},{'schema_version':True,'corrections':[]}])
def test_unknown_correction_schema_blocks_even_without_rows(payload):
    i,raw=fixture();v=values(i);v['corrections']=payload;r=build_conservation_report(inputs=pack(v),raw_pages=raw)
    assert any(f['code']=='CORRECTION_SCHEMA_UNSUPPORTED' for f in r['failures'])
    assert all(row['status']=='BLOCKED' and row['global_failure_indices'] for row in r['rows'])
