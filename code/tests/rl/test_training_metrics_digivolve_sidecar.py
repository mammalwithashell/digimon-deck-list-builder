"""Sidecar round-trip test for digivolve-shaping fields on TrainingRunMetadata."""

from __future__ import annotations

from pathlib import Path

from digimon_gym.agents.training_metrics import TrainingRunMetadata


def test_sidecar_round_trips_digivolve_fields(tmp_path: Path) -> None:
    meta = TrainingRunMetadata(
        run_id="test-run",
        started_at="2026-05-23T00:00:00Z",
        digivolve_shaping=True,
        digivolve_reward=0.1,
        dna_digivolve_bonus=0.3,
    )

    out = tmp_path / "metadata.json"
    meta.save(out)
    loaded = TrainingRunMetadata.load(out)

    assert loaded.digivolve_shaping is True
    assert loaded.digivolve_reward == 0.1
    assert loaded.dna_digivolve_bonus == 0.3


def test_sidecar_legacy_file_loads_with_unshaped_defaults(tmp_path: Path) -> None:
    """A pre-feature sidecar (no digivolve_* keys) must load and produce
    correct unshaped semantics. Note `digivolve_reward` / `dna_digivolve_bonus`
    default to 0.0 on TrainingRunMetadata (not 0.1 / 0.3 as on TrainingConfig)
    so a legacy run is never mis-tagged as 'shaped with default values'."""
    legacy = tmp_path / "legacy.json"
    legacy.write_text('{"run_id": "legacy", "started_at": "2026-04-01T00:00:00Z"}')

    loaded = TrainingRunMetadata.load(legacy)
    assert loaded.digivolve_shaping is False
    assert loaded.digivolve_reward == 0.0
    assert loaded.dna_digivolve_bonus == 0.0
