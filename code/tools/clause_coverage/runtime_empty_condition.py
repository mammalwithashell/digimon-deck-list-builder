"""Opt-in exact empty-condition producer proof plus mandatory source replay.

This does not expand A2's supported section labels. Unknown sections stay in the
source inventory and must independently pass whole-card composition or block.
"""
from copy import deepcopy
from pathlib import Path

from tools.card_data import models,normalization,serialization,compatibility_conditions,compatibility_selections
from tools.card_data.errors import CardDataError
from . import runtime_projection_proof as projection
from .runtime_rule_composition import _verify_snapshot_dispatch,wrapper_dependencies
from .rule_card_composition import compose_rule_card

enc=serialization.canonical_json_bytes
_require=projection._require


def _dependencies():
    modules=(models,normalization,serialization,compatibility_conditions,compatibility_selections)
    return dict(sorted({**wrapper_dependencies(),
        **{m.__name__:projection._sha(Path(m.__file__).read_bytes())for m in modules},
        __name__:projection._sha(Path(__file__).read_bytes())}.items()))


def _empty_condition_evidence(card,a):
    selections=compatibility_selections.selections_from_bytes(a['condition_selections'])
    _require(not any(row.card_id==card.card_id for row in selections),
             'empty condition capability excludes target selections')
    pairs=[(oid,printing.printing_id)for printing in card.printings for oid in printing.observation_ids]
    members=dict(pairs)
    _require(len(members)==len(pairs) and set(a['observations'])==set(members),
             'complete original printing observations required')
    typed={};sections={}
    for oid,raw in sorted(a['observations'].items()):
        data=projection._json(raw)
        _require(type(data['schema_version'])is int and data['schema_version']==1,
                 'observation schema must be strict integer')
        observation=models.SourceObservation(**{**data,'source':models.SourceName(data['source'])})
        _require(observation.observation_id==oid and serialization.observation_digest(observation)==oid,
                 'observation content identity differs')
        candidate=normalization.observation_candidate(observation)
        _require(candidate.gameplay_card_id==card.card_id and candidate.printing_id==members[oid],
                 'foreign card or printing observation')
        typed[oid]=observation
        if observation.source is models.SourceName.OFFICIAL:
            sections[oid]=deepcopy(observation.extracted['text_sections'])
    # With target selections forbidden, production consensus succeeds only when
    # every official member contributes this exact empty value.
    value,ids=compatibility_conditions.official_condition_projection(card,typed,
        selections=selections,source_manifest_revision=a['new_input_revision'])
    _require(value=='' and set(ids)==set(sections) and bool(ids),
             'all official condition contributions must be exactly empty')
    return {'kind':'exact-unanimous-empty-condition-serialization-v1','projection':value,
        'producer':'tools.card_data.compatibility_conditions.official_condition_projection',
        'source_revision':a['new_input_revision'],'evidence_observation_ids':list(ids),
        'candidate_values':{oid:value for oid in ids},'official_sections':sections,
        'a2_section_coverage_claimed':False,
        'limit':'Empty compatibility value only; every source section still requires whole-card composition'}


def compose_empty_condition_runtime_rule_card(*,snapshot,expected_extraction_sha256,runtime_record,
        registry_bytes,runtime_binding,image_artifacts,**source_inputs):
    """No standalone source admission: recompute both serialization and source."""
    try:
        return _compose(snapshot,expected_extraction_sha256,runtime_record,registry_bytes,
                        runtime_binding,image_artifacts,source_inputs)
    except projection.RuntimeProjectionError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as error:
        raise projection.RuntimeProjectionError('empty runtime/source proof refused: '+str(error)) from error


def _compose(snapshot,expected_extraction_sha256,runtime_record,registry_bytes,runtime_binding,
             image_artifacts,source_inputs):
    meta,cid=_verify_snapshot_dispatch(snapshot=snapshot,expected_extraction_sha256=expected_extraction_sha256,
        runtime_record=runtime_record,registry_bytes=registry_bytes,source_inputs=source_inputs)
    a={k:v for k,v in source_inputs.items()if k!='scopes'}
    a.update(runtime_record=runtime_record,registry_bytes=registry_bytes,binding=runtime_binding,
             image_artifacts=image_artifacts)
    pin,deps,card,runtime,card_id,expected=projection._prepare_projection(a)
    _require(card_id==cid and runtime.get('xros_req')=='','runtime condition must be exactly empty')
    evidence=_empty_condition_evidence(card,a)
    expected['xros_req']=evidence['projection']
    for field,value in expected.items():
        _require(field in runtime and enc(runtime[field])==enc(value),'exact producer mismatch: '+field)
    view={'card_id':cid,**dict(card.fields),'xros_req':evidence['projection']}
    # Unknown sections are never removed or presented as understood by A2.
    source=compose_rule_card(**source_inputs,runtime_projection=enc(view),image_artifacts=image_artifacts)
    transforms={field:('generate._legacy_text'if field in projection.TEXT else
        'generate._legacy_kind'if field=='card_kind'else 'official_condition_projection-exact-empty-v1'if field=='xros_req'else
        'registry.compatibility_values'if field in {'index','norm_id','card_index'}else 'generate.project_canonical_card')
        for field in expected}
    return {'schema_version':1,'kind':'exact-empty-runtime-rule-wrapper-v1',
        'status':'EXACT_EMPTY_RUNTIME_SOURCE_COMPOSITION'if source['status']=='COMPOSED_RULE_CARD'else'BLOCKED',
        'card_id':cid,'snapshot':meta,'original_canonical_bytes':source_inputs['canonical_card'].decode('utf8'),
        'original_runtime_bytes':runtime_record.decode('utf8'),'derived_canonical_view':view,
        'derived_canonical_view_sha256':projection._sha(enc(view)),
        'input_sha256':pin['input_sha256'],'binding_sha256':projection._sha(runtime_binding),
        'producer_sha256':deps,'claimed_fields':list(expected),'projected_claimed_values':expected,
        'unclaimed_runtime_fields':{k:v for k,v in runtime.items()if k not in expected},
        'field_transformations':transforms,'empty_condition_evidence':evidence,
        'source_composition':source,'dependency_sha256':_dependencies(),
        'limit':'19 exact runtime fields plus separately required source composition; no A2 section coverage, publication, book or readiness approval'}


def validate_empty_condition_runtime_rule_card(result,**inputs):
    _require(enc(result)==enc(compose_empty_condition_runtime_rule_card(**inputs)),
             'empty runtime/source wrapper differs from complete replay')
