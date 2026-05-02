"""Training run metadata and metrics — file-based, no DB dependencies.

Writes JSON sidecar files alongside trained models so a future training
UI can display run history, per-archetype win rates, and matchup matrices
without any database.
"""

from __future__ import annotations

from dataclasses import dataclass, field, asdict
from pathlib import Path
import json


@dataclass
class ArchetypeResult:
    """Win rate of a trained agent against a specific archetype."""

    archetype_name: str
    games_played: int = 0
    wins: int = 0
    losses: int = 0
    draws: int = 0
    win_rate: float = 0.0
    meta_share: float = 0.0
    threat_index: float = 0.0


@dataclass
class MatchupRecord:
    """Archetype-vs-archetype win rate from evaluation."""

    archetype_a: str
    archetype_b: str
    games_played: int = 0
    a_wins: int = 0
    b_wins: int = 0
    draws: int = 0
    a_win_rate: float = 0.0


@dataclass
class TrainingRunMetadata:
    """Complete training run output — config, per-archetype results, matchups."""

    run_id: str
    started_at: str
    finished_at: str | None = None
    algorithm: str = "PPO"
    total_timesteps: int = 0
    deck_name: str = ""
    opponent_type: str = ""
    model_path: str = ""
    tensorboard_log_dir: str = ""
    hyperparameters: dict = field(default_factory=dict)
    observation_profile: str = ""
    tensor_version: int = 0
    feature_schema_version: str = ""
    tensor_size: int = 0
    tensor_layout_hash: str = ""
    action_space_size: int = 0
    card_registry_capacity: int = 0
    embedding_dim: int = 0

    # Aggregate results
    final_win_rate: float | None = None
    final_mean_reward: float | None = None
    total_games: int = 0

    # Per-archetype win rates (populated during gauntlet/evaluation)
    archetype_results: list[ArchetypeResult] = field(default_factory=list)

    # Archetype-vs-archetype matchup matrix (populated after round-robin eval)
    matchup_records: list[MatchupRecord] = field(default_factory=list)

    # Tournament ranking (ETWR) — matches gauntlet_orchestrator output format
    tournament_rankings: list[dict] = field(default_factory=list)

    # Readiness infrastructure metadata
    eval_suite_results: dict = field(default_factory=dict)
    checkpoint_timestamps: list[dict] = field(default_factory=list)

    def save(self, path: Path) -> None:
        """Write metadata to a JSON file."""
        path.write_text(json.dumps(asdict(self), indent=2))

    @classmethod
    def load(cls, path: Path) -> TrainingRunMetadata:
        """Load metadata from a JSON file."""
        data = json.loads(path.read_text())
        data["archetype_results"] = [
            ArchetypeResult(**r) for r in data.get("archetype_results", [])
        ]
        data["matchup_records"] = [
            MatchupRecord(**m) for m in data.get("matchup_records", [])
        ]
        return cls(**data)
