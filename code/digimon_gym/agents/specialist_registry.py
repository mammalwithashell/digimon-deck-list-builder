"""Specialist registry — deck-keyed, versioned model snapshots for the
deck-specialist league (``add-deck-specialist-league``).

One ``Specialist`` per starter deck records enough provenance to (a) load it as a
*coupled* (policy, deck) opponent and (b) verify tensor-layout compatibility
before play. This is the deck-axis sibling of ``champion_registry.py`` (which is
keyed by champion *name*); here the key is the **deck**, and each entry carries
the deck it pilots plus the league ``round`` it was snapshotted in.

The registry is also the source of truth for the per-round **opponent pool**: for
a given specialist, ``emit_round_pool`` returns the frozen (policy, deck) opponents
it should train against this round — the latest snapshot of every deck (including
its **own** deck, for the mirror) plus retained history — all filtered to the
active tensor layout. Unlike ``OpponentEntry`` (policy-only), a round-pool entry
carries ``deck`` so the league opponent mode can set both the opponent policy and
its deck from one entry (see the reuse-audit gap).
"""
from __future__ import annotations

import json
import re
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Union


def slugify_deck(deck: str) -> str:
    """Canonical archetype key -> filesystem slug, e.g.
    ``"ST-1 Gaia Red"`` -> ``"st-1"`` when it starts with an ``ST-N`` token,
    else a lowercased, hyphenated form of the whole name."""
    m = re.match(r"\s*(ST-\d+)", deck, flags=re.IGNORECASE)
    if m:
        return m.group(1).lower()
    return re.sub(r"[^a-z0-9]+", "-", deck.strip().lower()).strip("-")


def _norm_algorithm(raw: str) -> str:
    s = str(raw).lower()
    if s in ("lstm", "maskablerecurrentppo", "recurrent"):
        return "lstm"
    return "mlp"


@dataclass
class Specialist:
    """A frozen specialist snapshot: one deck's policy at one league round."""

    deck: str
    weights_path: str
    algorithm: str  # "lstm" | "mlp"
    observation_profile: str
    tensor_layout_hash: str
    round: int = 0
    source: str = ""
    created_at: str = ""

    @property
    def name(self) -> str:
        """Stable opponent name: ``<deck-slug>@r<round>``."""
        return f"{slugify_deck(self.deck)}@r{self.round}"

    @classmethod
    def from_meta(
        cls,
        deck: str,
        weights_path: Union[str, Path],
        meta: dict,
        round: int = 0,
        source: str = "",
        created_at: str = "",
    ) -> "Specialist":
        """Build a Specialist from a model's ``.meta.json`` payload."""
        return cls(
            deck=deck,
            weights_path=str(weights_path),
            algorithm=_norm_algorithm(meta.get("algorithm", "")),
            observation_profile=str(meta.get("observation_profile", "")),
            tensor_layout_hash=str(meta.get("tensor_layout_hash", "")),
            round=int(round),
            source=source,
            created_at=created_at,
        )


@dataclass
class LeaguePoolEntry:
    """A coupled (policy, deck) opponent for a round pool. Superset of
    ``OpponentEntry`` with the ``deck`` the policy pilots, so the league
    opponent mode can set ``deck2`` and load the policy from one entry."""

    name: str
    weights_path: str
    algorithm: str
    deck: str
    win_rate_vs_pool: float = 0.5
    games_played: int = 0


class SpecialistRegistry:
    """Deck-keyed, versioned collection of specialists, persisted as JSON.

    ``current`` holds the latest specialist per deck; ``history`` keeps every
    snapshot across rounds (for retained-history opponent pools and the mirror).
    """

    def __init__(
        self,
        path: Union[str, Path],
        current: Optional[Dict[str, Specialist]] = None,
        history: Optional[List[Specialist]] = None,
    ):
        self.path = Path(path)
        self.current: Dict[str, Specialist] = dict(current or {})
        self.history: List[Specialist] = list(history or [])

    # ---- persistence ----
    @classmethod
    def load(cls, path: Union[str, Path]) -> "SpecialistRegistry":
        p = Path(path)
        if not p.exists():
            return cls(p, {}, [])
        data = json.loads(p.read_text(encoding="utf-8"))
        current = {
            deck: Specialist(**spec) for deck, spec in data.get("current", {}).items()
        }
        history = [Specialist(**s) for s in data.get("history", [])]
        return cls(p, current, history)

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "version": 1,
            "current": {deck: asdict(s) for deck, s in self.current.items()},
            "history": [asdict(s) for s in self.history],
        }
        self.path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    # ---- mutation ----
    def set_current(self, specialist: Specialist) -> None:
        """Promote ``specialist`` to the current entry for its deck and append
        it to history (the durable per-round snapshot trail)."""
        self.current[specialist.deck] = specialist
        self.history.append(specialist)

    # ---- queries ----
    def decks(self) -> List[str]:
        return list(self.current.keys())

    def get(self, deck: str) -> Specialist:
        return self.current[deck]

    def compatible(self, tensor_layout_hash: str) -> List[Specialist]:
        """All snapshots (current ∪ history) whose layout matches — i.e. those a
        model on that layout can validly face."""
        seen = set()
        out: List[Specialist] = []
        for s in list(self.current.values()) + self.history:
            if s.tensor_layout_hash != tensor_layout_hash:
                continue
            key = (s.deck, s.round, s.weights_path)
            if key in seen:
                continue
            seen.add(key)
            out.append(s)
        return out

    def snapshots_for(
        self, deck: str, tensor_layout_hash: str, keep_last: int = 1
    ) -> List[Specialist]:
        """The last ``keep_last`` layout-compatible snapshots of ``deck`` (newest
        first by round). Used to assemble a deck's contribution to a round pool —
        and, when ``deck`` is the training specialist's own deck, the mirror."""
        snaps = [
            s
            for s in self.history
            if s.deck == deck and s.tensor_layout_hash == tensor_layout_hash
        ]
        snaps.sort(key=lambda s: s.round, reverse=True)
        # De-dupe by (round, weights_path) keeping newest.
        seen = set()
        uniq: List[Specialist] = []
        for s in snaps:
            key = (s.round, s.weights_path)
            if key in seen:
                continue
            seen.add(key)
            uniq.append(s)
        return uniq[: max(0, keep_last)]

    # ---- round-pool emission ----
    def emit_round_pool(
        self,
        for_deck: str,
        tensor_layout_hash: str,
        keep_last_per_deck: int = 1,
        include_mirror: bool = True,
    ) -> List[LeaguePoolEntry]:
        """Frozen (policy, deck) opponents for ``for_deck``'s next round.

        Includes, per deck, the last ``keep_last_per_deck`` layout-compatible
        snapshots — **including ``for_deck``'s own** snapshots when
        ``include_mirror`` (the mirror requirement). All entries carry their deck
        so the league opponent mode sets ``deck2`` + loads the policy from one
        entry. Layout-incompatible snapshots are excluded (the layout-hash gate).
        """
        entries: List[LeaguePoolEntry] = []
        for deck in self._all_decks():
            if deck == for_deck and not include_mirror:
                continue
            for snap in self.snapshots_for(deck, tensor_layout_hash, keep_last_per_deck):
                entries.append(
                    LeaguePoolEntry(
                        name=snap.name,
                        weights_path=snap.weights_path,
                        algorithm=snap.algorithm,
                        deck=snap.deck,
                    )
                )
        return entries

    def write_round_pool(
        self,
        out_path: Union[str, Path],
        for_deck: str,
        tensor_layout_hash: str,
        keep_last_per_deck: int = 1,
        include_mirror: bool = True,
    ) -> Path:
        """Emit ``for_deck``'s round pool to a JSON manifest the league opponent
        mode loads. Errors if the pool is empty (a silently-empty pool would
        leave the opponent seat passive — same guard as
        ``OpponentPool.from_champion_registry``)."""
        entries = self.emit_round_pool(
            for_deck, tensor_layout_hash, keep_last_per_deck, include_mirror
        )
        if not entries:
            raise ValueError(
                f"empty round pool for deck {for_deck!r} at layout "
                f"{tensor_layout_hash!r} — no compatible specialist snapshots yet "
                "(seed the registry from the generalist first)"
            )
        out = Path(out_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            json.dumps(
                {"version": 1, "for_deck": for_deck, "entries": [asdict(e) for e in entries]},
                indent=2,
            ),
            encoding="utf-8",
        )
        return out

    def _all_decks(self) -> List[str]:
        decks = list(self.current.keys())
        for s in self.history:
            if s.deck not in decks:
                decks.append(s.deck)
        return decks
