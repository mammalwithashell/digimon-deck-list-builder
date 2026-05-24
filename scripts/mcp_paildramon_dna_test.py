"""Drive digimon-engine-mcp via stdio JSON-RPC to test BT16-025 Paildramon DNA digivolve.

Scenario (matches user request):
- P0 board: BT12-022 ExVeemon (Lv.4 Blue), BT12-050 Stingmon (Lv.4 Green),
            BT16-085 Davis & Ken (Tamer).
- P0 hand: BT16-025 Paildramon (Lv.5 Blue/Green; DNA cost 0 from Blue Lv.4 + Green Lv.4).
- Action: DNA digivolve.
- Expected memory gain: +3 (one each from BT12-022, BT12-050, BT16-085).
- Expected Paildramon when-digivolving: suspend all opp Digimon with <= stack size.
- Expected Paildramon on-dna-digivolve: opp Digimon cannot unsuspend until end of opp turn.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
MCP_BIN = REPO / "target" / "debug" / "digimon-engine-mcp.exe"


class MCPClient:
    def __init__(self, binary: Path, pool: str = "all") -> None:
        self.proc = subprocess.Popen(
            [str(binary), "--pool", pool],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
            cwd=str(REPO),
        )
        self._id = 0

    def _send(self, method: str, params: dict | None = None) -> dict:
        self._id += 1
        req = {"jsonrpc": "2.0", "id": self._id, "method": method}
        if params is not None:
            req["params"] = params
        line = json.dumps(req) + "\n"
        assert self.proc.stdin is not None and self.proc.stdout is not None
        self.proc.stdin.write(line.encode("utf-8"))
        self.proc.stdin.flush()
        # Read until we get the response for this id (skip notifications/log lines).
        while True:
            raw = self.proc.stdout.readline()
            if not raw:
                stderr = self.proc.stderr.read().decode("utf-8", errors="replace") if self.proc.stderr else ""
                raise RuntimeError(f"MCP server closed unexpectedly. stderr:\n{stderr}")
            raw_str = raw.decode("utf-8", errors="replace").strip()
            if not raw_str:
                continue
            try:
                msg = json.loads(raw_str)
            except json.JSONDecodeError:
                # Server may emit a non-JSON ready banner on stdout (it does — "digimon-engine-mcp: ready ...").
                # Skip those.
                continue
            if msg.get("id") == self._id:
                return msg
            # Otherwise it's a notification/server message — ignore.

    def initialize(self) -> dict:
        return self._send(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "paildramon-dna-test", "version": "0.1"},
            },
        )

    def list_tools(self) -> dict:
        return self._send("tools/list")

    def call(self, tool: str, args: dict) -> dict:
        resp = self._send("tools/call", {"name": tool, "arguments": args})
        if "error" in resp:
            raise RuntimeError(f"tool {tool} error: {resp['error']}")
        # MCP envelope: result.content = [{type:'text', text:'<json>'}]
        content = resp["result"]["content"]
        # Concatenate text parts and parse as JSON.
        text = "".join(part.get("text", "") for part in content if part.get("type") == "text")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return {"_raw": text}

    def close(self) -> None:
        try:
            self.proc.stdin.close()  # type: ignore[union-attr]
        except Exception:
            pass
        self.proc.wait(timeout=5)


def short(obj, n=400):
    s = json.dumps(obj, indent=2, default=str)
    return s if len(s) <= n else s[:n] + "...[truncated]"


def main() -> int:
    if not MCP_BIN.exists():
        print(f"ERROR: MCP binary not found at {MCP_BIN}", file=sys.stderr)
        return 2

    mcp = MCPClient(MCP_BIN)
    try:
        init = mcp.initialize()
        print(f"[init] {init['result']['serverInfo']}")

        # Use new_game_debug with explicit hands and decks.
        # P0 hand: cards we want to play to set up the board.
        # P1 hand: empty (will draw on its turn).
        # Decks: padded with cheap fillers so draw doesn't fail. We use ST1-04 (Patamon, Lv.3 cost 3)
        # and ST1-01 (Yokomon egg) so both players have viable draws.
        # NOTE: P0 hand order is the order cards land in `hand[idx]`.
        p0_hand = [
            "BT16-085",  # Davis & Ken (Tamer, cost 4)
            "BT12-022",  # ExVeemon (Lv.4 Blue, cost 4)
            "BT12-050",  # Stingmon (Lv.4 Green, cost 4)
            "BT16-025",  # Paildramon (will DNA digivolve)
        ]

        # Pad decks with filler so draws don't error. Use ST1 starter cards (well-known IDs).
        filler = ["ST1-03"] * 30  # ST1-03 Agumon — generic Lv.3 Rookie

        r = mcp.call(
            "new_game_debug",
            {
                "hands": {"0": p0_hand, "1": []},
                "decks": {"0": filler, "1": filler},
                "first_player": 0,
            },
        )
        if not r.get("ok"):
            print(f"new_game_debug failed: {r}")
            return 1
        game_id = r["game_id"]
        print(f"[new_game] id={game_id}")

        state = mcp.call("state", {"game_id": game_id, "view": "god"})
        print(f"[state-start] phase={state.get('phase')} memory={state.get('memory')}")

        def show_phase(label):
            s = mcp.call("state", {"game_id": game_id, "view": "god"})
            print(f"[{label}] phase={s.get('phase')} memory={s.get('memory')} turn_count={s.get('turn_count')}")
            return s

        def show_hand(player):
            h = mcp.call("hand", {"game_id": game_id, "player": player, "view": "god"})
            print(f"  hand[{player}] = {h}")
            return h

        def show_field(player):
            f = mcp.call("field", {"game_id": game_id, "player": player})
            print(f"  field[{player}] = {f}")
            return f

        def legal(player):
            la = mcp.call("legal_actions", {"game_id": game_id, "player": player})
            return la

        # Auto-resolve mulligan: both players keep.
        # Mulligan is the first phase. The expected action is action_id 1 (Keep) for each player.
        # Use legal_actions to find the keep action.
        for pid in (0, 1):
            la = legal(pid)
            print(f"[mulligan-actions p{pid}] {short(la, 300)}")

        # Walk through mulligan: step each player's first legal action that isn't mulligan.
        # The state machine should ask each in order. Use a loop.
        for _ in range(4):
            s = mcp.call("state", {"game_id": game_id, "view": "god"})
            phase = s.get("phase", "")
            if "Mulligan" not in phase and "mulligan" not in str(s):
                break
            dp = s.get("current_decision_player", s.get("current_player"))
            la = legal(dp if dp is not None else 0)
            ids = la.get("action_ids") or [a["action_id"] for a in la.get("actions", [])]
            if not ids:
                print(f"[mulligan] no legal actions for p{dp}; state={short(s,300)}")
                break
            # Look for "keep" label
            keep_id = None
            for a in la.get("actions", []):
                if "keep" in str(a).lower():
                    keep_id = a.get("action_id")
                    break
            if keep_id is None:
                keep_id = ids[0]
            r = mcp.call("step", {"game_id": game_id, "action_id": keep_id})
            print(f"[mulligan-step p{dp} action_id={keep_id}] ok={r.get('ok')} new_phase={r.get('new_phase')}")

        show_phase("after-mulligan")
        show_hand(0)
        show_field(0)

        # ── Helpers ────────────────────────────────────────────────────────
        def actions_of(la):
            if isinstance(la, list):
                return la
            if isinstance(la, dict):
                return la.get("actions", la.get("legal_actions", []))
            return []

        def find_play_action(player: int, card_id: str):
            for a in actions_of(legal(player)):
                if a.get("kind") == "play" and a.get("card_id") == card_id:
                    return a["action_id"]
            return None

        def find_action_by_label_contains(player: int, needle: str):
            for a in actions_of(legal(player)):
                if needle.lower() in str(a.get("label", "")).lower():
                    return a["action_id"], a
            return None, None

        def step(player: int, action_id: int, label: str = ""):
            r = mcp.call("step", {"game_id": game_id, "action_id": action_id})
            ok = r.get("ok")
            ph = r.get("new_phase")
            ev = r.get("events_emitted", [])
            ps = r.get("pending_selection_after")
            print(f"  step p{player} action={action_id} [{label}] ok={ok} new_phase={ph} events={len(ev)} pending={'yes' if ps else 'no'}")
            for e in ev:
                print(f"    event: {json.dumps(e, default=str)}")
            if ps:
                print(f"    pending: {short(ps, 600)}")
            return r

        def pass_or_end_turn(player: int):
            la = legal(player)
            for a in actions_of(la):
                lbl = str(a.get("label", "")).lower()
                k = str(a.get("kind", "")).lower()
                if k in ("end_turn", "pass_turn", "end-turn", "pass", "pass-turn") or "end turn" in lbl or lbl == "pass" or "pass turn" in lbl:
                    return step(player, a["action_id"], lbl)
            # Fallback: try the MCP-level end_turn/pass_turn tools.
            r = mcp.call("end_turn", {"game_id": game_id})
            if r.get("ok"):
                print(f"  [end_turn tool] p{player} ok new_phase={r.get('new_phase')}")
                return r
            r = mcp.call("pass_turn", {"game_id": game_id})
            print(f"  [pass_turn tool] p{player} ok={r.get('ok')} new_phase={r.get('new_phase')}")
            return r

        def drain_pending(label: str = "drain"):
            """Resolve pending selection(s).

            Policy:
            - "play without paying the cost" (Davis & Ken's free Veemon/Wormmon trigger) → DECLINE
              (otherwise the free-played card returns to hand at end of opp turn, breaking our DNA set-up).
            - "by suspending ... gain 1 memory" (Davis & Ken's tamer suspend for memory) → ACCEPT
              (this is one of the three memory gains we want to verify).
            - Otherwise → ACCEPT the first non-decline option.
            """
            resolved_any = False
            for _ in range(12):
                sel = mcp.call("pending_selection", {"game_id": game_id})
                if not sel or not sel.get("kind"):
                    return resolved_any
                opts = sel.get("options", []) or []
                prompt = str(sel.get("prompt", "")).lower()
                print(f"  [{label}] pending kind={sel.get('kind')} prompt={sel.get('prompt')!r} optional={sel.get('is_optional')}")
                for o in opts[:6]:
                    print(f"    opt: {o}")
                if not opts:
                    return resolved_any
                decline_needed = (
                    "without paying" in prompt
                    or ("veemon" in prompt and "wormmon" in prompt)
                )
                sp = sel.get("selecting_player", 0)

                # If we want to decline an optional prompt, the "Pass / decline" action
                # lives in `legal_actions` (kind=pass), not in pending.options.
                if decline_needed and sel.get("is_optional"):
                    la = legal(sp)
                    pass_aid = None
                    for a in actions_of(la):
                        if str(a.get("kind", "")).lower() == "pass":
                            pass_aid = a["action_id"]
                            break
                    if pass_aid is not None:
                        print(f"    -> declining via step action_id={pass_aid} (Pass/decline)")
                        r = mcp.call("step", {"game_id": game_id, "action_id": pass_aid})
                        ok = r.get("ok")
                        ev = r.get("events_emitted", [])
                        print(f"       decline result ok={ok} events={len(ev)} new_phase={r.get('new_phase')}")
                        for e in ev:
                            print(f"       event: {json.dumps(e, default=str)}")
                        resolved_any = True
                        continue

                chosen = None
                for o in opts:
                    lbl = str(o.get("label", "")).lower()
                    if "decline" not in lbl and "skip" not in lbl and "pass" not in lbl:
                        chosen = o.get("action_id")
                        break
                if chosen is None:
                    chosen = opts[0].get("action_id")
                r = mcp.call("resolve_selection", {"game_id": game_id, "player": sp, "action_id": chosen})
                ok = r.get("ok")
                ev = r.get("events_emitted", [])
                print(f"    -> resolve p{sp} action_id={chosen} ok={ok} events={len(ev)}")
                for e in ev:
                    print(f"      event: {json.dumps(e, default=str)}")
                resolved_any = True
            return resolved_any

        # ── Walk through the scenario ──────────────────────────────────────
        print("\n=== Turn 1: P0 plays Davis & Ken (Tamer) ===")
        aid = find_play_action(0, "BT16-085")
        assert aid is not None, "Davis & Ken not playable"
        step(0, aid, "play Davis & Ken")
        show_phase("after Davis & Ken")
        show_field(0)

        # End P0 turn → P1 gets memory.
        # Actually after playing a cost-4 with memory 0, turn auto-ends. Check.
        s = mcp.call("state", {"game_id": game_id, "view": "god"})
        print(f"  after-play: phase={s.get('phase')} memory={s.get('memory')} turn={s.get('turn_player')}")

        if s.get("turn_player") == 0 and "Main" in str(s.get("phase", "")):
            # Manually end turn.
            pass_or_end_turn(0)

        print("\n=== Turn 2: P1 passes ===")
        la = legal(1)
        print(f"  P1 legal actions: {short(la, 600)}")
        pass_or_end_turn(1)
        # Resolve any pending selections (Davis & Ken's start-of-main free Veemon play).
        drain_pending("after-T2-pass")
        show_phase("after T2 + drains")
        show_hand(0)
        show_field(0)

        # We may need ExVeemon on the field. If Davis & Ken put it there for free, great.
        # Otherwise play it manually.
        def have_on_field(player, card_id):
            f = mcp.call("field", {"game_id": game_id, "player": player})
            return any(p.get("top_card_id") == card_id for p in f.get("battle_area", []))

        print("\n=== Ensure ExVeemon on field ===")
        if not have_on_field(0, "BT12-022"):
            s = mcp.call("state", {"game_id": game_id, "view": "god"})
            if s.get("turn_player") != 0 or "Main" not in str(s.get("phase", "")):
                pass_or_end_turn(s.get("turn_player", 1))
                drain_pending("after-pass-to-P0")
            aid = find_play_action(0, "BT12-022")
            if aid is not None:
                step(0, aid, "play ExVeemon")
                drain_pending("after-ExVeemon-play")
            else:
                print(f"  ExVeemon not playable; current legal={legal(0)}")
        show_field(0)

        print("\n=== Ensure Stingmon on field ===")
        for safety in range(6):
            if have_on_field(0, "BT12-050"):
                break
            s = mcp.call("state", {"game_id": game_id, "view": "god"})
            if s.get("turn_player") == 0 and "Main" in str(s.get("phase", "")):
                aid = find_play_action(0, "BT12-050")
                if aid is not None:
                    step(0, aid, "play Stingmon")
                    drain_pending("after-Stingmon-play")
                    continue
            # Need to cycle turn. Pass.
            pass_or_end_turn(s.get("turn_player", 1))
            drain_pending(f"cycle-{safety}")
        show_field(0)

        # End-turn loop until we have Paildramon DNA digivolve as a legal action.
        for safety in range(8):
            s = mcp.call("state", {"game_id": game_id, "view": "god"})
            print(f"\n[loop {safety}] phase={s.get('phase')} memory={s.get('memory')} turn={s.get('turn_player')}")
            if s.get("turn_player") == 0 and "Main" in str(s.get("phase", "")):
                la = legal(0)
                # Look for any digivolve action whose card_id is BT16-025
                dna_action = None
                for a in actions_of(la):
                    k = str(a.get("kind", "")).lower()
                    if "digivolve" in k and a.get("card_id") == "BT16-025":
                        dna_action = a
                        break
                if dna_action:
                    print(f"  found Paildramon digivolve action: {short(dna_action, 500)}")
                    show_field(0)
                    mem_before = s.get("memory")
                    r = step(0, dna_action["action_id"], "digivolve into Paildramon")
                    print("  >>> events after digivolve:")
                    for e in r.get("events_emitted", []):
                        print(f"      {json.dumps(e, default=str)}")
                    # Resolve any pending selection (Davis & Ken's "by suspending" choice).
                    while True:
                        sel = mcp.call("pending_selection", {"game_id": game_id})
                        if not sel or not sel.get("kind"):
                            break
                        print(f"  pending selection: {short(sel, 600)}")
                        # Pick the "use" option for Davis & Ken (suspend tamer) — and for any target picks.
                        opts = sel.get("options", []) or sel.get("valid_action_ids", [])
                        if not opts:
                            break
                        # If a clear "use" / non-decline option exists, pick the first non-decline.
                        chosen = None
                        if isinstance(opts[0], dict):
                            for o in opts:
                                lbl = str(o.get("label", "")).lower()
                                if "decline" not in lbl and "skip" not in lbl and "no" != lbl.strip():
                                    chosen = o.get("action_id")
                                    break
                            if chosen is None:
                                chosen = opts[0].get("action_id")
                        else:
                            chosen = opts[0]
                        print(f"  resolving with action_id={chosen}")
                        r2 = mcp.call("resolve_selection", {"game_id": game_id, "player": 0, "action_id": chosen})
                        print(f"    resolve result events: {len(r2.get('events_emitted', []))}")
                        for e in r2.get("events_emitted", []):
                            print(f"      {json.dumps(e, default=str)}")

                    s2 = mcp.call("state", {"game_id": game_id, "view": "god"})
                    print(f"\n=== POST-DIGIVOLVE STATE ===")
                    print(f"  memory_before={mem_before} memory_after={s2.get('memory')}")
                    show_field(0)
                    show_field(1)
                    return 0
            # Otherwise: pass / end turn
            dp = s.get("current_decision_player")
            if dp is None:
                dp = s.get("turn_player")
            r = pass_or_end_turn(dp)
            if r is None:
                # try opposite player
                pass_or_end_turn(1 - (dp or 0))

        print("ERROR: could not reach a state where Paildramon DNA digivolve was legal.")
        return 1
    finally:
        mcp.close()


if __name__ == "__main__":
    sys.exit(main())
