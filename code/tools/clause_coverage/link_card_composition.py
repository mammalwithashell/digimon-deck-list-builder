"""Inactive exact Link source/runtime composition; no book or admission writes.

The sealed snapshot is the authority prerequisite. Every original observation
remains present. This narrow capability supports unanimous ordinary conditions
and the two reviewed Link layouts; it does not broaden public A2 label support.
"""
from copy import deepcopy
from pathlib import Path
import ast
import importlib.util

from tools.card_data import generate, registry, normalization, models, serialization
from tools.card_data import compatibility_conditions, compatibility_corrections, compatibility_selections
from tools.card_data.errors import CardDataError
from . import source_units as units
from .runtime_projection_proof import FIELDS, _strict_ledger, _json, _sha, _require
from .runtime_rule_composition import _verify_snapshot_dispatch
from .link_source_units import derive_link_occurrences
from .link_yaml_binding import bind_link_yaml
from .condition_app_fusion import split_app_fusion_condition_units
from .text_split import split_clauses

enc = serialization.canonical_json_bytes


def link_dependencies():
    """Exact transitive local checker/producer closure, checked at both ends."""
    root = Path(__file__).resolve().parents[2]
    pending = [__name__]; result = {}
    while pending:
        name = pending.pop()
        if name in result or not (name == 'tools' or name.startswith('tools.')):
            continue
        path = root / Path(*name.split('.'))
        path = path.with_suffix('.py') if path.with_suffix('.py').is_file() else path / '__init__.py'
        if not path.is_file():
            continue
        raw = path.read_bytes(); result[name] = _sha(raw)
        package = name if path.name == '__init__.py' else name.rpartition('.')[0]
        pending.extend('.'.join(name.split('.')[:i]) for i in range(1, len(name.split('.'))))
        for node in ast.walk(ast.parse(raw)):
            if isinstance(node, ast.Import):
                pending.extend(x.name for x in node.names)
            elif isinstance(node, ast.ImportFrom):
                base = importlib.util.resolve_name('.' * node.level + (node.module or ''), package) if node.level else node.module
                if base:
                    pending.append(base); pending.extend(base + '.' + x.name for x in node.names)
    return dict(sorted(result.items()))


def compose_link_card(*, snapshot, expected_extraction_sha256, runtime_record,
                      registry_bytes, yaml_bytes, expected_yaml_sha256, **source_inputs):
    """Replay original source and twenty consumed runtime fields; no proof flags."""
    try:
        return _compose(snapshot, expected_extraction_sha256, runtime_record,
                        registry_bytes, yaml_bytes, expected_yaml_sha256, source_inputs)
    except (ValueError, TypeError, KeyError, AttributeError, IndexError, CardDataError) as error:
        raise units.SourceUnitError('Link composition refused: ' + str(error)) from error


def _compose(snapshot, extraction, raw_runtime, raw_registry, yaml_raw, yaml_pin, inputs):
    dependencies = link_dependencies()
    meta, cid = _verify_snapshot_dispatch(snapshot=snapshot, expected_extraction_sha256=extraction,
        runtime_record=raw_runtime, registry_bytes=raw_registry, source_inputs=inputs)
    _require(type(yaml_raw) is bytes and type(yaml_pin) is str and _sha(yaml_raw) == yaml_pin,
             'exact YAML byte pin required')
    ledger = _strict_ledger(inputs['adjudication_ledger'])
    runtime = _json(raw_runtime)
    reg = _json(raw_registry)
    _require(set(reg) == {'schema_version','registry_version','capacity','padding_index','assignment_count','assignments'}
        and all(type(reg[k]) is int for k in ('schema_version','registry_version','capacity','padding_index','assignment_count')),
        'strict registry envelope required')
    reg = registry.StableCardRegistry.from_dict(reg)
    _require(set(inputs['expected_official_universe']) <= set(reg.assignments), 'registry missing official members')
    corrections = compatibility_corrections.correction_document_from_bytes(inputs['correction_document'])
    selections = compatibility_selections.selections_from_bytes(inputs['condition_selections'])
    _require(not any(r.card_id == cid for r in (*corrections.corrections,*corrections.compatibility_corrections,*selections)),
             'Link target corrections or condition selections unsupported')
    expected = {}; condition = {}; conflict_review = []

    def verify_runtime(*, card, runtime, typed, kind):
        _require(len(card.faces) == 1 and not runtime.get('dual') and kind == 'Digimon', 'single Digimon Link layout required')
        replay = normalization.normalize_card(cid, typed.values(), adjudications=ledger,
            old_input_revision=inputs['old_input_revision'], new_input_revision=inputs['new_input_revision'],
            field_selection_input_revision=inputs['new_input_revision'])
        _require(enc(card.fields) == enc(replay.fields) and enc(card.provenance) == enc(replay.provenance),
                 'complete canonical fields/provenance differ from original producer replay')
        # Conflicts are retained historical evidence, including reviewed ones.
        # Only ordinary text conflicts handled again by the complete A1 binding
        # below may proceed with an exact current candidate-set approval.
        conflict_review.extend(normalization.canonical_conflict_review_rows(
            {cid: replay}, typed.values(), adjudications=ledger,
            input_revision=inputs['new_input_revision']))
        for conflict in replay.conflicts:
            if conflict.requires_adjudication(models.AdmissionStatus.ADMITTED):
                reviews = [row for row in conflict_review if row['field_path'] == conflict.field_path]
                _require(conflict.field_path in units._FIELDS and len(reviews) == 1
                    and reviews[0]['approval_required'] is True and reviews[0]['status'] == 'current',
                    'required canonical source conflict remains')
        projected = generate.project_canonical_card(card)
        expected.update({k:projected[k] for k in (*FIELDS,'link_effect_description_eng')})
        expected.update(registry.compatibility_values(cid, reg.assignments[cid], capacity=reg.capacity))
        value, ids = compatibility_conditions.official_condition_projection(card, typed,
            selections=selections, source_manifest_revision=inputs['new_input_revision'])
        officials = {oid:o for oid,o in typed.items() if o.source is models.SourceName.OFFICIAL}
        _require(set(ids) == set(officials) and bool(ids), 'all official condition sources required')
        contributions = {oid:'\n'.join(f"[{s['label']}] {s['text']}".strip()
            for s in o.extracted['text_sections'] if s['label'].lower() in compatibility_conditions._CONDITION_LABELS)
            for oid,o in officials.items()}
        _require(set(contributions.values()) == {value}, 'unanimous exact conditions required')
        condition.update(projection=value, candidate_values=contributions, evidence_observation_ids=list(ids),
            a2_section_coverage_claimed=False, producer='official_condition_projection', source_revision=inputs['new_input_revision'])
        expected['xros_req'] = value
        for field, wanted in expected.items():
            _require(field in runtime and enc(runtime[field]) == enc(wanted), 'exact runtime mismatch: ' + field)
        _require(set(runtime)-set(expected) <= {'rarity','card_name_jpn'}, 'unclaimed gameplay fields remain')

    context = units._validated_context_core(**inputs, runtime_projection=raw_runtime, _runtime_validator=verify_runtime)
    result = deepcopy(context['result']); typed = context['typed']; card = context['card']
    diagnostics = {}; yaml_bindings = {}; discharged = set(); obligations = []
    for oid, observation in typed.items():
        if observation.source is not models.SourceName.OFFICIAL:
            continue
        diagnostic = derive_link_occurrences(_json(inputs['observations'][oid]))
        _require(diagnostic['status'] == 'DERIVED_LINK_DIAGNOSTIC', 'unsupported complete Link printing')
        diagnostics[oid] = diagnostic
        binding = bind_link_yaml(card_id=cid, observation_bytes=inputs['observations'][oid],
            expected_observation_sha256=_sha(inputs['observations'][oid]), yaml_bytes=yaml_raw, expected_yaml_sha256=yaml_pin)
        _require(binding['status'] == 'BOUND_LINK_YAML_DIAGNOSTIC', 'Link YAML cost/DP binding refused')
        yaml_bindings[oid] = binding
        for occurrence in diagnostic['occurrences']:
            discharged.add((oid, occurrence['section_index']))
            if occurrence['role'] != 'effect':
                obligations.append({'observation_id':oid,'printing_id':normalization.observation_candidate(observation).printing_id, **deepcopy(occurrence)})
    pending = []
    # Ordinary composition may retain unsupported *unselected* alternatives.
    # This capability must account for every printing's structural siblings,
    # even when a different printing was chosen for the canonical text.
    for row in result['inventory']:
        key = (row['observation_id'],row['section_index'])
        if key not in discharged and row['label'].lower() not in (set(units._LABELS) | set(compatibility_conditions._CONDITION_LABELS)):
            pending.append({'code':'UNSUPPORTED_LINK_SIBLING',**row})
    for b in result['blockers']:
        key = (b.get('observation_id'),b.get('section_index'))
        if b['code'] == 'UNSUPPORTED_STRUCTURAL_PROJECTION':
            continue  # single face, exact condition + nonempty Link already proven
        if b['code'] in {'UNSUPPORTED_SECTION','UNSUPPORTED_RULE_OR_LINK'} and key in discharged:
            continue
        if b['code'] == 'UNSUPPORTED_SECTION' and b.get('label','').lower() in compatibility_conditions._CONDITION_LABELS:
            continue
        pending.append(b)
    result['blockers'] = pending
    result.update(kind='exact-link-whole-card-composition-v1', status='BLOCKED', snapshot=meta,
        original_canonical_bytes=inputs['canonical_card'].decode('utf8'), original_runtime_bytes=raw_runtime.decode('utf8'),
        claimed_fields=list(expected), projected_claimed_values=expected,
        unclaimed_runtime_fields={k:v for k,v in runtime.items() if k not in expected},
        condition_binding=condition, canonical_conflict_review=conflict_review,
        link_derivations=diagnostics, yaml_bindings=yaml_bindings,
        structural_obligations=obligations, native_compilation_verified=False,
        dependency_sha256=dependencies, consumption=[], limit='Offline source/runtime proof only; no compiler, book, admission, or training approval')
    result['input_sha256'].update(registry=_sha(raw_registry), yaml=yaml_pin, extraction=extraction)
    if not pending:
        _project_zones(result, context, diagnostics, condition)
        result['status'] = 'COMPOSED_LINK_CARD'
    else:
        result['consumption'] = [{**row,'disposition':'blocked','unit_indices':[]} for row in result['inventory']]
    _require(link_dependencies() == dependencies, 'checker dependencies changed during replay')
    return result


def _project_zones(result, context, diagnostics, condition):
    typed = context['typed']; card = context['card']; result['units'] = []; result['zones'] = {}
    fields = {**units._FIELDS,'link_effect_description_eng':'link'}
    for field, zone in fields.items():
        provenance = card.provenance[field]; oid = provenance.selected_observation_id
        _require(oid in diagnostics and provenance.authority is models.AuthorityClass.OFFICIAL and not provenance.derived,
                 'selected Link zone must retain official provenance')
        sections = typed[oid].extracted['text_sections']; pieces = []
        for index, section in enumerate(sections):
            label,text = section['label'],section['text']
            link = normalization._link_section_effect(label,text)
            if link is not None:
                mapped = 'link_effect_description_eng'; rendered = link
            elif label.lower() in compatibility_conditions._CONDITION_LABELS or label in ('Link Condition','Link DP'):
                continue
            else:
                mapped = 'security_effect_description_eng' if 'security' in label.lower() else 'inherited_effect_description_eng' if 'inherited' in label.lower() else 'effect_description_eng'
                rendered = normalization._render_effect_section(label,text)
            if mapped == field and rendered:
                pieces.append((index,rendered))
        _require('\n'.join(t for _,t in pieces) == card.fields[field], 'lossless Link zone projection mismatch: '+field)
        indices = []; spans = []
        for index,text in pieces:
            original = next(r for r in result['inventory'] if r['observation_id']==oid and r['section_index']==index)
            number = len(result['units']); indices.append(number)
            role = 'folded-link-dp' if any(o['role']=='dp' and o['section_index']==index for o in diagnostics[oid]['occurrences']) else 'effect'
            result['units'].append({**deepcopy(original),'field':field,'projected_text':text,'transformation':'producer-link-zone','role':role})
            if role != 'folded-link-dp':
                spans.extend({'unit_index':number,'span_index':i,'kind':s.kind,'text':s.text,'timings':list(s.timings),'keyword':s.keyword}
                             for i,s in enumerate(split_clauses(text)))
        result['zones'][zone] = {'field':field,'value':card.fields[field],'source_binding':serialization.canonicalize(provenance),
            'unit_indices':indices,'spans':spans,'presence':'nonempty' if pieces else 'projected-empty-box' if zone=='inherited' else 'absent-box'}
    oid = sorted(condition['evidence_observation_ids'])[0]
    selected = [{**row,'kind':'official-section','field':'xros_req','condition_type':row['label'].lower().replace(' ','_')} for row in result['inventory']
                if row['observation_id']==oid and row['label'].lower() in compatibility_conditions._CONDITION_LABELS]
    _require('\n'.join(f"[{u['label']}] {u['text']}".strip() for u in selected)==condition['projection'], 'condition occurrence closure')
    indices=[]; spans=[]
    for unit,split in zip(selected,split_app_fusion_condition_units(selected),strict=True):
        number=len(result['units']);indices.append(number)
        result['units'].append({**unit,'projected_text':unit['text'],'role':'condition','transformation':'unanimous-producer-condition'})
        spans.extend({'unit_index':number,**s} for s in split['spans'])
    result['zones']['conditions']={'field':'xros_req','value':condition['projection'],'unit_indices':indices,'spans':spans,'source_binding':condition}
    for row in result['inventory']:
        key=(row['observation_id'],row['section_index'])
        refs=[i for i,u in enumerate(result['units']) if (u['observation_id'],u['section_index'])==key]
        structural=[i for i,u in enumerate(result['structural_obligations']) if (u['observation_id'],u['section_index'])==key]
        result['consumption'].append({**deepcopy(row),'unit_indices':refs,'structural_obligation_indices':structural,
            'disposition':'consumed' if refs or structural else 'retained-alternative'})


def validate_link_card(result, **inputs):
    _require(enc(result) == enc(compose_link_card(**inputs)), 'Link composition differs from full replay')
