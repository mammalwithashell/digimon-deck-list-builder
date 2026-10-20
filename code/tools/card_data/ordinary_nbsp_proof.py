"""Prove-only ordinary U+00A0 to ASCII-space changes; never emits approvals.

External byte/count binding defines the reviewed full input scope. Raw source
and every printing are independently replayed for candidate rows; all other
inventory rows remain visible. This is neither behavioral equivalence nor a
license to carry forward a historical semantic fingerprint/oracle verdict.
"""
from __future__ import annotations
from collections import defaultdict
from pathlib import Path
import json,re,unicodedata
from urllib.parse import urljoin
from bs4 import BeautifulSoup,SoupStrainer
from . import form_restoration_proof as base,normalization,adjudications,serialization,models,source_manifest
from . import compatibility_corrections,compatibility_selections,corrections,errors,compatibility_diff,complete_candidate
from .sources import official
from ..clause_coverage import text_split,semantic_fingerprint
ProofError=base.ProofError
PREDICATE_VERSION='ordinary-nbsp-outside-literals-v1'
FIELDS={'effect_description_eng':('Effect','effect_text','effect'),'inherited_effect_description_eng':('Inherited Effect','inherited_text','inherited'),'security_effect_description_eng':('Security Effect','security_text','security')}
STRUCTURAL=re.compile(r'(?:\[|<|\()\s*(?:rule|link)\b',re.I)

def _load(raw):
 def pairs(xs):
  out={}
  for k,v in xs:
   base._require(k not in out,'duplicate JSON key');out[k]=v
  return out
 try:
  base._require(isinstance(raw,bytes),'input bytes required')
  d=json.loads(raw,object_pairs_hook=pairs);base._require(isinstance(d,dict),'object required');return d
 except (UnicodeError,TypeError,json.JSONDecodeError)as e:raise ProofError('invalid input JSON')from e

def text_reasons(old,new):
 if not isinstance(old,str)or not isinstance(new,str)or not old or not new:return ['empty_or_nontext']
 if '\u00a0'not in old or old.replace('\u00a0',' ')!=new:return ['not_exact_nbsp_only']
 # A changed byte inside any recognized literal/token is not in this family.
 pairs={'[':']','＜':'＞','<':'>','「':'」','『':'』','“':'”','‘':'’','"':'"'}
 stack=[]
 for i,ch in enumerate(old):
  if stack and ch==stack[-1]:stack.pop();continue
  if ch in pairs:stack.append(pairs[ch]);continue
  if ch=="'" and not(i>0 and i+1<len(old)and old[i-1].isalnum()and old[i+1].isalnum()):
   if stack and stack[-1]=="'":stack.pop()
   else:stack.append("'")
  elif ch in ']＞>」』”’'and not stack:return ['unbalanced_literal']
  if ch=='\u00a0'and stack:return ['nbsp_inside_literal_or_token']
 if stack:return ['unbalanced_literal']
 return []

def _runtime(value):return value.replace('\r\n','\n').replace('\n','\r\n')

def consumer_check(cid,field,old,new):
 zone=FIELDS[field][2]
 def rows(text):
  return [dict(id=f'{cid}#{zone}#{i}',card_id=cid,zone=zone,kind=s.kind,timings=s.timings,keyword=s.keyword,text=s.text)for i,s in enumerate(text_split.split_clauses(text))]
 before,after=rows(old),rows(new)
 same=len(before)==len(after)and all({k:v for k,v in a.items()if k!='text'}=={k:v for k,v in b.items()if k!='text'}and a['text'].replace('\u00a0',' ')==b['text']for a,b in zip(before,after))
 protected=lambda t:re.findall(r'\[[^\]]*\]|＜[^＞]*＞|<[^>]*>|"[^"\n]*"',t)
 a=semantic_fingerprint.bind_clause_semantics(before);b=semantic_fingerprint.bind_clause_semantics(after)
 return dict(clause_structure_preserved=same,literal_tokens_preserved=protected(old)==protected(new),
  old_semantics=a,new_semantics=b,semantic_fingerprints_changed=[r['semantic_sha256']for r in a]!=[r['semantic_sha256']for r in b],
  no_verdict_carry_forward=True,behavioral_equivalence_claimed=False)

def _raw_reasons(o,soup):
 e=o['extracted'];blocks=soup.find_all('div',class_='popupCol',id=e['printing']['printing_id']);reasons=[]
 if len(blocks)!=1:return ['raw_popup_not_unique']
 block=blocks[0]
 raw_text=unicodedata.normalize('NFKC',block.get_text(' ',strip=True)+' '+' '.join(n.get('alt','')for n in block.select('img')))
 if STRUCTURAL.search(raw_text)or base.RULE.search(raw_text):reasons.append('raw_rule_or_link')
 if block.has_attr('hidden')or block.get('style'):reasons.append('hidden_raw_popup')
 if [x.get_text(' ',strip=True)for x in block.select('li.cardNo')]!=[e['card_id']]:reasons.append('raw_card_identity')
 types=block.select('li.cardType')
 if len(types)!=1 or types[0].find(True)or types[0].has_attr('hidden')or types[0].get('style')or types[0].get_text(' ',strip=True)!=e['card_type']:reasons.append('raw_type_identity')
 if [urljoin(o['source_url'],x.get('src',''))for x in block.select('div.cardImg img')]!=[e['printing']['image_url']]:reasons.append('raw_image_identity')
 if official._text_sections(str(block))!=e['text_sections']:reasons.append('raw_section_replay')
 for label in block.select('dt.cardInfoTitSmall'):
  dd=label.find_next_sibling('dd');column=label.find_parent('div',class_='cardInfoCol')
  if label.find(True)or label.parent.name!='dl'or 'cardInfoBoxSmall'not in label.parent.get('class',[])or column is None or column.find_parent('div',class_='popupCol')is not block:reasons.append('not_literal_printed_section')
  for node in [label,*label.parents]:
   if node.has_attr('hidden')or node.get('style')or any(str(c).lower().startswith('cardfaq')for c in node.get('class',[])):reasons.append('hidden_or_faq_section')
   if node is block:break
  if dd is None or any(x.has_attr('hidden')or x.get('style')for x in [dd,*dd.find_all(True)]):reasons.append('hidden_or_missing_body')
 return reasons

def prove(**kwargs):
 try:return _prove(**kwargs)
 except ProofError:raise
 except (ValueError,TypeError,KeyError,AttributeError,IndexError,errors.CardDataError)as error:raise ProofError('NBSP proof refused: '+str(error))from error

def _prove(*,report_bytes,snapshot_bytes,canonical_bytes,ledger_bytes,corrections_bytes,aggregate_bytes,secondary_bytes,binding_bytes,raw_pages):
 req=base._require;sha=base._sha;digest=serialization.content_digest
 raw_inputs=dict(report_bytes=report_bytes,snapshot_bytes=snapshot_bytes,canonical_bytes=canonical_bytes,ledger_bytes=ledger_bytes,corrections_bytes=corrections_bytes,aggregate_bytes=aggregate_bytes,secondary_bytes=secondary_bytes)
 binding=_load(binding_bytes);req(binding['input_sha256']=={k:sha(v)for k,v in raw_inputs.items()},'input byte binding mismatch')
 report,snapshot,canonical,ledger,correction,aggregate,secondary=map(_load,raw_inputs.values())
 req(all(type(d.get('schema_version'))is int and d['schema_version']==1 for d in [report,canonical,aggregate]),'typed input schemas')
 for k in ['old_input_revision','new_input_revision','source_manifest_digest']:req(isinstance(report[k],str)and base.HEX.fullmatch(report[k]),'report revision identity')
 req(canonical['source_manifest_digest']==report['source_manifest_digest'],'canonical aggregate mismatch')
 scopes=[]
 for source,snap,parser,source_schema,scope in [('official',snapshot,'official-html-v5','world-card-list-html-v1','full-pool:official-catalog'),('digimoncard_io',secondary,'digimoncard-io-json-v1','digimoncard-io-public-api-v1','full-pool:digimoncard-io-catalog')]:
  manifest=snap['manifest']
  req(type(manifest['schema_version'])is int and manifest['schema_version']==1 and type(manifest['expected_count'])is int and manifest['source']==source and manifest['parser_version']==parser and manifest['source_schema_version']==source_schema and manifest['scope']==scope and manifest['complete']is True and manifest['failed_ids']==[],'complete typed source scope')
  scopes.append(models.ScopeManifest(**(manifest|{'source':models.SourceName(source),'observation_refs':tuple(models.ScopeObservationRef(**r)for r in manifest['observation_refs'])})))
 rebuilt=source_manifest.build_full_pool_source_manifest(scopes).to_dict()
 req(serialization.canonical_json_bytes(rebuilt)==serialization.canonical_json_bytes(aggregate)and rebuilt['manifest_revision']==report['source_manifest_digest'],'exact producer aggregate replay')
 req(canonical['source_manifests']=={k:v['scope_manifest_digest']for k,v in rebuilt['sources'].items()},'canonical source scope hashes')
 m=snapshot['manifest'];req(type(m['schema_version'])is int and m['schema_version']==1 and m['source']=='official'and m['scope']=='full-pool:official-catalog'and m['complete']is True and m['failed_ids']==[]and m['parser_version']=='official-html-v5'and m['source_schema_version']=='world-card-list-html-v1','strict complete official manifest required')
 by_id={};by_card=defaultdict(list);pages={}
 for o in snapshot['observations']:
  req(type(o['schema_version'])is int and o['schema_version']==1 and o['source']=='official'and o['parser_version']==m['parser_version']and o['source_schema_version']==m['source_schema_version'],'observation authority')
  typed=models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL}));oid=o['observation_id'];req(oid not in by_id and serialization.observation_digest(typed)==oid,'observation identity')
  url=o['source_url'];req(base._origin(url)and url in m['source_locations']and url in raw_pages,'official source/raw membership')
  if url not in pages:pages[url]=sha(raw_pages[url])
  req(pages[url]==o['raw_payload_digest'],'raw page mismatch');cid=o['extracted']['card_id'];req(models.CARD_ID_RE.fullmatch(cid),'invalid card id')
  by_id[oid]=o;by_card[cid].append(o)
 req(type(m['expected_count'])is int and m['expected_count']==len(by_card),'official universe count')
 req(isinstance(binding['official_card_ids'],list)and len(binding['official_card_ids'])==len(set(binding['official_card_ids']))and set(binding['official_card_ids'])==set(by_card),'full pinned official universe')
 req(len(m['observation_ids'])==len(by_id)and set(m['observation_ids'])==set(by_id),'observation closure')
 refs=m['observation_refs'];req(len(refs)==len(by_id)and {x['observation_id']for x in refs}==set(by_id),'printing closure')
 for ref in refs:
  e=by_id[ref['observation_id']]['extracted'];req(ref['gameplay_card_id']==e['card_id']and ref['printing_id']==e['printing']['printing_id'],'printing reference identity')
 all_candidates=defaultdict(list);secondary_by_id={};sm=secondary['manifest']
 for o in secondary['observations']:
  req(type(o['schema_version'])is int and o['schema_version']==1 and o['source']=='digimoncard_io'and o['parser_version']==sm['parser_version']and o['source_schema_version']==sm['source_schema_version'],'secondary observation authority')
  typed=models.SourceObservation(**(o|{'source':models.SourceName.DIGIMONCARD_IO}));oid=o['observation_id']
  req(oid not in secondary_by_id and serialization.observation_digest(typed)==oid,'secondary observation identity')
  candidate=normalization.observation_candidate(typed);secondary_by_id[oid]=candidate;all_candidates[candidate.gameplay_card_id].append(candidate)
 req(len(sm['observation_ids'])==len(secondary_by_id)and set(sm['observation_ids'])==set(secondary_by_id),'complete secondary observation closure')
 for ref in sm['observation_refs']:
  candidate=secondary_by_id[ref['observation_id']];req(candidate.gameplay_card_id==ref['gameplay_card_id']and candidate.printing_id==ref['printing_id'],'secondary printing identity')
 for o in by_id.values():
  candidate=normalization.observation_candidate(models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL})));all_candidates[candidate.gameplay_card_id].append(candidate)
 cards=canonical['cards'];req(isinstance(cards,dict),'canonical record map')
 for cid,card in cards.items():
  req(isinstance(card,dict)and isinstance(cid,str)and card.get('card_id')==cid and type(card.get('schema_version'))is int and card['schema_version']==1,'canonical record identity/schema')
 req(set(by_card)<=set(cards),'canonical official membership')
 req(set(ledger)=={'schema_version','adjudications'}and type(ledger['schema_version'])is int and ledger['schema_version']==1 and isinstance(ledger['adjudications'],list),'typed ledger envelope')
 req(all(isinstance(row,dict)and type(row.get('schema_version'))is int and row['schema_version']==1 for row in ledger['adjudications']),'typed ledger rows')
 rows=adjudications.validate_migration_adjudications([adjudications._row(x,path=Path('<pinned-ledger>'))for x in ledger['adjudications']])
 document=compatibility_corrections.correction_document_from_bytes(corrections_bytes)
 req(all(type(r.get('schema_version'))is int and r['schema_version']==1 for r in correction['corrections']),'typed correction rows')
 typed_observations={oid:models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL}))for oid,o in by_id.items()}
 typed_observations.update({oid:c.observation for oid,c in secondary_by_id.items()})
 correction_by_field={(c.card_id,c.field_path):c for c in document.corrections}
 correction_approved=set();correction_evidence=[]
 current=defaultdict(list)
 for row in rows:
  if row.kind.value=='runtime_field_cutover'and row.old_input_revision==report['old_input_revision']and row.new_input_revision==report['new_input_revision']:current[(row.card_id,row.field_path)].append(row)
 index={};approved=set()
 for d in report['runtime_field_diffs']:
  key=(d['card_id'],d['field_path']);req(key not in index,'duplicate runtime field');index[key]=d
  req(type(d.get('approved'))is bool,'typed report approval flag')
  req(d['old_digest']==digest(d['old_value'])and d['new_digest']==digest(d['new_value']),'runtime diff digest')
  matches=current.get(key,[]);req(len(matches)<=1,'duplicate current approval')
  for a in matches:
   req(a.old_digest==d['old_digest']and a.new_digest==d['new_digest']and a.old_value==d['old_value']and a.new_value==d['new_value']and a.provenance_observation_ids and set(a.provenance_observation_ids)<=set(d['provenance_observation_ids']),'current approval mismatch');approved.add(key)
  if d.get('approved')is True:
   if d.get('approval_kind')=='correction':
    c=correction_by_field.get(key)
    req(c is not None and c.correction_id==d.get('approval_id'),'unbound report correction')
    ids=c.evidence_observation_ids
    req(bool(ids)and len(ids)==len(set(ids))and set(ids)==set(d['provenance_observation_ids'])and len(d['provenance_observation_ids'])==len(ids),'exact correction diff evidence')
    card=cards[d['card_id']];prov=card['provenance'][d['field_path']]
    req(prov.get('adjudicated')is True and prov.get('derived')is False,'exact correction provenance flags')
    pairs=[(x['printing_id'],oid)for x in card['printings']for oid in x['observation_ids']]
    req(len(pairs)==len(set(pairs))and set(pairs)=={(x.printing_id,x.observation.observation_id)for x in all_candidates[d['card_id']]},'full corrected-card printing closure')
    typed_card=complete_candidate._canonical_card_from_payload(d['card_id'],card,path=Path('<pinned-canonical>'))
    req(corrections.correction_is_already_applied(typed_card,c,observations=typed_observations),'canonical correction not exactly applied')
    typed_diff=compatibility_diff.RuntimeFieldDiff(**{k:(tuple(d[k])if k=='provenance_observation_ids'else d[k])for k in ['card_id','field_path','old_value','new_value','old_digest','new_digest','provenance_observation_ids']})
    replay=compatibility_diff.approve_runtime_diffs([typed_diff],corrections=[c],adjudications=rows,observations=typed_observations,old_input_revision=report['old_input_revision'],new_input_revision=report['new_input_revision'])[0]
    req(replay.approval_kind=='correction'and replay.approval_id==d['approval_id'],'producer correction approval replay')
    correction_approved.add(key);correction_evidence.append({'card_id':c.card_id,'field_path':c.field_path,'correction_id':c.correction_id,'correction':serialization.canonicalize(c),'report_diff':d,'canonical_provenance':prov})
   else:
    req(d.get('approval_kind')=='migration_adjudication'and matches and d.get('approval_id')==matches[0].adjudication_id,'unbound report approval')
 req(set(current)<=set(index),'current ledger rows outside report')
 ledger_approved=set(approved);approved|=correction_approved
 remaining=set(index)-approved
 counts={'runtime_count':len(index),'remaining_runtime_count':len(remaining),'remaining_text_count':sum(k[1]in FIELDS for k in remaining)}
 if 'schema_version'in binding:
  req(type(binding['schema_version'])is int and binding['schema_version']==2,'unsupported binding schema')
  counts.update(approved_runtime_ledger_count=len(ledger_approved),approved_correction_count=len(correction_approved),approved_overlap_count=len(ledger_approved&correction_approved))
 else:req(not correction_approved,'correction accounting requires binding schema2')
 req(all(type(binding[k])is int and binding[k]==v for k,v in counts.items()),'full denominator binding')
 corrected={x['card_id']for doc in [report,correction]for key in ['corrections','compatibility_corrections']for x in doc.get(key,[])}
 conflicts={x['card_id']for x in report.get('canonical_conflicts',[])if x.get('approval_required')}
 eligible=[];excluded=[];soups={}
 for key,d in sorted(index.items()):
  cid,field=key
  if field not in FIELDS:continue
  reasons=text_reasons(d['old_value'],d['new_value']);evidence=[];check=None
  if cid in corrected:reasons.append('card_has_correction')
  if cid in conflicts:reasons.append('required_canonical_conflict')
  if cid in base.KNOWN_SOURCE_GAPS:reasons.append('known_adjacent_source_gap')
  if not by_card.get(cid):reasons.append('missing_official_printing')
  if not reasons:
   card=cards[cid];p=card['provenance'][field];ids=p['observation_ids'];selected=p['selected_observation_id'];own={o['observation_id']for o in by_card[cid]};printingids={oid for x in card['printings']for oid in x['observation_ids']}
   pairs=[(x['printing_id'],oid)for x in card['printings']for oid in x['observation_ids']]
   req(len(pairs)==len(set(pairs))and set(pairs)=={(c.printing_id,c.observation.observation_id)for c in all_candidates[cid]},'full canonical two-source printing closure')
   req(isinstance(ids,list)and len(ids)==len(set(ids))and set(ids)==set(d['provenance_observation_ids'])and own<=set(ids)<=printingids and selected in own and selected in ids,'complete same-card selected provenance')
   selected_value,selected_provenance,field_conflicts=normalization._select_field(cid,field,[c for c in all_candidates[cid]if field in c.fields],admission=models.AdmissionStatus.ADMITTED,adjudications=rows,field_selection_input_revision=report['source_manifest_digest'])
   req(serialization.canonicalize(selected_provenance)==p and selected_value==card['fields'][field],'exact producer selected-field provenance replay')
   if any(c.requires_adjudication(models.AdmissionStatus.ADMITTED)for c in field_conflicts):reasons.append('required_computed_canonical_conflict')
   label,facefield,_=FIELDS[field]
   if not isinstance(card['fields'].get(field),str)or _runtime(card['fields'][field])!=d['new_value']:reasons.append('canonical_new_value_mismatch')
   for o in by_card[cid]:
    e=o['extracted'];sections=e['text_sections'];typed=models.SourceObservation(**(o|{'source':models.SourceName.OFFICIAL}))
    if e.get('layout')!='single'or len(e.get('faces',[]))!=1:reasons.append('not_single_face')
    if e.get('printing_field_gaps'):reasons.append('printing_field_gap')
    if any(s['label']not in {v[0]for v in FIELDS.values()}for s in sections):reasons.append('unsupported_source_section')
    if any(STRUCTURAL.search(unicodedata.normalize('NFKC',s['text']))for s in sections):reasons.append('rule_or_link_structure')
    secs=[s for s in sections if s['label']==label]
    if len(secs)!=1 or _runtime(secs[0]['text'])!=d['new_value']:reasons.append('explicit_ordinary_section_mismatch')
    fields=normalization._normalize_candidate_fields(normalization._official_fields(e));faces=serialization.canonicalize(normalization._parse_faces(e['faces'],observation=typed))
    if faces!=card['faces']:reasons.append('canonical_face_replay_mismatch')
    if not isinstance(fields.get(field),str)or _runtime(fields[field])!=d['new_value']:reasons.append('official_projection_mismatch')
    for f,(_,ff,_)in FIELDS.items():
     values=[fields.get(f),card['fields'].get(f),faces[0].get(ff)if len(faces)==1 else None]
     if any(not isinstance(v,str)for v in values)or len({_runtime(v)for v in values if isinstance(v,str)})!=1:reasons.append('adjacent_zone_replay_mismatch')
    url=o['source_url'];pk=(url,cid)
    if pk not in soups:soups[pk]=BeautifulSoup(raw_pages[url].decode('utf8'),'html.parser',parse_only=SoupStrainer('div',id=[x['extracted']['printing']['printing_id']for x in by_card[cid]]))
    reasons.extend(_raw_reasons(o,soups[pk]));evidence.append({'observation_id':o['observation_id'],'printing_id':e['printing']['printing_id'],'raw_sha256':o['raw_payload_digest'],'source_url':url,'text_sections':sections})
   check=consumer_check(cid,field,d['old_value'],d['new_value'])
   if not check['clause_structure_preserved']or not check['literal_tokens_preserved']:reasons.append('consumer_structure_changed')
  row={'card_id':cid,'field_path':field,'diff':d,'existing_approved':key in approved,'official_evidence':evidence}
  if reasons:excluded.append(row|{'reasons':sorted(set(reasons))})
  else:eligible.append(row|{'consumer_check':check})
 eligiblekeys={(x['card_id'],x['field_path'])for x in eligible};excludedkeys={(x['card_id'],x['field_path'])for x in excluded}
 inventory=[{'card_id':k[0],'field_path':k[1],'diff':d,'existing_approved':k in approved,'classification':'eligible'if k in eligiblekeys else'excluded'if k in excludedkeys else'outside_family'}for k,d in sorted(index.items())]
 mods=[base,normalization,adjudications,serialization,models,source_manifest,compatibility_corrections,compatibility_selections,corrections,errors,official,text_split,semantic_fingerprint,compatibility_diff,complete_candidate]
 return {'status':'PROOF_ONLY_NO_APPROVALS','predicate_version':PREDICATE_VERSION,'predicate_sha256':sha(Path(__file__).read_bytes()),'dependency_sha256':{m.__name__:sha(Path(m.__file__).read_bytes())for m in mods},'rust_keyword_consumer_sha256':sha((Path(__file__).parents[2]/'digimon-engine/src/card_data.rs').read_bytes()),'input_sha256':binding['input_sha256'],'binding_sha256':sha(binding_bytes),'raw_page_sha256':pages,'source_manifest_revision':aggregate['manifest_revision'],'source_membership':aggregate['sources'],'official_card_ids':sorted(by_card),'runtime_inventory':inventory,'counts':counts,'correction_approvals':correction_evidence,'eligible':eligible,'excluded':excluded,'membership_sha256':digest(sorted(eligiblekeys)),'ledger_preserved_sha256':sha(ledger_bytes)}
