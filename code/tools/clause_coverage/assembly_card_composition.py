"""Original snapshot/source composition plus an exact Assembly legacy field.

This diagnostic preserves source obligations and does not establish native
behavior, named-material kind closure, publication, or training admission.
"""
import ast
from copy import deepcopy
from pathlib import Path

from .assembly_recipe_binding import bind_assembly_recipe
from .runtime_projection_proof import _json, _require, _sha, enc
from .runtime_rule_composition import compose_runtime_rule_card


def assembly_dependencies():
    """Include the complete local static import closure, including private APIs."""
    root = Path(__file__).resolve().parents[2]
    pending = [__name__]
    result = {}
    while pending:
        name = pending.pop()
        if name in result or not (name == 'tools' or name.startswith('tools.')):
            continue
        path = root / Path(*name.split('.'))
        path = path.with_suffix('.py') if path.with_suffix('.py').is_file() else path / '__init__.py'
        if not path.is_file():
            continue
        raw = path.read_bytes()
        result[name] = _sha(raw)
        package = name if path.name == '__init__.py' else name.rpartition('.')[0]
        parts = name.split('.')
        pending.extend('.'.join(parts[:i]) for i in range(1, len(parts)))
        for node in ast.walk(ast.parse(raw)):
            if isinstance(node, ast.Import):
                pending.extend(a.name for a in node.names)
            elif isinstance(node, ast.ImportFrom):
                base = node.module or ''
                if node.level:
                    prefix = package.split('.')[:len(package.split('.')) - node.level + 1]
                    base = '.'.join(prefix + ([base] if base else []))
                pending.append(base)
                pending.extend(base + '.' + a.name for a in node.names if a.name != '*')
    return dict(sorted(result.items()))


def compose_assembly_card(*, snapshot, expected_extraction_sha256, runtime_record,
                          registry_bytes, runtime_binding, image_artifacts,
                          yaml_bytes, expected_yaml_sha256, **source_inputs):
    """Recompute original whole-card and recipe proofs; accept no proof flags."""
    dependencies = assembly_dependencies()
    ordinary = compose_runtime_rule_card(snapshot=snapshot,
        expected_extraction_sha256=expected_extraction_sha256,
        runtime_record=runtime_record, registry_bytes=registry_bytes,
        runtime_binding=runtime_binding, image_artifacts=image_artifacts, **source_inputs)
    runtime = _json(runtime_record)
    cid = ordinary['card_id']
    _require('digixros_costs' in runtime, 'original snapshot Assembly legacy field required')
    legacy = enc(runtime['digixros_costs'])
    condition_inputs = {k: source_inputs[k] for k in (
        'aggregate', 'observations', 'canonical_card', 'correction_document',
        'raw_pages', 'old_input_revision', 'new_input_revision')}
    condition_inputs.update(runtime_projection=enc({'card_id': cid, 'xros_req': runtime['xros_req']}),
        selection_ledger=source_inputs['condition_selections'], image_artifacts=image_artifacts)
    recipe = bind_assembly_recipe(condition_inputs=condition_inputs,
        legacy_field=legacy, expected_legacy_field_sha256=_sha(legacy),
        yaml_bytes=yaml_bytes, expected_yaml_sha256=expected_yaml_sha256)
    proof = ordinary['runtime_projection_proof']
    _require(recipe['card_id'] == cid and enc(recipe['condition_binding']) == enc(proof['condition_binding']),
             'recipe and runtime condition bindings differ')
    _require(len(proof['claimed_fields']) == 19 and 'digixros_costs' not in proof['claimed_fields'],
             'unsupported base runtime claim inventory')
    unclaimed = deepcopy(proof['unclaimed_runtime_fields'])
    _require('digixros_costs' in unclaimed, 'legacy field was not separately unclaimed')
    del unclaimed['digixros_costs']
    _require(set(unclaimed) <= {'rarity', 'card_name_jpn'}, 'unclaimed gameplay runtime fields remain')
    source = ordinary['source_composition']
    obligations = deepcopy(recipe['unresolved_obligations'])
    composed = ordinary['status'] == 'EXACT_RUNTIME_SOURCE_COMPOSITION' and source['status'] == 'COMPOSED_RULE_CARD'
    status = ('SOURCE_RUNTIME_COMPOSED_WITH_IMPLEMENTATION_OBLIGATIONS' if obligations else
              'SOURCE_RUNTIME_COMPOSED') if composed else 'BLOCKED'
    result = {'version': 1, 'kind': 'assembly-whole-card-composition-diagnostic',
        'card_id': cid, 'status': status, 'source_status': source['status'],
        'snapshot': ordinary['snapshot'],
        'original_canonical_bytes': ordinary['original_canonical_bytes'],
        'original_runtime_bytes': ordinary['original_runtime_bytes'],
        'claimed_fields': [*proof['claimed_fields'], 'digixros_costs'],
        'projected_claimed_values': {**deepcopy(proof['projected_claimed_values']),
                                     'digixros_costs': deepcopy(recipe['legacy_value'])},
        'unclaimed_runtime_fields': unclaimed,
        'ordinary_runtime_binding': ordinary, 'source_composition': deepcopy(source),
        'recipe_binding': recipe,
        'implementation_correspondence': {
            'status': 'BLOCKED' if obligations or not composed else 'STATIC_RECIPE_ONLY',
            'unresolved_obligations': obligations, 'native_compilation_verified': False,
            'unreviewed_alt_path_paths': recipe['unreviewed_alt_path_paths'],
            'unreviewed_effect_paths': recipe['unreviewed_effect_paths']},
        'dependency_sha256': dependencies, 'admission_performed': False,
        'limit': 'Original source and 20 serialized fields only; native behavior, other YAML effects, named-material kind closure, book and readiness are separate gates'}
    _require(assembly_dependencies() == dependencies, 'checker dependencies changed during replay')
    return result


def validate_assembly_card(result, **inputs):
    _require(enc(result) == enc(compose_assembly_card(**inputs)),
             'Assembly composition differs from complete original-input replay')
