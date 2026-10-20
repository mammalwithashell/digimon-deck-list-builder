"""Rule-aware composition tests over complete actual source context."""
import copy,gzip,json
from pathlib import Path
import pytest
from tools.card_data import normalization,serialization,adjudications
from tools.card_data.models import SourceObservation,SourceName,AdmissionStatus
from tools.clause_coverage.source_units import SourceUnitError
from test_clause_source_units import inputs as ordinary_inputs
F=Path(__file__).parents[1]/'fixtures/card_data/rule_card_composition'
BASE=F.parent/'source_binding/conditions'
enc=serialization.canonical_json_bytes
TRAITS=('form_eng','type_eng','attribute_eng')

def inputs(cid='EX12-026'):
 a=ordinary_inputs();d=json.loads((F/(cid+'.json')).read_bytes());card=copy.deepcopy(d['card']);obs=d['observations']
 cs=[normalization.observation_candidate(SourceObservation(**{**o,'source':SourceName(o['source'])})) for o in obs.values()]
 ledger=tuple(adjudications._row(r,path=Path('<fixture>')) for r in json.loads(a['adjudication_ledger'])['adjudications'])
 fields=('card_kind','effect_description_eng','inherited_effect_description_eng','security_effect_description_eng',*TRAITS)
 for f in fields:
  v,p,_=normalization._select_field(cid,f,[c for c in cs if f in c.fields],admission=AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=d['source_revision'])
  card['fields'][f]=v;card['provenance'][f]=serialization.canonicalize(p)
 runtime={'card_id':cid,**{f:card['fields'][f] for f in fields},'xros_req':d['xros_req']}
 images={r['image_artifact_path']:(BASE/(r['image_sha256']+'.webp')).read_bytes() for r in json.loads(a['correction_document'])['compatibility_corrections']}
 return {**a,'canonical_card':enc(card),'runtime_projection':enc(runtime),'observations':{k:enc(v) for k,v in obs.items()},'raw_pages':{u:gzip.decompress((F/n).read_bytes()) for u,n in d['raw_files'].items()},'image_artifacts':images}

def compose(**a):
 from tools.clause_coverage.rule_card_composition import compose_rule_card
 return compose_rule_card(**a)

@pytest.mark.parametrize('cid',['BT25-025','EX12-026','EX12-076','P-213'])
def test_four_actual_complete_rule_cards(cid):
 a=inputs(cid);before=copy.deepcopy(a);r=compose(**a)
 assert r['status']=='COMPOSED_RULE_CARD' and not r['blockers'] and a==before
 assert set(r['trait_bindings'])==set(TRAITS)
 official={k for k,v in a['observations'].items() if json.loads(v)['source']=='official'}
 assert set(r['rule_derivations'])==official
 assert r['structural_obligations']
 for ob in r['structural_obligations']:
  o=json.loads(a['observations'][ob['observation_id']]);s=o['extracted']['text_sections'][ob['section_index']]['text']
  assert s==ob['whole_container'] and s[ob['char_start']:ob['char_end']]==ob['line']
  assert s.encode()[ob['utf8_byte_start']:ob['utf8_byte_end']].decode()==ob['line']
  assert ob['purpose']=='trait-derivation-only' and not ob['executable']
 for unit in r['units']:
  original=json.loads(a['observations'][unit['observation_id']])['extracted']['text_sections'][unit['section_index']]['text']
  assert unit['text']==original
 assert len(r['consumption'])==len(r['inventory'])
 assert all('structural_obligation_indices' in c for c in r['consumption'])
 assert 'Hybrid' in r['trait_bindings']['form_eng']['value'] if cid=='EX12-076' else True

@pytest.mark.parametrize('field',TRAITS)
@pytest.mark.parametrize('mutation',['runtime','canonical','provenance','missing'])
def test_each_trait_requires_exact_current_producer_binding(field,mutation):
 a=inputs();c=json.loads(a['canonical_card']);r=json.loads(a['runtime_projection'])
 if mutation=='runtime':r[field]=['Invented']
 elif mutation=='canonical':c['fields'][field]=['Invented'];r[field]=['Invented']
 elif mutation=='provenance':c['provenance'][field]['selected_observation_id']='0'*64
 else:r.pop(field)
 a['canonical_card']=enc(c);a['runtime_projection']=enc(r)
 with pytest.raises(SourceUnitError):compose(**a)

@pytest.mark.parametrize('mutation',['raw','missing_member','stale','universe','validated','stripped_rule'])
def test_original_full_context_never_spoofed(mutation):
 a=inputs()
 if mutation=='raw':a['raw_pages']={k:v+b'x' for k,v in a['raw_pages'].items()}
 elif mutation=='missing_member':a['observations'].pop(next(iter(a['observations'])))
 elif mutation=='stale':a['old_input_revision']='0'*64
 elif mutation=='universe':a['expected_official_universe']=a['expected_official_universe'][:-1]
 elif mutation=='validated':a['validated']=True
 else:
  k=next(k for k,v in a['observations'].items() if json.loads(v)['source']=='official');o=json.loads(a['observations'][k]);o['extracted']['text_sections'][1]['text']='[On Play] Draw 1.';a['observations'][k]=enc(o)
 with pytest.raises(SourceUnitError):compose(**a)

@pytest.mark.parametrize('field,value',[('dual',{'unsupported':True}),('link_effect_description_eng','<Link> Unsupported')])
def test_other_structures_not_waived(field,value):
 a=inputs();d=json.loads(a['runtime_projection']);d[field]=value;a['runtime_projection']=enc(d);r=compose(**a)
 assert r['status']=='BLOCKED' and not r['units'] and not r['zones']

def test_any_unselected_rule_derivation_failure_blocks_whole_card(monkeypatch):
 import tools.clause_coverage.rule_card_composition as m
 original=m.derive_rule_occurrences
 def bad(o):
  r=original(o);r['blockers'].append({'code':'UNSUPPORTED_RULE_LINE'});r['status']='BLOCKED_RULE_DIAGNOSTIC';return r
 monkeypatch.setattr(m,'derive_rule_occurrences',bad)
 r=compose(**inputs('EX12-076'))
 assert r['status']=='BLOCKED' and not r['units'] and not r['zones']

def test_submitted_derivation_metadata_must_recompute():
 from tools.clause_coverage.rule_card_composition import validate_rule_card
 a=inputs();r=compose(**a);r['structural_obligations'][0]['char_start']+=1
 with pytest.raises(SourceUnitError):validate_rule_card(r,**a)

def test_only_unselected_official_rule_failure_still_blocks(monkeypatch):
 import tools.clause_coverage.rule_card_composition as m
 a=inputs('EX12-076');card=json.loads(a['canonical_card'])
 selected={p['selected_observation_id'] for p in card['provenance'].values() if p.get('selected_observation_id')}
 oid=next(k for k,v in a['observations'].items() if json.loads(v)['source']=='official' and k not in selected)
 real=m.derive_rule_occurrences
 def bad(o):
  r=real(o)
  if o['observation_id']==oid:r['blockers'].append({'code':'UNSUPPORTED_RULE_LINE'})
  return r
 monkeypatch.setattr(m,'derive_rule_occurrences',bad)
 r=compose(**a)
 assert r['status']=='BLOCKED' and not r['units'] and not r['zones']
 assert any(b.get('observation_id')==oid for b in r['blockers'])


def test_zero_clause_card_retains_all_empty_zones_and_no_rule_obligations():
 a=ordinary_inputs('BT1-045');c=json.loads(a['canonical_card']);r=json.loads(a['runtime_projection'])
 cs=[normalization.observation_candidate(SourceObservation(**{**json.loads(o),'source':SourceName(json.loads(o)['source'])})) for o in a['observations'].values()]
 ledger=tuple(adjudications._row(x,path=Path('<fixture>')) for x in json.loads(a['adjudication_ledger'])['adjudications'])
 for f in TRAITS:
  v,p,_=normalization._select_field('BT1-045',f,[x for x in cs if f in x.fields],admission=AdmissionStatus.ADMITTED,adjudications=ledger,field_selection_input_revision=a['new_input_revision'])
  c['fields'][f]=v;c['provenance'][f]=serialization.canonicalize(p);r[f]=v
 r['xros_req']='';a.update(canonical_card=enc(c),runtime_projection=enc(r),image_artifacts={})
 result=compose(**a)
 assert result['status']=='COMPOSED_RULE_CARD' and not result['structural_obligations']
 assert len(result['zones'])==4 and all(not z['value'] and not z['spans'] for z in result['zones'].values())
