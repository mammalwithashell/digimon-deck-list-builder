"""Opt-in paired main/primary-face correction proof; never whole-card admission.

The external reviewed sidecar pin authenticates image/printing association and
exact input byte provenance. Self-pinning a proposal is NOT approval. The caller
must supply records extracted from independently pinned full containers; this
helper does not load or authenticate those full containers itself. Original
sources remain intact; a face mirror is not a second executable effect.
"""
from pathlib import Path
import re
from urllib.parse import urlparse

from tools.card_data import (models,normalization,serialization,source_manifest,
    complete_candidate,corrections,generate,adjudications,compatibility_corrections,compatibility_selections,errors)
from tools.card_data.compatibility_corrections import correction_document_from_bytes
from tools.card_data.errors import CardDataError
from . import source_units, runtime_projection_proof
from .source_units import _json,_sha,_require,_ledger,SourceUnitError

FIELD='effect_description_eng'
FACE='faces.0.effect_text'
_KIND='reviewed-paired-main-image-v1'


def _dependencies():
    modules=(models,normalization,serialization,source_manifest,complete_candidate,corrections,
        generate,adjudications,compatibility_corrections,compatibility_selections,errors,source_units)
    return {**runtime_projection_proof.producer_dependencies(), **{m.__name__:_sha(Path(m.__file__).read_bytes()) for m in modules},
        runtime_projection_proof.__name__:_sha(Path(runtime_projection_proof.__file__).read_bytes()),
        __name__:_sha(Path(__file__).read_bytes())}


def _pins(inputs):
    return {k:({x:_sha(y) for x,y in sorted(v.items())} if isinstance(v,dict) else _sha(v))
        for k,v in inputs.items() if k in ('aggregate','scopes','observations','canonical_card',
            'runtime_card','adjudication_ledger','correction_document','raw_pages')}


def bind_paired_main_correction_unit(**inputs):
    try:
        return _bind(**inputs)
    except SourceUnitError:
        raise
    except (ValueError, TypeError, KeyError, AttributeError, IndexError, CardDataError) as error:
        raise SourceUnitError('paired main correction proof refused: '+str(error)) from error


def _bind(*, aggregate, scopes, observations, canonical_card, runtime_card,
          adjudication_ledger, correction_document, raw_pages, old_input_revision,
          new_input_revision, expected_official_universe, sidecar,
          expected_reviewed_sidecar_sha256, images):
    all_inputs=locals().copy()
    entry_dependencies=_dependencies()
    _require(isinstance(sidecar,bytes) and isinstance(expected_reviewed_sidecar_sha256,str)
        and re.fullmatch('[0-9a-f]{64}',expected_reviewed_sidecar_sha256)
        and _sha(sidecar)==expected_reviewed_sidecar_sha256,'externally pinned sidecar bytes required')
    proof=_json(sidecar)
    _require(isinstance(proof,dict) and set(proof)=={'schema_version','kind','card_id',
        'source_revision','input_sha256','dependency_sha256','corrections','evidence_observation_ids','images'}
        and type(proof['schema_version']) is int and proof['schema_version']==1
        and proof['kind']==_KIND,'unsupported paired correction schema')
    _require(isinstance(new_input_revision,str) and re.fullmatch('[0-9a-f]{64}',new_input_revision)
        and new_input_revision==old_input_revision==proof['source_revision'],'equal current source pins required')
    _require(proof['dependency_sha256']==entry_dependencies,'checker dependency drift')
    _require(proof['input_sha256']==_pins(all_inputs),'reviewed input bytes differ')
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
                if section['label'].lower() in ('effect','main effect'):
                    originals.append({'observation_id':oid,'printing_id':candidate.printing_id,
                        'section_index':index,**section,'raw_payload_digest':ob.raw_payload_digest,'source_url':ob.source_url})
    ledger=_ledger(adjudication_ledger)
    base=normalization.normalize_card(cid,typed.values(),adjudications=ledger,
        old_input_revision=old_input_revision,new_input_revision=new_input_revision,
        field_selection_input_revision=new_input_revision)
    _require(len(base.faces)==1 and base.faces[0].face_id=='primary'
        and base.faces[0].kind.value in ('tamer','digimon','option'), 'single primary ordinary face required')
    document=correction_document_from_bytes(correction_document)
    rows=[r for r in document.corrections if r.card_id==cid]
    _require(len(rows)==2 and {r.field_path for r in rows}=={FIELD,FACE}
        and not any(r.card_id==cid for r in document.compatibility_corrections),
        'exactly two linked main/primary-face rows required')
    _require(all(isinstance(r.value,str) and r.value for r in rows)
        and rows[0].value==rows[1].value,'paired values must be equal nonempty text')
    bindings=proof['corrections']
    _require(isinstance(bindings,dict) and set(bindings)=={FIELD,FACE},'exact paired field proof required')
    raw_rows=[r for r in _json(correction_document)['corrections'] if r['card_id']==cid]
    _require(len(raw_rows)==2,'duplicate correction rows')
    for row in raw_rows:
        pin=bindings[row['field_path']]
        _require(isinstance(pin,dict) and set(pin)=={'correction_id','sha256'}
            and pin['correction_id']==row['correction_id']
            and pin['sha256']==_sha(serialization.canonical_json_bytes(row)), 'exact correction row pin required')
    evidence=proof['evidence_observation_ids']
    _require(isinstance(evidence,list) and all(isinstance(o,str) for o in evidence)
        and len(evidence)==2 and len(set(evidence))==2
        and all(set(r.evidence_observation_ids)==set(evidence) for r in rows)
        and all(o in typed and typed[o].source is models.SourceName.OFFICIAL for o in evidence)
        and set(evidence)<={s['observation_id'] for s in originals}, 'exact paired same-card official evidence required')
    associations=proof['images']
    _require(isinstance(images,dict) and isinstance(associations,list) and len(associations)==2,
        'two immutable reviewed images required')
    seen_names=set();seen_ids=set();seen_printings=set()
    for image in associations:
        _require(isinstance(image,dict) and set(image)=={'artifact_name','sha256','printing_id','official_observation_id'},
            'strict image association schema required')
        name=image['artifact_name'];oid=image['official_observation_id'];printing=image['printing_id']
        _require(isinstance(name,str) and name in images and name not in seen_names
            and isinstance(images[name],bytes) and _sha(images[name])==image['sha256']
            and oid in evidence and oid not in seen_ids
            and printing==refs[oid][1]['printing_id'] and printing not in seen_printings,
            'exact distinct image/official printing association required')
        seen_names.add(name);seen_ids.add(oid);seen_printings.add(printing)
    _require(set(images)==seen_names and seen_printings=={cid,cid+'_P1'},'primary and P1 evidence closure required')
    corrected=base
    for row in sorted(rows,key=lambda r:r.field_path):
        corrected=corrections.apply_correction(corrected,row,observations=typed)
    _require(serialization.canonical_json_bytes(record)==serialization.canonical_json_bytes(corrected)
        and serialization.canonical_json_bytes(card)==serialization.canonical_json_bytes(corrected),
        'complete canonical record differs from exact paired producer replay')
    # Complete equality above preserves all sibling fields, faces, and provenance.
    _require(base.fields.get('security_effect_description_eng')==corrected.fields.get('security_effect_description_eng')
        and base.faces[0].security_text==corrected.faces[0].security_text,
        'security must remain untouched')
    runtime=_json(runtime_card);projected=generate.project_canonical_card(corrected)
    _require(isinstance(runtime,dict),'runtime record object required')
    fields=runtime_projection_proof.FIELDS
    for field in fields:
        _require(field in runtime and serialization.canonical_json_bytes(runtime[field])==serialization.canonical_json_bytes(projected[field]),
            'runtime printed field differs from producer: '+field)
    _require(_dependencies()==entry_dependencies,'checker dependencies changed during proof')
    return {'version':1,'kind':'paired-main-correction-proof','status':'PROOF_ONLY_BOUND',
        'whole_card_status':'NOT_COMPOSED','remaining_integration_gate':'Rule/condition/Link/Dual/ordinary validation and executable composition remain required',
        'source_revision':new_input_revision,'input_sha256':_pins(all_inputs),'sidecar_sha256':_sha(sidecar),
        'dependencies':entry_dependencies,'source_observations':{o:_json(b) for o,b in sorted(observations.items())},
        'original_sections':originals,'original_selected_value':base.fields[FIELD],
        'original_provenance':serialization.canonicalize(base.provenance),
        'corrections':raw_rows,'images':associations,'unchanged_security':True,
        'unit':{'kind':'reviewed-paired-main-image-correction','authority':'adjudicated','card_id':cid,
            'field_path':FIELD,'value':corrected.fields[FIELD],'runtime_value':projected[FIELD],
            'provenance':serialization.canonicalize(corrected.provenance[FIELD]),
            'runtime_transformation':'generate.project_canonical_card-v1',
            'source_offset_policy':'adjudicated value; no fabricated source or pixel offsets'},
        'face_mirror_obligation':{'field_path':FACE,'value':corrected.faces[0].effect_text,'executable':False,
            'provenance':serialization.canonicalize(corrected.provenance[FACE])},
        'runtime_claims':{f:runtime[f] for f in fields},
        'unclaimed_runtime_fields':{f:runtime[f] for f in sorted(set(runtime)-set(fields))}}


def validate_paired_main_correction_binding(binding,**inputs):
    _require(serialization.canonical_json_bytes(binding)==serialization.canonical_json_bytes(bind_paired_main_correction_unit(**inputs)),
        'paired main proof differs from exact replay')
