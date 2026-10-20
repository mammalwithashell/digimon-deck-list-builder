"""Opt-in Dual source composition with explicit reviewed image association.

Raw diagnostics remain blocked and linked to a narrowly reviewed resolution.
No source mutation, behavior verdict, clause IDs or book publication occurs.
"""
from copy import deepcopy
from pathlib import Path
from tools.card_data import generate,registry as registry_module,normalization,serialization
from . import dual_kind_association,clean_dual_association,source_units,source_binding,text_split
from .condition_app_fusion import split_app_fusion_condition_units

_KEYS={'aggregate','scopes','observations','canonical_card','runtime_projection','adjudication_ledger','raw_pages','correction_document','condition_selections','old_input_revision','new_input_revision','expected_official_universe'}
_UNCLAIMED={'card_name_jpn','rarity'}
def compose_dual_card(*,image_artifacts,association_proposal,association_pin,association_review,
        review_pin,runtime_snapshot,runtime_snapshot_pin,registry,registry_pin,
        association_proof: bytes | None = None,**inputs):
    try:
        source_units._require(set(inputs)==_KEYS,'unsupported Dual input keys/private injection')
        return _compose(image_artifacts,association_proposal,association_pin,association_review,review_pin,
                        runtime_snapshot,runtime_snapshot_pin,registry,registry_pin,association_proof,inputs)
    except source_units.SourceUnitError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError) as error:
        raise source_units.SourceUnitError('Dual composition refused: '+str(error)) from error

def _compose(images,proposal,proposal_pin,review_raw,review_pin,snapshot,snapshot_pin,registry_raw,registry_pin,proof_raw,inputs):
    req=source_units._require;sha=source_units._sha;js=source_units._json;enc=serialization.canonical_json_bytes
    req(sha(review_raw)==review_pin,'external association review pin mismatch');review=js(review_raw)
    p=js(proposal);owned={r['image_artifact']for r in p['observations']}
    family=p.get('kind');clean=family=='clean-dual-image-association-proposal'
    req(type(p.get('version'))is int and p['version']==1 and family in {'clean-dual-image-association-proposal','dual-image-kind-association-proposal'},'unsupported association family/version')
    proof_inputs=dict(aggregate=inputs['aggregate'],scopes=inputs['scopes'],observations=inputs['observations'],raw_pages=inputs['raw_pages'],images={k:images[k]for k in owned},proposal=proposal,expected_proposal_sha256=proposal_pin,expected_source_revision=inputs['new_input_revision'],expected_official_universe=inputs['expected_official_universe'])
    if clean:
        req(isinstance(proof_raw,bytes) and sha(proof_raw)==review.get('proof_sha256'), 'exact reviewed clean proof bytes required')
        reviewed_proof=js(proof_raw)
        association=clean_dual_association.prove_clean_dual_association(**proof_inputs,correction_document=inputs['correction_document'])
        req(enc(reviewed_proof)==enc(association),'complete clean proof differs from reviewed source/input closure')
        review_keys={'actual_observation_count','decision','existing_mismatch_helper_sha256','freeze_sha256','implementation_changed','independently_verified_raw_boxes','limitations','proof_sha256','reviewer_note','schema_version','scope','tests','verified_pins','visual_review'}
        req(set(review)==review_keys and type(review['schema_version'])is int and review['schema_version']==1
            and review['decision']=='APPROVED' and review['implementation_changed']is False
            and type(review['actual_observation_count'])is int and review['actual_observation_count']==len(association['source_observations']), 'unsupported clean association review')
        req(enc(review['independently_verified_raw_boxes'])==enc(association['associations']), 'clean review does not bind complete exact association')
        pins=review['verified_pins'];prefix='code/tests/fixtures/card_data/clean_dual_association/'
        req(pins[prefix+'proposal.json']==proposal_pin and all(pins[prefix+name]==sha(images[name])for name in owned),'clean reviewed proposal/image pins differ')
        req({k:v for k,v in pins.items()if k.startswith('code/tools/')}=={'code/tools/'+k:v for k,v in association['dependency_sha256'].items()},'clean association dependency closure changed since review')
        authority='reviewed-clean-source-image-association'
    else:
        req(proof_raw is None,'clean proof cannot authorize mismatch family')
        req(review['status']=='ROOT_INDEPENDENT_CODE_AND_EXACT_IMAGE_ASSOCIATION_APPROVED_PROOF_ONLY'
            and review['proposal_sha256']==proposal_pin and review['source_revision']==inputs['new_input_revision']
            and review['all_3_images_visually_verified']is True,'unsupported association approval')
        association=dual_kind_association.prove_dual_kind_association(**proof_inputs)
        checker=next(r for r in review['files']if Path(r['path']).name=='dual_kind_association.py')
        req(checker['sha256']==association['checker_sha256'],'association checker changed since review')
        req(review['dependencies']=={'code/tools/'+k:v for k,v in association['dependency_sha256'].items()},'association dependencies changed since review')
        authority='reviewed-image-derived-kind-association'
    cid=association['card_id'];req(sha(snapshot)==snapshot_pin and sha(registry_raw)==registry_pin,'full runtime/registry pin mismatch')
    snapshot_doc=js(snapshot);runtime=js(inputs['runtime_projection']);req(cid in snapshot_doc and enc(snapshot_doc[cid])==enc(runtime),'runtime is not exact full original record')
    registry_doc=js(registry_raw)
    registry_metadata={'schema_version','registry_version','capacity','padding_index','assignment_count'}
    req(set(registry_doc)==registry_metadata|{'assignments'}
        and all(type(registry_doc[k])is int for k in registry_metadata),'strict registry envelope required')
    req(isinstance(registry_doc['assignments'],dict)
        and set(inputs['expected_official_universe'])<=set(registry_doc['assignments']),
        'registry must retain complete official-universe membership')
    reg=registry_module.StableCardRegistry.from_dict(registry_doc);req(cid in reg.assignments,'missing registry membership')
    ledger=source_units._ledger(inputs['adjudication_ledger'])
    binding=source_binding._bind_dual_condition_units(aggregate=inputs['aggregate'],observations=inputs['observations'],canonical_card=inputs['canonical_card'],runtime_projection=enc({'card_id':cid,'xros_req':runtime['xros_req']}),selection_ledger=inputs['condition_selections'],correction_document=inputs['correction_document'],image_artifacts=images,raw_pages=inputs['raw_pages'],old_input_revision=inputs['old_input_revision'],new_input_revision=inputs['new_input_revision'])
    req(all(u['kind']=='official-section'for u in binding['units']),'image-corrected Dual condition not supported')
    condition_spans=split_app_fusion_condition_units(binding['units'])
    projection_box={}
    def validate_runtime(*,card,runtime,typed,kind):
        replay=normalization.normalize_card(cid,typed.values(),adjudications=ledger,old_input_revision=inputs['old_input_revision'],new_input_revision=inputs['new_input_revision'],field_selection_input_revision=inputs['new_input_revision'])
        req(enc(card.fields)==enc(replay.fields),'full canonical gameplay fields differ from producer replay')
        req(len(card.faces)==2 and [(f.face_id,f.kind.value)for f in card.faces]==[('primary','digimon'),('dual','option')],'exact ordered Dual faces required')
        projected=generate.project_canonical_card(card,legacy_bootstrap={'xros_req':binding['projection']})
        indices=registry_module.compatibility_values(cid,reg.assignments[cid],capacity=reg.capacity)
        expected={**projected,**indices}
        req(set(runtime)==set(expected),'missing or additional runtime fields')
        req(enc({k:v for k,v in runtime.items()if k not in _UNCLAIMED})==enc({k:v for k,v in expected.items()if k not in _UNCLAIMED}),'full gameplay Dual projection mismatch')
        projection_box['claimed_fields']=sorted(set(expected)-_UNCLAIMED)
    context=source_units._validated_context_core(**inputs,_runtime_validator=validate_runtime)
    result=deepcopy(context['result']);diagnostics=association['original_diagnostics'];hard=[];resolved=[]
    for d in diagnostics:
        for b in d['blockers']:
            if clean or b['code']!='TOP_LEVEL_KIND_DISAGREES_WITH_FACES':hard.append(b)
    for b in result['blockers']:
        if b['code']=='UNSUPPORTED_LAYOUT'and b.get('observation_id')in {d['source_observation']['observation_id']for d in diagnostics}:resolved.append(b)
        elif b['code']=='UNSUPPORTED_STRUCTURAL_PROJECTION':resolved.append(b)
        elif b['code']=='UNSUPPORTED_SECTION'and b['label']in {'Special Digivolution Condition','DUAL Effect','DUAL Rule'}:resolved.append(b)
        else:hard.append(b)
    result.update(kind='whole-dual-card-source-composition',status='BLOCKED'if hard else'COMPOSED_DUAL_SOURCE',blockers=hard,
        original_diagnostics=diagnostics,association_resolution={'review_sha256':review_pin,'proposal_sha256':proposal_pin,'authority':authority,'discharged_structure':resolved},
        condition_binding=binding,runtime_source_record=deepcopy(runtime),unclaimed_runtime_fields=sorted(_UNCLAIMED),claimed_runtime_fields=projection_box['claimed_fields'],units=[],consumption=[])
    result['input_sha256'].update(association_review=review_pin,association_proposal=proposal_pin,runtime_snapshot=snapshot_pin,registry=registry_pin,images={k:sha(v)for k,v in sorted(images.items())})
    if clean:result['input_sha256']['association_proof']=sha(proof_raw)
    if hard:
        result['consumption']=[{**deepcopy(row),'disposition':'blocked','unit_indices':[]}for row in result['inventory']];return result
    selected={u['observation_id']for u in binding['units']};req(len(selected)==1,'one complete condition printing required');selected=next(iter(selected))
    diag=next(d for d in diagnostics if d['source_observation']['observation_id']==selected)
    cmap={(u['observation_id'],u['section_index']):i for i,u in enumerate(binding['units'])}
    for occurrence in diag['occurrences']:
        idx=occurrence['section_index'];role=occurrence['role'];req(role is not None,'unhandled Dual occurrence')
        if role=='condition':
            req((selected,idx)in cmap,'missing face condition binding');spans=condition_spans[cmap[(selected,idx)]]['spans']
        elif role=='arts-reminder':spans=[]
        else:spans=[{'kind':s.kind,'timings':s.timings,'keyword':s.keyword,'text':s.text}for s in text_split.split_clauses(occurrence['text'])]
        result['units'].append({**deepcopy(occurrence),'observation_id':selected,'spans':deepcopy(spans),'source_role':'structural-obligation'if role=='arts-reminder'else'printed-face-container'})
    req({u['section_index']for u in result['units']}==set(range(len(diag['global_section_inventory']))),'all selected source containers required')
    result['zones_by_face']=deepcopy(diag['face_inventory'])
    result['consumption']=[{**deepcopy(row),'disposition':'consumed'if row['observation_id']==selected else'retained-alternative','unit_indices':[u['occurrence_index']for u in result['units']if u['observation_id']==row['observation_id']and u['section_index']==row['section_index']]}for row in result['inventory']]
    return result
