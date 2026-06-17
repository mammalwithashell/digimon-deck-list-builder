"""Phase 5: confirm the 6 starter lists are training-ready through the deck-pool /
archetype wiring (the `--archetypes` path) and resolve to legal 54-card decks."""
import json
from digimon_gym.agents.gauntlet import (
    canonicalize_archetype, _load_fully_implemented_archetypes,
    _parse_library_decklist as _parse_decklist,
)

KEYS = ["ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
        "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet"]

with open("data/deck_library.json", encoding="utf-8") as f:
    lib = json.load(f)

ready = _load_fully_implemented_archetypes()
print(f"training-ready archetype set size: {len(ready) if ready else 'None'}\n")

ok = True
for key in KEYS:
    canon = canonicalize_archetype(key)
    entry = lib["archetypes"][key]["decklists"][0]
    cards = _parse_decklist(entry)
    n = len(cards) if cards else 0
    in_ready = (ready is not None) and (canon in ready)
    status = "OK" if (n == 54 and in_ready) else "PROBLEM"
    if status != "OK":
        ok = False
    print(f"  {key:<22} canon='{canon}' cards={n} training_ready={in_ready}  [{status}]")

print("\nALL WIRED" if ok else "\nWIRING PROBLEM")
