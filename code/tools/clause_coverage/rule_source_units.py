"""Pure Rule occurrence/trait derivation diagnostic, never source authority.

Input observations are caller supplied. This adapter does not authenticate their
identity, raw HTML, images, selected canonical values or runtime projection.
Whole source containers and all ordinary siblings remain untouched; Rule units
are structural derivations, not additional executable effects or clause IDs.
"""
from copy import deepcopy
import hashlib
import re
import unicodedata

from tools.card_data import normalization
from tools.card_data.sources import official
from tools.card_data.source_conservation import rule_grant_check


class RuleUnitError(ValueError):
    """Malformed input is distinct from an explicitly unsupported Rule."""


_CATEGORIES = {'traits': official._RULE_TRAIT,
               'forms': official._RULE_FORM,
               'attributes': official._RULE_ATTRIBUTE}
_RULE_WORD = re.compile(r'\b(?:Special\s*)?Rule\b', re.I)


def _rule_word(text):
    # Broad Unicode detection is a refusal guard, never spelling normalization.
    return bool(_RULE_WORD.search(unicodedata.normalize('NFKC', text)))


def derive_rule_occurrences(observation):
    """Preserve every occurrence and derive only exact supported whole lines.

    Repeated source lines remain distinct although producer grants deduplicate.
    Unsupported rules retain the complete source and blockers, with no projected
    fields. Complete source/context/correction binding is a later integration
    gate; even a successful result is only a pure derivation diagnostic.
    """
    if not isinstance(observation, dict) or not isinstance(observation.get('extracted'), dict):
        raise RuleUnitError('expected an observation with extracted source fields')
    e = observation['extracted']
    sections = e.get('text_sections')
    if (not isinstance(e.get('card_id'), str) or not isinstance(sections, list)
            or any(not isinstance(s, dict) or not isinstance(s.get('label'), str)
                   or not isinstance(s.get('text'), str) for s in sections)
            or any(not isinstance(e.get(k), str) for k in ('form', 'type', 'attribute'))):
        raise RuleUnitError('explicit card, printed trait rows and section inventory required')
    occurrences, blockers, inventory = [], [], []
    expected = {key: [] for key in _CATEGORIES}
    for section_index, section in enumerate(sections):
        text = section['text']
        inventory.append({'section_index': section_index, **deepcopy(section)})
        labelled_rule = _rule_word(section['label'])
        if labelled_rule and not text.strip():
            blockers.append({'code': 'EMPTY_RULE_SECTION', 'section_index': section_index})
        start = 0
        for line_index, raw_line in enumerate(text.splitlines(keepends=True)):
            line = raw_line.rstrip('\r\n')
            end = start + len(line)
            if labelled_rule or _rule_word(line):
                location = {'section_index': section_index, 'line_index': line_index,
                            'char_start': start, 'char_end': end,
                            'utf8_byte_start': len(text[:start].encode('utf8')),
                            'utf8_byte_end': len(text[:end].encode('utf8')),
                            'line': line}
                matches = []
                for category, pattern in _CATEGORIES.items():
                    # Match the original line, not Unicode-normalized text.
                    match = pattern.match(line)
                    if match and line[match.end():].strip() in ('', '.'):
                        value = match.group(1).strip()
                        if value and not any(char in value for char in '/[]\r\n'):
                            matches.append((category, value))
                if len(matches) != 1:
                    blockers.append({'code': 'UNSUPPORTED_RULE_LINE', **location})
                else:
                    category, value = matches[0]
                    if value not in expected[category]:
                        expected[category].append(value)
                    occurrences.append({
                        'occurrence_index': len(occurrences), **location,
                        'label': section['label'], 'whole_container': text,
                        'container_sha256': hashlib.sha256(text.encode('utf8')).hexdigest(),
                        'category': category, 'value': value,
                        'derivation': 'official-v5-whole-line-rule-grant',
                    })
            start += len(raw_line)
    grants = e.get('rule_grants')
    valid_shape = (isinstance(grants, dict) and set(grants) == set(_CATEGORIES)
                   and all(isinstance(v, list) and all(isinstance(x, str) for x in v)
                           for v in grants.values()))
    if not valid_shape:
        blockers.append({'code': 'INVALID_RULE_GRANT_SHAPE'})
    else:
        # Existing independent completeness guard preserves multiplicity checks
        # on the semantic lists; our occurrence inventory never deduplicates.
        blockers.extend(rule_grant_check(sections, grants))
        if grants != expected:
            blockers.append({'code': 'RULE_GRANTS_DIFFER_FROM_OCCURRENCES',
                             'expected': expected, 'actual': deepcopy(grants)})
    projected = None
    if not blockers:
        fields = normalization._official_fields(e)
        projected = {key: deepcopy(fields[key])
                     for key in ('form_eng', 'type_eng', 'attribute_eng')}
    return {
        'version': 1, 'kind': 'pure-rule-occurrence-diagnostic',
        'status': ('BLOCKED_RULE_DIAGNOSTIC' if blockers else
                   'DERIVED_RULE_DIAGNOSTIC' if occurrences else 'NO_RULE_GRANTS_DIAGNOSTIC'),
        'card_id': e['card_id'], 'source_observation': deepcopy(observation),
        'section_inventory': inventory, 'occurrences': occurrences,
        'source_grants': deepcopy(grants), 'derived_grants': expected,
        'projected_fields': projected, 'blockers': blockers,
        'integration_gate': 'complete source/canonical/runtime/correction binding not performed',
    }
