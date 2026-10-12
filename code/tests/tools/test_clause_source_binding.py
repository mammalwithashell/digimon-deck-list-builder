"""Contract tests only; fixture projections are not production approvals."""
import copy
import gzip
import json
from pathlib import Path
import pytest
from tools.card_data.normalization import _official_fields, _normalize_candidate_fields
from tools.card_data.serialization import canonical_json_bytes, content_digest
from tools.clause_coverage.source_binding import SourceBindingError, derive_binding, validate_binding

F=Path(__file__).parents[1]/'fixtures/card_data/source_binding'

def fixture():
    data=json.loads((F/'observations.json').read_text(encoding='utf-8'))
    obs=data['observations']; ids=data['universe']
    refs=[{'observation_id':o['observation_id'],'gameplay_card_id':o['extracted']['card_id'],'printing_id':o['extracted']['printing']['printing_id']} for o in obs]
    aggregate={'sources':{'official':{'observation_ids':sorted(o['observation_id'] for o in obs)}}}
    revision=content_digest(aggregate,exclude_operational=True);aggregate['manifest_revision']=revision
    cards={};runtime={}
    fields=('effect_description_eng','inherited_effect_description_eng','security_effect_description_eng')
    for cid in ids:
        own=[o for o in obs if o['extracted']['card_id']==cid];values=_normalize_candidate_fields(_official_fields(own[0]['extracted']))
        cards[cid]={'card_id':cid,'fields':values,'printings':[{'printing_id':o['extracted']['printing']['printing_id'],'observation_ids':[o['observation_id']]}for o in own],'provenance':{f:{'authority':'official','derived':False,'adjudicated':False,'selected_observation_id':own[0]['observation_id'],'observation_ids':[o['observation_id']for o in own]}for f in fields}}
        runtime[cid]={'card_id':cid,**values}
    inputs={'snapshot':{'manifest':{'schema_version':1,'source':'official','scope':'full-pool:official-catalog','parser_version':'official-html-v5','source_schema_version':'world-card-list-html-v1','source_locations':sorted({o['source_url'] for o in obs}),'complete':True,'failed_ids':[],'expected_count':len(ids),'observation_ids':sorted(o['observation_id'] for o in obs),'observation_refs':refs},'observations':obs},'canonical':{'schema_version':1,'source_manifest_digest':revision,'cards':cards},'runtime':runtime,'source_manifest':aggregate,'corrections':{'schema_version':1,'corrections':[]},'selections':{'schema_version':1,'selections':[]},'adjudications':{'schema_version':1,'adjudications':[]}}
    return inputs,{u:gzip.decompress((F/n).read_bytes())for u,n in data['raw_files'].items()},ids

def pack(v):return {k:canonical_json_bytes(x)for k,x in v.items()}

def test_real_source_positive_full_closure_and_no_mutation():
    v,raw,ids=fixture();before=copy.deepcopy((v,raw));b=derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)
    validate_binding(b,inputs=pack(v),raw_pages=raw,expected_universe=ids)
    assert (v,raw)==before
    assert b['cards']['BT1-045']['clauses']==[]
    assert set(b['cards']['BT1-045']['zones'])=={'effect','inherited','security'}
    assert b['cards']['BT1-001']['zones']['inherited']
    assert 'verdict' not in b and 'accuracy' not in b

@pytest.mark.parametrize('mutation',['missing_card','printing','foreign_provenance','field_value','runtime','raw','observation','unknown_label','dual','correction','schema2','selection','adjudication','missing_input','aggregate','manifest_refs'])
def test_input_boundary_refusals(mutation):
    v,raw,ids=fixture();c=v['canonical']['cards']['BT1-010'];p=c['provenance']['effect_description_eng']
    if mutation=='missing_card':v['runtime'].pop('BT1-045')
    elif mutation=='printing':c['printings'][0]['printing_id']='BT9-999'
    elif mutation=='foreign_provenance':p['selected_observation_id']=v['canonical']['cards']['BT1-001']['provenance']['inherited_effect_description_eng']['selected_observation_id']
    elif mutation=='field_value':c['fields']['effect_description_eng']='invented'
    elif mutation=='runtime':v['runtime']['BT1-010']['effect_description_eng']+=' changed'
    elif mutation=='raw':raw[next(iter(raw))]+=b'changed'
    elif mutation=='observation':v['snapshot']['observations'][0]['observation_id']='0'*64
    elif mutation=='unknown_label':v['snapshot']['observations'][0]['extracted']['text_sections'].append({'label':'Link Effect','text':'Draw 1.'})
    elif mutation=='dual':v['runtime']['BT1-010']['dual']={'option':{'effect_text':'Draw 1.'}}
    elif mutation=='correction':v['corrections']['corrections']=[{'card_id':'BT1-010'}]
    elif mutation=='schema2':v['corrections']={'schema_version':2,'corrections':[],'compatibility_corrections':[]}
    elif mutation=='selection':v['selections']['selections']=[{}]
    elif mutation=='adjudication':p['adjudicated']=True
    elif mutation=='missing_input':v.pop('canonical')
    elif mutation=='aggregate':v['source_manifest']['sources']['official']['observation_ids'].pop()
    elif mutation=='manifest_refs':v['snapshot']['manifest']['observation_refs'].pop()
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('mutation',['missing_unit','changed_unit','clause','zone','sibling','unknown_version','zero_card','self_hash'])
def test_submitted_metadata_not_its_own_authority(mutation):
    v,raw,ids=fixture();b=derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids);c=b['cards']['BT1-010']
    if mutation=='missing_unit':c['units'].pop()
    elif mutation=='changed_unit':c['units'][0]['text']='invented'
    elif mutation=='clause':c['clauses'][0]['text']='invented'
    elif mutation=='zone':c['zones']['effect']=[]
    elif mutation=='sibling':c['clauses'].append(copy.deepcopy(c['clauses'][0]))
    elif mutation=='unknown_version':b['version']=999
    elif mutation=='zero_card':b['cards'].pop('BT1-045')
    elif mutation=='self_hash':b['inputs']['runtime']['sha256']='0'*64
    with pytest.raises(SourceBindingError):validate_binding(b,inputs=pack(v),raw_pages=raw,expected_universe=ids)

def test_independent_universe_prevents_pair_shrink():
    v,raw,ids=fixture()
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids[:-1])

def test_input_byte_change_invalidates_prior_binding():
    v,raw,ids=fixture();inputs=pack(v);b=derive_binding(inputs=inputs,raw_pages=raw,expected_universe=ids);inputs['runtime']+=b' '
    with pytest.raises(SourceBindingError):validate_binding(b,inputs=inputs,raw_pages=raw,expected_universe=ids)

def test_boolean_version_is_not_version_one():
    v,raw,ids=fixture();b=derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids);b['version']=True
    with pytest.raises(SourceBindingError):validate_binding(b,inputs=pack(v),raw_pages=raw,expected_universe=ids)

def test_empty_timing_sibling_preserved_at_splitter_boundary(monkeypatch):
    # Synthetic splitter boundary, no new source observation/approval is minted.
    from tools.clause_coverage import source_binding as sb
    original=sb.text_split.split_clauses
    def spans(text):
        rows=original(text)
        if rows:return [sb.text_split.ClauseSpan(kind='timing',timings=['On Play'],text=''),*rows]
        return rows
    monkeypatch.setattr(sb.text_split,'split_clauses',spans)
    v,raw,ids=fixture();b=derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)
    rows=b['cards']['BT1-010']['clauses'];assert rows[0]['text']=='' and rows[0]['timings']==['On Play']
    assert len(rows[1]['semantic_payload'][-1])==len(rows)
    b['cards']['BT1-010']['clauses'].pop(0)
    with pytest.raises(SourceBindingError):validate_binding(b,inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('ledger',['corrections','selections','adjudications'])
def test_nonempty_adapter_ledgers_explicitly_unsupported(ledger):
    v,raw,ids=fixture();v[ledger][ledger]=[{'not_an_approval':True}]
    with pytest.raises(SourceBindingError,match='unsupported '+ledger):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

def test_input_unknown_collection_and_duplicate_json_refused():
    v,raw,ids=fixture();i=pack(v);i['corrections']=b'{"schema_version":1,"corrections":[],"unknown":[]}'
    with pytest.raises(SourceBindingError):derive_binding(inputs=i,raw_pages=raw,expected_universe=ids)
    i['corrections']=b'{"schema_version":1,"schema_version":1,"corrections":[]}'
    with pytest.raises(SourceBindingError):derive_binding(inputs=i,raw_pages=raw,expected_universe=ids)
import pytest
from tools.clause_coverage.source_binding import derive_binding,SourceBindingError
from tools.card_data.serialization import content_digest

@pytest.mark.parametrize('field,value',[('source','digimoncard_io'),('parser_version','official-html-v4'),('source_schema_version','unknown'),('scope','partial:unreviewed')])
def test_manifest_headers_must_bind_supported_official_scope(field,value):
    v,raw,ids=fixture();v['snapshot']['manifest'][field]=value
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

def rebind(v):
    changes={}
    for o in v['snapshot']['observations']:
        old=o['observation_id'];new=content_digest({k:o[k] for k in ('raw_payload_digest','parser_version','source_schema_version','extracted')},exclude_operational=True);o['observation_id']=new;changes[old]=new
    m=v['snapshot']['manifest'];m['observation_ids']=[changes[x] for x in m['observation_ids']]
    for ref in m['observation_refs']:ref['observation_id']=changes[ref['observation_id']]
    for card in v['canonical']['cards'].values():
        for p in card['printings']:p['observation_ids']=[changes[x] for x in p['observation_ids']]
        for p in card['provenance'].values():p['observation_ids']=[changes[x] for x in p['observation_ids']];p['selected_observation_id']=changes[p['selected_observation_id']]
    a=v['source_manifest'];a['sources']['official']['observation_ids']=sorted(changes[x] for x in a['sources']['official']['observation_ids']);a.pop('manifest_revision');a['manifest_revision']=content_digest(a,exclude_operational=True);v['canonical']['source_manifest_digest']=a['manifest_revision']

def test_explicit_unsupported_dual_layout_not_ordinary_path():
    v,raw,ids=fixture()
    for o in v['snapshot']['observations']:
        if o['extracted']['card_id']=='BT1-010':o['extracted']['layout']='dual'
    rebind(v)
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('runtime_kind',[2,'Option'])
def test_runtime_kind_cannot_change_egg_zone_semantics(runtime_kind):
    v,raw,ids=fixture();v['runtime']['BT1-001']['card_kind']=runtime_kind
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('kind',[3,'Digi-Egg'])
def test_runtime_egg_kind_positive_controls(kind):
    v,raw,ids=fixture();v['runtime']['BT1-001']['card_kind']=kind
    b=derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)
    assert b['cards']['BT1-001']['zones']['inherited']

@pytest.mark.parametrize('kind',[True,False,2,'Option',4,'Dual',None])
def test_runtime_egg_kind_strict_negative_controls(kind):
    v,raw,ids=fixture();v['runtime']['BT1-001']['card_kind']=kind
    with pytest.raises(SourceBindingError,match='runtime card kind'):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('field',['schema_version','source','scope','parser_version','source_schema_version','source_locations'])
def test_missing_manifest_headers_fail_closed(field):
    v,raw,ids=fixture();v['snapshot']['manifest'].pop(field)
    with pytest.raises(SourceBindingError):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

@pytest.mark.parametrize('layout',[None,'dual','unknown',''])
def test_unsupported_layout_after_identity_rebind(layout):
    v,raw,ids=fixture()
    for o in v['snapshot']['observations']:
        if o['extracted']['card_id']=='BT1-010':o['extracted']['layout']=layout
    rebind(v)
    with pytest.raises(SourceBindingError,match='layout'):derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)

def test_official_full_manifest_positive_control():
    v,raw,ids=fixture();m=v['snapshot']['manifest']
    assert (m['source'],m['scope'],m['parser_version'],m['source_schema_version'])==('official','full-pool:official-catalog','official-html-v5','world-card-list-html-v1')
    assert derive_binding(inputs=pack(v),raw_pages=raw,expected_universe=ids)['universe']==ids

# A1 selected-field adapter: complete real candidate membership, not whole-book support.
def selection_fixture(card_id='BT4-104',field='effect_description_eng'):
    from tools.card_data.models import SourceObservation,SourceName,AdmissionStatus
    from tools.card_data import normalization,adjudications,serialization
    f=F/'canonical_selection';d=json.loads((f/'fields.json').read_text(encoding='utf-8'));a=gzip.decompress((f/'aggregate.json.gz').read_bytes())
    own={oid:o for oid,o in d['observations'].items()if o['extracted']['card_id']==card_id}
    typed=[SourceObservation(**{**o,'source':SourceName(o['source'])})for o in own.values()]
    rows=tuple(adjudications._row(r,path=Path('<fixture>'))for r in d['adjudications']['adjudications'])
    candidates=[normalization.observation_candidate(o)for o in typed]
    value,provenance,_=normalization._select_field(card_id,field,[c for c in candidates if field in c.fields],admission=AdmissionStatus.ADMITTED,adjudications=rows,field_selection_input_revision=d['revision'])
    # Producer replay fixture; original blocked candidate provenance is preserved
    # in fields.json. This test projection is never emitted to a production tree.
    canonical={'card_id':card_id,'field_path':field,'value':value,'provenance':serialization.canonicalize(provenance)}
    return dict(aggregate=a,observations={oid:canonical_json_bytes(o)for oid,o in own.items()},canonical_field=canonical_json_bytes(canonical),adjudication_ledger=canonical_json_bytes(d['adjudications']),old_input_revision=d['revision'],new_input_revision=d['revision'])

@pytest.mark.parametrize('card,field',[('BT4-104','effect_description_eng'),('BT26-104','effect_description_eng'),('BT26-104','security_effect_description_eng')])
def test_selected_field_exact_producer_replay(card,field):
    from tools.clause_coverage.source_binding import bind_canonical_field
    args=selection_fixture(card,field);before=copy.deepcopy(args);b=bind_canonical_field(**args)
    assert b['selected_observation_id']==json.loads(args['canonical_field'])['provenance']['selected_observation_id']
    assert b['adjudication_id'] and args==before

@pytest.mark.parametrize('mutation',['missing','duplicate','foreign','field','value','digest','old_revision','new_revision','bool_version','member','selected','provenance','canonical_value','observation_missing','observation_foreign','aggregate','unequal_revisions','caller_bool'])
def test_selected_field_adversarial(mutation):
    from tools.clause_coverage.source_binding import bind_canonical_field
    args=selection_fixture();ledger=json.loads(args['adjudication_ledger']);row=next(r for r in ledger['adjudications']if r['card_id']=='BT4-104');canonical=json.loads(args['canonical_field'])
    if mutation=='missing':ledger['adjudications'].remove(row)
    elif mutation=='duplicate':ledger['adjudications'].append({**row,'adjudication_id':'duplicate-current'})
    elif mutation=='foreign':row['card_id']='BT26-104'
    elif mutation=='field':row['field_path']='security_effect_description_eng'
    elif mutation=='value':row['new_value']='invented'
    elif mutation=='digest':row['new_digest']='0'*64
    elif mutation=='old_revision':row['old_input_revision']='0'*64
    elif mutation=='new_revision':row['new_input_revision']='0'*64
    elif mutation=='bool_version':row['schema_version']=True
    elif mutation=='member':row['provenance_observation_ids'].pop()
    elif mutation=='selected':row['selected_observation_id']=row['provenance_observation_ids'][0]
    elif mutation=='provenance':canonical['provenance']['adjudicated']=False
    elif mutation=='canonical_value':canonical['value']='invented'
    elif mutation=='observation_missing':args['observations'].pop(next(iter(args['observations'])))
    elif mutation=='observation_foreign':args['observations']['0'*64]=next(iter(args['observations'].values()))
    elif mutation=='aggregate':args['aggregate']+=b'corruption'
    elif mutation=='unequal_revisions':args['old_input_revision']='0'*64
    elif mutation=='caller_bool':args['new_input_revision']=True
    args['adjudication_ledger']=canonical_json_bytes(ledger);args['canonical_field']=canonical_json_bytes(canonical)
    with pytest.raises(SourceBindingError):bind_canonical_field(**args)

def test_a1_does_not_promote_original_unreviewed_candidate_provenance():
    from tools.clause_coverage.source_binding import bind_canonical_field
    args=selection_fixture();d=json.loads((F/'canonical_selection/fields.json').read_text(encoding='utf-8'));card=d['cards']['BT4-104']
    field=json.loads(args['canonical_field']);field['provenance']=card['provenance']['effect_description_eng'];assert field['provenance']['adjudicated'] is False
    args['canonical_field']=canonical_json_bytes(field)
    with pytest.raises(SourceBindingError,match='producer replay'):bind_canonical_field(**args)

@pytest.mark.parametrize('mutation',['ledger_bool','provenance_bool_alias','unknown_field','duplicate_id','old_value','aggregate_member_drop','wrong_printing','observation_changed'])
def test_a1_additional_wire_and_membership_refusals(mutation):
    from tools.clause_coverage.source_binding import bind_canonical_field
    args=selection_fixture();ledger=json.loads(args['adjudication_ledger']);field=json.loads(args['canonical_field']);row=next(r for r in ledger['adjudications']if r['card_id']=='BT4-104')
    if mutation=='ledger_bool':ledger['schema_version']=True
    elif mutation=='provenance_bool_alias':field['provenance']['adjudicated']=1
    elif mutation=='unknown_field':field['field_path']='xros_req'
    elif mutation=='duplicate_id':ledger['adjudications'].append(copy.deepcopy(row))
    elif mutation=='old_value':row['old_value']=[]
    elif mutation=='aggregate_member_drop':
        a=json.loads(args['aggregate']);a['sources']['official']['observation_refs'].pop();args['aggregate']=canonical_json_bytes(a)
    else:
        oid=next(k for k,v in args['observations'].items() if json.loads(v)['source']=='official');o=json.loads(args['observations'][oid])
        if mutation=='wrong_printing':o['extracted']['printing']['printing_id']='BT9-999'
        else:o['extracted']['card_id']='BT9-999'
        args['observations'][oid]=canonical_json_bytes(o)
    args['adjudication_ledger']=canonical_json_bytes(ledger);args['canonical_field']=canonical_json_bytes(field)
    with pytest.raises(SourceBindingError):bind_canonical_field(**args)



def condition_fixture(cid='EX12-076'):
    f=F/'conditions';d=json.loads((f/'inputs.json').read_text(encoding='utf-8'))
    return dict(aggregate=gzip.decompress((F/'canonical_selection/aggregate.json.gz').read_bytes()),observations={oid:canonical_json_bytes(o)for oid,o in d['observations'].items()if o['extracted']['card_id']==cid},canonical_card=canonical_json_bytes(d['cards'][cid]),runtime_projection=canonical_json_bytes(d['runtime'][cid]),selection_ledger=(f/'card_compatibility_selections.json').read_bytes(),correction_document=(f/'card_corrections.json').read_bytes(),image_artifacts={path:(f/name).read_bytes()for path,name in d['images'].items()},raw_pages={url:gzip.decompress((f/name).read_bytes())for url,name in d['raw_files'].items()},old_input_revision=d['revision'],new_input_revision=d['revision'])

@pytest.mark.parametrize('cid',['EX12-046','EX12-063','EX12-076','EX3-010'])
def test_condition_units_real_four_card_batch(cid):
    from tools.clause_coverage.source_binding import bind_condition_units,validate_condition_units
    args=condition_fixture(cid);before=copy.deepcopy(args);b=bind_condition_units(**args);validate_condition_units(b,**args);assert args==before
    assert b['projection']==json.loads(args['runtime_projection'])['xros_req']
    if cid=='EX3-010':
        assert len(b['units'])==1 and b['units'][0]['kind']=='image-correction'
        assert 'observation_id' not in b['units'][0] and b['units'][0]['image_sha256']
    else:
        assert any('Assembly' in u['text']for u in b['units'])
        assert all(u['kind']=='official-section'for u in b['units'])

@pytest.mark.parametrize('mutation',['runtime','missing_observation','foreign_member','raw_change','stale_selection','selection_value','missing_image','changed_image','correction_stale','correction_value','selection_overlap','wrong_card','old_revision','bool_schema','canonical_member'])
def test_condition_adapter_refusals(mutation):
    from tools.clause_coverage.source_binding import bind_condition_units
    correction_case=mutation in ['missing_image','changed_image','correction_stale','correction_value','selection_overlap','bool_schema'];args=condition_fixture('EX3-010'if correction_case else 'EX12-076')
    if mutation=='runtime':
        r=json.loads(args['runtime_projection']);r['xros_req']=r['xros_req'].split('\n')[0];args['runtime_projection']=canonical_json_bytes(r)
    elif mutation=='missing_observation':args['observations'].pop(next(iter(args['observations'])))
    elif mutation=='foreign_member':args['observations']['0'*64]=next(iter(args['observations'].values()))
    elif mutation=='raw_change':args['raw_pages']={k:v+b'changed'for k,v in args['raw_pages'].items()}
    elif mutation in ['stale_selection','selection_value']:
        d=json.loads(args['selection_ledger']);b=bind_condition_units(**args);values=b['candidate_values'];oid=sorted(values)[0]
        r=dict(schema_version=1,selection_id='synthetic-negative-only',selection_kind='official_condition_projection',card_id='EX12-076',field_path='xros_req',source_manifest_revision=args['new_input_revision'],candidate_values=values,candidate_set_digest=content_digest(values),selected_observation_id=oid,selected_value=values[oid],selected_value_digest=content_digest(values[oid]),reviewer='test',reason='explicit fault injection, not production approval',reviewed_at='2026-10-12T00:00:00Z')
        r['source_manifest_revision'if mutation=='stale_selection'else 'selected_value']='0'*64;d['selections'].append(r);args['selection_ledger']=canonical_json_bytes(d)
    elif mutation=='missing_image':args['image_artifacts']={}
    elif mutation=='changed_image':args['image_artifacts']={k:v+b'changed'for k,v in args['image_artifacts'].items()}
    elif mutation in ['correction_stale','correction_value','bool_schema']:
        d=json.loads(args['correction_document'])
        if mutation=='bool_schema':d['schema_version']=True
        else:d['compatibility_corrections'][0]['source_manifest_revision'if mutation=='correction_stale'else 'value']='0'*64
        args['correction_document']=canonical_json_bytes(d)
    elif mutation=='selection_overlap':
        d=json.loads(args['selection_ledger']);r=json.loads(args['correction_document'])['compatibility_corrections'][0];d['selections'].append(dict(schema_version=1,selection_id='synthetic-overlap',selection_kind='official_condition_projection',card_id='EX3-010',field_path='xros_req',source_manifest_revision=r['source_manifest_revision'],candidate_values=r['candidate_values'],candidate_set_digest=r['candidate_set_digest'],selected_observation_id=r['base_observation_id'],selected_value=r['base_value'],selected_value_digest=r['base_value_digest'],reviewer='test',reason='negative overlap',reviewed_at=r['reviewed_at']));args['selection_ledger']=canonical_json_bytes(d)
    elif mutation=='wrong_card':
        d=json.loads(args['runtime_projection']);d['card_id']='BT1-001';args['runtime_projection']=canonical_json_bytes(d)
    elif mutation=='old_revision':args['old_input_revision']='0'*64
    elif mutation=='canonical_member':
        d=json.loads(args['canonical_card']);d['printings'].pop();args['canonical_card']=canonical_json_bytes(d)
    with pytest.raises(SourceBindingError):bind_condition_units(**args)

@pytest.mark.parametrize('mutation',['unit_omission','clause_omission','multiplicity','text','fake_official'])
def test_condition_metadata_replayed_not_self_certified(mutation):
    from tools.clause_coverage.source_binding import bind_condition_units,validate_condition_units
    args=condition_fixture('EX3-010'if mutation=='fake_official'else 'EX12-076');b=bind_condition_units(**args)
    if mutation=='unit_omission':b['units'].pop()
    elif mutation=='clause_omission':b['units'][0]['spans'].pop()
    elif mutation=='multiplicity':b['units'].append(copy.deepcopy(b['units'][0]))
    elif mutation=='text':b['units'][-1]['text']='Assembly'
    else:b['units'][0]['kind']='official-section';b['units'][0]['observation_id']='0'*64
    with pytest.raises(SourceBindingError):validate_condition_units(b,**args)

@pytest.mark.parametrize('cid',['EX12-046','EX12-063','EX12-076'])
def test_structured_assembly_does_not_get_folded_into_digivolve(cid):
    from tools.clause_coverage.source_binding import bind_condition_units
    b=bind_condition_units(**condition_fixture(cid))
    special=next(u for u in b['units']if u['condition_type']=='special_play_condition')
    digivolve=next(u for u in b['units']if u['condition_type']=='special_digivolution_condition')
    assert special['text'].startswith('Assembly -') and special['section_index']!=digivolve['section_index']
    assert 'Assembly' not in digivolve['text'] and all('Assembly'not in s['text']for s in digivolve['spans'])
    assert any('Assembly'in s['text']for s in special['spans'])
    assert not any(s['text']=='[Special Digivolution Condition]'for u in b['units']for s in u['spans'])

def test_unsupported_image_path_refused_by_approved_document_loader():
    from tools.clause_coverage.source_binding import bind_condition_units
    a=condition_fixture('EX3-010');d=json.loads(a['correction_document']);d['compatibility_corrections'][0]['image_artifact_path']='../../image.webp';a['correction_document']=canonical_json_bytes(d)
    with pytest.raises(SourceBindingError):bind_condition_units(**a)


def test_unknown_structural_section_is_not_silently_omitted():
    # Synthetic adversarial source-schema mutation, never a production approval.
    from tools.clause_coverage.source_binding import bind_condition_units
    a=condition_fixture();aggregate=json.loads(a['aggregate']);card=json.loads(a['canonical_card']);changes={};newobs={}
    for oid,raw in a['observations'].items():
        o=json.loads(raw)
        if o['source']=='official':o['extracted']['text_sections'].append({'label':'Future Condition','text':'Additional restriction.'})
        new=content_digest({k:o[k]for k in ['raw_payload_digest','parser_version','source_schema_version','extracted']},exclude_operational=True);changes[oid]=new;o['observation_id']=new;newobs[new]=canonical_json_bytes(o)
    for lane in aggregate['sources'].values():
        lane['observation_ids']=[changes.get(x,x)for x in lane['observation_ids']]
        for ref in lane['observation_refs']:ref['observation_id']=changes.get(ref['observation_id'],ref['observation_id'])
    aggregate['membership']['observation_ids']=[changes.get(x,x)for x in aggregate['membership']['observation_ids']]
    aggregate.pop('manifest_revision');rev=content_digest(aggregate,exclude_operational=True);aggregate['manifest_revision']=rev
    for p in card['printings']:p['observation_ids']=[changes[x]for x in p['observation_ids']]
    for p in card['provenance'].values():p['observation_ids']=[changes.get(x,x)for x in p['observation_ids']];p['selected_observation_id']=changes.get(p['selected_observation_id'],p['selected_observation_id'])
    a.update(aggregate=canonical_json_bytes(aggregate),observations=newobs,canonical_card=canonical_json_bytes(card),old_input_revision=rev,new_input_revision=rev)
    with pytest.raises(SourceBindingError,match='unsupported condition source section'):bind_condition_units(**a)
