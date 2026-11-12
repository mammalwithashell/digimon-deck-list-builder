"""Regenerate code/digimon-engine/src/runners/dcgo_int_menu_base.json.

DCGO's `SetIntSelection` records the card-authored `SelectionElement<int>.value`,
not a branch index. Most cards number their menu from 1 (1 = first option, and
the decline is often the constant 3 even when option 2 was not offered); a
minority number from 0. The replay resolver needs to know which, per card.

A card is listed here when every `SelectionElement<int>` value in its C# file is
>= 1 (so its menus are 1-based). Cards that use 0 anywhere are left to the
resolver's legacy 0-based position mapping.

Usage (from the base repo, where DCGO is checked out):
    python code/tools/dcgo-replay/scripts/gen_int_menu_base.py
"""
import json
import os
import re
import sys

root = "DCGO/Assets/Scripts/CardEffect"
out = "code/digimon-engine/src/runners/dcgo_int_menu_base.json"
if not os.path.isdir(root):
    sys.exit("run from the base repo root (DCGO/ must be checked out)")

one_based = []
for d, _, files in os.walk(root):
    for f in files:
        if not f.endswith(".cs"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8-sig", errors="replace").read()
        if "SelectionElement<int>" not in text:
            continue
        vals = [int(m.group(1)) for m in re.finditer(r"value\s*:\s*(\d+)\s*,\s*spriteIndex", text)]
        if vals and min(vals) >= 1:
            one_based.append(f[:-3].replace("_", "-"))
one_based.sort()
with open(out, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(one_based, fh, indent=0)
    fh.write("\n")
print(len(one_based), "one-based cards ->", out)
