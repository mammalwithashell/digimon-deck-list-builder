"""Which permanent a recorded board position holds, from DCGO's own frame placement (raw rows only).

A recorded board (`boards`, Core.cs BoardsJson) lists a seat's permanents by card id, in `GetFieldPermanents()` order:
`FieldPermanents` walked by frame index (Player.cs:669-684). Two copies of a card are told apart only by their frames,
and a permanent keeps its frame while it stays on the field:

* a fresh play takes `CardSource.PreferredFrame()` - main-phase plays too: PlayCardClass hands the card to
  PlayPermanentClass, which never reads the packet's TargetFrameID (CardController.cs:1221, :1533-1541);
* PreferredFrame is the first EMPTY battle-area frame in an order fixed by the card's kind, the seat (local or not)
  and the scene layout (CardSource.cs:2340-2410). The frame parents are BattleAreaFrameParent's children in order
  (Player.cs:15-56); their positions are BattleScene.unity's YourPermanentFrame / OpponentPermanentFrame children
  (anchoredPosition, anchors and pivot all centred, the parents' GridLayoutGroup disabled, and no script moves them);
* a digivolve keeps the root's frame; the hybrid re-row after a play (CardController.cs:1811-1856) and the breeding
  -> battle move (CardObjectController.cs:1149-1197) move a permanent, both to PreferredFrame;
* anything else that changes a board (a deletion, a bounce, a card trashed off the top) shows up as the next board.

The model follows each seat's frames from the plays the raw logged (each permanent tagged with DCGO's CardIndex when a
row names it: the play packet, a 0.2.0 `card_indexes`, or the played hand slot) and checks EVERY recorded board of
that seat against it, keeping each hypothesis a board allows (which copy left, whether the breeding area moved, a
Digimon's top card changed in place). An Option that shows up with no play row (a Delay Option placed by its own
effect, CardEffectCommons.PlaceDelayOptionCards, which never reaches the recorder's PlayCardClass hook) is explained
only when nothing else does and nothing else left: it takes PreferredFrame like any play, its CardIndex unknown. A pick's position is named only when every hypothesis puts the same tagged
permanent there. When a board matches no hypothesis, the hypotheses outgrow a bound, a card's kind is unknown, or a
DNA / App Fusion packet places a permanent by TargetFrameID (DNADigivolveEffects.cs:33, not modelled), the model is
off for that seat for the rest of the game: it never guesses.
"""

BREEDING_FRAME = 16  # Player.cs:15-56: the battle frames are 0..15, the breeding frame follows them
MAX_HYPOTHESES = 4096

# BattleScene.unity: YourPermanentFrame (the local seat, Player.isYou) / OpponentPermanentFrame children, in child
# (= FrameID) order: anchoredPosition (x, y).
YOU_LAYOUT = [(-640, -151), (-480, -151), (-320, -151), (-160, -151), (0, -151), (160, -151), (320, -151),
              (480, -151), (640, -151), (-330, -342.66), (-160, -342.66), (0, -342.66), (160, -342.66),
              (330, -342.66), (490, -342.66), (650, -342.66)]
OPP_LAYOUT = [(640, 171), (480, 171), (320, 171), (160, 171), (0, 171), (-160, 171), (-320, 171), (-480, 171),
              (-640, 171), (310, 350.2), (160, 350.2), (0, 350.2), (-160, 350.2), (-310, 350.2), (-470, 350.2),
              (-630, 350.2)]
# CardSource.cs:2368: the other seat's Digimon / Digi-Egg order is this fixed FrameID list
OPP_DIGIMON_ORDER = [4, 3, 5, 2, 6, 1, 7, 0, 8, 11, 10, 12, 9, 13, 14, 15]

# cards.json card_kind (CEntity_Base.cs:347 CardKind order: Digimon, Tamer, Option, DigiEgg); 4 is a Digimon-side
# variant there. Anything else is unknown to the model.
DIGIMON_KINDS, TAMER_KINDS, EGG_KIND = (0, 4), (1, 2), 3


def preferred_order(is_you, tamer_side):
    """The FrameIDs PreferredFrame tries, first to last (CardSource.cs:2340-2410). `tamer_side`: a Tamer or Option."""
    frames = range(len(YOU_LAYOUT))
    if is_you:
        lay = YOU_LAYOUT
        if tamer_side:   # OrderBy y, ThenBy |x|, ThenBy x
            return sorted(frames, key=lambda f: (lay[f][1], abs(lay[f][0]), lay[f][0]))
        return sorted(frames, key=lambda f: (-lay[f][1], abs(lay[f][0]), lay[f][0]))  # OrderByDescending y
    lay = OPP_LAYOUT
    if tamer_side:       # OrderByDescending y, ThenBy |x|, ThenByDescending x
        return sorted(frames, key=lambda f: (-lay[f][1], abs(lay[f][0]), -lay[f][0]))
    return list(OPP_DIGIMON_ORDER)


def _be32(b, at):
    if len(b) < at + 4 or not all(isinstance(x, int) for x in b[at:at + 4]):
        return None
    n = (b[at] << 24) | (b[at + 1] << 16) | (b[at + 2] << 8) | b[at + 3]
    return n - (1 << 32) if n & 0x80000000 else n


class _Seat:
    """One seat's frame hypotheses: frozensets of (frame, card_id, card_index or None)."""

    def __init__(self, cards, is_you):
        self.cards = cards
        self.is_you = is_you
        self.hyps = {frozenset()}
        self.alive = True
        self.packet = None  # (card_id, card_index, target_breeding) of the seat's latest queued play packet

    def kill(self):
        self.alive = False
        self.hyps = set()

    def kind(self, card_id):
        k = (self.cards.get(card_id) or {}).get("card_kind")
        if k in DIGIMON_KINDS:
            return "digimon"
        if k in TAMER_KINDS:
            return "tamer"
        if k == EGG_KIND:
            return "egg"
        return None

    def preferred(self, h, card_id):
        taken = {f for f, _, _ in h}
        side = self.kind(card_id)
        for f in preferred_order(self.is_you, side == "tamer"):
            if f not in taken:
                return f
        return None

    def _rerow(self, h, frame):
        """CardController.cs:1811-1856: after a play, move to an empty PreferredFrame on the kind's own row."""
        cur = next(x for x in h if x[0] == frame)
        side = self.kind(cur[1])
        rest = h - {cur}
        p = self.preferred(rest | {cur}, cur[1])
        if p is None or p == frame or frame == BREEDING_FRAME:
            return h
        lay = YOU_LAYOUT if self.is_you else OPP_LAYOUT
        py, fy = lay[p][1], lay[frame][1]
        if side == "digimon":
            move = py > fy + 5 if self.is_you else py < fy - 5
        else:
            move = py < fy - 5 if self.is_you else py > fy + 5
        return rest | {(p, cur[1], cur[2])} if move else h

    def play(self, card_id, index, target_id, to_breeding, owner_known):
        """A PlayCardClass resolve of `card_id`: fresh (target None) or onto a permanent whose top is `target_id`."""
        if not self.alive:
            return
        if self.kind(card_id) is None:
            return self.kill()
        new = set(self.hyps) if not owner_known else set()
        for h in self.hyps:
            if target_id is not None and to_breeding:
                # a digivolve in the breeding area: its egg may have hatched unseen (no row logs a hatch's card)
                breed = next((x for x in h if x[0] == BREEDING_FRAME), None)
                if breed is None or breed[1] == target_id:
                    new.add((h - ({breed} if breed else set())) | {(BREEDING_FRAME, card_id, index)})
            elif target_id is not None:
                for x in h:
                    if x[1] == target_id:
                        g = (h - {x}) | {(x[0], card_id, index)}
                        new.add(g if x[0] == BREEDING_FRAME else self._rerow(g, x[0]))
            elif to_breeding:
                if all(f != BREEDING_FRAME for f, _, _ in h):
                    new.add(h | {(BREEDING_FRAME, card_id, index)})
            else:
                f = self.preferred(h, card_id)
                if f is not None:
                    new.add(self._rerow(h | {(f, card_id, index)}, f))
        self._set(new)

    def _set(self, hyps):
        if not hyps or len(hyps) > MAX_HYPOTHESES:
            return self.kill()
        self.hyps = hyps

    def check(self, listing, full):
        """Keep the hypotheses that explain `listing` (battle area, plus the breeding area when `full`).

        When none does, one more explanation is tried: an Option the recorder never saw played (see
        `_with_unlogged_options`). It is a last resort and a pure appearance - nothing else may leave in the same
        step - so it never turns an inconsistent model into a consistent one by also guessing a departure."""
        if not self.alive:
            return
        if any(not isinstance(c, str) or not c for c in listing):
            return self.kill()
        out = self._explain(self.hyps, listing, full)
        if out is None:
            return self.kill()
        if not out:
            out = self._explain_unlogged_options(listing, full)
            if out is None:
                return self.kill()
        self._set(out)

    def _explain(self, hyps, listing, full, min_kept=None):
        """The hypotheses `hyps` yield for `listing`, or None past MAX_HYPOTHESES. `min_kept`: {hypothesis: how many
        permanents it must still hold} - an explanation that lets any of them go is dropped."""
        out = set()
        for h in hyps:
            bases = [h]
            breed = next((x for x in h if x[0] == BREEDING_FRAME), None)
            if breed is not None:
                rest = h - {breed}
                f = self.preferred(rest, breed[1])  # CardObjectController.cs:1160 MovePermanent -> PreferredFrame
                if f is not None:
                    bases.append(rest | {(f, breed[1], breed[2])})
            if full and listing and self.kind(listing[-1]) == "egg":
                bases += [b | {(BREEDING_FRAME, listing[-1], None)} for b in bases
                          if all(x[0] != BREEDING_FRAME for x in b)]
            for b in bases:
                seq = sorted(b)
                kept_breed = None
                if not full:
                    kept_breed = next((x for x in seq if x[0] == BREEDING_FRAME), None)
                    seq = [x for x in seq if x[0] != BREEDING_FRAME]
                for keep in self._align(seq, listing):
                    g = frozenset(keep) | ({kept_breed} if kept_breed else set())
                    if min_kept is not None and len(g) < min_kept[h]:
                        continue
                    out.add(g)
                    if len(out) > MAX_HYPOTHESES:
                        return None
        return out

    def _explain_unlogged_options(self, listing, full):
        """`listing` as every hypothesis grown by Options nobody logged playing, nothing else changed.

        CardEffectCommons.PlaceDelayOptionCards (a Delay Option placed by its own effect, e.g. a [Security] [Main]
        placement) builds a PlayPermanentClass directly: the Option takes PreferredFrame and the hybrid re-row like
        any play (CardController.cs:1811-1856), but PlayCardClass.PlayCard - the recorder's only play hook - never
        runs, so no row names it and its DCGO CardIndex is unknown. It is the only direct PlayPermanentClass path
        besides a DigiEgg's hatch (HatchDigiEggClass, handled by `_explain`), so only an Option is admitted. Every
        Option id on the listing is tried as the newcomer, up to two of them (a placement can place several)."""
        options = sorted({c for c in listing if self.is_option(c)})
        if not options:
            return set()
        grown = set()
        for h in self.hyps:
            frontier = [h]
            for _ in range(2):
                nxt = []
                for g in frontier:
                    for c in options:
                        f = self.preferred(g, c)
                        if f is None:
                            continue
                        g2 = self._rerow(g | {(f, c, None)}, f)
                        grown.add(g2)
                        nxt.append(g2)
                frontier = nxt
        out = set()
        for g in grown:
            # the grown hypothesis must be kept whole, not whittled down by a departure
            res = self._explain({g}, listing, full, {g: len(g)})
            if res is None:
                return None
            out |= res
            if len(out) > MAX_HYPOTHESES:
                return None
        return out

    def is_option(self, card_id):
        """An Option proper (cards.json card_kind 2): the card kind a Delay Option placement puts on the field."""
        k = (self.cards.get(card_id) or {}).get("card_kind")
        return k == 2 or "option" in str(k).lower()

    def _align(self, seq, listing):
        """Every way `seq` (frame order) yields `listing`: an entry stays (same id, or a Digimon whose top changed in
        place to another Digimon, its index then unknown) or has left the field."""
        res = []

        def rec(i, j, kept):
            if j == len(listing):
                res.append(list(kept))
                return
            if len(seq) - i < len(listing) - j:
                return
            f, c, k = seq[i]
            if c == listing[j]:
                rec(i + 1, j + 1, kept + [(f, c, k)])
            elif f != BREEDING_FRAME and self.kind(c) == "digimon" and self.kind(listing[j]) == "digimon":
                rec(i + 1, j + 1, kept + [(f, listing[j], None)])
            rec(i + 1, j, kept)

        rec(0, 0, [])
        return res

    def frame_occupant(self, frame):
        """The permanent at battle FrameID `frame` when EVERY hypothesis agrees on it: (listing position among the
        battle frames, card id, DCGO CardIndex or None). None when the model is off, a hypothesis leaves the frame
        empty, or two hypotheses disagree on the card or on how many battle permanents sit below it."""
        if not self.alive or not self.hyps or not 0 <= frame < BREEDING_FRAME:
            return None
        seen = set()
        for h in self.hyps:
            at = next((x for x in h if x[0] == frame), None)
            if at is None:
                return None
            seen.add((sum(1 for x in h if x[0] < frame), at[1], at[2]))
        cards = {(pos, cid) for pos, cid, _ in seen}
        if len(cards) != 1:
            return None
        (pos, cid), = cards
        ks = {k for _, _, k in seen}
        k = ks.pop() if len(ks) == 1 else None
        return pos, cid, (k if isinstance(k, int) and not isinstance(k, bool) else None)

    def is_tamer(self, card_id):
        """A Tamer proper (cards.json card_kind 1, the engine's CardKind::Tamer, as `convert_raw.is_tamer` reads it).
        `kind()` groups Options with Tamers because both take the Tamer row for FRAME PLACEMENT; a burst returns a
        Tamer, never an Option (a Delay Option on the battle area)."""
        k = (self.cards.get(card_id) or {}).get("card_kind")
        return k == 1 or "tamer" in str(k).lower()

    def occupant(self, position, full):
        """DCGO's CardIndex of the permanent at listing `position`, when every hypothesis agrees; else None."""
        if not self.alive or not self.hyps:
            return None
        seen = set()
        for h in self.hyps:
            seq = [x for x in sorted(h) if full or x[0] != BREEDING_FRAME]
            if not 0 <= position < len(seq):
                return None
            seen.add(seq[position][2])
            if len(seen) > 1:
                return None
        k = seen.pop()
        return k if isinstance(k, int) and not isinstance(k, bool) else None


def _full_listing(r):
    """Core.cs: SetTargetFrames and the attack / activate packets log GetFieldPermanents() (battle + breeding);
    play packets, play resolves and SetAttackingPermaent log GetBattleAreaPermanents()."""
    m = r.get("method")
    if m == "SelectPermanentEffect.SetTargetFrames":
        return True
    a = r.get("args") or []
    return m == "TurnStateMachine.QueueMainPhaseAction_Internal" and len(a) >= 2 and a[1] in (0, 2)


def _played_index(r, seat, packet, card_id, pos):
    ks = r.get("card_indexes")
    if isinstance(ks, list) and pos < len(ks) and isinstance(ks[pos], int):
        return ks[pos]
    if pos == 0 and packet is not None and packet[0] == card_id and isinstance(packet[1], int):
        return packet[1]
    hand = r.get("hand") or {}
    cards = hand.get("cards")
    slot = hand.get("slot", -1)
    if pos == 0 and hand.get("player") == seat and isinstance(cards, list) and isinstance(slot, int) \
            and 0 <= slot < len(cards) and (cards[slot] or {}).get("card_id") == card_id:
        k = cards[slot].get("card_index")
        return k if isinstance(k, int) else None
    return None


def pick_identities(rows, cards, you):
    """{raw row index: {(player, position): DCGO CardIndex}} for each SetTargetFrames board position whose card id
    occurs more than once on that seat's board and whose occupant the frame model names."""
    return identities(rows, cards, you)[0]


def burst_tamers(rows, cards, you):
    """{raw row index of a Burst Digivolve play packet: (seat, position, card_id, card_index or None)}: the Tamer the
    packet's BurstTamerFrameID names, where the frame model is certain of it. A remote seat's own pick of that Tamer
    (SelectBurstDigivolutionEffect.SelectTamer, `isLocal: true`) ran on its own client and left no row, but the
    packet carries the picked permanent's FrameID (TurnStateMachine.cs:3016-3030, `permanent.PermanentFrame.FrameID`),
    and a FrameID names a permanent (module docstring). The Tamer is named only when every hypothesis puts the same
    card id at that frame, the card is a Tamer, and the packet's own board (battle area, logged at queue time, before
    the play resolves) is explained by the model."""
    return identities(rows, cards, you)[1]


def identities(rows, cards, you):
    """(`pick_identities`, `burst_tamers`) from one pass over the rows."""
    seats = [_Seat(cards, s == you) for s in (0, 1)]
    found = {}
    burst = {}
    for ridx, r in enumerate(rows):
        m = r.get("method")
        if m == "TurnStateMachine.QueueMainPhaseAction_Internal":
            a = r.get("args") or []
            if len(a) >= 3 and a[0] in (0, 1) and a[1] == 4:
                b = a[2] if isinstance(a[2], list) else []
                n_roots = _be32(b, 8)
                n_apps = _be32(b, 16 + 4 * n_roots) if n_roots is not None and n_roots >= 0 else None
                if n_roots is None or n_roots != 0 or n_apps is None or n_apps != 0:
                    seats[a[0]].kill()  # DNA / App Fusion: CreateNewPermanent at a TargetFrameID (not modelled)
                play = r.get("play") or {}
                seats[a[0]].packet = (play.get("card_id"), play.get("card_index"), bool(play.get("target_breeding")))
        boards = r.get("boards")
        if isinstance(boards, list) and len(boards) == 2:
            full = _full_listing(r)
            for p in (0, 1):
                if isinstance(boards[p], list):
                    seats[p].check(boards[p], full)
            if m == "TurnStateMachine.QueueMainPhaseAction_Internal":
                a = r.get("args") or []
                if len(a) >= 3 and a[0] in (0, 1) and a[1] == 4 and isinstance(a[2], list):
                    n_roots = _be32(a[2], 8)
                    f = _be32(a[2], 12 + 4 * n_roots) if n_roots is not None and 0 <= n_roots < 64 else None
                    if f is not None and f >= 0 and isinstance(boards[a[0]], list):
                        # frame_occupant is None for a seat the model has switched off
                        occ = seats[a[0]].frame_occupant(f)
                        if occ is not None and seats[a[0]].is_tamer(occ[1]):
                            burst[ridx] = (a[0],) + occ
            if m == "SelectPermanentEffect.SetTargetFrames":
                named = {}
                for p in (0, 1):
                    board = boards[p] if isinstance(boards[p], list) else []
                    for i, c in enumerate(board):
                        if board.count(c) >= 2:
                            k = seats[p].occupant(i, full)
                            if k is not None:
                                named[(p, i)] = k
                if named:
                    found[ridx] = named
        if m == "PlayCardClass.PlayCard":
            hand = r.get("hand") or {}
            owner = hand.get("player")
            tgt = r.get("target")
            tgt_id = tgt.get("card_id") if isinstance(tgt, dict) else None
            for s, seat in enumerate(seats):
                if owner in (0, 1) and owner != s:
                    continue
                known = owner == s
                packet = seat.packet if known else None
                to_breeding = bool(r.get("breeding_area") or r.get("target_breeding")
                                   or (packet is not None and packet[0] == (r.get("cards") or [None])[0] and packet[2]))
                for pos, c in enumerate(r.get("cards") or []):
                    seat.play(c, _played_index(r, s, packet, c, pos) if known else None,
                              tgt_id if pos == 0 else None, to_breeding, known)
                seat.packet = None if known else seat.packet
    return found, burst
