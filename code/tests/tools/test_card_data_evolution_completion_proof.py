"""Synthetic source-bound route completion witnesses; never production approvals."""
import copy
import re
import hashlib
from pathlib import Path
import pytest
from tools.card_data import models, normalization, serialization, source_manifest
from tools.card_data.sources import official
from tools.card_data.evolution_completion_proof import ProofError, prove

URL='https://world.digimoncard.com/cards/?search=true&category=522001'
CID='BT1-001'
sha=lambda b:hashlib.sha256(b).hexdigest()
enc=serialization.canonical_json_bytes

def fixture(*, partial='Green from Lv.2', raw_edit=None, routes=None):
    routes=routes or [dict(slot=1,colors=['Green'],cost=0,from_level=2)]
    def block(pid, body):
        return f'''<li class="image_lists_item"><div class="popupCol" id="{pid}"><div class="cardDetailColInner"><li class="cardNo">{CID}</li><li class="cardType">Digimon</li><li class="cardLv">Lv.3</li><div class="cardTitle">Testmon</div><div class="cardImg"><a class="card_img"><img src="https://world.digimoncard.com/images/cardlist/card/{pid}.png"></a></div><dl><dt class="cardInfoTit">Color</dt><dd><span class="cardColor_green">Green</span></dd><dt class="cardInfoTit">Digivolve Cost 1</dt><dd class="cardInfoData">{body}</dd></dl></div></div></li>'''
    full='<span class="cardColor_green">Green</span> 0 from Lv.2'
    alt=partial.replace('Green','<span class="cardColor_green">Green</span>')
    blocks=[block(CID,full),block(CID+'_P1',alt)]
    raw=(''.join(blocks)).encode()
    if raw_edit:raw=raw_edit(raw)
    observations=[]
    for pid,b in zip((CID,CID+'_P1'),blocks):
        p=official._parse_printing(b,source_url=URL,category_id='522001')
        o=dict(schema_version=1,observation_id='0'*64,source='official',source_record_key=f'522001:{CID}:{pid}',source_url=URL,raw_payload_digest=sha(raw),parser_version='official-html-v5',source_schema_version='world-card-list-html-v1',first_acquired_at='2026-11-22T00:00:00Z',extracted=p.extracted)
        o['observation_id']=serialization.observation_digest(models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL})));observations.append(o)
    secondary=dict(schema_version=1,observation_id='0'*64,source='digimoncard_io',source_record_key=CID,source_url='https://digimoncard.io/api-public/search.php',raw_payload_digest='1'*64,parser_version='digimoncard-io-json-v1',source_schema_version='digimoncard-io-public-api-v1',first_acquired_at='2026-11-22T00:00:00Z',extracted=dict(card_id=CID,canonical_fields={'evo_costs':routes},printing={'printing_id':CID}))
    secondary['observation_id']=serialization.observation_digest(models.SourceObservation(**(secondary|{'source':models.SourceName.DIGIMONCARD_IO})))
    def snapshot(source,obs,parser,schema):
        refs=[dict(observation_id=o['observation_id'],gameplay_card_id=CID,printing_id=o['extracted']['printing']['printing_id']) for o in obs]
        return dict(manifest=dict(schema_version=1,source=source,scope='full-pool:official-catalog' if source=='official' else 'full-pool:digimoncard-io-catalog',source_locations=[o['source_url'] for o in obs],first_acquired_at='2026-11-22T00:00:00Z',parser_version=parser,source_schema_version=schema,observation_ids=[o['observation_id'] for o in obs],observation_refs=refs,complete=True,expected_count=1,failed_ids=[]),observations=obs)
    a=snapshot('official',observations,'official-html-v5','world-card-list-html-v1');b=snapshot('digimoncard_io',[secondary],'digimoncard-io-json-v1','digimoncard-io-public-api-v1')
    scopes=[models.ScopeManifest(**(s['manifest']|{'source':models.SourceName(s['manifest']['source']),'observation_refs':tuple(models.ScopeObservationRef(**x) for x in s['manifest']['observation_refs'])})) for s in (a,b)]
    aggregate=source_manifest.build_full_pool_source_manifest(scopes).to_dict();revision=aggregate['manifest_revision']
    candidates=[normalization.observation_candidate(models.SourceObservation(**(o|{'source':models.SourceName(o['source'])}))) for o in observations+[secondary]]
    selected=observations[0]['observation_id'];binding=normalization._canonical_conflict_binding(card_id=CID,field_path='evo_costs',candidates=candidates,selected_value=routes,selected_observation_id=selected,old_input_revision=revision,new_input_revision=revision)
    row=serialization.canonicalize(binding)|dict(approval_required=True,status='missing')
    row['adjudication_template']=dict(kind='canonical_field_selection',card_id=CID,field_path='evo_costs',old_input_revision=revision,new_input_revision=revision,old_digest=binding.candidate_digest,new_digest=binding.selected_value_digest,new_value=routes,selected_observation_id=selected,provenance_observation_ids=list(binding.candidate_observation_ids))
    report=dict(schema_version=1,source_manifest_digest=revision,old_input_revision='2'*64,new_input_revision='3'*64,canonical_conflicts=[row],corrections=[],compatibility_corrections=[])
    images={o['extracted']['printing']['image_url']:b'synthetic PNG identity '+o['observation_id'].encode() for o in observations}
    witnesses=[]
    for o in observations:
        e=o['extracted'];w={k:o[k] for k in ('observation_id','source_record_key','raw_payload_digest')}
        w.update(card_id=CID,printing_id=e['printing']['printing_id'],image_url=e['printing']['image_url'],image_sha256=sha(images[e['printing']['image_url']]),rendered_card_id=CID,printed_routes=copy.deepcopy(routes),visually_checked=True,reviewer='synthetic independent reviewer',reviewed_at='2026-11-22T00:00:00Z',candidate_digest=binding.candidate_digest,selected_observation_id=selected);witnesses.append(w)
    witness=dict(schema_version=1,report_sha256=sha(enc(report)),source_manifest_digest=revision,new_input_revision=report['new_input_revision'],witnesses=witnesses)
    args=dict(report_bytes=enc(report),official_bytes=enc(a),secondary_bytes=enc(b),aggregate_bytes=enc(aggregate),witness_bytes=enc(witness),raw_pages={URL:raw},image_bytes=images)
    args['binding_bytes']=enc(dict(schema_version=1,input_sha256={k:sha(args[k]) for k in ('report_bytes','official_bytes','secondary_bytes','aggregate_bytes')},conflict_keys=[[CID,'evo_costs']],conflict_count=1))
    return args

def change_json(args,key,fn):
    import json
    value=json.loads(args[key]);fn(value);args[key]=enc(value)

@pytest.mark.parametrize('partial',['Green from Lv.2','0 from Lv.2','from Lv.2','1 from'])
def test_complete_printed_witness_does_not_infer_missing_components(partial):
    a=fixture(partial=partial)
    p=prove(**a)
    if partial=='1 from':assert not p['eligible'] # present cost1 contradicts selected0
    else:
        assert len(p['eligible'])==1 and not p['excluded']
        assert p['eligible'][0]['adjudication_template']['kind']=='canonical_field_selection'
    assert p['status']=='PROOF_ONLY_NO_APPROVALS' and 'adjudications' not in p
    assert p==prove(**a)

def test_missing_printed_level_blocks_even_if_card_level_known():
    a=fixture(partial='0 from');change_json(a,'witness_bytes',lambda w:w['witnesses'][1]['printed_routes'][0].pop('from_level'))
    with pytest.raises(ProofError):prove(**a)

def test_level_completion_requires_exact_full_rendered_badge():
    p=prove(**fixture(partial='0 from'));assert len(p['eligible'])==1
    assert 'from_level' in p['eligible'][0]['official_evidence'][1]['missing_components'][0]['fields']

@pytest.mark.parametrize('mode',['missing_witness','duplicate_witness','foreign_witness','wrong_color','wrong_cost','wrong_level','slot_shuffle','wrong_card','wrong_image','no_review','stale_candidate','selected_id','stale_revision','missing_candidate','changed_raw','changed_image','stale_report'])
def test_missing_malformed_or_stale_binding_refuses(mode):
    a=fixture()
    def edit(w):
        row=w['witnesses'][1]
        if mode=='missing_witness':w['witnesses'].pop()
        elif mode=='duplicate_witness':w['witnesses'].append(copy.deepcopy(row))
        elif mode=='foreign_witness':row['source_record_key']='foreign'
        elif mode=='wrong_color':row['printed_routes'][0]['colors']=['Red']
        elif mode=='wrong_cost':row['printed_routes'][0]['cost']=9
        elif mode=='wrong_level':row['printed_routes'][0]['from_level']=6
        elif mode=='slot_shuffle':row['printed_routes'][0]['slot']=2
        elif mode=='wrong_card':row['rendered_card_id']='BT1-002'
        elif mode=='wrong_image':row['image_url']+='foreign'
        elif mode=='no_review':row['visually_checked']=False
        elif mode=='stale_candidate':row['candidate_digest']='f'*64
        elif mode=='selected_id':row['selected_observation_id']='f'*64
        elif mode=='stale_revision':w['new_input_revision']='f'*64
    if mode=='missing_candidate':change_json(a,'report_bytes',lambda q:q['canonical_conflicts'][0]['candidate_values'].pop())
    elif mode=='changed_raw':a['raw_pages'][URL]+=b'changed'
    elif mode=='changed_image':a['image_bytes'][next(iter(a['image_bytes']))]+=b'changed'
    elif mode=='stale_report':change_json(a,'report_bytes',lambda q:q.update(new_input_revision='f'*64))
    else:change_json(a,'witness_bytes',edit)
    with pytest.raises(ProofError):prove(**a)

@pytest.mark.parametrize('edit',[lambda r:r+r,lambda r:r.replace(b'class="cardInfoData"',b'class="cardInfoData" hidden'),lambda r:r.replace(b'0 from Lv.2',b'9 from Lv.2'),lambda r:r.replace(b'<span class="cardColor_green">Green</span> 0',b'<span class="cardColor_red">Red</span> 0'),lambda r:r.replace(b'Digivolve Cost 1',b'Digivolve Cost 2'),lambda r:r.replace(b'<div class="cardTitle">Testmon',b'<div class="cardTitle">Wrongmon')])
def test_duplicate_hidden_false_base_and_route_identity_never_eligible(edit):
    a=fixture(raw_edit=edit)
    try:p=prove(**a)
    except ProofError:return
    assert not p['eligible'] and len(p['excluded'])==1


@pytest.mark.parametrize('wrapper',[b'<section hidden>',b'<section style="display:none">',b'<section class="cardFaqAnswer">'])
def test_hidden_or_faq_outer_ancestor_cannot_be_discarded(wrapper):
    with pytest.raises(ProofError):prove(**fixture(raw_edit=lambda r:wrapper+r+b'</section>'))

def rebind_outer(a):
    """Bypass only outer byte checks to test the deeper immutable contracts."""
    import json
    change_json(a,'binding_bytes',lambda b:b.update(input_sha256={k:sha(a[k]) for k in ('report_bytes','official_bytes','secondary_bytes','aggregate_bytes')}))
    change_json(a,'witness_bytes',lambda w:w.update(report_sha256=sha(a['report_bytes'])))

@pytest.mark.parametrize('mode',['missing_value','missing_observation','wrong_ref','wrong_record_key','selected_other','template_value','candidate_digest','missing_conflict','scope_incomplete'])
def test_rebound_envelope_cannot_hide_candidate_or_source_tampering(mode):
    a=fixture()
    if mode=='missing_value':change_json(a,'report_bytes',lambda q:q['canonical_conflicts'][0]['candidate_values'].pop())
    elif mode=='missing_observation':change_json(a,'official_bytes',lambda s:s['observations'].pop())
    elif mode=='wrong_ref':change_json(a,'official_bytes',lambda s:s['manifest']['observation_refs'][0].update(printing_id='BT1-002'))
    elif mode=='wrong_record_key':change_json(a,'official_bytes',lambda s:s['observations'][0].update(source_record_key='522001:BT1-002:BT1-001'))
    elif mode=='selected_other':change_json(a,'report_bytes',lambda q:q['canonical_conflicts'][0].update(selected_observation_id=q['canonical_conflicts'][0]['candidate_observation_ids'][0]))
    elif mode=='template_value':change_json(a,'report_bytes',lambda q:q['canonical_conflicts'][0]['adjudication_template'].update(new_value=[]))
    elif mode=='candidate_digest':change_json(a,'report_bytes',lambda q:q['canonical_conflicts'][0].update(candidate_digest='f'*64))
    elif mode=='missing_conflict':change_json(a,'report_bytes',lambda q:q.update(canonical_conflicts=[]))
    elif mode=='scope_incomplete':change_json(a,'official_bytes',lambda s:s['manifest'].update(complete=False))
    rebind_outer(a)
    with pytest.raises(ProofError):prove(**a)

@pytest.mark.parametrize('edit',[lambda r:re.sub(rb'<dt class="cardInfoTit">Digivolve Cost 1</dt><dd class="cardInfoData">.*?</dd>',b'',r),lambda r:r.replace(b'0 from Lv.2',b'0 from TAMER'),lambda r:r.replace(b'0 from Lv.2',b'0 from Lv.2 or DNA')])
def test_whole_slot_and_special_route_confusion_cannot_complete(edit):
    try:p=prove(**fixture(raw_edit=edit))
    except ProofError:return
    assert not p['eligible'] and len(p['excluded'])==1
