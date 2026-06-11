"""Versioned pool of past trained agents for self-play and PFSP."""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import List, Optional

import numpy as np


@dataclass
class OpponentEntry:
    name: str
    weights_path: str
    algorithm: str
    win_rate_vs_pool: float
    games_played: int


@dataclass
class OpponentPool:
    manifest_path: Path
    version: int = 1
    entries: List[OpponentEntry] = field(default_factory=list)

    @classmethod
    def load(cls, manifest_path: Path) -> "OpponentPool":
        path = Path(manifest_path)
        if not path.exists():
            return cls(manifest_path=path, entries=[])
        data = json.loads(path.read_text())
        entries = [OpponentEntry(**entry) for entry in data.get("entries", [])]
        return cls(
            manifest_path=path,
            version=int(data.get("version", 1)),
            entries=entries,
        )

    @classmethod
    def from_champion_registry(
        cls,
        registry_path: Path | str,
        layout_hash: str,
        manifest_path: Path | str = Path("opponent_pool.json"),
    ) -> "OpponentPool":
        """Derive a training opponent pool from the champion registry
        (`harden-training-pipeline` D3 — pool-based fictitious self-play).

        Includes every champion whose tensor-layout hash matches
        ``layout_hash``, with uniform weights (``win_rate_vs_pool=0.5``,
        ``games_played=0`` — PFSP reweighting happens at sample time, not
        in the manifest). An empty compatible cohort is an explicit error:
        a silently-empty pool would leave the opponent seat passive.
        """
        from digimon_gym.agents.champion_registry import ChampionRegistry

        registry = ChampionRegistry.load(registry_path)
        compatible = registry.compatible(layout_hash)
        if not compatible:
            raise ValueError(
                f"no champions in {registry_path} match layout hash "
                f"{layout_hash!r} — register one (champion_admin.py promote) "
                "or pick a different opponent mode"
            )
        entries = [
            OpponentEntry(
                name=c.name,
                weights_path=c.weights_path,
                algorithm=c.algorithm,
                win_rate_vs_pool=0.5,
                games_played=0,
            )
            for c in compatible
        ]
        return cls(manifest_path=Path(manifest_path), entries=entries)

    @property
    def size(self) -> int:
        return len(self.entries)

    def register(self, entry: OpponentEntry) -> None:
        if any(existing.name == entry.name for existing in self.entries):
            raise ValueError(f"duplicate opponent name: {entry.name}")
        self.entries.append(entry)

    def get(self, name: str) -> OpponentEntry:
        for entry in self.entries:
            if entry.name == name:
                return entry
        raise KeyError(name)

    def save(self) -> None:
        self.manifest_path.parent.mkdir(parents=True, exist_ok=True)
        self.manifest_path.write_text(
            json.dumps(
                {
                    "version": self.version,
                    "entries": [asdict(entry) for entry in self.entries],
                },
                indent=2,
            )
        )

    def sample(
        self,
        rng_seed: Optional[int] = None,
        mode: str = "uniform",
        power: float = 2.0,
    ) -> OpponentEntry:
        if not self.entries:
            raise RuntimeError("opponent pool is empty")
        rng = np.random.default_rng(rng_seed)
        if mode == "uniform":
            return self.entries[int(rng.integers(0, len(self.entries)))]
        if mode == "pfsp":
            weights = np.array(
                [(1.0 - entry.win_rate_vs_pool) ** power for entry in self.entries],
                dtype=np.float64,
            )
            if float(weights.sum()) == 0.0:
                weights = np.ones_like(weights)
            weights = weights / weights.sum()
            return self.entries[int(rng.choice(len(self.entries), p=weights))]
        raise ValueError(f"unknown sampling mode: {mode}")

    def record_match(self, opponent_name: str, agent_won: bool) -> None:
        entry = self.get(opponent_name)
        new_total = entry.games_played + 1
        prior_opponent_wins = entry.win_rate_vs_pool * entry.games_played
        new_opponent_wins = prior_opponent_wins + (0.0 if agent_won else 1.0)
        entry.win_rate_vs_pool = new_opponent_wins / new_total
        entry.games_played = new_total
