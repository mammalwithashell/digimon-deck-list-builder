"""Actual immutable R3 Link inputs; no prospective canonical edits."""
import copy,gzip,hashlib,json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes as enc
from tools.card_data.errors import CardDataError
F=Path(__file__).parents[1]/'fixtures/card_data/link_card_composition'
S=F.parent/'runtime_snapshot'
sha=lambda b:hashlib.sha256(b).hexdigest()

@pytest.fixture(scope='module')
def snapshot(tmp_path_factory):
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    root=tmp_path_factory.mktemp('link-snapshot');raw=(S/'extraction.json').read_bytes();art=json.loads(raw)
    for role,row in art['files'].items():
        p=root/row['path'];p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(gzip.decompress((S/(role+'.json.gz')).read_bytes()))
    return load_runtime_snapshot(candidate_root=root,extraction_artifact=raw,expected_extraction_sha256=sha(raw))

def inputs(snapshot,cid='BT21-074'):
    record=snapshot.card(cid);m=json.loads((F/'manifest.json').read_bytes());row=m['cards'][cid]
    def read(n):
        b=(F/n).read_bytes();assert sha(b)==m['sha256'][n];return b
    a=dict(snapshot=snapshot,expected_extraction_sha256=snapshot.metadata()['extraction_artifact_sha256'],
        runtime_record=record['runtime_record'],registry_bytes=record['registry_bytes'],canonical_card=record['canonical_card'],
        aggregate=snapshot.file('aggregate'),scopes={s:snapshot.file(r)for s,r in [('official','official_scope'),('digimoncard_io','secondary_scope')]},
        observations={k:read(n)for k,n in row['observations'].items()},raw_pages={k:gzip.decompress(read(n))for k,n in row['raw_pages'].items()},
        adjudication_ledger=snapshot.file('adjudications'),correction_document=snapshot.file('corrections'),condition_selections=snapshot.file('selections'),
        old_input_revision=snapshot.metadata()['source_revision'],new_input_revision=snapshot.metadata()['source_revision'],
        expected_official_universe=snapshot.metadata()['official_card_ids'],
        yaml_bytes=(F.parent/'link_yaml_binding'/f'{cid}.yaml').read_bytes())
    a['expected_yaml_sha256']=sha(a['yaml_bytes']);return a

def compose(a):
    from tools.clause_coverage.link_card_composition import compose_link_card
    return compose_link_card(**a)

def test_actual_positive_all_original_fields_and_occurrences(snapshot):
    a=inputs(snapshot);r=compose(a)
    assert r['status']=='COMPOSED_LINK_CARD'
    assert len(r['claimed_fields'])==20
    assert r['original_runtime_bytes'].encode()==a['runtime_record']
    assert r['original_canonical_bytes'].encode()==a['canonical_card']
    assert len(r['inventory'])==4 and len(r['consumption'])==4
    assert r['zones']['inherited']['value']==''
    assert r['zones']['link']['value']=='[When Linking] Delete 1 of your opponentÃ¢â‚¬â„¢s level 4 or lower Digimon.'.replace('Ã¢â‚¬â„¢',"'")
    assert len(r['structural_obligations'])==2
    assert len([o for o in r['structural_obligations']if o['role']=='dp'])==1
    assert '[Special Rule] +4000 DP' in r['zones']['effect']['value']
    assert all('+4000 DP'not in s['text']for s in r['zones']['effect']['spans'])
    assert r['condition_binding']['a2_section_coverage_claimed'] is False
    assert set(r['unclaimed_runtime_fields'])=={'rarity','card_name_jpn'}
    assert r['native_compilation_verified'] is False

def test_original_bt21_071_canonical_conflict_stays_refused(snapshot):
    with pytest.raises((ValueError,CardDataError),match='required canonical source conflict'):compose(inputs(snapshot,'BT21-071'))

@pytest.mark.parametrize('change',['canonical','runtime','scope','missing_observation','raw','yaml_cost','yaml_dp','yaml_unknown','yaml_pin','registry','ledger','condition','correction','source_revision','private_flag'])
def test_binding_refusals(snapshot,change):
    a=inputs(snapshot)
    if change in ('canonical','runtime','registry','ledger','condition','correction'):
        key={'canonical':'canonical_card','runtime':'runtime_record','registry':'registry_bytes','ledger':'adjudication_ledger','condition':'condition_selections','correction':'correction_document'}[change]
        a[key]+=b' '
    elif change=='scope':a['scopes']['official']+=b' '
    elif change=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
    elif change=='raw':a['raw_pages'][next(iter(a['raw_pages']))]+=b'!'
    elif change=='yaml_pin':a['expected_yaml_sha256']='0'*64
    elif change=='source_revision':a['new_input_revision']='0'*64
    elif change=='private_flag':a['validated']=True
    else:
        old,new={'yaml_cost':(b'cost: 3',b'cost: 4'),'yaml_dp':(b'dp_modifier: 4000',b'dp_modifier: 5000'),'yaml_unknown':(b'kind: link_condition',b'kind: link_condition\n    unknown: true')}[change]
        assert old in a['yaml_bytes'];a['yaml_bytes']=a['yaml_bytes'].replace(old,new);a['expected_yaml_sha256']=sha(a['yaml_bytes'])
    with pytest.raises((ValueError,CardDataError)):compose(a)


def rebind_snapshot(snapshot,tmp_path,role,mutate):
    """Adversarial new test pin, never a claim of independently reviewed bytes."""
    from tools.clause_coverage.runtime_snapshot import load_runtime_snapshot
    art=json.loads((S/'extraction.json').read_bytes())
    for name,row in art['files'].items():
        raw=snapshot.file(name)
        if name==role:
            d=json.loads(raw);mutate(d);raw=enc(d)
            if role in ('canonical','runtime'):
                records=d['cards'] if role=='canonical'else d
                art[role+'_record_sha256']={k:sha(enc(v))for k,v in records.items()}
            row.update(sha256=sha(raw),bytes=len(raw))
        dest=tmp_path/row['path'];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(raw)
    raw=enc(art)
    return load_runtime_snapshot(candidate_root=tmp_path,extraction_artifact=raw,expected_extraction_sha256=sha(raw))

@pytest.mark.parametrize('change',['kind_bool','link_effect','inherited','main_dp','condition','gameplay_extra',
    'registry_bool','registry_unknown','canonical_attribute','canonical_provenance'])
def test_fresh_test_pins_do_not_approve_altered_gameplay(snapshot,tmp_path,change):
    role='registry' if change.startswith('registry_')else 'canonical' if change.startswith('canonical_')else 'runtime'
    def mutate(d):
        if role=='registry':
            if change=='registry_bool':d['registry_version']=True
            else:d['approved']=True
        elif role=='canonical':
            r=d['cards']['BT21-074']
            if change=='canonical_attribute':r['fields']['attribute_eng']=['Virus']
            else:r['provenance']['attribute_eng']['derived']=True
        else:
            r=d['BT21-074']
            if change=='kind_bool':r['card_kind']=False
            elif change=='link_effect':r['link_effect_description_eng']='[When Linking] Draw 9.'
            elif change=='inherited':r['inherited_effect_description_eng']=r['link_effect_description_eng']
            elif change=='main_dp':r['effect_description_eng']=r['effect_description_eng'].replace('+4000 DP','+5000 DP')
            elif change=='condition':r['xros_req']=''
            else:r['digixros_costs']=[]
    rebound=rebind_snapshot(snapshot,tmp_path,role,mutate)
    with pytest.raises((ValueError,CardDataError)):compose(inputs(rebound))

@pytest.mark.parametrize('old,new',[(b'filter: { trait_has: Appmon }',b'filter: { trait_has: Tool }'),
    (b'scope: linked',b'scope: inherited'),(b'target: {}',b'target: { level_eq: 4 }'),
    (b'cost: 3',b'cost: true'),(b'kind: link_condition',b'kind: link_condition\n    kind: link_condition')])
def test_yaml_rebound_pin_preserves_exact_source_cost_filter_host(snapshot,old,new):
    a=inputs(snapshot);assert old in a['yaml_bytes'];a['yaml_bytes']=a['yaml_bytes'].replace(old,new)
    a['expected_yaml_sha256']=sha(a['yaml_bytes'])
    with pytest.raises(ValueError):compose(a)

def test_output_tamper_requires_complete_replay(snapshot):
    from tools.clause_coverage.link_card_composition import validate_link_card
    a=inputs(snapshot);r=compose(a);r['structural_obligations'][0]['char_start']+=1
    with pytest.raises(ValueError):validate_link_card(r,**a)
