"""Pure-function parser for cards.json `xros_req` strings.

Recognized shape (this slice): "[Marker] [Name]: Cost N", where Marker is
one of {Digivolve, DNA Digivolve, App Fusion, Burst Digivolve}.

Permissive: any line that doesn't match a known production is returned
verbatim in `XrosReqParseResult.unparsed_lines` rather than raising.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Optional


_MARKER_TO_KIND = {
    "[Digivolve]": "digivolve",
    "[DNA Digivolve]": "dna_digivolve",
    "[App Fusion]": "app_fusion",
    "[Burst Digivolve]": "burst_digivolve",
}

# "[Marker] Lv.N w/[Trait] trait: Cost N"
_RE_LV_TRAIT = re.compile(
    r"^\s*(\[(?:Digivolve|DNA Digivolve|App Fusion|Burst Digivolve)\])"
    r"\s*Lv\.(\d+)\s*w/\[([^\]]+)\]\s*trait\s*:\s*Cost\s*(\d+)\s*$"
)

# "[Marker] Lv.N w/[Name] in name: Cost N"
_RE_LV_NAME_IN_NAME = re.compile(
    r"^\s*(\[(?:Digivolve|DNA Digivolve|App Fusion|Burst Digivolve)\])"
    r"\s*Lv\.(\d+)\s*w/\[([^\]]+)\]\s*in name\s*:\s*Cost\s*(\d+)\s*$"
)

# "[Marker] Lv.N w/[Name] in text: Cost N"
_RE_LV_NAME_IN_TEXT = re.compile(
    r"^\s*(\[(?:Digivolve|DNA Digivolve|App Fusion|Burst Digivolve)\])"
    r"\s*Lv\.(\d+)\s*w/\[([^\]]+)\]\s*in text\s*:\s*Cost\s*(\d+)\s*$"
)

# "[Marker] [Name]: Cost N"
_RE_NAMED_TARGET_ONLY = re.compile(
    r"^\s*(\[(?:Digivolve|DNA Digivolve|App Fusion|Burst Digivolve)\])"
    r"\s*\[([^\]]+)\]\s*:\s*Cost\s*(\d+)\s*$"
)


@dataclass(frozen=True)
class ParsedAltPath:
    kind: str
    from_: Optional[dict]
    materials: Optional[list]
    cost: int


@dataclass(frozen=True)
class XrosReqParseResult:
    parsed: list[ParsedAltPath]
    unparsed_lines: list[str]


def _split_lines(xros_req: str) -> list[str]:
    return [ln.strip() for ln in xros_req.replace("\r\n", "\n").split("\n") if ln.strip()]


def _try_lv_trait(line: str) -> Optional[ParsedAltPath]:
    m = _RE_LV_TRAIT.match(line)
    if not m:
        return None
    return ParsedAltPath(
        kind=_MARKER_TO_KIND[m.group(1)],
        from_={"level_eq": int(m.group(2)), "trait_has": m.group(3)},
        materials=None,
        cost=int(m.group(4)),
    )


def _try_lv_name_in_name(line: str) -> Optional[ParsedAltPath]:
    m = _RE_LV_NAME_IN_NAME.match(line)
    if not m:
        return None
    return ParsedAltPath(
        kind=_MARKER_TO_KIND[m.group(1)],
        from_={"level_eq": int(m.group(2)), "name_contains": m.group(3)},
        materials=None,
        cost=int(m.group(4)),
    )


def _try_lv_name_in_text(line: str) -> Optional[ParsedAltPath]:
    m = _RE_LV_NAME_IN_TEXT.match(line)
    if not m:
        return None
    return ParsedAltPath(
        kind=_MARKER_TO_KIND[m.group(1)],
        from_={"level_eq": int(m.group(2)), "name_in_text": m.group(3)},
        materials=None,
        cost=int(m.group(4)),
    )


def _try_named_target_only(line: str) -> Optional[ParsedAltPath]:
    m = _RE_NAMED_TARGET_ONLY.match(line)
    if not m:
        return None
    marker, name, cost = m.group(1), m.group(2), int(m.group(3))
    return ParsedAltPath(
        kind=_MARKER_TO_KIND[marker],
        from_={"name_is": name},
        materials=None,
        cost=cost,
    )


_PRODUCTIONS = (
    _try_lv_name_in_name,
    _try_lv_name_in_text,
    _try_lv_trait,
    _try_named_target_only,
)


def parse(xros_req: str) -> XrosReqParseResult:
    if not xros_req:
        return XrosReqParseResult(parsed=[], unparsed_lines=[])
    parsed: list[ParsedAltPath] = []
    unparsed: list[str] = []
    for line in _split_lines(xros_req):
        for production in _PRODUCTIONS:
            ap = production(line)
            if ap is not None:
                parsed.append(ap)
                break
        else:
            unparsed.append(line)
    return XrosReqParseResult(parsed=parsed, unparsed_lines=unparsed)
