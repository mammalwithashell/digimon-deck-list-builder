"""Opt-in paired main/face correction + full Rule/condition/runtime composition.

An externally reviewed correction sidecar and independently verified original
record extraction remain prerequisites. No candidate admission/book/verdicts.
"""
from copy import deepcopy
import re
from pathlib import Path
import ast
import importlib.util
from tools.card_data import generate,registry,serialization,complete_candidate,corrections
from . import runtime_projection_proof as rp
from .paired_main_correction_units import bind_paired_main_correction_unit,FIELD,FACE
from tools.card_data.compatibility_corrections import correction_document_from_bytes
from .source_units import _validated_context_core,_require,_json,_sha,SourceUnitError,_unsupported_ordinary_text
from .rule_source_units import _rule_word
from .rule_card_composition import _compose_rule_context
from .text_split import split_clauses

enc=serialization.canonical_json_bytes
_SOURCE_KEYS={'aggregate','scopes','observations','canonical_card','adjudication_ledger',
 'raw_pages','correction_document','condition_selections','old_input_revision',
 'new_input_revision','expected_official_universe'}



def checker_dependencies():
    """Exact local static import closure, including this entrypoint and dispatchers."""
    root=Path(__file__).resolve().parents[2]
    pending=[__name__];result={}
    while pending:
        name=pending.pop()
        if name in result or not name.startswith('tools.'):continue
        path=root/Path(*name.split('.'))
        path=path.with_suffix('.py') if path.with_suffix('.py').is_file() else path/'__init__.py'
        if not path.is_file():continue
        raw=path.read_bytes();result[name]=_sha(raw)
        package=name if path.name=='__init__.py'else name.rpartition('.')[0]
        for node in ast.walk(ast.parse(raw.decode('utf-8-sig'))):
            if isinstance(node,ast.Import):pending.extend(x.name for x in node.names)
            elif isinstance(node,ast.ImportFrom):
                base=importlib.util.resolve_name('.'*node.level+(node.module or ''),package)if node.level else node.module
                if base:
                    pending.append(base);pending.extend(base+'.'+x.name for x in node.names)
    return dict(sorted(result.items()))


def compose_paired_main_card(*,runtime_record,registry_bytes,projection_binding,
        correction_sidecar,expected_reviewed_sidecar_sha256,correction_images,
        image_artifacts,**inputs):
    """Recompute every proof internally; no caller context or private flags accepted."""
    supplied=locals().copy();supplied.pop('inputs');supplied.update(inputs)
    try:
        _require(set(inputs)==_SOURCE_KEYS,'exact original source inputs required')
        return _compose(supplied,inputs)
    except SourceUnitError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError) as e:
        raise SourceUnitError('corrected composition refused: '+str(e)) from e


def _compose(a,inputs):
    pin=_json(a['projection_binding']);pinned={k:v for k,v in a.items()if k!='projection_binding'}
    _require(set(pin)=={'schema_version','input_sha256','producer_sha256','checker_sha256'} and
        type(pin['schema_version'])is int and pin['schema_version']==1 and
        pin['input_sha256']==rp.input_hashes(pinned) and pin['producer_sha256']==rp.producer_dependencies()
        and pin['checker_sha256']==checker_dependencies(),
        'exact original input and producer pins required')
    correction_args={k:v for k,v in inputs.items()if k!='condition_selections'}
    correction_args.update(runtime_card=a['runtime_record'],sidecar=a['correction_sidecar'],
        expected_reviewed_sidecar_sha256=a['expected_reviewed_sidecar_sha256'],images=a['correction_images'])
    correction=bind_paired_main_correction_unit(**correction_args)
    rp._strict_ledger(inputs['adjudication_ledger'])
    raw=_json(inputs['canonical_card']);cid=raw['card_id']
    card=complete_candidate._canonical_card_from_payload(cid,raw,path=Path('<original-corrected-record>'))
    runtime=_json(a['runtime_record'])
    _require(len(card.faces)==1 and not runtime.get('dual') and not runtime.get('link_effect_description_eng')
        and not card.fields.get('link_effect_description_eng'),'unsupported corrected Dual/Link')
    reg_raw=_json(a['registry_bytes'])
    _require(set(reg_raw)=={'schema_version','registry_version','capacity','padding_index','assignment_count','assignments'}
        and all(type(reg_raw.get(k))is int for k in ('schema_version','registry_version','capacity','padding_index','assignment_count')),
        'exact registry envelope and strict numeric headers')
    reg=registry.StableCardRegistry.from_dict(reg_raw)
    _require(set(inputs['expected_official_universe'])<=set(reg.assignments),'registry omits official members')
    projected=generate.project_canonical_card(card)
    expected={k:projected[k]for k in rp.FIELDS}
    expected.update(registry.compatibility_values(cid,reg.assignments[cid],capacity=reg.capacity))
    from .source_binding import bind_condition_units
    conditions=bind_condition_units(aggregate=inputs['aggregate'],observations=inputs['observations'],
        canonical_card=inputs['canonical_card'],runtime_projection=enc({'card_id':cid,'xros_req':runtime['xros_req']}),
        selection_ledger=inputs['condition_selections'],correction_document=inputs['correction_document'],
        image_artifacts=a['image_artifacts'],raw_pages=inputs['raw_pages'],
        old_input_revision=inputs['old_input_revision'],new_input_revision=inputs['new_input_revision'])
    expected['xros_req']=conditions['projection']
    _require(all(k in runtime and enc(runtime[k])==enc(v)for k,v in expected.items()),'exact corrected runtime producer mismatch')
    _require(set(runtime)-set(expected)<={'rarity','card_name_jpn'},
        'unclaimed gameplay runtime fields require a separately reviewed capability')
    view={'card_id':cid,**dict(card.fields),'xros_req':conditions['projection']}
    source_inputs={**inputs,'runtime_projection':enc(view)}
    # Only this entrypoint constructs the private callback after an exact proof.
    # Reapply both typed rows to the core's independently reconstructed original
    # card; compare the full original record, not only faces or selected fields.
    def validate_corrected_record(*,card,replayed_card,typed):
        document=correction_document_from_bytes(inputs['correction_document'])
        rows=[r for r in document.corrections if r.card_id==cid]
        _require(len(rows)==2 and {r.field_path for r in rows}=={FIELD,FACE},'exact paired face correction required')
        corrected=replayed_card
        for row in sorted(rows,key=lambda r:r.field_path):
            corrected=corrections.apply_correction(corrected,row,observations=typed)
        _require(enc(raw)==enc(corrected) and enc(card)==enc(corrected),
            'private paired full-record replay differs from original canonical bytes')
    private_binding=deepcopy(correction)
    private_binding['original_provenance']=correction['original_provenance'][FIELD]
    context=_validated_context_core(**source_inputs,_correction_binding=private_binding,
        _face_validator=validate_corrected_record)
    # The proof above establishes exactly the paired same-card corrections and its full
    # document. Unknown sibling structure blockers remain untouched.
    context['result']['blockers']=[b for b in context['result']['blockers']if b['code']!='UNSUPPORTED_CORRECTION']
    corrected_text=correction['unit']['value']
    if (_rule_word(corrected_text) or _unsupported_ordinary_text(corrected_text)
        or re.search(r'[\[<\uff1c](?:Assembly|DigiXros|DNA Digivol(?:ve|ution)|Digivolve|App Fusion)\b',corrected_text,re.I)):
        context['result']['blockers'].append({'code':'UNSUPPORTED_CORRECTED_FIELD_STRUCTURE',
            'field_path':FIELD,'text':corrected_text})
    result=_compose_rule_context(context,image_artifacts=a['image_artifacts'],inputs=source_inputs)
    result['correction_binding']=correction
    result['runtime_projection']={'status':'EXACT_CORRECTED_RUNTIME_PROJECTION','card_id':cid,
        'claimed_fields':list(expected),'projected_claimed_values':expected,
        'original_runtime_bytes':a['runtime_record'].decode(),'original_canonical_bytes':inputs['canonical_card'].decode(),
        'unclaimed_runtime_fields':{k:v for k,v in runtime.items()if k not in expected},
        'derived_canonical_view':view,'input_sha256':pin['input_sha256'],
        'producer_sha256':pin['producer_sha256'],'checker_sha256':pin['checker_sha256'],'projection_binding_sha256':_sha(a['projection_binding']),
        'limit':'19 claimed serialization fields only; independently verified extraction required; no admission'}
    result['checker_sha256']=_sha(Path(__file__).read_bytes())
    if result['status']!='COMPOSED_RULE_CARD':
        _require(checker_dependencies()==pin['checker_sha256'],'checker drift during paired composition')
        return result
    index=len(result['units']);unit={**deepcopy(correction['unit']),'projected_text':correction['unit']['value'],
        'transformation':'reviewed-paired-main-image-correction','sidecar_sha256':correction['sidecar_sha256']}
    result['units'].append(unit)
    spans=[{'unit_index':index,'span_index':i,'kind':s.kind,'timings':list(s.timings),'keyword':s.keyword,'text':s.text}
           for i,s in enumerate(split_clauses(unit['value']))]
    result['zones']['effect']={'field':FIELD,'value':unit['value'],'source_binding':correction,
        'unit_indices':[index],'presence':'reviewed-correction','spans':spans}
    original_keys={(s['observation_id'],s['section_index'])for s in correction['original_sections']}
    for row in result['consumption']:
        if (row['observation_id'],row['section_index']) in original_keys:
            _require(not row['unit_indices'],'original corrected container must not execute twice')
            row.update(disposition='superseded-by-reviewed-correction',replacement_unit_indices=[index],
                reason='original official container retained; reviewed image correction supplies exact main value',
                consumption_purposes=['retained-original-correction-evidence'])
    _require(len([r for r in result['consumption']if r['disposition']=='superseded-by-reviewed-correction'])==len(original_keys),
        'corrected original container conservation mismatch')
    result['structural_obligations'].append({**deepcopy(correction['face_mirror_obligation']),
        'kind':'paired-main-face-consistency','source_unit_index':index,
        'correction_sidecar_sha256':correction['sidecar_sha256']})
    _require(checker_dependencies()==pin['checker_sha256'],'checker drift during paired composition')
    result['status']='COMPOSED_PAIRED_MAIN_CARD'
    result['kind']='paired-main-rule-aware-whole-card-source-composition'
    return result


def validate_paired_main_card(binding,**inputs):
    _require(enc(binding)==enc(compose_paired_main_card(**inputs)),'corrected composition differs from full replay')
