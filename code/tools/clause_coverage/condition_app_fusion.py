"""Opt-in App Fusion spans for source units already validated through A2.

App Fusion has its own timing identity. This module does not interpret linked
materials, choose a pair, validate source authority, or change the shared text
splitter. Original units and complete material/cost/reminder bodies survive.
"""
from copy import deepcopy
import re

from .condition_dna_alias import split_dna_condition_units
from .condition_spans import ConditionSpanError
from .text_split import TIMING_MARKER_NAMES

_MARKER = '[App Fusion]'
_NAME = r'\[[^\[\]\r\n<>＜＞]+\]'
_REMINDER = r'If 2 such cards are linked together, stack the link card on top and digivolve\.?'
_BODY = re.compile(
    rf'{_NAME}(?:[ \t]*&[ \t]*{_NAME})+[ \t]*:[ \t]*Cost [0-9]+'
    rf'(?:\s+(?:{_REMINDER}|\({_REMINDER}\)))?'
)
_RESERVED = TIMING_MARKER_NAMES | {
    'App Fusion', 'DNA Digivolution', 'Special Play Condition',
    'Special Digivolution Condition', 'Effect', 'Inherited Effect', 'Security Effect',
}
_RESERVED_LOWER = {name.lower() for name in _RESERVED}
_HEADER_NAME = re.compile(r'\b(?:Condition|Header|Section)\b', re.IGNORECASE)


def split_app_fusion_condition_units(units: list[dict]) -> list[dict]:
    """Retain ordered occurrences and a distinct App Fusion timing span.

    Accept only a leading exact marker in an official special-digivolution
    section. Named alternatives, finite cost and the known optional printed
    reminder are retained as opaque text; unknown shapes fail closed. A lone
    marker retains its explicit empty span. Other approved grammars delegate
    to the existing DNA spelling adapter without changes.
    """
    if not isinstance(units, list):
        raise ConditionSpanError('condition units must be an ordered list')
    result = []
    for index, unit in enumerate(units):
        text = unit.get('text') if isinstance(unit, dict) else None
        if not isinstance(text, str) or _MARKER not in text:
            row = split_dna_condition_units([unit])[0]
            row['unit_index'] = index
            result.append(row)
            continue
        stripped = text.strip()
        if (not stripped.startswith(_MARKER) or stripped.count(_MARKER) != 1
                or unit.get('kind') != 'official-section' or unit.get('field') != 'xros_req'
                or not isinstance(unit.get('label'), str)
                or unit['label'].lower() != 'special digivolution condition'
                or unit.get('condition_type') != 'special_digivolution_condition'):
            raise ConditionSpanError('unsupported App Fusion source or marker placement')
        body = stripped[len(_MARKER):].lstrip()
        if body and (not _BODY.fullmatch(body) or any(
                name.strip().lower() in _RESERVED_LOWER or not name.strip() or _HEADER_NAME.search(name)
                for name in re.findall(r'\[([^\[\]]*)\]', body))):
            raise ConditionSpanError('unsupported or mixed App Fusion material/cost grammar')
        result.append({'unit_index': index, 'source_unit': deepcopy(unit), 'spans': [{
            'span_index': 0, 'kind': 'timing', 'timings': ['App Fusion'],
            'keyword': None, 'text': body,
        }]})
    return result
