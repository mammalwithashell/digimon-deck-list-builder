"""Training run metadata and metrics — file-based, no DB dependencies.

Writes JSON sidecar files alongside trained models so a future training
UI can display run history, per-archetype win rates, and matchup matrices
without any database.
"""

from __future__ import annotations

from dataclasses import dataclass, field, asdict
from pathlib import Path
import json
import subprocess


def resolve_git_sha(cwd: Path | None = None) -> str:
    """Best-effort `git rev-parse HEAD` for run provenance.

    Returns the literal "unknown" when git is unavailable or the working
    directory is not inside a git repository (e.g. cloud training images
    shipped without `.git`). Never raises.
    """
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=str(cwd) if cwd is not None else None,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return "unknown"
    sha = result.stdout.strip()
    if result.returncode != 0 or not sha:
        return "unknown"
    return sha


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
    training_mode: str = "standard"
    sampling_policy: str = ""
    training_seed: int | None = None
    curriculum_seed: int | None = None
    eval_seed: int | None = None
    deck_pool_snapshot_path: str = ""
    deck_pool_snapshot_hash: str = ""
    eligible_archetypes: list[str] = field(default_factory=list)
    eligible_deck_count: int = 0
    base_checkpoint: str = ""
    fine_tune_deck_config: dict = field(default_factory=dict)

    # Aggregate results
    final_win_rate: float | None = None
    final_mean_reward: float | None = None
    final_draw_rate: float | None = None
    final_mean_eval_terminal_score: float | None = None
    final_mean_eval_dense_reward: float | None = None
    final_mean_eval_episode_length: float | None = None
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

    # Digivolve reward shaping config (persisted from TrainingConfig so
    # downstream tooling can filter/group shaped vs. unshaped runs without
    # introspecting the hyperparameters dict). Defaults are zero/False so
    # pre-feature sidecars round-trip with correct unshaped semantics; the
    # numeric defaults differ from TrainingConfig (0.1 / 3.9) on purpose —
    # legacy sidecars must NOT be mis-tagged as 'shaped at default values'.
    # See docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
    digivolve_shaping: bool = False
    digivolve_reward: float = 0.0
    dna_digivolve_bonus: float = 0.0
    # BO3 match training (`add-bo3-match-training`). Default `"single"` so
    # pre-feature sidecars load with the correct semantics.
    match_format: str = "single"

    # Universal gameplay reward shape (`add-gameplay-reward-config`).
    # Persisted from TrainingConfig + the loaded ProfileLoader so
    # downstream tooling can distinguish paired/baseline runs by
    # gameplay-yaml content (path + hash) without re-loading the YAML.
    # Empty defaults preserve backward-compat with pre-feature sidecars.
    reward_gameplay_path: str = ""
    reward_gameplay_hash: str = ""

    # Run provenance (`harden-training-pipeline` D5). `git_sha` is the
    # best-effort HEAD commit at run start ("unknown" outside a git
    # checkout). Bounty knobs are persisted so gauntlet runs are
    # auditable after the fact. Defaults preserve backward-compat with
    # pre-feature sidecars.
    git_sha: str = ""
    bounty_threshold: float = 0.0
    bounty_bonus: float = 0.0
    # Action-space sub-range boundaries
    # (SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START,
    # BREEDING_SOURCE_SELECT_END) — validated on resume_from/init_from so
    # an engine change that moves boundaries while keeping the total
    # action-space size constant cannot silently load. Empty list on
    # pre-feature sidecars (validation warns and skips).
    action_space_structure: list[int] = field(default_factory=list)
    # Path of the opponent-pool manifest when opponent="pool"
    # (`harden-training-pipeline` D3 — registry-derived curricula).
    opponent_pool_manifest: str = ""

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
