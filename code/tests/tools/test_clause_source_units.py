"""A3a producer replay fixtures, not production source approvals."""
import copy
import gzip
import json
from pathlib import Path

import pytest

from tools.card_data import adjudications, normalization, serialization
from tools.card_data.models import SourceObservation, SourceName, AdmissionStatus

F = Path(__file__).parents[1] / 'fixtures/card_data/source_units'
BASE = F.parent / 'source_binding'


def inputs(cid='BT1-010'):
    d = json.loads((F / 'inputs.json').read_text(encoding='utf-8'))
    observations = {oid:o for oid,o in d['observations'].items() if o['extracted']['card_id']==cid}
    candidates = [normalization.observation_candidate(SourceObservation(**{**o,'source':SourceName(o['source'])})) for o in observations.values()]
    ledger = tuple(adjudications._row(r,path=Path('<reviewed-fixture>')) for r in d['reviewed_ledger']['adjudications'])
    card = copy.deepcopy(d['cards'][cid])
    # Replay exact producer over complete membership and actual reviewed rows.
    # Original candidate (pre-adoption) provenance remains archived in inputs.json.
    for field in ('card_kind','effect_description_eng','inherited_effect_description_eng','security_effect_description_eng'):
        value,provenance,_ = normalization._select_field(cid,field,[c for c in candidates if field in c.fields],admission=AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=d['revision'])
        card['fields'][field]=value;card['provenance'][field]=serialization.canonicalize(provenance)
    runtime={'card_id':cid,**{f:card['fields'][f] for f in ('card_kind','effect_description_eng','inherited_effect_description_eng','security_effect_description_eng')}}
    enc=serialization.canonical_json_bytes
    return dict(aggregate=gzip.decompress((BASE/'canonical_selection/aggregate.json.gz').read_bytes()),scopes={s:gzip.decompress((F/(s+'-scope.json.gz')).read_bytes()) for s in ('official','digimoncard_io')},observations={oid:enc(o)for oid,o in observations.items()},canonical_card=enc(card),runtime_projection=enc(runtime),adjudication_ledger=enc(d['reviewed_ledger']),raw_pages={u:gzip.decompress((F/n).read_bytes())for u,n in d['raw_files'].items()},correction_document=(BASE/'conditions/card_corrections.json').read_bytes(),condition_selections=(BASE/'conditions/card_compatibility_selections.json').read_bytes(),old_input_revision=d['revision'],new_input_revision=d['revision'],expected_official_universe=d['expected_official_universe'])


def compose(**args):
    from tools.clause_coverage.source_units import compose_ordinary_card
    return compose_ordinary_card(**args)


@pytest.mark.parametrize('cid',['BT1-010','BT1-045','BT4-104','BT26-104'])
def test_real_four_card_complete_ordinary_composition(cid):
    args=inputs(cid);before=copy.deepcopy(args);r=compose(**args)
    assert r['status']=='COMPOSED_ORDINARY' and not r['blockers']
    assert args==before
    card=json.loads(args['canonical_card'])
    assert set(r['zones'])=={'effect','inherited','security'}
    for zone,row in r['zones'].items():
        assert row['value']==card['fields'][row['field']]
        assert row['source_binding']['selected_observation_id'] in args['observations']
    assert 'clauses' not in r and 'ready' not in r


def test_zero_clause_card_keeps_explicit_empty_zones():
    r=compose(**inputs('BT1-045'))
    assert all(z['value']=='' and not z['spans'] for z in r['zones'].values())
    assert all(z['presence'] in ('absent-box','printed-empty-box') for z in r['zones'].values())
    assert r['source_evidence'] and all(not o['text_sections']for o in r['source_evidence'])


@pytest.mark.parametrize('cid',['BT4-104','BT26-104'])
def test_actual_canonical_choices_have_real_approval_reference(cid):
    r=compose(**inputs(cid))
    assert r['zones']['effect']['source_binding']['adjudication_id']
    if cid=='BT26-104':
        assert r['zones']['security']['source_binding']['adjudication_id']
        assert [s['timings']for s in r['zones']['effect']['spans']]==[
            ['Start of Your Main Phase'],['On Play'],['End of Your Turn']]


@pytest.mark.parametrize('mutation',['missing_obs','foreign_obs','raw','aggregate','scope','universe','runtime_text','runtime_kind','card_kind','provenance','value','missing_zone','bool_schema','stale_revision','duplicate_json'])
def test_authority_boundary_refusals(mutation):
    from tools.clause_coverage.source_units import SourceUnitError
    a=inputs();c=json.loads(a['canonical_card']);r=json.loads(a['runtime_projection'])
    if mutation=='missing_obs':a['observations'].pop(next(iter(a['observations'])))
    elif mutation=='foreign_obs':a['observations']['0'*64]=next(iter(a['observations'].values()))
    elif mutation=='raw':a['raw_pages']={k:v+b'changed'for k,v in a['raw_pages'].items()}
    elif mutation=='aggregate':a['aggregate']+=b'bad'
    elif mutation=='scope':
        s=json.loads(a['scopes']['official']);s['scope']='partial:unreviewed';a['scopes']['official']=serialization.canonical_json_bytes(s)
    elif mutation=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
    elif mutation=='runtime_text':r['effect_description_eng']='invented'
    elif mutation=='runtime_kind':r['card_kind']='Option'
    elif mutation=='card_kind':c['fields']['card_kind']='Digi-Egg';r['card_kind']='Digi-Egg'
    elif mutation=='provenance':c['provenance']['effect_description_eng']['selected_observation_id']='0'*64
    elif mutation=='value':c['fields']['effect_description_eng']='invented';r['effect_description_eng']='invented'
    elif mutation=='missing_zone':c['fields'].pop('inherited_effect_description_eng')
    elif mutation=='bool_schema':c['schema_version']=True
    elif mutation=='stale_revision':a['old_input_revision']='0'*64
    elif mutation=='duplicate_json':a['aggregate']=b'{"schema_version":1,"schema_version":1}'
    a['canonical_card']=serialization.canonical_json_bytes(c);a['runtime_projection']=serialization.canonical_json_bytes(r)
    with pytest.raises(SourceUnitError):compose(**a)


@pytest.mark.parametrize('mutation',['missing','stale','foreign','duplicate'])
def test_canonical_choice_cannot_be_invented(mutation):
    from tools.clause_coverage.source_units import SourceUnitError
    a=inputs('BT4-104');d=json.loads(a['adjudication_ledger']);row=next(r for r in d['adjudications']if r['card_id']=='BT4-104' and r['new_input_revision']==a['new_input_revision'])
    if mutation=='missing':d['adjudications'].remove(row)
    elif mutation=='stale':row['new_input_revision']='0'*64
    elif mutation=='foreign':row['card_id']='BT1-010'
    else:d['adjudications'].append(copy.deepcopy(row))
    a['adjudication_ledger']=serialization.canonical_json_bytes(d)
    with pytest.raises(SourceUnitError):compose(**a)


def test_output_metadata_never_validates_itself():
    from tools.clause_coverage.source_units import validate_ordinary_card,SourceUnitError
    a=inputs();r=compose(**a);r['zones']['effect']['spans']=[]
    with pytest.raises(SourceUnitError):validate_ordinary_card(r,**a)


@pytest.mark.parametrize('field,value',[('xros_req','[Digivolve] Cost 3'),('dual',{'option':{}}),('link_effect_description_eng','Draw 1.')])
def test_structural_projection_has_inventory_and_blocker_not_partial_zones(field,value):
    a=inputs();runtime=json.loads(a['runtime_projection']);runtime[field]=value
    a['runtime_projection']=serialization.canonical_json_bytes(runtime)
    r=compose(**a)
    assert r['status']=='BLOCKED' and r['blockers'] and r['source_evidence']
    assert r['units']==[] and r['zones']=={}


def test_wrong_label_alternates_remain_visible_after_exact_approved_choice():
    r=compose(**inputs('BT26-104'))
    alternate=[u for u in r['inventory']if u['label']=='Link Condition']
    assert len(alternate)==2
    assert all(u['selection_role']=='retained-alternative'for u in alternate)
    assert r['retained_alternative_structures']
    assert all(u['label']!='Link Condition'for u in r['units'])


def test_selected_unknown_link_cannot_use_alternate_review_exception():
    from tools.clause_coverage.source_units import _partition_structure_blockers
    r=compose(**inputs('BT26-104'));b=r['retained_alternative_structures'][0]
    z=r['zones']['security']
    active,alternatives=_partition_structure_blockers([b],{b['observation_id']},z['source_binding'],z['value'])
    assert active==[b] and alternatives==[]


@pytest.mark.parametrize('code',['UNSUPPORTED_LAYOUT','UNSUPPORTED_RULE_GRANTS','UNSUPPORTED_RULE_OR_LINK'])
def test_unselected_rule_and_dual_remain_conservatively_blocked(code):
    from tools.clause_coverage.source_units import _partition_structure_blockers
    b={'code':code,'observation_id':'unselected'}
    assert _partition_structure_blockers([b],set(),{},'')==([b],[])


def test_section_trace_preserves_multiplicity_empty_body_and_empty_boxes():
    from tools.clause_coverage.source_units import _ordinary_sections
    sections=[{'label':'Effect','text':'[On Play]'}, {'label':'Effect','text':'[On Play]'},
              {'label':'Inherited Effect','text':''}]
    assert _ordinary_sections(sections,'Digimon')==[
        (0,'effect_description_eng','[On Play]','producer-render'),
        (1,'effect_description_eng','[On Play]','producer-render'),
        (2,'inherited_effect_description_eng','','producer-render')]


def test_egg_section_trace_uses_explicit_kind_without_changing_original_text():
    from tools.clause_coverage.source_units import _ordinary_sections
    s=[{'label':'Effect','text':'[Your Turn] Original effect.'}];before=copy.deepcopy(s)
    assert _ordinary_sections(s,'Digi-Egg')==[(0,'inherited_effect_description_eng',s[0]['text'],'producer-egg-zone')]
    assert s==before


def test_option_embedded_security_uses_producer_routing_without_header_invention():
    from tools.clause_coverage.source_units import _ordinary_sections
    s=[{'label':'Effect','text':'[Main] Draw 1.\n[Security] Activate this card\'s [Main] effect.'}]
    rows=_ordinary_sections(s,'Option')
    assert rows==[(0,'security_effect_description_eng',"[Security] Activate this card's [Main] effect.",'producer-embedded-security'),
                  (0,'effect_description_eng','[Main] Draw 1.','producer-render')]


def test_noninheritable_security_relocation_outside_initial_scope_refused():
    from tools.clause_coverage.source_units import _ordinary_sections,SourceUnitError
    with pytest.raises(SourceUnitError,match='relocation'):
        _ordinary_sections([{'label':'Inherited Effect','text':'[Security] Draw 1.'}],'Option')


@pytest.mark.parametrize('field,value',[('kind','option'),('effect_text','Invented face text.')])
def test_canonical_primary_face_must_match_producer(field,value):
    from tools.clause_coverage.source_units import SourceUnitError
    a=inputs();c=json.loads(a['canonical_card']);c['faces'][0][field]=value
    a['canonical_card']=serialization.canonical_json_bytes(c)
    with pytest.raises(SourceUnitError):compose(**a)


@pytest.mark.parametrize('marker',['＜Link＞','<Link>','<link>','＜link＞','< Link >'])
def test_all_link_marker_spellings_refuse_ordinary_composition(marker):
    from tools.clause_coverage.source_units import _unsupported_ordinary_text
    assert _unsupported_ordinary_text(marker+' [Appmon] trait: Cost 1\n[Your Turn] Draw 1.')


def test_ordinary_text_without_structural_markers_is_not_rejected():
    from tools.clause_coverage.source_units import _unsupported_ordinary_text
    assert not _unsupported_ordinary_text('[On Play] Reveal 5 cards.')


def test_official_raw_page_url_must_belong_to_pinned_scope():
    from tools.clause_coverage.source_units import SourceUnitError
    a=inputs();oid=next(oid for oid,raw in a['observations'].items()if json.loads(raw)['source']=='official')
    o=json.loads(a['observations'][oid]);new_url='https://world.digimoncard.com/cards/?unreviewed=foreign-scope'
    a['raw_pages'][new_url]=a['raw_pages'][o['source_url']];o['source_url']=new_url
    a['observations'][oid]=serialization.canonical_json_bytes(o)
    with pytest.raises(SourceUnitError):compose(**a)
