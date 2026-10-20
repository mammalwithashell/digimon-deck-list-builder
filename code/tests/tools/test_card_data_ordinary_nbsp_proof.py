"""Bounded NBSP proof: real seven-printing fixtures and refusal boundaries."""
import copy,gzip,json,hashlib
from pathlib import Path
import pytest
from tools.card_data.serialization import canonical_json_bytes,content_digest
F=Path(__file__).parents[1]/'fixtures/card_data/ordinary_nbsp'
FIELDS={'effect_description_eng','inherited_effect_description_eng','security_effect_description_eng'}
def bundle():
 b={k:json.loads(gzip.decompress((F/(k+'.json.gz')).read_bytes()))for k in ['report','snapshot','canonical','ledger','corrections','aggregate','secondary']}
 meta=json.loads((F/'provenance.json').read_bytes());b['raw_pages']={x['url']:gzip.decompress((F/x['file']).read_bytes())for x in meta['pages']};return b
def args(b):
 x={k+'_bytes':canonical_json_bytes(b[k])for k in ['report','snapshot','canonical','ledger','corrections','aggregate','secondary']}
 current={(r['card_id'],r['field_path'])for r in b['ledger']['adjudications']if r['kind']=='runtime_field_cutover'and r['old_input_revision']==b['report']['old_input_revision']and r['new_input_revision']==b['report']['new_input_revision']}
 remaining=[d for d in b['report']['runtime_field_diffs']if(d['card_id'],d['field_path'])not in current]
 bind={'input_sha256':{k:hashlib.sha256(v).hexdigest()for k,v in x.items()},'runtime_count':len(b['report']['runtime_field_diffs']),'remaining_runtime_count':len(remaining),'remaining_text_count':sum(d['field_path']in FIELDS for d in remaining),'official_card_ids':sorted({o['extracted']['card_id']for o in b['snapshot']['observations']})}
 return x|{'raw_pages':b['raw_pages'],'binding_bytes':canonical_json_bytes(bind)}
def prove(b):
 from tools.card_data.ordinary_nbsp_proof import prove
 return prove(**args(b))
def target(b):return next(d for d in b['report']['runtime_field_diffs']if d['card_id']=='BT7-059'and d['field_path']=='inherited_effect_description_eng')
def refresh_diff(d):
 for side in ['old','new']:d[side+'_digest']=content_digest(d[side+'_value'])
def test_actual_four_cards_full_inventory_and_original_inputs():
 b=bundle();old=copy.deepcopy(b);r=prove(b);assert b==old
 assert r['status']=='PROOF_ONLY_NO_APPROVALS'
 assert len(r['runtime_inventory'])==len(b['report']['runtime_field_diffs'])==9
 assert {d['card_id']for d in r['eligible']}=={'BT1-011','BT7-059','BT9-001','BT11-020'}
 assert sum(len(d['official_evidence'])for d in r['eligible'])==7
 assert all(d['consumer_check']['no_verdict_carry_forward']for d in r['eligible'])
 assert r['eligible']and 'adjudications'not in r
@pytest.mark.parametrize('old,new',[('[Your Turn] Gain\u00a01 memory.','[Your Turn] Gain 1 memory.'),('[Your Turn] [Once Per Turn] A\u00a0B.','[Your Turn] [Once Per Turn] A B.')])
def test_safe_equation_controls(old,new):
 from tools.card_data.ordinary_nbsp_proof import text_reasons
 assert text_reasons(old,new)==[]
@pytest.mark.parametrize('old,new',[(None,'x'),('',''),('a','a'),('a\u00a0b','a  b'),('a\u00a0b','a b!'),('[A\u00a0B] x','[A B] x'),('＜Security\u00a0A. +1＞','＜Security A. +1＞'),('[Your\u00a0Turn] A','[Your Turn] A'),('"A\u00a0B"','"A B"'),("'A\u00a0B'","'A B'"),('“A\u00a0B”','“A B”'),('[A\u00a0B','[A B'),('a\u00a0b\n','a b\r\n')])
def test_exact_and_protected_boundaries(old,new):
 from tools.card_data.ordinary_nbsp_proof import text_reasons
 assert text_reasons(old,new)
@pytest.mark.parametrize('change',['wording','timing','target','duration','newline','protected','canonical','faces','foreign_selected','missing_provenance','raw_missing','raw_changed','raw_duplicate','raw_hidden','raw_faq','raw_unknown','conflict','correction','rule','dual','link','gap','duplicate_diff','digest','scope_bool','scope_source','scope_parser','scope_kind','ledger_bool','ledger_row_bool','count','unbound_approval'])
def test_full_boundary_refuses_or_excludes(change):
 b=bundle();d=target(b);cid=d['card_id'];o=next(o for o in b['snapshot']['observations']if o['extracted']['card_id']==cid);card=b['canonical']['cards'][cid]
 if change in ['wording','timing','target','duration','newline']:
  d['new_value']+={'wording':'!','timing':' [Main]','target':' another Digimon','duration':' until end of turn','newline':'\n'}[change];refresh_diff(d)
 elif change=='protected':d['old_value']='[A\u00a0B]';d['new_value']='[A B]';refresh_diff(d)
 elif change=='canonical':card['fields'][d['field_path']]='changed'
 elif change=='faces':card['faces'][0]['inherited_text']='changed'
 elif change=='foreign_selected':card['provenance'][d['field_path']]['selected_observation_id']=next(x['observation_id']for x in b['snapshot']['observations']if x['extracted']['card_id']!=cid)
 elif change=='missing_provenance':d['provenance_observation_ids']=[]
 elif change=='raw_missing':b['raw_pages'].pop(o['source_url'])
 elif change.startswith('raw_'):
  raw=b['raw_pages'][o['source_url']]
  if change=='raw_changed':raw+=b'x'
  elif change=='raw_duplicate':raw+=raw
  elif change=='raw_hidden':raw=raw.replace(b'id="BT7-059"',b'id="BT7-059" hidden')
  elif change=='raw_faq':raw=raw.replace(b'class="cardInfoCol"',b'class="cardFAQCol"')
  else:raw=raw.replace(b'[Inherited Effect]',b'[Unknown Effect]')
  b['raw_pages'][o['source_url']]=raw
 elif change=='conflict':b['report']['canonical_conflicts'].append({'card_id':cid,'field_path':'dp','approval_required':True})
 elif change=='correction':b['corrections']['corrections'].append({'card_id':cid})
 elif change=='rule':o['extracted']['text_sections'].append({'label':'Rule','text':'Trait: Has [Dragon] type.'})
 elif change=='dual':o['extracted']['layout']='dual'
 elif change=='link':o['extracted']['text_sections'].append({'label':'Link Effect','text':'Gain 1 memory.'})
 elif change=='gap':o['extracted']['printing_field_gaps']=[{'field':'dp'}]
 elif change=='duplicate_diff':b['report']['runtime_field_diffs'].append(copy.deepcopy(d))
 elif change=='digest':d['new_digest']='0'*64
 elif change=='scope_bool':b['snapshot']['manifest']['schema_version']=True
 elif change=='scope_source':b['snapshot']['manifest']['source']='foreign'
 elif change=='scope_parser':b['snapshot']['manifest']['parser_version']='unknown'
 elif change=='scope_kind':b['snapshot']['manifest']['scope']='selected-cards'
 elif change=='ledger_bool':b['ledger']['schema_version']=True
 elif change=='ledger_row_bool':b['ledger']['adjudications'][0]['schema_version']=True
 elif change=='count':b['snapshot']['manifest']['expected_count']-=1
 elif change=='unbound_approval':d['approved']=True;d['approval_id']='invented'
 try:r=prove(b)
 except ValueError:return
 assert not any(x['card_id']==cid and x['field_path']==d['field_path']for x in r['eligible'])
def test_raw_binding_mismatch_is_fatal():
 from tools.card_data.ordinary_nbsp_proof import prove,ProofError
 a=args(bundle());a['report_bytes']+=b' '
 with pytest.raises(ProofError):prove(**a)


def rebind_observations(b):
 # Test-only re-pinning reaches the DOM/structure guard beyond the byte guard.
 mapping={}
 for o in b['snapshot']['observations']:
  old=o['observation_id'];o['raw_payload_digest']=hashlib.sha256(b['raw_pages'][o['source_url']]).hexdigest()
  o['observation_id']=content_digest({k:o[k]for k in ['raw_payload_digest','parser_version','source_schema_version','extracted']},exclude_operational=True);mapping[old]=o['observation_id']
 def replace(v):
  if isinstance(v,str):return mapping.get(v,v)
  if isinstance(v,list):return [replace(x)for x in v]
  if isinstance(v,dict):return {k:replace(x)for k,x in v.items()}
  return v
 for key in ['snapshot','canonical','report','ledger']:b[key]=replace(b[key])
 from tools.card_data import models,source_manifest
 scopes=[]
 for snap in [b['snapshot'],b['secondary']]:
  m=snap['manifest'];scopes.append(models.ScopeManifest(**(m|{'source':models.SourceName(m['source']),'observation_refs':tuple(models.ScopeObservationRef(**x)for x in m['observation_refs'])})))
 b['aggregate']=source_manifest.build_full_pool_source_manifest(scopes).to_dict()
 b['canonical']['source_manifests']={k:v['scope_manifest_digest']for k,v in b['aggregate']['sources'].items()}
 for key in ['canonical','report']:b[key]['source_manifest_digest']=b['aggregate']['manifest_revision']
 for card in b['canonical']['cards'].values():
  for prov in card['provenance'].values():
   if 'observation_ids'in prov:prov['observation_ids'].sort()

@pytest.mark.parametrize('change',['hidden_popup','faq_section','unknown_section','nested_label','duplicate_popup','rule','dual','link','gap'])
def test_repinned_dom_and_structural_fail_closed(change):
 b=bundle();o=next(x for x in b['snapshot']['observations']if x['extracted']['card_id']=='BT7-059');raw=b['raw_pages'][o['source_url']]
 if change=='hidden_popup':raw=raw.replace(b'id="BT7-059"',b'id="BT7-059" hidden')
 elif change=='faq_section':raw=raw.replace(b'class="cardInfoCol"',b'class="cardFAQCol"')
 elif change=='unknown_section':raw=raw.replace(b'[Inherited Effect]',b'[Unknown Effect]')
 elif change=='nested_label':raw=raw.replace(b'[Inherited Effect]</dt>',b'<span>[Inherited Effect]</span></dt>')
 elif change=='duplicate_popup':raw+=raw
 elif change=='rule':o['extracted']['text_sections'].append({'label':'Rule','text':'Trait: Has [Dragon] type.'})
 elif change=='dual':o['extracted']['layout']='dual'
 elif change=='link':o['extracted']['text_sections'].append({'label':'Link Effect','text':'Gain 1 memory.'})
 else:o['extracted']['printing_field_gaps']=[{'field':'dp'}]
 if change in ['hidden_popup','faq_section','unknown_section','nested_label','duplicate_popup']:assert raw!=b['raw_pages'][o['source_url']]
 b['raw_pages'][o['source_url']]=raw;rebind_observations(b)
 result=prove(b)
 assert not any(x['card_id']=='BT7-059'for x in result['eligible'])
 assert any(x['card_id']=='BT7-059'and x['reasons']for x in result['excluded'])

def test_full_inventory_outside_rows_and_historical_ledger_preserved():
 b=bundle();r=prove(b)
 assert [(x['card_id'],x['field_path'])for x in r['runtime_inventory']]==sorted((x['card_id'],x['field_path'])for x in b['report']['runtime_field_diffs'])
 assert r['ledger_preserved_sha256']==hashlib.sha256(canonical_json_bytes(b['ledger'])).hexdigest()
 assert sum(x['existing_approved']for x in r['runtime_inventory'])==3
 assert len(r['eligible'])+len(r['excluded'])==sum(x['field_path']in FIELDS for x in b['report']['runtime_field_diffs'])

@pytest.mark.parametrize('change',['secondary_omitted','secondary_boolean','foreign_printing','aggregate_membership','aggregate_boolean'])
def test_complete_two_source_closure(change):
 b=bundle()
 if change=='secondary_omitted':b['secondary']['observations'].pop()
 elif change=='secondary_boolean':b['secondary']['manifest']['schema_version']=True
 elif change=='foreign_printing':b['canonical']['cards']['BT7-059']['printings'][0]['observation_ids'].append('f'*64)
 elif change=='aggregate_membership':b['aggregate']['membership']['observation_count']-=1
 else:b['aggregate']['schema_version']=True
 with pytest.raises(ValueError):prove(b)

@pytest.mark.parametrize('text',['(Rule) Trait: Has [Hybrid] Form.','＜Link＞ More text.'])
def test_unparsed_raw_structural_note_is_excluded(text):
 from bs4 import BeautifulSoup
 from tools.card_data.ordinary_nbsp_proof import _raw_reasons
 b=bundle();o=next(x for x in b['snapshot']['observations']if x['extracted']['card_id']=='BT7-059')
 soup=BeautifulSoup(b['raw_pages'][o['source_url']],'html.parser');block=soup.find('div',class_='popupCol',id='BT7-059');note=soup.new_tag('div');note.string=text;block.append(note)
 assert 'raw_rule_or_link'in _raw_reasons(o,soup)

def test_canonical_source_scope_hashes_bound():
 b=bundle();b['canonical']['source_manifests']['official']='f'*64
 with pytest.raises(ValueError):prove(b)


@pytest.mark.parametrize('field,value',[('card_id','BT1-011'),('card_id',None),('schema_version',True),('schema_version',1.0),('schema_version',2),('schema_version',None)])
def test_canonical_record_identity_and_schema_refused(field,value):
 b=bundle();b['canonical']['cards']['BT7-059'][field]=value
 from tools.card_data.ordinary_nbsp_proof import ProofError
 with pytest.raises(ProofError,match='canonical record'):
  prove(b)
