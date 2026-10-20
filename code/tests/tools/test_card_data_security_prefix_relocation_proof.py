"""Frozen actual Security boxes and exact paired migration boundaries."""
import copy
import hashlib
import json
from pathlib import Path
import pytest
from tools.card_data import security_prefix_relocation_proof as proof
from tools.card_data.serialization import canonical_json_bytes, content_digest
from tools.card_data.errors import AdjudicationError
from tools.card_data import normalization
from tools.card_data.models import SourceObservation, SourceName
from tools.card_data.serialization import canonicalize

FIX = Path(__file__).parents[1] / 'fixtures/card_data/security_relocation'
I = 'inherited_effect_description_eng'
S = 'security_effect_description_eng'

def bundle(cid='AD1-021'):
    meta=json.loads((FIX/(cid+'.json')).read_text(encoding='utf-8'))
    raw=(FIX/(cid+'.html')).read_bytes()
    assert hashlib.sha256(raw).hexdigest()==meta['fixture_sha256']
    o=copy.deepcopy(meta['original_observation'])
    text=next(s['text'] for s in o['extracted']['text_sections'] if s['label']=='Security Effect')
    old=('Security Effect ' if cid in {'AD1-021','BT1-110'} else '')+text
    r=dict(source_manifest_digest='3'*64,old_input_revision='1'*64,new_input_revision='2'*64,runtime_field_diffs=[dict(card_id=cid,field_path=I,old_value=old,new_value=''),dict(card_id=cid,field_path=S,old_value='',new_value=text)],canonical_conflicts=[],corrections=[],compatibility_corrections=[])
    s=dict(manifest=dict(complete=True,failed_ids=[],expected_count=1),observations=[o])
    c=dict(cards={cid:dict(fields={I:'',S:text},provenance={},printings=[],faces=canonicalize(normalization._parse_faces(o['extracted']['faces'], observation=SourceObservation(**(o | {'source': SourceName.OFFICIAL})))))},source_manifest_digest='3'*64)
    c['cards'][cid]['fields']['effect_description_eng']=c['cards'][cid]['faces'][0]['effect_text']
    b=dict(report=r,snapshot=s,canonical=c,ledger=dict(schema_version=1,adjudications=[]),corrections=dict(schema_version=2,corrections=[],compatibility_corrections=[]),raw_pages={o['source_url']:raw})
    refresh(b);return b

def refresh(b):
    for o in b['snapshot']['observations']:
        o['raw_payload_digest']=hashlib.sha256(b['raw_pages'][o['source_url']]).hexdigest()
        o['observation_id']=content_digest({k:o[k] for k in ('raw_payload_digest','parser_version','source_schema_version','extracted')},exclude_operational=True)
    obs=b['snapshot']['observations'];m=b['snapshot']['manifest']
    m['observation_ids']=[o['observation_id'] for o in obs]
    m['observation_refs']=[dict(observation_id=o['observation_id'],gameplay_card_id=o['extracted']['card_id'],printing_id=o['extracted']['printing']['printing_id']) for o in obs]
    for d in b['report']['runtime_field_diffs']:
        d['old_digest']=content_digest(d['old_value']);d['new_digest']=content_digest(d['new_value'])
        d['provenance_observation_ids']=[o['observation_id'] for o in obs if o['extracted']['card_id']==d['card_id']]
        c=b['canonical']['cards'].get(d['card_id'])
        if c:
            c['provenance'][d['field_path']]=dict(observation_ids=d['provenance_observation_ids'],selected_observation_id=d['provenance_observation_ids'][0])
            c['printings']=[dict(printing_id=o['extracted']['printing']['printing_id'],observation_ids=[o['observation_id']]) for o in obs if o['extracted']['card_id']==d['card_id']]

def args(b,remaining=None):
    x={k+'_bytes':canonical_json_bytes(b[k]) for k in ('report','snapshot','canonical','ledger','corrections')}
    n=len(b['report']['runtime_field_diffs'])
    bind=dict(input_sha256={k:hashlib.sha256(v).hexdigest() for k,v in x.items()},runtime_count=n,remaining_runtime_count=n if remaining is None else remaining,remaining_text_count=n if remaining is None else remaining,official_card_count=b['snapshot']['manifest']['expected_count'])
    return x|dict(raw_pages=b['raw_pages'],binding_bytes=canonical_json_bytes(bind))

@pytest.mark.parametrize('cid',['AD1-021','BT1-110'])
def test_actual_exact_pair(cid):
    p=proof.prove(**args(bundle(cid)))
    assert p['status']=='PROOF_ONLY_NO_APPROVALS' and len(p['eligible'])==1
    assert len(p['eligible'][0]['diffs'])==2 and p['total_runtime_diffs']==2
    assert not hasattr(proof,'emit_reviewed') and 'adjudications' not in p

@pytest.mark.parametrize('cid',['AD1-019','AD1-022'])
def test_actual_no_header_is_separate(cid):
    assert not proof.prove(**args(bundle(cid)))['eligible']

@pytest.mark.parametrize('change',['wording','space','case','header','old_security','new_inherited','dual','digimon','missing_face','real_inherited','conflict','correction','gap','raw_type','raw_duplicate','raw_hidden','raw_zone','wrong_image','missing_pair','digest','missing_provenance'])
def test_boundary(change):
    b=bundle();r=b['report'];o=b['snapshot']['observations'][0];e=o['extracted'];raw=b['raw_pages'][o['source_url']]
    if change in {'wording','space','case','header'}:
        r['runtime_field_diffs'][0]['old_value']={'wording':'[Security] Delete this card.','space':'[Security]  Play this card without paying the cost.','case':'[security] Play this card without paying the cost.','header':'Security Effect '+r['runtime_field_diffs'][0]['old_value']}[change]
    elif change=='old_security':r['runtime_field_diffs'][1]['old_value']='existing'
    elif change=='new_inherited':r['runtime_field_diffs'][0]['new_value']='[Your Turn] Draw 1.'
    elif change=='dual':e['layout']='dual'
    elif change=='digimon':e['card_type']='Digimon'
    elif change=='missing_face':e['faces']=[]
    elif change=='real_inherited':e['text_sections'].append(dict(label='Inherited Effect',text='[Your Turn] Draw 1.'))
    elif change=='conflict':r['canonical_conflicts']=[dict(card_id=e['card_id'],approval_required=True,field_path='dp')]
    elif change=='correction':b['corrections']['corrections']=[dict(card_id=e['card_id'])]
    elif change=='gap':e['printing_field_gaps']=[dict(field='dp')]
    elif change=='raw_type':raw=raw.replace(b'Tamer',b'Digimon')
    elif change=='raw_duplicate':raw+=raw
    elif change=='raw_hidden':raw=raw.replace(b'[Security Effect]</dt>',b'[Security Effect]</dt><dd hidden>foreign</dd>')
    elif change=='raw_zone':raw=raw.replace(b'Security Effect',b'Inherited Effect')
    elif change=='wrong_image':raw=raw.replace(b'AD1-021.png',b'AD1-022.png')
    elif change=='missing_pair':r['runtime_field_diffs'].pop()
    if change.startswith('raw_') or change=='wrong_image': assert raw!=b['raw_pages'][o['source_url']]
    b['raw_pages'][o['source_url']]=raw;refresh(b)
    if change=='digest':r['runtime_field_diffs'][0]['old_digest']='f'*64
    if change=='missing_provenance':r['runtime_field_diffs'][0]['provenance_observation_ids']=[]
    if change in {'digest','missing_provenance','correction'}:
        with pytest.raises(proof.ProofError):proof.prove(**args(b))
    else:assert not proof.prove(**args(b))['eligible']

def approval(b,d):
    return dict(schema_version=1,adjudication_id='test-'+d['field_path'],kind='runtime_field_cutover',card_id=d['card_id'],field_path=d['field_path'],old_input_revision=b['report']['old_input_revision'],new_input_revision=b['report']['new_input_revision'],old_digest=d['old_digest'],new_digest=d['new_digest'],old_value=d['old_value'],new_value=d['new_value'],provenance_observation_ids=d['provenance_observation_ids'],reason='Independent exact test review',reviewer='test',reviewed_at='2026-10-12T00:00:00Z')

def test_atomic_existing_overlap():
    b=bundle();b['ledger']['adjudications']=[approval(b,d) for d in b['report']['runtime_field_diffs']]
    p=proof.prove(**args(b,remaining=0));assert p['eligible'][0]['overlap']=='complete_exact_pair'
    b['ledger']['adjudications'].pop()
    with pytest.raises(proof.ProofError,match='partial'):proof.prove(**args(b,remaining=1))

def test_stale_binding_and_duplicate_rejected():
    b=bundle();a=args(b);a['ledger_bytes']+=b' '
    with pytest.raises(proof.ProofError,match='binding'):proof.prove(**a)
    b['report']['runtime_field_diffs'].append(copy.deepcopy(b['report']['runtime_field_diffs'][0]))
    with pytest.raises(proof.ProofError,match='duplicate'):proof.prove(**args(b))

def test_deterministic_dependencies():
    a=args(bundle());p=proof.prove(**a);assert p==proof.prove(**a)
    assert {'form_restoration_proof.py','normalization.py','sources/official.py'}<=p['dependency_sha256'].keys()
@pytest.mark.parametrize('change',['target','negation','duration','timing','newline','punctuation'])
def test_exact_sentence_change_never_equivalent(change):
    b=bundle();d=b['report']['runtime_field_diffs'][1]
    d['new_value'] += {'target':' another card','negation':' not','duration':' until end of turn','timing':' [Main]','newline':'\n','punctuation':'!'}[change]
    refresh(b);assert not proof.prove(**args(b))['eligible']

@pytest.mark.parametrize('change',['raw_missing','raw_changed','wrong_observation_id','missing_ref','counts','canonical_value','canonical_printings','ledger_value','duplicate_ledger'])
def test_pinned_and_complete_input_fail_closed(change):
    b=bundle();o=b['snapshot']['observations'][0]
    if change=='raw_missing':b['raw_pages'].clear()
    elif change=='raw_changed':b['raw_pages'][o['source_url']]+=b'changed'
    elif change=='wrong_observation_id':o['observation_id']='f'*64
    elif change=='missing_ref':b['snapshot']['manifest']['observation_refs'].clear()
    elif change=='canonical_value':b['canonical']['cards']['AD1-021']['fields'][S]='changed'
    elif change=='canonical_printings':b['canonical']['cards']['AD1-021']['printings']=[]
    elif change in {'ledger_value','duplicate_ledger'}:
        row=approval(b,b['report']['runtime_field_diffs'][0]);b['ledger']['adjudications']=[row]
        if change=='ledger_value':row['new_value']='changed'
        else:b['ledger']['adjudications'].append(copy.deepcopy(row))
    a=args(b)
    if change=='counts':
        bind=json.loads(a['binding_bytes']);bind['runtime_count']=0;a['binding_bytes']=canonical_json_bytes(bind)
    if change in {'canonical_value','canonical_printings'}:assert not proof.prove(**a)['eligible']
    else:
        with pytest.raises((ValueError, AdjudicationError)):proof.prove(**a)

def test_dissenting_alternate_and_omitted_printing_provenance():
    b=bundle();o=copy.deepcopy(b['snapshot']['observations'][0]);o['extracted']['printing']['printing_id']+='_P1'
    o['extracted']['text_sections'][-1]['text']='[Security] Draw 1.'
    b['snapshot']['observations'].append(o);refresh(b)
    assert not proof.prove(**args(b))['eligible']
    b['report']['runtime_field_diffs'][0]['provenance_observation_ids'].pop()
    with pytest.raises(proof.ProofError):proof.prove(**args(b))

def test_real_fixture_roundtrip_sections_and_projection():
    from tools.card_data.sources import official
    from tools.card_data import normalization
    for cid in ['AD1-019','AD1-022','AD1-021','BT1-110']:
        b=bundle(cid);o=b['snapshot']['observations'][0]
        assert official._text_sections(b['raw_pages'][o['source_url']].decode('utf-8'))==o['extracted']['text_sections']
        fields=normalization._normalize_candidate_fields(normalization._official_fields(o['extracted']))
        assert fields[I]=='' and fields[S]==b['report']['runtime_field_diffs'][1]['new_value']

@pytest.mark.parametrize('change',['canonical_dual','hidden_parent'])
def test_layout_and_hidden_parent_are_not_ordinary_security_boxes(change):
    b=bundle();o=b['snapshot']['observations'][0]
    if change=='canonical_dual':b['canonical']['cards']['AD1-021']['faces'].append(copy.deepcopy(b['canonical']['cards']['AD1-021']['faces'][0]))
    else:
        raw=b['raw_pages'][o['source_url']]
        changed=raw.replace(b'<div class="popupCol"',b'<div hidden class="popupCol"')
        assert changed!=raw;b['raw_pages'][o['source_url']]=changed;refresh(b)
    assert not proof.prove(**args(b))['eligible']

def test_cli_proof_only_and_no_overwrite(tmp_path):
    b=bundle();a=args(b)
    argv=[]
    for name in ['report','snapshot','canonical','ledger','corrections','binding']:
        p=tmp_path/(name+'.json');p.write_bytes(a[name+'_bytes']);argv += ['--'+name,str(p)]
    url,raw=next(iter(b['raw_pages'].items()));(tmp_path/'raw.html').write_bytes(raw)
    (tmp_path/'index.json').write_text(json.dumps([dict(url=url,file='raw.html')]))
    out=tmp_path/'proof.json';argv += ['--cache-index',str(tmp_path/'index.json'),'--out',str(out)]
    assert proof.main(argv)==0
    before=out.read_bytes()
    with pytest.raises(proof.ProofError,match='overwrite'):proof.main(argv)
    assert out.read_bytes()==before
    with pytest.raises(SystemExit):proof.main(['emit-reviewed',*argv])

def test_whole_runtime_inventory_preserves_outside_family_fields():
    b=bundle();b['report']['runtime_field_diffs'].append(dict(card_id='AD1-021',field_path='effect_description_eng',old_value='old main',new_value='new main'))
    refresh(b);p=proof.prove(**args(b))
    assert p['total_runtime_diffs']==p['remaining_runtime_diffs']==p['remaining_text_diffs']==3
    assert len(p['runtime_inventory'])==3
    assert [r['classification'] for r in p['runtime_inventory']].count('outside_family')==1
    assert len(p['eligible'])==1 and len(p['eligible'][0]['diffs'])==2

@pytest.mark.parametrize('change',['face_id','inherited_text','security_text','effect_text','field_main'])
def test_normalized_canonical_face_source_consistency(change):
    b=bundle();c=b['canonical']['cards']['AD1-021'];f=c['faces'][0]
    if change=='face_id':f['face_id']='foreign'
    elif change=='inherited_text':f['inherited_text']='[Your Turn] Draw 1.'
    elif change=='security_text':f['security_text']='[Security] Delete 1 Digimon.'
    elif change=='effect_text':f['effect_text']='[On Play] Gain 10 memory.'
    else:c['fields']['effect_description_eng']='[On Play] Gain 10 memory.'
    assert not proof.prove(**args(b))['eligible']

@pytest.mark.parametrize('change',['mismatch','missing_report','report_bool','report_bad_hex','canonical_bool','same_bad_hex'])
def test_report_and_canonical_aggregate_must_match_even_when_individually_pinned(change):
    b=bundle()
    if change=='mismatch':b['report']['source_manifest_digest']='4'*64
    elif change=='missing_report':b['report'].pop('source_manifest_digest')
    elif change=='report_bool':b['report']['source_manifest_digest']=True
    elif change=='report_bad_hex':b['report']['source_manifest_digest']='Z'*64
    elif change=='canonical_bool':b['canonical']['source_manifest_digest']=True
    else:b['canonical']['source_manifest_digest']=b['report']['source_manifest_digest']='Z'*64
    with pytest.raises(proof.ProofError,match='aggregate'):
        proof.prove(**args(b))

@pytest.mark.parametrize('prefix',['security effect ','SECURITY EFFECT ','Security effect ','Security Effect\u00a0','Security Effect  ','Security Effect\t','Security Effect\n',' Security Effect ','Security Effect Security Effect ','Security Effect','[Security] Security Effect '])
def test_only_one_literal_ascii_prefix(prefix):
    b=bundle();body=b['report']['runtime_field_diffs'][1]['new_value']
    b['report']['runtime_field_diffs'][0]['old_value']=prefix+body;refresh(b)
    assert not proof.prove(**args(b))['eligible']

@pytest.mark.parametrize('change',['blocker_condition','target_count','cost','negation','timing'])
def test_actual_option_body_is_never_rewritten(change):
    b=bundle('BT1-110');d=b['report']['runtime_field_diffs'][1]
    d['new_value']=d['new_value'].replace('without','with') if change=='blocker_condition' else d['new_value']+{'target_count':' 1','cost':' cost 0','negation':' not','timing':' [Main]'}[change]
    refresh(b);assert not proof.prove(**args(b))['eligible']

def test_internal_label_words_are_not_globally_removed():
    body='[Security] The phrase Security Effect remains in this test body.'
    inherited=dict(old_value='Security Effect '+body,new_value='')
    security=dict(old_value='',new_value=body)
    assert proof._pair_equation(inherited,security)
    security['new_value']=body.replace('Security Effect ','')
    assert not proof._pair_equation(inherited,security)

@pytest.mark.parametrize('change',['faq_container','missing_printed_info_column'])
def test_security_header_in_faq_or_unknown_container_is_not_printed_source(change):
    import re
    b=bundle();o=b['snapshot']['observations'][0];url=o['source_url'];raw=b['raw_pages'][url]
    if change=='faq_container':
        altered=re.sub(rb'(<dl class="cardInfoBoxSmall">\s*<dt class="cardInfoTitSmall">\[Security Effect\]</dt>.*?</dl>)',rb'<div class="cardFaqList">\1</div>',raw,flags=re.S)
    else:altered=raw.replace(b'class="cardInfoCol"',b'class="unrecognizedInfoCol"')
    assert altered!=raw;b['raw_pages'][url]=altered;refresh(b)
    assert not proof.prove(**args(b))['eligible']

@pytest.mark.parametrize('markup', ['<span hidden>[Security Effect]</span>', '<span style="display:none">[Security Effect]</span>', '<span>[Security Effect]</span>'])
def test_security_label_must_be_literal_text_without_child_elements(markup):
    b=bundle();o=b['snapshot']['observations'][0];url=o['source_url'];raw=b['raw_pages'][url]
    altered=raw.replace(b'[Security Effect]</dt>', markup.encode()+b'</dt>')
    assert altered!=raw;b['raw_pages'][url]=altered;refresh(b)
    assert not proof.prove(**args(b))['eligible']


def test_unwrapped_literal_security_label_remains_supported():
    assert proof.prove(**args(bundle()))['eligible']

@pytest.mark.parametrize('change',['foreign_selected','missing_selected','duplicate_provenance','boolean_ledger','unknown_ledger_field','unknown_correction_schema','boolean_correction_schema','unknown_correction_field','malformed_correction_row'])
def test_r1_complete_envelopes_and_selected_provenance(change):
    b=bundle();p=b['canonical']['cards']['AD1-021']['provenance'][S]
    if change=='foreign_selected':p['selected_observation_id']='f'*64
    elif change=='missing_selected':p.pop('selected_observation_id')
    elif change=='duplicate_provenance':p['observation_ids']*=2
    elif change=='boolean_ledger':b['ledger']['schema_version']=True
    elif change=='unknown_ledger_field':b['ledger']['future_rows']=[]
    elif change=='unknown_correction_schema':b['corrections']['schema_version']=999
    elif change=='boolean_correction_schema':b['corrections']['schema_version']=True
    elif change=='unknown_correction_field':b['corrections']['future_rows']=[]
    else:b['corrections']['corrections']=[{'card_id':'AD1-021'}]
    with pytest.raises(proof.ProofError):proof.prove(**args(b))


def test_r1_dependency_output_pins_complete_added_loader_closure():
    from tools.card_data import compatibility_corrections, compatibility_selections, corrections, errors
    output=proof.prove(**args(bundle()))['dependency_sha256']
    for module in (compatibility_corrections,compatibility_selections,corrections,errors):
        assert output[Path(module.__file__).name]==hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
