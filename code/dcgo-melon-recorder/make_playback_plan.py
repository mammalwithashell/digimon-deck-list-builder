"""Turn a native recording of a game our engine played into a DCGO playback plan for the MelonLoader mod.

    python make_playback_plan.py <native recording .json> <plan .txt>

The input is a `GameRecorder` recording (`digimon-engine-cli sim-record`, or any eval recording) whose
`initial_state` is the post-mulligan position. The plan is what the mod's shuffle postfixes force (Core.cs
"playback"), one item per line:

    first player (0 = seat 0 = the room's master client, 1 = seat 1 = the joiner)
    seat 0 main deck, top first      (comma-separated card ids)
    seat 0 egg deck, top first
    seat 1 main deck, top first
    seat 1 egg deck, top first

Orientation. The native recording stores zones the engine's way (a Vec popped from the END, so index 0 = bottom).
DCGO's lists are top first. DCGO draws the opening hand from the top 5, then deals security one card at a time onto
the FRONT of its security list, then the rest is the library. A DCGO main deck of
    hand + security (engine Vec order) + reversed(library)
therefore deals exactly the recorded hand, security and library (checked against a DCGO pre/post-mulligan snapshot
pair: kept hand = top 5, security = the next 5 reversed). A seat that redrew gets the same order again at the
mulligan reshuffle, so its new hand is the recorded one.

Then run `dcgo-agent --script <the same recording>` and one PvP client per seat, both with playback.txt pointing at
this plan.
"""
import json
import sys


def side_order(side):
    main = list(side["initial_hand"]) + list(side["security_order"]) + list(reversed(side["library_order"]))
    egg = list(reversed(side.get("digitama_library_order") or []))
    return main, egg


def main(argv):
    if len(argv) != 3:
        print(__doc__)
        return 2
    with open(argv[1], encoding="utf-8") as f:
        doc = json.load(f)
    rec = doc.get("recording", doc)
    init = rec.get("initial_state")
    if not init:
        print("recording has no initial_state", file=sys.stderr)
        return 1
    first = int(init["first_player_id"]) - 1  # recordings use the 1/2 player convention
    m0, e0 = side_order(init["player1"])
    m1, e1 = side_order(init["player2"])
    with open(argv[2], "w", encoding="utf-8", newline="\n") as f:
        f.write("%d\n" % first)
        for lst in (m0, e0, m1, e1):
            f.write(",".join(lst) + "\n")
    mull = [a for a in rec.get("actions", []) if a.get("phase") == "Mulligan"]
    print("plan -> %s: first player seat %d, decks %d+%d / %d+%d, mulligans %s, %d decisions after them"
          % (argv[2], first, len(m0), len(e0), len(m1), len(e1),
             [(a["player_id"] - 1, "redraw" if a["action_id"] == 1 else "keep") for a in mull],
             len(rec.get("actions", [])) - len(mull)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
