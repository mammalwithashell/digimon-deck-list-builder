#!/usr/bin/env python3
"""Fix alt_digi scripts that set _alt_digi_cost without constraint attributes.

Reads cards.json to find cards with [Digivolve] entries in xros_req,
parses the FIRST [Digivolve] line to extract constraint info (name, level,
trait), then patches the corresponding script file to add the missing
constraint attributes after the _alt_digi_cost line.

Only modifies scripts where constraints are actually missing.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, List, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]
CARDS_JSON = REPO_ROOT / "data" / "cards.json"
SCRIPTS_DIR = REPO_ROOT / "digimon_gym" / "engine" / "data" / "scripts"


@dataclass
class AltDigiConstraint:
    """Parsed constraint from a [Digivolve] xros_req line."""
    level: Optional[int] = None
    name: Optional[str] = None
    trait: Optional[str] = None
    color: Optional[str] = None  # not used for now; color constraints are rare
    cost: Optional[int] = None
    raw_line: str = ""


def parse_digivolve_line(line: str) -> Optional[AltDigiConstraint]:
    """Parse a single [Digivolve] line and extract constraints.

    Returns None if the line is not a [Digivolve] line or cannot be parsed.

    Handles these patterns (from most specific to least):
    - [Digivolve] [CardName]: Cost N                          -> name
    - [Digivolve] [CardName] in name: Cost N                  -> name
    - [Digivolve] Lv.N [CardName]: Cost N                     -> level + name
    - [Digivolve] Lv.N w/[Name] in name: Cost M              -> level + name
    - [Digivolve] Lv.N w/[Name] in its name ...: Cost M      -> level + name
    - [Digivolve] Lv.N w/[Trait] trait: Cost M                -> level + trait
    - [Digivolve] Lv.N w/[Trait1]/[Trait2] trait: Cost M      -> level + trait (first)
    - [Digivolve] Lv.N w/[Name] in text: Cost M              -> level (+ text, but we just set level)
    - [Digivolve] [Name1]/[Name2]/...: Cost N                 -> name (first)
    - [Digivolve] Color Lv.N w/[Trait] trait: Cost M          -> level + trait (+ color)
    - [Digivolve] Lv.N w/[Name] in name or w/[Trait]: Cost M -> level + name (first clause)
    - [Digivolve] Lv.N w/[Name] in name and [Trait] trait: C  -> level + name + trait
    - [Digivolve] Lv.N w/[Trait1]/[Trait2]/[Trait3]: Cost N   -> level + trait (first)
    - [Digivolve] [Name]/Lv.N w/[Trait] trait: Cost M         -> name (first alt)
    - [Digivolve] [Name] or Lv.N w/[Trait] trait: Cost M      -> name (first alt)
    """
    line = line.strip()
    if not line.startswith("[Digivolve]"):
        return None

    # Remove the [Digivolve] prefix
    rest = line[len("[Digivolve]"):].strip()

    constraint = AltDigiConstraint(raw_line=line)

    # Extract cost from the end
    cost_match = re.search(r"Cost\s+(\d+)\s*$", rest)
    if cost_match:
        constraint.cost = int(cost_match.group(1))
        rest = rest[:cost_match.start()].rstrip(": ")

    # Skip patterns we can't handle or that have complex conditional requirements:
    # - "While you have [X]" patterns
    # - "[Name] w/[X] digivolution card" patterns
    # - "[Name] w/N or more [Trait] trait cards under" patterns
    # - "play cost N or higher/less" patterns
    if re.search(r"While you have", rest):
        return constraint  # return with just cost
    if re.search(r"digivolution card", rest):
        # e.g. "[Alphamon] w/[Ouryumon] digivolution card"
        # Extract the main name
        name_match = re.match(r"\[([^\]]+)\]", rest)
        if name_match:
            constraint.name = name_match.group(1)
        return constraint
    if re.search(r"\d+\s+(or more\s+)?\[[\w\s]+\]\s+trait\s+cards?\s+under", rest):
        # e.g. "[Takuya Kanbara] w/2 or more [Hybrid] trait cards under"
        name_match = re.match(r"\[([^\]]+)\]", rest)
        if name_match:
            constraint.name = name_match.group(1)
        return constraint
    if re.search(r"play cost \d+", rest):
        # e.g. "Play cost 10 or higher Lv.6 w/[Vortex Warriors] trait"
        # Extract level if present
        lv_match = re.search(r"Lv\.(\d+)", rest)
        if lv_match:
            constraint.level = int(lv_match.group(1))
        # Extract trait if present
        trait_match = re.search(r"w/\[([^\]]+)\]\s+trait", rest)
        if trait_match:
            constraint.trait = trait_match.group(1)
        return constraint

    # --- Pattern: "Lv.N [Name]" (e.g. "Lv.3 [Argomon]") ---
    m = re.match(r"Lv\.(\d+)\s+\[([^\]]+)\]$", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        return constraint

    # --- Pattern: "[Name]/Lv.N w/[Trait] trait" (e.g. "[Ryudamon]/Lv.3 w/[Chronicle] trait") ---
    m = re.match(r"\[([^\]]+)\]/Lv\.\d+\s+w/\[", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: "[Name] or Lv.N w/[Trait] trait" (e.g. "[Dorumon] or Lv.3 w/[SoC] trait") ---
    m = re.match(r"\[([^\]]+)\]\s+or\s+Lv\.\d+\s+w/\[", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: "[Name] or Lv.N w/[Trait] trait" but "[Dinobeemon] or Lv.5 w/[Insectoid] trait" ---
    m = re.match(r"\[([^\]]+)\]\s+or\s+", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: Lv.N w/[Name] in name and [Trait] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+(?:its\s+)?name\s+and\s+\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        constraint.trait = m.group(3)
        return constraint

    # --- Pattern: Lv.N w/[Name] in (its) name or ... / w/o ... ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+(?:its\s+)?name(?:\s+or\s+|\s+w/o\s+)", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Name] in (its) name ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+(?:its\s+)?name", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Name1]/[Name2] in (its) name ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\](?:/\[([^\]]+)\])?\s+in\s+(?:its\s+)?name", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)  # first name
        return constraint

    # --- Pattern: Color Lv.N w/[Trait] trait (e.g. "Black Lv.2 w/[X Antibody] trait") ---
    m = re.match(r"(?:(\w+(?:/\w+)?)\s+)?Lv\.(\d+)\s+w/\[([^\]]+)\]\s+trait$", rest)
    if m:
        if m.group(1):
            constraint.color = m.group(1)
        constraint.level = int(m.group(2))
        constraint.trait = m.group(3)
        return constraint

    # --- Pattern: Lv.N w/[Trait1]/[Trait2] trait (multi-trait) ---
    # Use [^\]]+ instead of .+? to avoid matching across ] boundaries
    m = re.match(r"(?:(\w+(?:/\w+)?)\s+)?Lv\.(\d+)\s+w/(\[[^\]]+\](?:/\[[^\]]+\])*)\s+trait$", rest)
    if m:
        if m.group(1):
            constraint.color = m.group(1)
        constraint.level = int(m.group(2))
        # Extract first trait
        traits_str = m.group(3)
        first_trait_match = re.match(r"\[([^\]]+)\]", traits_str)
        if first_trait_match:
            constraint.trait = first_trait_match.group(1)
        return constraint

    # --- Pattern: Lv.N w/[Trait1]/[Trait2]/[Trait3] (no "trait" suffix) ---
    # Use [^\]]+ instead of .+? to avoid matching across ] boundaries
    m = re.match(r"Lv\.(\d+)\s+w/(\[[^\]]+\](?:/\[[^\]]+\])*)$", rest)
    if m:
        constraint.level = int(m.group(1))
        traits_str = m.group(2)
        first_trait_match = re.match(r"\[([^\]]+)\]", traits_str)
        if first_trait_match:
            constraint.trait = first_trait_match.group(1)
        return constraint

    # --- Pattern: Lv.N w/[Name] in text or w/[Trait] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+text\s+or\s+w/\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        # Text-contains is hard to enforce, but level helps; set both options
        # Set level at minimum
        return constraint

    # --- Pattern: Lv.N w/[Name] in text or Lv.N w/[Trait] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+text\s+or\s+Lv\.\d+\s+w/\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        return constraint

    # --- Pattern: Lv.N w/[Name] in text ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+text", rest)
    if m:
        constraint.level = int(m.group(1))
        # Text-contains is complex, but level is the key constraint
        return constraint

    # --- Pattern: Lv.N w/[Trait] in name or w/[Trait2] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+in\s+(?:its\s+)?name\s+or\s+w/\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Trait] trait and play cost ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+trait\s+and\s+", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.trait = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Trait] trait or Lv.N w/[Name] in name ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+trait\s+or\s+", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.trait = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Trait] trait w/o [Trait2] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+trait\s+w/o\s+", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.trait = m.group(2)
        return constraint

    # --- Pattern: Lv.N or higher w/[Name] in name ---
    m = re.match(r"Lv\.(\d+)\s+or\s+higher\s+w/\[([^\]]+)\]\s+in\s+name", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.name = m.group(2)
        return constraint

    # --- Pattern: Lv.N or higher w/[Trait] trait ---
    m = re.match(r"Lv\.(\d+)\s+or\s+higher\s+w/\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.trait = m.group(2)
        return constraint

    # --- Pattern: Lv.N w/[Trait] or [Trait2] in any trait or w/[Trait3] trait ---
    m = re.match(r"Lv\.(\d+)\s+w/\[([^\]]+)\]\s+or\s+\[([^\]]+)\]\s+in\s+any\s+trait\s+or\s+w/\[([^\]]+)\]\s+trait", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.trait = m.group(2)  # first trait
        return constraint

    # --- Pattern: "Lv. N 2-color w/Color" (note the space in "Lv. N") ---
    m = re.match(r"Lv\.\s*(\d+)\s+2-color\s+w/(\w+)", rest)
    if m:
        constraint.level = int(m.group(1))
        constraint.color = m.group(2)
        return constraint

    # --- Pattern: "2-color w/[Name] in name" ---
    m = re.match(r"2-color\s+w/\[([^\]]+)\]\s+in\s+name", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: "3 color [Name]" ---
    m = re.match(r"3\s+color\s+\[([^\]]+)\]$", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: Lv.N w/<Save> in text (full-width brackets) ---
    m = re.match(r"Lv\.(\d+)\s+w/\uff1c\w+\uff1e\s+in\s+text", rest)
    if m:
        constraint.level = int(m.group(1))
        return constraint

    # --- Pattern: Color Lv.N w/<Save> in text ---
    m = re.match(r"(?:\w+(?:,\s*\w+)*(?:,?\s*or\s+\w+)?\s+)?Lv\.(\d+)\s+w/\uff1c\w+\uff1e\s+in\s+text", rest)
    if m:
        constraint.level = int(m.group(1))
        return constraint

    # --- Pattern: Lv.N w/[Trait] in name or w/[Trait2] ---
    # Catch-all for level + something
    m = re.match(r"Lv\.(\d+)", rest)
    if m:
        constraint.level = int(m.group(1))
        # Try to find a trait
        trait_match = re.search(r"w/\[([^\]]+)\]\s+trait", rest)
        if trait_match:
            constraint.trait = trait_match.group(1)
        else:
            # Try name
            name_match = re.search(r"w/\[([^\]]+)\]\s+in\s+(?:its\s+)?name", rest)
            if name_match:
                constraint.name = name_match.group(1)
        return constraint

    # --- Pattern: "[CardName] in name" (without level) ---
    m = re.match(r"\[([^\]]+)\]\s+in\s+name$", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: "[Name1]/[Name2]/...": multi-name, take first ---
    m = re.match(r"\[([^\]]+)\](?:/\[([^\]]+)\])+$", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: Simple "[CardName]" (name match) ---
    m = re.match(r"\[([^\]]+)\]$", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # --- Pattern: "[Name] w/[Trait] under" or "[Name] w/[Trait] in digivolution cards" ---
    m = re.match(r"\[([^\]]+)\]\s+w/", rest)
    if m:
        constraint.name = m.group(1)
        return constraint

    # Fallback: just return with whatever we have (cost only)
    return constraint


def card_id_to_script_path(card_id: str) -> Optional[Path]:
    """Convert card ID to script file path.

    BT23-102 -> scripts/bt23/bt23_102.py
    P-206    -> scripts/p/p_206.py
    EX5-059  -> scripts/ex5/ex5_059.py
    """
    m = re.match(r"([A-Za-z]+)(\d*)-(\d+)", card_id)
    if not m:
        return None
    prefix = m.group(1).lower()
    set_num = m.group(2)
    card_num = m.group(3)

    set_id = f"{prefix}{set_num}" if set_num else prefix
    filename = f"{set_id}_{card_num}.py"
    return SCRIPTS_DIR / set_id / filename


def script_needs_fix(content: str) -> bool:
    """Check if a script has _alt_digi_cost but no constraint attributes."""
    if "_alt_digi_cost" not in content:
        return False
    has_constraint = any(
        attr in content
        for attr in ["_alt_digi_name", "_alt_digi_level", "_alt_digi_color", "_alt_digi_trait"]
    )
    return not has_constraint


def build_constraint_lines(constraint: AltDigiConstraint, effect_var: str, indent: str) -> List[str]:
    """Build the Python lines to insert after _alt_digi_cost."""
    lines = []
    if constraint.level is not None:
        lines.append(f"{indent}{effect_var}._alt_digi_level = {constraint.level}")
    if constraint.name is not None:
        lines.append(f'{indent}{effect_var}._alt_digi_name = "{constraint.name}"')
    if constraint.trait is not None:
        lines.append(f'{indent}{effect_var}._alt_digi_trait = "{constraint.trait}"')
    return lines


def patch_script(script_path: Path, constraint: AltDigiConstraint) -> Tuple[bool, str]:
    """Patch a script file to add missing constraint attributes.

    Returns (modified, message).
    """
    content = script_path.read_text(encoding="utf-8")

    if not script_needs_fix(content):
        return False, "already has constraints"

    # No constraints to add
    if constraint.level is None and constraint.name is None and constraint.trait is None:
        return False, f"no parseable constraints from: {constraint.raw_line}"

    # Find the _alt_digi_cost line(s)
    lines = content.split("\n")
    new_lines = []
    modified = False
    i = 0
    while i < len(lines):
        line = lines[i]
        new_lines.append(line)

        # Match: effectN._alt_digi_cost = X
        cost_match = re.match(r"^(\s+)(effect\d+)\._alt_digi_cost\s*=\s*\d+", line)
        if cost_match:
            indent = cost_match.group(1)
            effect_var = cost_match.group(2)
            constraint_lines = build_constraint_lines(constraint, effect_var, indent)
            if constraint_lines:
                new_lines.extend(constraint_lines)
                modified = True

        i += 1

    if modified:
        new_content = "\n".join(new_lines)
        script_path.write_text(new_content, encoding="utf-8")
        attrs = []
        if constraint.level is not None:
            attrs.append(f"level={constraint.level}")
        if constraint.name is not None:
            attrs.append(f'name="{constraint.name}"')
        if constraint.trait is not None:
            attrs.append(f'trait="{constraint.trait}"')
        return True, f"added {', '.join(attrs)}"

    return False, "no _alt_digi_cost line found in expected format"


def main():
    # Load cards.json
    with open(CARDS_JSON, "r", encoding="utf-8") as f:
        cards = json.load(f)

    # Collect cards with [Digivolve] in xros_req
    digivolve_cards = {}
    for card_id, card_data in cards.items():
        xros_req = card_data.get("xros_req", "")
        if not xros_req:
            continue

        # Split on \r\n to get individual lines, take first [Digivolve]
        xros_lines = xros_req.split("\r\n")
        first_digivolve = None
        for xline in xros_lines:
            xline = xline.strip()
            if xline.startswith("[Digivolve]"):
                first_digivolve = xline
                break

        if first_digivolve:
            digivolve_cards[card_id] = first_digivolve

    print(f"Found {len(digivolve_cards)} cards with [Digivolve] in xros_req")

    # Process each card
    modified_count = 0
    skipped_count = 0
    no_script_count = 0
    already_fixed_count = 0
    no_constraint_count = 0
    error_count = 0

    results = []

    for card_id, digivolve_line in sorted(digivolve_cards.items()):
        script_path = card_id_to_script_path(card_id)
        if script_path is None or not script_path.exists():
            no_script_count += 1
            continue

        # Parse the digivolve line
        constraint = parse_digivolve_line(digivolve_line)
        if constraint is None:
            error_count += 1
            results.append(f"  ERROR: Could not parse: {digivolve_line}")
            continue

        # Check if script needs fixing
        content = script_path.read_text(encoding="utf-8")
        if "_alt_digi_cost" not in content:
            skipped_count += 1
            continue

        if not script_needs_fix(content):
            already_fixed_count += 1
            continue

        # No useful constraints parsed
        if constraint.level is None and constraint.name is None and constraint.trait is None:
            no_constraint_count += 1
            results.append(f"  SKIP {card_id}: no parseable constraints from: {digivolve_line}")
            continue

        # Apply the fix
        was_modified, msg = patch_script(script_path, constraint)
        if was_modified:
            modified_count += 1
            results.append(f"  FIXED {card_id}: {msg}")
        else:
            results.append(f"  NOOP {card_id}: {msg}")

    # Print results
    print(f"\n{'=' * 70}")
    print("SUMMARY")
    print(f"{'=' * 70}")
    print(f"  Cards with [Digivolve] xros_req:  {len(digivolve_cards)}")
    print(f"  Scripts modified:                  {modified_count}")
    print(f"  Already have constraints:          {already_fixed_count}")
    print(f"  No script file found:              {no_script_count}")
    print(f"  No _alt_digi_cost in script:       {skipped_count}")
    print(f"  No parseable constraints:          {no_constraint_count}")
    print(f"  Parse errors:                      {error_count}")
    print(f"{'=' * 70}")

    if results:
        print("\nDETAILS:")
        for r in results:
            print(r)


if __name__ == "__main__":
    main()
