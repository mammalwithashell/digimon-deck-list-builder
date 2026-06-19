"""Phase 4/5 battle-test: play full games across all ST1-6 starter-deck pairings
through the production PyO3 wire (RustHeadlessGame = HeadlessRunner, the training
path). Detects panics (exceptions), soft-locks (a non-terminal state with no legal
action), and confirms DigimonEnv-style reset/step. Greedy policy, seed-balanced.

Run: PYTHONPATH=code python openspec/changes/.../notes/play_starter_games.py [games_per_pairing]
"""
import json, sys, itertools, collections
import numpy as np
import digimon_engine as de

GAMES_PER_PAIRING = int(sys.argv[1]) if len(sys.argv) > 1 else 6
MAX_STEPS = 800

decks = {}
with open("data/deck_library.json", encoding="utf-8") as f:
    lib = json.load(f)
for key in ["ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
            "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet"]:
    raw = lib["archetypes"][key]["decklists"][0]["decklist"]
    ids = json.loads(raw)
    assert len(ids) == 54, f"{key}: expected 54 cards, got {len(ids)}"
    decks[key] = ids
short = {k: k.split()[0] for k in decks}  # "ST-1 Gaia Red" -> "ST-1"

def play_one(d1, d2, seed):
    """Drive a full game with greedy actions; return (winner, turns, status)."""
    g = de.RustHeadlessGame(d1, d2, seed=seed)
    steps = 0
    while steps < MAX_STEPS:
        st = g.get_rl_state()
        if st["game_over"]:
            return st.get("winner_id", 0), st.get("turn_count", 0), "completed"
        mask = np.asarray(g.get_action_mask())
        if not bool((mask > 0).any()):
            return 0, st.get("turn_count", 0), "SOFTLOCK"
        g.step(int(g.greedy_action()))
        steps += 1
    return 0, MAX_STEPS, "TIMEOUT"

pairings = list(itertools.combinations_with_replacement(decks.keys(), 2))
results = []
crashes = []
softlocks = []
print(f"Playing {GAMES_PER_PAIRING} games x {len(pairings)} pairings "
      f"= {GAMES_PER_PAIRING*len(pairings)} games (greedy, seed-balanced)\n")
for a, b in pairings:
    rec = {"pair": f"{short[a]} vs {short[b]}", "completed": 0, "p1": 0, "p2": 0,
           "draw": 0, "softlock": 0, "timeout": 0, "crash": 0, "turns": []}
    for i in range(GAMES_PER_PAIRING):
        seed = i  # seed parity flips first player (Game::new uses seed % 2)
        try:
            winner, turns, status = play_one(decks[a], decks[b], seed)
        except Exception as e:  # PyO3 surfaces a Rust panic as an exception
            rec["crash"] += 1
            crashes.append((rec["pair"], seed, repr(e)[:200]))
            continue
        if status == "completed":
            rec["completed"] += 1
            rec["turns"].append(turns)
            rec[{1: "p1", 2: "p2", 0: "draw"}[winner]] += 1
        elif status == "SOFTLOCK":
            rec["softlock"] += 1
            softlocks.append((rec["pair"], seed))
        elif status == "TIMEOUT":
            rec["timeout"] += 1
    avg = (sum(rec["turns"]) / len(rec["turns"])) if rec["turns"] else 0
    flag = "" if (rec["crash"] == 0 and rec["softlock"] == 0) else "  <<<< ISSUE"
    print(f"  {rec['pair']:<16}  done {rec['completed']}/{GAMES_PER_PAIRING}  "
          f"P1 {rec['p1']} P2 {rec['p2']} draw {rec['draw']}  "
          f"timeout {rec['timeout']} softlock {rec['softlock']} crash {rec['crash']}  "
          f"avg_turns {avg:.0f}{flag}", flush=True)
    results.append(rec)

tot = GAMES_PER_PAIRING * len(pairings)
done = sum(r["completed"] for r in results)
print(f"\n=== TOTALS ===")
print(f"games={tot} completed={done} "
      f"timeout={sum(r['timeout'] for r in results)} "
      f"softlock={sum(r['softlock'] for r in results)} "
      f"crash={sum(r['crash'] for r in results)}")
if crashes:
    print("\nCRASHES:")
    for c in crashes[:20]:
        print("  ", c)
if softlocks:
    print("\nSOFTLOCKS:")
    for s in softlocks[:20]:
        print("  ", s)

# --- DigimonEnv-style reset/step check for each list (Phase 5 readiness) ---
print("\n=== reset/step sanity (each list vs itself) ===")
for key in decks:
    g = de.RustHeadlessGame(decks[key], decks[key], seed=0)
    mask = np.asarray(g.get_action_mask())
    legal = int((mask > 0).sum())
    g.step(int(g.greedy_action()))
    print(f"  {short[key]:<6} reset->mask shape {mask.shape} legal={legal} step ok")

print("\nDONE" if not crashes and not softlocks else "\nDONE WITH ISSUES")
