"""The remote archive step must never capture a checkpoint that is still being written."""
import contextlib
import io
import json
import os
import tarfile
import time
import zipfile

from tools.cloud_search import backup_experiment


def index(root):
    script = backup_experiment.REMOTE_INDEX.replace("/opt/digimon-search/experiment", root.as_posix())
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        exec(compile(script, "remote_index", "exec"), {})
    return json.loads(out.getvalue())


def test_fresh_or_torn_checkpoint_skips_the_archive(tmp_path):
    root = tmp_path / "experiment"
    model = root / "generation/gen0/model.zip"
    model.parent.mkdir(parents=True)
    with zipfile.ZipFile(model, "w") as archive:
        archive.writestr("weights", "ready")
    result = index(root)
    assert result["unstable"] == ["generation/gen0/model.zip"]
    assert result["archive_sha256"] is None
    assert not (tmp_path / "checkpoint-backup.tar.gz").exists()

    model.write_bytes(model.read_bytes()[:-10])  # truncated mid-write
    old = time.time() - 120
    os.utime(model, (old, old))
    assert index(root)["unstable"] == ["generation/gen0/model.zip"]


def test_quiet_valid_checkpoint_is_archived(tmp_path):
    root = tmp_path / "experiment"
    model = root / "generation/gen0/model.zip"
    model.parent.mkdir(parents=True)
    with zipfile.ZipFile(model, "w") as archive:
        archive.writestr("weights", "ready")
    old = time.time() - 120
    os.utime(model, (old, old))
    result = index(root)
    assert result["unstable"] == [] and result["archive_sha256"]
    with tarfile.open(tmp_path / "checkpoint-backup.tar.gz") as archive:
        assert archive.getnames() == ["generation/gen0/model.zip"]
