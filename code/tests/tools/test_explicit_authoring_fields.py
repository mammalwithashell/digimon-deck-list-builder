import re

import pytest
import yaml

from tools.explicit_authoring_fields import ROOT, annotate, optional_steps


def test_preserves_lines_comments_and_flow_mappings():
    raw = 'effects:\n  # keep\n  - process:\n      - select_hand: # keep too\n          of: you\n      - select_trash: {of: you, optional: true}\n'
    updated, counts = annotate(raw)
    assert counts == {'optional': 1}
    assert updated == raw.replace('          of: you', '          optional: false\n          of: you')
    assert annotate(updated) == (updated, {})
    updated, counts = annotate('process: [{select_hand: {of: you}}, {select_trash: {}}]')
    assert updated == 'process: [{select_hand: {optional: false, of: you}}, {select_trash: {optional: false}}]'
    assert counts == {'optional': 2}
    assert annotate('process: [{select_hand: { of: you }}]')[0] == 'process: [{select_hand: { optional: false, of: you }}]'
    assert annotate('process:\n- select_hand: &pick\n    of: you\n')[0] == 'process:\n- select_hand: &pick\n    optional: false\n    of: you\n'


def test_buckets_colors_crlf_and_into_paths():
    raw = 'alt_paths:\r\n  - kind: digivolve\r\n    from: {level_eq: 3} # comment\r\n  - kind: digivolve\r\n    direction: into\r\n    from: {level_eq: 6}\r\nprocess: [{reveal_search: {buckets: [{to: hand, max: 1}]}}]\r\n'
    updated, counts = annotate(raw)
    assert counts == {'optional': 1, 'any_color': 1}
    assert updated == raw.replace('{level_eq: 3}', '{any_color: true, level_eq: 3}').replace('{to: hand', '{optional: false, to: hand')
    assert annotate('alt_paths: [{kind: digivolve, from: {level_eq: 3, any_of: [{color_is: red}, {color_is: blue}]}}]')[1] == {}
    assert annotate('alt_paths: [{kind: digivolve, from: {level: {eq: 3}}}]')[1] == {'any_color': 1}


def test_empty_and_invalid_inputs():
    assert annotate('') == ('', {})
    with pytest.raises(yaml.YAMLError):
        annotate('effects: [')
    with pytest.raises(ValueError):
        annotate('process: [{select_hand: null}]')


def test_optional_verb_inventory_covers_all_optional_argument_structs():
    source = (ROOT / 'code/digimon-dsl/src/step.rs').read_text()
    optional_args = []
    structs = list(re.finditer(r'pub struct (\w+)', source))
    for i, match in enumerate(structs):
        end = structs[i + 1].start() if i + 1 < len(structs) else len(source)
        if re.search(r'pub optional: bool', source[match.end():end]):
            optional_args.append(match[1])
    assert len(optional_args) == 21
    variants = set()
    for args in optional_args:
        if args == 'RevealSearchBucket':
            continue
        for variant in re.findall(r'^    (\w+)\(' + args + r'\)', source, re.M):
            key = re.search(r'StepSpec::' + variant + r'\(v\) => (?:\{\s*)?kv!\(s, "([^"]+)"', source)
            assert key, variant
            variants.add(key[1])
    assert variants == optional_steps()
