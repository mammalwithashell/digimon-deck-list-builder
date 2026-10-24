"""A failed search health check must not start an expensive generation."""
import json
from pathlib import Path

import pytest

from tools import english_meta_search_job as job


def test_pipeline_stops_before_generation_and_records_failed_check(monkeypatch, tmp_path):
    stages = []
    monkeypatch.setattr(job, "prepare", lambda *args: stages.append("prepare"))
    monkeypatch.setattr(job, "bootstrap", lambda *args: stages.append("bootstrap"))
    monkeypatch.setattr(job, "generation", lambda *args: stages.append("generation"))

    def failed_check(*args):
        stages.append("check")
        raise RuntimeError("unhealthy search games")

    monkeypatch.setattr(job, "check", failed_check)
    with pytest.raises(RuntimeError, match="unhealthy"):
        job.pipeline(tmp_path, Path("cli"), 400, 4, 42)
    assert stages == ["prepare", "bootstrap", "check"]
    state = json.loads((tmp_path / "pipeline_state.json").read_text())
    assert state == {"phase": "check", "status": "failed", "error": "unhealthy search games"}


def test_pipeline_preserves_existing_run_without_starting_work(monkeypatch, tmp_path):
    (tmp_path / "fresh_seed.zip").write_bytes(b"previous checkpoint")
    monkeypatch.setattr(job, "prepare", lambda *args: pytest.fail("must preserve old run"))
    with pytest.raises(FileExistsError, match="fresh directory"):
        job.pipeline(tmp_path, Path("cli"), 400, 4, 42)
    assert (tmp_path / "fresh_seed.zip").read_bytes() == b"previous checkpoint"
