"""Confirm whether BT16-085 Davis & Ken's optional 'by suspending → +1 memory'
prompt was actually surfaced as a player selection, or auto-resolved by the engine.

Strategy: after setting up the board, take the DNA digivolve action and then
poll `pending_selection` AND `effect_queue` between every action. Dump every
selection's options verbatim. Use the `events` MCP tool to compare totals to
per-step events_emitted.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Reuse the driver from the first script.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from mcp_paildramon_dna_test import MCPClient, MCP_BIN, short  # type: ignore


def main() -> int:
    if not MCP_BIN.exists():
        print(f"ERROR: MCP binary not found at {MCP_BIN}", file=sys.stderr)
        return 2

    mcp = MCPClient(MCP_BIN)
    try:
        mcp.initialize()
        p0_hand = ["BT16-085", "BT12-022", "BT12-050", "BT16-025"]
        filler = ["BT12-022"] * 30  # implemented-pool-safe filler
        r = mcp.call(
            "new_game_debug",
            {"hands": {"0": p0_hand, "1": []}, "decks": {"0": filler, "1": filler}, "first_player": 0},
        )
        gid = r["game_id"]

        def state():
            return mcp.call("state", {"game_id": gid, "view": "god"})

        def legal(p):
            la = mcp.call("legal_actions", {"game_id": gid, "player": p})
            return la if isinstance(la, list) else la.get("actions", [])

        def pending():
            return mcp.call("pending_selection", {"game_id": gid})

        def queue():
            return mcp.call("effect_queue", {"game_id": gid})

        def events(since=None):
            args = {"game_id": gid}
            if since is not None:
                args["since_seq"] = since
            return mcp.call("events", args)

        def find_play(player, cid):
            for a in legal(player):
                if a.get("kind") == "play" and a.get("card_id") == cid:
                    return a["action_id"]
            return None

        def find_decline(player):
            for a in legal(player):
                if a.get("kind") == "pass":
                    return a["action_id"]
            return None

        def step(action_id):
            return mcp.call("step", {"game_id": gid, "action_id": action_id})

        def dump(label):
            s = state()
            p = pending()
            q = queue()
            print(f"\n--- {label} ---")
            print(f"phase={s.get('phase')} memory={s.get('memory')} turn={s.get('turn_player')} decision_p={s.get('current_decision_player')}")
            if p and p.get("kind"):
                print(f"PENDING: kind={p.get('kind')} optional={p.get('is_optional')} prompt={p.get('prompt')!r}")
                print(f"  source_kind={p.get('source_kind')} source_perm={p.get('source_permanent')} source_card={p.get('source_card_id')}")
                for o in p.get("options", []):
                    print(f"  option: action_id={o.get('action_id')} label={o.get('label')!r}")
            else:
                print("PENDING: none")
            qlen = len(q.get("queue", [])) if isinstance(q, dict) else 0
            if qlen:
                print(f"EFFECT_QUEUE: {qlen} entries")
                for e in q.get("queue", [])[:6]:
                    print(f"  q: {e}")

        # ── Setup ──
        # T1: P0 plays Davis & Ken.
        aid = find_play(0, "BT16-085")
        step(aid)
        dump("after play D&K")

        # T2: P1 pass → D&K start-of-main trigger → decline.
        aid = find_decline(1)
        step(aid)
        dump("after P1 pass (D&K trigger now pending)")
        # Decline D&K's free-Veemon offer.
        aid = find_decline(0)
        step(aid)
        dump("after decline D&K free-Veemon")

        # T3 P0: play ExVeemon.
        aid = find_play(0, "BT12-022")
        step(aid)
        dump("after play ExVeemon")

        # P1's turn now (memory swung). Pass → D&K triggers again → decline.
        aid = find_decline(1)
        if aid is not None:
            step(aid)
            dump("after P1 pass (post-ExVeemon)")
            aid = find_decline(0)
            if aid is not None:
                step(aid)
                dump("after decline D&K (post-ExVeemon)")

        # T-next P0: play Stingmon.
        aid = find_play(0, "BT12-050")
        step(aid)
        dump("after play Stingmon")

        # Cycle once more if needed.
        for _ in range(3):
            s = state()
            if s.get("turn_player") == 0 and "Main" in str(s.get("phase", "")):
                # Look for DNA digivolve action.
                la = legal(0)
                dna = None
                for a in la:
                    if a.get("kind") == "dna_digivolve" and a.get("card_id") == "BT16-025":
                        dna = a
                        break
                if dna:
                    print(f"\n=== Found DNA digivolve action_id={dna['action_id']} ===")
                    break
            aid = find_decline(s.get("turn_player", 1))
            if aid is not None:
                step(aid)
                dump(f"cycling pass (turn={s.get('turn_player')})")
                aid = find_decline(0)
                if aid is not None:
                    step(aid)
                    dump("after possible decline")

        s = state()
        events_before_seq = (events().get("events") or [])
        last_seq_before_dna = (events_before_seq[-1].get("seq") if events_before_seq else -1)
        print(f"\n>>> Last event seq before DNA digivolve = {last_seq_before_dna}")

        # ── The actual test: step the DNA digivolve ──
        la = legal(0)
        dna = next(a for a in la if a.get("kind") == "dna_digivolve" and a.get("card_id") == "BT16-025")
        print(f"\n=== STEP {dna['action_id']} (DNA digivolve into Paildramon) ===")
        r = step(dna["action_id"])
        print(f"  step result ok={r.get('ok')} new_phase={r.get('new_phase')} events_emitted={len(r.get('events_emitted', []))}")
        for e in r.get("events_emitted", []):
            print(f"    event: {json.dumps(e)}")
        dump("immediately after DNA digivolve step")

        # Drain materials, polling pending_selection AND legal_actions after each resolve.
        iteration = 0
        while True:
            iteration += 1
            if iteration > 12:
                print("!! too many drain iterations; aborting")
                break
            p = pending()
            if not p or not p.get("kind"):
                print(f"\n--- drain iter {iteration}: no pending; done ---")
                break
            sp = p.get("selecting_player", 0)
            print(f"\n--- drain iter {iteration} ---")
            print(f"PENDING: kind={p.get('kind')} optional={p.get('is_optional')} prompt={p.get('prompt')!r} selecting_p={sp}")
            print(f"  source_kind={p.get('source_kind')} source_perm={p.get('source_permanent')} source_card={p.get('source_card_id')}")
            for o in p.get("options", []):
                print(f"  option: action_id={o.get('action_id')} label={o.get('label')!r}")
            # Also dump legal_actions to see if a "decline" exists at this point.
            la = legal(sp)
            for a in la:
                if a.get("kind") in ("pass",):
                    print(f"  LEGAL pass action_id={a.get('action_id')} label={a.get('label')!r}")
            # Pick option (mandatory pickers: first option; optional: pick whatever we want).
            if not p.get("options"):
                print("  no options to pick; breaking")
                break
            # If the prompt mentions "suspending" or "memory" or is from Tamer source — accept first option (the user wants the memory).
            prompt_low = str(p.get("prompt", "")).lower()
            opt0 = p["options"][0]
            chosen = opt0.get("action_id")
            print(f"  -> resolving action_id={chosen}")
            r2 = mcp.call("resolve_selection", {"game_id": gid, "player": sp, "action_id": chosen})
            evs = r2.get("events_emitted", [])
            print(f"  resolve result ok={r2.get('ok')} events_emitted={len(evs)}")
            for e in evs:
                print(f"    event: {json.dumps(e)}")

        print("\n=== FINAL STATE ===")
        s = state()
        print(f"memory={s.get('memory')} phase={s.get('phase')}")
        # Pull all events emitted from the DNA digivolve onward.
        all_events = events(since=last_seq_before_dna + 1).get("events", [])
        print(f"events since DNA digivolve step ({len(all_events)}):")
        for e in all_events:
            print(f"  {json.dumps(e)}")

        # Field check.
        f0 = mcp.call("field", {"game_id": gid, "player": 0})
        for p in f0.get("battle_area", []):
            print(f"P0 field: {p.get('top_card_name')} suspended={p.get('is_suspended')} stack={p.get('stack_card_ids')}")

        return 0
    finally:
        mcp.close()


if __name__ == "__main__":
    sys.exit(main())
