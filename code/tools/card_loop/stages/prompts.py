"""Versioned stage prompt templates (design D14) and their strict renderer.

One template per worker stage, `code/tools/card_loop/prompts/<stage>.md`. The
first line is the version header, `version: <n>`; it is metadata (the attempt
ledger's `prompt_version`, which the scorecard partitions on), so it is
stripped from the rendered text. Bump it on any change a model could notice.

Substitution is deliberately dumb and strict:

* `{field}` -- a lower-case identifier in single braces -- is a field. Anything
  else in braces (a JSON example such as `{"verdict": ...}`) is left alone.
* `{{` and `}}` are literal braces.
* A field the template names but the caller omits raises `PromptFieldError`,
  and so does a field the caller passes that the template does not name: a
  silently ignored value is how a prompt drifts away from its executor.
* Values are inserted verbatim and never re-expanded.

Templates are narrow (D14): they say what to produce (files at canonical paths,
the JSON result shape) and point at the contracts -- the exam MCP tools,
`docs/digimon-rules/`, the DSL test API, the no-approximations rules -- instead
of embedding them, and they never send a worker into an orchestrator skill.
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

from ..contracts import STAGES

PROMPTS_DIR = Path(__file__).resolve().parent.parent / "prompts"

_VERSION = re.compile(r"version:\s*(\S+)\s*")
_TOKEN = re.compile(r"\{\{|\}\}|\{([a-z_][a-z0-9_]*)\}")


class PromptFieldError(ValueError):
    """A template and the fields given to it do not match."""


@dataclass(frozen=True)
class Template:
    version: str
    body: str
    fields: tuple[str, ...]        # in first-use order


def parse_template(text: str) -> Template:
    first, _, body = text.partition("\n")
    m = _VERSION.fullmatch(first.strip())
    if not m:
        raise PromptFieldError(f"template must start with a `version: <n>` line, got {first!r}")
    seen: list[str] = []
    for tok in _TOKEN.finditer(body):
        name = tok.group(1)
        if name and name not in seen:
            seen.append(name)
    return Template(version=m.group(1), body=body, fields=tuple(seen))


def render_text(text: str, **fields) -> str:
    """Render a template's text (header included) with exactly its fields."""
    tpl = parse_template(text)
    missing = [f for f in tpl.fields if f not in fields]
    extra = sorted(set(fields) - set(tpl.fields))
    if missing or extra:
        parts = []
        if missing:
            parts.append(f"missing field(s) {missing}")
        if extra:
            parts.append(f"unknown field(s) {extra}")
        raise PromptFieldError("; ".join(parts))

    def sub(m: re.Match) -> str:
        tok = m.group(0)
        if tok == "{{":
            return "{"
        if tok == "}}":
            return "}"
        return str(fields[m.group(1)])

    return _TOKEN.sub(sub, tpl.body)


def template_path(stage: str) -> Path:
    if stage not in STAGES:
        raise ValueError(f"unknown stage {stage!r}")
    return PROMPTS_DIR / f"{stage}.md"


@lru_cache(maxsize=None)
def _load(path: str, mtime: float) -> Template:
    return parse_template(Path(path).read_text(encoding="utf-8"))


def template(stage: str) -> Template:
    p = template_path(stage)
    return _load(str(p), p.stat().st_mtime)


def version(stage: str) -> str:
    """The stage template's `version:` value -- the attempt's `prompt_version`."""
    return template(stage).version


def fields(stage: str) -> tuple[str, ...]:
    return template(stage).fields


def render(stage: str, **values) -> str:
    """The rendered prompt for `stage` (header stripped). Strict both ways."""
    p = template_path(stage)
    return render_text(p.read_text(encoding="utf-8"), **values)


def available_stages() -> tuple[str, ...]:
    return tuple(s for s in STAGES if template_path(s).is_file())
