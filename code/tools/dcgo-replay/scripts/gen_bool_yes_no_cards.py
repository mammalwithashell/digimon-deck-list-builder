"""Regenerate code/digimon-engine/src/runners/dcgo_bool_yes_no_cards.json.

A PvP recording's `generic_bool` row (DCGO `UserSelectionManager.SetBoolForPlayer`,
UserSelectionManager.cs:82-120) carries only the chosen bool: no message, no card.
It is the answer to a card-authored `SetBoolSelection` menu
(UserSelectionManager.cs:227-281), whose two `SelectionElement<bool>` buttons mean
whatever that card says: "Digivolve" / "Do not digivolve" (BT20_095.cs:126-130),
"From hand" / "From trash" (BT20_095.cs:142-146), "Security Top" / "Security
Bottom", ... A bool can therefore answer an engine yes/no prompt only when the
card that installed the prompt asks a YES/NO bool in DCGO at all.

A card is listed here when its own C# file builds a `SelectionElement<bool>`
whose `false` button is a decline ("No", "Not ...", "Do not ...", "Don't ...",
"Decline", "Cancel", "Skip"). Cards whose only bool menus are either/or
(hand/trash, top/bottom, ...) are left out: a bool from them never means
"no, decline". Bool menus asked by shared helpers (RevealLibrary.cs:270-282
"Will you select cards?", Guard.cs:89-98, Ascension.cs:30-39) are not attributed
to the calling card; the replay stops honestly on them until a fold maps them.

Usage (from the base repo, where DCGO is checked out):
    python code/tools/dcgo-replay/scripts/gen_bool_yes_no_cards.py
"""
import json
import os
import re
import sys

root = "DCGO/Assets/Scripts/CardEffect"
out = "code/digimon-engine/src/runners/dcgo_bool_yes_no_cards.json"
if not os.path.isdir(root):
    sys.exit("run from the base repo root (DCGO/ must be checked out)")

# `new(message: "X", value: false, ...)` or `new SelectionElement<bool>(message: $"X", value : false, ...)`
element = re.compile(r'message\s*:\s*\$?"([^"]*)"\s*,\s*value\s*:\s*false\b')
decline = re.compile(r"^\s*(no\b|not\b|do not\b|don't\b|don’t\b|decline\b|cancel\b|skip\b)", re.IGNORECASE)

cards = []
for d, _, files in os.walk(root):
    for f in files:
        if not f.endswith(".cs"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8-sig", errors="replace").read()
        if "SetBoolSelection" not in text:
            continue
        if any(decline.match(m.group(1)) for m in element.finditer(text)):
            cards.append(f[:-3].replace("_", "-"))
cards = sorted(set(cards))
with open(out, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(cards, fh, indent=0)
    fh.write("\n")
print(len(cards), "cards with a yes/no bool menu ->", out)
