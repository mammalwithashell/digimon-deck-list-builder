"""CLI entry point for the DCGO transpiler."""
import os
import sys
import json
import time
from typing import Dict

from .extractors import parse_cs_file
from .generators import generate_python_script
from .validation import (
    _build_validation_patterns,
    _build_reverse_patterns,
    validate_card,
    scan_api_set,
    scan_cards_for_patterns,
    format_scan_report,
    read_priority_sets,
    ValidationResult,
)


def _run_scan_api(scan_arg: str, card_db: Dict[str, dict]):
    """Run --scan-api mode: analyze pattern coverage for sets."""
    if scan_arg.upper() == "ALL":
        priority_sets = read_priority_sets()
        if not priority_sets:
            print("No priority sets found in priority_sets.txt")
            return

        print(f"Scanning {len(priority_sets)} priority sets...")
        all_cards = []
        for set_id in priority_sets:
            # Use cards.json data if available
            set_cards = [c for c in card_db.values()
                         if c["card_id"].startswith(set_id + "-")]
            if set_cards:
                all_cards.extend(set_cards)
            else:
                print(f"  {set_id}: not in cards.json, skipping (run --bulk ingest first)")

        if all_cards:
            results = scan_cards_for_patterns(all_cards)
            results["set_id"] = "ALL"
            results["card_count"] = len(all_cards)
            print(format_scan_report(results))
    else:
        set_id = scan_arg.upper()
        # Use cards.json data if available
        set_cards = [c for c in card_db.values()
                     if c["card_id"].startswith(set_id + "-")]
        if set_cards:
            results = scan_api_set(set_id, cards=set_cards)
        else:
            print(f"  {set_id} not in cards.json, fetching from API...")
            try:
                results = scan_api_set(set_id)
            except Exception as e:
                print(f"  Failed to fetch {set_id}: {e}")
                return
        print(format_scan_report(results))


def main():
    # Check for --scan-api mode first
    scan_api_arg = None
    for i, arg in enumerate(sys.argv[1:], 1):
        if arg == "--scan-api" and i < len(sys.argv) - 1:
            scan_api_arg = sys.argv[i + 1]
            break

    if scan_api_arg is not None:
        # Load cards.json
        project_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
        cards_json_path = os.path.join(project_root, "digimon_gym", "engine", "data", "cards.json")
        card_db: Dict[str, dict] = {}
        if os.path.exists(cards_json_path):
            with open(cards_json_path, encoding="utf-8") as f:
                data = json.load(f)
            if isinstance(data, dict):
                card_db = data
            else:
                for c in data:
                    card_db[c["card_id"]] = c
        print(f"Card database: {len(card_db)} cards")
        _run_scan_api(scan_api_arg, card_db)
        return

    if len(sys.argv) < 2:
        print("Usage:")
        print("  python transpile_dcgo.py <DCGO_DIR> [OUTPUT_DIR] [--validate]")
        print("  python transpile_dcgo.py --scan-api <SET_ID|ALL>")
        print()
        print("  e.g. python transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT24")
        print("  e.g. python transpile_dcgo.py --scan-api BT22")
        print("  e.g. python transpile_dcgo.py --scan-api ALL")
        sys.exit(1)

    validate = "--validate" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]

    dcgo_dir = args[0]
    # Infer set_id from directory name (e.g. /tmp/.../BT24 -> bt24)
    set_id = os.path.basename(dcgo_dir.rstrip("/")).lower()

    # Project root is two levels up from this file (tools/transpiler/cli.py -> project root)
    project_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

    output_dir = args[1] if len(args) > 1 else os.path.join(
        project_root, "digimon_gym", "engine", "data", "scripts", "generated", set_id
    )

    output_dir = os.path.abspath(output_dir)
    os.makedirs(output_dir, exist_ok=True)

    # Load cards.json for card metadata (name, level, kind)
    cards_json_path = os.path.join(
        project_root, "digimon_gym", "engine", "data", "cards.json"
    )
    card_db: Dict[str, dict] = {}
    all_card_names: set = set()
    all_traits: set = set()
    if os.path.exists(cards_json_path):
        with open(cards_json_path, encoding="utf-8") as f:
            data = json.load(f)
        entries = data.values() if isinstance(data, dict) else data
        for c in entries:
            card_db[c.get("card_id", "")] = c
            if c.get("card_name_eng"):
                all_card_names.add(c["card_name_eng"])
            for t in (c.get("type_eng") or []):
                if t:
                    all_traits.add(t)

    if all_card_names:
        print(f"Card database: {len(card_db)} cards, {len(all_card_names)} unique names, {len(all_traits)} unique traits")

    # Find all .cs files
    cs_files = []
    for root, dirs, files in os.walk(dcgo_dir):
        for f in files:
            if f.endswith('.cs') and not f.endswith('.meta'):
                cs_files.append(os.path.join(root, f))

    cs_files.sort()
    print(f"Found {len(cs_files)} C# files in {dcgo_dir}")

    stats = {"total": 0, "with_effects": 0, "factory": 0, "activate": 0, "total_effects": 0}
    report = []

    for cs_path in cs_files:
        class_name, effects = parse_cs_file(cs_path)
        card_id = class_name.replace("_", "-")  # BT14_001 -> BT14-001
        module_name = class_name.lower()  # bt14_001

        stats["total"] += 1
        stats["total_effects"] += len(effects)

        if effects:
            stats["with_effects"] += 1
            for e in effects:
                if e.is_factory:
                    stats["factory"] += 1
                else:
                    stats["activate"] += 1

        py_code = generate_python_script(class_name, card_id, effects, card_db)
        out_path = os.path.join(output_dir, f"{module_name}.py")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(py_code)

        effect_summary = []
        for e in effects:
            if e.is_factory:
                effect_summary.append(f"  [factory] {e.factory_method}")
            else:
                # P4/P5: Distinguish descriptive-tagged from no-action
                descriptive_tags = {"ignore_color_req", "app_fusion_condition",
                                    "link_condition", "also_treated_as", "play_restriction",
                                    "play_token", "force_attack", "change_security_attack",
                                    "disable_effect", "add_temp_effect", "put_to_security"}
                real_actions = [a for a in e.actions if a not in descriptive_tags]
                tag_actions = [a for a in e.actions if a in descriptive_tags]
                if real_actions:
                    actions = ", ".join(e.actions)
                elif tag_actions:
                    actions = ", ".join(tag_actions) + " (descriptive-tagged)"
                else:
                    actions = "no-action"
                inherited = " (inherited)" if e.is_inherited else ""
                once = " (1/turn)" if e.max_count_per_turn > 0 else ""
                effect_summary.append(f"  [{e.timing}] {actions}{inherited}{once}")

        report.append(f"{class_name}: {len(effects)} effects")
        for s in effect_summary:
            report.append(s)

    # Write __init__.py
    init_path = os.path.join(output_dir, "__init__.py")
    with open(init_path, "w", encoding="utf-8") as f:
        f.write("")

    print(f"\nTranspilation complete:")
    print(f"  Scripts processed: {stats['total']}")
    print(f"  Scripts with effects: {stats['with_effects']}")
    print(f"  Total effects extracted: {stats['total_effects']}")
    print(f"    Factory effects: {stats['factory']}")
    print(f"    Activate effects: {stats['activate']}")
    print(f"  Output directory: {output_dir}")

    # Write report
    report_path = os.path.join(output_dir, "TRANSPILE_REPORT.md")
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(f"# {set_id.upper()} Transpilation Report\n\n")
        f.write(f"Generated from DCGO C# card scripts.\n\n")
        f.write(f"- Total scripts: {stats['total']}\n")
        f.write(f"- Scripts with effects: {stats['with_effects']}\n")
        f.write(f"- Total effects: {stats['total_effects']}\n")
        f.write(f"- Factory effects: {stats['factory']}\n")
        f.write(f"- Activate effects: {stats['activate']}\n\n")
        f.write("## Per-Card Breakdown\n\n```\n")
        f.write("\n".join(report))
        f.write("\n```\n")

    print(f"  Report: {report_path}")

    # ── Cross-Validation against digimoncard.io effect text ──
    if validate and card_db:
        print("\n--- Cross-Validation ---")
        forward_patterns = _build_validation_patterns()
        reverse_patterns = _build_reverse_patterns()

        validated_count = 0
        forward_issues = []
        reverse_issues = []
        timing_issues = []
        structural_warnings = []

        for cs_path in cs_files:
            class_name, _ = parse_cs_file(cs_path)
            card_id = class_name.replace("_", "-")
            module_name = class_name.lower()
            card_meta = card_db.get(card_id, {})

            if not card_meta:
                continue

            # Read the generated Python script
            py_path = os.path.join(output_dir, f"{module_name}.py")
            if not os.path.exists(py_path):
                continue

            with open(py_path, "r", encoding="utf-8") as f:
                py_code = f.read()

            validated_count += 1

            # Run full validation
            result = validate_card(
                card_id, card_meta, py_code,
                forward_patterns=forward_patterns,
                reverse_patterns=reverse_patterns,
            )

            for issue in result.forward_issues:
                forward_issues.append(f"{card_id}: API has '{issue}' but script missing implementation")
            for issue in result.reverse_issues:
                reverse_issues.append(f"{card_id}: script has '{issue}' but API text doesn't mention it")
            for issue in result.timing_issues:
                timing_issues.append(f"{card_id}: {issue}")
            for warning in result.structural_warnings:
                structural_warnings.append(f"{card_id}: {warning}")

        total_issues = len(forward_issues) + len(reverse_issues) + len(timing_issues)
        print(f"  Cards validated: {validated_count}")
        print(f"  Forward mismatches: {len(forward_issues)}")
        print(f"  Reverse mismatches: {len(reverse_issues)}")
        print(f"  Timing mismatches: {len(timing_issues)}")
        print(f"  Structural warnings: {len(structural_warnings)}")

        # Write categorized report
        with open(report_path, "a", encoding="utf-8") as f:
            f.write(f"\n\n## Cross-Validation Results\n\n")
            f.write(f"Checked {validated_count} cards against digimoncard.io effect text.\n\n")

            if forward_issues:
                f.write("### Forward Mismatches (API mentions X, script missing)\n\n```\n")
                for issue in sorted(forward_issues):
                    f.write(f"{issue}\n")
                f.write("```\n\n")

            if reverse_issues:
                f.write("### Reverse Mismatches (Script claims X, API doesn't mention)\n\n```\n")
                for issue in sorted(reverse_issues):
                    f.write(f"{issue}\n")
                f.write("```\n\n")

            if timing_issues:
                f.write("### Timing Mismatches\n\n```\n")
                for issue in sorted(timing_issues):
                    f.write(f"{issue}\n")
                f.write("```\n\n")

            if structural_warnings:
                f.write("### Structural Warnings\n\n```\n")
                for warning in sorted(structural_warnings):
                    f.write(f"{warning}\n")
                f.write("```\n\n")

            if not (forward_issues or reverse_issues or timing_issues or structural_warnings):
                f.write("All cards passed cross-validation.\n")

        print(f"  Results appended to {report_path}")

        # Print first few issues to console
        all_issues = forward_issues + reverse_issues + timing_issues
        for issue in all_issues[:10]:
            print(f"    {issue}")
        if len(all_issues) > 10:
            print(f"    ... and {len(all_issues) - 10} more")

    print(f"\n  Report: {report_path}")
