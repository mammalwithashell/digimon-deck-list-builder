"""CardRegistry: deterministic card_id → integer mapping for tensor encoding.

Mirrors the C# CardRegistry. Card IDs are assigned stable integer indices
starting at 1.  0 is reserved for padding/empty.

Supports two cards.json formats:
  - New dict format: keys are card_ids, each entry has "index" and "norm_id"
  - Legacy array format: sorted alphabetically, indices assigned sequentially
"""

from __future__ import annotations
from typing import Dict, Optional
import json
import os

REGISTRY_CAPACITY = 20_000


class CardRegistry:
    _id_to_int: Dict[str, int] = {}
    _int_to_id: Dict[int, str] = {}
    _id_to_norm: Dict[str, float] = {}
    PADDING_ID: int = 0
    CAPACITY: int = REGISTRY_CAPACITY
    _initialized: bool = False

    @classmethod
    def initialize(cls, json_path: Optional[str] = None):
        """Load cards.json and build id maps.

        Detects dict vs array format automatically.
        """
        if json_path is None:
            json_path = os.path.join(os.path.dirname(__file__), "cards.json")

        with open(json_path, "r", encoding="utf-8") as f:
            data = json.load(f)

        cls._id_to_int = {}
        cls._int_to_id = {}
        cls._id_to_norm = {}

        if isinstance(data, dict):
            # New dict format: keys are card_ids with index/norm_id fields
            for card_id, entry in data.items():
                idx = entry["index"]
                cls._id_to_int[card_id] = idx
                cls._int_to_id[idx] = card_id
                cls._id_to_norm[card_id] = entry.get(
                    "norm_id", idx / REGISTRY_CAPACITY
                )
        else:
            # Legacy array format: sort alphabetically, assign sequential IDs
            card_ids = sorted(
                {entry["card_id"] for entry in data if entry.get("card_id")}
            )
            for idx, cid in enumerate(card_ids, start=1):
                cls._id_to_int[cid] = idx
                cls._int_to_id[idx] = cid
                cls._id_to_norm[cid] = idx / REGISTRY_CAPACITY

        cls._initialized = True

    @classmethod
    def initialize_from_list(cls, card_ids: list[str]):
        """Build registry from an explicit list (useful for tests)."""
        sorted_ids = sorted(set(cid for cid in card_ids if cid))
        cls._id_to_int = {}
        cls._int_to_id = {}
        cls._id_to_norm = {}
        for idx, cid in enumerate(sorted_ids, start=1):
            cls._id_to_int[cid] = idx
            cls._int_to_id[idx] = cid
            cls._id_to_norm[cid] = idx / REGISTRY_CAPACITY
        cls._initialized = True

    @classmethod
    def ensure_initialized(cls):
        if not cls._initialized:
            cls.initialize()

    @classmethod
    def get_id(cls, card_id: str) -> int:
        """Return the integer ID for a card_id string. 0 if unknown."""
        cls.ensure_initialized()
        if not card_id:
            return cls.PADDING_ID
        return cls._id_to_int.get(card_id, cls.PADDING_ID)

    @classmethod
    def get_norm_id(cls, card_id: str) -> float:
        """Return the normalized ID for a card_id string. 0.0 if unknown."""
        cls.ensure_initialized()
        if not card_id:
            return 0.0
        return cls._id_to_norm.get(card_id, 0.0)

    @classmethod
    def get_string_id(cls, internal_id: int) -> Optional[str]:
        """Return the card_id string for an integer ID. None if unknown."""
        cls.ensure_initialized()
        return cls._int_to_id.get(internal_id)

    @classmethod
    def count(cls) -> int:
        cls.ensure_initialized()
        return len(cls._id_to_int)

    @classmethod
    def reset(cls):
        cls._id_to_int.clear()
        cls._int_to_id.clear()
        cls._id_to_norm.clear()
        cls._initialized = False
