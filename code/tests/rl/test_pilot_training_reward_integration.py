"""Pilot-training integration tests for the reward profiles pipeline.

Group 8 task 8.8: verify that with default config:
- The reward profile factory is active and points at the shipped YAML.
- `_build_reward_profile_factory(cfg)` returns a wrapper-applier whose
  `loader` matches the shipped `profiles.yaml`.
- `digivolve_shaping=True` maps to override `_digivolve_shaped`.
- Explicit `reward_profile_override` takes precedence over the
  digivolve-shaping fallback.
- Sidecar write + read + resume-hash check round-trip cleanly.
- The CLI argparser exposes `--reward-profiles-override-mismatch`.

Avoids running an actual SB3 training loop (slow + heavy deps); the
end-to-end "factory wraps a real DigimonEnv and produces a real
reward_breakdown" check was already covered in the Group 7 REPL smoke
test and the wrapper-level tests in test_reward_profile_wrapper.py.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from digimon_gym.agents.pilot_training import (
    _build_argparser,
    _build_reward_profile_factory,
)
from digimon_gym.agents.reward.profile_loader import ProfileLoader
from digimon_gym.agents.reward.run_metadata import (
    RewardProfilesHashMismatchError,
    SIDECAR_FILENAME,
    check_resume_hash,
    write_sidecar,
)
from digimon_gym.agents.reward.wrapper import RewardProfileWrapper
from digimon_gym.agents.training_config import TrainingConfig


# ============================================================================
# Factory
# ============================================================================


def test_default_config_factory_is_active_and_points_at_shipped_yaml():
    cfg = TrainingConfig()
    factory = _build_reward_profile_factory(cfg)
    assert factory.active is True
    assert factory.loader is not None
    assert factory.loader.profiles_path == Path("code/digimon_gym/agents/reward/profiles.yaml")
    assert factory.loader.gameplay_path == Path("code/digimon_gym/agents/reward/gameplay.yaml")
    # Shipped YAML defines these profiles (add-gameplay-reward-config:
    # `_digivolve_shaped` removed; `gameplay` is the new universal root).
    by_name = factory.loader.profiles.by_name
    assert "gameplay" in by_name
    assert "_default" in by_name
    assert "dna_omnimon_combo_v1" in by_name


def test_factory_inactive_when_profiles_path_missing():
    cfg = TrainingConfig(reward_profiles_path="does/not/exist.yaml")
    factory = _build_reward_profile_factory(cfg)
    assert factory.active is False
    assert factory.loader is None
    assert factory.effective_override is None


def test_factory_loader_failure_raises_at_run_start():
    with tempfile.TemporaryDirectory() as tmp:
        bad = Path(tmp) / "broken.yaml"
        bad.write_text("invalid: ::: yaml ::\n", encoding="utf-8")
        cfg = TrainingConfig(reward_profiles_path=str(bad))
        with pytest.raises(RuntimeError, match=r"Failed to load reward profiles"):
            _build_reward_profile_factory(cfg)


# ============================================================================
# digivolve_shaping ↔ _digivolve_shaped alias
# ============================================================================


def test_digivolve_shaping_true_is_now_inert():
    """add-gameplay-reward-config §D9: `digivolve_shaping=True` is silently
    no-op. The _digivolve_shaped profile is removed; the universal
    gameplay default already carries digivolve weights, so the flag has
    nothing to remap to."""
    cfg = TrainingConfig(digivolve_shaping=True)
    factory = _build_reward_profile_factory(cfg)
    assert factory.effective_override is None


def test_explicit_override_supersedes_digivolve_shaping_alias():
    cfg = TrainingConfig(
        digivolve_shaping=True,
        reward_profile_override="dna_omnimon_combo_v1",
    )
    factory = _build_reward_profile_factory(cfg)
    assert factory.effective_override == "dna_omnimon_combo_v1"


def test_digivolve_shaping_false_default_yields_no_override():
    cfg = TrainingConfig()  # digivolve_shaping defaults to False
    factory = _build_reward_profile_factory(cfg)
    assert factory.effective_override is None


# ============================================================================
# Wrapper integration via the factory
# ============================================================================


def test_factory_wraps_a_real_env_with_reward_profile_wrapper():
    import gymnasium
    from gymnasium import spaces

    class _DummyEnv(gymnasium.Env):
        observation_space = spaces.Box(low=0.0, high=1.0, shape=(1,))
        action_space = spaces.Discrete(1)
        def reset(self, *, seed=None, options=None):
            return self.observation_space.sample() * 0, {}
        def step(self, action):
            return self.observation_space.sample() * 0, 0.0, False, False, {"reward_occurrences": []}

    cfg = TrainingConfig()
    factory = _build_reward_profile_factory(cfg)
    wrapped = factory(_DummyEnv())
    assert isinstance(wrapped, RewardProfileWrapper)


# ============================================================================
# Sidecar + resume-hash integration
# ============================================================================


def test_sidecar_write_then_read_round_trips_all_six_fields(tmp_path: Path):
    cfg = TrainingConfig()
    factory = _build_reward_profile_factory(cfg)
    assert factory.active
    loader: ProfileLoader = factory.loader

    sidecar = write_sidecar(
        tmp_path,
        reward_profiles_path=cfg.reward_profiles_path,
        reward_profiles_hash=loader.profiles.profiles_hash,
        reward_gameplay_path=cfg.reward_gameplay_path,
        reward_gameplay_hash=loader.profiles.gameplay_hash,
        reward_profile_override=factory.effective_override,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )
    assert sidecar.exists()
    assert sidecar.name == SIDECAR_FILENAME

    import json
    payload = json.loads(sidecar.read_text(encoding="utf-8"))
    assert set(payload) == {
        "reward_gameplay_path",
        "reward_gameplay_hash",
        "reward_profiles_path",
        "reward_profiles_hash",
        "reward_profile_override",
        "reward_assignments_snapshot",
    }
    assert payload["reward_profiles_hash"].startswith("sha256:")
    assert payload["reward_gameplay_hash"].startswith("sha256:")
    assert payload["reward_assignments_snapshot"]["_default"] == "_default"


def test_resume_hash_check_names_gameplay_file_on_mismatch(tmp_path: Path):
    """Two-file `add-gameplay-reward-config`: when the gameplay-yaml
    hash drifts, the error MUST name `gameplay.yaml` so operators see
    which file changed without grepping."""
    write_sidecar(
        tmp_path,
        reward_profiles_path="profiles.yaml",
        reward_profiles_hash="sha256:profiles_unchanged",
        reward_gameplay_path="gameplay.yaml",
        reward_gameplay_hash="sha256:gameplay_old",
        reward_profile_override=None,
        reward_assignments_snapshot={"_default": "_default"},
    )
    with pytest.raises(RewardProfilesHashMismatchError) as exc:
        check_resume_hash(
            tmp_path,
            "sha256:profiles_unchanged",
            current_gameplay_hash="sha256:gameplay_NEW",
        )
    assert exc.value.file_name == "gameplay.yaml"
    assert "gameplay.yaml" in str(exc.value)
    assert "sha256:gameplay_old" in str(exc.value)


def test_resume_hash_check_names_profiles_file_on_mismatch(tmp_path: Path):
    """When only the profiles-yaml hash drifts, the error MUST name
    `profiles.yaml`."""
    write_sidecar(
        tmp_path,
        reward_profiles_path="profiles.yaml",
        reward_profiles_hash="sha256:profiles_old",
        reward_gameplay_path="gameplay.yaml",
        reward_gameplay_hash="sha256:gameplay_unchanged",
        reward_profile_override=None,
        reward_assignments_snapshot={"_default": "_default"},
    )
    with pytest.raises(RewardProfilesHashMismatchError) as exc:
        check_resume_hash(
            tmp_path,
            "sha256:profiles_NEW",
            current_gameplay_hash="sha256:gameplay_unchanged",
        )
    assert exc.value.file_name == "profiles.yaml"
    assert "profiles.yaml" in str(exc.value)


def test_resume_hash_check_raises_on_mismatch_without_override(tmp_path: Path):
    write_sidecar(
        tmp_path,
        reward_profiles_path="anywhere.yaml",
        reward_profiles_hash="sha256:checkpoint_hash",
        reward_profile_override=None,
        reward_assignments_snapshot={"_default": "_default"},
    )
    with pytest.raises(RewardProfilesHashMismatchError) as exc:
        check_resume_hash(tmp_path, "sha256:different_hash")
    msg = str(exc.value)
    assert "sha256:checkpoint_hash" in msg
    assert "sha256:different_hash" in msg
    assert "--reward-profiles-override-mismatch" in msg


def test_resume_hash_check_passes_when_hashes_match(tmp_path: Path):
    write_sidecar(
        tmp_path,
        reward_profiles_path="x.yaml",
        reward_profiles_hash="sha256:abc",
        reward_profile_override=None,
        reward_assignments_snapshot={"_default": "_default"},
    )
    # Same hash → no exception.
    check_resume_hash(tmp_path, "sha256:abc")


def test_resume_hash_check_bypasses_with_override_flag(tmp_path: Path):
    write_sidecar(
        tmp_path,
        reward_profiles_path="x.yaml",
        reward_profiles_hash="sha256:abc",
        reward_profile_override=None,
        reward_assignments_snapshot={"_default": "_default"},
    )
    # Override → no exception even on mismatch.
    check_resume_hash(tmp_path, "sha256:DIFF", override_mismatch=True)


def test_resume_hash_check_missing_sidecar_is_no_op(tmp_path: Path):
    # Legacy checkpoint with no sidecar — must not break resume.
    check_resume_hash(tmp_path, "sha256:anything")


# ============================================================================
# CLI flag
# ============================================================================


def test_cli_argparser_exposes_reward_profiles_override_mismatch():
    parser = _build_argparser()
    args = parser.parse_args(["--config", "dummy.yaml"])
    # Default False.
    assert args.reward_profiles_override_mismatch is False

    args = parser.parse_args(
        ["--config", "dummy.yaml", "--reward-profiles-override-mismatch"],
    )
    assert args.reward_profiles_override_mismatch is True
