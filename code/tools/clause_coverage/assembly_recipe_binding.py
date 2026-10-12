"""Offline Assembly source/legacy/YAML correspondence for two narrow grammars.

The misleading legacy reduce_cost_per_card/name_contains keys are serialized
compatibility, never the Assembly semantic model. Full source authority,
whole-card composition, named-card kind closure and native execution remain
separate mandatory gates. No caller-supplied derived proof is accepted.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re

import yaml
from yaml.tokens import AliasToken, AnchorToken, TagToken

from tools import ingest_cards, xros_cost_parser
from tools.card_data import serialization
from . import source_binding, link_yaml_binding as lexical


class AssemblyBindingError(ValueError):
    pass


def _require(test, message):
    if not test:
        raise AssemblyBindingError(message)


def _recipe(text):
    _require(type(text) is str, 'Assembly text required')
    prefix = re.fullmatch(r'Assembly -([1-9][0-9]*): (.+)', text)
    _require(prefix is not None, 'unsupported complete Assembly grammar')
    cost, body = int(prefix[1]), prefix[2]
    _require(cost <= 65535, 'Assembly reduction exceeds native range')
    names = re.fullmatch(r'\[([^\[\]\r\n]+)\]×\[([^\[\]\r\n]+)\]', body)
    if names:
        _require(names[1] != names[2] and all(n.strip() == n and n for n in names.groups()),
                 'ambiguous or unsupported named materials')
        materials = [{'name_is': n, 'count': 1} for n in names.groups()]
        family = 'ordered_exact_names'
    else:
        branch = re.fullmatch(r'Lv\.([1-9][0-9]*) or lower Digimon card w/\[([^\[\]\r\n]+)\] in text or w/\[([^\[\]\r\n]+)\] trait', body)
        _require(branch is not None and int(branch[1]) <= 255,
                 'unsupported complete Assembly material grammar')
        _require(all(n.strip() == n and n for n in branch.groups()[1:]), 'unsupported qualifier spacing')
        materials = [{'count': 1, 'predicate': {'all_of': [
            {'kind': 'digimon'}, {'level_lte': int(branch[1])},
            {'any_of': [{'in_text_contains': branch[2]}, {'trait_has': branch[3]}]}]}}]
        family = 'bounded_digimon_text_or_trait'
    return {'family': family, 'total_reduction': cost, 'materials': materials,
            'source_zones': ['trash'], 'stack_under': True,
            'stack_order': 'left_to_right_top_to_bottom'}, f'[Assembly -{cost}] {body}'


def bind_assembly_recipe(*, condition_inputs, legacy_field, expected_legacy_field_sha256,
                         yaml_bytes, expected_yaml_sha256):
    """Recompute A2 and every supplied byte; return a diagnostic or refuse.

condition_inputs are the original complete public A2 inputs, including full
aggregate membership and all same-card observations. Its xros_req projection
stays exact; caller whole-card/runtime-snapshot checks remain mandatory.
"""
    try:
        return _bind(condition_inputs, legacy_field, expected_legacy_field_sha256,
                     yaml_bytes, expected_yaml_sha256)
    except AssemblyBindingError:
        raise
    except (ValueError, TypeError, KeyError, IndexError, AttributeError, yaml.YAMLError) as error:
        raise AssemblyBindingError('Assembly recipe binding refused: ' + str(error)) from error


def _bind(inputs, legacy_raw, legacy_pin, yaml_raw, yaml_pin):
    lexical._pin(legacy_raw, legacy_pin); lexical._pin(yaml_raw, yaml_pin)
    _require(type(inputs) is dict, 'original condition input mapping required')
    binding = source_binding.bind_condition_units(**inputs)
    _require(binding['selection_id'] is None and all(u['kind'] == 'official-section' for u in binding['units']),
             'Assembly selected/corrected conditions require separate review')
    selected = [u for u in binding['units'] if u['label'] == 'Special Play Condition']
    _require(len(selected) == 1, 'one exact Special Play Condition required')
    semantic, adapted = _recipe(selected[0]['text'])
    source_occurrences = []
    for oid, raw in sorted(inputs['observations'].items()):
        observation = json.loads(raw, object_pairs_hook=lexical._pairs)
        if observation['source'] != 'official':
            continue
        e = observation['extracted']
        _require(e['layout'] == 'single' and e['card_type'] == 'Digimon'
                 and len(e['faces']) == 1 and e['faces'][0]['kind'] == 'digimon'
                 and e['faces'][0]['face_id'] == 'primary'
                 and e['faces'][0]['text_sections'] == e['text_sections'], 'single Digimon source required')
        occurrences = [(i, s) for i, s in enumerate(e['text_sections']) if s['label'] == 'Special Play Condition']
        _require(len(occurrences) == 1, 'one Assembly occurrence per complete printing required')
        i, section = occurrences[0]
        recipe, adapted_other = _recipe(section['text'])
        _require(recipe == semantic and adapted_other == adapted, 'conflicting Assembly printings')
        source_occurrences.append({'observation_id': oid, 'printing_id': e['printing']['printing_id'],
            'section_index': i, 'condition_text': section['text'], 'original_observation': deepcopy(observation)})
    _require(source_occurrences, 'official Assembly evidence required')
    dna, legacy = ingest_cards._parse_xros_costs(adapted)
    _require(not dna and len(legacy) == 1, 'legacy parser did not produce one complete recipe')
    submitted = json.loads(legacy_raw, object_pairs_hook=lexical._pairs,
        parse_constant=lambda _: (_ for _ in ()).throw(AssemblyBindingError('nonfinite JSON')))
    _require(serialization.canonical_json_bytes(submitted) == serialization.canonical_json_bytes(legacy),
             'complete legacy field differs from source-derived serialization')
    text = yaml_raw.decode('utf-8')
    _require(not any(isinstance(t, (AliasToken, AnchorToken, TagToken)) for t in yaml.scan(text)),
             'YAML aliases, anchors and explicit tags unsupported')
    node = yaml.compose(text, Loader=yaml.BaseLoader); doc = lexical._node_value(node)
    _require(type(doc) is dict and doc.get('card') == binding['card_id'] and doc.get('kind') == 'digimon',
             'YAML identity differs from source')
    paths = doc.get('alt_paths')
    _require(type(paths) is list and all(type(p) is dict for p in paths), 'alt-path list required')
    matches = [i for i, p in enumerate(paths) if p.get('kind') == 'assembly']
    _require(len(matches) == 1, 'exactly one Assembly path required')
    index = matches[0]; path = paths[index]
    _require(set(path) == {'kind', 'materials', 'cost'}, 'unknown Assembly path fields')
    pn = lexical._field_node(node, 'alt_paths').value[index]
    _require(lexical._decimal(lexical._field_node(pn, 'cost'), 65535) == semantic['total_reduction'],
             'Assembly total reduction differs')
    obligations = []
    if semantic['family'] == 'ordered_exact_names':
        expected = [{'kind': 'digimon', 'name_is': m['name_is'], 'zones': ['trash'], 'stack_under': 'true'}
                    for m in semantic['materials']]
        obligations = [{'code': 'NAMED_MATERIAL_KIND_CLOSURE_REQUIRED',
                        'names': [m['name_is'] for m in semantic['materials']], 'yaml_kind': 'digimon'}]
    else:
        predicate = deepcopy(semantic['materials'][0]['predicate'])
        predicate['all_of'][1]['level_lte'] = str(predicate['all_of'][1]['level_lte'])
        expected = [{'filter': predicate, 'zones': ['trash'], 'stack_under': 'true'}]
    _require(path['materials'] == expected, 'Assembly material order/predicates/origin/count differ')
    for mn in lexical._field_node(pn, 'materials').value:
        stack = lexical._field_node(mn, 'stack_under')
        _require(stack.style is None and stack.value == 'true', 'plain native true required')
        if semantic['family'] == 'bounded_digimon_text_or_trait':
            pred = lexical._field_node(mn, 'filter')
            level = lexical._field_node(lexical._field_node(pred, 'all_of').value[1], 'level_lte')
            lexical._decimal(level, 255)
    effects = doc.get('effects')
    _require(type(effects) is list, 'original effect inventory required')
    return {'version': 1, 'kind': 'assembly-recipe-binding-diagnostic',
        'status': 'BOUND_ASSEMBLY_RECIPE_DIAGNOSTIC', 'card_id': binding['card_id'],
        'source_authority': False, 'native_compilation_verified': False, 'training_source_complete': False,
        'condition_binding': binding, 'semantic_recipe': semantic, 'source_occurrences': source_occurrences,
        'legacy_adapter': {'version': 1, 'label_only_transform': adapted,
                           'legacy_key_names_are_not_semantics': True}, 'legacy_value': legacy,
        'yaml_text': text, 'yaml_path': f'alt_paths.{index}', 'yaml_span': lexical._span(text, pn),
        'unreviewed_alt_path_paths': [f'alt_paths.{i}' for i in range(len(paths)) if i != index],
        'unreviewed_effect_paths': [f'effects.{i}' for i in range(len(effects))],
        'unresolved_obligations': obligations,
        'input_pins': {'legacy_field_sha256': legacy_pin, 'yaml_sha256': yaml_pin},
        'dependency_sha256': {m.__name__: hashlib.sha256(Path(m.__file__).read_bytes()).hexdigest()
                             for m in (ingest_cards, xros_cost_parser, serialization, source_binding, lexical)},
        'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
