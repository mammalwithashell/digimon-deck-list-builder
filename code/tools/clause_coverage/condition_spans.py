"""Opt-in segmentation of already validated A2 condition source units.

This pure transform is not a source validator. Callers must validate the A2
binding against its complete inputs before passing its ordered units here.
It assigns only local occurrence indices, not clause IDs or evidence verdicts.
The original source unit (including an image correction's full projection)
is retained independently of the text used for segmentation.
"""
from __future__ import annotations

from copy import deepcopy
import re

from .text_split import split_clauses


class ConditionSpanError(ValueError):
    """The unit is outside this explicitly supported segmentation contract."""


_LABELS = frozenset({
    'special play condition', 'special digivolution condition',
    'special evolution condition', 'dna digivolution', 'digixros',
})
_IMAGE_HEADER = '[Special Digivolution Condition]'
_COST_START = re.compile(
    r'^\[(?:Digivolve|DNA Digivolve|Arts Digivolve|Blast Digivolve|Burst Digivolve)\]'
)
_SOURCE_HEADER = re.compile(
    r'\[(?:[^\[\]]*\b(?:Condition|Header|Section)\b[^\[\]]*|Effect|Inherited Effect|Security Effect)\]',
    re.IGNORECASE,
)
_ASSEMBLY_START = re.compile(r'^Assembly(?=\s|$)')
_CONDITION_TIMINGS = frozenset({
    'Digivolve', 'DNA Digivolve', 'Arts Digivolve', 'Blast Digivolve',
    'Burst Digivolve', 'Assembly', 'DigiXros', 'Use Req.',
})
_COST_NAMES = '|'.join(re.escape(name) for name in sorted(_CONDITION_TIMINGS, key=len, reverse=True))
_COST_MARKER = re.compile(
    rf'\[(?:{_COST_NAMES})\]|<(?:{_COST_NAMES})>|＜(?:{_COST_NAMES})＞'
)


def _condition_occurrences(text: str):
    """Cost routes never use the general splitter's compound-timing grouping."""
    markers = list(_COST_MARKER.finditer(text))
    if not markers or text[:markers[0].start()].strip():
        raise ConditionSpanError('unsupported condition grammar before cost marker')
    spans = []
    for index, marker in enumerate(markers):
        end = markers[index + 1].start() if index + 1 < len(markers) else len(text)
        local = split_clauses(text[marker.start():end])
        if (len(local) != 1 or local[0].kind != 'timing'
                or len(local[0].timings) != 1
                or local[0].timings[0] not in _CONDITION_TIMINGS):
            raise ConditionSpanError('unsupported condition grammar within cost route')
        spans.extend(local)
    return spans


def split_condition_units(units: list[dict]) -> list[dict]:
    """Preserve every ordered unit and every locally split span occurrence.

    Empty collections represent no conditions; an empty supplied condition is
    refused. Marker-only spans are meaningful and are preserved. This does not
    deduplicate repeated printings, source sections, or identical alternatives.
    """
    if not isinstance(units, list):
        raise ConditionSpanError('condition units must be an ordered list')
    result = []
    for index, unit in enumerate(units):
        if not isinstance(unit, dict) or unit.get('field') != 'xros_req':
            raise ConditionSpanError('expected an xros_req source unit')
        text = unit.get('text')
        if not isinstance(text, str) or not text.strip():
            raise ConditionSpanError('empty or non-text condition unit')
        text = text.strip()
        kind = unit.get('kind')
        if kind == 'official-section':
            label = unit.get('label')
            if (not isinstance(label, str) or label.lower() not in _LABELS
                    or unit.get('condition_type') != label.lower().replace(' ', '_')):
                raise ConditionSpanError('unsupported condition label/type')
            if _SOURCE_HEADER.search(text):
                raise ConditionSpanError('nested source header is not condition text')
            if label.lower() == 'special play condition':
                # Local source-section boundary permits the plain spelling.
                # Reduction and complete material restrictions stay in the body.
                text = _ASSEMBLY_START.sub('[Assembly]', text, count=1)
        elif kind == 'image-correction':
            # A2 already verifies the complete reviewed image correction.
            # Only this single-section shape has a segmentation contract here.
            if not text.startswith(_IMAGE_HEADER):
                raise ConditionSpanError('unsupported image correction header')
            text = text[len(_IMAGE_HEADER):].lstrip()
            if not _COST_START.match(text) or _SOURCE_HEADER.search(text):
                raise ConditionSpanError('unsupported image correction segmentation')
        else:
            raise ConditionSpanError('unsupported condition source kind')
        spans = _condition_occurrences(text)
        if not spans:
            raise ConditionSpanError('condition produced no spans')
        if any(set(span.timings) - _CONDITION_TIMINGS for span in spans):
            raise ConditionSpanError('effect timing outside condition segmentation scope')
        result.append({
            'unit_index': index,
            'source_unit': deepcopy(unit),
            'spans': [
                {'span_index': span_index, 'kind': span.kind,
                 'timings': list(span.timings), 'keyword': span.keyword,
                 'text': span.text}
                for span_index, span in enumerate(spans)
            ],
        })
    return result
