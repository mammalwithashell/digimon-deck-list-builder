"""Signal collection and merge logic for future-set transpiler audits."""

from __future__ import annotations

from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from tools.transpiler.extractors import parse_cs_file
from tools.transpiler.validation import TRANSPILER_ACTIONS, format_scan_report, scan_api_set


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DCGO_ROOT = PROJECT_ROOT / "DCGO" / "Assets" / "Scripts" / "CardEffect"


def normalize_set_id(set_id: str) -> str:
    text = set_id.strip().lower()
    if not text:
        raise ValueError("set_id is required")
    return text


def resolve_dcgo_set_dir(
    *,
    set_id: str,
    dcgo_set_dir: str | None = None,
    repo_root: Path | None = None,
) -> Path:
    root = repo_root or PROJECT_ROOT
    if dcgo_set_dir:
        raw = Path(dcgo_set_dir)
        return raw if raw.is_absolute() else (root / raw)
    return root / "DCGO" / "Assets" / "Scripts" / "CardEffect" / set_id.upper()


def _sorted_counter(counter: Counter[str]) -> dict[str, int]:
    return dict(sorted(counter.items(), key=lambda item: (-item[1], item[0])))


def collect_api_scan_signals(set_id: str) -> dict[str, Any]:
    raw = scan_api_set(set_id.upper())
    uncovered_keywords = sorted(str(item) for item in raw.get("uncovered_keywords", set()))
    uncovered_actions = sorted(str(item) for item in raw.get("uncovered_actions", set()))
    return {
        "set_id": set_id,
        "card_count": int(raw.get("card_count", 0) or 0),
        "keyword_counts": dict(raw.get("keyword_counts", {}) or {}),
        "action_counts": dict(raw.get("action_counts", {}) or {}),
        "timing_counts": dict(raw.get("timing_counts", {}) or {}),
        "uncovered_keywords": uncovered_keywords,
        "uncovered_actions": uncovered_actions,
        "report_text": format_scan_report(
            {
                **raw,
                "uncovered_keywords": set(uncovered_keywords),
                "uncovered_actions": set(uncovered_actions),
            }
        ),
    }


def collect_dcgo_parse_signals(
    *,
    set_id: str,
    dcgo_set_dir: Path,
) -> dict[str, Any]:
    action_counts: Counter[str] = Counter()
    timing_counts: Counter[str] = Counter()
    factory_counts: Counter[str] = Counter()
    descriptive_tag_counts: Counter[str] = Counter()
    card_examples_by_action: dict[str, set[str]] = defaultdict(set)
    file_count = 0
    effect_count = 0
    no_action_effect_count = 0
    parse_errors: list[str] = []

    if not dcgo_set_dir.exists():
        return {
            "set_id": set_id,
            "source_dir": str(dcgo_set_dir),
            "file_count": 0,
            "effect_count": 0,
            "no_action_effect_count": 0,
            "action_counts": {},
            "timing_counts": {},
            "factory_method_counts": {},
            "descriptive_tag_counts": {},
            "unmapped_action_counts": {},
            "action_examples": {},
            "parse_errors": [f"DCGO set directory not found: {dcgo_set_dir}"],
        }

    for cs_file in sorted(dcgo_set_dir.rglob("*.cs")):
        file_count += 1
        try:
            class_name, effects = parse_cs_file(str(cs_file))
        except Exception as exc:  # pragma: no cover - defensive only
            parse_errors.append(f"{cs_file.name}: {exc}")
            continue

        card_id = class_name.replace("_", "-").upper()
        for effect in effects:
            effect_count += 1
            timing = str(getattr(effect, "timing", "") or "").strip()
            if timing:
                timing_counts[timing] += 1

            factory_method = str(getattr(effect, "factory_method", "") or "").strip()
            if factory_method:
                factory_counts[factory_method] += 1

            descriptive_tag = str(getattr(effect, "descriptive_tag", "") or "").strip()
            if descriptive_tag:
                descriptive_tag_counts[descriptive_tag] += 1

            actions = list(getattr(effect, "actions", []) or [])
            if not actions:
                no_action_effect_count += 1
                continue

            for action in actions:
                action_name = str(action).strip()
                if not action_name:
                    continue
                action_counts[action_name] += 1
                if len(card_examples_by_action[action_name]) < 5:
                    card_examples_by_action[action_name].add(card_id)

    unmapped_action_counts = {
        name: count
        for name, count in action_counts.items()
        if name not in TRANSPILER_ACTIONS
    }

    return {
        "set_id": set_id,
        "source_dir": str(dcgo_set_dir),
        "file_count": file_count,
        "effect_count": effect_count,
        "no_action_effect_count": no_action_effect_count,
        "action_counts": _sorted_counter(action_counts),
        "timing_counts": _sorted_counter(timing_counts),
        "factory_method_counts": _sorted_counter(factory_counts),
        "descriptive_tag_counts": _sorted_counter(descriptive_tag_counts),
        "unmapped_action_counts": _sorted_counter(Counter(unmapped_action_counts)),
        "action_examples": {
            key: sorted(values)
            for key, values in sorted(card_examples_by_action.items(), key=lambda item: item[0])
        },
        "parse_errors": sorted(parse_errors),
    }


def merge_transpiler_signals(
    *,
    set_id: str,
    api_scan: dict[str, Any],
    dcgo_parse: dict[str, Any],
    max_gap_items: int = 40,
) -> dict[str, Any]:
    keyword_counts = dict(api_scan.get("keyword_counts", {}) or {})
    action_counts = dict(api_scan.get("action_counts", {}) or {})
    api_uncovered_keywords = list(api_scan.get("uncovered_keywords", []) or [])
    api_uncovered_actions = list(api_scan.get("uncovered_actions", []) or [])
    dcgo_unmapped_action_counts = dict(dcgo_parse.get("unmapped_action_counts", {}) or {})
    dcgo_no_action_effect_count = int(dcgo_parse.get("no_action_effect_count", 0) or 0)
    action_examples = dict(dcgo_parse.get("action_examples", {}) or {})

    gaps: list[dict[str, Any]] = []

    for keyword in sorted(api_uncovered_keywords):
        count = int(keyword_counts.get(keyword, 0) or 0)
        severity = "high" if count >= 8 else ("medium" if count >= 3 else "low")
        gaps.append(
            {
                "gap_id": f"api_keyword:{keyword.lower().replace(' ', '_')}",
                "kind": "keyword",
                "label": keyword,
                "api_count": count,
                "dcgo_count": 0,
                "severity": severity,
                "evidence": [f"API keyword appears in {count} card(s)."],
            }
        )

    for action in sorted(api_uncovered_actions):
        count = int(action_counts.get(action, 0) or 0)
        severity = "high" if count >= 8 else ("medium" if count >= 3 else "low")
        evidence = [f"API action pattern appears in {count} card(s)."]
        if action in action_examples:
            evidence.append(
                "Example card(s): " + ", ".join(action_examples[action][:3])
            )
        gaps.append(
            {
                "gap_id": f"api_action:{action}",
                "kind": "action",
                "label": action,
                "api_count": count,
                "dcgo_count": int(dcgo_unmapped_action_counts.get(action, 0) or 0),
                "severity": severity,
                "evidence": evidence,
            }
        )

    for action, count in sorted(
        dcgo_unmapped_action_counts.items(),
        key=lambda item: (-int(item[1]), item[0]),
    ):
        api_count = int(action_counts.get(action, 0) or 0)
        severity = "high" if count >= 10 else ("medium" if count >= 4 else "low")
        evidence = [f"DCGO parse found unsupported action in {count} effect(s)."]
        if action in action_examples:
            evidence.append(
                "Example card(s): " + ", ".join(action_examples[action][:3])
            )
        gaps.append(
            {
                "gap_id": f"dcgo_action:{action}",
                "kind": "action",
                "label": action,
                "api_count": api_count,
                "dcgo_count": int(count or 0),
                "severity": severity,
                "evidence": evidence,
            }
        )

    if dcgo_no_action_effect_count > 0:
        gaps.append(
            {
                "gap_id": "dcgo_no_action_effects",
                "kind": "structural",
                "label": "no_action_effects",
                "api_count": 0,
                "dcgo_count": dcgo_no_action_effect_count,
                "severity": "medium" if dcgo_no_action_effect_count < 10 else "high",
                "evidence": [
                    f"{dcgo_no_action_effect_count} parsed DCGO effect(s) produced no actions."
                ],
            }
        )

    # Stable deterministic ordering for testability.
    severity_rank = {"critical": 4, "high": 3, "medium": 2, "low": 1}
    gaps.sort(
        key=lambda item: (
            -severity_rank.get(str(item.get("severity", "low")), 1),
            -int(item.get("api_count", 0) or 0),
            -int(item.get("dcgo_count", 0) or 0),
            str(item.get("gap_id", "")),
        )
    )
    limited_gaps = gaps[: max(1, max_gap_items)]

    validation_metrics = {
        "api_uncovered_keyword_count": len(api_uncovered_keywords),
        "api_uncovered_action_count": len(api_uncovered_actions),
        "dcgo_unmapped_action_count": len(dcgo_unmapped_action_counts),
        "dcgo_no_action_effect_count": dcgo_no_action_effect_count,
        "merged_gap_count": len(limited_gaps),
    }

    return {
        "set_id": set_id,
        "api_scan": api_scan,
        "dcgo_parse": dcgo_parse,
        "merged_gaps": limited_gaps,
        "validation_metrics": validation_metrics,
    }


def build_transpiler_audit_inputs(
    *,
    set_id: str,
    dcgo_set_dir: str | None = None,
    max_gap_items: int = 40,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    normalized_set_id = normalize_set_id(set_id)
    resolved_dcgo_dir = resolve_dcgo_set_dir(
        set_id=normalized_set_id,
        dcgo_set_dir=dcgo_set_dir,
        repo_root=repo_root,
    )
    api_scan = collect_api_scan_signals(normalized_set_id)
    dcgo_parse = collect_dcgo_parse_signals(
        set_id=normalized_set_id,
        dcgo_set_dir=resolved_dcgo_dir,
    )
    return merge_transpiler_signals(
        set_id=normalized_set_id,
        api_scan=api_scan,
        dcgo_parse=dcgo_parse,
        max_gap_items=max_gap_items,
    )


def format_gap_summary(merged_gaps: list[dict[str, Any]]) -> str:
    if not merged_gaps:
        return "No merged gaps."
    lines: list[str] = []
    for gap in merged_gaps:
        label = str(gap.get("label", "unknown"))
        kind = str(gap.get("kind", "unknown"))
        severity = str(gap.get("severity", "low"))
        api_count = int(gap.get("api_count", 0) or 0)
        dcgo_count = int(gap.get("dcgo_count", 0) or 0)
        lines.append(
            f"- [{severity}] {kind}::{label} (api={api_count}, dcgo={dcgo_count})"
        )
    return "\n".join(lines)
