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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import frame_model  # noqa: E402

SCHEMA_VERSION = 1  # recording-row version written by DCGO ActionSpace.SCHEMA_VERSION (the engine action-space constant is a separate, newer number)
ACTION_KEEP, ACTION_REDRAW, ACTION_PASS, ACTION_HATCH, ACTION_CONCEDE = 0, 1, 62, 60, 93
ACTION_MOVE = 61  # MOVE_FROM_BREEDING
ATTACK_START, TARGETS_PER_ATTACKER, SECURITY_TARGET = 100, 15, 14
PLAY_PACKET_ID, ATTACK_PACKET_ID, ACTIVATE_PERMANENT_PACKET_ID = 4, 0, 2
ACTIVATE_CARD_PACKET_ID = 1  # ActivateCardAction: a hand card's [Main]
HAND_EFFECT_START, HAND_EFFECT_END = 30, 60  # engine action ids: HAND_EFFECT_START + hand slot
FIELD_EFFECT_START, EFFECTS_PER_PERMANENT = 1000, 10
PLAY_HAND_START, PLAY_HAND_END = 0, 30
DIGIVOLVE_START, FIELDS_PER_HAND, BREEDING_TARGET = 400, 15, 14
DNA_DIGIVOLVE_START, DNA_DIGIVOLVE_END = 63, 93  # DNA digivolve: DNA_DIGIVOLVE_START + hand index
PASS_PACKET_ID = 5  # QueueMainPhaseAction_Internal(playerID, packetId=5, bytes): PassAction

SELECTION_PREFIXES = ("SelectCardEffect.", "UserSelectionManager.SetBoolForPlayer", "SelectHandEffect.",
                      "SelectPermanentEffect.", "SelectAttackEffect.", "SelectCountEffect.")


LAST_TRUNCATION = None


def _s32(b, i):
    """Big-endian signed 32-bit int at byte offset `i` of a packet payload."""
    v = (b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]
    return v - (1 << 32) if v & 0x80000000 else v


def hand_slot_of_card_index(rows, ridx, seat, card_index):
    """(hand slot, card id) of the hand card with DCGO `card_index` for `seat`, read from the first PlayCard / ctor row
    after rows[ridx] whose hand listing (`hand.cards`, each with its `card_index`) names it, or (None, None).

    The scan stops at the seat's next queued main-phase packet (the next decision): a listing logged after it
    describes a later hand."""
    for later in rows[ridx + 1:]:
        lm = later.get("method", "")
        if lm == "TurnStateMachine.QueueMainPhaseAction_Internal":
            break
        if lm not in ("PlayCardClass.ctor", "PlayCardClass.PlayCard"):
            continue
        hand = later.get("hand") or {}
        if hand.get("player") != seat:
            continue
        for slot, c in enumerate(hand.get("cards") or []):
            if c.get("card_index") == card_index and c.get("card_id") is not None:
                return slot, c["card_id"]
    return None, None


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


def is_tamer(cards, card_id):
    """Whether cards.json knows `card_id` as a Tamer (card_kind 1, the engine's CardKind::Tamer); unknown is False."""
    k = (cards.get(card_id) or {}).get("card_kind")
    return k == 1 or "tamer" in str(k).lower()


def jogress_root_slots(roots, anchor_frame, anchor_slot, n_battle):
    """Compact battle-area positions of a DNA play's two root frames, or None when they are not certain.

    DCGO names the roots by FrameID, in pick order (TurnStateMachine.cs:2983 queues
    [permanents[0].PermanentFrame.FrameID, permanents[1].PermanentFrame.FrameID]; CardController.cs:540 and :1682
    resolve them as Owner.fieldCardFrames[id]), while the engine's DNA material prompts take compact battle-area
    indices. A FrameID is the frame's list position (Player.cs:31/48 `FrameID = count`, the breeding frame last) and
    indexes FieldPermanents (CardObjectController.cs:564), which GetBattleAreaPermanents walks in id order
    (Player.cs:621), so a root's compact position is the number of occupied battle frames below it. Frame occupancy
    is not logged; the one anchor is the play's TargetFrameID, whose permanent (TurnStateMachine.cs:1819) the mod
    logs at compact `anchor_slot`. Each root is bounded from that anchor (both roots are occupied battle frames:
    CR 4.3 6-5-1-2-2 "multiple Digimon in your battle area", SelectJogressEffect.cs:210) and accepted only when its
    bounds meet."""
    if (len(roots) != 2 or len(set(roots)) != 2 or min(roots) < 0
            or not (0 <= anchor_slot <= anchor_frame) or anchor_slot >= n_battle):
        return None
    slots = []
    for r in roots:
        others = [o for o in roots if o != r]
        if r == anchor_frame:
            lo = hi = anchor_slot
        elif r < anchor_frame:
            # anchor_slot occupied battle frames lie below the anchor, r among them
            between = sum(1 for o in others if r < o < anchor_frame)
            below = sum(1 for o in others if o < r)
            lo = max(below, anchor_slot - 1 - (anchor_frame - r - 1))
            hi = min(r, anchor_slot - 1 - between)
        else:
            between = sum(1 for o in others if anchor_frame < o < r)
            above = sum(1 for o in others if o > r)
            lo = anchor_slot + 1 + between
            hi = min(anchor_slot + 1 + (r - anchor_frame - 1), n_battle - 1 - above)
        if lo != hi or not 0 <= lo < n_battle:
            return None
        slots.append(lo)
    return slots if len(set(slots)) == 2 else None


def pick_card_indexes(args, card_ids):
    """DCGO's CardIndex of each picked card, parallel to `card_ids`, or None.

    SelectHandEffect.SetTargetHandCards(playerID, int[] cardIndexes) and
    SelectCardEffect.SetTargetCardAndIndicies(playerID, int[] cardIndexes, int[] ...) name the picks by CardIndex
    (the mod resolves `card_ids` from those same indexes), so the two lists pair one to one."""
    if len(args) < 2 or not isinstance(args[1], list) or len(args[1]) != len(card_ids):
        return None
    if not all(isinstance(k, int) and not isinstance(k, bool) for k in args[1]):
        return None
    return list(args[1])


def pick_list_positions(args, card_ids):
    """The position of each pick in the candidate list DCGO showed, parallel to `card_ids`, or None.

    SelectCardEffect.SetTargetCardAndIndicies(playerID, int[] cardIndexes, int[] positions): `positions` is
    the third argument, one non-negative int per picked card."""
    if len(args) < 3 or not isinstance(args[2], list) or len(args[2]) != len(card_ids):
        return None
    if not all(isinstance(k, int) and not isinstance(k, bool) and k >= 0 for k in args[2]):
        return None
    return list(args[2])


def _hand_listing(row, seat):
    """The (card_id, card_index) list of `seat`'s hand that a PlayCardClass.PlayCard row logged (Core.cs OwnerHandJson:
    the owner's HandCards in hand order, each with its CardIndex, taken BEFORE the played card leaves), or None."""
    if row.get("method") != "PlayCardClass.PlayCard":
        return None
    hand = row.get("hand") or {}
    cards = hand.get("cards")
    if hand.get("player") != seat or not isinstance(cards, list):
        return None
    return [(c.get("card_id"), c.get("card_index")) for c in cards if isinstance(c, dict)]


def returned_tamer_index(rows, ridx, seat, card_id):
    """DCGO's CardIndex of the Tamer a Burst Digivolve returned to `seat`'s hand, or None when it is not certain.

    `rows[ridx]` is the burst digivolve's own PlayCardClass.PlayCard row: DCGO returns the picked Tamer to its owner's
    hand while it resolves that play (CardController.cs:942 BounceTamer returns it; CardController.cs:254-256 is only
    the BurstTamer frame lookup; CR 4.3 8-3: the specified Tamer returns to the hand).
    The returned card is a card of the picked id that is
      * absent from the hand listing this row logged (taken before the play),
      * present in the owner's NEXT hand listing in the same turn, and
      * was taken out of the hand by a play earlier: its last listing before this row is the PlayCard row that
        played it (the hand slot names it), so it went from the hand onto the field.
    A copy drawn or searched in between was never played from the hand and does not qualify. Exactly one card must
    qualify; two (another bounce before the next listing) or none leave the pick positional."""
    before = _hand_listing(rows[ridx], seat)
    if before is None:
        return None
    turn = rows[ridx].get("turn")
    after = None
    for r in rows[ridx + 1:]:
        if r.get("method") in ("TurnStateMachine.Init", "TurnStateMachine.EndGame") or r.get("turn") != turn:
            break
        after = _hand_listing(r, seat)
        if after is not None:
            break
    if after is None:
        return None
    had = {k for _, k in before}
    new = [k for c, k in after if c == card_id and isinstance(k, int) and k not in had]

    def played_from_hand(k):
        for r in reversed(rows[:ridx]):
            if r.get("method") == "TurnStateMachine.Init":
                return False
            listing = _hand_listing(r, seat)
            if listing is None or all(x != k for _, x in listing):
                continue
            slot = (r.get("hand") or {}).get("slot", -1)
            return 0 <= slot < len(listing) and listing[slot][1] == k
        return False

    qualified = [k for k in new if played_from_hand(k)]
    return qualified[0] if len(qualified) == 1 else None


def convert(rows, cards, allow_truncated=False):
    snap = next((r for r in rows if r["method"].startswith("Snapshot.SetRedraw")), None)
    mulligans = [r for r in rows if r["method"] == "TurnStateMachine.SetRedraw"]
    if snap is None:
        # A live log read at the FIRST mulligan decision has no SetRedraw yet: the agent's request snapshot holds the
        # same pre-mulligan zones (hands dealt, nothing redrawn).
        snap = next((r for r in rows if r["method"] == "Snapshot.Request" and r.get("turn_count", 0) == 0), None)
    deck_of = deck_order
    if snap is None:
        # Mod builds before 2026-11-13 took the pre-mulligan snapshot only for the FIRST game of a DCGO session, so
        # later games have only the post-mulligan one. The replay restores zones from `initial_state` (written from
        # that same snapshot) and skips the mulligan decisions, so game_start needs each deck's cards, not their
        # pre-mulligan order: hand + security + library is the whole deck.
        snap = next((r for r in rows if r["method"] == "Snapshot.PostMulligan"), None)
        deck_of = lambda pl: list(pl["hand"]) + list(pl["security"]) + list(pl["library"])  # noqa: E731
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
        "my_deck_post_shuffle": deck_of(players[you]),
        # Each DCGO client simulates the whole game locally (PvP included), so its snapshot holds BOTH decks in order.
        "opp_deck_post_shuffle": deck_of(players[bot]),
        "my_egg_deck": list(players[you]["egg_library"]),
        "opp_egg_deck": list(players[bot]["egg_library"]),
    }]
    if not is_ai:
        out[0]["opp_decklist_composition"] = deck_of(players[bot])

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
    dna_play_indexes = set()  # card indexes whose LATEST queued play packet named DNA (jogress) roots
    dna_unknown_indexes = set()  # card indexes whose latest queued play packet's jogress list could not be read
    dna_roots = {}  # card index -> (jogress root FrameIDs or None when unreadable, the packet's TargetFrameID)
    dna_picks = {}  # card index -> the material pick rows (in `out`) the local seat logged before its DNA packet
    # card index -> (BurstTamerFrameID, the Tamer pick row in `out` the local seat logged just before the packet, or
    # None when no such row exists - a remote seat's pick ran on its own client only, the frame model's certain
    # naming of that Tamer (frame_model.burst_tamers: seat, position, card id, card index) or None, the packet row's
    # boards)
    burst_plays = {}
    # card index -> whether a play packet logged with NO payload bytes (an older recorder build) looks like a DNA
    dna_bytes_absent = {}
    winner = -1
    reason = ""
    inferred = []

    def board_operand_index(r, seat, position):
        """DCGO's CardIndex of the permanent at `position` of `seat`'s recorded board, when it must be named: the
        board holds another copy of that card (so the position alone cannot say which permanent it is) and the row
        is a recorder 0.2.0 row whose `board_indexes` lines up with `boards` (Core.cs BoardsJson: the top card's
        CardIndex per entry). None otherwise."""
        boards, exact = r.get("boards"), r.get("board_indexes")
        if not (isinstance(boards, list) and len(boards) == 2 and isinstance(exact, list) and len(exact) == 2
                and seat in (0, 1) and isinstance(boards[seat], list) and isinstance(exact[seat], list)
                and len(exact[seat]) == len(boards[seat]) and isinstance(position, int)
                and 0 <= position < len(boards[seat])):
            return None
        k = exact[seat][position]
        if isinstance(k, bool) or not isinstance(k, int) or boards[seat].count(boards[seat][position]) < 2:
            return None
        return k

    def action(actor, action_id, phase, source, card_id=None, inferred_row=False, boards=None, card_index=None,
               operand_card_index=None, operand_source="recorder", attack_target_card_index=None):
        nonlocal step
        row = {"type": "action", "step": step, "actor": actor, "action_id": action_id,
               "phase": phase, "source": source}
        if boards:
            row["board_p0"], row["board_p1"] = boards[0], boards[1]
        if inferred_row:
            row["inferred"] = True  # synthesised by this converter, not observed by the recorder
        if card_id:
            row["card_id"] = card_id
        if isinstance(card_index, int):
            # DCGO's CardIndex of the played card (its hand slot in the resolve row's hand listing): the replay pairs
            # it with the engine card this action moves (ReplayDriver::record_dcgo_card_indexes)
            row["card_index"] = card_index
        if isinstance(operand_card_index, int) and not isinstance(operand_card_index, bool):
            # DCGO's CardIndex of the action's own-board operand (the attacker, the digivolve target, the activated
            # permanent) when the board holds several copies of that card: the replay resolves the permanent by
            # identity (`ReplayDriver::named_operand_slot`) instead of pairing duplicates in occurrence order
            row["operand_card_index"] = operand_card_index
            row["operand_card_index_source"] = operand_source
        if attack_target_card_index is not None:
            row["attack_target_card_index"] = attack_target_card_index
        out.append(row)
        step += 1

    def move_last_before(k):
        """Move the row just written ahead of the k rows before it, renumbering the steps (the rows keep order)."""
        if not k:
            return
        row = out.pop()
        at = len(out) - k
        row["step"] = out[at]["step"]
        for later in out[at:]:
            later["step"] += 1
        out.insert(at, row)

    # id(out row) -> index in `rows` of the raw row that wrote it (`tag_sources`), so a row written out of raw order
    # (a DNA / burst action moved ahead of its picks) is still placed by when DCGO logged it.
    src_of = {}

    def tag_sources(ridx):
        for x in out:
            src_of.setdefault(id(x), ridx)

    def last_action_src(seat):
        """Raw index of the row that wrote `seat`'s latest action in `out`, or -1."""
        return max((src_of.get(id(x), -1) for x in out if x.get("type") == "action" and x.get("actor") == seat),
                   default=-1)

    def own_picks_before(seat, ridx, limit):
        """The trailing run (at most `limit`, in logged order) of `seat`'s own-side SelectPermanentEffect rows that END
        `out` right before the packet at rows[ridx]: contiguous, logged in the packet's turn with no turn boundary
        between, and after the seat's previous action in raw order (a DNA's material picks, written after its action,
        are not a later play's picks)."""
        turn = rows[ridx].get("turn")
        floor = last_action_src(seat)
        found = []
        for row in reversed(out):
            if len(found) == limit:
                break
            src = src_of.get(id(row), -1)
            if not (row.get("type") == "selection" and row.get("prompt") == "SelectPermanentEffect"
                    and row.get("actor") == seat and not row.get("synthesized") and row.get("targets")
                    and all(t.get("player") == seat for t in row["targets"])):
                break
            if src <= floor or rows[src].get("turn") != turn or any(
                    ("turn" in x and x["turn"] != turn)
                    or x.get("method") in ("TurnStateMachine.Init", "TurnStateMachine.SetBreedingPhase")
                    for x in rows[src + 1:ridx]):
                break
            found.append(row)
        return found[::-1]

    def material_picks(seat, ridx):
        """The rows a local seat's DNA root selection logged: DCGO asks both roots (SelectJogressEffect, own side) and
        queues the play from that selection's end coroutine (TurnStateMachine.cs:2969-2983), so they are the seat's two
        own-side SelectPermanentEffect rows immediately before its play packet (`own_picks_before`)."""
        return own_picks_before(seat, ridx, 2)

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
    # Breeding-area state per seat, for SetBreedingPhase(doBreeding=true): hatch when the area is empty, move its
    # Digimon when occupied (CR 4.3 6-4-1-1 / 6-4-1-2; DCGO TurnStateMachine.cs:1058-1068 hatches when CanHatch, which
    # needs an empty breeding area, Player.cs:1172, and moves otherwise). The decisions themselves fill and empty it, and
    # so do EFFECTS (g6: BT20-095's <Delay> moved Ryudamon BT20-010 out during the other seat's turn), so the rows'
    # boards correct it (`field_board`, `battle_board`, the play rows). The area holds at most one card (CR 4.3 3-4-7-2).
    #   occ[p]     True / False, or None = unknown (unknown_why[p] says why; a doBreeding then needs `breeding_outcome`)
    #   top[p]     the card ids the breeding card's top can be (several when a row cannot say which permanent it
    #              changed); empty = not known (a hatched Digi-Egg)
    #   copies[p]  {id: copies of that id in p's BATTLE area while the breeding area holds it}, or None when not
    #              anchored: a battle board showing more copies than that is the breeding card having moved there or a
    #              copy entering unseen, which no board tells apart
    occ = {0: False, 1: False}
    top = {0: set(), 1: set()}
    copies = {0: None, 1: None}
    unknown_why = {0: None, 1: None}
    pending_owner = {}  # a breeding-area play whose row names no owner: {"card": id, "saved": {seat: state}}

    def describe(v):
        return "unknown" if v is None else ("occupied" if v else "empty")

    def set_breeding(p, occupied, ids=(), why="", battle=None):
        if occ[p] != occupied:
            inferred.append("P%d breeding area %s: %s (it was %s)" % (p, describe(occupied), why, describe(occ[p])))
        occ[p] = occupied
        top[p] = set(ids) if occupied is not False else set()  # unknown: what it holds IF it is occupied
        unknown_why[p] = why if occupied is None else None
        copies[p] = {c: battle.count(c) for c in top[p]} if occupied and top[p] and isinstance(battle, list) else None

    def field_board(p, fp, where):
        """A GetFieldPermanents() board of seat p: battle AND breeding permanents in FrameID order (Player.cs:669-684),
        the breeding frame being the last FrameID (Player.cs:41-54, isBattleAreaFrameID Player.cs:1561-1563). So the
        breeding permanent, when there is one, is the board's LAST entry: a candidate top card elsewhere on the board
        (a moved Digimon, another copy) says nothing."""
        pend = pending_owner.get("card")
        if pend is not None and p in pending_owner["saved"] and (not fp or fp[-1] != pend):
            # not this seat's breeding card: the unnamed breeding-area play went to the other seat
            occ[p], top[p], copies[p], unknown_why[p] = pending_owner["saved"].pop(p)
            if len(pending_owner["saved"]) == 1:
                (q,) = pending_owner["saved"]
                set_breeding(q, True, {pend}, "%s, played into a breeding area of a seat the row does not name, is "
                                              "not P%d's breeding card (%s)" % (pend, p, where))
                pending_owner.clear()
        if not fp:
            if occ[p] is not False:
                set_breeding(p, False, why="P%d's field is empty (%s)" % (p, where))
            return
        last = fp[-1]
        if is_egg(cards, last):
            # a Digi-Egg has no DP and is never moved (CR 4.3 4-17-2): it can only be the breeding card
            if not (occ[p] is True and top[p] == {last}):
                set_breeding(p, True, {last}, "Digi-Egg %s ends P%d's field board (%s)" % (last, p, where), [])
            return
        if occ[p] is not False and top[p]:
            if last not in top[p]:
                # (also settles an unknown area: the one way it could still be occupied is by one of these, last here)
                set_breeding(p, False, why="%s is not the last permanent on P%d's field board, where the breeding one "
                                           "is (%s)" % ("/".join(sorted(top[p])), p, where))
            elif occ[p] is True:
                # consistent: if the area holds it, the other copies on the board are in the battle area
                top[p] = {last}
                copies[p] = {last: fp.count(last) - 1}

    def battle_board(p, b, where):
        """A GetBattleAreaPermanents() board of seat p (no breeding permanent)."""
        if occ[p] is not True or not top[p]:
            return
        if copies[p] is None:
            seen = sorted(c for c in top[p] if c in b)
            if not seen:
                copies[p] = {c: 0 for c in top[p]}
                return
            set_breeding(p, None, top[p], why="%s is in P%d's battle area (%s) and no earlier board anchored how many copies of "
                                      "it were there: the breeding one may have moved there" % (seen[0], p, where))
            return
        for c in sorted(top[p]):
            n = b.count(c)
            if n > copies[p].get(c, 0):
                set_breeding(p, None, top[p], why="P%d's battle area shows %d %s (%s), more than the %d it held while its "
                                          "breeding area held %s: the breeding one moved there (BT20-095's <Delay>, a "
                                          "breeding-area effect) or another copy entered by a row that does not name "
                                          "its owner; the boards cannot tell" % (p, n, c, where, copies[p].get(c, 0), c))
                return
            copies[p][c] = n

    def entered_battle(p, card, replaced=None):
        """A play of seat p's put `card` on top of a battle-area permanent (whose top was `replaced`) or into a new one."""
        if p in (0, 1) and occ[p] is True and copies[p] is not None:
            if replaced in copies[p] and copies[p][replaced] > 0:
                copies[p][replaced] -= 1
            if card in copies[p]:
                copies[p][card] += 1

    def play_row_breeding(r, tp):
        """What a PlayCardClass row (logged at construction, before it resolves: its boards are the state before the
        play) says of the breeding areas."""
        where = "seq %s" % r.get("seq")
        boards = r.get("boards")
        boards = boards if isinstance(boards, list) and len(boards) == 2 and all(
            isinstance(x, list) for x in boards) else None
        if boards is not None:
            for p in (0, 1):
                battle_board(p, boards[p], where)
        played = (r.get("cards") or [None])[0]
        if not played:
            return
        tgt = r.get("target") if isinstance(r.get("target"), dict) else None
        tid = tgt.get("card_id") if tgt else None
        owner = (r.get("hand") or {}).get("player", -1)
        if r.get("breeding_area") is True:
            # PlayCardClass._isBreedingArea: the card is played INTO its owner's breeding area
            if owner in (0, 1):
                set_breeding(owner, True, {played}, "%s played into it (%s)" % (played, where))
            else:
                pending_owner.clear()
                pending_owner.update({"card": played, "saved": {
                    s: (occ[s], set(top[s]), copies[s], unknown_why[s]) for s in (0, 1)}})
                for s in (0, 1):
                    set_breeding(s, None, {played}, why="%s was played into a breeding area and the row does not name whose "
                                              "(%s)" % (played, where))
            return
        if tgt is None or tp not in (0, 1):
            entered_battle(owner, played)
            return
        if r.get("target_breeding") is True:
            # the mod's TargetSlotJson found the target in the TURN player's breeding area: it stays, under a new top
            set_breeding(tp, True, {played}, "%s digivolved in it (%s)" % (played, where),
                         boards[tp] if boards else None)
            return
        if r.get("target_slot", -1) >= 0:
            entered_battle(tp, played, tid)
            return
        # TargetSlotJson looks among the TURN player's permanents only (Core.cs, TurnPlayerId()): a target it did not find
        # is the other seat's, in its battle area or its breeding area
        o = 1 - tp
        if boards is not None and tid not in boards[o]:
            # not in that seat's battle area: the target is its breeding Digimon
            set_breeding(o, True, {played}, "%s digivolved P%d's breeding %s during P%d's turn (%s)" % (
                played, o, tid, tp, where), boards[o])
        elif occ[o] is True and tid in top[o]:
            # a breeding top card's id that is also in the battle area: either permanent may be the target; the area
            # stays occupied either way, by one of the two
            top[o] = {tid, played}
            copies[o] = ({tid: boards[o].count(tid), played: boards[o].count(played) + 1}
                         if boards is not None else None)
        else:
            entered_battle(o, played, tid)

    def breeding_outcome(p, start):
        """A doBreeding whose area's occupancy is unknown: HATCH when the rows after it show a Digi-Egg in p's breeding
        area before anything else could have put one there. A Digi-Egg has no DP and is never moved (CR 4.3 4-17-2);
        a MOVE leaves the area empty, and only a hatch or a play INTO the area fills it. Nothing here proves a MOVE:
        that stays undecided.

        A play INTO the area made by an effect goes through PlayPermanentClass, which the mod does not hook
        (raw-20261113T025510Z#g5 seq 834: P1's field board ends with Pinamon (BT20-004), which no logged row put
        there). So a Digi-Egg seen there proves the hatch only while no effect can have resolved since the decision:
        the first row that shows effect activity (an effect's selection, "use?" answer, trigger order, a play - whose
        [On Play] / [When Digivolving] effects follow it - a source placement, an attack or a [Main] activation) ends
        the evidence. Queueing a play packet runs nothing; a forced effect that resolves with no row at all stays
        invisible to this check."""
        def effect_may_have_run(x, mm):
            if mm == "TurnStateMachine.QueueMainPhaseAction_Internal":
                return len(x.get("args", [])) >= 2 and x["args"][1] in (ATTACK_PACKET_ID, ACTIVATE_PERMANENT_PACKET_ID)
            return (mm.startswith(("MultipleSkills.", "OptionalSkill.", "UserSelectionManager.", "SelectCardEffect.",
                                   "SelectHandEffect.", "SelectCountEffect.", "SelectAttackEffect.",
                                   "SelectPermanentEffect.", "Permanent.", "TurnStateMachine.SetAttackingPermaent"))
                    or (mm in ("PlayCardClass.ctor", "PlayCardClass.PlayCard")))

        for x in rows[start + 1:]:
            mm = x.get("method", "")
            if mm in ("TurnStateMachine.SetBreedingPhase", "TurnStateMachine.Init", "TurnStateMachine.EndGame"):
                return None
            if mm in ("PlayCardClass.ctor", "PlayCardClass.PlayCard"):
                if x.get("breeding_area") is True:
                    return None
                tid = (x.get("target") or {}).get("card_id") if isinstance(x.get("target"), dict) else None
                if x.get("target_breeding") is True and x.get("tp") == p:
                    if tid and is_egg(cards, tid):
                        return "%s digivolved onto Digi-Egg %s in P%d's breeding area (seq %s)" % (
                            (x.get("cards") or ["?"])[0], tid, p, x.get("seq"))
                    return None
            elif mm.startswith("Snapshot."):
                pl = next((q for q in x.get("players", []) if q.get("id") == p), None)
                if pl is not None and "breeding" in pl:
                    br = pl["breeding"]
                    return ("snapshot shows Digi-Egg %s in P%d's breeding area (seq %s)" % (br[0], p, x.get("seq"))
                            if br and is_egg(cards, br[0]) else None)
            elif mm == "SelectPermanentEffect.SetTargetFrames" or (
                    mm == "TurnStateMachine.QueueMainPhaseAction_Internal" and len(x.get("args", [])) >= 2
                    and x["args"][1] in (ATTACK_PACKET_ID, ACTIVATE_PERMANENT_PACKET_ID)):
                fb = x.get("boards")
                if isinstance(fb, list) and len(fb) == 2 and isinstance(fb[p], list) and fb[p] and is_egg(cards, fb[p][-1]):
                    return "Digi-Egg %s ends P%d's field board (seq %s)" % (fb[p][-1], p, x.get("seq"))
            if effect_may_have_run(x, mm):
                # an effect may now have resolved, and played a Digi-Egg into the area unlogged
                return None
        return None

    queued_play_indexes = set()  # card indexes of queued main-phase play packets not yet resolved
    # Seats with an `engine_action` stamp anywhere in the raw: agent-played seats. The mod stamps EVERY menu answer of
    # such a seat in order (Core.cs: AppendStamp for the local seat, ReadStamp from the stamp file for the other one),
    # -1 - no stamp - meaning the engine asked nothing. A seat with none is a human (or the remote seat without the
    # other client's stamp file).
    stamped_seats = {x["args"][0] for x in rows
                     if "engine_action" in x and isinstance(x.get("args"), list) and x["args"] and x["args"][0] in (0, 1)}
    # Which copy a board-position pick names, where the board holds several of that card (`frame_model`): DCGO's
    # frame placement followed from the logged plays and checked against every recorded board, or nothing.
    frame_names = frame_model.pick_identities(rows, cards, you)
    burst_names = frame_model.burst_tamers(rows, cards, you)
    for ridx, r in enumerate(rows):
        tag_sources(ridx - 1)
        m, a = r["method"], r.get("args", [])
        tp, turn = r.get("tp", -1), r.get("turn", r.get("turn_count", -1))
        if m.startswith("Snapshot."):
            pending_owner.clear()
            for pl in r.get("players", []):
                if "breeding" in pl and pl.get("id") in (0, 1):
                    # exact: the breeding area's permanents (top card ids) and the battle area's
                    br = pl["breeding"]
                    set_breeding(pl["id"], bool(br), set(br[:1]), "snapshot (seq %s)" % r.get("seq"),
                                 pl.get("battle"))
        elif (m == "SelectPermanentEffect.SetTargetFrames"
              or (m == "TurnStateMachine.QueueMainPhaseAction_Internal" and len(a) >= 2
                  and a[1] in (ATTACK_PACKET_ID, ACTIVATE_PERMANENT_PACKET_ID))):
            # These rows' boards are GetFieldPermanents() of both seats (dcgo-melon-recorder Core.cs SelectionExtras)
            boards = r.get("boards")
            if isinstance(boards, list) and len(boards) == 2:
                for p in (0, 1):
                    if isinstance(boards[p], list):
                        field_board(p, boards[p], "seq %s" % r.get("seq"))
        elif ((m == "TurnStateMachine.QueueMainPhaseAction_Internal" and len(a) >= 2 and a[1] == PLAY_PACKET_ID)
              or m == "TurnStateMachine.SetAttackingPermaent"):
            # These rows' boards are GetBattleAreaPermanents() of both seats (Core.cs PlayPacketJson / SelectionExtras)
            boards = r.get("boards")
            if isinstance(boards, list) and len(boards) == 2:
                for p in (0, 1):
                    if isinstance(boards[p], list):
                        battle_board(p, boards[p], "seq %s" % r.get("seq"))
        elif m in ("PlayCardClass.ctor", "PlayCardClass.PlayCard"):
            play_row_breeding(r, tp)

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
            # Human main-phase plays have a play packet naming the card; the built-in bot sets PlayCard directly
            # on its own turn. An unqueued paid play outside that bot path is an effect's play or digivolve,
            # whose selection rows carry the decision.
            hand_cards = (r.get("hand") or {}).get("cards") or []
            slot = (r.get("hand") or {}).get("slot", -1)
            played_index = hand_cards[slot].get("card_index") if 0 <= slot < len(hand_cards) else None
            listed_card = hand_cards[slot].get("card_id") if 0 <= slot < len(hand_cards) else None
            played_card = (r.get("cards") or [None])[0]
            if played_index is not None and listed_card is not None and played_card is not None \
                    and listed_card != played_card:
                # The listing NAMES a card at the slot and it is not the played card (frame_model._played_index checks
                # the same): that index belongs to another card, so the play is positional only. A listing with no
                # card_id (an older recorder build) or a row with a hand but no `cards` names nothing to compare, so
                # the index stays and the queued-play gate below decides (dropping it sent the row past that gate as
                # a positional play and left its queued index in the set).
                inferred.append("P%s play of %s (seq %s): hand slot %s lists %s, not the played card, so its card index "
                                "%s is not taken (positional only)" % (actor, played_card, r.get("seq"), slot,
                                                                       listed_card, played_index))
                played_index = None
            bot_paid_play = is_ai and actor == bot == tp and r.get("pay_cost") is True
            if played_index is not None and played_index not in queued_play_indexes and not bot_paid_play:
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
            is_dna = played_index is not None and played_index in dna_play_indexes
            digivolve_field = None
            if r.get("target") is not None and not is_dna:
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
            absent = dna_bytes_absent.pop(played_index, None) if played_index is not None else None
            if absent is True:
                truncated_at = truncated_at or (
                    "play of %s whose play packet bytes were not logged (an older recorder build), right after two "
                    "own-side picks of the kind a DNA's root selection logs: whether it is a DNA digivolve cannot be "
                    "told at step %d" % (card, step))
                break
            if absent is False:
                inferred.append("P%d play of %s at step %d: its play packet bytes were not logged (an older recorder "
                                "build), so it is taken as an ordinary play with no DNA check" % (actor, card, step))
            if played_index is not None and played_index in dna_unknown_indexes:
                truncated_at = truncated_at or ("play of %s whose play packet has no readable jogress list at step %d"
                                                % (card, step))
                break
            if is_dna:
                # A DNA (jogress) digivolve. DCGO asks the two material Digimon (SelectPermanentEffect, this seat,
                # own side) BEFORE it queues the play; the engine starts DNA from the hand card (DNA_DIGIVOLVE_START
                # + hand index) and then asks the first and second material. Put the action ahead of the material
                # picks already written, so the replay sees the engine's order. A REMOTE seat's root selection never
                # runs on this client (only its play packet arrives), so it has no picks: they are synthesised from
                # the packet's root FrameIDs when those place with certainty. A DNA action is never written without
                # both materials.
                dna_play_indexes.discard(played_index)
                if slot >= DNA_DIGIVOLVE_END - DNA_DIGIVOLVE_START:
                    truncated_at = truncated_at or ("DNA digivolve of %s from hand slot %d at step %d" % (card, slot, step))
                    break
                # The picks were found when the packet was queued (`material_picks`): rows logged between the packet
                # and this resolve do not hide them, and a remote seat has none (its selection ran on its own client).
                pick_rows = [x for x in dna_picks.pop(played_index, []) if any(x is y for y in out)]
                picks = len(pick_rows)
                # Place the packet's roots: anchored on its TargetFrameID, whose permanent this resolve row logs at
                # compact `target_slot` in the resolution-time battle area `boards` (TurnStateMachine.cs:1819 reads
                # the same frame; the roots resolve against the same frames, CardController.cs:540).
                roots, anchor_frame = dna_roots.pop(played_index, (None, None))
                boards = r.get("boards")
                anchor_slot = r.get("target_slot")
                placed, why = None, "the play packet's jogress roots could not be read"
                if roots is not None:
                    why = "jogress roots %s have no resolution-time anchor (target_slot / boards) for TargetFrameID %s" % (
                        roots, anchor_frame)
                    if (actor == tp and isinstance(anchor_slot, int) and r.get("target_breeding") is not True
                            and isinstance(boards, list) and len(boards) == 2 and isinstance(boards[actor], list)):
                        placed = jogress_root_slots(roots, anchor_frame, anchor_slot, len(boards[actor]))
                        why = "jogress roots %s are not certain from TargetFrameID %s at slot %s of %d" % (
                            roots, anchor_frame, anchor_slot, len(boards[actor]))
                if picks == 2:
                    picked = [t.get("frame") for row in pick_rows for t in row.get("targets") or []]
                    if placed is not None and picked != placed:
                        truncated_at = truncated_at or (
                            "DNA digivolve of %s: material picks %s contradict the jogress roots %s (slots %s) at step %d"
                            % (card, picked, roots, placed, step))
                        break
                elif picks == 1:
                    truncated_at = truncated_at or ("DNA digivolve of %s with a single material pick at step %d"
                                                    % (card, step))
                    break
                elif placed is None:
                    truncated_at = truncated_at or ("DNA digivolve of %s with no material picks: %s at step %d"
                                                    % (card, why, step))
                    break
                at = next(i for i, x in enumerate(out) if x is pick_rows[0]) if picks else None
                if infer and not bot_bred:
                    # the bot's inferred breeding decision comes before this play and the picks DCGO asked for it
                    breeding_for_bot(ridx, turn)
                    bot_bred = True
                    if at is not None:
                        move_last_before(len(out) - 1 - at)
                        at += 1
                action(actor, DNA_DIGIVOLVE_START + slot, "Main", "main_phase", card_id=card, boards=boards,
                       card_index=played_index)
                if at is not None:
                    move_last_before(len(out) - 1 - at)  # ahead of its picks (and of rows logged after them)
                if not picks:
                    for frame in placed:
                        out.append({"type": "selection", "step": step, "actor": actor, "prompt": "SelectPermanentEffect",
                                    "phase": "Main", "board_p0": boards[0], "board_p1": boards[1],
                                    "targets": [{"player": actor, "frame": frame}], "synthesized": "jogress_roots"})
                        step += 1
                    inferred.append("P%d DNA %s materials from its jogress roots %s -> slots %s" % (
                        actor, card, roots, placed))
                continue
            burst = burst_plays.pop(played_index, None) if played_index is not None else None
            burst_named = None  # the frame model's naming of a REMOTE seat's Tamer, when there is no logged pick
            if burst is not None:
                burst_frame, burst_pick, burst_named, burst_boards = burst
                if burst_pick is None and burst_named is not None and digivolve_field is not None:
                    seat_, position, tamer_id, tamer_index = burst_named
                    board = burst_boards[seat_] if isinstance(burst_boards, list) and len(burst_boards) == 2 else None
                    if tamer_index is None:
                        tamer_index = returned_tamer_index(rows, ridx, seat_, tamer_id)
                        index_source = "hand_listing"  # derived from the owner's next hand listing
                    else:
                        index_source = "frame_model"
                    ambiguous = isinstance(board, list) and board.count(tamer_id) >= 2
                    if (seat_ != actor or not isinstance(board, list) or not 0 <= position < len(board)
                            or board[position] != tamer_id or (ambiguous and tamer_index is None)):
                        burst_named = None
                    else:
                        burst_named = (seat_, position, tamer_id, tamer_index, index_source, board)
                else:
                    burst_named = None
                if (burst_pick is None and burst_named is None) or (
                        burst_pick is not None and not any(burst_pick is x for x in out)) or digivolve_field is None:
                    truncated_at = truncated_at or (
                        "Burst Digivolve of %s (BurstTamerFrameID %d) at step %d: no pick of one of P%d's own Tamers "
                        "right before the play packet and after P%d's previous action names the returned Tamer (a "
                        "remote seat's SelectBurstDigivolutionEffect pick is local to its own client), and the frame "
                        "model is not certain which permanent that FrameID names (it must see every one of the seat's "
                        "permanents played and explain every recorded board, the frame must hold a Tamer, and one "
                        "copy among several needs DCGO's CardIndex); the recorder would need to log the Tamer's "
                        "compact battle-area index or its CardIndex (Core.cs: the play packet's `play` object)"
                        % (card, burst_frame, step, actor, actor))
                    break
            if infer and not bot_bred:
                breeding_for_bot(ridx, turn)
                bot_bred = True
            if digivolve_field is not None:
                action_id = DIGIVOLVE_START + slot * FIELDS_PER_HAND + digivolve_field
            else:
                action_id = PLAY_HAND_START + slot
            operand_k = None
            tgt = r.get("target")
            if digivolve_field is not None and digivolve_field != BREEDING_TARGET and isinstance(tgt, dict):
                # The digivolve target's own CardIndex (the permanent's top card, CardJson(target.TopCard)): only
                # worth naming when the board holds more than one copy of it, and only when the row's board agrees
                # that it is the card at the target slot
                tk, board = tgt.get("card_index"), (r.get("boards") or [None, None])[actor] if r.get("boards") else None
                exact = r.get("board_indexes")
                if (isinstance(tk, int) and not isinstance(tk, bool) and isinstance(board, list)
                        and 0 <= digivolve_field < len(board) and board[digivolve_field] == tgt.get("card_id")
                        and board.count(tgt.get("card_id")) >= 2
                        and not (isinstance(exact, list) and len(exact) == 2 and isinstance(exact[actor], list)
                                 and len(exact[actor]) == len(board) and exact[actor][digivolve_field] != tk)):
                    operand_k = tk
            action(actor, action_id, "Main", "main_phase", card_id=card, boards=r.get("boards"), card_index=played_index,
                   operand_card_index=operand_k)
            if burst is not None and burst_named is not None:
                # A REMOTE seat's pick ran on its own client and left no row: write it from the frame model's certain
                # naming of the packet's BurstTamerFrameID, right after the digivolve action (where the engine asks it).
                seat_, position, tamer_id, tamer_index, index_source, board = burst_named
                target = {"player": seat_, "frame": position}
                if tamer_index is not None:
                    target["card_index"] = tamer_index
                    target["card_index_source"] = index_source
                row = {"type": "selection", "step": step, "actor": actor, "prompt": "SelectPermanentEffect",
                       "phase": "Main", "board_p0": burst_boards[0], "board_p1": burst_boards[1],
                       "targets": [target], "mechanic": "burst", "synthesized": "burst_tamer_frame"}
                out.append(row)
                step += 1
                inferred.append(
                    "P%d Burst Digivolve of %s: its returned Tamer %s (battle position %d%s) was inferred from the play "
                    "packet's BurstTamerFrameID %d by the frame model (the seat's own pick ran on its client only)" % (
                        actor, card, tamer_id, position,
                        ", CardIndex %d via %s" % (tamer_index, index_source) if tamer_index is not None else "",
                        burst_frame))
            elif burst is not None:
                # ahead of the Tamer pick, which answers the engine's burst cost route; `mechanic` marks the pick as
                # the burst's Tamer (the schema's alt-cost declaration field), never an effect's own pick
                burst_pick["mechanic"] = "burst"
                # The pick names the Tamer by board position only, and a board can hold several copies of it
                # (raw-20261113T025510Z#g6: three ST24-13). The returned card's DCGO CardIndex, read from the owner's
                # next hand listing, names the copy (`returned_tamer_index`); the replay resolves it by identity.
                t = burst_pick["targets"][0]
                board = burst_pick.get("board_p%d" % actor) or []
                if 0 <= t.get("frame", -1) < len(board):
                    k = returned_tamer_index(rows, ridx, actor, board[t["frame"]])
                    if k is not None:
                        t["card_index"] = k
                        t["card_index_source"] = "hand_listing"  # derived from the owner's next hand listing
                at = next(i for i, x in enumerate(out) if x is burst_pick)
                move_last_before(len(out) - 1 - at)
            continue
        if m == "TurnStateMachine.SetBreedingPhase":
            if a[1]:
                # doBreeding: hatch when the breeding area is empty, otherwise move its Digimon to the battle area
                # (DCGO's own rule in SetBreedingPhase). Occupancy: snapshots, boards and plays above, else the
                # decisions' own effect.
                p = a[0]
                if occ.get(p) is None:
                    seen = breeding_outcome(p, ridx)
                    if seen is None:
                        truncated_at = truncated_at or (
                            "P%d's breeding decision at step %d (seq %s): HATCH or MOVE cannot be told - the breeding "
                            "area's occupancy is unknown (%s) and no later row shows a Digi-Egg there before an effect "
                            "could have resolved - an effect's play into the breeding area (PlayPermanentClass) is "
                            "unlogged, so an egg seen after one proves nothing. The recorder "
                            "would settle it by logging GetBreedingAreaPermanents() of both seats beside every `boards` "
                            "payload (Core.cs SelectionExtras / TargetSlotJson / PlayPacketJson), and TargetSlotJson "
                            "would place a non-turn player's target if it searched the target's owner instead of "
                            "TurnPlayerId()" % (p, step, r.get("seq"), unknown_why.get(p)))
                        break
                    inferred.append("P%d breeding HATCH at step %d: its occupancy was unknown, then %s" % (p, step, seen))
                    moving = False
                else:
                    moving = occ[p] is True
                action(p, ACTION_MOVE if moving else ACTION_HATCH, "Breeding", "breeding_action")
                occ[p], top[p], copies[p], unknown_why[p] = (not moving), set(), None, None
            else:
                action(a[0], ACTION_PASS, "Breeding", "breeding_action")
            continue
        if m == "TurnStateMachine.QueueMainPhaseAction_Internal":
            if isinstance(r.get("play"), dict) and r["play"].get("card_index") is not None:
                ci = r["play"]["card_index"]
                queued_play_indexes.add(ci)
                # PlayCardAction bytes: card index, frame, then int[] JogressEvoRootsFrameIDs (count + ids). A
                # non-empty jogress list is a DNA digivolve (DCGO ActionEncoder.cs reads the same field).
                # The queue logs ATTEMPTS (a DNA packet DCGO rejects never resolves), so the LATEST packet for a card
                # decides: an empty list clears an earlier DNA flag, and a list that cannot be read leaves the play's
                # kind unknown (the resolve then stops rather than guess).
                # Bytes ABSENT (older recorder builds logged the payload as a type name, or not at all) differ from
                # bytes present but unreadable: the first is an ordinary play unless the rows before the packet look
                # like a DNA's root picks, the second stops at the resolve.
                bytes_logged = len(a) >= 3 and isinstance(a[2], list)
                b = a[2] if bytes_logged else []
                n_roots = (b[8] << 24) | (b[9] << 16) | (b[10] << 8) | b[11] if len(b) >= 12 else -1
                seat = a[0] if a and a[0] in (0, 1) else tp
                # DCGO asks the roots on the deciding client only (SetUp_SelectDigivolutionRoots isLocal: true,
                # TurnStateMachine.cs:2969): this client logged them for its own seat (and both seats of an AI game)
                roots_logged_here = is_ai or seat == you
                dna_play_indexes.discard(ci)
                dna_unknown_indexes.discard(ci)
                dna_roots.pop(ci, None)
                dna_picks.pop(ci, None)
                dna_bytes_absent.pop(ci, None)
                burst_plays.pop(ci, None)
                # BurstTamerFrameID follows the jogress list (PlayCardAction: card index, TargetFrameID, int[] jogress,
                # BurstTamerFrameID, int[] app fusion). A Burst Digivolve's Tamer is picked BEFORE the packet is
                # queued: the human UI asks it (SelectBurstDigivolutionEffect.SelectTamer, a local
                # SelectPermanentEffect over the seat's own Tamers, SelectBurstDigivolutionEffect.cs:107-190) and
                # queues the play from that pick's end coroutine (TurnStateMachine.cs:3016-3030), so the pick is the
                # seat's row right before the packet. The engine asks it AFTER the digivolve action, as one of its
                # digivolution cost routes, so the resolve writes the action ahead of it. A remote seat's pick never
                # ran on this client (SetIsLocal): only the raw FrameID is known.
                burst_at = 12 + 4 * n_roots
                if a[1] == PLAY_PACKET_ID and bytes_logged and 0 <= n_roots < 0x80000000 and len(b) >= burst_at + 4:
                    burst = (b[burst_at] << 24) | (b[burst_at + 1] << 16) | (b[burst_at + 2] << 8) | b[burst_at + 3]
                    burst = burst - (1 << 32) if burst & 0x80000000 else burst
                    if burst >= 0:
                        # The seat's own-side pick right before the packet, made after its previous action
                        # (`own_picks_before`), naming one permanent that is a Tamer of that seat: the burst's Tamer
                        # pick offers the seat's own Tamers only (SelectBurstDigivolutionEffect.cs:107-190).
                        run = own_picks_before(seat, ridx, 1) if roots_logged_here else []
                        last = run[0] if run else {}
                        tgt = (last.get("targets") or [{}])[0] if len(last.get("targets") or []) == 1 else {}
                        board = last.get("board_p%d" % seat) or []
                        picked = board[tgt["frame"]] if 0 <= tgt.get("frame", -1) < len(board) else None
                        pick = last if picked is not None and is_tamer(cards, picked) else None
                        # a REMOTE seat's pick never ran on this client: the frame model may still name the permanent
                        # its FrameID points at (certain or None, never a guess)
                        named = None if roots_logged_here else burst_names.get(ridx)
                        burst_plays[ci] = (burst, pick, named, r.get("boards"))
                if a[1] == PLAY_PACKET_ID and not bytes_logged:
                    dna_bytes_absent[ci] = roots_logged_here and len(material_picks(seat, ridx)) == 2
                elif a[1] != PLAY_PACKET_ID or not 0 <= n_roots < 0x80000000:
                    dna_unknown_indexes.add(ci)
                elif n_roots > 0:
                    dna_play_indexes.add(ci)
                    dna_picks[ci] = material_picks(seat, ridx) if roots_logged_here else []
                    # the root FrameIDs follow the count; the packet's TargetFrameID (bytes 4-7) is their anchor
                    def s32(i):
                        v = (b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]
                        return v - (1 << 32) if v & 0x80000000 else v
                    roots = [s32(12 + 4 * k) for k in range(n_roots)] if len(b) >= 12 + 4 * n_roots else None
                    dna_roots[ci] = (roots, s32(4))
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
                # DCGO's skill index is POSITIONAL (into the permanent's OnDeclaration effects); the engine's
                # sub-slot is semantic: 2 = [Main], 3 = <Link>. The mod logs which kind it named (activate_link);
                # rows from before that field keep the old positional reading.
                if "activate_link" in r:
                    skill = 3 if r["activate_link"] else 2
                action(a[0], FIELD_EFFECT_START + perm * EFFECTS_PER_PERMANENT + skill, "Main", "main_phase",
                       boards=r.get("boards"), operand_card_index=board_operand_index(r, a[0], perm))
                continue
            if a[1] == ACTIVATE_CARD_PACKET_ID and len(a) >= 3 and isinstance(a[2], list) and len(a[2]) >= 8:
                # ActivateCardAction(card index, skill index): a HAND card's [Main] ("[Hand] [Main] ..."). The engine's
                # action is HAND_EFFECT_START + the card's current hand slot (action/space.rs), not the deck-order card
                # index the packet carries. The slot is read from the hand listing of the PlayCard row that follows
                # (the listing names every hand card with its CardIndex); an activation whose card never plays has no
                # such listing, so it truncates rather than guessing a slot.
                ci, skill = _s32(a[2], 0), _s32(a[2], 4)
                slot, listed = hand_slot_of_card_index(rows, ridx, a[0], ci)
                if skill != 0 or slot is None or not (0 <= slot < HAND_EFFECT_END - HAND_EFFECT_START):
                    truncated_at = truncated_at or (
                        "activate packet for hand card index %d (skill %d) at step %d: its hand slot is not named by "
                        "a following play listing" % (ci, skill, step))
                    break
                action(a[0], HAND_EFFECT_START + slot, "Main", "main_phase", card_id=listed, card_index=ci)
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
            action(actor, ATTACK_START + a[0] * TARGETS_PER_ATTACKER + target, "Main", "main_phase", boards=r.get("boards"),
                   operand_card_index=board_operand_index(r, actor, a[0]),
                   attack_target_card_index=board_operand_index(r, 1 - actor, a[1]))
            continue
        if m in ("TurnStateMachine.QueueMainPhaseAction", "TurnStateMachine.PassTurn"):
            continue  # the Internal row above already carries the decision
        if m == "SelectCardEffect.SetTargetCardAndIndicies" and r.get("stray"):
            continue  # a pick DCGO asked among identical candidates and the engine did not (the mod marks it)
        if m == "SelectCardEffect.SetTargetCardAndIndicies" and "card_ids" in r:
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectCardEffect", "phase": "Main"}
            if r["card_ids"] and None not in r["card_ids"]:
                row["card_ids"] = r["card_ids"]
                indexes = pick_card_indexes(a, r["card_ids"])
                if indexes is not None:
                    row["card_indexes"] = indexes
                # SetTargetCardAndIndicies(playerID, int[] cardIndexes, int[] positions): the third list is the
                # position of each pick in the candidate list DCGO showed. The replay checks it against the
                # number of entries DCGO builds for a refired effect (ReplayDriver::
                # refire_duplicate_effect_pick_fold); no other reader uses it.
                positions = pick_list_positions(a, r["card_ids"])
                if positions is not None:
                    row["indexes"] = positions
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
            if "engine_action" in r:
                row["engine_action"] = r["engine_action"]
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
            # A declined immediate effect during the recorded Arts evolution
            # happens before DCGO adds the chosen card to the stack. The
            # simulator's Arts operation is atomic and has already completed;
            # this explicit decline has no state change to apply there.
            if (a[1] is False and ridx > 0 and ridx + 1 < len(rows)
                    and rows[ridx - 1].get("method") == "PlayCardClass.PlayCard"
                    and rows[ridx - 1].get("cards") == ["BT26-050"]
                    and (rows[ridx - 1].get("target") or {}).get("card_id") == "BT26-049"
                    and rows[ridx + 1].get("method") == "Permanent.AddCardSource"
                    and isinstance(rows[ridx + 1].get("args"), list)
                    and rows[ridx + 1]["args"]
                    and isinstance(rows[ridx + 1]["args"][0], dict)
                    and rows[ridx + 1]["args"][0].get("card_id") == "BT26-050"):
                inferred.append("declined immediate effect before BT26-050 Arts evolution (raw seq %s)" % r.get("seq"))
                continue
            # Preserve human and remote answers even when no engine action was
            # stamped. The prompt name distinguishes this activation decision
            # from a generic yes/no menu during replay.
            if "engine_action" in r:
                out.append({"type": "selection", "step": step, "actor": a[0], "prompt": "generic_bool",
                            "phase": "Main", "bool_value": bool(a[1]), "engine_action": r["engine_action"]})
                step += 1
            elif a[0] in (0, 1) and (a[0] not in stamped_seats or not a[1]):
                # An answer with no stamp that is NOT the agent's own "Use". Keep it, marked unstamped; the replay
                # applies it only while an optional accept gate of that seat is parked, and otherwise consumes it with
                # a note or halts where it would be lost (ReplayDriver::classify_unstamped_answer). Two cases:
                #  * a seat that is never stamped (a human, or the REMOTE seat with no stamp file): both answers.
                #    Dropping them lost declines - P0 refusing EX12-070's replacement - which the replay's
                #    implied-accept folds then turned into a "yes".
                #  * an UNSTAMPED FALSE of a stamped (agent) seat. Core.cs stamps an agent's decline as 62 and leaves
                #    only its "Use" answer unstamped, so this "No" did not come from the agent: it came from DCGO's AI,
                #    a StallWatch timeout or a human, and an agent click of "decline" on an effect the engine resolved
                #    without a choice is stamped -1 (Core.cs:2184-2187), which is divergence evidence. ReadStamp also
                #    returns -1 after its 0.5 s timeout while still advancing the stamp index (Core.cs:718-736), so a
                #    slow remote answer shifts later stamps. Never drop a false.
                out.append({"type": "selection", "step": step, "actor": a[0], "prompt": "generic_bool",
                            "phase": "Main", "bool_value": bool(a[1]), "unstamped": True})
                step += 1
            elif a[0] in (0, 1):
                # An agent-played seat's unstamped TRUE: its agent's own "Use" (Core.cs stamps only the declines),
                # which the engine folds into the next pick or resolved without a prompt. Not a decision to replay,
                # but recorded with its seq so the drop is auditable.
                inferred.append("P%d unstamped OptionalSkill answer %s (seq %s) dropped: P%d's answers are stamped "
                                "elsewhere in this raw and only a decline is stamped, so this agent 'Use' is not a "
                                "separate decision the engine asked" % (a[0], a[1], r.get("seq"), a[0]))
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
            # BT26-029's recorded inherited trigger is De-Digivolve 1.
            # After its target is chosen, DCGO records the forced count 1;
            # there is no zero or larger declaration for that keyword.
            if (a[1] == 1 and len(out) >= 2
                    and out[-1].get("prompt") == "SelectPermanentEffect"
                    and out[-1].get("targets")
                    and out[-1].get("actor") == a[0]
                    and out[-2].get("prompt") == "MultipleSkills"
                    and out[-2].get("chosen_effect_card") == "BT26-029"
                    and out[-2].get("actor") == a[0]):
                inferred.append("forced De-Digivolve 1 count after BT26-029 target (raw seq %s)" % r.get("seq"))
                continue
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
            # a declined pick sends null arrays (SetTargetFrames(pid, null, null), ST23-12's optional "you may"
            # declined with the back button): no targets, a cancel
            targets = [{"player": tp if is_tp else 1 - tp, "frame": idx} for is_tp, idx in zip(a[1] or [], a[2] or [])]
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectPermanentEffect",
                   "phase": "Main"}
            if r.get("boards") and len(r["boards"]) == 2:
                # DCGO's own field order at the pick (GetFieldPermanents); the replay maps it onto engine slots
                row["board_p0"], row["board_p1"] = r["boards"][0], r["boards"][1]
            for t in targets:
                # A board can hold several copies of the picked card; DCGO's CardIndex names the copy. A recorder
                # 0.2.0 row logs it per board entry (`board_indexes`, Core.cs BoardsJson); older rows get it from the
                # frame model when that is certain. The replay resolves a named copy by identity.
                board = (r.get("boards") or [None, None])[t["player"]] if r.get("boards") else None
                if (not isinstance(board, list) or not 0 <= t["frame"] < len(board)
                        or board.count(board[t["frame"]]) < 2):
                    continue
                exact = r.get("board_indexes")
                k, source = None, None
                if (isinstance(exact, list) and len(exact) == 2 and isinstance(exact[t["player"]], list)
                        and len(exact[t["player"]]) == len(board)):
                    k = exact[t["player"]][t["frame"]]
                    source = "recorder"
                if not isinstance(k, int) or isinstance(k, bool):
                    k = frame_names.get(ridx, {}).get((t["player"], t["frame"]))
                    source = "frame_model"
                if k is not None:
                    t["card_index"] = k
                    # which of the two grounded it: a recorder-logged index is DCGO's own, a frame-model one is derived
                    # (DCGO's frame placement followed from the logged plays)
                    t["card_index_source"] = source
            if targets:
                row["targets"] = targets
            else:
                row["cancel"] = True
            out.append(row)
            step += 1
            continue
        if m == "SelectAttackEffect.SetAttackTarget":
            # (attackOwnerID, targetIsTurnPlayer, permanentIndex): the boolean identifies the TARGET owner,
            # as set from selectedPermanent.TopCard.Owner in DCGO SelectAttackEffect.cs. -2 (NopIndex) =
            # "Not Attack"; -1 (SecurityIndex) = the player; otherwise the defender's compact battle-area index,
            # turn-relative like SetTargetFrames. Same row the Unity recorder writes (DCGO_RECORDING_SCHEMA.md).
            if tp not in (0, 1) or len(a) < 3:
                truncated_at = truncated_at or ("SelectAttackEffect without a turn player at step %d" % step)
                break
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectAttackEffect", "phase": "Main"}
            if a[2] == -2:
                row["cancel"] = True
            elif a[2] < 0:
                row["targets"] = [{"player": 1 - tp, "frame": -1}]
            else:
                row["targets"] = [{"player": tp if a[1] else 1 - tp, "frame": a[2]}]
            out.append(row)
            step += 1
            continue
        if m == "SelectHandEffect.SetTargetHandCards" and "card_ids" in r:
            row = {"type": "selection", "step": step, "actor": a[0], "prompt": "SelectHandEffect", "phase": "Main",
                   "zone": "Hand"}
            if r["card_ids"] and None not in r["card_ids"]:
                row["card_ids"] = r["card_ids"]
                indexes = pick_card_indexes(a, r["card_ids"])
                if indexes is not None:
                    row["card_indexes"] = indexes
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
