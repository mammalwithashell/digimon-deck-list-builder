"""Regenerate code/digimon-engine/src/runners/dcgo_int_menu_base.json.

DCGO's `SetIntSelection` records the card-authored `SelectionElement<int>.value`,
not a branch index. Most cards number their menu from 1 (1 = first option, and
the decline is often the constant 3 even when option 2 was not offered); a
minority number from 0. The replay resolver needs to know which, per card.

A card is listed here when every `SelectionElement<int>` value in its C# file is
>= 1 (so its menus are 1-based). Cards that use 0 anywhere are left to the
resolver's legacy 0-based position mapping.

A menu element is written two ways in the DCGO scripts and BOTH are read:
the named form `new(message: "...", value: 1, spriteIndex: 0)` and the
positional form `new SelectionElement<int>("...", 1, 0)` (BT25-083, BT25-040,
BT25-042 use the latter; a named-only scan silently dropped them, so their
menus were never read as one-based).

Usage (from the base repo, where DCGO is checked out):
    python code/tools/dcgo-replay/scripts/gen_int_menu_base.py

From a worktree (whose ./DCGO is an empty placeholder, CLAUDE.md rule 29), name
the base copy: DCGO_CARD_EFFECT_ROOT=<base>/DCGO/Assets/Scripts/CardEffect
"""
import json
import os
import re
import sys

root = os.environ.get("DCGO_CARD_EFFECT_ROOT", "DCGO/Assets/Scripts/CardEffect")
out = "code/digimon-engine/src/runners/dcgo_int_menu_base.json"
if not os.path.isdir(root):
    sys.exit("run from the base repo root (DCGO/ must be checked out)")

# new SelectionElement<int>("Hand", 1, 0) - message (string literal or expression), value, sprite
POSITIONAL = re.compile(r"SelectionElement<int>\(\s*(?:\$?\"[^\"]*\"|[A-Za-z_][\w.()]*)\s*,\s*(-?\d+)\s*,\s*\d+\s*\)")

one_based = []
for d, _, files in os.walk(root):
    for f in files:
        if not f.endswith(".cs"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8-sig", errors="replace").read()
        if "SelectionElement<int>" not in text:
            continue
        vals = [int(m.group(1)) for m in re.finditer(r"value\s*:\s*(\d+)\s*,\s*spriteIndex", text)]
        vals += [int(m.group(1)) for m in POSITIONAL.finditer(text)]
        if vals and min(vals) >= 1:
            one_based.append(f[:-3].replace("_", "-"))
one_based.sort()
with open(out, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(one_based, fh, indent=0)
    fh.write("\n")
print(len(one_based), "one-based cards ->", out)
