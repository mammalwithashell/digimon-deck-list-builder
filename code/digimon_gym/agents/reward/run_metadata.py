"""Reward-profile run-metadata sidecar (`reward_profiles.meta.json`).

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Run reproducibility via profile name and content hash".

The sidecar is written next to the model artifacts at run-start and
records the 4 spec-mandated fields:

    {
      "reward_profiles_path":         "<configured path>",
      "reward_profiles_hash":         "sha256:<hex>",
      "reward_profile_override":      <str | null>,
      "reward_assignments_snapshot":  { <arch>: <profile>, ... }
    }

The sidecar is separate from `<model>.meta.json` so:
  1. The existing `TrainingRunMetadata` schema stays stable.
  2. The resume-hash check has a focused source of truth.
  3. Tests can read the sidecar without parsing the larger metadata.

Resume semantics (spec):
  - On resume, the runner loads the sidecar and compares
    `reward_profiles_hash` against the current loader's hash.
  - On mismatch: raise `RewardProfilesHashMismatchError` with both
    hashes named, unless the override flag is set.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Optional


SIDECAR_FILENAME = "reward_profiles.meta.json"


class RewardProfilesHashMismatchError(RuntimeError):
    """Raised when a resume's current profile hash differs from the
    checkpoint's recorded hash. Message names both hashes and the
    override flag to use.
    """

    def __init__(self, checkpoint_hash: str, current_hash: str) -> None:
        super().__init__(
            f"Reward profiles changed since checkpoint.\n"
            f"  Checkpoint hash: {checkpoint_hash}\n"
            f"  Current hash:    {current_hash}\n"
            f"Pass --reward-profiles-override-mismatch to proceed anyway."
        )
        self.checkpoint_hash = checkpoint_hash
        self.current_hash = current_hash


def write_sidecar(
    run_dir: Path,
    *,
    reward_profiles_path: str,
    reward_profiles_hash: str,
    reward_profile_override: Optional[str],
    reward_assignments_snapshot: Dict[str, str],
) -> Path:
    """Write the 4-field sidecar at `<run_dir>/reward_profiles.meta.json`.

    Returns the sidecar path. Creates `run_dir` if missing.
    """
    run_dir.mkdir(parents=True, exist_ok=True)
    payload = {
        "reward_profiles_path": reward_profiles_path,
        "reward_profiles_hash": reward_profiles_hash,
        "reward_profile_override": reward_profile_override,
        "reward_assignments_snapshot": dict(reward_assignments_snapshot),
    }
    sidecar = run_dir / SIDECAR_FILENAME
    sidecar.write_text(
        json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False),
        encoding="utf-8",
    )
    return sidecar


def read_sidecar(run_dir: Path) -> Optional[Dict[str, Any]]:
    """Read the sidecar if present; return None when missing. Raises
    `RuntimeError` on parse failure (corrupted sidecar should not be
    silently ignored — that defeats the reproducibility guarantee).
    """
    sidecar = run_dir / SIDECAR_FILENAME
    if not sidecar.exists():
        return None
    try:
        return json.loads(sidecar.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as e:
        raise RuntimeError(
            f"Failed to read reward profile sidecar at {sidecar}: {e}"
        ) from e


def check_resume_hash(
    checkpoint_run_dir: Path,
    current_hash: str,
    *,
    override_mismatch: bool = False,
) -> None:
    """Compare the resume target's recorded hash against current.

    - When the sidecar is missing, no-op (legacy checkpoints without
      a sidecar are silently allowed; future runs WILL write one).
    - When the hashes match, no-op.
    - When they differ AND `override_mismatch=False`, raise
      `RewardProfilesHashMismatchError`.
    - When they differ AND `override_mismatch=True`, return silently
      (the caller is responsible for writing a new sidecar with the
      current hash, which `write_sidecar` does at run-start).
    """
    snap = read_sidecar(checkpoint_run_dir)
    if snap is None:
        return
    checkpoint_hash = snap.get("reward_profiles_hash")
    if not checkpoint_hash or checkpoint_hash == current_hash:
        return
    if override_mismatch:
        return
    raise RewardProfilesHashMismatchError(checkpoint_hash, current_hash)
