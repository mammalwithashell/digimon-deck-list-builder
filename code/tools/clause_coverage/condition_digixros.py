"""Opt-in plain DigiXros segmentation for already validated A2 source units.

This is a local spelling adapter, not a source validator or a cost interpreter.
Only the plain keyword in an explicit Special Play Condition becomes a timing
marker. Original units and every reduction/material/reminder byte are retained.
The existing splitter and all other source-section contracts are unchanged.
"""
from copy import deepcopy
import re

from .condition_spans import ConditionSpanError, split_condition_units
from .text_split import TIMING_MARKER_NAMES

_PREFIX = re.compile(r'\ADigiXros[ \t]*-[1-9][0-9]*(?::[ \t]*\S|[ \t]+from[ \t]+\S)')
_ANOTHER_ROUTE = re.compile(
    r'\bDigiXros\b|(?:^|\n)[ \t]*(?:Assembly|App Fusion|DNA Digivolution|Digivolve)(?=\s|[:\-\[])'
)
_TIMING_NAMES = '|'.join(re.escape(name) for name in sorted(TIMING_MARKER_NAMES, key=len, reverse=True))
_NESTED_TIMING = re.compile(rf'\[(?:{_TIMING_NAMES})\]|<(?:{_TIMING_NAMES})>|＜(?:{_TIMING_NAMES})＞')


def split_digixros_condition_units(units: list[dict]) -> list[dict]:
    """Return the existing span shape, with original source units unchanged.

    Plain DigiXros must contain one finite integer reduction and a nonempty
    material body. Unknown/mixed grammars fail closed. Infinite *material
    counts* are opaque body text and are never converted into a finite count.
    No clause IDs, source approvals, or evidence verdicts are produced here.
    """
    if not isinstance(units, list):
        raise ConditionSpanError('condition units must be an ordered list')
    prepared = deepcopy(units)
    transformed = {}
    for index, unit in enumerate(prepared):
        if not isinstance(unit, dict):
            continue  # Let the shared validator give the normal diagnostic.
        raw = unit.get('text')
        if not isinstance(raw, str):
            continue
        text = raw.strip()
        if not text.startswith('DigiXros'):
            continue
        if (unit.get('kind') != 'official-section' or unit.get('field') != 'xros_req'
                or not isinstance(unit.get('label'), str)
                or unit['label'].lower() != 'special play condition'
                or unit.get('condition_type') != 'special_play_condition'
                or not _PREFIX.match(text)):
            raise ConditionSpanError('unsupported plain DigiXros source or grammar')
        body = text[len('DigiXros'):].lstrip()
        if _ANOTHER_ROUTE.search(body):
            raise ConditionSpanError('multiple plain DigiXros routes in one source unit')
        # This narrow adapter supports material keyword references (e.g. Save),
        # but has no grammar for a timing inside a material restriction. Refuse
        # even an apparent reference until that spelling is explicitly tested.
        if _NESTED_TIMING.search(body):
            raise ConditionSpanError('timing marker inside plain DigiXros material body')
        unit['text'] = '[DigiXros]' + text[len('DigiXros'):]
        transformed[index] = body
    result = split_condition_units(prepared)
    for index, body in transformed.items():
        row = result[index]
        expected = [{'span_index': 0, 'kind': 'timing', 'timings': ['DigiXros'],
                     'keyword': None, 'text': body}]
        if row['spans'] != expected:
            raise ConditionSpanError('mixed or lossy plain DigiXros segmentation')
    for index, row in enumerate(result):
        row['source_unit'] = deepcopy(units[index])
    return result
