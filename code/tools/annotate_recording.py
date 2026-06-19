#!/usr/bin/env python
"""Replay a training/eval game recording and print human-readable actions.

Turns raw action ids + slot indices into named board events, e.g.

    step 19  t6 P1 mem+2  CONCEDE
    step 20  t5 P1 mem+3  ATTACK  WereGarurumon (slot0, P1) -> Gabumon (slot2, P2)
                                  attacker sources: [WereGarurumon]  CannotAttack=YES

so a low/odd win rate or a step-limit loop is legible without decoding by hand.
Replays through the real engine (`RustReplayRunner`), so the card at each slot —
top Digimon + its source stack — comes straight from game state.

Usage:
    PYTHONPATH=code python code/tools/annotate_recording.py <recording.json> [--from N] [--to N]
    PYTHONPATH=code python code/tools/annotate_recording.py <rec.json> --loop   # auto-focus a repeated-action loop
"""
from __future__ import annotations

import argparse
import json
import sys

import digimon_engine as de

# Action-space ranges (code/digimon-engine/src/action/space.rs).
ATTACK_START, ATTACK_END = 100, 400
TARGETS_PER_ATTACKER = 15
SECURITY_TARGET = 14


def _cat(a: int) -> str:
    if a == 93: return "CONCEDE"
    if a in (94, 95): return "PLAY-ORDER"
    if 0 <= a < 30: return "PLAY"
    if 30 <= a < 60: return "SELECT"
    if a == 60: return "HATCH"
    if a == 61: return "MOVE-FROM-BREEDING"
    if a == 62: return "PASS/END-TURN"
    if 63 <= a < 93: return "DNA-DIGIVOLVE"
    if ATTACK_START <= a < ATTACK_END: return "ATTACK"
    if 400 <= a < 1000: return "DIGIVOLVE"
    if 1000 <= a < 1150: return "FIELD-EFFECT"
    if 1150 <= a < 1195: return "TRASH-EFFECT"
    if a >= 2000: return "SOURCE-SELECT"
    return f"OTHER({a})"


def _slot(state: dict, player_id: int, index: int):
    """Return (topCardName, [source names], modifier types) for a battle-area slot."""
    p = state.get(f"player{player_id}")
    if not p:
        return None, [], []
    ba = p.get("battleArea") or []
    if index < 0 or index >= len(ba):
        return None, [], []
    s = ba[index]
    name = s.get("topCardName") or s.get("name")
    sources = [c.get("cardName") for c in (s.get("sources") or [])]
    mods = [m.get("type") for m in (s.get("modifiers") or [])]
    return name, sources, mods


def annotate(action_id: int, acting: int, state: dict) -> str:
    cat = _cat(action_id)
    if cat == "ATTACK":
        off = action_id - ATTACK_START
        atk_slot, tgt = off // TARGETS_PER_ATTACKER, off % TARGETS_PER_ATTACKER
        opp = 2 if acting == 1 else 1
        an, asrc, amods = _slot(state, acting, atk_slot)
        line = f"ATTACK  {an} (slot{atk_slot}, P{acting})"
        if tgt == SECURITY_TARGET:
            line += " -> SECURITY"
        else:
            tn, _, _ = _slot(state, opp, tgt)
            line += f" -> {tn} (slot{tgt}, P{opp})"
        if asrc:
            line += f"  | sources={asrc}"
        if amods:
            line += f"  | attacker_modifiers={amods}"
        return line
    if cat in ("PLAY", "DIGIVOLVE", "FIELD-EFFECT", "DNA-DIGIVOLVE"):
        # name the acting player's relevant permanent where it's cheap to do so
        return f"{cat} (action {action_id})"
    return cat


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("recording")
    ap.add_argument("--from", dest="frm", type=int, default=0)
    ap.add_argument("--to", type=int, default=10**9)
    ap.add_argument("--loop", action="store_true",
                    help="Auto-focus the densest repeated-action run (a step-limit loop).")
    args = ap.parse_args()

    full = json.load(open(args.recording, encoding="utf-8"))
    rec = full.get("recording", full)
    out = full.get("outcome", {})
    print(f"# {args.recording.split('/')[-1]}")
    print(f"# outcome: result={out.get('result')} winner=P{out.get('winner_id')} "
          f"reason={out.get('win_reason') or out.get('draw_reason')} steps={out.get('step_count')}")

    # locate a loop (longest run of one action id) if requested
    focus_lo, focus_hi = args.frm, args.to
    if args.loop:
        acts = [a["action_id"] for a in rec["actions"]]
        best = (0, 0, None); run = 0; start = 0
        for i, a in enumerate(acts):
            if i and a == acts[i - 1]:
                run += 1
            else:
                run, start = 1, i
            if run > best[0]:
                best = (run, start, a)
        focus_lo = max(0, best[1] - 3)
        focus_hi = best[1] + 6
        print(f"# loop: action {best[2]} x{best[0]} starting near step {best[1]} "
              f"-> showing steps {focus_lo}..{focus_hi}")

    r = de.RustReplayRunner(json.dumps(rec), False)
    step = 0
    while not r.is_complete:
        state_before = r.get_state()
        res = r.step()
        step += 1
        if focus_lo <= step <= focus_hi:
            t = res.get("turn_number")
            mem = res.get("memory_before")
            line = annotate(res["action_id"], res["player_id"], state_before)
            print(f"  step{step:>4} t{t} P{res['player_id']} mem{mem:+d}  {line}")
        if step > focus_hi:
            break


if __name__ == "__main__":
    main()
