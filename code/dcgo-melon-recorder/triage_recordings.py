"""Triage a folder of MelonLoader `raw-*.jsonl` recordings: which games could seed training?

Usage:
  python code/dcgo-melon-recorder/triage_recordings.py <raw dir or file>... [options]

Options:
  --replay <dcgo-replay.exe>   also replay each converted game through our engine (slow-ish)
  --out-dir <dir>              keep the converted schema-v1 recordings here (default: a temp dir)
  --json <path>                write the full per-game report as JSON
  --min-turns <n>              shorter games are `too_short` (default 4)
  --cards <path>               data/cards.json (default)
  --admissibility <path>       data/card_admissibility.json (default; the training gate's own artifact)
  --candidates <path>          qa/dcgo-pvp/training_candidates.json (default): the owner's picked training games,
                               matched by start time; their status is reported in its own section

A raw file holds every game of one DCGO launch; each game (split at TurnStateMachine.Init) is triaged on its
own and named `<file>#g<n>`.

Each game gets the FIRST blocking verdict of this ladder (all blockers are still listed):

  not_a_game      the log never reached a mulligan snapshot (menus only, or a crash before the game)
  unfinished      no end of game was observed (disconnect, crash, game still running)
  converter_gap   the converter cannot encode a decision yet (names the DCGO method; fix convert_raw.py)
  too_short       fewer than --min-turns turns (early surrender / disconnect)
  illegal_deck    a deck is not 50 cards + 0-5 eggs with <= 4 copies per card number
  blocked_cards   a card in either deck is not admissible under the training gate (layer 2,
                  data/card_admissibility.json: reasons `unregistered`, `untested_lowering`, ...)
  replay_failed   (--replay only) our engine diverges from what DCGO did
  trainable       everything above passes (`trainable_unverified` without --replay)

Bot games (`is_ai`) are reported with opponent=bot: the bot seat's decisions are reconstructed, not
hooked, so prefer PvP games for training seeds. The global mechanics gate (data/mechanics_readiness.json,
layer 1) is printed once; when it is closed no game is trainable regardless of its own verdict.

Raw recordings hold BOTH full decks in order (the client simulates the whole game): never commit
someone else's game.
"""
import json
import os
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import convert_raw  # noqa: E402

VERDICT_ORDER = ["not_a_game", "unfinished", "converter_gap", "too_short", "illegal_deck",
                 "blocked_cards", "replay_failed", "trainable_unverified", "trainable"]


def load_json(path, default=None):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except OSError:
        return default


def deck_of(zones):
    main = list(zones.get("library_order", [])) + list(zones.get("security_order", [])) + list(zones.get("initial_hand", []))
    return main, list(zones.get("digitama_library_order", []))


def deck_problems(main, eggs):
    out = []
    if len(main) != 50:
        out.append("main deck has %d cards" % len(main))
    if len(eggs) > 5:
        out.append("egg deck has %d cards" % len(eggs))
    for cid, n in sorted(Counter(main + eggs).items()):
        if n > 4:
            out.append("%d copies of %s" % (n, cid))
    return out


def inadmissible(card_ids, adm_cards):
    out = {}
    for cid in sorted(set(card_ids)):
        e = adm_cards.get(cid)
        if e is None:
            out[cid] = "unregistered"
        elif not e.get("admissible"):
            out[cid] = e.get("reason") or "not_admissible"
    return out


def run_replay(exe, path, cards_path):
    """dcgo-replay's verdict for one converted file: pass / partial_pass / fail (+ the first divergence line)."""
    try:
        p = subprocess.run([exe, "-i", path, "--cards-json", cards_path, "--verbose"],
                           capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=900)
    except subprocess.TimeoutExpired:
        return "fail", "replay timed out"
    try:
        rep = json.loads(p.stdout)
    except ValueError:
        return "fail", "replay crashed: " + (p.stderr.strip().splitlines() or ["?"])[-1][:300]
    lines = p.stderr.splitlines()
    detail = ""
    for i, ln in enumerate(lines):
        if "→" in ln and i + 1 < len(lines):
            detail = lines[i + 1].strip()
    for k in ("pass", "partial_pass"):
        if rep.get(k):
            return k, detail
    if rep.get("quirk"):
        return "quirk", detail
    return "fail", detail


def split_games(rows):
    """One DCGO launch writes ONE raw file; every TurnStateMachine.Init starts a new game in it."""
    games, cur = [], None
    for r in rows:
        if r.get("method") == "TurnStateMachine.Init":
            cur = []
            games.append(cur)
        if cur is not None:
            cur.append(r)
    return games or [rows]


def triage_file(path, *args):
    try:
        rows = [json.loads(l) for l in open(path, encoding="utf-8") if l.strip()]
    except (OSError, ValueError) as e:
        return [{"file": os.path.basename(path), "verdict": "not_a_game", "blockers": ["unreadable: %s" % e]}]
    games = split_games(rows)
    base = os.path.basename(path)
    return [triage_one(base if len(games) == 1 else "%s#g%d" % (base, i), game, *args)
            for i, game in enumerate(games, 1)]


def triage_one(name, rows, cards, cards_path, adm_cards, min_turns, out_dir, replay_exe):
    g = {"file": name, "verdict": None, "blockers": []}
    if not any(r.get("method", "").startswith("Snapshot.") for r in rows):
        g["verdict"] = "not_a_game"
        g["blockers"].append("no game snapshot in the log")
        return g
    g["turns"] = max((r.get("turn") or 0) for r in rows)
    try:
        out, _ = convert_raw.convert(rows, cards, allow_truncated=True)
    except (SystemExit, Exception) as e:  # an old/odd log the converter cannot start on
        g["verdict"] = "not_a_game"
        g["blockers"].append("converter could not start: %s" % e)
        return g
    start = next((r for r in out if r.get("type") == "game_start"), {})
    init = next((r for r in out if r.get("type") == "initial_state"), None)
    end = next((r for r in out if r.get("type") == "game_end"), None)
    me = start.get("my_player_id")
    g.update({"game_id": start.get("game_id"), "started": start.get("timestamp"),
              "opponent": "bot" if start.get("is_ai") else "human", "my_seat": me,
              "decisions": sum(1 for r in out if r.get("type") in ("action", "selection"))})
    if end is not None:
        g["winner"] = end.get("winner")
        g["end_reason"] = end.get("reason")
        g["result"] = "won" if end.get("winner") == me else ("lost" if end.get("winner") in (0, 1) else "unknown")

    trunc = convert_raw.LAST_TRUNCATION
    if trunc:
        g["blockers"].append(("converter_gap", trunc))
    elif end is None:
        ended = any(r.get("method") == "TurnStateMachine.EndGame" for r in rows)
        g["blockers"].append(("unfinished", "game ended but the winner could not be determined (old mod build?)"
                              if ended else "no end of game observed (disconnect, crash, or still running)"))
    if g["turns"] < min_turns:
        g["blockers"].append(("too_short", "%d turns" % g["turns"]))

    if init is not None:
        g["decks"] = {}
        for side in ("my", "opp"):
            main, eggs = deck_of(init[side])
            probs = deck_problems(main, eggs)
            bad = inadmissible(main + eggs, adm_cards)
            g["decks"][side] = {"cards": sorted(set(main + eggs)), "inadmissible": bad, "problems": probs}
            if probs:
                g["blockers"].append(("illegal_deck", "%s: %s" % (side, "; ".join(probs))))
            if bad:
                g["blockers"].append(("blocked_cards", "%s: %d card(s)" % (side, len(bad))))

    conv_path = os.path.join(out_dir, os.path.splitext(g["file"].replace("#", "-"))[0] + ".v1.jsonl")
    if "#" in g["file"]:
        stem, game_no = g["file"].split("#")
        conv_path = os.path.join(out_dir, "%s-%s.v1.jsonl" % (os.path.splitext(stem)[0], game_no))
    with open(conv_path, "w", encoding="utf-8", newline="\n") as f:
        for r in out:
            f.write(json.dumps(r, separators=(",", ":")) + "\n")
    g["converted"] = conv_path

    if replay_exe:
        verdict, detail = run_replay(replay_exe, conv_path, cards_path)
        g["replay"] = verdict
        if detail:
            g["replay_detail"] = detail
        # a truncated recording can only ever partially pass; only a full pass of a whole game counts
        if verdict != "pass":
            g["blockers"].append(("replay_failed", "%s %s" % (verdict, detail)))

    kinds = [b[0] for b in g["blockers"]]
    g["verdict"] = next((v for v in VERDICT_ORDER if v in kinds),
                        "trainable" if replay_exe else "trainable_unverified")
    g["blockers"] = ["%s: %s" % b for b in g["blockers"]]
    return g


def main(argv):
    opts = {"--replay": None, "--out-dir": None, "--json": None, "--min-turns": "4",
            "--cards": "data/cards.json", "--admissibility": "data/card_admissibility.json",
            "--candidates": "qa/dcgo-pvp/training_candidates.json"}
    paths, i = [], 1
    while i < len(argv):
        if argv[i] in opts:
            opts[argv[i]] = argv[i + 1]
            i += 2
        elif argv[i] in ("-h", "--help"):
            raise SystemExit(__doc__)
        else:
            paths.append(argv[i])
            i += 1
    if not paths:
        raise SystemExit(__doc__)
    files = []
    for p in paths:
        if os.path.isdir(p):
            files += sorted(os.path.join(p, f) for f in os.listdir(p) if f.startswith("raw-") and f.endswith(".jsonl"))
        else:
            files.append(p)

    cards = convert_raw.load_cards(opts["--cards"])
    adm = load_json(opts["--admissibility"])
    if adm is None:
        raise SystemExit("%s is missing: the card gate needs it" % opts["--admissibility"])
    adm_cards = adm.get("cards", {})
    mech = load_json(os.path.join(os.path.dirname(opts["--admissibility"]), "mechanics_readiness.json"), {})
    out_dir = opts["--out-dir"] or tempfile.mkdtemp(prefix="dcgo-triage-")
    os.makedirs(out_dir, exist_ok=True)

    games = []
    for n, f in enumerate(files, 1):
        for g in triage_file(f, cards, opts["--cards"], adm_cards, int(opts["--min-turns"]), out_dir, opts["--replay"]):
            games.append(g)
            print("[%d/%d] %-38s %-20s %s" % (n, len(files), g["file"], g["verdict"],
                                              " | ".join(g["blockers"][:2])), flush=True)

    counts = Counter(g["verdict"] for g in games)
    print("\nmechanics gate (layer 1): %s" % ("open" if mech.get("ready") else "CLOSED - no game is trainable until it opens"))
    print("verdicts: " + ", ".join("%s %d" % (v, counts[v]) for v in VERDICT_ORDER if counts[v]))

    # What to do next: the cards and converter gaps that block the most otherwise-usable games.
    unblock, unblock_won, gaps = Counter(), Counter(), Counter()
    for g in games:
        if g["verdict"] == "converter_gap":
            gaps[next(b for b in g["blockers"] if b.startswith("converter_gap")).split(" at step")[0]] += 1
        if g["verdict"] in ("blocked_cards",):
            for side in g.get("decks", {}).values():
                for cid in side["inadmissible"]:
                    unblock[cid] += 1
                    if g.get("result") == "won":
                        unblock_won[cid] += 1
    if unblock:
        print("\ncards blocking the most games (games / of them won):")
        for cid, n in unblock.most_common(25):
            name = (cards.get(cid) or {}).get("card_name_eng", "")
            print("  %-10s %-24s %3d / %d" % (cid, name, n, unblock_won[cid]))
    if gaps:
        print("\nconverter gaps (games):")
        for k, n in gaps.most_common():
            print("  %3d  %s" % (n, k))
    picked = {c["started"]: c for c in (load_json(opts["--candidates"], {}) or {}).get("games", [])}
    if picked:
        print("\ntraining candidates (%s):" % opts["--candidates"])
        seen = set()
        for g in games:
            c = picked.get(g.get("started"))
            if c is None:
                continue
            seen.add(c["started"])
            g["candidate"] = c
            print("  %s  %s vs %s, %s  ->  %s" % (g["file"], c.get("my_deck"), c.get("opp_deck"), c.get("result"), g["verdict"]))
            for b in g["blockers"]:
                print("      " + b)
            for side, d in sorted(g.get("decks", {}).items()):
                if d["inadmissible"]:
                    print("      %s cards to implement: %s" % (side, ", ".join(
                        "%s %s" % (cid, (cards.get(cid) or {}).get("card_name_eng", "")) for cid in d["inadmissible"])))
        for k, c in picked.items():
            if k not in seen:
                print("  %s (%s): not in these recordings" % (c.get("raw"), k))
    usable = [g for g in games if g["verdict"].startswith("trainable")]
    if usable:
        print("\ntrainable games:")
        for g in usable:
            print("  %s  vs %s, %s in %d turns (%s)" % (g["file"], g.get("opponent"), g.get("result"), g["turns"],
                                                     g.get("end_reason")))
    print("\nconverted recordings: %s" % out_dir)
    if opts["--json"]:
        with open(opts["--json"], "w", encoding="utf-8", newline="\n") as f:
            json.dump({"mechanics_ready": bool(mech.get("ready")), "counts": dict(counts), "games": games}, f, indent=1)
        print("report: %s" % opts["--json"])


if __name__ == "__main__":
    main(sys.argv)
