"""Versioned hyperparameter config for pilot training."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

import yaml


VALID_ALGORITHMS = {"mlp", "lstm"}
VALID_OPPONENTS = {"greedy", "random", "agent", "pool", "league"}

# `harden-training-pipeline` D1: opponent="self-play" is retired. DigimonEnv
# builds observations from Player 1's perspective only, so the old mode (which
# simply skipped OpponentWrapper) had the agent picking Player 2's actions
# against wrong-perspective input. The 2026-05-31 self-play run collapsed
# (22.5% vs greedy at 500k steps) while reporting a flat 100% in-run win rate.
SELF_PLAY_RETIRED_MSG = (
    "opponent='self-play' is retired: DigimonEnv observations are built from "
    "Player 1's perspective only, so self-play (which skipped OpponentWrapper) "
    "made the agent select Player 2's actions against wrong-perspective input, "
    "silently corrupting the policy. Use pool-based fictitious self-play "
    "instead: opponent='pool' with a champion-derived manifest "
    "(python code/tools/champion_admin.py emit-pool --out pool.json), "
    "then --opponent pool --opponent-pool-manifest pool.json. "
    "See docs/TRAINING_RUNBOOK.md and openspec/changes/harden-training-pipeline."
)
VALID_RECORD_GAME_MODES = {"off", "all", "sampled", "draws", "anomalies", "eval"}
VALID_MULLIGAN_LOG_MODES = {"on", "off"}
VALID_EVAL_GAME_LOG_MODES = {"on", "off"}
VALID_MATCH_FORMATS = {"bo3", "single"}


@dataclass
class TrainingConfig:
    algorithm: str = "mlp"
    timesteps: int = 500_000
    seed: int = 0
    n_envs: int = 1
    # Vectorized-env backend. "subproc" spawns one OS process per env
    # (true multi-core rollout collection via SubprocVecEnv); "dummy" steps
    # all envs serially in the training process (DummyVecEnv). MUST be a
    # declared field — pilot_training.make_vec_env reads it via getattr, and
    # from_yaml drops any override key that isn't a dataclass field, so an
    # undeclared value silently falls back to "dummy" (the historical bug:
    # `--set vec_env_backend=subproc` never engaged → every run ran serial).
    vec_env_backend: str = "dummy"
    learning_rate: float = 3e-4
    # Learning-rate schedule: "constant" (default) or "linear" (decay base LR to 0
    # over the run). Used by the deck-specialist league's per-round decay.
    lr_schedule: str = "constant"
    n_steps: int = 2048
    batch_size: int = 64
    n_epochs: int = 10
    gamma: float = 0.99
    gae_lambda: float = 0.95
    clip_range: float = 0.2
    ent_coef: float = 0.01
    vf_coef: float = 0.5
    max_grad_norm: float = 0.5
    lstm_hidden_size: int = 256
    opponent: str = "greedy"
    opponent_pool_manifest: Optional[str] = None
    opponent_pool_mode: str = "pfsp"
    # Deck-specialist league (`add-deck-specialist-league`): round-pool manifest
    # for `opponent="league"`, carrying coupled (policy, deck) opponents. The
    # opponent's deck AND policy are both taken from each entry (unlike the pool
    # path, where the deck comes from the deck-pool wrapper).
    league_pool_manifest: Optional[str] = None
    deck_pool_variants: bool = False
    gauntlet_path: Optional[str] = None
    eval_freq: int = 25_000
    eval_episodes: int = 50
    eval_suite: Optional[str] = None
    # In-training anchored eval (`harden-training-pipeline` D2): every
    # `anchored_eval_freq` steps, play `anchored_eval_games` seat-balanced
    # games per anchor (greedy + layout-compatible champions from
    # `champions_registry`). 0 disables the panel entirely.
    anchored_eval_freq: int = 100_000
    anchored_eval_games: int = 24
    champions_registry: str = "models/champions/registry.json"
    checkpoint_every: int = 50_000
    keep_last_checkpoints: int = 3
    resume_from: Optional[str] = None
    init_from: Optional[str] = None
    generalist: bool = False
    curriculum_seed: Optional[int] = None
    eval_seed: Optional[int] = None
    curriculum_pool: Optional[str] = None
    curriculum_pool_out: Optional[str] = None
    # Declared scope for the eligible archetype set. Intersected with the
    # DSL-implemented safety floor at load time. Applies to both generalist
    # mode (filters the deck pool) and gauntlet mode (filters opponents).
    # Names are canonicalized via the archetype alias index, so aliases like
    # "Red Hybrid" resolve to the canonical "Red Hybrid (AncientGreymon)".
    allowed_archetypes: Optional[List[str]] = None
    models_dir: str = "models"
    tensorboard_log: str = "runs/pilot_ppo"
    run_name: Optional[str] = None
    tensor_profile: str = "standard_lite_v2"
    record_games: str = "off"
    record_games_dir: Optional[str] = None
    record_game_tensors: bool = False
    record_games_max: int = 25
    record_games_sample_rate: float = 0.01
    # When set via YAML, quote the value ("on" / "off") — unquoted `off`/`on`
    # are YAML 1.1 booleans and would fail validation as bool literals.
    mulligan_log: str = "on"
    # Per-game eval-game-log emission. See
    # openspec/changes/add-per-game-eval-log/. Writes one row per
    # completed eval game to models/<run>/eval_game_log.jsonl.
    eval_game_log: str = "on"
    # Digivolve reward shaping (asymmetric — agent only, never opponent).
    # All three default OFF/zero so existing runs are byte-identical when
    # users don't set them. See
    # docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
    digivolve_shaping: bool = False
    digivolve_reward: float = 0.1       # per regular digivolve
    dna_digivolve_bonus: float = 3.9    # additional on top of digivolve_reward
    # Best-of-three match training (`add-bo3-match-training`).
    # `bo3`: one Gym episode = one BO3 match (up to 3 games). Concede
    #   (action 93) and SelectPlayOrder (actions 94/95) enabled.
    # `single`: legacy behavior — one Gym episode = one game.
    match_format: str = "bo3"
    # Reward profiles (`add-reward-profiles` Group 8 wiring).
    # When `reward_profiles_path` resolves to a readable YAML file, the
    # pilot_training env factories wrap the env in `RewardProfileWrapper`.
    # The override (when set) takes precedence over per-archetype
    # assignment — useful for fixed-deck training or for forcing a
    # specific profile during eval. `reward_profiles_hot_reload`
    # toggles the mtime-check + reload at each env.reset().
    reward_profiles_path: str = "code/digimon_gym/agents/reward/profiles.yaml"
    # Universal gameplay reward shape (`add-gameplay-reward-config`). Loads
    # alongside `reward_profiles_path` via the two-file ProfileLoader.
    # Defines the single `gameplay` profile every archetype overlay inherits.
    reward_gameplay_path: str = "code/digimon_gym/agents/reward/gameplay.yaml"
    reward_profile_override: Optional[str] = None
    reward_profiles_hot_reload: bool = True

    def __post_init__(self) -> None:
        self._validate()

    @classmethod
    def from_yaml(
        cls,
        path: Path,
        overrides: Optional[Dict[str, Any]] = None,
    ) -> "TrainingConfig":
        raw = yaml.safe_load(Path(path).read_text()) or {}
        merged = {**raw, **(overrides or {})}
        known = {k: v for k, v in merged.items() if k in cls.__dataclass_fields__}
        cfg = cls(**known)
        cfg._validate()
        return cfg

    def _validate(self) -> None:
        if self.algorithm not in VALID_ALGORITHMS:
            raise ValueError(
                f"algorithm must be one of {sorted(VALID_ALGORITHMS)}, got {self.algorithm}"
            )
        if self.opponent == "self-play":
            raise ValueError(SELF_PLAY_RETIRED_MSG)
        if self.opponent not in VALID_OPPONENTS:
            raise ValueError(
                f"opponent must be one of {sorted(VALID_OPPONENTS)}, got {self.opponent}"
            )
        if self.timesteps <= 0:
            raise ValueError("timesteps must be > 0")
        if self.n_envs < 1:
            raise ValueError("n_envs must be >= 1")
        if self.vec_env_backend not in {"dummy", "subproc"}:
            raise ValueError(
                f"vec_env_backend must be 'dummy' or 'subproc', got {self.vec_env_backend!r}"
            )
        if self.n_steps <= 0:
            raise ValueError("n_steps must be > 0")
        if self.batch_size <= 0:
            raise ValueError("batch_size must be > 0")
        if self.eval_freq < 0:
            raise ValueError("eval_freq must be >= 0")
        if self.eval_episodes <= 0:
            raise ValueError("eval_episodes must be > 0")
        if self.anchored_eval_freq < 0:
            raise ValueError("anchored_eval_freq must be >= 0 (0 disables)")
        if self.anchored_eval_games <= 0:
            raise ValueError("anchored_eval_games must be > 0")
        if self.checkpoint_every < 0:
            raise ValueError("checkpoint_every must be >= 0")
        if self.keep_last_checkpoints < 1:
            raise ValueError("keep_last_checkpoints must be >= 1")
        if self.opponent == "pool" and not self.opponent_pool_manifest:
            raise ValueError("opponent=pool requires opponent_pool_manifest")
        if self.opponent == "league" and not self.league_pool_manifest:
            raise ValueError("opponent=league requires league_pool_manifest")
        if self.lr_schedule not in {"constant", "linear"}:
            raise ValueError(
                f"lr_schedule must be 'constant' or 'linear', got {self.lr_schedule!r}"
            )
        if not isinstance(self.tensor_profile, str) or not self.tensor_profile.strip():
            raise ValueError("tensor_profile must be a non-blank string")
        if self.record_games not in VALID_RECORD_GAME_MODES:
            raise ValueError(
                f"record_games must be one of {sorted(VALID_RECORD_GAME_MODES)}, "
                f"got {self.record_games}"
            )
        if self.mulligan_log not in VALID_MULLIGAN_LOG_MODES:
            raise ValueError(
                f"mulligan_log must be one of {sorted(VALID_MULLIGAN_LOG_MODES)}, "
                f"got {self.mulligan_log}"
            )
        if self.eval_game_log not in VALID_EVAL_GAME_LOG_MODES:
            raise ValueError(
                f"eval_game_log must be one of {sorted(VALID_EVAL_GAME_LOG_MODES)}, "
                f"got {self.eval_game_log}"
            )
        if self.record_games_max < 0:
            raise ValueError("record_games_max must be >= 0")
        if not 0.0 <= self.record_games_sample_rate <= 1.0:
            raise ValueError("record_games_sample_rate must be between 0 and 1")
        if self.digivolve_reward < 0:
            raise ValueError("digivolve_reward must be >= 0")
        if self.dna_digivolve_bonus < 0:
            raise ValueError("dna_digivolve_bonus must be >= 0")
        # `add-reward-profiles` deprecation: legacy flat shaping fields.
        # When set to a non-default value, the user is asking for a
        # specific shaping that the reward profiles framework now
        # subsumes. Warn them toward the v2 surface — but DON'T remove
        # the fields yet (the runner still honors them via the
        # `_digivolve_shaped` profile when `digivolve_shaping=True`).
        if self.digivolve_reward != 0.1:
            import warnings as _w
            _w.warn(
                f"`digivolve_reward = {self.digivolve_reward}` is deprecated. "
                "Define a custom reward profile in profiles.yaml (e.g., inherit "
                "from `_digivolve_shaped` and override the `digivolve` "
                "component weight) and select it via `reward_profile_override` "
                "or per-archetype assignment instead. The flat field is "
                "scheduled for removal in v2.",
                DeprecationWarning,
                stacklevel=2,
            )
        if self.dna_digivolve_bonus != 3.9:
            import warnings as _w
            _w.warn(
                f"`dna_digivolve_bonus = {self.dna_digivolve_bonus}` is deprecated. "
                "Define a custom reward profile that overrides the `dna_digivolve` "
                "component weight and select it via `reward_profile_override` "
                "or per-archetype assignment instead. The flat field is "
                "scheduled for removal in v2.",
                DeprecationWarning,
                stacklevel=2,
            )
        if self.match_format not in VALID_MATCH_FORMATS:
            raise ValueError(
                f"match_format must be one of {sorted(VALID_MATCH_FORMATS)}, "
                f"got {self.match_format}"
            )

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)
