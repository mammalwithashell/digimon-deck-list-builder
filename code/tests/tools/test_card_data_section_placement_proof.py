"""Exact section placement and physical/source identity adversaries."""
import copy
import json
import hashlib
import pytest
from test_card_data_evolution_completion_proof import fixture as evolution_fixture, URL, CID
from tools.card_data import models, normalization, serialization, source_manifest
from tools.card_data.sources import official
from tools.card_data.section_placement_proof import prove, ProofError

enc=serialization.canonical_json_bytes
sha=lambda b:hashlib.sha256(b).hexdigest()
MAIN='effect_description_eng';INHERITED='inherited_effect_description_eng';SECURITY='security_effect_description_eng'
BODY='[Your Turn] This Digimon gets +2000 DP.'

def fixture(raw_edit=None, printed_body=BODY, main_body='', omit_empty_security=False):
    previous=evolution_fixture(); a=json.loads(previous['official_bytes']);b=json.loads(previous['secondary_bytes'])
    blocks=official._card_blocks(previous['raw_pages'][URL].decode());new=[]
    for i,block in enumerate(blocks):
        inside=f'<div class="cardInfoCol"><dl class="cardInfoBoxSmall"><dt class="cardInfoTitSmall">[Effect]</dt><dd>{main_body or "-"}</dd><dt class="cardInfoTitSmall">[Inherited Effect]</dt><dd>{BODY if i==0 else "-"}</dd><dt class="cardInfoTitSmall">[Security Effect]</dt><dd>{"-" if i==0 else BODY}</dd></dl></div>'
        if omit_empty_security and i==0:inside=inside.replace('<dt class="cardInfoTitSmall">[Security Effect]</dt><dd>-</dd>','')
        new.append(block.replace('</div></div></li>',inside+'</div></div></li>'))
    raw=''.join(new).encode()
    if raw_edit:raw=raw_edit(raw)
    for o,block in zip(a['observations'],new):
        o['extracted']=official._parse_printing(block,source_url=URL,category_id='522001').extracted;o['raw_payload_digest']=sha(raw)
        o['observation_id']=serialization.observation_digest(models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL})))
    selected_values={MAIN:main_body,INHERITED:BODY,SECURITY:''}
    b['observations'][0]['extracted']['canonical_fields'].update(selected_values)
    for o in b['observations']:o['observation_id']=serialization.observation_digest(models.SourceObservation(**(o|{'source':models.SourceName.DIGIMONCARD_IO})))
    scopes=[]
    for s in (a,b):
        m=s['manifest'];m['observation_ids']=[o['observation_id'] for o in s['observations']];m['observation_refs']=[dict(observation_id=o['observation_id'],gameplay_card_id=CID,printing_id=o['extracted']['printing']['printing_id']) for o in s['observations']]
        scopes.append(models.ScopeManifest(**(m|{'source':models.SourceName(m['source']),'observation_refs':tuple(models.ScopeObservationRef(**x) for x in m['observation_refs'])})))
    agg=source_manifest.build_full_pool_source_manifest(scopes).to_dict();rev=agg['manifest_revision'];obs=[models.SourceObservation(**(o|{'source':models.SourceName(o['source'])})) for s in (a,b) for o in s['observations']]
    card=normalization.normalize_card(CID,obs,field_selection_input_revision=rev)
    selected_values={f:card.fields[f] for f in (MAIN,INHERITED,SECURITY)}
    canonical=dict(schema_version=1,source_manifest_digest=rev,cards={CID:serialization.canonicalize(card)})
    cs=[normalization.observation_candidate(o) for o in obs];selected=a['observations'][0]['observation_id'];rows=[]
    for field in (INHERITED,SECURITY):
        cb=normalization._canonical_conflict_binding(card_id=CID,field_path=field,candidates=cs,selected_value=selected_values[field],selected_observation_id=selected,old_input_revision=rev,new_input_revision=rev)
        row=serialization.canonicalize(cb)|dict(approval_required=True,status='missing');row['adjudication_template']=dict(kind='canonical_field_selection',card_id=CID,field_path=field,old_input_revision=rev,new_input_revision=rev,old_digest=cb.candidate_digest,new_digest=cb.selected_value_digest,new_value=selected_values[field],selected_observation_id=selected,provenance_observation_ids=list(cb.candidate_observation_ids));rows.append(row)
    report=dict(candidate_canonical_revision=serialization.content_digest(canonical['cards']),schema_version=1,source_manifest_digest=rev,old_input_revision='2'*64,new_input_revision='3'*64,canonical_conflicts=rows,corrections=[],compatibility_corrections=[])
    images={o['extracted']['printing']['image_url']:b'synthetic image '+o['observation_id'].encode() for o in a['observations']};witnesses=[]
    for o in a['observations']:
        e=o['extracted'];w={k:o[k] for k in ('observation_id','source_record_key','raw_payload_digest')}
        w.update(card_id=CID,printing_id=e['printing']['printing_id'],image_url=e['printing']['image_url'],image_sha256=sha(images[e['printing']['image_url']]),rendered_card_id=CID,visually_checked=True,reviewer='synthetic reviewer',reviewed_at='2026-11-22T00:00:00Z',fields={MAIN:selected_values[MAIN],INHERITED:printed_body,SECURITY:selected_values[SECURITY]},rendered_section_labels={MAIN:'Effect',INHERITED:'Inherited Effect',SECURITY:'Security Effect' if selected_values[SECURITY] else None},section_locations={MAIN:'main_box',INHERITED:'inherited_box',SECURITY:'security_box'},field_bindings={f['field_path']:{'candidate_digest':f['candidate_digest'],'selected_observation_id':selected} for f in rows});witnesses.append(w)
    witness=dict(schema_version=1,report_sha256=sha(enc(report)),source_manifest_digest=rev,new_input_revision=report['new_input_revision'],witnesses=witnesses)
    args=dict(report_bytes=enc(report),official_bytes=enc(a),secondary_bytes=enc(b),aggregate_bytes=enc(agg),canonical_bytes=enc(canonical),witness_bytes=enc(witness),raw_pages={URL:raw},image_bytes=images)
    args['binding_bytes']=enc(dict(schema_version=1,input_sha256={k:sha(args[k]) for k in ('report_bytes','official_bytes','secondary_bytes','aggregate_bytes','canonical_bytes')},plans=[dict(card_id=CID,fields=[INHERITED,SECURITY])],pending_text_keys=sorted([[CID,INHERITED],[CID,SECURITY]])))
    return args

def change(a,key,fn,rebind=False):
    value=json.loads(a[key]);fn(value);a[key]=enc(value)
    if rebind:
        binding=json.loads(a['binding_bytes']);binding['input_sha256']={k:sha(a[k]) for k in binding['input_sha256']};a['binding_bytes']=enc(binding)
        w=json.loads(a['witness_bytes']);w['report_sha256']=sha(a['report_bytes']);a['witness_bytes']=enc(w)

def test_literal_section_swap_proves_pair_not_runtime_approval():
    p=prove(**fixture());assert len(p['eligible'])==1 and len(p['eligible'][0]['adjudication_templates'])==2
    assert p['eligible'][0]['runtime_mapping'][INHERITED]==BODY and p['eligible'][0]['runtime_mapping'][SECURITY]==''
    assert p['status']=='PROOF_ONLY_NO_APPROVALS' and 'adjudications' not in p

@pytest.mark.parametrize('mode',['wrong_section','missing_printing','foreign_printing','changed_image','wrong_card','stale_candidate','selected_id','hidden_parent','faq_parent','duplicate_popup','wrong_label','wrong_body','missing_body','selected_false','candidate_removed','source_ref','runtime_swap','wording','blanket_blank'])
def test_wrong_section_identity_or_literal_body_never_eligible(mode):
    raw_edits={'hidden_parent':lambda r:b'<section hidden>'+r+b'</section>','faq_parent':lambda r:b'<section class="cardFaqAnswer">'+r+b'</section>','duplicate_popup':lambda r:r+r,'wrong_label':lambda r:r.replace(b'[Inherited Effect]',b'[Link Effect]'),'wrong_body':lambda r:r.replace(BODY.encode(),b'Gain 99 memory.'),'missing_body':lambda r:r.replace(BODY.encode(),b'-')}
    a=fixture(raw_edit=raw_edits.get(mode),printed_body=BODY+' ' if mode=='wording' else BODY)
    if mode=='wrong_section':change(a,'witness_bytes',lambda w:w['witnesses'][0]['section_locations'].update({INHERITED:'security_box'}))
    elif mode=='missing_printing':change(a,'witness_bytes',lambda w:w['witnesses'].pop())
    elif mode=='foreign_printing':change(a,'witness_bytes',lambda w:w['witnesses'][0].update(source_record_key='foreign'))
    elif mode=='changed_image':a['image_bytes'][next(iter(a['image_bytes']))]+=b'changed'
    elif mode=='wrong_card':change(a,'witness_bytes',lambda w:w['witnesses'][0].update(rendered_card_id='BT1-002'))
    elif mode=='stale_candidate':change(a,'witness_bytes',lambda w:w['witnesses'][0]['field_bindings'][INHERITED].update(candidate_digest='f'*64))
    elif mode=='selected_id':change(a,'witness_bytes',lambda w:w['witnesses'][0]['field_bindings'][INHERITED].update(selected_observation_id='f'*64))
    elif mode=='selected_false':change(a,'report_bytes',lambda q:q['canonical_conflicts'][0].update(selected_value=''),True)
    elif mode=='candidate_removed':change(a,'report_bytes',lambda q:q['canonical_conflicts'][0]['candidate_values'].pop(),True)
    elif mode=='source_ref':change(a,'official_bytes',lambda s:s['manifest']['observation_refs'][0].update(printing_id='BT1-002'),True)
    elif mode=='runtime_swap':change(a,'canonical_bytes',lambda c:c['cards'][CID]['fields'].update({INHERITED:'',SECURITY:BODY}),True)
    elif mode=='blanket_blank':change(a,'witness_bytes',lambda w:w['witnesses'][0]['fields'].update({INHERITED:''}))
    try:p=prove(**a)
    except ProofError:return
    assert not p['eligible'] and len(p['excluded'])==1


def test_upper_persistent_security_aura_stays_main_and_is_not_approved():
    aura='[Security] [All Turns] All of your Digimon get +1000 DP.'
    p=prove(**fixture(main_body=aura));assert len(p['eligible'])==1
    assert p['eligible'][0]['runtime_mapping'][SECURITY]==''
    assert MAIN not in p['eligible'][0]['fields']
    assert p['eligible'][0]['runtime_mapping'][MAIN]==aura

def test_upper_one_shot_security_is_preserved_but_not_in_simple_pair():
    body='[Security] At the end of the battle, play this card without paying the cost.'
    a=fixture(main_body=body,omit_empty_security=True)
    change(a,'witness_bytes',lambda w:[v['section_locations'].update({SECURITY:'security_trigger_in_main_box'}) for v in w['witnesses']])
    p=prove(**a);assert not p['eligible']
    assert p['excluded'][0]['runtime_mapping'][SECURITY]==body
    assert 'upper_one_shot_security_requires_separate_proof' in p['excluded'][0]['reasons']

def test_actual_projector_wrong_zone_is_detected(monkeypatch):
    from tools.card_data import section_placement_proof as proof
    actual=proof.generate.project_canonical_card
    def bad(card):
        r=actual(card);r[INHERITED],r[SECURITY]=r[SECURITY],r[INHERITED];return r
    monkeypatch.setattr(proof.generate,'project_canonical_card',bad)
    with pytest.raises(ProofError,match='runtime zone'):prove(**fixture())

def test_corrected_source_selection_is_excluded_not_promoted():
    a=fixture()
    change(a,'canonical_bytes',lambda c:c['cards'][CID]['fields'].update({INHERITED:'',SECURITY:BODY}),True)
    c=json.loads(a['canonical_bytes'])
    change(a,'report_bytes',lambda q:q.update(corrections=[dict(card_id=CID,field_path=INHERITED)],candidate_canonical_revision=serialization.content_digest(c['cards'])),True)
    p=prove(**a);assert not p['eligible']
    assert 'corrected_canonical_differs_from_source_selection' in p['excluded'][0]['reasons']
