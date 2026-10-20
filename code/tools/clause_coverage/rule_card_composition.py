"""Opt-in Rule trait binding plus ordinary/condition composition.

Sealed reviewed observations are required. Raw hash pairing is not independent
HTML/image authentication. Rule obligations retain original text and offsets;
they are structural trait derivations, never additional executable clauses.
"""
from copy import deepcopy

from tools.card_data import normalization,serialization,models
from tools.card_data.errors import CardDataError
from .rule_source_units import derive_rule_occurrences
from .source_binding import bind_canonical_field
from .source_units import (_validated_context,_ledger,_json,_require,
    _unsupported_ordinary_text,SourceUnitError)
from .source_composition import _compose_context

_TRAITS=('form_eng','type_eng','attribute_eng')


def compose_rule_card(*,image_artifacts,**inputs):
    """Replay all raw inputs and all three trait fields, or retain blockers."""
    try:
        return _compose(image_artifacts=image_artifacts,inputs=inputs)
    except SourceUnitError:
        raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as error:
        raise SourceUnitError('Rule card composition refused: '+str(error)) from error


def _compose(*,image_artifacts,inputs):
    return _compose_rule_context(_validated_context(**inputs),image_artifacts=image_artifacts,inputs=inputs)


def _compose_rule_context(context,*,image_artifacts,inputs):
    result=context['result'];cid=result['card_id'];runtime=context['runtime']
    record=_json(inputs['canonical_card']);ledger=_ledger(inputs['adjudication_ledger'])
    candidates=[normalization.observation_candidate(o) for o in context['typed'].values()]
    derivations={};obligations=[];discharged_sections=set();rule_failures=[]
    for oid,o in sorted(context['typed'].items()):
        if o.source is not models.SourceName.OFFICIAL:
            continue
        raw=_json(inputs['observations'][oid]);d=derive_rule_occurrences(raw)
        derivations[oid]=d
        if d['blockers']:
            rule_failures.append({'code':'UNSUPPORTED_RULE_DERIVATION','observation_id':oid,
                                  'details':deepcopy(d['blockers'])})
            continue
        candidate=next(c for c in candidates if c.observation.observation_id==oid)
        _require(all(serialization.canonical_json_bytes(d['projected_fields'][f])==
                     serialization.canonical_json_bytes(candidate.fields[f]) for f in _TRAITS),
                 'Rule derivation differs from official producer trait projection')
        for occurrence in d['occurrences']:
            obligations.append({**deepcopy(occurrence),'observation_id':oid,
                'printing_id':candidate.printing_id,'source_url':o.source_url,
                'raw_payload_digest':o.raw_payload_digest,'purpose':'trait-derivation-only',
                'executable':False})
        for s in d['section_inventory']:
            occurrences=[r for r in d['occurrences'] if r['section_index']==s['section_index']]
            if not occurrences:
                continue
            # Inspect residual text only to prove that this exact blocker has
            # no unhandled Rule/Link sibling. Original bytes are never edited.
            residual=s['text']
            for r in sorted(occurrences,key=lambda x:x['char_start'],reverse=True):
                residual=residual[:r['char_start']]+residual[r['char_end']:]
            if not _unsupported_ordinary_text(residual):
                discharged_sections.add((oid,s['section_index']))
    bindings={}
    for field in _TRAITS:
        value,provenance,conflicts=normalization._select_field(cid,field,
            [c for c in candidates if field in c.fields],admission=models.AdmissionStatus.ADMITTED,
            adjudications=ledger,field_selection_input_revision=inputs['new_input_revision'])
        _require(field in runtime and serialization.canonical_json_bytes(record['fields'][field])==
                 serialization.canonical_json_bytes(value)==serialization.canonical_json_bytes(runtime[field])
                 and serialization.canonical_json_bytes(record['provenance'][field])==
                     serialization.canonical_json_bytes(provenance),
                 'canonical trait/provenance/runtime differs from producer replay: '+field)
        _require(provenance.authority is models.AuthorityClass.OFFICIAL and not provenance.derived,
                 'trait binding requires selected official authority')
        binding={'selected_observation_id':provenance.selected_observation_id,
                 'provenance':serialization.canonicalize(provenance),'adjudication_id':None}
        if any(c.requires_adjudication(models.AdmissionStatus.ADMITTED) for c in conflicts):
            binding=bind_canonical_field(aggregate=inputs['aggregate'],observations=inputs['observations'],
                canonical_field=serialization.canonical_json_bytes({'card_id':cid,'field_path':field,
                    'value':value,'provenance':record['provenance'][field]}),
                adjudication_ledger=inputs['adjudication_ledger'],old_input_revision=inputs['old_input_revision'],
                new_input_revision=inputs['new_input_revision'])
        _require(binding['selected_observation_id'] in derivations,'trait selected source missing Rule replay')
        bindings[field]={'value':deepcopy(value),'source_binding':binding}
    retained=[]
    for b in result['blockers']:
        oid=b.get('observation_id');d=derivations.get(oid)
        if d and not d['blockers']:
            if b['code']=='UNSUPPORTED_RULE_GRANTS':
                continue
            if b['code']=='UNSUPPORTED_RULE_OR_LINK' and (oid,b.get('section_index')) in discharged_sections:
                continue
        retained.append(b)
    result['blockers']=retained+rule_failures
    result=_compose_context(context,image_artifacts=image_artifacts,inputs=inputs)
    result.update(kind='rule-aware-whole-card-source-composition',trait_bindings=bindings,
                  rule_derivations=derivations,structural_obligations=obligations)
    for row in result['consumption']:
        row['structural_obligation_indices']=[i for i,o in enumerate(obligations)
            if (o['observation_id'],o['section_index'])==(row['observation_id'],row['section_index'])]
        row['consumption_purposes']=(['ordinary-or-condition-source'] if row['unit_indices'] else [])+(
            ['trait-derivation-only'] if row['structural_obligation_indices'] else [])
    if result['status']=='COMPOSED_CARD':result['status']='COMPOSED_RULE_CARD'
    return result


def validate_rule_card(binding,**inputs):
    _require(serialization.canonical_json_bytes(binding)==serialization.canonical_json_bytes(compose_rule_card(**inputs)),
             'Rule composition differs from complete replay')

