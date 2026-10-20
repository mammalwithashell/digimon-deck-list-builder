"""Pin actual Link source/YAML pairs without granting runtime authority."""
from copy import deepcopy
import hashlib
import importlib
import json
from pathlib import Path

import pytest

F = Path(__file__).parents[1] / 'fixtures/card_data'


def bind(name='BT21-071', *, source=None, script=None, **overrides):
    raw = (F / 'link_source_units' / f'{name}.json').read_bytes() if source is None else source
    yaml = (F / 'link_yaml_binding' / f'{name.split("_P")[0]}.yaml').read_bytes() if script is None else script
    args = dict(card_id=name.split('_P')[0], observation_bytes=raw,
                expected_observation_sha256=hashlib.sha256(raw).hexdigest(), yaml_bytes=yaml,
                expected_yaml_sha256=hashlib.sha256(yaml).hexdigest())
    args.update(overrides)
    return importlib.import_module('tools.clause_coverage.link_yaml_binding').bind_link_yaml(**args)


@pytest.mark.parametrize('name,cost,dp', [('BT21-071', 2, 3000), ('BT21-071_P1', 2, 3000), ('BT21-074', 3, 4000)])
def test_actual_source_and_yaml(name, cost, dp):
    result = bind(name)
    assert result['status'] == 'BOUND_LINK_YAML_DIAGNOSTIC'
    assert result['blockers'] == []
    assert result['source_authority'] is False and result['training_source_complete'] is False
    assert result['native_compilation_verified'] is False
    assert result['bindings']['link_condition']['value'] == {'trait': 'Appmon', 'cost': cost}
    assert result['bindings']['linked_dp']['value'] == dp
    assert result['unreviewed_effect_paths']
    assert result == bind(name)
    for binding in result['bindings'].values():
        span = binding['yaml_span']
        text = result['yaml_text'][span['char_start']:span['char_end']]
        assert text
        assert result['yaml_text'].encode()[span['utf8_start']:span['utf8_end']].decode() == text


def test_original_source_yaml_and_unrelated_effects_preserved():
    raw = (F / 'link_source_units/BT21-071.json').read_bytes()
    script = (F / 'link_yaml_binding/BT21-071.yaml').read_bytes()
    result = bind(source=raw, script=script)
    assert result['observation'] == json.loads(raw)
    assert result['yaml_text'].encode() == script
    assert len(result['unreviewed_effect_paths']) == 3
    assert len(result['source_derivation']['occurrences']) == 3


@pytest.mark.parametrize('old,new', [
    ('cost: 2\n    filter: { trait_has: Appmon }', 'cost: 3\n    filter: { trait_has: Appmon }'),
    ('filter: { trait_has: Appmon }', 'filter: { trait_has: Tool }'),
    ('filter: { trait_has: Appmon }', 'filter: { trait_has: Appmon, level_eq: 4 }'),
    ('dp_modifier: 3000', 'dp_modifier: 4000'),
    ('dp_modifier: 3000', 'dp_modifier: true'),
    ('dp_modifier: 3000', 'dp_modifier: 03_000'),
    ('dp_modifier: 3000', 'dp_modifier: "3000"'),
    ('dp_modifier: 3000', 'dp_modifier: !!int 3000'),
    ('cost: 2\n    filter:', 'cost: true\n    filter:'),
    ('cost: 2\n    filter:', 'cost: 02\n    filter:'),
    ('cost: 2\n    filter:', 'cost: 0x2\n    filter:'),
    ('cost: 2\n    filter:', 'cost: "2"\n    filter:'),
    ('cost: 2\n    filter:', 'cost: 2.0\n    filter:'),
    ('target: {}', 'target: { level_eq: 4 }'),
    ('scope: linked\n    kind: aura', 'scope: inherited\n    kind: aura'),
    ('kind: link_condition', 'kind: link_requirement'),
    ('kind: link_condition', 'kind: link_condition\n    active_when: { your_turn: true }'),
    ('kind: link_condition', 'kind: link_condition\n    scope: inherited'),
    ('dp_modifier: 3000', 'dp_modifier: 3000\n    dp_modifier_fn: { literal: 1 }'),
    ('dp_modifier: 3000', 'dp_modifier: 3000\n    active_when: { your_turn: true }'),
    ('card: BT21-071', 'card: BT21-074'),
    ('cost: 2\n    filter:', 'cost: 2\n    cost: 2\n    filter:'),
    ('target: {}', 'target: &same {}\n    alias: *same'),
    ('target: {}', 'target: { <<: {} }'),
])
def test_changed_or_unsupported_yaml_is_blocked(old, new):
    script = (F / 'link_yaml_binding/BT21-071.yaml').read_text(encoding='utf-8')
    assert old in script
    result = bind(script=script.replace(old, new).encode())
    assert result['status'] == 'BLOCKED' and result['blockers']
    assert result['bindings'] == {}


@pytest.mark.parametrize('suffix', [
    '\n  - kind: link_condition\n    cost: 2\n    filter: { trait_has: Appmon }\n',
    '\n  - scope: linked\n    kind: aura\n    target: {}\n    dp_modifier: 3000\n',
])
def test_ambiguous_duplicate_bindings_block(suffix):
    raw = (F / 'link_yaml_binding/BT21-071.yaml').read_bytes()
    assert bind(script=raw + suffix.encode())['status'] == 'BLOCKED'


@pytest.mark.parametrize('override', [
    {'expected_yaml_sha256': '0' * 64}, {'expected_observation_sha256': '0' * 64},
    {'expected_yaml_sha256': True}, {'card_id': 'BT21-074'}, {'yaml_bytes': b'\xff'},
    {'observation_bytes': b'{}'}, {'yaml_bytes': b'[]'},
])
def test_missing_foreign_or_unpinned_inputs_refused(override):
    assert bind(**override)['status'] == 'BLOCKED'


@pytest.mark.parametrize('change', ['wrong_card', 'foreign_source', 'missing_dp', 'malformed', 'duplicate_json', 'face_drift'])
def test_source_failures_cannot_be_bypassed(change):
    raw = json.loads((F / 'link_source_units/BT21-071.json').read_bytes())
    if change == 'wrong_card': raw['extracted']['card_id'] = 'BT21-074'
    if change == 'foreign_source': raw['source'] = 'secondary'
    if change == 'missing_dp': raw['extracted']['text_sections'].pop(2)
    if change == 'malformed': raw['extracted'] = None
    if change == 'face_drift': raw['extracted']['faces'][0]['text_sections'][2]['text'] = '+1 DP'
    source = json.dumps(raw).encode()
    if change == 'duplicate_json': source = source[:-1] + b',"source":"official"}'
    assert bind(source=source)['status'] == 'BLOCKED'


def test_repeated_source_occurrences_remain_distinct():
    raw = json.loads((F / 'link_source_units/BT21-071.json').read_bytes())
    sections = raw['extracted']['text_sections']
    sections.append(deepcopy(sections[-1]))
    raw['extracted']['faces'][0]['text_sections'] = deepcopy(sections)
    result = bind(source=json.dumps(raw).encode())
    assert result['status'] == 'BOUND_LINK_YAML_DIAGNOSTIC'
    assert len(result['bindings']['link_condition']['source_occurrences']) == 2
    assert len(result['source_derivation']['occurrences']) == 5


def test_unicode_offsets_and_crlf_do_not_rewrite_original():
    raw = (F / 'link_yaml_binding/BT21-071.yaml').read_bytes()
    raw = raw.replace(b'\r\n', b'\n').replace(b'\n', b'\r\n')
    result = bind(script=raw)
    assert result['status'] == 'BOUND_LINK_YAML_DIAGNOSTIC'
    assert result['yaml_text'].encode() == raw


def test_oversized_matching_cost_still_refuses_native_u16_overflow():
    raw = json.loads((F / 'link_source_units/BT21-071.json').read_bytes())
    for section in raw['extracted']['text_sections']:
        if section['label'] == 'Inherited Effect':
            section['text'] = section['text'].replace('Cost 2 ', 'Cost 65536 ')
    raw['extracted']['faces'][0]['text_sections'] = deepcopy(raw['extracted']['text_sections'])
    script = (F / 'link_yaml_binding/BT21-071.yaml').read_text(encoding='utf-8')
    script = script.replace('cost: 2\n    filter:', 'cost: 65536\n    filter:')
    assert bind(source=json.dumps(raw).encode(), script=script.encode())['status'] == 'BLOCKED'


def test_nonstring_selected_summary_is_not_a_native_clause():
    script = (F / 'link_yaml_binding/BT21-071.yaml').read_text(encoding='utf-8')
    script = script.replace('summary: "[Link] +3000 DP"', 'summary: { injected: true }')
    assert bind(script=script.encode())['status'] == 'BLOCKED'


@pytest.mark.parametrize('path,value', [
    (('extracted', 'card_type'), 'Tamer'),
    (('extracted', 'faces', 0, 'kind'), 'option'),
    (('schema_version',), True), (('schema_version',), 900),
    (('parser_version',), True), (('parser_version',), 'official-html-future'),
    (('source_schema_version',), True), (('source_schema_version',), 'unknown-source-schema'),
])
def test_unsupported_source_kind_and_version_are_not_a_binding(path, value):
    raw = json.loads((F / 'link_source_units/BT21-071.json').read_bytes())
    target = raw
    for key in path[:-1]: target = target[key]
    target[path[-1]] = value
    result = bind(source=json.dumps(raw).encode())
    assert result['status'] == 'BLOCKED' and result['bindings'] == {}
    assert result['observation'] == raw
