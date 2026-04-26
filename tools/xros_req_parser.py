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


_PRODUCTIONS = (_try_named_target_only,)


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
