#!/usr/bin/env python3
"""Audit transpiled card sets for faithfulness to official card text.

Uses the validation infrastructure from tools/transpiler/validation.py to check
generated scripts against card metadata (from cards.json or digimoncard.io API).

Usage:
    python tools/audit_transpiled_sets.py --sets BT22,EX5,EX6
    python tools/audit_transpiled_sets.py --sets ALL --json
    python tools/audit_transpiled_sets.py --sets BT22 --use-api --threshold 0.8
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Dict, List, Optional

# Resolve project root so imports work from any cwd
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from tools.transpiler.validation import (
    ValidationResult,
    _build_reverse_patterns,
    _build_validation_patterns,
    fetch_api_set,
    parse_api_effect_text,
    validate_card,
)

# Paths
CARDS_JSON = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "cards.json"
GENERATED_ROOT = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts" / "generated"

ALL_SETS = ["BT8", "BT10", "BT11", "BT15", "BT16", "BT19", "BT22", "EX5", "EX6", "EX8", "ST12"]

# Scoring weights (same as tools/transpiler/scoring.py)
W_EFFECTS = 0.40
W_ACTIONS = 0.30
W_FORWARD = 0.20
W_COROUTINE = 0.10
DEFAULT_THRESHOLD = 0.7


# ─── Data Classes ────────────────────────────────────────────────────

@dataclass
class CardAuditResult:
    card_id: str
    card_name: str = ""
    score: float = 1.0
    effects_ratio: float = 1.0
    actions_ratio: float = 1.0
    forward_match_ratio: float = 1.0
    coroutine_score: float = 1.0
    forward_issues: List[str] = field(default_factory=list)
    reverse_issues: List[str] = field(default_factory=list)
    timing_issues: List[str] = field(default_factory=list)
    structural_warnings: List[str] = field(default_factory=list)
    below_threshold: bool = False
    has_script: bool = True
    api_effect_blocks: int = 0
    script_effect_count: int = 0


@dataclass
class SetAuditResult:
    set_id: str
    total_cards: int = 0
    cards_audited: int = 0
    cards_with_issues: int = 0
    cards_below_threshold: int = 0
    cards_missing_script: int = 0
    average_score: float = 0.0
    cards: Dict[str, CardAuditResult] = field(default_factory=dict)


# ─── Discovery ───────────────────────────────────────────────────────

def _module_to_card_id(module_name: str) -> str:
    """Convert module filename to card ID: bt22_001 -> BT22-001."""
    parts = module_name.split("_")
    if len(parts) >= 2:
        prefix = parts[0].upper()
        number = parts[-1]
        return f"{prefix}-{number}"
    return module_name.upper()


def discover_set_scripts(set_id: str) -> Dict[str, Path]:
    """Find all generated scripts for a set. Returns {card_id: path}."""
    set_dir = GENERATED_ROOT / set_id.lower()
    if not set_dir.exists():
        return {}
    scripts = {}
    for py_file in sorted(set_dir.glob("*.py")):
        if py_file.name == "__init__.py":
            continue
        card_id = _module_to_card_id(py_file.stem)
        scripts[card_id] = py_file
    return scripts


# ─── Metadata Loading ────────────────────────────────────────────────

def load_cards_json() -> Dict[str, dict]:
    """Load all cards from cards.json, indexed by card_id."""
    with open(CARDS_JSON) as f:
        data = json.load(f)
    if isinstance(data, dict):
        return data
    # If it's a list, index by card_id
    return {c["card_id"]: c for c in data if "card_id" in c}


def load_set_metadata(set_id: str, card_db: Dict[str, dict], use_api: bool = False) -> Dict[str, dict]:
    """Get card metadata for a set, optionally fetching from API."""
    prefix = set_id.upper() + "-"
    cards = {cid: meta for cid, meta in card_db.items() if cid.startswith(prefix)}

    if use_api and not cards:
        api_cards = fetch_api_set(set_id)
        for ac in api_cards:
            cid = ac.get("cardnumber", "")
            if cid:
                cards[cid] = {
                    "card_id": cid,
                    "card_name_eng": ac.get("name", ""),
                    "effect_description_eng": _strip_html(ac.get("main_effect", "") or ""),
                    "inherited_effect_description_eng": _strip_html(ac.get("source_effect", "") or ""),
                    "security_effect_description_eng": "",
                }
    return cards


def _strip_html(text: str) -> str:
    """Strip HTML tags, converting <br> to newlines."""
    text = re.sub(r"<br\s*/?>", "\n", text, flags=re.I)
    text = re.sub(r"<[^>]+>", "", text)
    return text.strip()


# ─── Scoring ─────────────────────────────────────────────────────────

RE_EFFECT_ENTRY = re.compile(r'"timing"\s*:', re.I)
RE_CALLBACK = re.compile(r"def\s+activate_\w+|set_on_process_callback")
RE_ACTION_CALL = re.compile(
    r"game\.\w+|\.draw_cards\(|\.add_memory\(|\.delete_permanent\(|"
    r"\.suspend\(|\.unsuspend\(|\.recovery\(|register_modifier\("
)


def standalone_score(
    card_id: str,
    card_meta: dict,
    script_code: str,
    validation_result: ValidationResult,
    threshold: float = DEFAULT_THRESHOLD,
) -> CardAuditResult:
    """Score a card without needing EffectBlock objects."""
    result = CardAuditResult(card_id=card_id)
    result.card_name = card_meta.get("card_name_eng", "")

    # Count expected effects from API text
    parsed_blocks = parse_api_effect_text(card_meta)
    expected = len(parsed_blocks)
    result.api_effect_blocks = expected

    if expected == 0:
        # Vanilla card
        result.score = 1.0
        return result

    # Count script effects
    effect_entries = RE_EFFECT_ENTRY.findall(script_code)
    callbacks = RE_CALLBACK.findall(script_code)
    script_effects = max(len(effect_entries), len(callbacks))
    result.script_effect_count = script_effects

    effects_ratio = min(script_effects / expected, 1.0) if expected > 0 else 1.0

    # Count action calls in script
    action_calls = RE_ACTION_CALL.findall(script_code)
    actions_ratio = min(len(action_calls) / max(script_effects, 1), 1.0) if script_effects > 0 else 0.0

    # Forward match ratio
    forward_issues = len(validation_result.forward_issues)
    forward_match_ratio = max(1.0 - (forward_issues / expected), 0.0)

    # Coroutine coverage: do all effect blocks have callbacks?
    has_unmapped = script_effects < expected
    coroutine_score = 0.0 if has_unmapped else 1.0

    score = (
        W_EFFECTS * effects_ratio
        + W_ACTIONS * actions_ratio
        + W_FORWARD * forward_match_ratio
        + W_COROUTINE * coroutine_score
    )
    score = round(max(0.0, min(1.0, score)), 4)

    result.score = score
    result.effects_ratio = round(effects_ratio, 4)
    result.actions_ratio = round(actions_ratio, 4)
    result.forward_match_ratio = round(forward_match_ratio, 4)
    result.coroutine_score = round(coroutine_score, 4)
    result.below_threshold = score < threshold

    result.forward_issues = list(validation_result.forward_issues)
    result.reverse_issues = list(validation_result.reverse_issues)
    result.timing_issues = list(validation_result.timing_issues)
    result.structural_warnings = list(validation_result.structural_warnings)

    return result


# ─── Audit Logic ─────────────────────────────────────────────────────

def audit_set(
    set_id: str,
    card_db: Dict[str, dict],
    threshold: float = DEFAULT_THRESHOLD,
    use_api: bool = False,
) -> SetAuditResult:
    """Audit all cards in a set."""
    result = SetAuditResult(set_id=set_id)
    scripts = discover_set_scripts(set_id)
    metadata = load_set_metadata(set_id, card_db, use_api=use_api)

    # Build patterns once
    forward_patterns = _build_validation_patterns()
    reverse_patterns = _build_reverse_patterns()

    # Union of card IDs from scripts and metadata
    all_card_ids = sorted(set(scripts.keys()) | set(metadata.keys()))
    result.total_cards = len(all_card_ids)

    scores = []

    for card_id in all_card_ids:
        meta = metadata.get(card_id, {})
        script_path = scripts.get(card_id)

        if not script_path or not script_path.exists():
            card_result = CardAuditResult(card_id=card_id, has_script=False, score=0.0)
            card_result.card_name = meta.get("card_name_eng", "")
            result.cards[card_id] = card_result
            result.cards_missing_script += 1
            continue

        script_code = script_path.read_text()
        result.cards_audited += 1

        # Validate
        vr = validate_card(card_id, meta, script_code, forward_patterns, reverse_patterns)

        # Score
        card_result = standalone_score(card_id, meta, script_code, vr, threshold)

        if card_result.below_threshold:
            result.cards_below_threshold += 1

        has_issues = (
            card_result.forward_issues
            or card_result.reverse_issues
            or card_result.timing_issues
            or card_result.structural_warnings
        )
        if has_issues:
            result.cards_with_issues += 1

        scores.append(card_result.score)
        result.cards[card_id] = card_result

    result.average_score = round(sum(scores) / len(scores), 4) if scores else 0.0
    return result


# ─── Reporting ───────────────────────────────────────────────────────

def format_json_report(result: SetAuditResult) -> dict:
    """Convert SetAuditResult to JSON-serializable dict."""
    cards_dict = {}
    for cid, cr in result.cards.items():
        cards_dict[cid] = asdict(cr)

    # Summarize top issues
    fwd_counter = Counter()
    rev_counter = Counter()
    timing_counter = Counter()
    for cr in result.cards.values():
        for issue in cr.forward_issues:
            fwd_counter[issue] += 1
        for issue in cr.reverse_issues:
            rev_counter[issue] += 1
        for issue in cr.timing_issues:
            timing_counter[issue] += 1

    worst = sorted(result.cards.values(), key=lambda c: c.score)[:10]

    return {
        "set_id": result.set_id,
        "total_cards": result.total_cards,
        "cards_audited": result.cards_audited,
        "cards_with_issues": result.cards_with_issues,
        "cards_below_threshold": result.cards_below_threshold,
        "cards_missing_script": result.cards_missing_script,
        "average_score": result.average_score,
        "cards": cards_dict,
        "summary": {
            "top_forward_issues": fwd_counter.most_common(15),
            "top_reverse_issues": rev_counter.most_common(10),
            "top_timing_issues": timing_counter.most_common(10),
            "worst_cards": [
                {"card_id": c.card_id, "name": c.card_name, "score": c.score}
                for c in worst
            ],
        },
    }


def format_markdown_report(result: SetAuditResult) -> str:
    """Render a human-readable markdown audit report."""
    lines = [
        f"# Audit Report: {result.set_id}",
        "",
        f"- **Total cards**: {result.total_cards}",
        f"- **Cards audited**: {result.cards_audited}",
        f"- **Cards with issues**: {result.cards_with_issues}",
        f"- **Cards below threshold**: {result.cards_below_threshold}",
        f"- **Cards missing script**: {result.cards_missing_script}",
        f"- **Average score**: {result.average_score:.4f}",
        "",
    ]

    # Worst cards
    worst = sorted(result.cards.values(), key=lambda c: c.score)[:20]
    lines.append("## Worst-Scoring Cards")
    lines.append("")
    lines.append("| Card ID | Name | Score | Forward | Reverse | Timing |")
    lines.append("|---------|------|-------|---------|---------|--------|")
    for cr in worst:
        if cr.score >= DEFAULT_THRESHOLD and not cr.forward_issues:
            continue
        lines.append(
            f"| {cr.card_id} | {cr.card_name} | {cr.score:.2f} | "
            f"{len(cr.forward_issues)} | {len(cr.reverse_issues)} | {len(cr.timing_issues)} |"
        )
    lines.append("")

    # Top forward issues
    fwd_counter = Counter()
    for cr in result.cards.values():
        for issue in cr.forward_issues:
            fwd_counter[issue] += 1

    if fwd_counter:
        lines.append("## Top Forward Issues (API mentions X, script missing)")
        lines.append("")
        for issue, count in fwd_counter.most_common(15):
            lines.append(f"- **{issue}**: {count} cards")
        lines.append("")

    # Top reverse issues
    rev_counter = Counter()
    for cr in result.cards.values():
        for issue in cr.reverse_issues:
            rev_counter[issue] += 1

    if rev_counter:
        lines.append("## Top Reverse Issues (script claims X, API doesn't mention)")
        lines.append("")
        for issue, count in rev_counter.most_common(10):
            lines.append(f"- **{issue}**: {count} cards")
        lines.append("")

    # Top timing issues
    timing_counter = Counter()
    for cr in result.cards.values():
        for issue in cr.timing_issues:
            timing_counter[issue] += 1

    if timing_counter:
        lines.append("## Timing Issues")
        lines.append("")
        for issue, count in timing_counter.most_common(10):
            lines.append(f"- **{issue}**: {count} cards")
        lines.append("")

    return "\n".join(lines)


# ─── CLI ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Audit transpiled card sets")
    parser.add_argument("--sets", required=True, help="Comma-separated set IDs or ALL")
    parser.add_argument("--json", action="store_true", help="Output JSON reports")
    parser.add_argument("--use-api", action="store_true", help="Fetch from digimoncard.io API")
    parser.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD, help="Score threshold")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory for reports")
    args = parser.parse_args()

    set_ids = ALL_SETS if args.sets.upper() == "ALL" else [s.strip() for s in args.sets.split(",")]

    output_dir = Path(args.output_dir) if args.output_dir else PROJECT_ROOT / "tools" / "audit_reports"
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load cards.json once
    print(f"Loading cards.json...", file=sys.stderr)
    card_db = load_cards_json()
    print(f"Loaded {len(card_db)} cards", file=sys.stderr)

    all_results = []

    for set_id in set_ids:
        print(f"\nAuditing {set_id}...", file=sys.stderr)
        result = audit_set(set_id, card_db, threshold=args.threshold, use_api=args.use_api)
        all_results.append(result)

        print(
            f"  {result.cards_audited}/{result.total_cards} audited, "
            f"{result.cards_with_issues} with issues, "
            f"{result.cards_below_threshold} below threshold, "
            f"avg score: {result.average_score:.4f}",
            file=sys.stderr,
        )

        if args.json:
            report = format_json_report(result)
            out_path = output_dir / f"{set_id}_audit.json"
            with open(out_path, "w") as f:
                json.dump(report, f, indent=2)
            print(f"  JSON report: {out_path}", file=sys.stderr)

        md_report = format_markdown_report(result)
        md_path = output_dir / f"{set_id}_AUDIT_REPORT.md"
        with open(md_path, "w") as f:
            f.write(md_report)
        print(f"  Markdown report: {md_path}", file=sys.stderr)

    # Print aggregate summary to stdout
    print("\n" + "=" * 60)
    print("AGGREGATE AUDIT SUMMARY")
    print("=" * 60)
    total_audited = sum(r.cards_audited for r in all_results)
    total_issues = sum(r.cards_with_issues for r in all_results)
    total_below = sum(r.cards_below_threshold for r in all_results)
    total_missing = sum(r.cards_missing_script for r in all_results)

    all_scores = []
    for r in all_results:
        for cr in r.cards.values():
            if cr.has_script:
                all_scores.append(cr.score)

    overall_avg = sum(all_scores) / len(all_scores) if all_scores else 0.0

    print(f"Sets audited: {len(all_results)}")
    print(f"Total cards audited: {total_audited}")
    print(f"Total cards with issues: {total_issues}")
    print(f"Total cards below threshold ({args.threshold}): {total_below}")
    print(f"Total cards missing script: {total_missing}")
    print(f"Overall average score: {overall_avg:.4f}")
    print()
    print("Per-set breakdown:")
    print(f"{'Set':<8} {'Audited':>8} {'Issues':>8} {'Below':>8} {'Avg Score':>10}")
    print("-" * 44)
    for r in sorted(all_results, key=lambda x: x.average_score):
        print(f"{r.set_id:<8} {r.cards_audited:>8} {r.cards_with_issues:>8} {r.cards_below_threshold:>8} {r.average_score:>10.4f}")


if __name__ == "__main__":
    main()
