#!/usr/bin/env python3
"""UserPromptSubmit hook: when a prompt mentions Digimon card IDs, remind the
assistant to VIEW the actual card images (not just cards.json / the YAML DSL).

Reads the hook payload as JSON on stdin, scans the prompt for card IDs, and prints
a short context block listing each ID with its resolved local image path plus an
instruction to invoke the `digimon-card-lookup` skill and `Read` the images.

Cheap (regex + path existence only) and silent when the prompt mentions no card IDs.
Card *names* and *archetypes* are intentionally not scanned here (too fuzzy / noisy);
the skill's description handles those triggers, and the block below points at the
resolver for them.
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

DEFAULT_IMAGE_DIR = r"C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card"
CARD_ID_SCAN_RE = re.compile(r"\b(?:BT|EX|ST|RB|AD|LM|P)\d{0,2}-\d{2,3}\b", re.IGNORECASE)


def image_dir() -> Path:
    return Path(os.environ.get("DIGIMON_CARD_IMAGE_DIR", DEFAULT_IMAGE_DIR))


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0  # not JSON / nothing to do — stay silent

    prompt = str(payload.get("prompt", "") or "")
    if not prompt:
        return 0

    # Preserve first-seen order, de-duplicate, normalize to upper-case.
    seen: list[str] = []
    for m in CARD_ID_SCAN_RE.finditer(prompt):
        cid = m.group(0).upper()
        if cid not in seen:
            seen.append(cid)
    if not seen:
        return 0

    idir = image_dir()
    lines = [
        "[digimon-card-lookup] Card IDs detected in this prompt. Before reasoning about "
        "their effects, invoke the `digimon-card-lookup` skill and `Read` the card "
        "image(s) — the printed card face is the authoritative source for card text, "
        "not cards.json. Resolved local images:",
    ]
    for cid in seen:
        img = idir / f"{cid}.webp"
        if img.is_file():
            lines.append(f"  - {cid}: {img}")
        else:
            lines.append(
                f"  - {cid}: (not in local mirror — run "
                f"`python .claude/skills/digimon-card-lookup/resolve_cards.py {cid}` "
                f"to fetch it from the CDN)"
            )
    lines.append(
        "  For a card NAME (many printings) or an ARCHETYPE, run "
        "`python .claude/skills/digimon-card-lookup/resolve_cards.py <query>`."
    )
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
