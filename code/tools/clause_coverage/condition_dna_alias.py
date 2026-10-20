"""Opt-in printed DNA Digivolution spelling adapter for validated A2 units.

Only one leading printed marker in an explicit official digivolution section is
aliased to the existing DNA timing. This is neither an interpreter nor a source
validator; complete source binding remains a caller prerequisite.
"""
from copy import deepcopy
import re

from .condition_digixros import split_digixros_condition_units
from .condition_spans import ConditionSpanError
from .text_split import TIMING_MARKER_NAMES

_ALIAS = '[DNA Digivolution]'
_NAMES = '|'.join(re.escape(name) for name in sorted(TIMING_MARKER_NAMES | {'App Fusion'}, key=len, reverse=True))
_NESTED = re.compile(rf'\[(?:{_NAMES})\]|<(?:{_NAMES})>|＜(?:{_NAMES})＞')
_PLAIN_ROUTE = re.compile(r'(?:^|\n)[ \t]*(?:DigiXros|Assembly|App Fusion|DNA Digivolution)(?=\s|[:\-\[])')
_REMINDER = 'Digivolve unsuspended with the 2 specified Digimon stacked on top of each other.'


def split_dna_condition_units(units: list[dict]) -> list[dict]:
    """Preserve all units and opaque cost/material/reminder text without IDs.

    Marker-only units remain meaningful. Repeated source units remain distinct.
    Multiple routes within one alias unit require a later explicit grammar and
    are refused, as are any nested timings (including apparent references).
    """
    if not isinstance(units, list):
        raise ConditionSpanError('condition units must be an ordered list')
    prepared = deepcopy(units)
    transformed = {}
    for index, unit in enumerate(prepared):
        if not isinstance(unit, dict) or not isinstance(unit.get('text'), str):
            continue
        text = unit['text'].strip()
        if _ALIAS not in text:
            continue
        if (not text.startswith(_ALIAS) or text.count(_ALIAS) != 1
                or unit.get('kind') != 'official-section' or unit.get('field') != 'xros_req'
                or not isinstance(unit.get('label'), str)
                or unit['label'].lower() != 'special digivolution condition'
                or unit.get('condition_type') != 'special_digivolution_condition'):
            raise ConditionSpanError('unsupported printed DNA source or alias placement')
        body = text[len(_ALIAS):].lstrip()
        # The observed printed reminder begins with Digivolve. Preserve that
        # exact single occurrence while refusing an additional unmarked route.
        without_reminder = body.replace(_REMINDER, '', 1)
        if (_NESTED.search(body) or _PLAIN_ROUTE.search(body)
                or re.search(r'\bDigivolve\b', without_reminder)):
            raise ConditionSpanError('nested or mixed printed DNA condition route')
        unit['text'] = '[DNA Digivolve]' + text[len(_ALIAS):]
        transformed[index] = body
    result = split_digixros_condition_units(prepared)
    for index, body in transformed.items():
        expected = [{'span_index':0, 'kind':'timing', 'timings':['DNA Digivolve'],
                     'keyword':None, 'text':body}]
        if result[index]['spans'] != expected:
            raise ConditionSpanError('mixed or lossy printed DNA condition segmentation')
    for index, row in enumerate(result):
        row['source_unit'] = deepcopy(units[index])
    return result
