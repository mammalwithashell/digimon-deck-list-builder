"""Generate the keyword semantics matrix used by the verification ladder.

The source of truth for rule-derived rows is
``docs/digimon-rules/keyword-semantics.md``. Engine-only keyword variants are
listed as explicit extension rows so the Rust completeness gate can still fail
when a new ``Keyword`` variant appears without a semantics row.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Iterable


SCHEMA_VERSION = 1
DEFAULT_SOURCE_DOC = Path("docs/digimon-rules/keyword-semantics.md")
DEFAULT_ENGINE_ENUM = Path("code/digimon-engine/src/enums.rs")
DEFAULT_OUTPUT = Path("qa/rules/keyword_semantics_matrix.json")

KEYWORD_ID_MAP = {
    "`<Security A. +x / -x>` (was `<Security Attack>`)": [
        "SecurityAttackPlus",
        "SecurityAttackMinus",
    ],
    "`<Blocker>`": ["Blocker"],
    "`<Recovery +x (Deck)>`": ["Recovery"],
    "`<Piercing>`": ["Piercing"],
    "`<Draw x>`": ["DrawX"],
    "`<Jamming>`": ["Jamming"],
    "`<Digisorption -x>`": ["Digisorption"],
    "`<Reboot>`": ["Reboot"],
    "`<De-Digivolve x>`": ["DeDigivolve"],
    "`<Retaliation>`": ["Retaliation"],
    "`<Digi-Burst X>`": ["DigiBurst"],
    "`<Rush>`": ["Rush"],
    "`<Blitz>`": ["Blitz"],
    "`<Delay>`": ["Delay"],
    "`<Decoy (X)>`": ["Decoy"],
    "`<Armor Purge>`": ["ArmorPurge"],
    "`<Save>`": ["Save"],
    "`<Material Save x>`": ["MaterialSave"],
    "`<Evade>`": ["Evade"],
    "`<Raid>`": ["Raid"],
    "`<Alliance>`": ["Alliance"],
    "`<Barrier>`": ["Barrier"],
    "`<Blast Digivolve>`": ["BlastDigivolve"],
    "`<Fortitude>`": ["Fortitude"],
    "`<Mind Link>`": ["MindLink"],
    "`<Partition (…)>`": ["Partition"],
    "`<Collision>`": ["Collision"],
    "`<Blast DNA Digivolve (…)>`": ["BlastDnaDigivolve"],
    "`<Scapegoat>`": ["Scapegoat"],
    "`<Vortex>`": ["Vortex"],
    "`<Overclock (…)>`": ["Overclock"],
    "`<Iceclad>`": ["Iceclad"],
    "`<Decode (…)>`": ["Decode"],
    "`<Fragment (X)>`": ["Fragment"],
    "`<Execute>`": ["Execute"],
    "`<Progress>`": ["Progress"],
    "`<Link +x>`": ["Link"],
    "`<Training>`": ["Training"],
    # 4.x keywords (Comprehensive Rules Ver. 4.3 §16-42 … §16-47). Those with no
    # engine `Keyword` variant yet still get a rules row so the matrix is the
    # full §16 denominator; the completeness gate only checks engine -> matrix.
    "`<Use Req.>`": ["UseReq"],
    "`<Ascension>`": ["Ascension"],
    "`<Engage>`": ["Engage"],
    "`<Guard>`": ["Guard"],
    "`<Detach (…)>`": ["Detach"],
    "`<Succession (…)>`": ["Succession"],
}

PARAMETERIZED_KEYWORDS = {
    "SecurityAttackPlus",
    "SecurityAttackMinus",
    "Recovery",
    "DrawX",
    "Digisorption",
    "DeDigivolve",
    "DigiBurst",
    "Decoy",
    "MaterialSave",
    "Partition",
    "BlastDnaDigivolve",
    "Overclock",
    "Decode",
    "Fragment",
    "Link",
    "Detach",
    "Succession",
}

# Rows for engine `Keyword` variants that are NOT §16 keyword effects in the
# Comprehensive Rules. Since Ver. 4.3 made <Engage>/<Guard>/<Ascension> real §16
# rules (16-44 / 16-45 / 16-43) they come from the table above; only Arts
# Digivolve remains an extension: the manual defines it as a DUAL-card *rule*
# (§4-20, §2-11), not a keyword effect, but the engine models it as a Keyword.
EXTENSION_ROWS = [
    {
        "keyword_id": "ArtsDigivolve",
        "display": "<Arts Digivolve>",
        "kind": "optional",
        "type_when": "DUAL card used as an Option; replaces the post-use trash (§4-20, overwrite processing per §18-1)",
        "semantics": (
            "Instead of the pending-processing trash after using the DUAL card as "
            "an Option, one of your cards on the field may digivolve into it "
            "without paying the cost; modeled by the engine as a card-specific keyword."
        ),
        "rule": "4-20",
        "source": "engine_extension",
    },
]


def load_engine_keyword_variants(path: Path = DEFAULT_ENGINE_ENUM) -> list[str]:
    text = path.read_text(encoding="utf-8")
    start = text.index("pub enum Keyword {")
    rest = text[start:].split("{", 1)[1]
    block = rest.split("\n}", 1)[0]
    variants: list[str] = []
    for raw_line in block.splitlines():
        line = raw_line.split("//", 1)[0].strip()
        match = re.match(r"^([A-Z][A-Za-z0-9_]*)(?:\([^)]*\))?,", line)
        if match:
            variants.append(match.group(1))
    return variants


def _markdown_table_rows(source: Path) -> Iterable[dict[str, str]]:
    for line in source.read_text(encoding="utf-8").splitlines():
        if not line.startswith("| "):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 5 or cells[0] in {"Keyword", "---"}:
            continue
        if cells[0].startswith("---"):
            continue
        yield {
            "display": cells[0],
            "raw_kind": cells[1],
            "type_when": cells[2],
            "semantics": cells[3],
            "rule": cells[4],
        }


def _kind(raw_kind: str) -> str:
    if raw_kind == "Persistent":
        return "persistent"
    if raw_kind == "Mandatory":
        return "mandatory"
    if raw_kind == "Optional":
        return "optional"
    if raw_kind.startswith("Opt-cost"):
        return "optional_cost_then_mandatory"
    raise ValueError(f"unknown keyword kind: {raw_kind}")


def _decorate(row: dict, engine_keywords: set[str]) -> dict:
    kind = row["kind"]
    semantics_lower = row["semantics"].lower()
    keyword_id = row["keyword_id"]
    return {
        **row,
        "optionality": {
            "persistent": "persistent",
            "mandatory": "mandatory",
            "optional": "optional",
            "optional_cost_then_mandatory": "optional_cost_then_mandatory",
        }[kind],
        "optional_choice": kind in {"optional", "optional_cost_then_mandatory"},
        "cost_optional": kind == "optional_cost_then_mandatory",
        "effect_mandatory_after_cost": kind == "optional_cost_then_mandatory",
        "once_per_window": "once" in semantics_lower or "max 1" in semantics_lower,
        "parameterized": keyword_id in PARAMETERIZED_KEYWORDS,
        "engine_keyword": keyword_id in engine_keywords,
    }


def build_keyword_semantics_matrix(
    source: Path = DEFAULT_SOURCE_DOC,
    *,
    engine_enum: Path = DEFAULT_ENGINE_ENUM,
) -> dict:
    engine_keywords = set(load_engine_keyword_variants(engine_enum))
    rows: list[dict] = []

    for parsed in _markdown_table_rows(source):
        keyword_ids = KEYWORD_ID_MAP.get(parsed["display"])
        if keyword_ids is None:
            raise ValueError(f"unmapped keyword row: {parsed['display']}")
        for keyword_id in keyword_ids:
            kind = _kind(parsed["raw_kind"])
            rows.append(
                _decorate(
                    {
                        "keyword_id": keyword_id,
                        "display": parsed["display"].replace("`", ""),
                        "kind": kind,
                        "type_when": parsed["type_when"],
                        "semantics": parsed["semantics"],
                        "rule": parsed["rule"],
                        "source": "rules",
                    },
                    engine_keywords,
                )
            )

    for extension in EXTENSION_ROWS:
        rows.append(_decorate(dict(extension), engine_keywords))

    matrix_ids = {row["keyword_id"] for row in rows if row["engine_keyword"]}
    missing = sorted(engine_keywords - matrix_ids)
    if missing:
        raise ValueError(f"engine Keyword variants missing matrix rows: {missing}")

    return {
        "schema_version": SCHEMA_VERSION,
        "source_doc": str(source).replace("\\", "/"),
        "engine_enum": str(engine_enum).replace("\\", "/"),
        "rows": rows,
    }


def canonical_json(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE_DOC)
    parser.add_argument("--engine-enum", type=Path, default=DEFAULT_ENGINE_ENUM)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    matrix = build_keyword_semantics_matrix(args.source, engine_enum=args.engine_enum)
    rendered = canonical_json(matrix)
    if args.check:
        actual = args.out.read_text(encoding="utf-8")
        if actual != rendered:
            raise SystemExit(f"{args.out} is stale; regenerate keyword semantics matrix")
        print(f"{args.out} is current")
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"wrote {args.out} ({len(matrix['rows'])} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
