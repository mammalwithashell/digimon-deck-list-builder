"""Exact claimed-field forward runtime proof, not whole-card source readiness.

Inputs and producer pins require an independently verified extraction artifact
pairing these exact records with the original full candidate file hashes. This
helper does not verify full-container extraction or admission. It proves the
supplied canonical record's serialization, not invented canonical authority.
No bootstrap is inferred from runtime values; compatibility metadata is unclaimed.
"""
from pathlib import Path
import ast
import hashlib
import importlib.util
import json

from tools.card_data import generate, registry, serialization, complete_candidate
from tools.card_data import compatibility_corrections, adjudications
from tools.card_data.errors import CardDataError
from .source_binding import bind_condition_units

enc = serialization.canonical_json_bytes
VERSION = 'exact-claimed-runtime-projection-v1'
FIELDS = ('card_id','card_name_eng','card_effect_class_name','play_cost','dp','level',
          'card_kind','card_colors','type_eng','form_eng','attribute_eng',
          'effect_description_eng','inherited_effect_description_eng',
          'security_effect_description_eng','evo_costs')
TEXT = {'effect_description_eng','inherited_effect_description_eng','security_effect_description_eng'}


class RuntimeProjectionError(ValueError):
    pass


def _require(value,message):
    if not value: raise RuntimeProjectionError(message)


def _sha(value): return hashlib.sha256(value).hexdigest()


def _unique(pairs):
    d={}
    for k,v in pairs:
        _require(k not in d,'duplicate JSON field');d[k]=v
    return d


def _json(raw):
    _require(isinstance(raw,bytes),'input must be original bytes')
    return json.loads(raw.decode('utf-8'),object_pairs_hook=_unique)


def producer_dependencies():
    """Pin local static import closure, including function-local producer imports."""
    root=Path(__file__).resolve().parents[2]
    pending=['tools.card_data.generate','tools.card_data.registry','tools.card_data.complete_candidate',
             'tools.card_data.compatibility_corrections','tools.card_data.adjudications',
             'tools.clause_coverage.source_binding']
    result={}
    while pending:
        name=pending.pop()
        if name in result or not name.startswith('tools.'):continue
        p=root/Path(*name.split('.'))
        p=p.with_suffix('.py') if p.with_suffix('.py').is_file() else p/'__init__.py'
        if not p.is_file():continue
        raw=p.read_bytes();result[name]=_sha(raw)
        package=name if p.name=='__init__.py' else name.rpartition('.')[0]
        for node in ast.walk(ast.parse(raw.decode('utf-8-sig'))):
            if isinstance(node,ast.Import):pending.extend(x.name for x in node.names)
            elif isinstance(node,ast.ImportFrom):
                base=importlib.util.resolve_name('.'*node.level+(node.module or ''),package) if node.level else node.module
                if base:
                    pending.append(base);pending.extend(base+'.'+x.name for x in node.names)
    return dict(sorted(result.items()))


def input_hashes(inputs):
    def hashed(v):
        if isinstance(v,bytes):return _sha(v)
        if isinstance(v,dict):return {k:hashed(x)for k,x in sorted(v.items())}
        return _sha(enc(v))
    return {k:hashed(v)for k,v in sorted(inputs.items())if k!='binding'}


def _strict_ledger(raw):
    """Validate every wire row before the legacy loader can coerce scalars."""
    ledger=_json(raw)
    required={'schema_version','adjudication_id','kind','card_id','old_input_revision',
              'new_input_revision','old_digest','new_digest','reason','reviewer','reviewed_at'}
    optional={'field_path','selected_observation_id','provenance_observation_ids',
              'affected_deck_ids','old_value','new_value'}
    _require(set(ledger)=={'schema_version','adjudications'} and type(ledger['schema_version'])is int
             and ledger['schema_version']==1 and isinstance(ledger['adjudications'],list),
             'invalid whole review ledger')
    for row in ledger['adjudications']:
        _require(isinstance(row,dict) and required<=set(row)<=required|optional,
                 'invalid adjudication row fields')
        _require(type(row['schema_version'])is int and row['schema_version']==1,
                 'invalid adjudication row version')
        for name in required-{'schema_version'}:
            _require(isinstance(row[name],str) and bool(row[name].strip()),
                     'invalid adjudication scalar: '+name)
        for name in ('field_path','selected_observation_id'):
            _require(row.get(name)is None or isinstance(row[name],str),
                     'invalid adjudication optional scalar: '+name)
        for name in ('provenance_observation_ids','affected_deck_ids'):
            _require(isinstance(row.get(name,[]),list) and
                     all(isinstance(v,str)for v in row.get(name,[])),
                     'invalid adjudication array: '+name)
    # Typed model validation supplies enum, digest, field-path, membership and ID checks.
    return adjudications.validate_migration_adjudications(
        adjudications._row(r,path=Path('<pinned-ledger>'))for r in ledger['adjudications'])


def prove_runtime_projection(*,canonical_card,runtime_record,registry_bytes,aggregate,
        observations,correction_document,condition_selections,adjudication_ledger,
        raw_pages,image_artifacts,old_input_revision,new_input_revision,
        expected_official_universe,binding):
    supplied=locals().copy()
    try:return _prove(supplied)
    except RuntimeProjectionError:raise
    except (ValueError,TypeError,KeyError,AttributeError,IndexError,CardDataError) as e:
        raise RuntimeProjectionError('runtime projection refused: '+str(e)) from e


def _prove(a):
    pin=_json(a['binding'])
    _require(set(pin)=={'schema_version','input_sha256','producer_sha256'} and type(pin['schema_version'])is int
             and pin['schema_version']==1,'unsupported binding envelope')
    _require(pin['input_sha256']==input_hashes(a),'input bytes differ from reviewed pins')
    deps=producer_dependencies();_require(pin['producer_sha256']==deps,'producer dependency changed')
    canonical=_json(a['canonical_card']);runtime=_json(a['runtime_record']);cid=canonical['card_id']
    _require(runtime['card_id']==cid,'foreign runtime card')
    card=complete_candidate._canonical_card_from_payload(cid,canonical,path=Path('<pinned-original>'))
    _require(len(card.faces)==1 and not runtime.get('dual') and not runtime.get('link_effect_description_eng')
             and not card.fields.get('link_effect_description_eng'),'unsupported Dual/Link projection')
    _require(card.fields['card_kind'] in {'Digimon','Tamer','Option','Digi-Egg'},'unsupported primary kind')
    _require(type(runtime['card_kind'])is int,'runtime kind must be an integer enum')
    document=compatibility_corrections.correction_document_from_bytes(a['correction_document'])
    _require(not any(r.card_id==cid for r in (*document.corrections,*document.compatibility_corrections)),
             'target correction composition unsupported')
    _strict_ledger(a['adjudication_ledger'])
    agg=_json(a['aggregate']);universe=a['expected_official_universe']
    _require(isinstance(universe,list) and len(universe)==len(set(universe))
             and set(universe)==set(agg['sources']['official']['gameplay_card_ids']), 'full official universe mismatch')
    reg_raw=_json(a['registry_bytes'])
    _require(isinstance(reg_raw,dict) and set(reg_raw)=={'schema_version','registry_version',
             'capacity','padding_index','assignment_count','assignments'},'invalid registry envelope')
    _require(all(type(reg_raw[name])is int for name in ('schema_version','registry_version',
             'capacity','padding_index','assignment_count')),'invalid registry numeric headers')
    reg=registry.StableCardRegistry.from_dict(reg_raw)
    _require(set(universe)<=set(reg.assignments),'registry omits official members')
    projected=generate.project_canonical_card(card)
    expected={k:projected[k]for k in FIELDS}
    expected.update(registry.compatibility_values(cid,reg.assignments[cid],capacity=reg.capacity))
    condition=bind_condition_units(aggregate=a['aggregate'],observations=a['observations'],
        canonical_card=a['canonical_card'],runtime_projection=enc({'card_id':cid,'xros_req':runtime['xros_req']}),
        selection_ledger=a['condition_selections'],correction_document=a['correction_document'],
        image_artifacts=a['image_artifacts'],raw_pages=a['raw_pages'],
        old_input_revision=a['old_input_revision'],new_input_revision=a['new_input_revision'])
    expected['xros_req']=condition['projection']
    for field,value in expected.items():
        _require(field in runtime and enc(runtime[field])==enc(value),'exact producer mismatch: '+field)
    view={'card_id':cid,**dict(card.fields),'xros_req':condition['projection']}
    transforms={field:('generate._legacy_text'if field in TEXT else
        'generate._legacy_kind'if field=='card_kind'else 'A2-condition-source'if field=='xros_req'else
        'registry.compatibility_values'if field in {'index','norm_id','card_index'}else 'generate.project_canonical_card')
        for field in expected}
    return {'version':VERSION,'status':'EXACT_CLAIMED_RUNTIME_PROJECTION','card_id':cid,
        'original_canonical_bytes':a['canonical_card'].decode('utf-8'),'original_runtime_bytes':a['runtime_record'].decode('utf-8'),
        'input_sha256':pin['input_sha256'],'binding_sha256':_sha(a['binding']),
        'producer_sha256':deps,'helper_sha256':_sha(Path(__file__).read_bytes()),
        'claimed_fields':list(expected),'projected_claimed_values':expected,
        'unclaimed_runtime_fields':{k:v for k,v in runtime.items()if k not in expected},
        'derived_canonical_view':view,'field_transformations':transforms,'condition_binding':condition,
        'extraction_prerequisite':'Independently verified exact-record extraction artifact and full candidate file hashes',
        'limit':'Serialization of claimed fields only; no full-container admission; separate source composition and adoption required'}


def validate_runtime_projection(proof,**inputs):
    _require(enc(proof)==enc(prove_runtime_projection(**inputs)),'submitted runtime proof differs from complete replay')
