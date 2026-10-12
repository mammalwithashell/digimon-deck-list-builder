"""Opt-in whole-card ordinary/condition source composition.

Complete sealed inputs are replayed; no caller context or 'validated' flag is
accepted. This verifies source bindings, not HTML extraction/image authenticity.
Rule, Dual, Link and image-corrected card composition remain explicit blockers.
No clause IDs, books, evidence verdicts or production data are emitted.
"""
from copy import deepcopy

from tools.card_data.compatibility_conditions import _CONDITION_LABELS
from tools.card_data.errors import CardDataError
from tools.card_data.serialization import canonical_json_bytes
from .condition_app_fusion import split_app_fusion_condition_units
from .condition_spans import ConditionSpanError
from .source_binding import bind_condition_units
from .source_units import (
    SourceUnitError, _validated_context, _compose_verified_ordinary, _require, _sha,
)


def compose_condition_card(*, image_artifacts, **inputs):
    """Compose every supported unit or retain whole-card blockers and inventory."""
    try:
        return _compose(image_artifacts=image_artifacts, **inputs)
    except SourceUnitError:
        raise
    except (ValueError, TypeError, KeyError, AttributeError, IndexError, CardDataError) as error:
        raise SourceUnitError('whole-card source composition refused: '+str(error)) from error


def _blocked(result):
    result['status']='BLOCKED'
    result['consumption']=[{**deepcopy(row),'unit_indices':[],
        'disposition':'blocked','reason':'whole-card unsupported structure or grammar'}
        for row in result['inventory']]
    return result


def _compose(*, image_artifacts, **inputs):
    return _compose_context(_validated_context(**inputs), image_artifacts=image_artifacts, inputs=inputs)


def _compose_context(context, *, image_artifacts, inputs):
    """Private dispatcher; public callers always replay complete raw inputs."""
    result = context['result']; runtime = context['runtime']
    _require(isinstance(runtime.get('xros_req'), str), 'explicit runtime condition field required')
    _require(isinstance(image_artifacts, dict) and all(isinstance(k,str) and isinstance(v,bytes)
             for k,v in image_artifacts.items()), 'image artifact bytes required')
    result.update(kind='whole-card-source-unit-composition', condition_binding=None, consumption=[])
    result['input_sha256']['images']={k:_sha(v)for k,v in sorted(image_artifacts.items())}
    condition_inventory=[r for r in result['inventory']if r['label'].lower() in _CONDITION_LABELS]
    delegated={}
    for r in condition_inventory:
        delegated.setdefault(r['observation_id'],set()).add(r['section_index'])
    other_projection=(len(context['card'].faces)>1 or runtime.get('dual')
        or runtime.get('link_effect_description_eng') or context['card'].fields.get('link_effect_description_eng'))
    pending=[]; hard=[]
    for blocker in result['blockers']:
        code=blocker['code']
        if ((code=='UNSUPPORTED_SECTION' and blocker['label'].lower() in _CONDITION_LABELS)
                or code=='UNSUPPORTED_CONDITION_SELECTION'
                or (code=='UNSUPPORTED_STRUCTURAL_PROJECTION' and not other_projection)):
            pending.append(blocker)
        else:
            hard.append(blocker)
    if hard:
        # Never present a partial card; pending condition dispatch is also
        # recorded, rather than silently discharged on an unsupported card.
        result['blockers']=hard+pending
        return _blocked(result)
    binding=None; condition_units=[]; condition_spans=[]
    if condition_inventory or runtime['xros_req'] or pending:
        # A2's public contract accepts precisely these two runtime fields.
        # Both the original full input and this exact view remain hash-pinned.
        view=canonical_json_bytes({'card_id':result['card_id'],'xros_req':runtime['xros_req']})
        binding=bind_condition_units(aggregate=inputs['aggregate'],observations=inputs['observations'],
            canonical_card=inputs['canonical_card'],runtime_projection=view,
            selection_ledger=inputs['condition_selections'],correction_document=inputs['correction_document'],
            image_artifacts=image_artifacts,raw_pages=inputs['raw_pages'],
            old_input_revision=inputs['old_input_revision'],new_input_revision=inputs['new_input_revision'])
        _require(binding['card_id']==result['card_id'] and binding['projection']==runtime['xros_req'],
                 'condition binding does not belong to same card/projection')
        condition_units=binding['units']
        _require(all(u.get('kind')=='official-section'for u in condition_units),
                 'image-corrected whole-card composition unsupported')
        _require('\n'.join(f"[{u['label']}] {u['text']}".strip()for u in condition_units)==runtime['xros_req'],
                 'condition occurrences do not reconstruct full projection')
        keys=[(u['observation_id'],u['section_index'])for u in condition_units]
        _require(len(keys)==len(set(keys)), 'duplicate condition occurrence')
        selected={u['observation_id']for u in condition_units}
        _require(len(selected)<=1, 'condition source selection mixes observations')
        expected={(r['observation_id'],r['section_index'])for r in condition_inventory
                  if r['observation_id'] in selected}
        _require(set(keys)==expected and (bool(condition_units) or not runtime['xros_req']),
                 'condition selection omitted an occurrence')
        inventory_by_key={(r['observation_id'],r['section_index']):r for r in result['inventory']}
        for u in condition_units:
            original=inventory_by_key[(u['observation_id'],u['section_index'])]
            _require(all(u[k]==original[k]for k in ('printing_id','label','text')),
                     'condition occurrence differs from verified original')
        try:
            condition_spans=split_app_fusion_condition_units(condition_units)
        except ConditionSpanError as error:
            result['condition_binding']=binding
            result['blockers']=pending+[{'code':'UNSUPPORTED_CONDITION_GRAMMAR','reason':str(error)}]
            return _blocked(result)
    result['blockers']=[]
    result=_compose_verified_ordinary(context, delegated_indices=delegated)
    result['condition_binding']=binding
    indices=[];spans=[]
    for unit,span_row in zip(condition_units,condition_spans,strict=True):
        _require(span_row['source_unit']==unit, 'condition span source changed')
        index=len(result['units']);indices.append(index)
        result['units'].append({**deepcopy(unit),'transformation':'a2-condition-source',
                                'projected_text':unit['text']})
        spans.extend({'unit_index':index,**deepcopy(s)}for s in span_row['spans'])
    result['zones']['conditions']={'field':'xros_req','value':runtime['xros_req'],
        'presence':'nonempty'if runtime['xros_req']else'absent-box',
        'source_binding':binding,'unit_indices':indices,'spans':spans}
    consumed={}
    for index,u in enumerate(result['units']):
        consumed.setdefault((u['observation_id'],u['section_index']),[]).append(index)
    for original in result['inventory']:
        key=(original['observation_id'],original['section_index']);refs=consumed.get(key,[])
        result['consumption'].append({**deepcopy(original),'unit_indices':refs,
            'disposition':'consumed'if refs else'retained-alternative',
            'reason':'selected producer projection'if refs else'not selected for this section field'})
    _require(set(consumed)<={(r['observation_id'],r['section_index'])for r in result['inventory']},
             'consumption references foreign source')
    result['status']='COMPOSED_CARD'
    return result


def validate_condition_card(binding, **inputs):
    """Recompute complete composition; submitted metadata cannot certify itself."""
    expected=compose_condition_card(**inputs)
    _require(canonical_json_bytes(binding)==canonical_json_bytes(expected),
             'whole-card composition differs from complete source replay')
