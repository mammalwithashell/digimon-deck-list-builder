"""Project the scenario-authoring contract out of `docs/DCGO_EXAM.md`.

The contract is prose in the operating manual. An agent composing a line needs
it in targeted pieces, not as an 800-line read -- but a second hand-maintained
copy would drift from the manual within a release. So this generates the guide
FROM the manual, and `--check` gates that they still agree, the same drift
pattern `impact_index` and `keyword_semantics_matrix` already use.

Each topic maps to the questions authors actually got wrong during the first
campaign; the mapping lives in `TOPIC_ANCHORS` and is deliberately explicit, so
a heading rename in the manual fails this generator instead of silently
emptying a topic the agent depends on.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

TOPICS = ("format", "steps", "prompts", "decks", "assert", "verdicts")

#: topic -> (title, substrings that identify its heading(s) in the manual)
#:
#: Each needle tuple deliberately carries more than one phrase: the manual's
#: real `##`/`###` headings (verified against docs/DCGO_EXAM.md -- see the
#: task report for why they differ from the first guess) plus enough of a
#: generic phrase that a differently-worded doc still resolves the same
#: topic rather than failing outright.
TOPIC_ANCHORS: dict[str, tuple[str, tuple[str, ...]]] = {
    "format": (
        "Scenario format",
        (
            "scenario",  # "The scenario file" (real) / "Scenario format" (generic)
            "is an identity, not a label",  # `clause:` id shape
        ),
    ),
    "steps": (
        "Step vocabulary",
        (
            "step vocabulary",
            "`do:` is symbolic",
            "`main:`",  # field [Main] / <Delay> activation
            "selection steps (`select:`)",
            "`ordinal:`",  # stacked-trigger disambiguator
        ),
    ),
    "prompts": (
        "Prompt kinds and the two folds",
        (
            "prompt kind",
            "`expect:` is asserted",  # the 13 decision kinds
            "selection surface",  # our SelectionKind vs DCGO's widgets
            "the table",
            "translation mechanisms",  # ordinal:/trigger:/optional_gate_fold/...
        ),
    ),
    "decks": (
        "Decks, stacking, and the sim-only trap",
        (
            "decks and stacking",
            "is a prefix",  # `stack` is a PREFIX, initial shuffle only
            "stack position becomes",
            "sim-only",  # oracle vs sim-only deal different hands
        ),
    ),
    "assert": (
        "Assertions are backfilled",
        (
            "assertions",
            "assertion backfill",
        ),
    ),
    "verdicts": (
        "The five verdict classes",
        (
            "verdict classes",
            "honesty constraints",
        ),
    ),
}


def _sections(doc_text: str) -> list[tuple[str, str]]:
    """Split the manual into (heading, body) pairs on ATX headings."""
    out: list[tuple[str, str]] = []
    heading = None
    buf: list[str] = []
    for line in doc_text.splitlines():
        m = re.match(r"^#{2,4}\s+(.*)$", line)
        if m:
            if heading is not None:
                out.append((heading, "\n".join(buf).strip()))
            heading = m.group(1).strip()
            buf = []
        else:
            buf.append(line)
    if heading is not None:
        out.append((heading, "\n".join(buf).strip()))
    return out


def build_guide(doc_text: str) -> dict:
    """Build the guide. Raises ``ValueError`` if any topic comes out empty."""
    sections = _sections(doc_text)
    topics: dict[str, dict] = {}

    for topic, (title, needles) in TOPIC_ANCHORS.items():
        body_parts: list[str] = []
        for heading, section_body in sections:
            low = heading.lower()
            if not any(n.lower() in low for n in needles):
                continue
            # A matched heading with an empty body (a short organizational
            # sub-header immediately followed by a deeper heading, e.g. "`stack`
            # is a PREFIX ..." right above "What each stack position becomes")
            # still carries load-bearing wording IN the heading itself. Dropping
            # it because its own body is blank would silently discard exactly
            # the sentence an anchor was written to catch.
            if section_body:
                body_parts.append(f"### {heading}\n\n{section_body}")
            else:
                body_parts.append(f"### {heading}")
        body = "\n\n".join(body_parts).strip()
        if not body:
            raise ValueError(
                f"topic {topic!r} matched no section in the manual -- a heading was "
                f"probably renamed. Fix TOPIC_ANCHORS rather than shipping an empty "
                f"topic: an empty topic answers an agent's question with silence."
            )
        topics[topic] = {"title": title, "body": body}

    return {"version": 1, "source": "docs/DCGO_EXAM.md", "topics": topics}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--doc", type=Path, default=Path("docs/DCGO_EXAM.md"))
    parser.add_argument("--out", type=Path, default=Path("qa/exam-authoring-guide.json"))
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the generated guide differs from --out (drift gate)",
    )
    args = parser.parse_args(argv)

    guide = build_guide(args.doc.read_text(encoding="utf-8"))
    text = json.dumps(guide, indent=2, sort_keys=True) + "\n"

    if args.check:
        if not args.out.exists():
            print(f"authoring guide missing: {args.out}")
            print(f"Run `python -m tools.clause_coverage.authoring_guide --out {args.out}`.")
            return 1
        if args.out.read_text(encoding="utf-8") != text:
            print(f"authoring guide is stale: {args.out}")
            print(f"Run `python -m tools.clause_coverage.authoring_guide --out {args.out}`.")
            return 1
        print(f"authoring guide is current: {args.out}")
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
