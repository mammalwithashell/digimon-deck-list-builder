"""Exact original record fixtures; no canonical replay or runtime rewriting."""
import copy
import gzip
import hashlib
import json
from pathlib import Path
import pytest

F=Path(__file__).parents[1]/'fixtures/card_data/runtime_projection'

@pytest.mark.parametrize('field,value',[
    ('registry_version',True),('padding_index',False),('schema_version',1.0),
    ('registry_version',1.0),('padding_index',0.0),('capacity',20000.0),
    ('assignment_count',None)])
def test_all_registry_headers_strict_integer_with_fresh_pins(field,value):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=json.loads(a['registry_bytes'])
    r[field]=float(r[field])if field=='assignment_count'else value
    a['registry_bytes']=p.enc(r);b=json.loads(a['binding'])
    b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)


def test_unknown_registry_metadata_is_not_silently_discarded():
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=json.loads(a['registry_bytes']);r['invented_contract']='accepted?'
    a['registry_bytes']=p.enc(r);b=json.loads(a['binding'])
    b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

@pytest.mark.parametrize('field,value',[
    ('schema_version',True),('schema_version',1.0),('schema_version','1'),
    ('reason',True),('reason',''),('reviewer',[]),('reviewed_at',None),
    ('adjudication_id',7),('kind',True),('kind','unknown'),('card_id',False),
    ('old_input_revision',True),('new_input_revision','bad'),('old_digest',[]),
    ('new_digest','bad'),('field_path',True),('selected_observation_id',True),
    ('provenance_observation_ids',''),('provenance_observation_ids',[True]),
    ('affected_deck_ids',''),('affected_deck_ids',[False]),('unknown_field',True)])
def test_every_ledger_row_wire_type_checked_with_fresh_pins(field,value):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();d=json.loads(a['adjudication_ledger'])
    # First row is historical/unrelated; even unused rows must validate.
    d['adjudications'][0][field]=value;a['adjudication_ledger']=p.enc(d)
    b=json.loads(a['binding']);b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

@pytest.mark.parametrize('rows',[{},None,[True],[[]]])
def test_invalid_whole_ledger_array_or_row(rows):
    from tools.clause_coverage import runtime_projection_proof as p
    with pytest.raises(p.RuntimeProjectionError):
        p._strict_ledger(p.enc({'schema_version':1,'adjudications':rows}))

def inputs(cid='BT24-031'):
    from tools.clause_coverage import runtime_projection_proof as p
    m=json.loads((F/'metadata.json').read_bytes());d=m['cards'][cid]
    unpack=lambda n:gzip.decompress((F/n).read_bytes())
    corrections=unpack('corrections.json.gz')
    a=dict(canonical_card=(F/(cid+'-canonical.json')).read_bytes(),runtime_record=(F/(cid+'-runtime.json')).read_bytes(),
        registry_bytes=unpack('registry.json.gz'),aggregate=unpack('aggregate.json.gz'),
        observations={k:p.enc(v)for k,v in json.loads(unpack(cid+'-observations.json.gz')).items()},
        correction_document=corrections,condition_selections=unpack('selections.json.gz'),adjudication_ledger=unpack('ledger.json.gz'),
        raw_pages={u:unpack(n)for u,n in d['raw_files'].items()},
        image_artifacts={r['image_artifact_path']:(F/(r['image_sha256']+'.webp')).read_bytes()for r in json.loads(corrections)['compatibility_corrections']},
        old_input_revision=m['source_revision'],new_input_revision=m['source_revision'],expected_official_universe=m['official_universe'])
    a['binding']=p.enc({'schema_version':1,'input_sha256':p.input_hashes(a),'producer_sha256':p.producer_dependencies()})
    return a

def prove(a):
    from tools.clause_coverage.runtime_projection_proof import prove_runtime_projection
    return prove_runtime_projection(**a)

@pytest.mark.parametrize('cid',['BT24-003','BT24-031','BT24-083','BT24-090'])
def test_four_original_candidate_records(cid):
    a=inputs(cid);old=copy.deepcopy(a);r=prove(a)
    assert a==old and r['status']=='EXACT_CLAIMED_RUNTIME_PROJECTION'
    assert r['original_canonical_bytes']==a['canonical_card'].decode()
    assert r['original_runtime_bytes']==a['runtime_record'].decode()
    assert {'index','norm_id','card_index','xros_req','card_kind'}<=set(r['claimed_fields'])
    assert {'rarity','card_name_jpn'}<=set(r['unclaimed_runtime_fields'])
    assert 'ready'not in r and 'clauses'not in r

@pytest.mark.parametrize('field,value',[('card_kind',True),('card_kind',4),('index',True),('card_index',999),('card_id','BT1-001'),('effect_description_eng','invented'),('xros_req','invented')])
def test_runtime_tampering_even_with_rebound_input_pin(field,value):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=json.loads(a['runtime_record']);r[field]=value;a['runtime_record']=p.enc(r)
    b=json.loads(a['binding']);b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

@pytest.mark.parametrize('mutation',['source','image','ledger','producer','registry','condition','canonical','universe','unknown_binding'])
def test_pinned_inputs_never_trust_modified_bytes(mutation):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs()
    if mutation=='source':a['observations'][next(iter(a['observations']))]+=b' '
    elif mutation=='image':a['image_artifacts'][next(iter(a['image_artifacts']))]+=b'x'
    elif mutation=='ledger':a['adjudication_ledger']+=b' '
    elif mutation=='registry':a['registry_bytes']+=b' '
    elif mutation=='condition':a['condition_selections']+=b' '
    elif mutation=='canonical':a['canonical_card']+=b' '
    elif mutation=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
    else:
        b=json.loads(a['binding'])
        if mutation=='producer':b['producer_sha256'][next(iter(b['producer_sha256']))]='0'*64
        else:b['validated']=True
        a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

@pytest.mark.parametrize('change',['lf','nbsp','trim','case'])
def test_runtime_text_not_inverse_normalized(change):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs('BT24-090');r=json.loads(a['runtime_record']);s=r['effect_description_eng']
    r['effect_description_eng']={'lf':s.replace('\r\n','\n'),'nbsp':s.replace(' ','\u00a0',1),'trim':' '+s,'case':s.lower()}[change]
    assert r['effect_description_eng']!=s
    a['runtime_record']=p.enc(r);b=json.loads(a['binding']);b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

def test_output_recomputed():
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=prove(a);r['claimed_fields'].pop()
    with pytest.raises(p.RuntimeProjectionError):p.validate_runtime_projection(r,**a)

@pytest.mark.parametrize('change',['boolean_ledger','boolean_registry','missing_official','missing_secondary','stale_revision','changed_source','changed_image','dual','link','correction'])
def test_rebound_inputs_still_obey_structural_and_a2_contracts(change):
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=json.loads(a['runtime_record'])
    if change=='boolean_ledger':
        x=json.loads(a['adjudication_ledger']);x['schema_version']=True;a['adjudication_ledger']=p.enc(x)
    elif change=='boolean_registry':
        x=json.loads(a['registry_bytes']);x['assignments']['BT24-031']=True;a['registry_bytes']=p.enc(x)
    elif change.startswith('missing_'):
        source='official'if change=='missing_official'else'digimoncard_io'
        a['observations'].pop(next(k for k,v in a['observations'].items()if json.loads(v)['source']==source))
    elif change=='stale_revision':a['new_input_revision']='0'*64
    elif change=='changed_source':
        k=next(iter(a['observations']));x=json.loads(a['observations'][k]);x['extracted']['card_id']='BT1-001';a['observations'][k]=p.enc(x)
    elif change=='changed_image':a['image_artifacts'][next(iter(a['image_artifacts']))]+=b'x'
    elif change=='dual':r['dual']={'digimon':{},'option':{}}
    elif change=='link':r['link_effect_description_eng']='[Link] unsupported'
    else:
        x=json.loads(a['correction_document']);row=x['corrections'][0];row['card_id']='BT24-031';a['correction_document']=p.enc(x)
    a['runtime_record']=p.enc(r);b=json.loads(a['binding']);b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    with pytest.raises(p.RuntimeProjectionError):prove(a)

@pytest.mark.parametrize('text,expected',[
    ('a\nb','a\r\nb'),('a\rb','a\r\nb'),('a\r\nb','a\r\nb'),
    ('a\r\nb\nc\rd','a\r\nb\nc\rd'),('A\u00a0B ','A\u00a0B ')])
def test_exact_existing_serializer_asymmetry(text,expected):
    from tools.card_data import generate
    assert generate._legacy_text(text)==expected

def test_fixtures_are_original_candidate_records_not_reconstructed_views():
    metadata=json.loads((F/'metadata.json').read_bytes())
    for cid,d in metadata['cards'].items():
        for kind in ('canonical','runtime'):
            assert hashlib.sha256((F/(cid+'-'+kind+'.json')).read_bytes()).hexdigest()==d[kind+'_sha256']
    a=inputs('BT24-090');canonical=json.loads(a['canonical_card']);runtime=json.loads(a['runtime_record'])
    assert canonical['fields']['card_kind']=='Option' and runtime['card_kind']==2
    assert '\r\n'in runtime['effect_description_eng'] and '\r\n'not in canonical['fields']['effect_description_eng']

def test_unknown_metadata_is_retained_unclaimed_not_used_as_bootstrap():
    from tools.clause_coverage import runtime_projection_proof as p
    a=inputs();r=json.loads(a['runtime_record']);r['unclaimed_test_metadata']={'x':1};a['runtime_record']=p.enc(r)
    b=json.loads(a['binding']);b['input_sha256']=p.input_hashes(a);a['binding']=p.enc(b)
    result=prove(a)
    assert result['unclaimed_runtime_fields']['unclaimed_test_metadata']=={'x':1}
    assert 'unclaimed_test_metadata'not in result['claimed_fields']

def test_dependency_closure_includes_loaded_package_initializers():
    from tools.clause_coverage import runtime_projection_proof as p
    deps=p.producer_dependencies()
    assert {'tools.card_data','tools.clause_coverage','tools.card_data.generate',
            'tools.card_data.registry','tools.card_data.models','tools.card_data.serialization',
            'tools.card_data.normalization','tools.card_data.compatibility_corrections',
            'tools.card_data.compatibility_selections','tools.card_data.sources.official',
            'tools.clause_coverage.source_binding'}<=set(deps)
