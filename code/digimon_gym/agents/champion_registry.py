"""Champion registry — frozen, versioned model snapshots that serve as
fixed evaluation anchors (Tier 2 of the anchored evaluation frame).

Each champion records enough provenance to (a) load it as an opponent and
(b) verify observation-profile / tensor-layout compatibility before play,
so a model is never scored against an anchor it cannot validly face.

Part of the `add-model-evaluation-harness` change.
"""
from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import List, Optional, Union


#: A candidate is promoted to champion only if it beats the current panel by at
#: least this share over a seat-balanced match (the AlphaGo-Zero gating rule).
PROMOTION_THRESHOLD = 0.55


def _norm_algorithm(raw: str) -> str:
    s = str(raw).lower()
    if s in ("lstm", "maskablerecurrentppo", "recurrent"):
        return "lstm"
    return "mlp"


def should_promote(panel_wins: int, panel_games: int,
                   threshold: float = PROMOTION_THRESHOLD) -> bool:
    """Gate a candidate: promote iff its aggregate win rate over the champion
    panel is at least ``threshold``. No games => not promoted."""
    if panel_games <= 0:
        return False
    return panel_wins / panel_games >= threshold


@dataclass
class Champion:
    """A frozen model snapshot used as a fixed evaluation anchor."""

    name: str
    weights_path: str
    algorithm: str  # "lstm" | "mlp"
    observation_profile: str
    tensor_layout_hash: str
    source: str = ""
    created_at: str = ""

    @classmethod
    def from_meta(
        cls,
        name: str,
        weights_path: Union[str, Path],
        meta: dict,
        source: str = "",
        created_at: str = "",
    ) -> "Champion":
        """Build a Champion from a model's ``.meta.json`` payload."""
        return cls(
            name=name,
            weights_path=str(weights_path),
            algorithm=_norm_algorithm(meta.get("algorithm", "")),
            observation_profile=str(meta.get("observation_profile", "")),
            tensor_layout_hash=str(meta.get("tensor_layout_hash", "")),
            source=source,
            created_at=created_at,
        )


class ChampionRegistry:
    """An ordered, versioned collection of champions persisted as JSON."""

    def __init__(self, path: Union[str, Path], champions: Optional[List[Champion]] = None):
        self.path = Path(path)
        self.champions: List[Champion] = list(champions or [])

    @classmethod
    def load(cls, path: Union[str, Path]) -> "ChampionRegistry":
        p = Path(path)
        if not p.exists():
            return cls(p, [])
        data = json.loads(p.read_text(encoding="utf-8"))
        return cls(p, [Champion(**c) for c in data.get("champions", [])])

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        payload = {"version": 1, "champions": [asdict(c) for c in self.champions]}
        self.path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def names(self) -> List[str]:
        return [c.name for c in self.champions]

    def get(self, name: str) -> Champion:
        for c in self.champions:
            if c.name == name:
                return c
        raise KeyError(name)

    def register(self, champion: Champion) -> None:
        if any(c.name == champion.name for c in self.champions):
            raise ValueError(f"duplicate champion name: {champion.name}")
        self.champions.append(champion)

    def compatible(self, tensor_layout_hash: str) -> List[Champion]:
        """Champions whose tensor layout matches ``tensor_layout_hash`` —
        i.e. those a model on that layout can validly play against."""
        return [c for c in self.champions if c.tensor_layout_hash == tensor_layout_hash]
