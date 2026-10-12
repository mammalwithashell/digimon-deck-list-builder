"""Conservative source-to-YAML Link cost/DP correspondence, offline only.

Pins prevent accidental input drift; they do not establish source authority.
The future whole-card consumer must independently authenticate every printing,
canonical choice, image and loaded compiler. This helper neither compiles YAML
nor validates the other effects. It never authorizes runtime/training admission.
"""
from copy import deepcopy
import hashlib
import json
import re

import yaml
from yaml.nodes import MappingNode, ScalarNode, SequenceNode
from yaml.tokens import AliasToken, AnchorToken, TagToken

from .link_source_units import derive_link_occurrences


class _Invalid(ValueError):
    pass


def _require(test, reason):
    if not test:
        raise _Invalid(reason)


def _pin(raw, expected):
    _require(type(raw) is bytes, 'exact bytes required')
    _require(type(expected) is str and re.fullmatch('[0-9a-f]{64}', expected), 'SHA256 pin required')
    _require(hashlib.sha256(raw).hexdigest() == expected, 'input pin mismatch')


def _pairs(items):
    value = {}
    for key, item in items:
        _require(key not in value, 'duplicate JSON key')
        value[key] = item
    return value


def _node_value(node):
    """BaseLoader keeps scalars lexical; do not import YAML1.1 coercions."""
    if isinstance(node, ScalarNode):
        _require(node.tag == 'tag:yaml.org,2002:str', 'unsupported scalar tag')
        return node.value
    if isinstance(node, SequenceNode):
        _require(node.tag == 'tag:yaml.org,2002:seq', 'unsupported sequence tag')
        return [_node_value(v) for v in node.value]
    _require(isinstance(node, MappingNode) and node.tag == 'tag:yaml.org,2002:map', 'mapping required')
    value = {}
    for key, item in node.value:
        _require(isinstance(key, ScalarNode), 'scalar mapping key required')
        key = _node_value(key)
        _require(key != '<<' and key not in value, 'duplicate or merged YAML key')
        value[key] = _node_value(item)
    return value


def _field_node(node, name):
    return next(value for key, value in node.value if key.value == name)


def _decimal(node, maximum):
    _require(isinstance(node, ScalarNode) and node.style is None
             and re.fullmatch('0|[1-9][0-9]*', node.value), 'plain canonical decimal required')
    value = int(node.value)
    _require(value <= maximum, 'numeric range unsupported')
    return value


def _span(text, node):
    start, end = node.start_mark.index, node.end_mark.index
    return {'char_start': start, 'char_end': end,
            'utf8_start': len(text[:start].encode('utf-8')),
            'utf8_end': len(text[:end].encode('utf-8'))}


def bind_link_yaml(*, card_id, observation_bytes, expected_observation_sha256,
                   yaml_bytes, expected_yaml_sha256):
    """Bind one pinned observation and whole YAML; retain repeated occurrences.

    Canonical decimal is deliberately narrower than native YAML's integer syntax.
    Unknown keys in the two selected clauses refuse; unrelated clauses and all
    original bytes remain unreviewed. No caller-derived occurrence/proof accepted.
    """
    result = {'version': 1, 'kind': 'link-yaml-binding-diagnostic', 'status': 'BLOCKED',
              'source_authority': False, 'training_source_complete': False,
              'native_compilation_verified': False, 'card_id': card_id,
              'observation': None, 'yaml_text': None, 'source_derivation': None,
              'bindings': {}, 'unreviewed_effect_paths': [], 'blockers': []}
    try:
        _require(type(card_id) is str and re.fullmatch(r'[A-Z]+[0-9]*-[0-9]+', card_id), 'card ID required')
        _pin(observation_bytes, expected_observation_sha256)
        _pin(yaml_bytes, expected_yaml_sha256)
        observation = json.loads(observation_bytes.decode('utf-8'), object_pairs_hook=_pairs,
                                 parse_constant=lambda _: (_ for _ in ()).throw(_Invalid('nonfinite JSON')))
        result['observation'] = deepcopy(observation)
        _require(isinstance(observation, dict)
                 and type(observation.get('schema_version')) is int and observation['schema_version'] == 1
                 and observation.get('parser_version') == 'official-html-v5'
                 and observation.get('source_schema_version') == 'world-card-list-html-v1',
                 'unsupported official observation schema or parser version')
        extracted = observation.get('extracted') if isinstance(observation, dict) else None
        _require(isinstance(extracted, dict) and extracted.get('card_id') == card_id
                 and extracted.get('card_type') == 'Digimon', 'source Digimon identity mismatch')
        faces = extracted.get('faces')
        _require(extracted.get('layout') == 'single' and isinstance(faces, list) and len(faces) == 1
                 and isinstance(faces[0], dict) and faces[0].get('face_id') == 'primary'
                 and faces[0].get('kind') == 'digimon'
                 and faces[0].get('text_sections') == extracted.get('text_sections'), 'single source face mismatch')
        diagnostic = derive_link_occurrences(observation)
        result['source_derivation'] = diagnostic
        _require(diagnostic['status'] == 'DERIVED_LINK_DIAGNOSTIC', 'source Link structure unsupported')
        conditions = [o for o in diagnostic['occurrences'] if o['role'] == 'condition']
        dp = [o for o in diagnostic['occurrences'] if o['role'] == 'dp']
        _require(conditions and len(dp) == 1, 'unambiguous source cost and DP required')
        _require(all(o['value'] == conditions[0]['value'] for o in conditions), 'conflicting source conditions')
        text = yaml_bytes.decode('utf-8')
        result['yaml_text'] = text
        _require(not any(isinstance(t, (AliasToken, AnchorToken, TagToken)) for t in yaml.scan(text)),
                 'YAML aliases, anchors and explicit tags unsupported')
        node = yaml.compose(text, Loader=yaml.BaseLoader)
        doc = _node_value(node)
        _require(isinstance(doc, dict) and doc.get('card') == card_id and doc.get('kind') == 'digimon',
                 'Digimon YAML identity mismatch')
        effects = doc.get('effects')
        _require(isinstance(effects, list) and all(isinstance(e, dict) for e in effects), 'effects mappings required')
        _require(not any(e.get('kind') == 'link_requirement' for e in effects), 'additional Link route unsupported')
        cost_ids = [i for i, e in enumerate(effects) if e.get('kind') == 'link_condition']
        dp_ids = [i for i, e in enumerate(effects) if e.get('kind') == 'aura' and e.get('scope') == 'linked']
        _require(len(cost_ids) == 1 and len(dp_ids) == 1, 'exactly one Link condition and linked aura required')
        ci, di = cost_ids[0], dp_ids[0]
        c, d = effects[ci], effects[di]
        _require(set(c) <= {'kind', 'cost', 'filter', 'summary'} and {'cost', 'filter'} <= set(c),
                 'conditional or unknown Link condition fields')
        _require(set(d) <= {'scope', 'kind', 'target', 'dp_modifier', 'summary'}
                 and {'target', 'dp_modifier'} <= set(d), 'conditional or unknown linked aura fields')
        _require(c['filter'] == {'trait_has': 'Appmon'} and d['target'] == {}, 'host predicates unsupported')
        _require(all('summary' not in clause or isinstance(clause['summary'], str) for clause in (c, d)),
                 'summary must be a scalar string')
        effects_node = _field_node(node, 'effects')
        cn, dn = effects_node.value[ci], effects_node.value[di]
        cost = _decimal(_field_node(cn, 'cost'), 2**16 - 1)
        bonus = _decimal(_field_node(dn, 'dp_modifier'), 2**31 - 1)
        _require(conditions[0]['value'] == {'trait': 'Appmon', 'cost': cost}
                 and dp[0]['value'] == bonus, 'printed and YAML Link values differ')
        result['bindings'] = {
            'link_condition': {'yaml_path': f'effects.{ci}', 'yaml_span': _span(text, cn),
                               'value': conditions[0]['value'], 'source_occurrences': conditions},
            'linked_dp': {'yaml_path': f'effects.{di}', 'yaml_span': _span(text, dn),
                          'value': bonus, 'source_occurrences': dp},
        }
        result['unreviewed_effect_paths'] = [f'effects.{i}' for i in range(len(effects)) if i not in (ci, di)]
        result['input_pins'] = {'observation_sha256': expected_observation_sha256,
                                'yaml_sha256': expected_yaml_sha256}
        result['status'] = 'BOUND_LINK_YAML_DIAGNOSTIC'
    except (_Invalid, ValueError, UnicodeError, yaml.YAMLError) as error:
        result['bindings'] = {}
        result['blockers'].append({'code': 'UNSUPPORTED_LINK_YAML_BINDING', 'reason': str(error)})
    return result
