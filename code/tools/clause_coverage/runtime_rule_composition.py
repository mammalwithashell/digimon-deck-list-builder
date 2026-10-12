"""Opt-in original-runtime proof followed by ordinary/condition/Rule replay.

The internally derived canonical view is never represented as actual runtime.
This capability claims 19 fields and preserves all unclaimed runtime metadata.
"""
from pathlib import Path
import ast

from .runtime_snapshot import VerifiedRuntimeSnapshot
from .runtime_projection_proof import prove_runtime_projection, _json, _sha, _require, enc
from .rule_card_composition import compose_rule_card

_SOURCE_KEYS=frozenset(('aggregate','scopes','observations','canonical_card','adjudication_ledger',
    'raw_pages','correction_document','condition_selections','old_input_revision',
    'new_input_revision','expected_official_universe'))


def wrapper_dependencies():
    """Static local import closure includes private and function-local producers."""
    root=Path(__file__).resolve().parents[2]
    pending=[__name__];result={}
    while pending:
        name=pending.pop()
        if name in result or not (name=='tools' or name.startswith('tools.')):continue
        path=root/Path(*name.split('.'))
        path=path.with_suffix('.py')if path.with_suffix('.py').is_file()else path/'__init__.py'
        if not path.is_file():continue
        raw=path.read_bytes();result[name]=_sha(raw)
        package=name if path.name=='__init__.py'else name.rpartition('.')[0]
        pieces=name.split('.')
        pending.extend('.'.join(pieces[:i])for i in range(1,len(pieces)))
        for node in ast.walk(ast.parse(raw)):
            if isinstance(node,ast.Import):pending.extend(a.name for a in node.names)
            elif isinstance(node,ast.ImportFrom):
                base=node.module or ''
                if node.level:
                    prefix=package.split('.')[:len(package.split('.'))-node.level+1]
                    base='.'.join(prefix+([base]if base else []))
                pending.append(base)
                pending.extend(base+'.'+a.name for a in node.names if a.name!='*')
    return dict(sorted(result.items()))


def _verify_snapshot_dispatch(*,snapshot,expected_extraction_sha256,runtime_record,
        registry_bytes,source_inputs):
    """Private exact original-byte dispatch checks; no proof flag accepted."""
    _require(type(snapshot)is VerifiedRuntimeSnapshot,'verified snapshot handle required')
    _require(set(source_inputs)==_SOURCE_KEYS,'unsupported or missing source input fields')
    meta=snapshot.metadata()
    _require(expected_extraction_sha256==meta['extraction_artifact_sha256'],
             'current dispatch snapshot configuration differs')
    canonical=_json(source_inputs['canonical_card']);cid=canonical['card_id'];record=snapshot.card(cid)
    _require(record['canonical_card']==source_inputs['canonical_card'] and
        record['runtime_record']==runtime_record and record['registry_bytes']==registry_bytes,
        'original card/registry bytes differ from verified snapshot')
    for argument,role in (('aggregate','aggregate'),('adjudication_ledger','adjudications'),
            ('correction_document','corrections'),('condition_selections','selections')):
        _require(source_inputs[argument]==snapshot.file(role),'dispatch source file differs: '+argument)
    scopes=source_inputs['scopes']
    _require(isinstance(scopes,dict) and set(scopes)=={'official','digimoncard_io'} and
        scopes['official']==snapshot.file('official_scope') and
        scopes['digimoncard_io']==snapshot.file('secondary_scope'),'dispatch scopes differ')
    _require(source_inputs['old_input_revision']==source_inputs['new_input_revision']==meta['source_revision'],
             'dispatch aggregate source revisions differ')
    _require(source_inputs['expected_official_universe']==meta['official_card_ids'],
             'dispatch full official universe differs')
    return meta,cid


def compose_runtime_rule_card(*,snapshot,expected_extraction_sha256,runtime_record,
        registry_bytes,runtime_binding,image_artifacts,**source_inputs):
    """Replay proof and public source composition; no submitted proof is trusted."""
    meta,cid=_verify_snapshot_dispatch(snapshot=snapshot,expected_extraction_sha256=expected_extraction_sha256,
        runtime_record=runtime_record,registry_bytes=registry_bytes,source_inputs=source_inputs)
    proof_inputs={k:v for k,v in source_inputs.items()if k!='scopes'}
    proof=prove_runtime_projection(**proof_inputs,runtime_record=runtime_record,
        registry_bytes=registry_bytes,binding=runtime_binding,image_artifacts=image_artifacts)
    view=enc(proof['derived_canonical_view'])
    source=compose_rule_card(**source_inputs,runtime_projection=view,image_artifacts=image_artifacts)
    status='EXACT_RUNTIME_SOURCE_COMPOSITION'if source['status']=='COMPOSED_RULE_CARD'else'BLOCKED'
    return {'schema_version':1,'kind':'exact-runtime-rule-wrapper-v1','status':status,'card_id':cid,
        'snapshot':meta,'original_canonical_bytes':source_inputs['canonical_card'].decode('utf8'),
        'original_runtime_bytes':runtime_record.decode('utf8'),
        'derived_canonical_view':proof['derived_canonical_view'],
        'derived_canonical_view_sha256':_sha(view),'field_transformations':proof['field_transformations'],
        'claimed_fields':proof['claimed_fields'],'unclaimed_runtime_fields':proof['unclaimed_runtime_fields'],
        'runtime_projection_proof':proof,'source_composition':source,
        'dependency_sha256':wrapper_dependencies(),
        'limit':'19 runtime fields plus separate source composition; unclaimed metadata, publication, book and readiness not approved'}


def validate_runtime_rule_card(result,**inputs):
    _require(enc(result)==enc(compose_runtime_rule_card(**inputs)),
             'runtime/source wrapper differs from complete replay')
