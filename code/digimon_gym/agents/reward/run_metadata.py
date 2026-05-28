"""Reward-profile run-metadata sidecar (`reward_profiles.meta.json`).

Spec:
- `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
  ("Run reproducibility via profile name and content hash")
- `openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
  ("Sidecar persists gameplay hash separately")

The sidecar is written next to the model artifacts at run-start and
records 6 fields under the two-file architecture (4 archetype-overlay
fields + 2 gameplay fields):

    {
      "reward_gameplay_path":         "<configured gameplay.yaml path>",
      "reward_gameplay_hash":         "sha256:<hex>",
      "reward_profiles_path":         "<configured profiles.yaml path>",
      "reward_profiles_hash":         "sha256:<hex>",
      "reward_profile_override":      <str | null>,
      "reward_assignments_snapshot":  { <arch>: <profile>, ... }
    }

Pre-`add-gameplay-reward-config` sidecars (4 fields, no gameplay-*
entries) are still readable for backward compatibility. The resume
check treats missing `reward_gameplay_hash` as "checkpoint pre-dates
the gameplay split — only compare profiles_hash".

The sidecar is separate from `<model>.meta.json` so:
  1. The existing `TrainingRunMetadata` schema stays stable.
  2. The resume-hash check has a focused source of truth.
  3. Tests can read the sidecar without parsing the larger metadata.

Resume semantics (spec):
  - On resume, the runner loads the sidecar and compares BOTH the
    gameplay-hash and the profiles-hash against the current loader's
    hashes.
  - On mismatch in either file: raise
    `RewardProfilesHashMismatchError` naming which file drifted +
    both hashes, unless the override flag is set.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Optional


SIDECAR_FILENAME = "reward_profiles.meta.json"


class RewardProfilesHashMismatchError(RuntimeError):
    """Raised when a resume's current profile hash differs from the
    checkpoint's recorded hash. Message names BOTH hashes AND which
    file drifted (`file_name` field — "gameplay.yaml" or "profiles.yaml")
    so the operator can fix the relevant file or pass the override flag.
    """

    def __init__(
        self,
        checkpoint_hash: str,
        current_hash: str,
        *,
        file_name: str = "profiles.yaml",
    ) -> None:
        # `file_name` defaults to "profiles.yaml" so pre-`add-gameplay-
        # reward-config` constructions (which only checked the single
        # hash) continue to produce a sensible message.
        super().__init__(
            f"Reward {file_name} changed since checkpoint.\n"
            f"  Checkpoint hash: {checkpoint_hash}\n"
            f"  Current hash:    {current_hash}\n"
            f"Pass --reward-profiles-override-mismatch to proceed anyway."
        )
        self.checkpoint_hash = checkpoint_hash
        self.current_hash = current_hash
        self.file_name = file_name


def write_sidecar(
    run_dir: Path,
    *,
    reward_profiles_path: str,
    reward_profiles_hash: str,
    reward_profile_override: Optional[str],
    reward_assignments_snapshot: Dict[str, str],
    reward_gameplay_path: str = "",
    reward_gameplay_hash: str = "",
) -> Path:
    """Write the 6-field sidecar at `<run_dir>/reward_profiles.meta.json`.

    `reward_gameplay_path` and `reward_gameplay_hash` default to empty
    strings for backward compatibility — callers that haven't migrated
    to the two-file loader continue to write 4-field sidecars. New
    callers (pilot_training) SHALL pass both gameplay fields.

    Returns the sidecar path. Creates `run_dir` if missing.
    """
    run_dir.mkdir(parents=True, exist_ok=True)
    payload = {
        "reward_gameplay_path": reward_gameplay_path,
        "reward_gameplay_hash": reward_gameplay_hash,
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
    current_gameplay_hash: str = "",
) -> None:
    """Compare the resume target's recorded hashes against current.

    Two-file check (when `current_gameplay_hash` is non-empty):
      - Compare `reward_gameplay_hash` AND `reward_profiles_hash`.
      - On any mismatch, raise `RewardProfilesHashMismatchError` with
        `file_name="gameplay.yaml"` or `"profiles.yaml"` to identify
        which one drifted.
      - If both drifted, the gameplay-side error fires first (operators
        usually want to know about the universal shape change first;
        archetype overlay drift is secondary signal).

    Single-hash check (when `current_gameplay_hash=""`):
      - Backward-compat path used by pre-two-file callers. Only the
        profiles-hash is compared.

    Common cases:
      - Sidecar missing → no-op (legacy checkpoints without a sidecar
        are silently allowed; future runs WILL write one).
      - All recorded hashes match current → no-op.
      - Mismatch + `override_mismatch=True` → return silently (the
        caller is responsible for writing a new sidecar with the
        current hashes, which `write_sidecar` does at run-start).
    """
    snap = read_sidecar(checkpoint_run_dir)
    if snap is None:
        return

    # Compare gameplay hash first (universal-shape changes are the
    # operator's first concern). The sidecar field is optional — pre-
    # two-file checkpoints don't have it; skip the gameplay comparison
    # in that case.
    if current_gameplay_hash:
        checkpoint_gameplay_hash = snap.get("reward_gameplay_hash") or ""
        if (
            checkpoint_gameplay_hash
            and checkpoint_gameplay_hash != current_gameplay_hash
        ):
            if not override_mismatch:
                raise RewardProfilesHashMismatchError(
                    checkpoint_gameplay_hash,
                    current_gameplay_hash,
                    file_name="gameplay.yaml",
                )

    # Then the profiles-side hash.
    checkpoint_hash = snap.get("reward_profiles_hash")
    if not checkpoint_hash or checkpoint_hash == current_hash:
        return
    if override_mismatch:
        return
    raise RewardProfilesHashMismatchError(
        checkpoint_hash, current_hash, file_name="profiles.yaml"
    )
