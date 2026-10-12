"""Proof-only image/printing kind association; never repairs or approves source.

The caller supplies a separately pinned proposal. Matching image bytes does not
prove their contents: independent visual review is still mandatory. Complete
source scopes/observations and raw disagreements are retained without mutation.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
from urllib.parse import urlparse,urljoin
from bs4 import BeautifulSoup,SoupStrainer
from tools.card_data import models,serialization,source_manifest,normalization,errors
from tools.card_data.sources import official
from . import dual_source_units

class AssociationError(ValueError):pass
def require(value,message):
    if not value:raise AssociationError(message)
def digest(raw):return hashlib.sha256(raw).hexdigest()
def encode(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,indent=2).encode()
def pairs(rows):
    out={}
    for k,v in rows:
        require(k not in out,'duplicate JSON key');out[k]=v
    return out
def decode(raw):
    require(isinstance(raw,bytes),'explicit input bytes required')
    return json.loads(raw,object_pairs_hook=pairs)
def prove_dual_kind_association(**inputs):
    try:return _prove(**inputs)
    except AssociationError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,errors.CardDataError) as error:
        raise AssociationError('Dual kind association refused: '+str(error)) from error

def _prove(*,aggregate,scopes,observations,raw_pages,images,proposal,
           expected_proposal_sha256,expected_source_revision,expected_official_universe):
    require(digest(proposal)==expected_proposal_sha256,'external proposal pin mismatch')
    p=decode(proposal)
    require(set(p)=={'version','kind','status','card_id','source_revision','reason','observations'}
        and type(p['version'])is int and p['version']==1
        and p['kind']=='dual-image-kind-association-proposal'
        and p['status']=='PENDING_INDEPENDENT_REVIEW'
        and isinstance(p['reason'],str)and bool(p['reason'].strip()),'unsupported proposal envelope')
    require(set(scopes)=={'official','digimoncard_io'},'complete two-lane scopes required')
    typed=[];scope_data={}
    for name,raw in scopes.items():
        s=decode(raw);scope_data[name]=s
        require(type(s['schema_version'])is int and s['schema_version']==1 and s['complete']is True
                and type(s['expected_count'])is int and s['source']==name,'scope authority headers')
        require(s['scope']==('full-pool:official-catalog' if name=='official' else 'full-pool:digimoncard-io-catalog'),'full source scope required')
        if name=='official':require(s['parser_version']=='official-html-v5'and s['source_schema_version']=='world-card-list-html-v1','unsupported official schema')
        else:require(s['parser_version']=='digimoncard-io-json-v1'and s['source_schema_version']=='digimoncard-io-public-api-v1','unsupported secondary schema')
        typed.append(models.ScopeManifest(**{**s,'source':models.SourceName(name),'observation_refs':tuple(models.ScopeObservationRef(**r)for r in s['observation_refs'])}))
    rebuilt=source_manifest.build_full_pool_source_manifest(typed).to_dict();a=decode(aggregate)
    require(serialization.canonical_json_bytes(a)==serialization.canonical_json_bytes(rebuilt),'aggregate differs from producer scope replay')
    require(a['manifest_revision']==expected_source_revision==p['source_revision'],'explicit source revision mismatch')
    official_ids=a['sources']['official']['gameplay_card_ids']
    require(isinstance(expected_official_universe,(list,tuple))and len(set(expected_official_universe))==len(expected_official_universe)
            and set(expected_official_universe)==set(official_ids),'full official universe mismatch')
    cid=p['card_id'];require(isinstance(cid,str)and cid in official_ids,'card outside official universe')
    refs={r['observation_id']:(name,r)for name,lane in a['sources'].items()for r in lane['observation_refs']if r['gameplay_card_id']==cid}
    require(set(observations)==set(refs),'complete same-card observation membership required')
    originals={};official_obs={}
    for oid,raw in observations.items():
        o=decode(raw);name,ref=refs[oid];lane=a['sources'][name]
        require(type(o['schema_version'])is int and o['schema_version']==1 and o['source']==name
                and o['observation_id']==oid and o['extracted']['card_id']==cid
                and o['parser_version']==lane['parser_version']and o['source_schema_version']==lane['source_schema_version'],'observation identity mismatch')
        typed_o=models.SourceObservation(**{**o,'source':models.SourceName(name)})
        require(serialization.observation_digest(typed_o)==oid,'observation content hash mismatch')
        candidate=normalization.observation_candidate(typed_o)
        require(candidate.printing_id==ref['printing_id']and candidate.gameplay_card_id==cid,'printing identity mismatch')
        originals[oid]=o
        if name=='official':official_obs[oid]=o
    rows=p['observations'];require(isinstance(rows,list)and len(rows)==len(official_obs)
        and {r['observation_id']for r in rows}==set(official_obs),'all official printings required exactly once')
    require(isinstance(images,dict)and all(isinstance(k,str)and isinstance(v,bytes)for k,v in images.items()),'owned image bytes required')
    require(len({r['image_artifact']for r in rows})==len(rows)and set(images)=={r['image_artifact']for r in rows},'image ownership must be unique and complete')
    associations=[];diagnostics=[]
    keys={'observation_id','printing_id','source_url','raw_sha256','source_image_url','image_artifact','image_sha256','raw_kind','image_kind','face_identity','faces_sha256','sections_sha256'}
    for r in rows:
        require(set(r)==keys and all(isinstance(r[k],str)for k in keys-{'face_identity'}),'association row schema')
        o=official_obs[r['observation_id']];e=o['extracted'];printing=e['printing'];url=o['source_url'];origin=urlparse(url)
        require(origin.scheme=='https'and origin.netloc=='world.digimoncard.com'and origin.path=='/cards/'and url in scope_data['official']['source_locations'],'official origin mismatch')
        require(r['printing_id']==printing['printing_id']and r['source_url']==url and r['source_image_url']==printing['image_url'],'printing/image source association mismatch')
        require(r['raw_sha256']==o['raw_payload_digest']==digest(raw_pages[url]),'raw page hash mismatch')
        require(r['image_sha256']==digest(images[r['image_artifact']]),'owned image hash mismatch')
        require(Path(r['image_artifact']).name==r['image_artifact']and r['image_artifact']==r['printing_id']+'.webp','unsupported owned image path')
        require(e['layout']=='dual'and r['raw_kind']==e['card_type']=='Digimon/Tamer'and r['image_kind']=='Digimon/Option','unsupported kind-association shape')
        identity=[{'face_id':f['face_id'],'kind':f['kind']}for f in e['faces']]
        require(identity==r['face_identity']==[{'face_id':'primary','kind':'digimon'},{'face_id':'dual','kind':'option'}],'unsupported ordered faces')
        require(r['faces_sha256']==digest(encode(e['faces']))and r['sections_sha256']==digest(encode(e['text_sections'])),'whole face/section binding mismatch')
        soup=BeautifulSoup(raw_pages[url].decode('utf8'),'html.parser',parse_only=SoupStrainer('div',id=printing['printing_id']))
        blocks=soup.find_all('div',class_='popupCol',id=printing['printing_id']);require(len(blocks)==1,'raw popup not unique');block=blocks[0]
        require(not block.has_attr('hidden')and not block.get('style'),'hidden popup')
        require([n.get_text(' ',strip=True)for n in block.select('li.cardNo')]==[cid],'raw card mismatch')
        kinds=block.select('li.cardType');require(len(kinds)==1 and not kinds[0].find(True)and not kinds[0].has_attr('hidden')and not kinds[0].get('style')and kinds[0].get_text(' ',strip=True)==r['raw_kind'],'raw kind disagreement not faithfully retained')
        require([urljoin(url,n.get('src',''))for n in block.select('div.cardImg img')]==[printing['image_url']],'raw image association mismatch')
        require(official._text_sections(str(block))==e['text_sections'],'raw global section mismatch')
        diagnostic=dual_source_units.derive_dual_occurrences(o)
        require(all(u['section_index']is not None for u in diagnostic['occurrences']),'face/global occurrence mismatch')
        diagnostics.append(diagnostic);associations.append(deepcopy(r))
    modules=(models,serialization,source_manifest,normalization,errors,official,dual_source_units)
    return {'version':1,'status':'PROOF_ONLY_PENDING_INDEPENDENT_REVIEW','approved':False,
        'authority':'proposed-image-derived-association','card_id':cid,'associations':associations,
        'source_observations':[deepcopy(originals[k])for k in sorted(originals)],'original_diagnostics':diagnostics,
        'input_sha256':{'aggregate':digest(aggregate),'scopes':{k:digest(v)for k,v in scopes.items()},
            'observations':{k:digest(v)for k,v in observations.items()},'raw_pages':{k:digest(v)for k,v in raw_pages.items()},
            'images':{k:digest(v)for k,v in images.items()},'proposal':digest(proposal)},
        'checker_sha256':digest(Path(__file__).read_bytes()),
        'dependency_sha256':{str(Path(m.__file__).relative_to(Path(__file__).parents[1])).replace('\\','/'):digest(Path(m.__file__).read_bytes())for m in modules},
        'gate':'Independent visual association review required; no source correction, composed card or runtime approval.'}

