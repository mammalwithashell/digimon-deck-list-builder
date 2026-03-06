#!/usr/bin/env python3
"""Batch fix: Add self-check to BeforePayCost cost_reduction conditions.

Problem: Scripts with "When you would play THIS CARD" cost reduction effects
use BeforePayCost timing with cost_reduction attribute, but their condition
functions don't verify that the card being played IS the card itself.
Since calculate_play_cost() scans ALL battle_area permanents' effects,
a copy already on the field leaks its cost reduction to every future play.

Fix: Add `if context.get('card_source') is not card: return False` as the
first check in every affected condition function.

Also fixes the _temp_play_cost_reduction process callback leak in BT5-085
and BT17-060 by adding the same self-check to their conditions.
"""

import re
from pathlib import Path

SCRIPTS_DIR = Path(__file__).parent.parent / "digimon_gym" / "engine" / "data" / "scripts"

# All 30 files that need fixing (from audit)
NEEDS_FIX = [
    # Category 1: cost_reduction attribute on BeforePayCost effect (28 files)
    "bt8/bt8_043.py",
    "bt8/bt8_112.py",
    "bt10/bt10_052.py",
    "bt8/bt8_054.py",
    "p/p_056.py",
    "bt13/bt13_080.py",
    "bt13/bt13_083.py",
    "bt13/bt13_086.py",
    "bt16/bt16_065.py",
    "bt16/bt16_100.py",
    "bt20/bt20_036.py",
    "bt20/bt20_043.py",
    "bt20/bt20_057.py",
    "bt21/bt21_020.py",
    "bt21/bt21_030.py",
    "bt21/bt21_093.py",
    "bt23/bt23_015.py",
    "bt23/bt23_034.py",
    "bt23/bt23_036.py",
    "bt23/bt23_044.py",
    "bt23/bt23_057.py",
    "ex6/ex6_039.py",
    "p/p_170.py",
    "p/p_171.py",
    "p/p_172.py",
    "p/p_174.py",
    "p/p_219.py",
    "p/p_222.py",
    "p/p_223.py",
    # Category 1b: _temp_play_cost_reduction in process callback (2 files)
    "bt5/bt5_085.py",
    "bt17/bt17_060.py",
]

SELF_CHECK = "            if context.get('card_source') is not card:\n                return False\n"

# Pattern: find the condition function for a BeforePayCost effect
# It's the first `def conditionN(context: ...)` after `set_timing(EffectTiming.BeforePayCost)`
BEFORE_PAY_COST_PATTERN = re.compile(
    r"(set_timing\(EffectTiming\.BeforePayCost\).*?"  # find BeforePayCost
    r"def\s+(condition\d+)\(context:\s*Dict\[str,\s*Any\]\)\s*->\s*bool:\s*\n)"  # then condition fn
    r"((?:\s*#[^\n]*\n)*)"  # optional comment lines
    r"(\s+)",  # capture indentation of first statement
    re.DOTALL,
)


def fix_file(rel_path: str) -> bool:
    """Add self-check to BeforePayCost condition in a script file."""
    full_path = SCRIPTS_DIR / rel_path
    if not full_path.exists():
        print(f"  SKIP (not found): {rel_path}")
        return False

    content = full_path.read_text(encoding="utf-8")

    # Check if already fixed
    if "card_source') is not card" in content or "card_source') is card" in content:
        print(f"  SKIP (already fixed): {rel_path}")
        return False

    # Find BeforePayCost timing
    if "EffectTiming.BeforePayCost" not in content:
        print(f"  SKIP (no BeforePayCost): {rel_path}")
        return False

    # Strategy: find the condition function that follows BeforePayCost,
    # and insert the self-check as its first line (after any comments).
    match = BEFORE_PAY_COST_PATTERN.search(content)
    if not match:
        print(f"  WARN (pattern not matched): {rel_path}")
        # Fallback: find any conditionN after BeforePayCost manually
        return fix_file_fallback(full_path, rel_path, content)

    prefix = match.group(1)  # everything up to and including the def line
    comments = match.group(3)  # optional comment lines
    indent = match.group(4)  # indentation of next statement

    # Build replacement: insert self-check after comments, before existing code
    insert_pos = match.start() + len(prefix) + len(comments)
    new_content = content[:insert_pos] + SELF_CHECK + content[insert_pos:]

    full_path.write_text(new_content, encoding="utf-8")
    print(f"  FIXED: {rel_path}")
    return True


def fix_file_fallback(full_path: Path, rel_path: str, content: str) -> bool:
    """Fallback: find BeforePayCost block and its condition function manually."""
    lines = content.split("\n")

    # Find line with BeforePayCost
    bpc_line = None
    for i, line in enumerate(lines):
        if "EffectTiming.BeforePayCost" in line:
            bpc_line = i
            break

    if bpc_line is None:
        print(f"  SKIP (fallback: no BeforePayCost line): {rel_path}")
        return False

    # Find the next conditionN function after BeforePayCost
    cond_line = None
    for i in range(bpc_line + 1, min(bpc_line + 40, len(lines))):
        if re.match(r"\s+def condition\d+\(context:", lines[i]):
            cond_line = i
            break

    if cond_line is None:
        print(f"  WARN (fallback: no condition found after BeforePayCost): {rel_path}")
        return False

    # Find the first non-comment, non-blank line after the def line
    insert_after = cond_line
    for i in range(cond_line + 1, min(cond_line + 10, len(lines))):
        stripped = lines[i].strip()
        if stripped and not stripped.startswith("#") and not stripped.startswith('"""') and not stripped.startswith("'''"):
            # Insert self-check before this line
            indent = "            "
            lines.insert(i, f"{indent}if context.get('card_source') is not card:")
            lines.insert(i + 1, f"{indent}    return False")
            new_content = "\n".join(lines)
            full_path.write_text(new_content, encoding="utf-8")
            print(f"  FIXED (fallback): {rel_path}")
            return True

    print(f"  WARN (fallback: couldn't find insertion point): {rel_path}")
    return False


def main():
    fixed = 0
    skipped = 0
    failed = 0

    print(f"Fixing {len(NEEDS_FIX)} files with BeforePayCost cost_reduction leak...")
    print()

    for rel_path in NEEDS_FIX:
        try:
            if fix_file(rel_path):
                fixed += 1
            else:
                skipped += 1
        except Exception as e:
            print(f"  ERROR: {rel_path}: {e}")
            failed += 1

    print()
    print(f"Results: {fixed} fixed, {skipped} skipped, {failed} failed")


if __name__ == "__main__":
    main()
