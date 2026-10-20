"""No live approvals: synthetic exact-bound Form family proof adversaries."""
import copy
import hashlib
import json
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes, content_digest
from tools.card_data.form_restoration_proof import ProofError, prove, emit_reviewed

URL="https://world.digimoncard.com/cards/?search=true&category=522001"
FIXTURE=Path(__file__).parents[1]/"fixtures/card_data/form_restoration/single.html"

def fixture(form="Rookie"):
    raw=FIXTURE.read_bytes().replace(b'Rookie',form.encode())
    extracted=dict(card_id='BT1-001',card_type='Digimon',form=form,layout='single',level=3,
        printing=dict(printing_id='BT1-001',image_url='https://world.digimoncard.com/images/cardlist/card/BT1-001.png'),
        rule_grants=dict(traits=[],attributes=[],forms=[]),text_sections=[],printing_field_gaps=[])
    obs=dict(schema_version=1,observation_id='',source='official',source_record_key='522001:BT1-001:BT1-001',
        source_url=URL,raw_payload_digest=hashlib.sha256(raw).hexdigest(),parser_version='official-html-v4',
        source_schema_version='world-card-list-html-v1',first_acquired_at='2026-10-12T00:00:00Z',extracted=extracted)
    snapshot=dict(manifest=dict(complete=True,expected_count=1,failed_ids=[],observation_ids=[],observation_refs=[]),observations=[obs])
    diff=dict(card_id='BT1-001',field_path='form_eng',old_value=[],new_value=[form],old_digest=content_digest([]),new_digest=content_digest([form]),provenance_observation_ids=[])
    report=dict(old_input_revision='1'*64,new_input_revision='2'*64,runtime_field_diffs=[diff],canonical_conflicts=[],corrections=[])
    bundle=[report,snapshot,{URL:raw}]; reidentify(bundle);return bundle

def reidentify(b):
    report,snapshot,pages=b
    for o in snapshot['observations']:
        o['observation_id']=content_digest({k:o[k] for k in ['raw_payload_digest','parser_version','source_schema_version','extracted']},exclude_operational=True)
    ids=[o['observation_id'] for o in snapshot['observations']]
    snapshot['manifest']['observation_ids']=ids
    snapshot['manifest']['observation_refs']=[dict(observation_id=o['observation_id'],gameplay_card_id=o['extracted']['card_id'],printing_id=o['extracted']['printing']['printing_id']) for o in snapshot['observations']]
    for d in report['runtime_field_diffs']:
        d['provenance_observation_ids']=[o['observation_id'] for o in snapshot['observations'] if o['extracted']['card_id']==d['card_id']]

def args(b):return dict(report_bytes=canonical_json_bytes(b[0]),snapshot_bytes=canonical_json_bytes(b[1]),raw_pages=b[2])
def review(proof):
    return dict(status='APPROVED',proof_sha256=content_digest(proof),predicate_sha256=proof['predicate_sha256'],
        old_input_revision=proof['old_input_revision'],new_input_revision=proof['new_input_revision'],
        membership_sha256=proof['membership_sha256'],count=len(proof['eligible']),reviewer='synthetic independent reviewer',reviewed_at='2026-10-12T00:00:00Z')

@pytest.mark.parametrize('form',['Rookie','Champion','Ultimate','Mega'])
def test_positive_exact_labels_deterministic_and_no_approvals(form):
    b=fixture(form);p=prove(**args(b));assert p==prove(**args(b))
    assert len(p['eligible'])==1 and p['total_form_diffs']==1 and not p['excluded']
    assert p['status']=='PROOF_ONLY_NO_APPROVALS' and 'adjudications' not in p
    assert p['eligible'][0]['diff']['new_value']==[form]

@pytest.mark.parametrize('change',['null','nonempty','multiple','hybrid','rule','special_rule','dual','correction','conflict','blank_type','japanese_type','known_gap','raw_duplicate','raw_two_forms','raw_wrong_card','raw_wrong_image','raw_hidden','raw_rule'])
def test_conservative_exclusions(change):
    b=fixture();d=b[0]['runtime_field_diffs'][0];o=b[1]['observations'][0];e=o['extracted']
    if change=='null':d['old_value']=None
    elif change=='nonempty':d['old_value']=['Champion']
    elif change=='multiple':d['new_value']=['Rookie','Appmon']
    elif change=='hybrid':d['new_value']=['Hybrid']
    elif change=='rule':e['rule_grants']['forms']=['Rookie']
    elif change=='special_rule':e['text_sections']=[dict(label='Special Rule',text='Overflow -3')]
    elif change=='dual':e['layout']='dual'
    elif change=='correction':b[0]['corrections']=[dict(card_id='BT1-001',field_path='dp')]
    elif change=='conflict':b[0]['canonical_conflicts']=[dict(card_id='BT1-001',field_path='form_eng',approval_required=True)]
    elif change=='blank_type':e['card_type']=''
    elif change=='japanese_type':e['card_type']='Japanese'
    elif change=='known_gap':
        e['card_id']='BT10-111';d['card_id']='BT10-111';e['printing']['printing_id']='BT10-111'
    elif change.startswith('raw_'):
        raw=b[2][URL]
        if change=='raw_duplicate':raw+=raw
        elif change=='raw_two_forms':raw=raw.replace(b'</div></div>',b'<dt class="cardInfoTit">Form</dt><dd>Rookie</dd></div></div>')
        elif change=='raw_wrong_card':raw=raw.replace(b'>BT1-001<',b'>BT1-002<')
        elif change=='raw_wrong_image':raw=raw.replace(b'BT1-001.png',b'BT1-002.png')
        elif change=='raw_hidden':raw=raw.replace(b'>Rookie<',b'><span hidden>Rookie</span><')
        elif change=='raw_rule':raw=raw.replace(b'</div></div>',b'<p>[Rule] Trait: Has [Hybrid] Form.</p></div></div>')
        b[2][URL]=raw;o['raw_payload_digest']=hashlib.sha256(raw).hexdigest()
    d['old_digest']=content_digest(d['old_value']);d['new_digest']=content_digest(d['new_value']);reidentify(b)
    p=prove(**args(b));assert not p['eligible'] and len(p['excluded'])==1

@pytest.mark.parametrize('change',['missing_raw','corrupt_raw','forged_identity','secondary','missing_ref','wrong_ref','incomplete','duplicate_diff','bad_digest','bad_revision'])
def test_integrity_failures_refuse(change):
    b=fixture();o=b[1]['observations'][0]
    if change=='missing_raw':b[2].clear()
    elif change=='corrupt_raw':b[2][URL]+=b'corrupt'
    elif change=='forged_identity':o['observation_id']='a'*64
    elif change=='secondary':o['source']='digimoncard_io'
    elif change=='missing_ref':b[1]['manifest']['observation_refs']=[]
    elif change=='wrong_ref':b[1]['manifest']['observation_refs'][0]['printing_id']='wrong'
    elif change=='incomplete':b[1]['manifest']['complete']=False
    elif change=='duplicate_diff':b[0]['runtime_field_diffs']*=2
    elif change=='bad_digest':b[0]['runtime_field_diffs'][0]['new_digest']='f'*64
    elif change=='bad_revision':b[0]['new_input_revision']='bad'
    with pytest.raises(ProofError):prove(**args(b))

def test_dissenting_alternate_cannot_be_filtered_from_proof():
    b=fixture();o=copy.deepcopy(b[1]['observations'][0]);o['extracted']['form']='Champion';o['extracted']['printing']['printing_id']='BT1-001_P1';b[1]['observations'].append(o);reidentify(b)
    assert not prove(**args(b))['eligible']

@pytest.mark.parametrize('change',['missing','proof','code','old_revision','new_revision','membership','count','reviewer','timestamp','status'])
def test_emission_requires_exact_independent_review(change):
    b=fixture();p=prove(**args(b));v=review(p)
    if change=='missing':v=None
    else:
        key={'proof':'proof_sha256','code':'predicate_sha256','old_revision':'old_input_revision','new_revision':'new_input_revision','membership':'membership_sha256','count':'count','reviewer':'reviewer','timestamp':'reviewed_at','status':'status'}[change]
        v[key]=0 if key=='count' else ''
    with pytest.raises(ProofError):emit_reviewed(**args(b),review=v)

def test_reviewed_emission_uses_existing_schema_without_other_approvals():
    from tools.card_data.models import MigrationAdjudication,AdjudicationKind
    b=fixture();p=prove(**args(b));out=emit_reviewed(**args(b),review=review(p));assert len(out)==1
    item=MigrationAdjudication(**{**out[0],'kind':AdjudicationKind(out[0]['kind'])})
    assert item.field_path=='form_eng' and item.old_value==[] and item.new_value==['Rookie']
    assert item.is_current(old_input_revision='1'*64,new_input_revision='2'*64,old_digest=content_digest([]),new_digest=content_digest(['Rookie']))
    assert b[0]['runtime_field_diffs'][0].get('approved') is None
    with pytest.raises(ProofError):emit_reviewed(**args(b),review=review(p),existing_adjudications=out)

@pytest.mark.parametrize('markup',[b'<dd hidden>Rookie</dd>',b'<dd style="display:none">Rookie</dd>'])
def test_hidden_form_cell_is_excluded(markup):
    b=fixture();raw=b[2][URL].replace(b'<dd class="cardInfoData">Rookie</dd>',markup)
    b[2][URL]=raw;b[1]['observations'][0]['raw_payload_digest']=hashlib.sha256(raw).hexdigest();reidentify(b)
    assert not prove(**args(b))['eligible']

@pytest.mark.parametrize('rule',[b'<img alt="[Rule]" src="rule.png">','[Rule]'.replace('[','［').replace(']','］').encode()])
def test_raw_rule_representation_is_never_hidden_by_parser(rule):
    b=fixture();raw=b[2][URL].replace(b'</div></div>',rule+b'</div></div>');b[2][URL]=raw
    b[1]['observations'][0]['raw_payload_digest']=hashlib.sha256(raw).hexdigest();reidentify(b)
    assert not prove(**args(b))['eligible']

def test_unanimous_alternate_requires_complete_evidence():
    b=fixture();o=copy.deepcopy(b[1]['observations'][0]);raw=b[2][URL].replace(b'BT1-001',b'BT1-001_P1').replace(b'>BT1-001_P1<',b'>BT1-001<')
    url=URL+'&alternate=1';o['source_url']=url;o['raw_payload_digest']=hashlib.sha256(raw).hexdigest();o['source_record_key']+=':alt';o['extracted']['printing']['printing_id']='BT1-001_P1';o['extracted']['printing']['image_url']=o['extracted']['printing']['image_url'].replace('BT1-001.png','BT1-001_P1.png');b[2][url]=raw;b[1]['observations'].append(o);reidentify(b)
    p=prove(**args(b));assert len(p['eligible'])==1 and len(p['eligible'][0]['official_evidence'])==2
    b[0]['runtime_field_diffs'][0]['provenance_observation_ids'].pop()
    with pytest.raises(ProofError):prove(**args(b))

def test_changed_inputs_invalidate_review_even_when_form_unchanged():
    b=fixture();p=prove(**args(b));v=review(p);b[0]['new_input_revision']='3'*64
    with pytest.raises(ProofError):emit_reviewed(**args(b),review=v)

def test_existing_validator_accepts_only_emitted_form_field():
    from tools.card_data.compatibility_diff import RuntimeFieldDiff,approve_runtime_diffs
    from tools.card_data.models import SourceObservation,SourceName,MigrationAdjudication,AdjudicationKind
    b=fixture();p=prove(**args(b));out=emit_reviewed(**args(b),review=review(p));a=MigrationAdjudication(**{**out[0],'kind':AdjudicationKind(out[0]['kind'])})
    o=b[1]['observations'][0];obs=SourceObservation(**{**o,'source':SourceName.OFFICIAL})
    d=RuntimeFieldDiff(**b[0]['runtime_field_diffs'][0]);other=RuntimeFieldDiff(card_id=d.card_id,field_path='dp',old_value=1,new_value=2,old_digest=content_digest(1),new_digest=content_digest(2),provenance_observation_ids=d.provenance_observation_ids)
    result=approve_runtime_diffs([d,other],corrections=[],adjudications=[a],observations={obs.observation_id:obs},old_input_revision='1'*64,new_input_revision='2'*64)
    assert result[0].approved and not result[1].approved

def test_cli_invalid_review_writes_no_output(tmp_path):
    from tools.card_data.form_restoration_proof import main
    b=fixture()
    for name,value in [('report',b[0]),('snapshot',b[1]),('review',{})]: (tmp_path/(name+'.json')).write_bytes(canonical_json_bytes(value))
    (tmp_path/'page.html').write_bytes(b[2][URL]);(tmp_path/'index.json').write_text(json.dumps([dict(url=URL,file='page.html')]))
    argv=['emit-reviewed','--report',str(tmp_path/'report.json'),'--snapshot',str(tmp_path/'snapshot.json'),'--cache-index',str(tmp_path/'index.json'),'--review',str(tmp_path/'review.json'),'--out',str(tmp_path/'output.json')]
    with pytest.raises(ProofError):main(argv)
    assert not (tmp_path/'output.json').exists()

@pytest.mark.parametrize('text',['＜Rule＞ Name: Also treated as [Shoutmon]/[Ballistamon].','<Rule> Name: Also treated as [Shoutmon].','［Rule］ Name: Also treated as [Shoutmon].'])
def test_extracted_rule_name_syntax_excludes_family(text):
    b=fixture();b[1]['observations'][0]['extracted']['text_sections']=[dict(label='Effect',text=text)];reidentify(b)
    assert not prove(**args(b))['eligible']


def test_cli_emission_is_existing_ledger_schema_and_refuses_duplicate(tmp_path):
    from tools.card_data.form_restoration_proof import main
    from tools.card_data.adjudications import load_migration_adjudications
    b=fixture();p=prove(**args(b))
    for name,value in [('report',b[0]),('snapshot',b[1]),('review',review(p)),('existing',dict(schema_version=1,adjudications=[]))]:
        (tmp_path/(name+'.json')).write_bytes(canonical_json_bytes(value))
    (tmp_path/'page.html').write_bytes(b[2][URL]);(tmp_path/'index.json').write_text(json.dumps([dict(url=URL,file='page.html')]))
    argv=['emit-reviewed','--report',str(tmp_path/'report.json'),'--snapshot',str(tmp_path/'snapshot.json'),'--cache-index',str(tmp_path/'index.json'),'--review',str(tmp_path/'review.json'),'--existing-adjudications',str(tmp_path/'existing.json'),'--out',str(tmp_path/'output.json')]
    assert main(argv)==0
    assert len(load_migration_adjudications(tmp_path/'output.json'))==1
    argv[argv.index('--existing-adjudications')+1]=str(tmp_path/'output.json');argv[-1]=str(tmp_path/'duplicate.json')
    with pytest.raises(ProofError,match='duplicated'):main(argv)
    assert not (tmp_path/'duplicate.json').exists()
