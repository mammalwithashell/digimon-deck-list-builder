"""End-to-end resume-hash-check test for `add-reward-profiles` Group 9
task 9.3.

Spec scenarios covered:
- "Resume fails loudly on hash mismatch"
- "Cosmetic YAML edits do not trigger mismatch"
- (Plus the override-flag bypass path.)

Realistic scenario:
1. Build the loader from the shipped `profiles.yaml`.
2. Write the sidecar from the loaded hash.
3. Copy the YAML, edit a component weight, build a new loader.
4. Resume check raises with both hashes named.
5. Re-attempt with override flag → succeeds.
6. Verify cosmetic edits (whitespace, key reordering) do NOT trigger
   mismatch — the canonical-hash invariant from Group 4 task 4.7.
"""

from __future__ import annotations

import shutil
import textwrap
from pathlib import Path

import pytest

from digimon_gym.agents.reward.profile_loader import ProfileLoader
from digimon_gym.agents.reward.run_metadata import (
    RewardProfilesHashMismatchError,
    check_resume_hash,
    write_sidecar,
)


SHIPPED_PROFILES = Path("code/digimon_gym/agents/reward/profiles.yaml")


def _copy_profiles_to(tmp: Path) -> Path:
    """Copy the shipped profiles.yaml to a writable temp location."""
    dst = tmp / "profiles.yaml"
    shutil.copyfile(SHIPPED_PROFILES, dst)
    return dst


def test_resume_check_passes_when_yaml_unchanged(tmp_path: Path):
    (tmp_path / "step1").mkdir(parents=True, exist_ok=True)
    profiles_path = _copy_profiles_to(tmp_path / "step1")

    loader = ProfileLoader(profiles_path)
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    write_sidecar(
        run_dir,
        reward_profiles_path=str(profiles_path),
        reward_profiles_hash=loader.profiles.content_hash,
        reward_profile_override=None,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )

    # Recompute hash (file unchanged) and verify resume check is a no-op.
    fresh_loader = ProfileLoader(profiles_path)
    check_resume_hash(run_dir, fresh_loader.profiles.content_hash)


def test_resume_check_raises_when_weight_changed(tmp_path: Path):
    # Set up: original YAML + sidecar.
    original_path = _copy_profiles_to(tmp_path)
    loader = ProfileLoader(original_path)
    original_hash = loader.profiles.content_hash

    run_dir = tmp_path / "run"
    run_dir.mkdir()
    write_sidecar(
        run_dir,
        reward_profiles_path=str(original_path),
        reward_profiles_hash=original_hash,
        reward_profile_override=None,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )

    # Now modify the YAML — bump security_remove weight from 1.5 to 1.7.
    text = original_path.read_text(encoding="utf-8")
    modified = text.replace("weight: 1.5", "weight: 1.7", 1)
    assert modified != text, "test setup: expected a 1.5 substring to mutate"
    original_path.write_text(modified, encoding="utf-8")

    # Reload + compute new hash + run resume check.
    new_loader = ProfileLoader(original_path)
    new_hash = new_loader.profiles.content_hash
    assert new_hash != original_hash, (
        "modifying a component weight should produce a different canonical hash"
    )

    # Without override → raises with both hashes named.
    with pytest.raises(RewardProfilesHashMismatchError) as exc:
        check_resume_hash(run_dir, new_hash)
    msg = str(exc.value)
    assert original_hash in msg
    assert new_hash in msg
    assert "--reward-profiles-override-mismatch" in msg


def test_resume_check_bypasses_with_override_flag(tmp_path: Path):
    original_path = _copy_profiles_to(tmp_path)
    loader = ProfileLoader(original_path)
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    write_sidecar(
        run_dir,
        reward_profiles_path=str(original_path),
        reward_profiles_hash=loader.profiles.content_hash,
        reward_profile_override=None,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )

    text = original_path.read_text(encoding="utf-8")
    original_path.write_text(text.replace("weight: 1.5", "weight: 1.7", 1), encoding="utf-8")
    new_loader = ProfileLoader(original_path)

    # With override → silent no-op.
    check_resume_hash(
        run_dir, new_loader.profiles.content_hash, override_mismatch=True,
    )


def test_resume_check_no_op_when_cosmetic_yaml_edit(tmp_path: Path):
    """Spec scenario: 'Cosmetic YAML edits do not trigger mismatch'.

    The canonical hash is computed over PARSED YAML with sorted keys
    and normalized floats. Whitespace / key reordering / trailing
    comments do NOT alter it.
    """
    original_path = _copy_profiles_to(tmp_path)
    loader = ProfileLoader(original_path)
    original_hash = loader.profiles.content_hash

    run_dir = tmp_path / "run"
    run_dir.mkdir()
    write_sidecar(
        run_dir,
        reward_profiles_path=str(original_path),
        reward_profiles_hash=original_hash,
        reward_profile_override=None,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )

    # Cosmetic edit: add blank lines + a comment, preserve semantic content.
    text = original_path.read_text(encoding="utf-8")
    cosmetic = (
        "# A comment added at the top.\n\n"
        + text.replace("profiles:", "profiles:  # inline comment", 1)
    )
    original_path.write_text(cosmetic, encoding="utf-8")

    new_loader = ProfileLoader(original_path)
    new_hash = new_loader.profiles.content_hash
    # Canonical hash invariant: cosmetic edits do not change it.
    assert new_hash == original_hash, (
        "cosmetic YAML edits (comments, blank lines) MUST NOT alter the "
        "canonical content hash — Group 4 task 4.7 contract."
    )

    # Resume check is a no-op.
    check_resume_hash(run_dir, new_hash)


def test_sidecar_assignments_snapshot_reflects_loaded_state(tmp_path: Path):
    """Sanity: the snapshot in the sidecar matches what the loader
    actually resolved. Defends against future code paths that bypass
    the loader's assignment view.
    """
    profiles_path = _copy_profiles_to(tmp_path)
    loader = ProfileLoader(profiles_path)
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    sidecar = write_sidecar(
        run_dir,
        reward_profiles_path=str(profiles_path),
        reward_profiles_hash=loader.profiles.content_hash,
        reward_profile_override=None,
        reward_assignments_snapshot=dict(loader.profiles.assignments),
    )
    import json
    payload = json.loads(sidecar.read_text(encoding="utf-8"))
    assert payload["reward_assignments_snapshot"]["_default"] == "_default"
    # Shipped YAML maps real archetypes too.
    assert (
        payload["reward_assignments_snapshot"].get("DNA Omnimon")
        == "dna_omnimon_combo_v1"
    )
