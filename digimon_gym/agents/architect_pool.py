"""Candidate pool for the architect agent deck optimization pipeline.

Scopes the card pool for a given archetype to a tractable action space,
encoding deck modifications as discrete swap actions (remove one card,
add another).  The pool is derived from archetype decklists in the deck
library, filtered to implementable (frozen-script) cards only.

The action space is structured as:
  - Action 0: no-op (agent elects to stop swapping)
  - Actions 1..N: encoded swap pairs (remove_idx, add_idx) over the
    candidate list, where N = n_candidates * n_candidates.
"""

from __future__ import annotations

import json
import logging
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np

from digimon_gym.agents.deck_pool import analyze_core
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.deck_finder import load_implemented_card_ids
from digimon_gym.engine.data.enums import CardKind

logger = logging.getLogger(__name__)

_DECK_LIBRARY_PATH = (
    Path(__file__).resolve().parent.parent
    / "engine" / "data" / "deck_library.json"
)


class CandidatePool:
    """Scopes the card pool for a given archetype to a tractable action space."""

    def __init__(
        self,
        base_deck: List[str],
        archetype_name: str,
        extra_cards: Optional[List[str]] = None,
        custom_pool: Optional[List[str]] = None,
    ) -> None:
        """Initialise the candidate pool.

        Args:
            base_deck: Flat list of card IDs (the starting deck, 50 main + eggs).
            archetype_name: Name in deck_library.json to pull related cards from.
            extra_cards: Additional card IDs to include in the pool.
            custom_pool: If provided, replaces the archetype-derived pool entirely.
        """
        self._archetype_name = archetype_name
        self._db = CardDatabase()

        # --- Build raw candidate set ---
        if custom_pool is not None:
            raw_ids: set[str] = set(custom_pool)
        else:
            raw_ids = self._collect_archetype_cards(archetype_name)

        if extra_cards:
            raw_ids.update(extra_cards)

        # Always include cards already in the base deck so the agent can
        # reason about keeping them.
        raw_ids.update(base_deck)

        # --- Filter to implementable, non-egg cards ---
        implemented = load_implemented_card_ids()
        filtered: List[str] = []
        for cid in raw_ids:
            if cid not in implemented:
                continue
            entity = self._db.get_card(cid)
            if entity is not None and entity.card_kind == CardKind.DigiEgg:
                continue
            filtered.append(cid)

        # Stable alphabetical ordering
        self._candidates: List[str] = sorted(filtered)
        self._candidate_to_idx: Dict[str, int] = {
            cid: i for i, cid in enumerate(self._candidates)
        }

        # --- Core / flex classification ---
        self._core_cards, self._flex_cards = analyze_core(base_deck)

        # --- Cache max copies per candidate ---
        self._max_copies: Dict[str, int] = {}
        for cid in self._candidates:
            entity = self._db.get_card(cid)
            self._max_copies[cid] = (
                entity.max_count_in_deck if entity is not None else 4
            )

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def candidates(self) -> List[str]:
        """Unique card IDs in the pool (sorted, stable ordering)."""
        return self._candidates

    @property
    def candidate_to_idx(self) -> Dict[str, int]:
        """Mapping of card_id to its index in the candidates list."""
        return self._candidate_to_idx

    @property
    def n_candidates(self) -> int:
        return len(self._candidates)

    @property
    def action_size(self) -> int:
        """Total number of discrete actions including no-op at index 0."""
        return self.n_candidates * self.n_candidates + 1

    @property
    def core_cards(self) -> Dict[str, int]:
        return self._core_cards

    @property
    def flex_cards(self) -> Dict[str, int]:
        return self._flex_cards

    # ------------------------------------------------------------------
    # Action encoding / decoding
    # ------------------------------------------------------------------

    def decode_action(self, action: int) -> Optional[Tuple[str, str]]:
        """Decode an action index to a swap pair or ``None`` for no-op.

        Returns:
            ``(remove_card_id, add_card_id)`` or ``None`` if *action* is 0.
        """
        if action == 0:
            return None
        a = action - 1
        remove_idx = a // self.n_candidates
        add_idx = a % self.n_candidates
        return self._candidates[remove_idx], self._candidates[add_idx]

    def encode_action(self, remove_card: str, add_card: str) -> int:
        """Encode a swap into an action index."""
        remove_idx = self._candidate_to_idx[remove_card]
        add_idx = self._candidate_to_idx[add_card]
        return remove_idx * self.n_candidates + add_idx + 1

    # ------------------------------------------------------------------
    # Action masking
    # ------------------------------------------------------------------

    def get_action_mask(self, current_deck: Dict[str, int]) -> np.ndarray:
        """Return a valid-action mask for the current deck composition.

        Args:
            current_deck: ``{card_id: count}`` for the current main deck.

        Returns:
            ``np.ndarray`` of shape ``(action_size,)`` with 1 for valid
            actions, 0 for invalid.

        Rules:
            - No-op (action 0) is always valid.
            - Can only remove cards present in the deck (count > 0).
            - Can only add cards not already at their max copies.
            - Cannot swap a card with itself.
            - Eggs are excluded from the candidate list at construction
              time, so they never appear here.
        """
        n = self.n_candidates
        mask = np.zeros(self.action_size, dtype=np.int8)
        mask[0] = 1  # no-op always valid

        # Pre-compute per-candidate flags
        can_remove = np.zeros(n, dtype=np.int8)
        can_add = np.zeros(n, dtype=np.int8)
        for i, cid in enumerate(self._candidates):
            if current_deck.get(cid, 0) > 0:
                can_remove[i] = 1
            if current_deck.get(cid, 0) < self._max_copies[cid]:
                can_add[i] = 1

        # Vectorised outer product then flatten
        # valid[r, a] = can_remove[r] & can_add[a] & (r != a)
        valid = np.outer(can_remove, can_add).astype(np.int8)
        np.fill_diagonal(valid, 0)  # cannot swap with self
        mask[1:] = valid.ravel()

        return mask

    # ------------------------------------------------------------------
    # Deck manipulation
    # ------------------------------------------------------------------

    def apply_swap(
        self,
        deck_counts: Dict[str, int],
        remove_card: str,
        add_card: str,
    ) -> Dict[str, int]:
        """Apply a single swap and return a new counts dict."""
        new_counts = dict(deck_counts)
        new_counts[remove_card] = new_counts.get(remove_card, 0) - 1
        if new_counts[remove_card] <= 0:
            del new_counts[remove_card]
        new_counts[add_card] = new_counts.get(add_card, 0) + 1
        return new_counts

    # ------------------------------------------------------------------
    # Vector representations
    # ------------------------------------------------------------------

    def deck_to_vector(self, deck_counts: Dict[str, int]) -> np.ndarray:
        """Convert deck counts to a normalised vector over candidates.

        Each position is ``count / max_copies`` for that card.
        Shape: ``(n_candidates,)``.
        """
        vec = np.zeros(self.n_candidates, dtype=np.float32)
        for cid, count in deck_counts.items():
            idx = self._candidate_to_idx.get(cid)
            if idx is not None:
                mc = self._max_copies.get(cid, 4)
                vec[idx] = count / mc if mc > 0 else 0.0
        return vec

    def vector_to_deck(self, vector: np.ndarray) -> Dict[str, int]:
        """Convert a normalised vector back to integer deck counts.

        Rounds to the nearest integer and clamps to ``[0, max_copies]``.
        """
        counts: Dict[str, int] = {}
        for i, cid in enumerate(self._candidates):
            mc = self._max_copies.get(cid, 4)
            raw = int(round(float(vector[i]) * mc))
            raw = max(0, min(raw, mc))
            if raw > 0:
                counts[cid] = raw
        return counts

    # ------------------------------------------------------------------
    # Deck list helpers
    # ------------------------------------------------------------------

    def deck_list_to_counts(self, deck_ids: List[str]) -> Dict[str, int]:
        """Convert a flat card-ID list to ``{card_id: count}``, main deck only.

        Egg cards are excluded from the returned dict.
        """
        counts: Dict[str, int] = {}
        for cid in deck_ids:
            entity = self._db.get_card(cid)
            if entity is not None and entity.card_kind == CardKind.DigiEgg:
                continue
            counts[cid] = counts.get(cid, 0) + 1
        return counts

    def counts_to_deck_list(
        self,
        deck_counts: Dict[str, int],
        egg_deck: List[str],
    ) -> List[str]:
        """Convert a counts dict and egg deck back to a flat card-ID list."""
        flat: List[str] = list(egg_deck)
        for cid, count in sorted(deck_counts.items()):
            flat.extend([cid] * count)
        return flat

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def to_json(self) -> dict:
        """Serialise pool metadata for saving alongside a trained model."""
        return {
            "archetype_name": self._archetype_name,
            "candidates": self._candidates,
            "max_copies": self._max_copies,
            "core_cards": self._core_cards,
            "flex_cards": self._flex_cards,
        }

    @classmethod
    def from_json(cls, data: dict) -> "CandidatePool":
        """Deserialise a pool from previously saved JSON.

        This bypasses the normal constructor (no deck-library lookup or
        filtering) and restores the exact candidate set that was used
        during training.
        """
        pool = object.__new__(cls)
        pool._archetype_name = data["archetype_name"]
        pool._db = CardDatabase()
        pool._candidates = list(data["candidates"])
        pool._candidate_to_idx = {
            cid: i for i, cid in enumerate(pool._candidates)
        }
        pool._core_cards = dict(data.get("core_cards", {}))
        pool._flex_cards = dict(data.get("flex_cards", {}))
        pool._max_copies = {
            cid: int(v) for cid, v in data.get("max_copies", {}).items()
        }
        # Backfill max_copies for any candidate missing from saved data
        for cid in pool._candidates:
            if cid not in pool._max_copies:
                entity = pool._db.get_card(cid)
                pool._max_copies[cid] = (
                    entity.max_count_in_deck if entity is not None else 4
                )
        return pool

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _collect_archetype_cards(self, archetype_name: str) -> set[str]:
        """Load all unique card IDs from the named archetype's decklists."""
        if not _DECK_LIBRARY_PATH.exists():
            logger.warning(
                "Deck library not found at %s; pool will contain only "
                "base-deck cards.",
                _DECK_LIBRARY_PATH,
            )
            return set()

        with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
            library = json.load(f)

        archetype_data = library.get("archetypes", {}).get(archetype_name)
        if archetype_data is None:
            logger.warning(
                "Archetype %r not found in deck library; pool will "
                "contain only base-deck cards.",
                archetype_name,
            )
            return set()

        card_ids: set[str] = set()
        for deck_entry in archetype_data.get("decklists", []):
            raw = deck_entry.get("decklist", "[]")
            try:
                ids: List[str] = json.loads(raw)
            except (json.JSONDecodeError, TypeError):
                continue
            card_ids.update(ids)

        return card_ids
