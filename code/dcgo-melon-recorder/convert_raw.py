"""Convert the MelonLoader mod's raw-<session>.jsonl into a schema-v1 DCGO recording.

The mod (Core.cs) logs plain method calls: TurnStateMachine.SetRedraw / SetBreedingPhase /
QueueMainPhaseAction[_Internal] / PassTurn / EndGame / Surrender, selection setters,
PlayCardClass.ctor (the card a seat is playing, with that seat's hand), Permanent.AddCardSource, and
Snapshot.<hook> rows holding both seats' zones. This script turns that stream into the rows
dcgo-replay consumes (docs/DCGO_RECORDING_SCHEMA.md).

Local seat (hooked setters):
  - mulligan     SetRedraw(actor, redrew)               -> action 0 (keep) / 1 (redraw)
  - breeding     SetBreedingPhase(actor, doBreeding)    -> PASS (62) when declined
  - main phase   QueueMainPhaseAction_Internal packet 5 -> PASS (62)   [PassAction]
Bot seat (NOT hooked as decisions - IL2CPP inlines the setters the AI brain uses), reconstructed:
  - a PlayCardClass.ctor row for the bot with a known hand slot and no target -> PLAY_HAND slot
  - the bot's breeding decision and the end of its turn are IMPLICIT (nothing fires): they are
    synthesised as PASS (62) at the turn boundary. This is an inference, flagged in the output's
    `inferred` list; the replay is the check on it (a wrong guess shows up as a divergence).
Anything else (hatching, a play onto a target, an attack, any selection) is NOT encoded: the converter
stops there and marks the recording truncated (`--allow-truncated`), because skipping a decision would
desynchronise every later row. dcgo-replay reports truncated recordings as partial.

Usage: python convert_raw.py raw-<session>.jsonl OUT.jsonl [--allow-truncated] [--cards data/cards.json]
"""
import json
import os
import sys
import uuid

SCHEMA_VERSION = 1  # recording-row version written by DCGO ActionSpace.SCHEMA_VERSION (the engine action-space constant is a separate, newer number)
ACTION_KEEP, ACTION_REDRAW, ACTION_PASS, ACTION_HATCH, ACTION_CONCEDE = 0, 1, 62, 60, 93
ACTION_MOVE = 61  # MOVE_FROM_BREEDING
ATTACK_START, TARGETS_PER_ATTACKER, SECURITY_TARGET = 100, 15, 14
PLAY_PACKET_ID, ATTACK_PACKET_ID, ACTIVATE_PERMANENT_PACKET_ID = 4, 0, 2
FIELD_EFFECT_START, EFFECTS_PER_PERMANENT = 1000, 10
PLAY_HAND_START, PLAY_HAND_END = 0, 30
DIGIVOLVE_START, FIELDS_PER_HAND, BREEDING_TARGET = 400, 15, 14
PASS_PACKET_ID = 5  # QueueMainPhaseAction_Internal(playerID, packetId=5, bytes): PassAction

SELECTION_PREFIXES = ("SelectCardEffect.", "UserSelectionManager.SetBoolForPlayer", "SelectHandEffect.",
                      "SelectPermanentEffect.", "SelectAttackEffect.", "SelectCountEffect.")


LAST_TRUNCATION = None


def deck_order(player):
    """Post-shuffle deck, drawn from index 0 first: the opening hand was drawn first, then the library."""
    return list(player["hand"]) + list(player["library"])


def load_cards(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except OSError:
        return {}


def is_egg(cards, card_id):
    c = cards.get(card_id) or {}
    return "egg" in str(c.get("card_kind", "")).lower() or c.get("level") == 2 and (c.get("play_cost") in (0, None))


def convert(rows, cards, allow_truncated=False):
    snap = next((r for r in rows if r["method"].startswith("Snapshot.SetRedraw")), None)
    mulligans = [r for r in rows if r["method"] == "TurnStateMachine.SetRedraw"]
    if snap is None:
        # A live log read at the FIRST mulligan decision has no SetRedraw yet: the agent's request snapshot holds the
        # same pre-mulligan zones (hands dealt, nothing redrawn).
        snap = next((r for r in rows if r["method"] == "Snapshot.Request" and r.get("turn_count", 0) == 0), None)
    if snap is None:
        raise SystemExit("no pre-mulligan snapshot (Snapshot.SetRedraw or a turn-0 Snapshot.Request)")
    you = snap["you"]
    bot = 1 - you
    players = {p["id"]: p for p in snap["players"]}
    # DCGO's first player mulligans first; with no mulligan recorded yet, the seat asking is the one deciding first.
    first_player = mulligans[0]["args"][0] if mulligans else you
    is_ai = bool(snap["is_ai"])

    out = [{
        "v": SCHEMA_VERSION,
        "type": "game_start",
        "game_id": uuid.uuid4().hex,
        "timestamp": snap["t"],
        "my_player_id": you,
        "first_player": first_player,
        "is_ai": is_ai,
        "my_deck_post_shuffle": deck_order(players[you]),
        # Each DCGO client simulates the whole game locally (PvP included), so its snapshot holds BOTH decks in order.
        "opp_deck_post_shuffle": deck_order(players[bot]),
        "my_egg_deck": list(players[you]["egg_library"]),
        "opp_egg_deck": list(players[bot]["egg_library"]),
    }]
    if not is_ai:
        out[0]["opp_decklist_composition"] = deck_order(players[bot])

    post = next((r for r in rows if r["method"] == "Snapshot.PostMulligan"), None)
    if post is not None:
        pm = {p["id"]: p for p in post["players"]}

        def zones(pl):
            return {"library_order": list(pl["library"]), "digitama_library_order": list(pl["egg_library"]),
                    "security_order": list(pl["security"]), "initial_hand": list(pl["hand"])}

        init = {"type": "initial_state", "first_player_id": first_player, "my": zones(pm[you]), "opp": zones(pm[bot])}
        out.append(init)

    step = 0
    truncated_at = None
    winner = -1
    reason = ""
    inferred = []

    def action(actor, action_id, phase, source, card_id=None, inferred_row=False, boards=None):
        nonlocal step
        row = {"type": "action", "step": step, "actor": actor, "action_id": action_id,
               "phase": phase, "source": source}
        if boards:
            row["board_p0"], row["board_p1"] = boards[0], boards[1]
        if inferred_row:
            row["inferred"] = True  # synthesised by this converter, not observed by the recorder
        if card_id:
            row["card_id"] = card_id
        out.append(row)
        step += 1

    # Bot turns: the bot decides without any hooked setter, so its breeding PASS comes first on a turn and its
    # main-phase PASS closes the turn. Track which bot turns already had their breeding / pass synthesised.
    bot_turn_open = None  # turn index of the bot's currently open turn
    bot_bred = False

    snap_idx = [i for i, x in enumerate(rows) if x["method"].startswith("Snapshot.")]

    def bot_eggs(x):
        return len(next(pp for pp in x["players"] if pp["id"] == bot)["egg_library"])

    def bot_hatched_near(idx):
        """INFERRED: the bot hatched an egg if its egg deck is smaller in the first snapshot after row idx than in the
        last snapshot before it. Only the bot's turns fall between those two snapshots when the local seat never
        hatches (the scripted walk always declines)."""
        # "before" = the last snapshot from BEFORE the bot's turn began (live requests also snapshot mid-turn,
        # after the hatch); "after" = the first snapshot after the decision.
        turn_now = rows[idx].get("turn", rows[idx].get("turn_count", -1))
        before = [i for i in snap_idx if i < idx and rows[i].get("turn_count", -1) < turn_now] or [i for i in snap_idx if i < idx]
        after = [i for i in snap_idx if i > idx]
        if not before or not after:
            return False
        return bot_eggs(rows[before[-1]]) - bot_eggs(rows[after[0]]) == 1

    def breeding_for_bot(idx, turn):
        """The bot's breeding decision for a turn: HATCH when the egg deck shrank, otherwise PASS (both inferred)."""
        if bot_hatched_near(idx):
            action(bot, ACTION_HATCH, "Breeding", "breeding_action", inferred_row=True)
            inferred.append("bot breeding HATCH (egg deck shrank), turn %s" % turn)
        else:
            action(bot, ACTION_PASS, "Breeding", "breeding_action", inferred_row=True)
            inferred.append("bot breeding PASS, turn %s" % turn)

    bot_turn_first_idx = None

    def close_bot_turn(why):
        nonlocal bot_turn_open, bot_bred
        if bot_turn_open is None:
            return
        if not bot_bred:
            breeding_for_bot(bot_turn_first_idx if bot_turn_first_idx is not None else len(rows), bot_turn_open)
        if why == "end of game":
            # The game ended by surrender in the middle of the bot's turn (possibly mid-effect, with a prompt open):
            # the bot never passed, so no pass is invented.
            inferred.append("bot turn %s cut off by the end of the game (no pass synthesised)" % bot_turn_open)
            bot_turn_open = None
            bot_bred = False
            return
        action(bot, ACTION_PASS, "Main", "main_phase", inferred_row=True)
        inferred.append("bot main PASS (%s), turn %s" % (why, bot_turn_open))
        bot_turn_open = None
        bot_bred = False

    started = False  # past the mulligans
    breeding_occupied = {0: False, 1: False}
    queued_play_indexes = set()  # card indexes of queued main-phase play packets not yet resolved
    for ridx, r in enumerate(rows):
        m, a = r["method"], r.get("args", [])
        tp, turn = r.get("tp", -1), r.get("turn", r.get("turn_count", -1))
        if m.startswith("Snapshot."):
            for pl in r.get("players", []):
                if "breeding" in pl:
                    breeding_occupied[pl["id"]] = bool(pl["breeding"])

        # entering a different turn than the bot's open one closes it
        if bot_turn_open is not None and tp != bot and turn != bot_turn_open and turn >= 0:
            close_bot_turn("turn change")

        if m == "TurnStateMachine.SetRedraw":
            action(a[0], ACTION_REDRAW if a[1] else ACTION_KEEP, "Mulligan", "mulligan")
            continue
        if m.startswith("Snapshot.") or m.startswith("Permanent.") or m == "TurnStateMachine.Init":
            continue
        if m in ("PlayCardClass.ctor", "PlayCardClass.PlayCard"):
            if turn < 1:
                continue  # the start-up constructions (tp -1)
            hand = r.get("hand") or {}
            actor = hand.get("player", tp)
            if actor not in (0, 1):
                continue
            # Every seat's main-phase play is taken from this row: it is logged when DCGO RESOLVES the play and carries
            # the resolved hand slot and target on every client. The play packet is logged when the play is queued
            # (replaying it then ran ahead of DCGO), and on the receiving PvP client it is decoded against a state
            # that has not caught up yet (hand slot / target missing).
            if r.get("pay_cost") is False:
                # an effect playing a card (not a main-phase decision): resolved by the effect itself
                continue
            if not r.get("cards") and not r.get("hand"):
                # an effect's play / digivolve step that played nothing (e.g. a "may digivolve" with no card chosen):
                # no card moved, no decision
                continue
            # A main-phase play is always queued first (a play packet naming the card). A paid play with no such
            # packet is an effect's play or digivolve ("may digivolve into ... with the cost reduced by 1"): the
            # effect's own selection rows carry that decision, so it is not a main-phase action.
            hand_cards = (r.get("hand") or {}).get("cards") or []
            slot = (r.get("hand") or {}).get("slot", -1)
            played_index = hand_cards[slot].get("card_index") if 0 <= slot < len(hand_cards) else None
            if played_index is not None and played_index not in queued_play_indexes:
                continue
            if played_index is not None:
                queued_play_indexes.discard(played_index)
            started = True
            # Only a BOT seat's turn is inferred (its breeding / end-of-turn are invisible). A human or PvP seat's
            # breeding and pass decisions are recorded rows.
            infer = is_ai and actor == bot
            if infer and bot_turn_open is None:
                bot_turn_open = turn
                bot_turn_first_idx = ridx
            card = (r.get("cards") or [None])[0]
            if card and is_egg(cards, card):
                truncated_at = truncated_at or ("bot hatch of %s at step %d" % (card, step))
                break
            slot = hand.get("slot", -1)
            digivolve_field = None
            if r.get("target") is not None:
                # A play onto an occupied frame is a digivolution: DIGIVOLVE_START + hand * 15 + field, where field is
                # the compact battle-area position of the target, or 14 (BREEDING_TARGET) for the breeding area.
                if r.get("target_breeding"):
                    digivolve_field = BREEDING_TARGET
                elif r.get("target_slot", -1) >= 0:
                    digivolve_field = r["target_slot"]
                else:
                    truncated_at = truncated_at or ("digivolve of %s with an unlocatable target at step %d" % (card, step))
                    break
            if not (0 <= slot < PLAY_HAND_END):
                truncated_at = truncated_at or ("play of %s with no hand slot at step %d" % (card, step))
                break
            if infer and not bot_bred:
                breeding_for_bot(ridx, turn)
                bot_bred = True
            if digivolve_field is not None:
                action_id = DIGIVOLVE_START + slot * FIELDS_PER_HAND + digivolve_field
            else:
                action_id = PLAY_HAND_START + slot
            action(actor, action_id, "Main", "main_phase", card_id=card, boards=r.get("boards"))
            continue
        if m == "TurnStateMachine.SetBreedingPhase":
            if a[1]:
                # doBreeding: hatch when the breeding area is empty, otherwise move its Digimon to the battle area
                # (DCGO's own rule in SetBreedingPhase). Occupancy comes from the last snapshot.
                action(a[0], ACTION_MOVE if breeding_occupied.get(a[0]) else ACTION_HATCH, "Breeding", "breeding_action")
                breeding_occupied[a[0]] = not breeding_occupied.get(a[0])
            else:
                action(a[0], ACTION_PASS, "Breeding", "breeding_action")
            continue
        if m == "TurnStateMachine.QueueMainPhaseAction_Internal":
            if isinstance(r.get("play"), dict) and r["play"].get("card_index") is not None:
                queued_play_indexes.add(r["play"]["card_index"])
            # GamePacketRegistration order: 0 attack, 1 activate card, 2 activate permanent, 3 cheat, 4 play, 5 pass
            # An attack is taken from SetAttackingPermaent, logged when DCGO EXECUTES the queued attack (with both
            # battle areas, the index basis). The attack packet is only the request: decoding it as well counted every
            # attack twice, and an attack DCGO never executes must not reach the replay.
            if a[1] == ACTIVATE_PERMANENT_PACKET_ID and len(a) >= 3 and isinstance(a[2], list) and len(a[2]) >= 8:
                # ActivatePermanentAction(field index, skill index): a permanent's [Main] effect. The field index is a
                # GetFieldPermanents() position (boards carry that order for the replay's slot remap).
                def be32(b, i):
                    v = (b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]
                    return v - (1 << 32) if v & 0x80000000 else v
                perm, skill = be32(a[2], 0), be32(a[2], 4)
                if not (0 <= perm < 15 and 0 <= skill < EFFECTS_PER_PERMANENT):
                    truncated_at = truncated_at or ("activate packet (%d, %d) at step %d" % (perm, skill, step))
                    break
                action(a[0], FIELD_EFFECT_START + perm * EFFECTS_PER_PERMANENT + skill, "Main", "main_phase",
                       boards=r.get("boards"))
                continue
            if a[1] in (PLAY_PACKET_ID, ATTACK_PACKET_ID):
                continue  # attacks are decoded from SetAttackingPermaent (and plays without packet info from PlayCard rows)
            if a[1] != PASS_PACKET_ID:
                truncated_at = truncated_at or ("main-phase packet %s at step %d" % (a[1], step))
                break
            action(a[0], ACTION_PASS, "Main", "main_phase")
            continue
        if m == "TurnStateMachine.SetAttackingPermaent":
            # (attacker compact battle-area index, target compact index on the opponent's side or -1 = security)
            actor = tp
            if actor not in (0, 1) or len(a) < 2:
                truncated_at = truncated_at or ("attack without a turn player at step %d" % step)
                break
            target = SECURITY_TARGET if a[1] < 0 else a[1]
            action(actor, ATTACK_START + a[0] * TARGETS_PER_ATTACKER + target, "Main", "main_phase", boards=r.get("boards"))
            continue
        if m in ("TurnStateMachine.QueueMainPhaseAction", "TurnStateMachine.PassTurn"):
            continue  # the Internal row above already carries the decision
        if m == "SelectCardEffect.SetTargetCardAndIndicies" and "card_ids" in r:
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectCardEffect", "phase": "Main"}
            if r["card_ids"] and None not in r["card_ids"]:
                row["card_ids"] = r["card_ids"]
            elif not r["card_ids"]:
                row["cancel"] = True
            else:
                truncated_at = truncated_at or ("SelectCardEffect with an unresolved card index at step %d" % step)
                break
            if r.get("zone"):
                row["zone"] = r["zone"]
            if r.get("mechanic"):
                row["mechanic"] = r["mechanic"]
            # DCGO asks Assembly one material element at a time, each with its own "No Selection"; the engine asks
            # once and one decline ends it. Keep only the first decline of a run.
            prev = out[-1] if out else None
            if (row.get("cancel") and row.get("mechanic") == "assembly" and prev is not None
                    and prev.get("type") == "selection" and prev.get("cancel") and prev.get("mechanic") == "assembly"
                    and prev.get("actor") == row["actor"]):
                continue
            out.append(row)
            step += 1
            continue
        if m == "MultipleSkills.SetTargetSkill":
            # trigger order: DCGO's index into its own list of the pending effects (-1 = "don't activate these
            # effects"); chosen_effect_card (the local seat's panel pick) lets the replay match the effect by card
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "MultipleSkills", "phase": "Main",
                   "int_value": a[1]}
            if r.get("chosen_effect_card"):
                row["chosen_effect_card"] = r["chosen_effect_card"]
            out.append(row)
            step += 1
            continue
        if m == "UserSelectionManager.SetBoolForPlayer":
            # The UserSelectionManager bool channel (`generic_bool`): yes/no gates and top/bottom placements
            # (EX12-018 "Top of digivolution sources" = true). The replay maps it onto the live two-branch prompt.
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "generic_bool",
                   "phase": "Main", "bool_value": bool(a[1])}
            if "engine_action" in r:
                row["engine_action"] = r["engine_action"]
            out.append(row)
            step += 1
            continue
        if m == "OptionalSkill.SetUseOptional":
            # "Use this optional effect?" - only an answer the engine asked for too carries an engine action (the
            # engine folds some into the next pick, or resolves a "may" with no legal target without asking)
            if "engine_action" in r:
                out.append({"type": "selection", "step": step, "actor": a[0], "prompt": "generic_bool",
                            "phase": "Main", "bool_value": bool(a[1]), "engine_action": r["engine_action"]})
                step += 1
            continue
        if m == "UserSelectionManager.SetIntForPlayer":
            # The UserSelectionManager fallback channel: the schema's `generic_int` prompt (an index into the prompt's
            # own branch list, e.g. "activate 1 of the effects below"). The replay resolves it against the live prompt.
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "generic_int",
                   "phase": "Main", "int_value": a[1]}
            if "engine_action" in r:
                row["engine_action"] = r["engine_action"]
            out.append(row)
            step += 1
            continue
        if m == "SelectCountEffect.SetCount":
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectCountEffect",
                   "phase": "Main", "count": a[1]}
            if "engine_action" in r:
                row["engine_action"] = r["engine_action"]
            out.append(row)
            step += 1
            continue
        if m == "SelectPermanentEffect.SetTargetFrames":
            # (playerID, isTurnPlayer[], UnitIndex[]): the turn-relative bit becomes an absolute player id, the
            # index is the compact battle-area position (the schema's `frame`).
            if tp not in (0, 1) or len(a) < 3:
                truncated_at = truncated_at or ("SelectPermanentEffect without a turn player at step %d" % step)
                break
            targets = [{"player": tp if is_tp else 1 - tp, "frame": idx} for is_tp, idx in zip(a[1], a[2])]
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectPermanentEffect",
                   "phase": "Main"}
            if r.get("boards") and len(r["boards"]) == 2:
                # DCGO's own field order at the pick (GetFieldPermanents); the replay maps it onto engine slots
                row["board_p0"], row["board_p1"] = r["boards"][0], r["boards"][1]
            if targets:
                row["targets"] = targets
            else:
                row["cancel"] = True
            out.append(row)
            step += 1
            continue
        if m == "SelectHandEffect.SetTargetHandCards" and "card_ids" in r:
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectHandEffect", "phase": "Main",
                   "zone": "Hand"}
            if r["card_ids"] and None not in r["card_ids"]:
                row["card_ids"] = r["card_ids"]
            elif not r["card_ids"]:
                row["cancel"] = True
            else:
                truncated_at = truncated_at or ("SelectHandEffect with an unresolved card index at step %d" % step)
                break
            out.append(row)
            step += 1
            continue
        if m.startswith(SELECTION_PREFIXES):
            truncated_at = truncated_at or ("%s at step %d" % (m, step))
            break
        if m == "TurnStateMachine.EndGame" and winner == -1 and truncated_at is None:
            # EndGame(Player winner, bool surrendered, string effectName): the mod logs the winner's id when it can;
            # older recordings only have the next Snapshot.EndGame - the turn player won when the other seat's
            # security is empty (the attack that ended the game).
            w = r.get("winner")
            if w is None:
                snap = next((x for x in rows[ridx + 1:ridx + 3] if x.get("method") == "Snapshot.EndGame"), None)
                if snap is not None and tp in (0, 1):
                    other = next((p for p in snap.get("players", []) if p.get("id") == 1 - tp), None)
                    if other is not None and not other.get("security"):
                        w = tp
            if w is not None and w >= 0:
                if bot_turn_open is not None:
                    close_bot_turn("end of game")
                winner = int(w)
                reason = "concede" if (len(a) > 1 and a[1] is True) else ((a[2] if len(a) > 2 and a[2] else "win"))
            continue
        if m == "TurnStateMachine.Surrender":
            # Observed directly (not inferred): the surrendering seat concedes. CONCEDE_GAME (93) is accepted by the
            # replay harness from either seat at any decision point. The bot's open turn ends with the game.
            if bot_turn_open is not None and truncated_at is None:
                close_bot_turn("end of game")
            action(a[0], ACTION_CONCEDE, "Main", "concede")
            winner = 1 - a[0]
            reason = "surrender"

    if truncated_at is None and bot_turn_open is not None and winner == -1:
        close_bot_turn("end of game")
    global LAST_TRUNCATION
    LAST_TRUNCATION = truncated_at
    if truncated_at is not None:
        if not allow_truncated:
            raise SystemExit("unsupported decision (%s); pass --allow-truncated to emit a partial recording" % truncated_at)
        reason = "truncated: " + truncated_at
        winner = -1
    if truncated_at is None and reason:
        out.append({"type": "game_end", "winner": winner, "reason": reason, "total_steps": step})
    # No observed end (a live log the agent is reading, or a crash): no game_end row, so the replay treats the
    # recording as a prefix instead of checking a winner that does not exist yet.
    # A truncated recording deliberately has NO game_end row: dcgo-replay then reports it as a partial pass
    # (a prefix that matched) instead of a full pass.
    return out, inferred


def main(argv):
    args = [x for x in argv[1:] if not x.startswith("--")]
    cards_path = "data/cards.json"
    if "--cards" in argv:
        cards_path = argv[argv.index("--cards") + 1]
        args = [x for x in args if x != cards_path]
    if len(args) != 2:
        raise SystemExit(__doc__)
    rows = [json.loads(l) for l in open(args[0], encoding="utf-8") if l.strip()]
    out, inferred = convert(rows, load_cards(cards_path), allow_truncated="--allow-truncated" in argv)
    with open(args[1], "w", encoding="utf-8", newline="\n") as f:
        for r in out:
            f.write(json.dumps(r, separators=(",", ":")) + "\n")
    last = out[-1]
    if last.get("type") != "game_end":
        print("TRUNCATED: no game_end row written (see the converter's truncated_at reason below)")
    print("wrote %d rows (%d decisions, %d inferred) -> %s" % (
        len(out), sum(1 for r in out if r["type"] == "action"), len(inferred), args[1]))
    for i in inferred:
        print("  inferred:", i)
    if LAST_TRUNCATION:
        print("  truncated at:", LAST_TRUNCATION)


if __name__ == "__main__":
    main(sys.argv)
