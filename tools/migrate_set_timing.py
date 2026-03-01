#!/usr/bin/env python3
"""
Migration script: adds set_timing() calls and EffectTiming imports to frozen card scripts.

Walks all .py files in digimon_gym/engine/data/scripts/{set_id}/ and:
1. Adds `from ....data.enums import EffectTiming` import if missing.
2. For each `# Timing: EffectTiming.XXX` comment, finds the next `effectN = ICardEffect()`
   line and inserts `effectN.set_timing(EffectTiming.XXX)` right after it.
3. Skips EffectTiming.None and EffectTiming.NoTiming (no timing to set).
4. Skips effects that already have a set_timing() call.

Usage:
    python tools/migrate_set_timing.py
    python tools/migrate_set_timing.py --dry-run
"""
from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

# Repo root is one level up from tools/
REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = REPO_ROOT / "digimon_gym" / "engine" / "data" / "scripts"

# Regex patterns
TIMING_COMMENT_RE = re.compile(r"^(\s*)# Timing: EffectTiming\.(\w+)\s*$")
EFFECT_INIT_RE = re.compile(r"^(\s*)(effect\d+)\s*=\s*ICardEffect\(\)\s*$")
SET_TIMING_RE = re.compile(r"^\s*effect\d+\.set_timing\(")
IMPORT_ICARDEFFECT_RE = re.compile(
    r"^from\s+\.{4}interfaces\.card_effect\s+import\s+ICardEffect\s*$"
)
IMPORT_EFFECTTIMING_RE = re.compile(
    r"^from\s+\.{4}data\.enums\s+import\s+EffectTiming"
)

# Timing values to skip
SKIP_TIMINGS = {"None", "NoTiming"}


def process_file(filepath: Path, dry_run: bool = False) -> int:
    """Process a single script file. Returns number of set_timing() calls added."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    lines = content.splitlines(keepends=True)
    added_count = 0
    modified = False

    # --- Step 1: Ensure EffectTiming import exists ---
    has_effecttiming_import = any(IMPORT_EFFECTTIMING_RE.match(line) for line in lines)
    if not has_effecttiming_import:
        # Find the ICardEffect import line and add EffectTiming import after it
        new_lines = []
        import_added = False
        for line in lines:
            new_lines.append(line)
            if not import_added and IMPORT_ICARDEFFECT_RE.match(line.strip()):
                # Check if the line content (stripped) matches; use original indentation
                if IMPORT_ICARDEFFECT_RE.match(line.strip()):
                    new_lines.append("from ....data.enums import EffectTiming\n")
                    import_added = True
                    modified = True
        if import_added:
            lines = new_lines

    # --- Step 2: Find timing comments and insert set_timing() calls ---
    # We need to process the lines tracking pending timing values
    new_lines = []
    pending_timing = None  # The timing value from a # Timing: comment
    i = 0
    while i < len(lines):
        line = lines[i]

        # Check for timing comment
        timing_match = TIMING_COMMENT_RE.match(line)
        if timing_match:
            timing_value = timing_match.group(2)
            if timing_value not in SKIP_TIMINGS:
                pending_timing = timing_value
            else:
                pending_timing = None
            new_lines.append(line)
            i += 1
            continue

        # Check for effectN = ICardEffect() when we have a pending timing
        if pending_timing is not None:
            effect_match = EFFECT_INIT_RE.match(line)
            if effect_match:
                indent = effect_match.group(1)
                effect_var = effect_match.group(2)
                new_lines.append(line)
                i += 1

                # Check if the next few lines already have set_timing for this effect
                already_has_timing = False
                lookahead_limit = min(i + 5, len(lines))
                for j in range(i, lookahead_limit):
                    if SET_TIMING_RE.match(lines[j]) and effect_var in lines[j]:
                        already_has_timing = True
                        break

                if not already_has_timing:
                    # Insert set_timing() call right after effectN = ICardEffect()
                    timing_line = f"{indent}{effect_var}.set_timing(EffectTiming.{pending_timing})\n"
                    new_lines.append(timing_line)
                    added_count += 1
                    modified = True

                pending_timing = None
                continue

        new_lines.append(line)
        i += 1

    # --- Step 3: Write back if modified ---
    if modified and not dry_run:
        with open(filepath, "w", encoding="utf-8", newline="") as f:
            f.writelines(new_lines)

    return added_count


def main():
    parser = argparse.ArgumentParser(
        description="Add set_timing() calls to frozen card scripts"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Don't write changes, just report what would be done",
    )
    args = parser.parse_args()

    if not SCRIPTS_DIR.is_dir():
        print(f"ERROR: Scripts directory not found: {SCRIPTS_DIR}", file=sys.stderr)
        sys.exit(1)

    files_processed = 0
    files_modified = 0
    total_timings_added = 0
    skipped_none_count = 0
    per_set_stats: dict[str, dict[str, int]] = {}

    # Walk set directories (one level deep)
    set_dirs = sorted(
        d for d in SCRIPTS_DIR.iterdir()
        if d.is_dir() and d.name != "__pycache__" and d.name != "generated"
    )

    for set_dir in set_dirs:
        set_id = set_dir.name
        set_files_modified = 0
        set_timings_added = 0

        py_files = sorted(set_dir.glob("*.py"))
        for py_file in py_files:
            if py_file.name == "__init__.py":
                continue

            files_processed += 1

            # Count None/NoTiming occurrences for stats
            with open(py_file, "r", encoding="utf-8") as f:
                file_lines = f.readlines()
            for file_line in file_lines:
                m = TIMING_COMMENT_RE.match(file_line)
                if m and m.group(2) in SKIP_TIMINGS:
                    skipped_none_count += 1

            count = process_file(py_file, dry_run=args.dry_run)
            if count > 0:
                files_modified += 1
                set_files_modified += 1
                set_timings_added += count
                total_timings_added += count

        if set_timings_added > 0 or set_files_modified > 0:
            per_set_stats[set_id] = {
                "files_modified": set_files_modified,
                "timings_added": set_timings_added,
            }

    # --- Print statistics ---
    mode = "[DRY RUN] " if args.dry_run else ""
    print(f"\n{'=' * 60}")
    print(f"{mode}Migration complete: set_timing() insertion")
    print(f"{'=' * 60}")
    print(f"  Files processed:        {files_processed}")
    print(f"  Files modified:         {files_modified}")
    print(f"  set_timing() calls added: {total_timings_added}")
    print(f"  NoTiming/None skipped:  {skipped_none_count}")
    print()

    if per_set_stats:
        print("Per-set breakdown:")
        print(f"  {'Set':<10} {'Files Modified':>15} {'Timings Added':>15}")
        print(f"  {'-'*10} {'-'*15} {'-'*15}")
        for set_id, stats in sorted(per_set_stats.items()):
            print(
                f"  {set_id:<10} {stats['files_modified']:>15} {stats['timings_added']:>15}"
            )
    print()


if __name__ == "__main__":
    main()
