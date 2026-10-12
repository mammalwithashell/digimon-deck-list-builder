"""Proof-only binding of a reviewed canonical inherited-field correction.

No source/book acceptance is produced. A reviewed sidecar SHA is an external
trust input; computing one's own SHA is not independent approval. Sealed reviewed
source extraction and image/printing association remain caller prerequisites.
"""
from pathlib import Path
import re
from urllib.parse import urlparse

from tools.card_data import (models,normalization,serialization,source_manifest,
    complete_candidate,corrections,generate,adjudications,compatibility_corrections,compatibility_selections,errors)
from tools.card_data.compatibility_corrections import correction_document_from_bytes
from tools.card_data.errors import CardDataError
from . import source_units
from .source_units import _json,_sha,_require,_ledger,SourceUnitError

FIELD='inherited_effect_description_eng'
_KIND='reviewed-inherited-field-image-v1'


def _dependencies():
    modules=(models,normalization,serialization,source_manifest,complete_candidate,corrections,
        generate,adjudications,compatibility_corrections,compatibility_selections,errors,source_units)
    return {**{m.__name__:_sha(Path(m.__file__).read_bytes()) for m in modules},
        __name__:_sha(Path(__file__).read_bytes())}


def _pins(inputs):
    return {k:({x:_sha(y) for x,y in sorted(v.items())} if isinstance(v,dict) else _sha(v))
        for k,v in inputs.items() if k in ('aggregate','scopes','observations','canonical_card',
            'runtime_card','adjudication_ledger','correction_document','raw_pages')}


def bind_canonical_correction_unit(**inputs):
    """Replay exact correction/image proof; deliberately not whole-card composition."""
    try:
        return _bind(**inputs)
    except SourceUnitError:
        raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as error:
        raise SourceUnitError('canonical correction proof refused: '+str(error)) from error


def _bind(*,aggregate,scopes,observations,canonical_card,runtime_card,adjudication_ledger,
          correction_document,raw_pages,old_input_revision,new_input_revision,
          expected_official_universe,sidecar,expected_reviewed_sidecar_sha256,image_bytes):
    all_inputs=locals().copy()
    _require(isinstance(sidecar,bytes) and isinstance(expected_reviewed_sidecar_sha256,str)
        and re.fullmatch('[0-9a-f]{64}',expected_reviewed_sidecar_sha256)
        and _sha(sidecar)==expected_reviewed_sidecar_sha256,'externally pinned sidecar bytes required')
    proof=_json(sidecar)
    _require(set(proof)=={'schema_version','kind','card_id','field_path','correction_id','source_revision',
        'input_sha256','dependency_sha256','correction_row_sha256','evidence_observation_ids','image'}
        and type(proof['schema_version']) is int and proof['schema_version']==1
        and proof['kind']==_KIND and proof['field_path']==FIELD,'unsupported correction proof schema/kind/field')
    _require(isinstance(new_input_revision,str) and re.fullmatch('[0-9a-f]{64}',new_input_revision)
        and new_input_revision==old_input_revision==proof['source_revision'],'equal current source pins required')
    _require(proof['dependency_sha256']==_dependencies(),'checker/producer dependencies differ from reviewed proof')
    _require(proof['input_sha256']==_pins(all_inputs),'input bytes differ from reviewed proof pins')
    _require(set(scopes)=={'official','digimoncard_io'},'both full source scopes required')
    typed_scopes=[]
    for source,raw in scopes.items():
        s=_json(raw)
        _require(type(s['schema_version']) is int and s['schema_version']==1
            and type(s['complete']) is bool and s['complete'] and type(s['expected_count']) is int
            and s['source']==source and s['scope'].startswith('full-pool:'),'invalid source scope')
        if source=='official':
            _require(s['scope']=='full-pool:official-catalog' and s['parser_version']=='official-html-v5'
                and s['source_schema_version']=='world-card-list-html-v1','unsupported official scope')
        typed_scopes.append(complete_candidate._manifest_from_dict(s))
    a=_json(aggregate);rebuilt=source_manifest.build_full_pool_source_manifest(typed_scopes).to_dict()
    _require(serialization.canonical_json_bytes(a)==serialization.canonical_json_bytes(rebuilt)
        and a['manifest_revision']==new_input_revision,'aggregate not exact complete-scope replay')
    universe=a['sources']['official']['gameplay_card_ids']
    _require(isinstance(expected_official_universe,(list,tuple)) and len(expected_official_universe)==len(set(expected_official_universe))
        and set(expected_official_universe)==set(universe),'independently pinned official universe mismatch')
    record=_json(canonical_card);cid=proof['card_id']
    _require(set(record)=={'admission','card_id','conflicts','faces','fields','printings','provenance','schema_version','tombstone_reason'}
        and record['card_id']==cid and cid in universe and type(record['schema_version']) is int
        and record['schema_version']==1,'canonical card identity/schema mismatch')
    card=complete_candidate._canonical_card_from_payload(cid,record,path=Path('<immutable-input>'))
    _require(card.admission is models.AdmissionStatus.ADMITTED,'admitted canonical card required')
    refs={r['observation_id']:(source,r) for source,lane in a['sources'].items()
        for r in lane['observation_refs'] if r['gameplay_card_id']==cid}
    _require(set(observations)==set(refs),'complete official and secondary card membership required')
    locations=next(s.source_locations for s in typed_scopes if s.source is models.SourceName.OFFICIAL)
    typed={};originals=[]
    for oid,raw in sorted(observations.items()):
        o=_json(raw);source,ref=refs[oid];lane=a['sources'][source]
        _require(type(o['schema_version']) is int and o['schema_version']==1 and o['observation_id']==oid
            and o['source']==source and o['parser_version']==lane['parser_version']
            and o['source_schema_version']==lane['source_schema_version'],'observation source identity mismatch')
        ob=models.SourceObservation(**{**o,'source':models.SourceName(source)})
        _require(serialization.observation_digest(ob)==oid,'observation content identity mismatch')
        candidate=normalization.observation_candidate(ob)
        _require(candidate.gameplay_card_id==cid and candidate.printing_id==ref['printing_id'],'foreign card/printing')
        typed[oid]=ob
        if source=='official':
            url=urlparse(ob.source_url)
            _require(url.scheme=='https' and url.netloc=='world.digimoncard.com' and url.path=='/cards/'
                and ob.source_url in locations and _sha(raw_pages[ob.source_url])==ob.raw_payload_digest,
                'official raw source pairing mismatch')
            for index,section in enumerate(ob.extracted['text_sections']):
                if section['label'].lower() in ('inherited effect','inheritable effect'):
                    originals.append({'observation_id':oid,'printing_id':candidate.printing_id,
                        'section_index':index,**section,'raw_payload_digest':ob.raw_payload_digest,'source_url':ob.source_url})
    ledger=_ledger(adjudication_ledger)
    base=normalization.normalize_card(cid,typed.values(),adjudications=ledger,
        old_input_revision=old_input_revision,new_input_revision=new_input_revision,
        field_selection_input_revision=new_input_revision)
    document=correction_document_from_bytes(correction_document)
    rows=[r for r in document.corrections if r.card_id==cid]
    _require(len(rows)==1 and not any(r.card_id==cid for r in document.compatibility_corrections),
        'exactly one supported same-card canonical correction required')
    correction=rows[0]
    raw_rows=[r for r in _json(correction_document)['corrections'] if r['correction_id']==proof['correction_id']]
    _require(len(raw_rows)==1 and correction.correction_id==proof['correction_id'] and correction.field_path==FIELD
        and isinstance(correction.value,str) and correction.value
        and _sha(serialization.canonical_json_bytes(raw_rows[0]))==proof['correction_row_sha256'],
        'correction row/value differs from pinned proof')
    evidence=proof['evidence_observation_ids']
    _require(isinstance(evidence,list) and len(evidence)==len(set(evidence)) and set(evidence)==set(correction.evidence_observation_ids)
        and all(oid in typed and typed[oid].source is models.SourceName.OFFICIAL for oid in evidence),
        'exact same-card official correction evidence required')
    _require(set(evidence)<={s['observation_id'] for s in originals},'correction lacks original inherited containers')
    corrected=corrections.apply_correction(base,correction,observations=typed)
    _require(serialization.canonical_json_bytes(card)==serialization.canonical_json_bytes(corrected),
        'complete canonical record differs from producer correction replay')
    image=proof['image']
    _require(isinstance(image,dict) and set(image)=={'artifact_name','sha256','printing_id','official_observation_id'}
        and isinstance(image_bytes,bytes) and _sha(image_bytes)==image['sha256']
        and image['artifact_name']==image['sha256']+'.webp','immutable explicit image binding required')
    image_oid=image['official_observation_id']
    _require(image_oid in evidence and refs[image_oid][0]=='official' and image['printing_id']==cid
        and refs[image_oid][1]['printing_id']==image['printing_id'],'image must bind explicit same-card primary official printing')
    runtime=_json(runtime_card)
    _require(runtime.get('card_id')==cid and isinstance(runtime.get(FIELD),str),'runtime corrected field identity required')
    projected=generate._legacy_text(corrected.fields[FIELD])
    _require(runtime[FIELD]==projected,'runtime field differs from exact pinned compatibility producer')
    modules=(models,normalization,serialization,source_manifest,complete_candidate,corrections,generate,adjudications,compatibility_corrections,compatibility_selections,errors)
    return {'version':1,'kind':'canonical-correction-unit-proof','status':'PROOF_ONLY_BOUND',
        'whole_card_status':'NOT_COMPOSED','remaining_integration_gate':'ordinary/condition/Rule/Dual and correction-aware context composition are not discharged',
        'input_sha256':_pins(all_inputs),'sidecar_sha256':_sha(sidecar),'source_revision':new_input_revision,
        'source_observations':{oid:_json(raw) for oid,raw in observations.items()},'original_sections':originals,
        'original_selected_value':base.fields[FIELD],'original_provenance':serialization.canonicalize(base.provenance[FIELD]),
        'unit':{'kind':'reviewed-canonical-image-correction','authority':'adjudicated','card_id':cid,'field_path':FIELD,
            'value':corrected.fields[FIELD],'runtime_value':projected,'provenance':serialization.canonicalize(corrected.provenance[FIELD]),
            'correction':raw_rows[0],'image':image,'runtime_transformation':'generate._legacy_text-v1',
            'source_offset_policy':'correction-derived value; no fabricated official-section or image-pixel offsets'},
        'dependencies':_dependencies(),
        'validator_sha256':_sha(Path(__file__).read_bytes())}


def validate_correction_binding(binding,**inputs):
    _require(serialization.canonical_json_bytes(binding)==serialization.canonical_json_bytes(bind_canonical_correction_unit(**inputs)),
        'correction proof differs from exact replay')
