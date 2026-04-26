# Per-card metadata `.md` pipeline + xros_req parser — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate one focused `.md` per card under `data/card_meta/<set>/<card_id>.md` (parsed `alt_paths:` + verbatim `cards.json` record + optional unparsed lines), driven by a permissive `xros_req` parser, so sub-agents can author Rust DSL YAML from a single per-card context file.

**Architecture:** Three Python modules: a pure-function `xros_req_parser`, a renderer `build_card_meta_md(card_id) -> str` extending `tools/resolve_deck.py`, and a CLI `tools/build_card_meta.py` with `--card / --set / --check / --coverage-check` modes. Output tree is checked into git; CI enforces (a) tree matches generator output and (b) parser coverage doesn't regress against a committed baseline.

**Tech Stack:** Python 3.11+, stdlib only (no new deps), pytest, GitHub Actions.

**Spec:** [docs/superpowers/specs/2026-04-25-card-meta-md-pipeline-design.md](../specs/2026-04-25-card-meta-md-pipeline-design.md)

**Spec deviations:**
- The spec said "no new workflow file." This plan adds `.github/workflows/card-meta-integrity.yml` because that matches the existing `frozen-integrity.yml` pattern (one lightweight workflow per integrity script). No "existing test job" exists in `.github/workflows/` to graft onto.
- The spec's §5.1 grammar listed 4 markers and called DigiXros out-of-scope. Empirical line-shape inventory (run during planning) found `DigiXros Requirements ...` lines exist (24) and several spec-unlisted but high-frequency shapes (`or`, `in text`, `w/o`, slash-trait-list, leading `[X] [X]: Cost N` with no Lv constraint). Task 3 covers the recognized grammar; the rest fall to permissive `## Unparsed xros_req` per spec §5.2.

---

## File structure

**Create:**
- `tools/xros_req_parser.py` — pure parser (parse + ParsedAltPath/XrosReqParseResult dataclasses + YAML-rendering helper)
- `tools/build_card_meta.py` — CLI orchestrator (`__main__` + `--card / --set / --check / --coverage-check`)
- `tests/tools/test_xros_req_parser.py`
- `tests/tools/test_build_card_meta.py`
- `data/card_meta/<set>/<card_id>.md` × ~4,085 (generated, committed)
- `data/card_meta/_coverage.md` (generated, committed)
- `data/card_meta/_coverage_baseline.json` (committed; updated when intentional grammar changes shift coverage)
- `.github/workflows/card-meta-integrity.yml`

**Modify:**
- `tools/resolve_deck.py` — add `build_card_meta_md(card_id: str) -> tuple[str, XrosReqParseResult]` near the bottom of the public API surface (after `resolve_cards`, before the CLI block).

---

## Task 1: xros_req parser — dataclasses + smallest grammar

**Files:**
- Create: `tools/xros_req_parser.py`
- Test:   `tests/tools/test_xros_req_parser.py`

The first slice covers the most common production: `[Marker] [Name]: Cost N`, accounting for ~338 lines on its own.

- [ ] **Step 1: Write the failing test**

```python
# tests/tools/test_xros_req_parser.py
"""Tests for tools.xros_req_parser."""
from __future__ import annotations

import pytest

from tools.xros_req_parser import (
    ParsedAltPath,
    XrosReqParseResult,
    parse,
)


def test_empty_returns_empty_result():
    result = parse("")
    assert result == XrosReqParseResult(parsed=[], unparsed_lines=[])


def test_named_target_only_digivolve():
    # AD1-001 / BT17-007 shape: "[Digivolve] [Koromon]: Cost 0"
    result = parse("[Digivolve] [Koromon]: Cost 0")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"name_is": "Koromon"},
            materials=None,
            cost=0,
        )
    ]


def test_unrecognized_line_is_unparsed():
    raw = "If 2 such cards are linked together, stack the link card on top and digivolve."
    result = parse(raw)
    assert result.parsed == []
    assert result.unparsed_lines == [raw]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'tools.xros_req_parser'`.

- [ ] **Step 3: Write the minimal parser**

```python
# tools/xros_req_parser.py
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/xros_req_parser.py tests/tools/test_xros_req_parser.py
git commit -m "tools: xros_req_parser scaffolding + named-target production"
```

---

## Task 2: xros_req parser — Lv-trait / Lv-name / Lv-text productions

Covers ~485 additional lines (Lv.N w/[X] trait + Lv.N w/[X] in name + Lv.N w/[X] in text).

**Files:**
- Modify: `tools/xros_req_parser.py`
- Modify: `tests/tools/test_xros_req_parser.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/tools/test_xros_req_parser.py`:

```python
def test_lv_trait_digivolve():
    result = parse("[Digivolve] Lv.5 w/[Xros Heart] trait: Cost 2")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 5, "trait_has": "Xros Heart"},
            materials=None,
            cost=2,
        )
    ]


def test_lv_name_in_name_digivolve():
    result = parse("[Digivolve] Lv.5 w/[Greymon] in name: Cost 3")
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 5, "name_contains": "Greymon"},
            materials=None,
            cost=3,
        )
    ]


def test_lv_name_in_text_digivolve():
    # AD1-001: "Lv.3 w/[Omnimon] in text"
    result = parse("[Digivolve] Lv.3 w/[Omnimon] in text: Cost 2")
    assert result.parsed == [
        ParsedAltPath(
            kind="digivolve",
            from_={"level_eq": 3, "name_in_text": "Omnimon"},
            materials=None,
            cost=2,
        )
    ]


def test_multiline_xros_req_each_line_parsed_independently():
    # AD1-004 shape
    raw = (
        "[Digivolve] Lv.5 w/[Greymon] in name: Cost 3\r\n"
        "[Digivolve] Lv.5 w/[Hero] trait: Cost 3"
    )
    result = parse(raw)
    assert result.unparsed_lines == []
    assert len(result.parsed) == 2
    assert result.parsed[0].from_ == {"level_eq": 5, "name_contains": "Greymon"}
    assert result.parsed[1].from_ == {"level_eq": 5, "trait_has": "Hero"}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 4 new failures (existing 3 still pass).

- [ ] **Step 3: Add productions to the parser**

Insert these regexes and helpers above `_PRODUCTIONS = ...` in `tools/xros_req_parser.py`:

```python
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
```

Update the `_PRODUCTIONS` tuple — order matters; more specific shapes must precede less specific ones to avoid mis-classification:

```python
_PRODUCTIONS = (
    _try_lv_name_in_name,
    _try_lv_name_in_text,
    _try_lv_trait,
    _try_named_target_only,
)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/xros_req_parser.py tests/tools/test_xros_req_parser.py
git commit -m "tools: xros_req_parser Lv-trait / Lv-name / Lv-text productions"
```

---

## Task 3: xros_req parser — DigiXros materials + DNA/App-Fusion `&`-list

Adds `DigiXros Requirements [Trait] [Name] x N` lines (24 occurrences) and `&`-conjoined material lists for DNA / App Fusion (`[A] & [B]`, ~24 lines).

**Files:**
- Modify: `tools/xros_req_parser.py`
- Modify: `tests/tools/test_xros_req_parser.py`

- [ ] **Step 1: Add failing tests**

```python
def test_app_fusion_two_materials():
    # AD1-005: "[App Fusion] [Globemon] & [Charismon]: Cost 0"
    result = parse("[App Fusion] [Globemon] & [Charismon]: Cost 0")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="app_fusion",
            from_=None,
            materials=[{"name_is": "Globemon"}, {"name_is": "Charismon"}],
            cost=0,
        )
    ]


def test_app_fusion_three_materials():
    result = parse("[App Fusion] [A] & [B] & [C]: Cost 1")
    assert result.parsed[0].materials == [
        {"name_is": "A"},
        {"name_is": "B"},
        {"name_is": "C"},
    ]


def test_digixros_requirements_simple():
    # 24 lines like: "DigiXros Requirements [Xros Heart] [Greymon] x 2"
    result = parse("DigiXros Requirements [Xros Heart] [Greymon] x 2")
    assert result.unparsed_lines == []
    assert result.parsed == [
        ParsedAltPath(
            kind="digixros",
            from_=None,
            materials=[{"trait_has": "Xros Heart", "name_is": "Greymon", "count_eq": 2}],
            cost=0,
        )
    ]


def test_descriptor_lines_are_unparsed():
    # 62 lines: "Stack the 2 specified Digimon and digivolve unsuspended."
    raw = "Stack the 2 specified Digimon and digivolve unsuspended."
    result = parse(raw)
    assert result.parsed == []
    assert result.unparsed_lines == [raw]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 4 new failures.

- [ ] **Step 3: Add productions**

Add to `tools/xros_req_parser.py`:

```python
# "[App Fusion] [A] & [B] (& [C])*: Cost N"  — also covers DNA Digivolve `&`-lists
_RE_AMP_MATERIALS = re.compile(
    r"^\s*(\[(?:DNA Digivolve|App Fusion|Burst Digivolve)\])"
    r"\s*((?:\[[^\]]+\]\s*&\s*)+\[[^\]]+\])"
    r"\s*:\s*Cost\s*(\d+)\s*$"
)

# "DigiXros Requirements [Trait] [Name] x N"
_RE_DIGIXROS_REQ = re.compile(
    r"^\s*DigiXros Requirements\s*\[([^\]]+)\]\s*\[([^\]]+)\]\s*x\s*(\d+)\s*$"
)


def _try_amp_materials(line: str) -> Optional[ParsedAltPath]:
    m = _RE_AMP_MATERIALS.match(line)
    if not m:
        return None
    names = re.findall(r"\[([^\]]+)\]", m.group(2))
    return ParsedAltPath(
        kind=_MARKER_TO_KIND[m.group(1)],
        from_=None,
        materials=[{"name_is": n} for n in names],
        cost=int(m.group(3)),
    )


def _try_digixros_requirements(line: str) -> Optional[ParsedAltPath]:
    m = _RE_DIGIXROS_REQ.match(line)
    if not m:
        return None
    return ParsedAltPath(
        kind="digixros",
        from_=None,
        materials=[{"trait_has": m.group(1), "name_is": m.group(2), "count_eq": int(m.group(3))}],
        cost=0,
    )
```

Update the dispatch tuple — DigiXros first (its line shape doesn't have a leading `[Marker]` so it could otherwise be misclassified by a future production):

```python
_PRODUCTIONS = (
    _try_digixros_requirements,
    _try_amp_materials,
    _try_lv_name_in_name,
    _try_lv_name_in_text,
    _try_lv_trait,
    _try_named_target_only,
)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 11 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/xros_req_parser.py tests/tools/test_xros_req_parser.py
git commit -m "tools: xros_req_parser DigiXros + &-materials productions"
```

---

## Task 4: xros_req parser — YAML-rendering helper

The renderer needs a deterministic `ParsedAltPath -> YAML lines` converter that matches the DSL spec's `alt_paths:` shape.

**Files:**
- Modify: `tools/xros_req_parser.py`
- Modify: `tests/tools/test_xros_req_parser.py`

- [ ] **Step 1: Add failing tests**

```python
from tools.xros_req_parser import render_alt_paths_yaml


def test_render_empty():
    assert render_alt_paths_yaml([]) == "_(none)_"


def test_render_named_target_only():
    paths = [ParsedAltPath(kind="digivolve", from_={"name_is": "Koromon"}, materials=None, cost=0)]
    assert render_alt_paths_yaml(paths) == (
        "- kind: digivolve\n"
        "  from: { name_is: \"Koromon\" }\n"
        "  cost: 0"
    )


def test_render_lv_trait():
    paths = [ParsedAltPath(
        kind="digivolve", from_={"level_eq": 5, "trait_has": "Xros Heart"},
        materials=None, cost=2,
    )]
    assert render_alt_paths_yaml(paths) == (
        "- kind: digivolve\n"
        "  from: { level_eq: 5, trait_has: \"Xros Heart\" }\n"
        "  cost: 2"
    )


def test_render_amp_materials():
    paths = [ParsedAltPath(
        kind="app_fusion", from_=None,
        materials=[{"name_is": "Globemon"}, {"name_is": "Charismon"}],
        cost=0,
    )]
    assert render_alt_paths_yaml(paths) == (
        "- kind: app_fusion\n"
        "  materials:\n"
        "    - { name_is: \"Globemon\" }\n"
        "    - { name_is: \"Charismon\" }\n"
        "  cost: 0"
    )


def test_render_multiple_paths_separated_by_blank_line():
    paths = [
        ParsedAltPath(kind="digivolve", from_={"level_eq": 5, "name_contains": "Greymon"}, materials=None, cost=3),
        ParsedAltPath(kind="digivolve", from_={"level_eq": 5, "trait_has": "Hero"}, materials=None, cost=3),
    ]
    out = render_alt_paths_yaml(paths)
    assert out == (
        "- kind: digivolve\n"
        "  from: { level_eq: 5, name_contains: \"Greymon\" }\n"
        "  cost: 3\n"
        "- kind: digivolve\n"
        "  from: { level_eq: 5, trait_has: \"Hero\" }\n"
        "  cost: 3"
    )
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 5 new failures.

- [ ] **Step 3: Add the renderer**

Append to `tools/xros_req_parser.py`:

```python
def _flow_dict(d: dict) -> str:
    """Render a dict as YAML flow style preserving insertion order.

    Strings are double-quoted unconditionally; ints are bare. Matches the
    `from: { name_is: "Koromon" }` style used in the DSL spec.
    """
    parts = []
    for k, v in d.items():
        if isinstance(v, str):
            parts.append(f'{k}: "{v}"')
        elif isinstance(v, int):
            parts.append(f"{k}: {v}")
        else:
            raise TypeError(f"unexpected value type for {k}: {type(v).__name__}")
    return "{ " + ", ".join(parts) + " }"


def _render_one(path: ParsedAltPath) -> str:
    lines = [f"- kind: {path.kind}"]
    if path.from_ is not None:
        lines.append(f"  from: {_flow_dict(path.from_)}")
    if path.materials is not None:
        lines.append("  materials:")
        for mat in path.materials:
            lines.append(f"    - {_flow_dict(mat)}")
    lines.append(f"  cost: {path.cost}")
    return "\n".join(lines)


def render_alt_paths_yaml(paths: list[ParsedAltPath]) -> str:
    """Render parsed alt paths as a YAML fragment matching DSL spec §3."""
    if not paths:
        return "_(none)_"
    return "\n".join(_render_one(p) for p in paths)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_xros_req_parser.py -v`
Expected: 16 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/xros_req_parser.py tests/tools/test_xros_req_parser.py
git commit -m "tools: xros_req_parser YAML renderer for ParsedAltPath"
```

---

## Task 5: `tools/resolve_deck.py::build_card_meta_md`

Renders the full `.md` body for a single card.

**Files:**
- Modify: `tools/resolve_deck.py` (append to public surface, before the `if __name__ == "__main__":` block)
- Modify: `tests/tools/test_resolve_deck.py`

- [ ] **Step 1: Add failing test**

Append to `tests/tools/test_resolve_deck.py`:

```python
def test_build_card_meta_md_renders_known_card():
    from tools.resolve_deck import build_card_meta_md

    body, parse_result = build_card_meta_md("BT17-007")

    # H1 header
    assert body.splitlines()[0] == "# BT17-007 — Agumon"

    # Alt paths block: BT17-007's xros_req is "[Digivolve] [Koromon]: Cost 0"
    assert "## Alt paths (parsed from xros_req)" in body
    assert '- kind: digivolve\n  from: { name_is: "Koromon" }\n  cost: 0' in body

    # Source record block: verbatim cards.json entry
    assert "## Source record" in body
    assert "```json" in body
    assert '"card_id": "BT17-007"' in body

    # No unparsed section for this card
    assert "## Unparsed xros_req" not in body

    # parse_result is the same XrosReqParseResult the renderer used
    assert len(parse_result.parsed) == 1
    assert parse_result.unparsed_lines == []


def test_build_card_meta_md_includes_unparsed_block_when_present():
    from tools.resolve_deck import build_card_meta_md

    # AD1-005's xros_req has a parsed line + a descriptor line
    body, parse_result = build_card_meta_md("AD1-005")

    assert "## Unparsed xros_req" in body
    assert parse_result.unparsed_lines  # non-empty
    # The unparsed text should appear verbatim in the file body
    for line in parse_result.unparsed_lines:
        assert line in body


def test_build_card_meta_md_xros_req_absent_emits_none_marker():
    from tools.resolve_deck import build_card_meta_md

    # Pick a card with no xros_req — most Tamers and Options have none.
    # ST2-13 Hammer Spark is an Option with no xros_req.
    body, _ = build_card_meta_md("ST2-13")
    assert "## Alt paths (parsed from xros_req)" in body
    # The body between the alt paths header and the source record header
    # should contain "_(none)_".
    alt_section = body.split("## Alt paths (parsed from xros_req)", 1)[1]
    alt_section = alt_section.split("## Source record", 1)[0]
    assert "_(none)_" in alt_section
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_resolve_deck.py -k "card_meta_md" -v`
Expected: 3 failures (`AttributeError` / `ImportError` for `build_card_meta_md`).

- [ ] **Step 3: Add `build_card_meta_md` to `tools/resolve_deck.py`**

Append the following before the `if __name__ == "__main__":` block (or before any CLI-related top-level code, near the other public API functions):

```python
import json as _json

from tools.xros_req_parser import (
    XrosReqParseResult,
    parse as _parse_xros_req,
    render_alt_paths_yaml as _render_alt_paths_yaml,
)

_CARDS_JSON_PATH = _PROJECT_ROOT / "data" / "cards.json"
_CARDS_JSON_CACHE: Optional[dict] = None


def _load_cards_json_raw() -> dict:
    """Load `data/cards.json` once per process and cache."""
    global _CARDS_JSON_CACHE
    if _CARDS_JSON_CACHE is None:
        with open(_CARDS_JSON_PATH, "r", encoding="utf-8") as f:
            _CARDS_JSON_CACHE = _json.load(f)
    return _CARDS_JSON_CACHE


def build_card_meta_md(card_id: str) -> tuple[str, XrosReqParseResult]:
    """Render the per-card metadata `.md` body for `card_id`.

    Returns (body, parse_result). The caller owns file I/O. Raises KeyError
    if `card_id` is not present in `data/cards.json`.
    """
    cards = _load_cards_json_raw()
    if card_id not in cards:
        raise KeyError(card_id)
    record = cards[card_id]

    name = record.get("card_name_eng") or ""
    xros_req = record.get("xros_req") or ""
    parse_result = _parse_xros_req(xros_req)
    alt_paths_yaml = _render_alt_paths_yaml(parse_result.parsed)

    record_json = _json.dumps(record, indent=2, sort_keys=True, ensure_ascii=False)

    parts = [
        f"# {card_id} — {name}",
        "",
        "## Alt paths (parsed from xros_req)",
        "",
        alt_paths_yaml,
        "",
        "## Source record",
        "",
        "```json",
        record_json,
        "```",
    ]
    if parse_result.unparsed_lines:
        parts += [
            "",
            "## Unparsed xros_req",
            "",
            "```text",
            *parse_result.unparsed_lines,
            "```",
        ]
    parts.append("")  # trailing newline
    body = "\n".join(parts)
    return body, parse_result
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_resolve_deck.py -k "card_meta_md" -v`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/resolve_deck.py tests/tools/test_resolve_deck.py
git commit -m "tools: resolve_deck.build_card_meta_md renderer"
```

---

## Task 6: `tools/build_card_meta.py` CLI — `--card`, `--set`, default-all

The minimal CLI that writes files. Check / coverage modes come in Task 7.

**Files:**
- Create: `tools/build_card_meta.py`
- Test:   `tests/tools/test_build_card_meta.py`

- [ ] **Step 1: Add failing test**

```python
# tests/tools/test_build_card_meta.py
"""Tests for tools.build_card_meta CLI."""
from __future__ import annotations

from pathlib import Path

import pytest

from tools.build_card_meta import build_one, set_id_from_card_id, write_card_meta


def test_set_id_from_card_id_standard():
    assert set_id_from_card_id("BT17-007") == "bt17"
    assert set_id_from_card_id("ST2-13") == "st2"
    assert set_id_from_card_id("AD1-005") == "ad1"


def test_set_id_from_card_id_promo_falls_back_to_misc():
    # Promo-style card_ids without a hyphen, or with non-set prefixes
    assert set_id_from_card_id("PROMO123") == "_misc"


def test_build_one_writes_file_with_lf_newlines(tmp_path: Path):
    # write_card_meta(card_id, root) must produce <root>/<set>/<card_id>.md
    out = write_card_meta("BT17-007", tmp_path)
    assert out == tmp_path / "bt17" / "BT17-007.md"
    assert out.exists()
    raw = out.read_bytes()
    # Reject CRLF — Windows write_text default would corrupt diffability.
    assert b"\r\n" not in raw
    # H1 starts the file
    assert raw.decode("utf-8").startswith("# BT17-007 — Agumon")


def test_build_one_returns_parse_stats():
    # build_one returns (card_id, n_parsed, n_unparsed) without writing
    cid, n_parsed, n_unparsed = build_one("BT17-007")
    assert cid == "BT17-007"
    assert n_parsed == 1
    assert n_unparsed == 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -v`
Expected: `ModuleNotFoundError`.

- [ ] **Step 3: Implement the CLI module**

```python
# tools/build_card_meta.py
"""CLI: regenerate per-card metadata `.md` files under data/card_meta/.

Usage:
  python -m tools.build_card_meta                    # rebuild all
  python -m tools.build_card_meta --card BT17-007
  python -m tools.build_card_meta --set bt17
  python -m tools.build_card_meta --check            # CI: rebuild to tempdir, diff
  python -m tools.build_card_meta --coverage-check   # CI: assert no coverage regression

Tree layout: data/card_meta/<set_lower>/<card_id>.md, plus _coverage.md and
_coverage_baseline.json at the root.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from tools.resolve_deck import _load_cards_json_raw, build_card_meta_md  # noqa: E402

CARD_META_ROOT = _PROJECT_ROOT / "data" / "card_meta"


def set_id_from_card_id(card_id: str) -> str:
    """Bucket a card_id into a set-level subdirectory.

    Standard cards (BT17-007, ST2-13, AD1-005) bucket by lowercase prefix.
    Card_ids without a hyphen bucket into `_misc`.
    """
    if "-" not in card_id:
        return "_misc"
    return card_id.split("-", 1)[0].lower()


def write_card_meta(card_id: str, root: Path) -> Path:
    body, _ = build_card_meta_md(card_id)
    set_dir = root / set_id_from_card_id(card_id)
    set_dir.mkdir(parents=True, exist_ok=True)
    out = set_dir / f"{card_id}.md"
    # Force LF on Windows; the file is checked in and must diff stably.
    out.write_text(body, encoding="utf-8", newline="\n")
    return out


def build_one(card_id: str) -> tuple[str, int, int]:
    """Build one card's .md and return (card_id, n_parsed, n_unparsed) without writing."""
    _, parse_result = build_card_meta_md(card_id)
    return card_id, len(parse_result.parsed), len(parse_result.unparsed_lines)


def _all_card_ids() -> list[str]:
    return sorted(_load_cards_json_raw().keys())


def _filter_by_set(card_ids: list[str], set_id: str) -> list[str]:
    target = set_id.lower()
    return [c for c in card_ids if set_id_from_card_id(c) == target]


def cmd_build(args: argparse.Namespace) -> int:
    if args.card:
        ids = [args.card]
    elif args.set:
        ids = _filter_by_set(_all_card_ids(), args.set)
        if not ids:
            print(f"no cards matched set {args.set!r}", file=sys.stderr)
            return 2
    else:
        ids = _all_card_ids()
    CARD_META_ROOT.mkdir(parents=True, exist_ok=True)
    for cid in ids:
        write_card_meta(cid, CARD_META_ROOT)
    print(f"wrote {len(ids)} card meta files to {CARD_META_ROOT}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="build_card_meta")
    parser.add_argument("--card", help="rebuild only this card_id")
    parser.add_argument("--set", help="rebuild only cards in this set (lowercase prefix)")
    parser.add_argument("--check", action="store_true", help="rebuild to tempdir and diff vs disk")
    parser.add_argument("--coverage-check", action="store_true", help="assert coverage didn't regress")
    args = parser.parse_args(argv)

    if args.check or args.coverage_check:
        # Stubs until Task 7. Make them visible if accidentally invoked.
        print("--check / --coverage-check not implemented yet (see Task 7)", file=sys.stderr)
        return 2
    return cmd_build(args)


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -v`
Expected: 4 passed.

- [ ] **Step 5: Smoke-run the CLI for one set**

Run: `PYTHONIOENCODING=utf-8 python -m tools.build_card_meta --set ad1`
Expected: prints `wrote N card meta files to .../data/card_meta`. Then `ls data/card_meta/ad1/ | head` shows `AD1-001.md` etc.

- [ ] **Step 6: Clean up the smoke-run output (we'll do the real bulk run in Task 9)**

Run: `rm -rf data/card_meta/ad1`

- [ ] **Step 7: Commit**

```bash
git add tools/build_card_meta.py tests/tools/test_build_card_meta.py
git commit -m "tools: build_card_meta CLI (--card / --set / default-all)"
```

---

## Task 7: CLI `--check` mode — tree matches generator output

**Files:**
- Modify: `tools/build_card_meta.py`
- Modify: `tests/tools/test_build_card_meta.py`

- [ ] **Step 1: Add failing test**

Append to `tests/tools/test_build_card_meta.py`:

```python
def test_check_mode_passes_when_tree_matches(tmp_path: Path, monkeypatch):
    from tools import build_card_meta as m
    # Point CARD_META_ROOT at a fresh tempdir, populate, then check.
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    m.write_card_meta("BT17-007", tmp_path)
    m.write_card_meta("ST2-13", tmp_path)
    rc = m.cmd_check(card_ids=["BT17-007", "ST2-13"])
    assert rc == 0


def test_check_mode_fails_on_mismatch(tmp_path: Path, monkeypatch, capsys):
    from tools import build_card_meta as m
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    m.write_card_meta("BT17-007", tmp_path)
    # Corrupt the file
    out = tmp_path / "bt17" / "BT17-007.md"
    out.write_text("STALE\n", encoding="utf-8", newline="\n")
    rc = m.cmd_check(card_ids=["BT17-007"])
    assert rc == 1
    captured = capsys.readouterr()
    assert "BT17-007" in captured.err


def test_check_mode_fails_on_missing_file(tmp_path: Path, monkeypatch, capsys):
    from tools import build_card_meta as m
    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    rc = m.cmd_check(card_ids=["BT17-007"])
    assert rc == 1
    captured = capsys.readouterr()
    assert "missing" in captured.err.lower() or "BT17-007" in captured.err
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -k "check_mode" -v`
Expected: 3 failures (`AttributeError: cmd_check`).

- [ ] **Step 3: Implement `cmd_check`**

In `tools/build_card_meta.py`, add:

```python
def cmd_check(card_ids: list[str] | None = None) -> int:
    """Rebuild every card to memory, compare against the on-disk tree.

    Returns 0 if every file matches; 1 if any file is missing or differs.
    Prints diffs to stderr.
    """
    ids = card_ids if card_ids is not None else _all_card_ids()
    failures: list[str] = []
    for cid in ids:
        body, _ = build_card_meta_md(cid)
        on_disk = CARD_META_ROOT / set_id_from_card_id(cid) / f"{cid}.md"
        if not on_disk.exists():
            failures.append(f"{cid}: missing on disk at {on_disk}")
            continue
        actual = on_disk.read_text(encoding="utf-8")
        if actual != body:
            failures.append(f"{cid}: contents differ from generator output")
    if failures:
        for f in failures:
            print(f, file=sys.stderr)
        print(
            f"--check failed for {len(failures)}/{len(ids)} cards. "
            "Run `python -m tools.build_card_meta` and commit the diff.",
            file=sys.stderr,
        )
        return 1
    print(f"--check OK ({len(ids)} cards match on disk)")
    return 0
```

Wire it into `main`:

```python
    if args.check:
        return cmd_check()
    if args.coverage_check:
        print("--coverage-check not implemented yet (see Task 8)", file=sys.stderr)
        return 2
    return cmd_build(args)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -k "check_mode" -v`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add tools/build_card_meta.py tests/tools/test_build_card_meta.py
git commit -m "tools: build_card_meta --check mode"
```

---

## Task 8: Coverage report + baseline + `--coverage-check`

**Files:**
- Modify: `tools/build_card_meta.py`
- Modify: `tests/tools/test_build_card_meta.py`

- [ ] **Step 1: Add failing tests**

Append to `tests/tools/test_build_card_meta.py`:

```python
def test_compute_coverage_buckets_correctly():
    from tools.build_card_meta import compute_coverage

    # (card_id, n_parsed, n_unparsed)
    stats = [
        ("A", 1, 0),  # fully parsed
        ("B", 2, 0),  # fully parsed
        ("C", 1, 1),  # partially parsed
        ("D", 0, 2),  # wholly unparsed
        ("E", 0, 0),  # no xros_req at all — counts as fully parsed (nothing to fail on)
    ]
    cov = compute_coverage(stats)
    assert cov["fully_parsed"] == 3
    assert cov["partially_parsed"] == 1
    assert cov["wholly_unparsed"] == 1


def test_coverage_check_passes_when_equal_to_baseline(tmp_path: Path, monkeypatch):
    from tools import build_card_meta as m

    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    baseline = tmp_path / "_coverage_baseline.json"
    baseline.write_text(
        json.dumps({"partially_parsed": 0, "wholly_unparsed": 0}),
        encoding="utf-8",
    )

    # Inject deterministic stats: zero failures
    rc = m.cmd_coverage_check(stats_override=[("BT17-007", 1, 0)])
    assert rc == 0


def test_coverage_check_fails_on_regression(tmp_path: Path, monkeypatch, capsys):
    from tools import build_card_meta as m

    monkeypatch.setattr(m, "CARD_META_ROOT", tmp_path)
    baseline = tmp_path / "_coverage_baseline.json"
    baseline.write_text(
        json.dumps({"partially_parsed": 0, "wholly_unparsed": 0}),
        encoding="utf-8",
    )

    # One newly partially-parsed card
    rc = m.cmd_coverage_check(stats_override=[("X", 1, 1)])
    assert rc == 1
    err = capsys.readouterr().err
    assert "regress" in err.lower()
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -k "coverage" -v`
Expected: 3 failures.

- [ ] **Step 3: Implement coverage logic**

Add to `tools/build_card_meta.py`:

```python
import datetime

COVERAGE_BASELINE_PATH_NAME = "_coverage_baseline.json"
COVERAGE_REPORT_PATH_NAME = "_coverage.md"


def compute_coverage(stats: list[tuple[str, int, int]]) -> dict:
    """Bucket each card. A card with zero unparsed lines is fully parsed
    (including cards with no xros_req at all). One unparsed line + at least
    one parsed line is partially parsed. All-unparsed-no-parsed is wholly
    unparsed.
    """
    fully = partial = wholly = 0
    partial_cards: list[str] = []
    wholly_cards: list[str] = []
    for cid, n_parsed, n_unparsed in stats:
        if n_unparsed == 0:
            fully += 1
        elif n_parsed > 0:
            partial += 1
            partial_cards.append(cid)
        else:
            wholly += 1
            wholly_cards.append(cid)
    return {
        "fully_parsed": fully,
        "partially_parsed": partial,
        "wholly_unparsed": wholly,
        "partial_cards": partial_cards,
        "wholly_cards": wholly_cards,
    }


def write_coverage_report(stats: list[tuple[str, int, int]]) -> Path:
    cov = compute_coverage(stats)
    total = len(stats)
    lines = [
        "# xros_req parser coverage",
        "",
        f"Generated: {datetime.datetime.utcnow().isoformat(timespec='seconds')}Z",
        "",
        f"- Total cards: {total}",
        f"- Fully parsed (incl. no xros_req): {cov['fully_parsed']}",
        f"- Partially parsed: {cov['partially_parsed']}",
        f"- Wholly unparsed (with xros_req): {cov['wholly_unparsed']}",
        "",
        "## Cards with partial xros_req parses",
        "",
    ]
    if cov["partial_cards"]:
        lines += [f"- {c}" for c in sorted(cov["partial_cards"])]
    else:
        lines.append("_(none)_")
    lines += ["", "## Cards with wholly unparsed xros_req", ""]
    if cov["wholly_cards"]:
        lines += [f"- {c}" for c in sorted(cov["wholly_cards"])]
    else:
        lines.append("_(none)_")
    lines.append("")
    out = CARD_META_ROOT / COVERAGE_REPORT_PATH_NAME
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    return out


def _load_baseline() -> dict:
    path = CARD_META_ROOT / COVERAGE_BASELINE_PATH_NAME
    if not path.exists():
        return {"partially_parsed": 0, "wholly_unparsed": 0}
    return json.loads(path.read_text(encoding="utf-8"))


def cmd_coverage_check(stats_override: list[tuple[str, int, int]] | None = None) -> int:
    if stats_override is not None:
        stats = stats_override
    else:
        stats = [build_one(c) for c in _all_card_ids()]
    cov = compute_coverage(stats)
    baseline = _load_baseline()
    regressed = (
        cov["partially_parsed"] > baseline.get("partially_parsed", 0)
        or cov["wholly_unparsed"] > baseline.get("wholly_unparsed", 0)
    )
    if regressed:
        print(
            "coverage regressed: "
            f"partial {cov['partially_parsed']} (baseline {baseline.get('partially_parsed', 0)}), "
            f"wholly {cov['wholly_unparsed']} (baseline {baseline.get('wholly_unparsed', 0)})",
            file=sys.stderr,
        )
        return 1
    print(
        f"coverage OK (partial {cov['partially_parsed']}, wholly {cov['wholly_unparsed']})"
    )
    return 0
```

Update the bulk-build path in `cmd_build` to also write the coverage report and refresh the baseline (so the committed tree always matches the latest run):

```python
def cmd_build(args: argparse.Namespace) -> int:
    if args.card:
        ids = [args.card]
    elif args.set:
        ids = _filter_by_set(_all_card_ids(), args.set)
        if not ids:
            print(f"no cards matched set {args.set!r}", file=sys.stderr)
            return 2
    else:
        ids = _all_card_ids()
    CARD_META_ROOT.mkdir(parents=True, exist_ok=True)
    stats: list[tuple[str, int, int]] = []
    for cid in ids:
        body, parse_result = build_card_meta_md(cid)
        set_dir = CARD_META_ROOT / set_id_from_card_id(cid)
        set_dir.mkdir(parents=True, exist_ok=True)
        (set_dir / f"{cid}.md").write_text(body, encoding="utf-8", newline="\n")
        stats.append((cid, len(parse_result.parsed), len(parse_result.unparsed_lines)))
    # Only refresh the report + baseline on a full build (--card/--set are partial).
    if not args.card and not args.set:
        write_coverage_report(stats)
        cov = compute_coverage(stats)
        baseline_payload = {
            "partially_parsed": cov["partially_parsed"],
            "wholly_unparsed": cov["wholly_unparsed"],
        }
        (CARD_META_ROOT / COVERAGE_BASELINE_PATH_NAME).write_text(
            json.dumps(baseline_payload, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
    print(f"wrote {len(ids)} card meta files to {CARD_META_ROOT}")
    return 0
```

Note: this means a full `python -m tools.build_card_meta` run **resets the baseline to the current coverage**. Because the tree itself is committed, any drop in coverage shows up as a baseline JSON change in the same commit — reviewable and intentional. CI's `--coverage-check` only fails when running on a tree where the baseline hasn't been updated yet (i.e. someone forgot to regen).

Wire `--coverage-check`:

```python
    if args.coverage_check:
        return cmd_coverage_check()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PYTHONIOENCODING=utf-8 python -m pytest tests/tools/test_build_card_meta.py -v`
Expected: all build_card_meta tests pass (10 total: 4 from Task 6 + 3 from Task 7 + 3 from Task 8).

- [ ] **Step 5: Commit**

```bash
git add tools/build_card_meta.py tests/tools/test_build_card_meta.py
git commit -m "tools: build_card_meta coverage report + --coverage-check"
```

---

## Task 9: Bulk regenerate the tree + commit

**Files:**
- Create (auto): `data/card_meta/<set>/<card_id>.md` × ~4,085
- Create (auto): `data/card_meta/_coverage.md`
- Create (auto): `data/card_meta/_coverage_baseline.json`

- [ ] **Step 1: Generate the full tree**

Run: `PYTHONIOENCODING=utf-8 python -m tools.build_card_meta`
Expected: `wrote 4085 card meta files to .../data/card_meta`. Should take well under a minute.

- [ ] **Step 2: Spot-check a few files**

```bash
ls data/card_meta/ | head -10
ls data/card_meta/bt17/ | head -5
cat data/card_meta/bt17/BT17-007.md
cat data/card_meta/_coverage.md
cat data/card_meta/_coverage_baseline.json
```

Verify:
- The file tree has one directory per set, no `_misc/` for hyphenated card_ids.
- BT17-007.md has the H1, alt paths block (named-target Koromon), source record, no unparsed section.
- `_coverage.md` lists realistic counts (most cards fully parsed, a small number partial / wholly unparsed).
- Baseline JSON matches the report's totals.

- [ ] **Step 3: Run the check + coverage-check round-trip**

```bash
PYTHONIOENCODING=utf-8 python -m tools.build_card_meta --check
PYTHONIOENCODING=utf-8 python -m tools.build_card_meta --coverage-check
```

Both should exit 0.

- [ ] **Step 4: Commit the generated tree**

```bash
git add data/card_meta/
git commit -m "data: initial card_meta tree (4085 cards) + xros_req coverage baseline"
```

---

## Task 10: CI workflow — `card-meta-integrity.yml`

Mirror `.github/workflows/frozen-integrity.yml`'s pattern.

**Files:**
- Create: `.github/workflows/card-meta-integrity.yml`

- [ ] **Step 1: Write the workflow**

```yaml
# .github/workflows/card-meta-integrity.yml
name: Card Meta Integrity

on:
  pull_request:
  push:
    branches: ["main"]

jobs:
  check-card-meta:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Python
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: Verify card_meta tree matches generator output
        env:
          PYTHONIOENCODING: utf-8
        run: python -m tools.build_card_meta --check

      - name: Verify xros_req parser coverage hasn't regressed
        env:
          PYTHONIOENCODING: utf-8
        run: python -m tools.build_card_meta --coverage-check
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/card-meta-integrity.yml
git commit -m "ci: card-meta-integrity workflow (--check + --coverage-check)"
```

- [ ] **Step 3: (Optional, if pushing) push the branch and watch the workflow on the PR**

The `--check` job will fail if anyone modifies `tools/xros_req_parser.py` or `tools/resolve_deck.py::build_card_meta_md` without regenerating `data/card_meta/`. The fix message printed by `--check` tells contributors exactly what to run.

---

## Self-review (run by the planner, not the executor)

**Spec coverage:**
- §3 file layout — Task 6 (`set_id_from_card_id` + `write_card_meta`).
- §4 file format (H1 / alt paths / source record / optional unparsed) — Task 5.
- §5 grammar (4 markers) — Tasks 1–3 cover the four spec markers + DigiXros (deviation noted at top).
- §5.2 permissive failure mode + coverage report — Tasks 1 & 8.
- §6.1 parser surface — Tasks 1–4.
- §6.2 `build_card_meta_md` extension — Task 5.
- §6.3 CLI surface (`--card`, `--set`, `--check`, `--coverage-check`) — Tasks 6–8.
- §7 CI integration — Task 10.
- §8 migration / rollout — Tasks 9–10.

**Placeholders:** none — every step has complete code or an explicit command.

**Type consistency:** `XrosReqParseResult.parsed: list[ParsedAltPath]` and `unparsed_lines: list[str]` are used identically across Tasks 1, 4, 5. `build_card_meta_md(card_id) -> tuple[str, XrosReqParseResult]` is used identically in Task 5's tests, Task 6's `build_one` and `write_card_meta`, and Task 7's `cmd_check`. `compute_coverage(stats: list[tuple[str, int, int]])` matches `build_one`'s return signature.

**Gaps:** the parser only covers ~75–80% of the empirically observed `xros_req` line shapes (the four most common shapes). The remaining ~20% (compound `or`, slash-trait-list, `w/o` exclusion, fullwidth `＜Save＞`, `While you have ...`, descriptor-only lines) deliberately fall to `## Unparsed xros_req` per the permissive failure mode. The coverage baseline locks the regression line; future grammar additions are non-breaking.
