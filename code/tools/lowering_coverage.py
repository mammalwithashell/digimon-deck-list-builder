#!/usr/bin/env python3
"""Measure which DSL constructs the engine's MECHANICS tests exercise.

Plan 2 Task 8a (the mechanics training gate, measurement only). The owner's
direction (2026-10-21): "test mechanics in the DSL rather than the cards that
use them; index the YAML and make sure the lowerings it references are tested".
This tool builds that index; it does not change the gate.

Inputs
------
* `qa/impact/dsl_impact_index.json` -- per card, the step verbs / predicate keys
  / timing strings its YAML uses (built by `tools/impact_index.py`).
* Card YAML text (regex only) for the KEYWORD names a card grants
  (`keyword: X`, `keywords: [X, ...]`) -- the impact index does not carry them.
* The canonical construct universe: step keys (`step.rs` `kv!`), predicate keys
  (`predicate.rs` `PredicateSpec`) -- both via `impact_index` loaders -- timing
  strings (`clause.rs` `enum Timing`, serde snake_case + renames) and keyword
  names (`enums.rs` `enum Keyword` plus every name a card YAML uses).
* The MECHANICS test sources, read as text (no cargo): see `SCAN_GLOBS`. Per-card
  tests under `tests/cards_behavioral/` are never scanned and never count.

Attribution rule (conservative, token presence)
-----------------------------------------------
A `#[test]` fn exercises a construct when its body -- or a same-file helper fn /
`const` / `static` it references, transitively by name -- contains:

* `variant`   -- the compiled-IR identifier: `CompiledStep::<Variant>` (verb),
  `CompiledTiming::<Variant>` (timing), a non-empty field of a
  `CompiledPredicate { .. }` literal or a `<x>.<field> = Some(..)` /
  `= vec![..]` assignment in a file that names `CompiledPredicate` (predicate);
* `spec`      -- the parse-level identifier: `StepSpec::<Variant>`,
  a non-empty field of a `PredicateSpec { .. }` literal;
* `yaml_key`  -- a string literal that parses as YAML and, scanned with the same
  structural walker as the card index (`impact_index.scan_general`), uses the
  verb / predicate / `when:` timing / `keyword:` name;
* `literal`   -- a string literal exactly equal to the timing string or keyword;
* `keyword`   -- the engine identifier `Keyword::<Name>`;
* `fixture`   -- a string literal exactly equal to the `card:` id of a fixture
  YAML that lives under a scanned root (e.g. `rules_conformance/fixtures/**`,
  loaded by id: `.hand(0, &["MAIN_DEL_ALL"])`); the fixture's constructs are
  extracted with the same structural walker as the card index.

Comments are stripped before matching; YAML keys only count inside string
literals. Cross-file helpers (`support.rs`) and `include_str!` of real card YAML are NOT
followed.

A construct is `tested` when at least one attributed test exists (all scanned
roots are outside `cards_behavioral`). It is `tested_behaviourally` when one of
those tests lives in a BEHAVIOURAL file (not `parse_*.rs`, and the file drives
the engine -- see `BEHAVIOUR_MARKERS`); otherwise it is `parsed_only`.

Usage
-----
    PYTHONPATH=code python -m tools.lowering_coverage            # write artifact
    PYTHONPATH=code python -m tools.lowering_coverage --check    # fail if stale
    PYTHONPATH=code python -m tools.lowering_coverage --report OUT.md \
        [--decks deckpool.json]                                  # measurement
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import textwrap
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

CODE_DIR = Path(__file__).resolve().parents[1]
if str(CODE_DIR) not in sys.path:
    sys.path.insert(0, str(CODE_DIR))

import yaml  # noqa: E402  (also imported by tools.impact_index)

from tools.impact_index import (  # noqa: E402
    PREDICATE_ALIASES,
    REPO,
    STEP_ALIASES,
    CardUsage,
    load_predicate_keys,
    load_step_keys,
    scan_general,
    scan_predicate,
)

SCHEMA_VERSION = 1
ENGINE = REPO / "code" / "digimon-engine"
DSL_SRC = REPO / "code" / "digimon-dsl" / "src"
CARDS_DIR = ENGINE / "cards"
IMPACT_INDEX = REPO / "qa" / "impact" / "dsl_impact_index.json"
LOWERING_MAP = REPO / "qa" / "impact" / "lowering_verb_map.json"
DECK_LIBRARY = REPO / "data" / "deck_library.json"
DEFAULT_OUT = REPO / "qa" / "impact" / "lowering_coverage.json"

# Mechanics test roots, relative to the engine crate. Directories are scanned
# recursively; a top-level `tests/<x>.rs` is its own binary.
SCAN_GLOBS = (
    "tests/dsl",
    "tests/rules_conformance",
    "tests/keyword_phase_*",
    "tests/keyword_semantics_matrix*",
    "tests/timing_dispatch*",
    "tests/cost_hooks",
    "tests/replacements*",
    "tests/effect_context*",
    "src/dsl_cards",
)
EXCLUDED_PART = "cards_behavioral"

# A file "drives the engine" when it names one of these. A file without any is
# parse/serde/validator-only, as is every `parse_*.rs`.
BEHAVIOUR_MARKERS = (
    "DebugRunner",
    "run_step",
    "run_steps",
    "Game::",
    "Game {",
    "DslCardEffect",
    "EffectContext",
    "HeadlessRunner",
    "dsl_cards::",
    "effects_for_card",
)

# Compiled-only step variants that lower FROM a canonical key under another name.
EXTRA_STEP_VARIANTS = {
    "GainMemoryFn": "gain_memory",
    "LoseMemoryFn": "lose_memory",
    "DrawFn": "draw",
}

KINDS = ("verb", "predicate", "timing", "keyword")
EMPTY_VALUES = {"None", "Default::default()", "vec![]", "Vec::new()", "false", "Vec::default()"}


# --------------------------------------------------------------------------
# Universe loaders
# --------------------------------------------------------------------------


def _enum_body(text: str, header: str) -> str:
    start = text.index(header)
    brace = text.index("{", start)
    depth = 0
    for i in range(brace, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[brace + 1 : i]
    raise ValueError(f"unterminated enum {header}")


def _top_level_variants(body: str) -> list[tuple[str, str]]:
    """(variant, preceding attribute text) for each depth-0 enum variant."""
    out: list[tuple[str, str]] = []
    depth = 0
    attrs: list[str] = []
    for line in _strip_line_comments(body).split("\n"):
        stripped = line.strip()
        if depth == 0:
            if stripped.startswith("#["):
                attrs.append(stripped)
            else:
                m = re.match(r"([A-Z][A-Za-z0-9]*)\b", stripped)
                if m:
                    out.append((m.group(1), " ".join(attrs)))
                    attrs = []
        depth += sum(line.count(c) for c in "{([") - sum(line.count(c) for c in "})]")
    return out


def _strip_line_comments(text: str) -> str:
    return re.sub(r"//[^\n]*", "", text)


def _snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def load_step_variant_map(dsl_src: Path = DSL_SRC) -> dict[str, str]:
    """CompiledStep / StepSpec variant name -> canonical step key."""
    step_rs = (dsl_src / "step.rs").read_text(encoding="utf-8")
    spec = dict(
        re.findall(
            r'StepSpec::([A-Za-z0-9]+)\([^)]*\)\s*=>\s*(?:\{\s*)?kv!\(\s*s\s*,\s*"([a-z0-9_]+)"',
            step_rs,
        )
    )
    compiled_rs = (dsl_src / "compiled.rs").read_text(encoding="utf-8")
    compiled = {v for v, _ in _top_level_variants(_enum_body(compiled_rs, "pub enum CompiledStep {"))}
    out = {v: k for v, k in spec.items() if v in compiled}
    for v, k in EXTRA_STEP_VARIANTS.items():
        if v in compiled:
            out[v] = k
    return out


def load_step_spec_map(dsl_src: Path = DSL_SRC) -> dict[str, str]:
    step_rs = (dsl_src / "step.rs").read_text(encoding="utf-8")
    return dict(
        re.findall(
            r'StepSpec::([A-Za-z0-9]+)\([^)]*\)\s*=>\s*(?:\{\s*)?kv!\(\s*s\s*,\s*"([a-z0-9_]+)"',
            step_rs,
        )
    )


def load_timing_map(dsl_src: Path = DSL_SRC) -> dict[str, str]:
    """`Timing` / `CompiledTiming` variant name -> timing string."""
    text = (dsl_src / "clause.rs").read_text(encoding="utf-8")
    out: dict[str, str] = {}
    for variant, attrs in _top_level_variants(_enum_body(text, "pub enum Timing {")):
        m = re.search(r'rename\s*=\s*"([a-z0-9_]+)"', attrs)
        out[variant] = m.group(1) if m else _snake(variant)
    return out


def load_predicate_fields(dsl_src: Path = DSL_SRC, predicate_keys: set[str] | None = None) -> dict[str, str]:
    """`CompiledPredicate` field -> predicate key (identity, or `X_eq` -> `X`)."""
    predicate_keys = predicate_keys or load_predicate_keys()
    text = (dsl_src / "compiled.rs").read_text(encoding="utf-8")
    body = _enum_body(text, "pub struct CompiledPredicate {")
    fields = set(re.findall(r"^\s+pub ([a-z_0-9]+)\s*:", body, re.M))
    out: dict[str, str] = {}
    for f in fields:
        if f in predicate_keys:
            out[f] = f
        elif f.endswith("_eq") and f[:-3] in predicate_keys:
            out[f] = f[:-3]
    return out


def load_engine_keywords(engine: Path = ENGINE) -> set[str]:
    text = (engine / "src" / "enums.rs").read_text(encoding="utf-8")
    return {v for v, _ in _top_level_variants(_enum_body(text, "pub enum Keyword {"))}


_KEYWORD_RE = re.compile(r"(?:^|[\s{,])keyword:\s*['\"]?([A-Za-z][A-Za-z0-9]*)")
_KEYWORDS_RE = re.compile(r"(?:^|[\s{,])keywords:\s*\[([^\]]*)\]")


def card_keywords(yaml_text: str) -> set[str]:
    """Keyword names a card YAML grants. Comment lines are ignored."""
    lines = [re.sub(r"\s+#.*$", "", ln) for ln in yaml_text.split("\n") if not ln.lstrip().startswith("#")]
    text = "\n".join(lines)
    out = set(_KEYWORD_RE.findall(text))
    for group in _KEYWORDS_RE.findall(text):
        out.update(t.strip().strip("'\"") for t in group.split(",") if t.strip())
    return {k for k in out if re.fullmatch(r"[A-Za-z][A-Za-z0-9]*", k)}


# --------------------------------------------------------------------------
# Rust lexing
# --------------------------------------------------------------------------


@dataclass
class Lexed:
    masked: str  # comments -> spaces, string contents -> spaces (offsets preserved)
    strings: list[tuple[int, int, str]]  # (start, end, decoded content)


def _unescape(s: str) -> str:
    out: list[str] = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            n = s[i + 1]
            if n == "\n":
                i += 2
                while i < len(s) and s[i] in " \t\r\n":
                    i += 1
                continue
            out.append({"n": "\n", "t": "\t", "r": "\r", "0": "\0"}.get(n, n))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


_RAW_RE = re.compile(r"b?r(#*)\"")


def lex_rust(text: str) -> Lexed:
    text = text.replace("\r\n", "\n")
    masked = list(text)
    strings: list[tuple[int, int, str]] = []
    i, n = 0, len(text)

    def blank(a: int, b: int) -> None:
        for k in range(a, b):
            if masked[k] != "\n":
                masked[k] = " "

    while i < n:
        c = text[i]
        if c == "/" and text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
            continue
        if c == "/" and text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
            continue
        prev_ident = i > 0 and (text[i - 1].isalnum() or text[i - 1] == "_")
        m = None if prev_ident or c not in "rb" else _RAW_RE.match(text, i)
        if m:
            hashes = m.group(1)
            start = m.end()
            close = '"' + hashes
            j = text.find(close, start)
            j = n if j < 0 else j
            strings.append((start, j, text[start:j]))
            blank(start, j)
            i = j + len(close)
            continue
        if c == '"' or (c == "b" and not prev_ident and text.startswith('b"', i)):
            start = i + (2 if c == "b" else 1)
            j = start
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            strings.append((start, j, _unescape(text[start:j])))
            blank(start, j)
            i = j + 1
            continue
        if c == "'":
            cm = re.match(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'", text[i : i + 16])
            if cm:
                blank(i + 1, i + cm.end() - 1)
                i += cm.end()
                continue
        i += 1
    return Lexed("".join(masked), strings)


def _match_brace(masked: str, open_idx: int) -> int:
    depth = 0
    for k in range(open_idx, len(masked)):
        ch = masked[k]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return k + 1
    return len(masked)


@dataclass
class Item:
    name: str
    start: int
    end: int
    is_test: bool


_FN_RE = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
_CONST_RE = re.compile(r"\b(?:const|static)\s+([A-Z_][A-Z0-9_]*)\s*:")


def extract_items(masked: str) -> list[Item]:
    items: list[Item] = []
    for m in _FN_RE.finditer(masked):
        # Body: first `{` after the signature at paren/bracket depth 0, unless a `;` comes first.
        depth, k = 0, m.end()
        body_open = None
        while k < len(masked):
            ch = masked[k]
            if ch in "([<":
                depth += 1
            elif ch in ")]>":
                depth = max(0, depth - 1)
            elif ch == ";" and depth == 0:
                break
            elif ch == "{" and depth == 0:
                body_open = k
                break
            k += 1
        if body_open is None:
            continue
        end = _match_brace(masked, body_open)
        head_start = max(masked.rfind("}", 0, m.start()), masked.rfind(";", 0, m.start()), masked.rfind("{", 0, m.start()))
        is_test = re.search(r"#\[\s*test\s*\]", masked[head_start + 1 : m.start()]) is not None
        items.append(Item(m.group(1), m.start(), end, is_test))
    for m in _CONST_RE.finditer(masked):
        depth, k = 0, m.end()
        while k < len(masked):
            ch = masked[k]
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            elif ch == ";" and depth == 0:
                break
            k += 1
        items.append(Item(m.group(1), m.start(), k + 1, False))
    return items


_IDENT_RE = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*\b")


def test_closures(masked: str, items: list[Item]) -> dict[int, list[Item]]:
    """For each test item index, the same-file items it reaches by name."""
    by_name: dict[str, list[Item]] = defaultdict(list)
    for it in items:
        by_name[it.name].append(it)
    out: dict[int, list[Item]] = {}
    for idx, it in enumerate(items):
        if not it.is_test:
            continue
        seen = {id(it)}
        reach = [it]
        queue = [it]
        while queue:
            cur = queue.pop()
            names = set(_IDENT_RE.findall(masked[cur.start : cur.end]))
            for name in names:
                for cand in by_name.get(name, ()):
                    if id(cand) in seen or cand.is_test:
                        continue
                    # skip items nested inside an already-reached span (they are covered)
                    seen.add(id(cand))
                    reach.append(cand)
                    queue.append(cand)
        out[idx] = reach
    return out


# --------------------------------------------------------------------------
# Signal extraction
# --------------------------------------------------------------------------


@dataclass
class Vocab:
    step_keys: set[str]
    predicate_keys: set[str]
    step_variants: dict[str, str]
    step_spec: dict[str, str]
    timing_variants: dict[str, str]
    timings: set[str]
    predicate_fields: dict[str, str]
    keywords: set[str]
    fixtures: dict[str, dict[str, set[str]]] = field(default_factory=dict)

    @classmethod
    def load(cls, card_keywords_seen: Iterable[str] = ()) -> "Vocab":
        step_keys = load_step_keys()
        predicate_keys = load_predicate_keys()
        timing_variants = load_timing_map()
        return cls(
            step_keys=step_keys,
            predicate_keys=predicate_keys,
            step_variants=load_step_variant_map(),
            step_spec={v: k for v, k in load_step_spec_map().items() if k in step_keys},
            timing_variants=timing_variants,
            timings=set(timing_variants.values()),
            predicate_fields=load_predicate_fields(predicate_keys=predicate_keys),
            keywords=load_engine_keywords() | set(card_keywords_seen),
        )


def _split_top_level(s: str) -> list[str]:
    parts, depth, cur = [], 0, []
    for ch in s:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(ch)
    parts.append("".join(cur))
    return parts


def _struct_literal_fields(masked: str, type_name: str) -> list[str]:
    out: list[str] = []
    for m in re.finditer(r"\b" + type_name + r"\s*\{", masked):
        open_idx = m.end() - 1
        end = _match_brace(masked, open_idx)
        for part in _split_top_level(masked[open_idx + 1 : end - 1]):
            fm = re.match(r"\s*([a-z_][a-z0-9_]*)\s*:(?!:)\s*(.*)$", part, re.S)
            if not fm:
                continue
            value = re.sub(r"\s+", "", fm.group(2))
            if value in {re.sub(r"\s+", "", e) for e in EMPTY_VALUES} or value == "":
                continue
            out.append(fm.group(1))
    return out


_YAML_CACHE: dict[str, Any] = {}
_YAML_FAIL = object()


def _parse_yaml(content: str) -> Any:
    if content in _YAML_CACHE:
        return _YAML_CACHE[content]
    data: Any = _YAML_FAIL
    if ":" in content and "\n" in content or re.match(r"\s*[a-z_]+\s*:", content):
        try:
            data = yaml.safe_load(textwrap.dedent(content.strip("\n")))
        except Exception:  # noqa: BLE001 -- any non-YAML literal is simply skipped
            data = _YAML_FAIL
    if not isinstance(data, (dict, list)):
        data = _YAML_FAIL
    _YAML_CACHE[content] = data
    return data


def _yaml_keywords(node: Any, out: set[str]) -> None:
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "keyword" and isinstance(v, str):
                out.add(v)
            elif k == "keywords" and isinstance(v, list):
                out.update(x for x in v if isinstance(x, str))
            _yaml_keywords(v, out)
    elif isinstance(node, list):
        for x in node:
            _yaml_keywords(x, out)


def yaml_constructs(data: Any, vocab: Vocab) -> dict[str, set[str]]:
    usage = CardUsage(card_id="", path="")
    scan_general(data, usage, vocab.step_keys, vocab.predicate_keys)
    if isinstance(data, list):
        scan_general(data, usage, vocab.step_keys, vocab.predicate_keys, step_context=True)
    elif isinstance(data, dict) and data:
        keys = [str(k) for k in data]
        if len(keys) == 1 and STEP_ALIASES.get(keys[0], keys[0]) in vocab.step_keys:
            scan_general(data, usage, vocab.step_keys, vocab.predicate_keys, step_context=True)
        elif all(PREDICATE_ALIASES.get(k, k) in vocab.predicate_keys for k in keys):
            scan_predicate(data, usage, vocab.predicate_keys)
    kws: set[str] = set()
    _yaml_keywords(data, kws)
    return {
        "verb": set(usage.verbs),
        "predicate": set(usage.predicates),
        "timing": {t for t in usage.timings if t in vocab.timings},
        "keyword": {k for k in kws if k in vocab.keywords},
    }


def signals_in(masked: str, strings: list[str], vocab: Vocab, *, names_compiled_predicate: bool) -> dict[tuple[str, str], set[str]]:
    """(kind, construct) -> signals found in this code + these string literals."""
    found: dict[tuple[str, str], set[str]] = defaultdict(set)
    for v in re.findall(r"\bCompiledStep::([A-Z][A-Za-z0-9]*)", masked):
        if v in vocab.step_variants:
            found[("verb", vocab.step_variants[v])].add("variant")
    for v in re.findall(r"\bStepSpec::([A-Z][A-Za-z0-9]*)", masked):
        if v in vocab.step_spec:
            found[("verb", vocab.step_spec[v])].add("spec")
    for v in re.findall(r"\bCompiledTiming::([A-Z][A-Za-z0-9]*)", masked):
        if v in vocab.timing_variants:
            found[("timing", vocab.timing_variants[v])].add("variant")
    for f in _struct_literal_fields(masked, "CompiledPredicate"):
        if f in vocab.predicate_fields:
            found[("predicate", vocab.predicate_fields[f])].add("variant")
    if names_compiled_predicate:
        for f in re.findall(r"\.([a-z_][a-z0-9_]*)\s*=\s*(?:Some\(|vec!\[)", masked):
            if f in vocab.predicate_fields:
                found[("predicate", vocab.predicate_fields[f])].add("variant")
    for f in _struct_literal_fields(masked, "PredicateSpec"):
        key = PREDICATE_ALIASES.get(f, f)
        if key in vocab.predicate_keys:
            found[("predicate", key)].add("spec")
    for v in re.findall(r"\bKeyword::([A-Z][A-Za-z0-9]*)", masked):
        if v in vocab.keywords:
            found[("keyword", v)].add("keyword")
    for content in strings:
        for kind, names in vocab.fixtures.get(content, {}).items():
            for name in names:
                found[(kind, name)].add("fixture")
        if content in vocab.timings:
            found[("timing", content)].add("literal")
        if content in vocab.keywords:
            found[("keyword", content)].add("literal")
        data = _parse_yaml(content)
        if data is _YAML_FAIL:
            continue
        for kind, names in yaml_constructs(data, vocab).items():
            for name in names:
                found[(kind, name)].add("yaml_key")
    return found


# --------------------------------------------------------------------------
# Test scanning
# --------------------------------------------------------------------------


@dataclass
class TestHit:
    test_id: str
    behavioural: bool
    signals: set[str] = field(default_factory=set)


def iter_test_files(engine: Path = ENGINE, suffix: str = ".rs") -> list[Path]:
    files: set[Path] = set()
    for pattern in SCAN_GLOBS:
        for root in engine.glob(pattern):
            if root.is_dir():
                files.update(root.rglob("*" + suffix))
            elif root.suffix == suffix:
                files.add(root)
    return sorted(p for p in files if EXCLUDED_PART not in p.relative_to(engine).parts)


def load_fixture_cards(engine: Path, vocab: "Vocab") -> dict[str, dict[str, set[str]]]:
    """Fixture card YAML under the scanned roots (e.g. `rules_conformance/fixtures/**`),
    keyed by its `card:` id. A test that names the id in a string literal (`.hand(0,
    &["MAIN_DEL_ALL"])`) is credited with the fixture's constructs (signal `fixture`)."""
    out: dict[str, dict[str, set[str]]] = {}
    for path in iter_test_files(engine, ".yaml"):
        try:
            data = yaml.safe_load(path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001 -- intentionally-invalid fixtures (bad.yaml)
            continue
        if not isinstance(data, dict) or not isinstance(data.get("card"), str):
            continue
        cons = yaml_constructs(data, vocab)
        prev = out.setdefault(data["card"], {k: set() for k in KINDS})
        for kind in KINDS:
            prev[kind] |= cons[kind]
    return out


def test_prefix(path: Path, engine: Path) -> str:
    rel = path.relative_to(engine)
    if rel.parts[0] == "src":
        return "lib::" + rel.as_posix()
    if len(rel.parts) == 2:  # tests/<binary>.rs
        return f"{rel.stem}::{rel.name}"
    return f"{rel.parts[1]}::" + Path(*rel.parts[2:]).as_posix()


def is_behavioural_file(path: Path, text: str) -> bool:
    if path.name.startswith("parse_"):
        return False
    return any(marker in text for marker in BEHAVIOUR_MARKERS)


def scan_file(path: Path, engine: Path, vocab: Vocab) -> tuple[bool, dict[tuple[str, str], list[TestHit]]]:
    if EXCLUDED_PART in path.as_posix():
        return False, {}
    text = path.read_text(encoding="utf-8", errors="replace")
    lexed = lex_rust(text)
    scan_file.last_tokens = set(_IDENT_RE.findall(lexed.masked)) | {  # type: ignore[attr-defined]
        w for (_a, _b, st) in lexed.strings for w in _IDENT_RE.findall(st)
    }
    behavioural = is_behavioural_file(path, lexed.masked)
    items = extract_items(lexed.masked)
    closures = test_closures(lexed.masked, items)
    names_cp = "CompiledPredicate" in lexed.masked
    prefix = test_prefix(path, engine)
    out: dict[tuple[str, str], list[TestHit]] = defaultdict(list)
    used_ids: Counter[str] = Counter()
    for idx, reach in sorted(closures.items(), key=lambda kv: items[kv[0]].start):
        test = items[idx]
        used_ids[test.name] += 1
        suffix = "" if used_ids[test.name] == 1 else f"#{used_ids[test.name]}"
        test_id = f"{prefix}::{test.name}{suffix}"
        spans = sorted((it.start, it.end) for it in reach)
        masked = "\n".join(lexed.masked[a:b] for a, b in spans)
        strings = [s for (a, b, s) in lexed.strings if any(sa <= a and b <= sb for sa, sb in spans)]
        for key, sigs in signals_in(masked, strings, vocab, names_compiled_predicate=names_cp).items():
            out[key].append(TestHit(test_id, behavioural, set(sigs)))
    return behavioural, out


# --------------------------------------------------------------------------
# Build
# --------------------------------------------------------------------------


# --------------------------------------------------------------------------
# Per-card attention signals (owner refinement 2026-10-21): clause count,
# replacement/protection constructs, own behavioural test.
# --------------------------------------------------------------------------

# Clause lists counted for `clause_count` (top-level YAML keys / paths).
CLAUSE_LIST_PATHS = (("effects",), ("dual", "digimon", "effects"), ("dual", "option", "effects"), ("alt_paths",))
EFFECT_CLAUSE_PATHS = CLAUSE_LIST_PATHS[:3]

PROTECTIVE_KEYWORDS = (
    "ArmorPurge", "Evade", "Barrier", "Fragment", "Scapegoat", "Decoy", "Guard",
    "Progress", "Jamming", "Partition", "Decode", "Detach",
)
# Matched against the canonical step keys (step.rs) and predicate keys (predicate.rs).
PROTECTION_NAME_RE = re.compile(r"immun|protect|unaffected|prevent|substitut|replacement|cannot")
# Matched against timing strings (clause.rs `enum Timing`) and replacement `trigger:` values.
PROTECTION_TIMING_RE = re.compile(r"would")
# Matched against `modifier:` values anywhere in the card (aura / add_modifier / flood_gate).
PROTECTION_MODIFIER_RE = re.compile(r"^(CannotBe|CanNotBe|Immune)")
# flood_gate clauses count only when their modifier forbids an action.
FORBID_MODIFIER_RE = re.compile(r"^(Cannot|CanNot)")
PROTECTION_CLAUSE_KINDS = ("replacement", "partition")


def _get_path(data: Any, path: tuple[str, ...]) -> Any:
    for key in path:
        if not isinstance(data, dict):
            return None
        data = data.get(key)
    return data


def _clauses(data: dict, paths: tuple[tuple[str, ...], ...]) -> list[dict]:
    out: list[dict] = []
    for path in paths:
        val = _get_path(data, path)
        if isinstance(val, list):
            out.extend(c for c in val if isinstance(c, dict))
    return out


def _modifiers(node: Any, out: set[str]) -> None:
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "modifier" and isinstance(v, str):
                out.add(v)
            _modifiers(v, out)
    elif isinstance(node, list):
        for x in node:
            _modifiers(x, out)


def protection_rule(vocab: "Vocab") -> dict[str, list[str]]:
    """The exact key lists the protection family is drawn from (reproducible)."""
    return {
        "verbs": sorted(k for k in vocab.step_keys if PROTECTION_NAME_RE.search(k)),
        "predicates": sorted(k for k in vocab.predicate_keys if PROTECTION_NAME_RE.search(k)),
        "timings": sorted(t for t in vocab.timings if PROTECTION_TIMING_RE.search(t)),
        "keywords": list(PROTECTIVE_KEYWORDS),
        "clause_kinds": list(PROTECTION_CLAUSE_KINDS) + ["flood_gate (modifier ~ ^(Cannot|CanNot))"],
        "replacement_triggers": ["any trigger: value of a replacement clause matching /would/"],
        "modifiers": ["any modifier: value matching /^(CannotBe|CanNotBe|Immune)/"],
    }


def card_attention(data: dict, constructs: dict[str, list[str]], rule: dict[str, list[str]]) -> dict[str, Any]:
    clauses = _clauses(data, CLAUSE_LIST_PATHS)
    effect_clauses = _clauses(data, EFFECT_CLAUSE_PATHS)
    prot: set[str] = set()
    for kind, key in (("verb", "verbs"), ("predicate", "predicates"), ("timing", "timings"), ("keyword", "keywords")):
        allowed = set(rule[key])
        prot.update(f"{kind}:{n}" for n in constructs[kind] if n in allowed)
    for c in effect_clauses:
        kind = c.get("kind")
        if kind in PROTECTION_CLAUSE_KINDS:
            prot.add(f"clause:{kind}")
        if kind == "flood_gate":
            mods: set[str] = set()
            _modifiers(c, mods)
            if any(FORBID_MODIFIER_RE.search(m) for m in mods):
                prot.add("clause:flood_gate")
        if kind == "replacement" and isinstance(c.get("trigger"), str) and PROTECTION_TIMING_RE.search(c["trigger"]):
            prot.add(f"trigger:{c['trigger']}")
    mods = set()
    _modifiers(data, mods)
    prot.update(f"modifier:{m}" for m in mods if PROTECTION_MODIFIER_RE.search(m))
    return {
        "clause_count": len(clauses),
        "effect_clause_count": len(effect_clauses),
        "protection_constructs": sorted(prot),
    }


def load_has_card_test(card_ids: Iterable[str]) -> set[str]:
    """Same scan `card_admissibility.py` does (module under cards_behavioral or a ledger test_path)."""
    from digimon_gym.agents.card_admissibility import (  # stdlib-only module
        DataPaths,
        behavioral_test_modules,
        discover_tested_ids,
    )

    paths = DataPaths()
    ledgers = [json.loads(p.read_text(encoding="utf-8")) for p in (paths.dsl_ledger, paths.rust_ledger) if p.exists()]
    return discover_tested_ids(
        card_ids, modules=behavioral_test_modules(paths.behavioral_tests), ledgers=ledgers, repo_root=paths.repo_root
    )


def load_card_constructs(impact: dict, cards_dir: Path = CARDS_DIR) -> dict[str, dict[str, list[str]]]:
    out: dict[str, dict[str, list[str]]] = {}
    for cid, row in impact["cards"].items():
        path = cards_dir / row["path"]
        kws = card_keywords(path.read_text(encoding="utf-8")) if path.exists() else set()
        out[cid] = {
            "verb": sorted(row.get("verbs", [])),
            "predicate": sorted(row.get("predicates", [])),
            "timing": sorted(row.get("timings", [])),
            "keyword": sorted(kws),
        }
    return out


def ex13_lists(deck_library: dict) -> list[tuple[str, list[str]]]:
    out: list[tuple[str, list[str]]] = []
    for arch_name, arch in sorted(deck_library.get("archetypes", {}).items()):
        for dl in arch.get("decklists", []):
            if dl.get("format") != "EX13":
                continue
            raw = dl.get("decklist")
            cards = json.loads(raw) if isinstance(raw, str) else list(raw or [])
            out.append((arch_name, [str(c) for c in cards]))
    return out


def build(
    *,
    engine: Path = ENGINE,
    impact: dict | None = None,
    cards_dir: Path = CARDS_DIR,
    deck_library: dict | None = None,
    lowering_map: dict | None = None,
    vocab: Vocab | None = None,
    has_card_test: set[str] | None = None,
) -> dict[str, Any]:
    impact = impact if impact is not None else json.loads(IMPACT_INDEX.read_text(encoding="utf-8"))
    deck_library = deck_library if deck_library is not None else json.loads(DECK_LIBRARY.read_text(encoding="utf-8"))
    if lowering_map is None:
        lowering_map = json.loads(LOWERING_MAP.read_text(encoding="utf-8")).get("files", {})
    card_cons = load_card_constructs(impact, cards_dir)
    vocab = vocab or Vocab.load(k for c in card_cons.values() for k in c["keyword"])
    vocab.fixtures = load_fixture_cards(engine, vocab)
    rule = protection_rule(vocab)
    attention: dict[str, dict[str, Any]] = {}
    for cid, row in impact["cards"].items():
        path = cards_dir / row["path"]
        data = yaml.safe_load(path.read_text(encoding="utf-8")) if path.exists() else {}
        attention[cid] = card_attention(data if isinstance(data, dict) else {}, card_cons[cid], rule)
    tested_ids = has_card_test if has_card_test is not None else load_has_card_test(card_cons)

    hits: dict[tuple[str, str], list[TestHit]] = defaultdict(list)
    files = iter_test_files(engine)
    parse_only_files: list[str] = []
    n_tests_files = 0
    file_tokens: dict[str, set[str]] = {}
    for path in files:
        behavioural, file_hits = scan_file(path, engine, vocab)
        file_tokens[path.relative_to(engine).as_posix()] = getattr(scan_file, "last_tokens", set())
        if not behavioural:
            parse_only_files.append(path.relative_to(engine).as_posix())
        n_tests_files += 1
        for key, hs in file_hits.items():
            hits[key].extend(hs)

    verb_files: dict[str, list[str]] = defaultdict(list)
    for fname, verbs in lowering_map.items():
        for v in verbs:
            verb_files[v].append(fname)

    universe: dict[str, set[str]] = {
        "verb": set(vocab.step_keys),
        "predicate": set(vocab.predicate_keys),
        "timing": set(vocab.timings) | {t for c in card_cons.values() for t in c["timing"]},
        "keyword": set(vocab.keywords),
    }
    for c in card_cons.values():
        for kind in KINDS:
            universe[kind].update(c[kind])

    constructs: dict[str, dict[str, Any]] = {}
    for kind in KINDS:
        for name in sorted(universe[kind]):
            hs = sorted(hits.get((kind, name), []), key=lambda h: h.test_id)
            entry: dict[str, Any] = {
                "kind": kind,
                "name": name,
                "tests": [h.test_id for h in hs],
                "signals": {h.test_id: sorted(h.signals) for h in hs},
                "tested": bool(hs),
                "tested_behaviourally": any(h.behavioural for h in hs),
            }
            entry["parsed_only"] = entry["tested"] and not entry["tested_behaviourally"]
            if kind == "verb":
                entry["lowering_files"] = sorted(verb_files.get(name, []))
            if not hs:
                # Near miss: the bare name (or its CamelCase variant) appears as a token
                # in a scanned file without meeting the attribution rule -- typically an
                # engine-level test of the primitive (`ctx.add_top_security_to_hand`,
                # `EffectTiming::OnLoseSecurity`) rather than of the DSL lowering.
                toks = {name, "".join(p.capitalize() for p in name.split("_"))}
                entry["near_miss_files"] = sorted(f for f, t in file_tokens.items() if toks & t)
            constructs[f"{kind}:{name}"] = entry

    cards: dict[str, dict[str, Any]] = {}
    for cid in sorted(card_cons):
        cons = [f"{k}:{n}" for k in KINDS for n in card_cons[cid][k]]
        untested = [c for c in cons if not constructs[c]["tested"]]
        parsed_only = [c for c in cons if constructs[c]["parsed_only"]]
        cards[cid] = {
            "constructs": cons,
            "untested_constructs": untested,
            "parsed_only_constructs": parsed_only,
            "lowering_covered": not untested,
            "behaviourally_covered": not untested and not parsed_only,
            **attention[cid],
            "has_card_test": cid in tested_ids,
        }

    lists = ex13_lists(deck_library)
    blocks_cards: dict[str, set[str]] = defaultdict(set)
    blocks_lists: dict[str, int] = Counter()
    blocks_lists_b: dict[str, int] = Counter()
    for cid, row in cards.items():
        for c in row["untested_constructs"]:
            blocks_cards[c].add(cid)
    for _arch, deck in lists:
        ids = set(deck)
        untested = {c for cid in ids if cid in cards for c in cards[cid]["untested_constructs"]}
        weak = untested | {c for cid in ids if cid in cards for c in cards[cid]["parsed_only_constructs"]}
        for c in untested:
            blocks_lists[c] += 1
        for c in weak:
            blocks_lists_b[c] += 1

    per_kind = {}
    for kind in KINDS:
        rows = [v for v in constructs.values() if v["kind"] == kind]
        used = [v for v in rows if f"{v['kind']}:{v['name']}" in blocks_cards or _used_by_cards(v, cards)]
        per_kind[kind] = {
            "total": len(rows),
            "tested": sum(v["tested"] for v in rows),
            "tested_behaviourally": sum(v["tested_behaviourally"] for v in rows),
            "parsed_only": sum(v["parsed_only"] for v in rows),
            "untested": sum(not v["tested"] for v in rows),
            "used_by_cards": len(used),
            "used_untested": sum(not v["tested"] for v in used),
        }
    top = sorted(
        blocks_cards,
        key=lambda c: (-blocks_lists[c], -len(blocks_cards[c]), c),
    )
    summary = {
        "constructs": per_kind,
        "cards": {
            "total": len(cards),
            "lowering_covered": sum(r["lowering_covered"] for r in cards.values()),
            "behaviourally_covered": sum(r["behaviourally_covered"] for r in cards.values()),
            "not_covered": sum(not r["lowering_covered"] for r in cards.values()),
        },
        "ex13_lists": len(lists),
        "top_untested": [
            {"construct": c, "cards_blocked": len(blocks_cards[c]), "ex13_lists_blocked": blocks_lists[c]}
            for c in top
        ],
        "scanned_files": n_tests_files,
        "parse_only_files": sorted(parse_only_files),
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "attribution": (
            "token presence in a #[test] fn body or a same-file helper/const it references; "
            "signals: variant | spec | yaml_key | literal | keyword | fixture; cards_behavioral never scanned"
        ),
        "scan_globs": list(SCAN_GLOBS),
        "clause_count_keys": ["/".join(p) for p in CLAUSE_LIST_PATHS],
        "protection_rule": rule,
        "constructs": constructs,
        "cards": cards,
        "summary": summary,
    }


def _used_by_cards(entry: dict, cards: dict) -> bool:
    key = f"{entry['kind']}:{entry['name']}"
    return any(key in r["constructs"] for r in cards.values())


def serialise(payload: dict) -> str:
    return json.dumps(payload, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


# --------------------------------------------------------------------------
# Measurement report
# --------------------------------------------------------------------------

FOCUS_LEADERS = (
    "TS Jupitermon",
    "Glowing Dawn",
    "Examon",
    "Data Squad",
    "Toho Braves",
    "Three Musketeers",
    "TS Ceresmon",
    "Chronomon",
    "Titans",
    "TS Mervamon",
)
TEXT_FIELDS = ("effect_description_eng", "inherited_effect_description_eng", "security_effect_description_eng")


def _card_status(cid: str, cards: dict, cards_json: dict) -> str:
    if cid in cards:
        row = cards[cid]
        if not row["lowering_covered"]:
            return "dsl_untested"
        return "dsl_covered" if row["behaviourally_covered"] else "dsl_parsed_only"
    meta = cards_json.get(cid)
    if meta is None:
        return "unknown"
    if any((meta.get(f) or "").strip() for f in TEXT_FIELDS):
        return "unimplemented"
    return "vanilla"


def deck_verdict(deck: list[str], payload: dict, cards_json: dict) -> dict[str, Any]:
    cards = payload["cards"]
    statuses = {cid: _card_status(cid, cards, cards_json) for cid in sorted(set(deck))}
    untested = sorted({c for cid in statuses if cid in cards for c in cards[cid]["untested_constructs"]})
    parsed = sorted({c for cid in statuses if cid in cards for c in cards[cid]["parsed_only_constructs"]})
    counts = Counter(statuses.values())
    blocked_by_impl = counts["unimplemented"] + counts["unknown"]
    return {
        "statuses": statuses,
        "counts": dict(counts),
        "untested": untested,
        "parsed_only": parsed,
        "lowering_covered": not untested and not blocked_by_impl,
        "behaviourally_covered": not untested and not parsed and not blocked_by_impl,
        "dsl_covered_ignoring_unimplemented": not untested,
    }


THRESHOLDS = (
    ("clauses>=4 OR protection", lambda r: r["clause_count"] >= 4 or bool(r["protection_constructs"])),
    ("clauses>=5 OR protection", lambda r: r["clause_count"] >= 5 or bool(r["protection_constructs"])),
    ("protection only", lambda r: bool(r["protection_constructs"])),
    ("effect clauses>=4 OR protection", lambda r: r["effect_clause_count"] >= 4 or bool(r["protection_constructs"])),
    ("effect clauses>=5 OR protection", lambda r: r["effect_clause_count"] >= 5 or bool(r["protection_constructs"])),
)


def _hist(values: list[int]) -> str:
    c = Counter(min(v, 10) for v in values)
    return ", ".join(f"{k if k < 10 else '10+'}: {c[k]}" for k in sorted(c))


def render_attention(payload: dict, *, decks: list[dict] | None, deck_library: dict) -> list[str]:
    cards = payload["cards"]
    rule = payload["protection_rule"]
    L: list[str] = ["## 5. Cards that warrant their own behavioural test (owner refinement)\n"]
    L.append(
        "`clause_count` = entries under " + ", ".join(f"`{k}`" for k in payload["clause_count_keys"])
        + " (every scope: inherited / security / linked / face_up / trash clauses are entries of `effects`). "
        "`effect_clause_count` is the same without `alt_paths` (alternate digivolve / DNA / assembly paths; "
        "most Digimon carry at least one, so they inflate the count for ordinary Digimon).\n"
    )
    L.append("Protection family (derived; reproducible from `protection_rule` in the artifact):\n")
    for k, v in rule.items():
        L.append(f"- {k}: " + ", ".join(f"`{x}`" for x in v))
    L.append("")
    universes: list[tuple[str, set[str]]] = [("all DSL cards", set(cards))]
    if decks is not None:
        universes.append(("10 DigiLab decks (unique DSL cards)", {c for d in decks for c in d["cards"] if c in cards}))
    focus = {c for arch, deck in ex13_lists(deck_library) if arch in FOCUS_LEADERS for c in deck if c in cards}
    universes.append(("EX13 focus-pool lists (unique DSL cards)", focus))
    for label, ids in universes:
        rows = [cards[c] for c in sorted(ids)]
        L.append(f"### {label}: {len(rows)} cards\n")
        L.append(f"- clause_count histogram: {_hist([r['clause_count'] for r in rows])}")
        L.append(f"- effect_clause_count histogram: {_hist([r['effect_clause_count'] for r in rows])}")
        for n in (3, 4, 5):
            L.append(
                f"- clause_count >= {n}: {sum(r['clause_count'] >= n for r in rows)} "
                f"(effect clauses >= {n}: {sum(r['effect_clause_count'] >= n for r in rows)})"
            )
        L.append(f"- any protection construct: {sum(bool(r['protection_constructs']) for r in rows)}")
        L.append(f"- have their own card test: {sum(r['has_card_test'] for r in rows)}\n")
        L.append("| threshold | need own test | of those lacking one | of those also not lowering-covered |")
        L.append("|---|---|---|---|")
        for name, pred in THRESHOLDS:
            need = [r for r in rows if pred(r)]
            lack = [r for r in need if not r["has_card_test"]]
            L.append(f"| {name} | {len(need)} | {len(lack)} | {sum(not r['lowering_covered'] for r in lack)} |")
        L.append("")
    prot_counts = Counter(p for r in cards.values() for p in r["protection_constructs"])
    L.append("Protection constructs by card count: " + ", ".join(f"`{k}` {n}" for k, n in sorted(prot_counts.items(), key=lambda kv: (-kv[1], kv[0]))))
    L.append("")
    if decks is not None:
        L.append("### Top-10-deck DSL cards: clause counts (all of them)\n")
        L.append("| card | clauses (effect-only) | protection | own test | lowering-covered |")
        L.append("|---|---|---|---|---|")
        for cid in sorted({c for d in decks for c in d["cards"] if c in cards}):
            r = cards[cid]
            L.append(
                f"| {cid} | {r['clause_count']} ({r['effect_clause_count']}) | "
                f"{', '.join(r['protection_constructs']) or '-'} | {'yes' if r['has_card_test'] else 'NO'} | "
                f"{'yes' if r['lowering_covered'] else 'no'} |"
            )
        L.append("")
    return L


def render_report(payload: dict, *, decks: list[dict] | None, deck_library: dict, cards_json: dict, admissibility: dict) -> str:
    s = payload["summary"]
    cons = payload["constructs"]
    cards = payload["cards"]
    L: list[str] = []
    L.append("# Task 8a -- lowering-coverage measurement\n")
    L.append(
        "Generated by `PYTHONPATH=code python -m tools.lowering_coverage --report ...` from "
        "`qa/impact/lowering_coverage.json`. Attribution is token presence (see caveats).\n"
    )
    L.append("## 0. Headline\n")
    lists_all = ex13_lists(deck_library)

    def _union(decklists: Iterable[list[str]]) -> int:
        return len({x for dk in decklists for cid in dk if cid in cards for x in cards[cid]["untested_constructs"]})

    if decks is not None:
        L.append(
            f"- Top-10 decks: {sum(deck_verdict(d['cards'], payload, cards_json)['lowering_covered'] for d in decks)}"
            f"/{len(decks)} fully lowering-covered; union of untested constructs across them: "
            f"{_union(d['cards'] for d in decks)}."
        )
    L.append(
        f"- EX13 lists: {sum(deck_verdict(dk, payload, cards_json)['lowering_covered'] for _a, dk in lists_all)}"
        f"/{len(lists_all)} covered; union of untested constructs: {_union(dk for _a, dk in lists_all)} "
        f"(focus leaders: {_union(dk for a, dk in lists_all if a in FOCUS_LEADERS)})."
    )
    lacking = sorted(
        cid for cid, r in cards.items() if (r["clause_count"] >= 4 or r["protection_constructs"]) and not r["has_card_test"]
    )
    L.append(
        "- Cards meeting `clauses>=4 OR protection` without their own card test (admissibility scan: "
        "`cards_behavioral/<set>/<id>.rs` or a ledger `test_path`; dedicated top-level test files such as "
        f"`tests/bt7_063_full_behavior.rs` are not seen): {', '.join(lacking) or 'none'}.\n"
    )
    L.append("## 1. Constructs\n")
    L.append("| kind | total | tested | behavioural | parsed-only | untested | used by cards | used & untested |")
    L.append("|---|---|---|---|---|---|---|---|")
    for kind in KINDS:
        k = s["constructs"][kind]
        L.append(
            f"| {kind} | {k['total']} | {k['tested']} | {k['tested_behaviourally']} | {k['parsed_only']} | "
            f"{k['untested']} | {k['used_by_cards']} | {k['used_untested']} |"
        )
    c = s["cards"]
    L.append(
        f"\nCards with a DSL spec: {c['total']}; lowering-covered {c['lowering_covered']} "
        f"(behaviourally {c['behaviourally_covered']}); not covered {c['not_covered']}. "
        f"Scanned {s['scanned_files']} mechanics test files.\n"
    )
    L.append("### Untested constructs USED by cards (ranked by EX13 lists blocked, then cards)\n")
    L.append(f"EX13 lists in the library: {s['ex13_lists']}.\n")
    L.append("| construct | EX13 lists blocked | cards blocked | example cards | near-miss files (engine-level, not credited) |")
    L.append("|---|---|---|---|---|")
    blocks: dict[str, list[str]] = defaultdict(list)
    for cid, row in cards.items():
        for cc in row["untested_constructs"]:
            blocks[cc].append(cid)
    for t in s["top_untested"]:
        ex = ", ".join(sorted(blocks[t["construct"]])[:6])
        nm = cons[t["construct"]].get("near_miss_files", [])
        nm_txt = ", ".join(f"`{f}`" for f in nm[:3]) + (f" (+{len(nm) - 3})" if len(nm) > 3 else "")
        L.append(f"| `{t['construct']}` | {t['ex13_lists_blocked']} | {t['cards_blocked']} | {ex} | {nm_txt or '-'} |")
    for kind in KINDS:
        unused = sorted(v["name"] for v in cons.values() if v["kind"] == kind and not v["tested"] and f"{kind}:{v['name']}" not in blocks)
        L.append(f"\nUntested `{kind}` constructs used by NO card ({len(unused)}): " + (", ".join(f"`{u}`" for u in unused) or "none"))
    L.append("")

    if decks is not None:
        L.append("## 2. DigiLab-11087 top-10 decks\n")
        L.append("| deck | DSL covered | DSL untested | parsed-only | unimplemented | vanilla | lowering-covered | untested constructs |")
        L.append("|---|---|---|---|---|---|---|---|")
        full = 0
        for d in decks:
            v = deck_verdict(d["cards"], payload, cards_json)
            full += v["lowering_covered"]
            cnt = v["counts"]
            L.append(
                f"| {d['name']} | {cnt.get('dsl_covered', 0)} | {cnt.get('dsl_untested', 0)} | {cnt.get('dsl_parsed_only', 0)} | "
                f"{cnt.get('unimplemented', 0) + cnt.get('unknown', 0)} | {cnt.get('vanilla', 0)} | "
                f"{'yes' if v['lowering_covered'] else 'no'} | {', '.join(f'`{x}`' for x in v['untested']) or '-'} |"
            )
        L.append(f"\nDecks fully lowering-covered: {full}/{len(decks)}.\n")
        L.append("Per-deck unimplemented cards (no YAML, printed text present):\n")
        for d in decks:
            v = deck_verdict(d["cards"], payload, cards_json)
            un = sorted(cid for cid, st in v["statuses"].items() if st in ("unimplemented", "unknown"))
            if un:
                L.append(f"- {d['name']}: {', '.join(un)}")
        L.append("")

    L.append("## 3. Library (EX13 lists)\n")
    lists = ex13_lists(deck_library)
    per_arch: dict[str, list[dict]] = defaultdict(list)
    for arch, deck in lists:
        per_arch[arch].append(deck_verdict(deck, payload, cards_json))
    tot = len(lists)
    cov = sum(v["lowering_covered"] for vs in per_arch.values() for v in vs)
    beh = sum(v["behaviourally_covered"] for vs in per_arch.values() for v in vs)
    dsl_only = sum(v["dsl_covered_ignoring_unimplemented"] for vs in per_arch.values() for v in vs)
    L.append(
        f"EX13 lists: {tot}; fully lowering-covered {cov} (behaviourally {beh}); "
        f"no untested construct among their DSL cards (ignoring unimplemented cards) {dsl_only}.\n"
    )
    L.append("| leader | lists | covered | DSL-clean (ignoring unimplemented) | lists with unimplemented cards | untested constructs (union) |")
    L.append("|---|---|---|---|---|---|")
    for leader in FOCUS_LEADERS:
        vs = per_arch.get(leader, [])
        union = sorted({x for v in vs for x in v["untested"]})
        L.append(
            f"| {leader} | {len(vs)} | {sum(v['lowering_covered'] for v in vs)} | "
            f"{sum(v['dsl_covered_ignoring_unimplemented'] for v in vs)} | "
            f"{sum(1 for v in vs if v['counts'].get('unimplemented', 0) + v['counts'].get('unknown', 0))} | "
            f"{', '.join(f'`{x}`' for x in union) or '-'} |"
        )
    L.append("")

    L.append("## 4. Versus today's per-card-test criterion\n")
    adm = admissibility.get("cards", {})
    becomes = sorted(cid for cid, r in cards.items() if r["lowering_covered"] and adm.get(cid, {}).get("reason") == "untested")
    loses = sorted(cid for cid, r in cards.items() if not r["lowering_covered"] and adm.get(cid, {}).get("admissible"))
    other = sorted(cid for cid, r in cards.items() if r["lowering_covered"] and adm.get(cid, {}).get("reason") not in (None, "untested"))
    L.append(f"- DSL cards today `untested` but lowering-covered (would flip IN): {len(becomes)} -- {', '.join(becomes) or '-'}")
    L.append(f"- DSL cards admissible today but NOT lowering-covered (would flip OUT): {len(loses)}")
    reasons = Counter(c for cid in loses for c in cards[cid]["untested_constructs"])
    L.append("  - top constructs behind the flip-outs: " + ", ".join(f"`{k}` ({n})" for k, n in reasons.most_common(15)))
    L.append(f"- Lowering-covered but blocked today for another reason (failed verdict / divergence): {len(other)} -- {', '.join(other) or '-'}")
    n_adm_dsl = sum(1 for cid in cards if adm.get(cid, {}).get("admissible"))
    L.append(f"- DSL cards admissible today: {n_adm_dsl}; lowering-covered: {sum(r['lowering_covered'] for r in cards.values())}.")
    L.append(
        "- Non-DSL cards (no YAML) are unchanged by this criterion: a card with printed text and no YAML is "
        "unimplemented under either rule; a vanilla card has no constructs.\n"
    )

    L.extend(render_attention(payload, decks=decks, deck_library=deck_library))
    L.append("## 6. Caveats\n")
    L.append(
        "- Token presence is a proxy for `tested`: a test that names `CompiledStep::Draw` in a fixture it never "
        "asserts on still counts; a construct exercised only through a cross-file helper (`support.rs`, "
        "`include_str!` of a card YAML) does NOT count. Attribution is per file + same-file helper closure by name, "
        "so a helper whose name collides with a method call can over-attribute."
    )
    L.append(
        "- `tested` (the brief's criterion) includes parse-only evidence. `tested_behaviourally` requires a hit in a "
        "file that drives the engine; the rest are `parsed_only` (parsed, not behaviourally tested)."
    )
    po = sorted(k for k, v in cons.items() if v["parsed_only"])
    L.append(f"- Parsed-only constructs ({len(po)}): " + (", ".join(f"`{k}`" for k in po) or "none"))
    yk_parse = sorted(
        k
        for k, v in cons.items()
        if v["parsed_only"] and all(set(sig) == {"yaml_key"} for sig in v["signals"].values())
    )
    L.append(f"- Of those, attributed ONLY by a YAML-key match in a parse-only file ({len(yk_parse)}): " + (", ".join(f"`{k}`" for k in yk_parse) or "none"))
    L.append(f"- Parse-only files ({len(s['parse_only_files'])}): " + ", ".join(f"`{p}`" for p in s["parse_only_files"]))
    L.append(
        "- Keywords: the universe is the engine `Keyword` enum plus names a card YAML grants (`keyword:` / "
        "`keywords:`). Printed keywords parsed from card text (not in the YAML) are not part of a card's construct "
        "list, so a card is not blocked by its printed-keyword mechanics here."
    )
    L.append(
        "- Comparator predicate keys (`dp`, `level`, `play_cost`, ...) lower to flat fields; they are credited only "
        "by a YAML-key match or by the `<key>_eq` field, not by `dp_lte` etc."
    )
    return "\n".join(L) + "\n"


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--check", action="store_true", help="exit 1 if the committed artifact differs from a fresh build")
    ap.add_argument("--report", type=Path, help="write the measurement markdown here (from the committed artifact)")
    ap.add_argument("--decks", type=Path, help="JSON list of {name, cards} decks for the report")
    args = ap.parse_args(argv)

    if args.report:
        payload = json.loads(args.out.read_text(encoding="utf-8"))
        decks = json.loads(args.decks.read_text(encoding="utf-8")) if args.decks else None
        text = render_report(
            payload,
            decks=decks,
            deck_library=json.loads(DECK_LIBRARY.read_text(encoding="utf-8")),
            cards_json=json.loads((REPO / "data" / "cards.json").read_text(encoding="utf-8")),
            admissibility=json.loads((REPO / "data" / "card_admissibility.json").read_text(encoding="utf-8")),
        )
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text, encoding="utf-8", newline="\n")
        print(f"wrote {args.report}")
        return 0

    fresh = serialise(build())
    if args.check:
        if not args.out.exists():
            print(f"error: {args.out} is missing; run without --check to create it", file=sys.stderr)
            return 1
        existing = args.out.read_bytes().decode("utf-8").replace("\r\n", "\n")
        if existing != fresh:
            print(
                f"error: {args.out} is stale; run `PYTHONPATH=code python -m tools.lowering_coverage`",
                file=sys.stderr,
            )
            return 1
        print(f"lowering coverage is current: {args.out}")
        return 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(fresh, encoding="utf-8", newline="\n")
    s = json.loads(fresh)["summary"]
    for kind in KINDS:
        k = s["constructs"][kind]
        print(f"{kind}: {k['tested']}/{k['total']} tested ({k['tested_behaviourally']} behavioural), {k['untested']} untested")
    print(f"cards: {s['cards']['lowering_covered']}/{s['cards']['total']} lowering-covered")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
