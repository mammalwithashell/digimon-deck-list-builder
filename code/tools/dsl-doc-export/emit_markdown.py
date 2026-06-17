#!/usr/bin/env python3
"""Generate the DSL Vocabulary Reference block inside docs/RUST_DSL_AGENT_GUIDE.md.

This is the Python *formatter* half of a Rust-emit + Python-format codegen pair,
mirroring `code/tools/action-space-export/emit_csharp.py` (rule 27). The Rust
*emit* half is the existing `dsl-schema-export` binary:

    cargo run -q -p dsl-schema-export | python code/tools/dsl-doc-export/emit_markdown.py

The schemars JSON gives us, for the four authoring enums:
  * PredicateSpec / Timing / DeclarativeKind — real YAML keys directly.
  * StepSpec — only PascalCase variant names + docs + arg refs (StepSpec has a
    hand-written serde impl, so schemars never sees the snake keys). We join the
    variants to their YAML keys by reading the authoritative `kv!(s, "<key>", v)`
    serialize literals in code/digimon-dsl/src/step.rs.

Usage counts + a fixture card come from scanning the card corpus. Counts are
ADVISORY and deliberately excluded from the drift signature (see --check): only
the structural set {(key, family, arg, doc)} gates CI, so adding a card never
fails the build, but adding an enum variant does until the block is regenerated.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

BEGIN = "<!-- BEGIN GENERATED:dsl-vocab -->"
END = "<!-- END GENERATED:dsl-vocab -->"
SHA_RE = re.compile(r"<!-- vocab-structural-sha:\s*([0-9a-f]{64})\s*-->")

# ---------------------------------------------------------------------------
# repo paths (script lives at code/tools/dsl-doc-export/emit_markdown.py)
# ---------------------------------------------------------------------------
REPO = Path(__file__).resolve().parents[3]
GUIDE = REPO / "docs" / "RUST_DSL_AGENT_GUIDE.md"
STEP_RS = REPO / "code" / "digimon-dsl" / "src" / "step.rs"
CLAUSE_RS = REPO / "code" / "digimon-dsl" / "src" / "clause.rs"
CARDS_DIR = REPO / "code" / "digimon-engine" / "cards"


def snake(s: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()


# ---------------------------------------------------------------------------
# schema type rendering
# ---------------------------------------------------------------------------
def render_type(node: dict) -> str:
    """Best-effort one-token rendering of a schema node as an arg shape."""
    if not isinstance(node, dict):
        return ""
    if "$ref" in node:
        return node["$ref"].split("/")[-1]
    for comb in ("anyOf", "allOf", "oneOf"):
        if comb in node:
            parts = [render_type(x) for x in node[comb] if x.get("type") != "null" or "$ref" in x]
            parts = [p for p in parts if p and p != "null"]
            nullable = any(x.get("type") == "null" for x in node[comb])
            inner = parts[0] if parts else "any"
            return inner + ("?" if nullable else "")
    t = node.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        base = non_null[0] if non_null else "null"
        suffix = "?" if "null" in t else ""
        if base == "array":
            return f"[{render_type(node.get('items', {}))}]{suffix}"
        return _scalar(base) + suffix
    if t == "array":
        return f"[{render_type(node.get('items', {}))}]"
    if t == "object":
        return "map"
    if t:
        return _scalar(t)
    return ""


def _scalar(t: str) -> str:
    return {"integer": "i32", "boolean": "bool", "string": "str", "number": "num"}.get(t, t)


def clean_doc(doc: str | None) -> str:
    if not doc:
        return ""
    return " ".join(doc.split()).replace("|", "\\|")


# ---------------------------------------------------------------------------
# enum extraction from the schema (+ step.rs key join)
# ---------------------------------------------------------------------------
def step_key_map(step_rs: str) -> dict[str, str]:
    """Variant(PascalCase) -> yaml key, from the serialize `kv!` literals."""
    out: dict[str, str] = {}
    for m in re.finditer(r'StepSpec::([A-Za-z0-9]+)\b[^=\n]*=>\s*\{?\s*kv!\(\s*s\s*,\s*"([a-z0-9_]+)"', step_rs):
        out[m.group(1)] = m.group(2)
    return out


def kind_doc_map(clause_rs: str) -> dict[str, str]:
    """DeclarativeKind variant -> `///` doc (snake key -> doc)."""
    out: dict[str, str] = {}
    body = re.split(r"enum DeclarativeKind\b", clause_rs)[1].split("{", 1)[1]
    doc: list[str] = []
    for line in body.split("\n"):
        s = line.strip()
        if s.startswith("///"):
            doc.append(s[3:].strip())
            continue
        if s.startswith("#["):
            continue
        mv = re.match(r"([A-Z][A-Za-z0-9]+)", s)
        if mv:
            out[snake(mv.group(1))] = " ".join(doc).strip()
            doc = []
        if s == "}":
            break
    return out


def extract(defs: dict, step_rs: str, clause_rs: str) -> dict[str, list[dict]]:
    rows: dict[str, list[dict]] = {"steps": [], "predicates": [], "timings": [], "kinds": []}

    # --- steps ---
    keymap = step_key_map(step_rs)
    for v in defs["StepSpec"]["oneOf"]:
        variant = list(v["properties"].keys())[0]  # PascalCase
        key = keymap.get(variant, snake(variant))
        arg = render_type(list(v["properties"].values())[0])
        rows["steps"].append({"key": key, "arg": arg, "doc": clean_doc(v.get("description"))})

    # --- predicates ---
    for key, node in defs["PredicateSpec"]["properties"].items():
        rows["predicates"].append(
            {"key": key, "arg": render_type(node), "doc": clean_doc(node.get("description"))}
        )

    # --- timings (oneOf: one big enum subschema + per-doc single-value ones) ---
    for sub in defs["Timing"]["oneOf"]:
        doc = clean_doc(sub.get("description"))
        for val in sub.get("enum", []):
            rows["timings"].append({"key": val, "arg": "", "doc": doc})

    # --- declarative kinds ---
    kdocs = kind_doc_map(clause_rs)
    for val in defs["DeclarativeKind"]["enum"]:
        rows["kinds"].append({"key": val, "arg": "", "doc": kdocs.get(val, "")})

    return rows


# ---------------------------------------------------------------------------
# corpus usage scan
# ---------------------------------------------------------------------------
def scan_usage(cards_dir: Path) -> dict[str, tuple[int, str | None]]:
    texts = {p: p.read_text(encoding="utf-8", errors="ignore") for p in cards_dir.rglob("*.yaml")}
    cache: dict[str, tuple[int, str | None]] = {}

    def usage(key: str) -> tuple[int, str | None]:
        if key in cache:
            return cache[key]
        pat = re.compile(rf"(^|\s)-?\s*{re.escape(key)}\s*:", re.M)
        cnt, ex = 0, None
        for p, t in sorted(texts.items()):
            if pat.search(t):
                cnt += 1
                if ex is None:
                    ex = p.relative_to(cards_dir).as_posix()
        cache[key] = (cnt, ex)
        return cache[key]

    return usage  # type: ignore[return-value]


def timing_usage(cards_dir: Path):
    texts = {p: p.read_text(encoding="utf-8", errors="ignore") for p in cards_dir.rglob("*.yaml")}

    def usage(key: str) -> tuple[int, str | None]:
        pat = re.compile(rf"\b{re.escape(key)}\b")
        cnt, ex = 0, None
        for p, t in sorted(texts.items()):
            if pat.search(t):
                cnt += 1
                if ex is None:
                    ex = p.relative_to(cards_dir).as_posix()
        return cnt, ex

    return usage


# ---------------------------------------------------------------------------
# family classification
# ---------------------------------------------------------------------------
def step_family(k: str) -> str:
    if k.startswith(("gain_memory", "lose_memory", "set_memory")):
        return "Memory"
    if "digixros" in k:
        return "DigiXros"
    if "link" in k or k == "app_fuse":
        return "Link / AppFuse"
    if "under_tamer" in k:
        return "Under-Tamer source"
    if "source" in k or k.endswith("sources") or "stacked" in k:
        return "Stack / Source"
    if "security" in k:
        return "Security"
    if "reveal" in k or k == "order_remainder":
        return "Reveal"
    if k.startswith(("select_", "digi_burst", "choose_from", "as_selecting")):
        return "Selection"
    if "replacement" in k:
        return "Replacement-flow"
    if k.startswith(("play_", "use_option")):
        return "Play / zone-in"
    if k.startswith(("draw", "trash_from", "add_to_hand", "return_all", "return_trash",
                     "shuffle", "recover", "move_trash", "trash_opponent", "move_from_breeding")):
        return "Hand / Deck / Trash"
    if k.startswith(("add_dp", "add_modifier", "add_player", "grant_", "arm_")):
        return "Modifier / Keyword"
    if k in ("delete_permanent", "delete_bound_permanents", "return_to_hand", "return_to_deck",
             "suspend", "unsuspend", "de_digivolve", "place_on_security", "play_token",
             "bind_permanent_property", "hatch", "trash_breeding_permanent"):
        return "Field mutation"
    if k.startswith(("battle", "may_attack", "force_attack", "redirect_attack", "cancel_attack",
                     "open_counter", "end_attack", "refire", "refund")):
        return "Combat"
    if k.startswith(("effect_initiated", "may_dna")):
        return "Effect digivolve / DNA"
    if k in ("if", "for_each", "per_selected", "schedule_delayed",
             "schedule_delete_played_at_turn_end", "place_self_as_delay_option", "optional",
             "activation_cost", "link_to_own_digimon"):
        return "Control flow"
    return "Other"


def pred_family(k: str) -> str:
    if k.startswith("event_"):
        return "Event payload"
    if k.startswith("replacement_"):
        return "Replacement payload"
    if k.startswith(("source_", "self_", "host_", "battle_opponent")):
        return "Source-relative"
    if k in ("binding_absent", "binding_card_kind", "binding_count_eq", "binding_exists",
             "binding_owner", "binding_present", "cost_target", "equals", "not_equals",
             "not_in_binding", "is_source"):
        return "Binding"
    if k.startswith("effect_") or k in ("returned_card_matching", "any_returned_card"):
        return "Effect-history"
    if any(x in k for x in ("memory", "security_count", "face_up_security", "opponent_security",
                            "your_turn", "opponents_turn", "all_turns", "can_hatch", "in_breeding",
                            "on_field", "dna_origin")):
        return "Context / global"
    if k in ("all_of", "any_of", "none_of", "not", "other", "owner", "zone", "extra"):
        return "Compound / structural"
    if any(x in k for x in ("count", "any_permanent", "any_field", "no_permanent",
                            "all_permanents", "has_alt_path", "aggregate")):
        return "Aggregate / existential"
    return "Identity / state"


# ---------------------------------------------------------------------------
# rendering
# ---------------------------------------------------------------------------
def structural_sha(rows: dict[str, list[dict]]) -> str:
    """Digest over the structural set only — keys, families, arg shapes, docs.

    Deliberately excludes usage counts + fixture paths so ordinary card
    authoring never trips the drift gate; only enum changes do.
    """
    h = hashlib.sha256()
    for cat in ("steps", "predicates", "timings", "kinds"):
        famfn = {"steps": step_family, "predicates": pred_family}.get(cat, lambda _k: "")
        for r in sorted(rows[cat], key=lambda x: x["key"]):
            h.update(f"{cat}|{r['key']}|{famfn(r['key'])}|{r['arg']}|{r['doc']}\n".encode())
    return h.hexdigest()


def tag(uses: int) -> str:
    if uses == 0:
        return "unused"
    if uses <= 2:
        return "rare"
    return ""


def render_table(items: list[dict], famfn, usage) -> list[str]:
    from collections import defaultdict

    groups: dict[str, list[dict]] = defaultdict(list)
    for it in items:
        groups[famfn(it["key"])].append(it)
    out: list[str] = []
    for fam in sorted(groups):
        gi = groups[fam]
        u = sum(1 for i in gi if usage(i["key"])[0] == 0)
        out.append(f"\n#### {fam} — {len(gi)} ({u} unused)\n")
        out.append("| key | arg | uses | tag | fixture | description |")
        out.append("|-----|-----|------|-----|---------|-------------|")
        for it in sorted(gi, key=lambda x: x["key"]):
            cnt, ex = usage(it["key"])
            arg = f"`{it['arg']}`" if it["arg"] else ""
            out.append(
                f"| `{it['key']}` | {arg} | {cnt} | {tag(cnt)} | "
                f"{('`' + ex + '`') if ex else '—'} | {it['doc']} |"
            )
    return out


def render_flat(items: list[dict], usage) -> list[str]:
    """Timings / kinds: no families, single table."""
    out = ["", "| key | uses | description |", "|-----|------|-------------|"]
    for it in sorted(items, key=lambda x: x["key"]):
        cnt, _ = usage(it["key"])
        out.append(f"| `{it['key']}` | {cnt} | {it['doc']} |")
    return out


def build_block(rows: dict, cards_dir: Path) -> str:
    sha = structural_sha(rows)
    step_use = scan_usage(cards_dir)
    pred_use = scan_usage(cards_dir)
    tim_use = timing_usage(cards_dir)

    n = {k: len(v) for k, v in rows.items()}
    lines = [
        BEGIN,
        f"<!-- vocab-structural-sha: {sha} -->",
        "<!-- DO NOT EDIT BY HAND. Generated by code/tools/dsl-doc-export/emit_markdown.py",
        "     from the digimon-dsl enums. Regenerate with:",
        "       cargo run -q -p dsl-schema-export | python code/tools/dsl-doc-export/emit_markdown.py",
        "     CI gate: .github/workflows/dsl-vocab-doc-drift.yml (rule 27 pattern). -->",
        "",
        "## DSL Vocabulary Reference (generated)",
        "",
        f"Complete, enum-derived index of every authoring primitive: "
        f"**{n['steps']} step verbs**, **{n['predicates']} predicates**, "
        f"**{n['timings']} timings**, **{n['kinds']} declarative kinds**. "
        "`uses` = card YAMLs referencing the key; `tag` flags `unused` (0 uses) "
        "or `rare` (1–2) vocabulary; `fixture` is a real card to open.",
        "",
        "### Step verbs (`process:` / `extra_cost:`)",
    ]
    lines += render_table(rows["steps"], step_family, step_use)
    lines += ["", "### Predicates (`filter:` / `condition:` / `active_when:`)"]
    lines += render_table(rows["predicates"], pred_family, pred_use)
    lines += ["", "### Timings (`when:`)"]
    lines += render_flat(rows["timings"], tim_use)
    lines += ["", "### Declarative kinds (`kind:`)"]
    lines += render_flat(rows["kinds"], tim_use)
    lines += ["", END]
    return "\n".join(lines)


def splice(guide_text: str, block: str) -> str:
    if BEGIN in guide_text and END in guide_text:
        pre = guide_text.split(BEGIN)[0]
        post = guide_text.split(END, 1)[1]
        return pre + block + post
    # first run: append before trailing newline
    return guide_text.rstrip() + "\n\n" + block + "\n"


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--schema", type=Path, help="schema JSON (default: stdin)")
    ap.add_argument("--guide", type=Path, default=GUIDE)
    ap.add_argument("--cards-dir", type=Path, default=CARDS_DIR)
    ap.add_argument("--step-rs", type=Path, default=STEP_RS)
    ap.add_argument("--clause-rs", type=Path, default=CLAUSE_RS)
    ap.add_argument("--check", action="store_true",
                    help="drift gate: compare committed structural sha to freshly-computed; "
                         "exit 1 on mismatch. Ignores usage-count churn.")
    args = ap.parse_args()

    schema_text = args.schema.read_text(encoding="utf-8") if args.schema else sys.stdin.read()
    schema = json.loads(schema_text)
    defs = schema.get("definitions") or schema.get("$defs") or {}
    rows = extract(defs, args.step_rs.read_text(encoding="utf-8"),
                   args.clause_rs.read_text(encoding="utf-8"))

    if args.check:
        fresh = structural_sha(rows)
        guide_text = args.guide.read_text(encoding="utf-8")
        m = SHA_RE.search(guide_text)
        committed = m.group(1) if m else None
        if committed != fresh:
            print("DSL vocab drift detected.", file=sys.stderr)
            print(f"  committed structural sha: {committed}", file=sys.stderr)
            print(f"  current   structural sha: {fresh}", file=sys.stderr)
            print("  Regenerate: cargo run -q -p dsl-schema-export | "
                  "python code/tools/dsl-doc-export/emit_markdown.py", file=sys.stderr)
            return 1
        print("DSL vocab reference is in sync.")
        return 0

    block = build_block(rows, args.cards_dir)
    guide_text = args.guide.read_text(encoding="utf-8")
    args.guide.write_text(splice(guide_text, block), encoding="utf-8")
    n = {k: len(v) for k, v in rows.items()}
    print(f"Wrote DSL vocab block: {n['steps']} steps, {n['predicates']} predicates, "
          f"{n['timings']} timings, {n['kinds']} kinds.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
