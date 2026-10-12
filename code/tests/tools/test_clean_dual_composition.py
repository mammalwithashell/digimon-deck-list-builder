import copy
import gzip
import hashlib
import json
from pathlib import Path
import pytest
from tools.card_data.errors import CardDataError
from test_clean_dual_association import inputs as association_inputs

F = Path(__file__).parents[1] / 'fixtures/card_data/clean_dual_composition'
BASE = F.parent / 'source_binding/conditions'
enc = lambda x: json.dumps(x, ensure_ascii=False, sort_keys=True, indent=2).encode()

def inputs():
    a = association_inputs(); pins = json.loads((F/'pins.json').read_bytes())
    runtime = gzip.decompress((F/'runtime-snapshot.json.gz').read_bytes())
    images = dict(a['images'])
    for row in json.loads(a['correction_document'])['compatibility_corrections']:
        images[row['image_artifact_path']] = (BASE/Path(row['image_artifact_path']).name).read_bytes()
    return dict(aggregate=a['aggregate'], scopes=a['scopes'], observations=a['observations'], raw_pages=a['raw_pages'],
        expected_official_universe=a['expected_official_universe'], old_input_revision=a['expected_source_revision'],
        new_input_revision=a['expected_source_revision'], canonical_card=(F/'canonical.json').read_bytes(),
        runtime_projection=enc(json.loads(runtime)['BT25-085']), adjudication_ledger=gzip.decompress((F/'reviewed-ledger.json.gz').read_bytes()),
        correction_document=a['correction_document'], condition_selections=gzip.decompress((F/'selections.json.gz').read_bytes()),
        image_artifacts=images, association_proposal=a['proposal'], association_pin=a['expected_proposal_sha256'],
        association_review=(F/'association-review.json').read_bytes(), review_pin=pins['review_sha256'],
        association_proof=(F/'association-proof.json').read_bytes(),
        runtime_snapshot=runtime, runtime_snapshot_pin=pins['runtime-snapshot.json.gz']['sha256'],
        registry=gzip.decompress((F/'registry.json.gz').read_bytes()), registry_pin=pins['registry.json.gz']['sha256'])

def compose(a):
    from tools.clause_coverage.dual_card_composition import compose_dual_card
    return compose_dual_card(**a)

def test_actual_clean_dual_original_complete_records():
    a=inputs(); before=copy.deepcopy(a); r=compose(a)
    assert a == before
    assert r['status']=='COMPOSED_DUAL_SOURCE'
    assert r['association_resolution']['authority']=='reviewed-clean-source-image-association'
    assert all(not d['blockers'] for d in r['original_diagnostics'])
    assert r['runtime_source_record']==json.loads(a['runtime_projection'])
    assert r['unclaimed_runtime_fields']==['card_name_jpn','rarity']
    assert len(r['original_diagnostics'])==3
    assert len(r['units'])==4 and len(r['consumption'])==12
    assert r['units'][-1]['role']=='arts-reminder' and not r['units'][-1]['spans']
    assert '＜Use Req. ([Three Musketeers] in text)＞' in r['units'][2]['text']
    assert r['condition_binding']['projection']==json.loads(a['runtime_projection'])['xros_req']
    assert [x['source_face']['kind'] for x in r['zones_by_face']]==['digimon','option']
    assert r['zones_by_face'][0]['source_face']['play_cost'] is None
    assert r['zones_by_face'][1]['source_face']['use_cost']==6
    assert all(x['zones'][z]['presence']=='absent-box' for x in r['zones_by_face'] for z in ('inherited','security'))
    assert len(r['input_sha256']['observations'])==6

@pytest.mark.parametrize('change', ['family','version_bool','review_family','review_decision','review_bool',
    'review_unknown','review_boxes','review_count','review_dependency','review_image','review_proposal',
    'review_pin','proposal_pin','missing_observation','raw','image','canonical_face','canonical_kind',
    'canonical_empty','correction','condition','registry_bool','registry_unknown','registry_index',
    'runtime_kind','runtime_dual','runtime_extra','runtime_missing','runtime_condition','snapshot_pin',
    'private_callback','ledger_bool','source_revision'])
def test_clean_family_exact_binding_and_conservation_refusals(change):
    a=inputs(); review=json.loads(a['association_review']); p=json.loads(a['association_proposal'])
    if change=='family':p['kind']='unknown'
    elif change=='version_bool':p['version']=True
    elif change=='review_family':
        a['association_review']=(F.parent/'dual_card_composition/association-review.json').read_bytes()
    elif change=='review_decision':review['decision']='PENDING'
    elif change=='review_bool':review['schema_version']=True
    elif change=='review_unknown':review['validated']=True
    elif change=='review_boxes':review['independently_verified_raw_boxes'].pop()
    elif change=='review_count':review['actual_observation_count']=True
    elif change=='review_dependency':review['verified_pins']['code/tools/card_data/normalization.py']='0'*64
    elif change=='review_image':review['verified_pins']['code/tests/fixtures/card_data/clean_dual_association/BT25-085.webp']='0'*64
    elif change=='review_proposal':review['verified_pins']['code/tests/fixtures/card_data/clean_dual_association/proposal.json']='0'*64
    elif change=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
    elif change=='raw':a['raw_pages'][next(iter(a['raw_pages']))]+=b'!'
    elif change=='image':a['image_artifacts']['BT25-085.webp']+=b'!'
    elif change.startswith('canonical_'):
        card=json.loads(a['canonical_card'])
        if change=='canonical_face':card['faces'].reverse()
        elif change=='canonical_kind':card['fields']['card_kind']='Tamer'
        else:card['faces'][0]['security_text']='[Security] Draw 1.'
        a['canonical_card']=enc(card)
    elif change=='correction':
        d=json.loads(a['correction_document']);d['corrections'][0]['card_id']='BT25-085';a['correction_document']=enc(d)
    elif change=='condition':a['condition_selections']=b'{"schema_version":true,"selections":[]}'
    elif change.startswith('registry_'):
        d=json.loads(a['registry'])
        if change=='registry_bool':d['registry_version']=True
        elif change=='registry_unknown':d['validated']=True
        else:d['assignments']['BT25-085']=19999
        a['registry']=enc(d);a['registry_pin']=hashlib.sha256(a['registry']).hexdigest()
    elif change.startswith('runtime_'):
        d=json.loads(a['runtime_projection'])
        if change=='runtime_kind':d['card_kind']=1
        elif change=='runtime_dual':d['dual']['effect']='Foreign text'
        elif change=='runtime_extra':d['unreviewed']=True
        elif change=='runtime_missing':d.pop('card_colors')
        else:d['xros_req']=''
        a['runtime_projection']=enc(d);full=json.loads(a['runtime_snapshot']);full['BT25-085']=d
        a['runtime_snapshot']=enc(full);a['runtime_snapshot_pin']=hashlib.sha256(a['runtime_snapshot']).hexdigest()
    elif change=='private_callback':a['_runtime_validator']=lambda **kw:None
    elif change=='ledger_bool':
        d=json.loads(a['adjudication_ledger']);d['schema_version']=True;a['adjudication_ledger']=enc(d)
    elif change=='source_revision':a['new_input_revision']='0'*64
    if change!='review_family':a['association_review']=enc(review)
    a['review_pin']='0'*64 if change=='review_pin' else hashlib.sha256(a['association_review']).hexdigest()
    a['association_proposal']=enc(p)
    a['association_pin']='0'*64 if change=='proposal_pin' else hashlib.sha256(a['association_proposal']).hexdigest()
    if change=='snapshot_pin':a['runtime_snapshot_pin']='0'*64
    with pytest.raises((ValueError,CardDataError)):compose(a)

@pytest.mark.parametrize('change',['cost','use_requirement','arts','colors'])
def test_fresh_snapshot_nested_clean_gameplay_refuses(change):
    a=inputs();record=json.loads(a['runtime_projection'])
    if change=='cost':record['dual']['option']['use_cost']=1
    elif change=='use_requirement':record['dual']['option']['effect_text']=record['dual']['option']['effect_text'].replace('Use Req.','Other Req.')
    elif change=='arts':record['dual']['digimon']['keywords']=[]
    else:record['dual']['digimon']['colors']=[]
    a['runtime_projection']=enc(record);full=json.loads(a['runtime_snapshot']);full['BT25-085']=record
    a['runtime_snapshot']=enc(full);a['runtime_snapshot_pin']=hashlib.sha256(a['runtime_snapshot']).hexdigest()
    with pytest.raises(ValueError):compose(a)

@pytest.mark.parametrize('change',['correction_bytes','correction_history','proof_digest','missing_proof',
    'proof_not_bytes','duplicate_key','proof_bool_version','proof_unknown_field'])
def test_clean_review_binds_exact_complete_historical_proof(change):
    a=inputs()
    if change=='correction_bytes':a['correction_document']+=b'\n'
    elif change=='correction_history':
        d=json.loads(a['correction_document']);d['corrections'].pop();a['correction_document']=enc(d)
    elif change=='missing_proof':a.pop('association_proof')
    elif change=='proof_not_bytes':a['association_proof']=a['association_proof'].decode()
    else:
        review=json.loads(a['association_review'])
        if change=='proof_digest':review['proof_sha256']='0'*64
        else:
            proof=json.loads(a['association_proof'])
            if change=='duplicate_key':a['association_proof']=b'{"version":1,"version":1}'
            else:
                if change=='proof_bool_version':proof['version']=True
                else:proof['unreviewed']=True
                a['association_proof']=enc(proof)
            review['proof_sha256']=hashlib.sha256(a['association_proof']).hexdigest()
        a['association_review']=enc(review);a['review_pin']=hashlib.sha256(a['association_review']).hexdigest()
    with pytest.raises(ValueError):compose(a)

def test_clean_proof_never_authorizes_existing_mismatch_family():
    from test_dual_card_composition import inputs as mismatch_inputs
    a=mismatch_inputs();a['association_proof']=(F/'association-proof.json').read_bytes()
    with pytest.raises(ValueError,match='clean proof cannot authorize mismatch family'):
        compose(a)
