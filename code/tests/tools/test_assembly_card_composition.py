"""Whole original R3 source plus exact legacy recipe; never readiness."""
import json
import pytest
from tools.card_data.errors import CardDataError
from tools.clause_coverage import runtime_projection_proof as projection
from test_link_card_composition import snapshot,rebind_snapshot,sha,enc
from test_assembly_recipe_binding import inputs as recipe_inputs


def repin(a):
    proof={k:v for k,v in a.items()if k not in {'snapshot','expected_extraction_sha256','scopes','yaml_bytes','expected_yaml_sha256','runtime_binding'}}
    a['runtime_binding']=enc({'schema_version':1,'input_sha256':projection.input_hashes(proof),'producer_sha256':projection.producer_dependencies()})


def inputs(snapshot,cid='BT24-081'):
    recipe=recipe_inputs(cid);c=recipe['condition_inputs'];record=snapshot.card(cid)
    assert record['canonical_card']==c['canonical_card']
    assert json.loads(record['runtime_record'])['digixros_costs']==json.loads(recipe['legacy_field'])
    a=dict(snapshot=snapshot,expected_extraction_sha256=snapshot.metadata()['extraction_artifact_sha256'],
        runtime_record=record['runtime_record'],registry_bytes=record['registry_bytes'],canonical_card=record['canonical_card'],
        aggregate=snapshot.file('aggregate'),scopes={s:snapshot.file(r)for s,r in [('official','official_scope'),('digimoncard_io','secondary_scope')]},
        observations=c['observations'],raw_pages=c['raw_pages'],image_artifacts=c['image_artifacts'],
        adjudication_ledger=snapshot.file('adjudications'),correction_document=snapshot.file('corrections'),condition_selections=snapshot.file('selections'),
        old_input_revision=snapshot.metadata()['source_revision'],new_input_revision=snapshot.metadata()['source_revision'],
        expected_official_universe=snapshot.metadata()['official_card_ids'],
        yaml_bytes=recipe['yaml_bytes'],expected_yaml_sha256=recipe['expected_yaml_sha256'])
    repin(a)
    return a


def compose(a):
    from tools.clause_coverage.assembly_card_composition import compose_assembly_card
    return compose_assembly_card(**a)


@pytest.mark.parametrize('cid',['BT24-081','BT26-073'])
def test_original_source_and_recipe_have_separate_implementation_status(snapshot,cid):
    a=inputs(snapshot,cid);r=compose(a)
    assert r['source_status']=='COMPOSED_RULE_CARD'
    assert len(r['claimed_fields'])==20 and r['claimed_fields'][-1]=='digixros_costs'
    assert r['projected_claimed_values']['digixros_costs']==json.loads(a['runtime_record'])['digixros_costs']
    assert set(r['unclaimed_runtime_fields'])=={'rarity','card_name_jpn'}
    assert r['original_runtime_bytes'].encode()==a['runtime_record']
    assert r['original_canonical_bytes'].encode()==a['canonical_card']
    assert r['source_composition']['inventory']==r['ordinary_runtime_binding']['source_composition']['inventory']
    assert r['source_composition']['units']==r['ordinary_runtime_binding']['source_composition']['units']
    assert len(r['recipe_binding']['source_occurrences'])==2
    assert r['implementation_correspondence']['native_compilation_verified'] is False
    if cid=='BT24-081':
        assert r['status']=='SOURCE_RUNTIME_COMPOSED_WITH_IMPLEMENTATION_OBLIGATIONS'
        assert r['implementation_correspondence']['status']=='BLOCKED'
        assert r['implementation_correspondence']['unresolved_obligations'][0]['code']=='NAMED_MATERIAL_KIND_CLOSURE_REQUIRED'
    else:
        assert r['status']=='SOURCE_RUNTIME_COMPOSED'
        assert len(r['recipe_binding']['condition_binding']['units'])==2
        assert r['implementation_correspondence']['unresolved_obligations']==[]
    assert r['admission_performed'] is False

@pytest.mark.parametrize('change',['runtime','canonical','ledger','correction','missing_observation','raw','yaml_cost','yaml_unknown','legacy_argument','witness','dependency_pin'])
def test_whole_input_and_untrusted_proof_refusals(snapshot,change):
    a=inputs(snapshot)
    if change in ('runtime','canonical','ledger','correction'):
        k={'runtime':'runtime_record','canonical':'canonical_card','ledger':'adjudication_ledger','correction':'correction_document'}[change];a[k]+=b' '
    elif change=='missing_observation':a['observations'].pop(next(iter(a['observations'])))
    elif change=='raw':a['raw_pages'][next(iter(a['raw_pages']))]+=b'!'
    elif change=='legacy_argument':a['legacy_field']=b'[]'
    elif change=='witness':a['validated_kind_witness']={'approved':True}
    elif change=='dependency_pin':
        b=json.loads(a['runtime_binding']);b['producer_sha256']={};a['runtime_binding']=enc(b)
    else:
        old,new=(b'cost: 6',b'cost: 7')if change=='yaml_cost'else(b'cost: 6',b'cost: 6\n    unknown: true')
        a['yaml_bytes']=a['yaml_bytes'].replace(old,new);a['expected_yaml_sha256']=sha(a['yaml_bytes'])
    with pytest.raises((ValueError,CardDataError,TypeError)):compose(a)


@pytest.mark.parametrize('change',['legacy_bool','legacy_double','legacy_order','legacy_unknown',
    'gameplay_unknown','kind_bool','condition','main','registry_bool','registry_unknown','canonical_provenance'])
def test_fresh_container_pins_do_not_prove_changed_semantics(snapshot,tmp_path,change):
    a=inputs(snapshot)
    role='registry'if change.startswith('registry_')else'canonical'if change=='canonical_provenance'else'runtime'
    def mutate(d):
        if role=='registry':
            if change=='registry_bool':d['registry_version']=True
            else:d['unknown']=True
        elif role=='canonical':d['cards']['BT24-081']['provenance']['effect_description_eng']['derived']=True
        else:
            row=d['BT24-081']
            if change=='legacy_bool':row['digixros_costs'][0]['elements'][0]['count']=True
            elif change=='legacy_double':row['digixros_costs'][0]['reduce_cost_per_card']=12
            elif change=='legacy_order':row['digixros_costs'][0]['elements'].reverse()
            elif change=='legacy_unknown':row['digixros_costs'][0]['approved']=True
            elif change=='gameplay_unknown':row['unknown_gameplay']=[]
            elif change=='kind_bool':row['card_kind']=False
            elif change=='condition':row['xros_req']=''
            else:row['effect_description_eng']+=' Draw 1.'
    changed=rebind_snapshot(snapshot,tmp_path,role,mutate)
    record=changed.card('BT24-081')
    a.update(snapshot=changed,expected_extraction_sha256=changed.metadata()['extraction_artifact_sha256'],**record)
    repin(a)
    with pytest.raises((ValueError,CardDataError,TypeError)):compose(a)


@pytest.mark.parametrize('change',['strip_obligation','extra_unit','legacy','claim_native','source_inventory'])
def test_output_revalidation_recomputes_proof_and_conservation(snapshot,change):
    from tools.clause_coverage.assembly_card_composition import validate_assembly_card
    a=inputs(snapshot);r=compose(a)
    if change=='strip_obligation':r['implementation_correspondence']['unresolved_obligations']=[]
    elif change=='extra_unit':r['source_composition']['units'].append(r['source_composition']['units'][0])
    elif change=='legacy':r['projected_claimed_values']['digixros_costs']=[]
    elif change=='claim_native':r['implementation_correspondence']['native_compilation_verified']=True
    else:r['source_composition']['inventory'].pop()
    with pytest.raises(ValueError):validate_assembly_card(r,**a)


def test_dependency_inventory_covers_both_proofs_and_actual_producers():
    from tools.clause_coverage.assembly_card_composition import assembly_dependencies
    deps=assembly_dependencies()
    assert {'tools.clause_coverage.assembly_card_composition','tools.clause_coverage.assembly_recipe_binding',
        'tools.clause_coverage.runtime_rule_composition','tools.clause_coverage.runtime_snapshot',
        'tools.clause_coverage.rule_card_composition','tools.clause_coverage.source_units',
        'tools.clause_coverage.source_binding','tools.card_data.generate',
        'tools.ingest_cards','tools.xros_cost_parser'}<=set(deps)
    assert all(len(h)==64 for h in deps.values())
