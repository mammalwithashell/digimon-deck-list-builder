"""Tests for `scripts/mcp_launch.py` — where it looks for a prebuilt MCP binary.

The launcher runs under the desktop app's MCP client, whose environment is the
Windows User environment, NOT an interactive bash shell. `CARGO_TARGET_DIR` is
derived per worktree by `~/.bashrc` (CLAUDE.md rule 31), so it is present in a
bash shell and ABSENT in the app. The launcher must reach the same per-worktree
directory either way, or every Rust MCP server fails to connect with
"no built binary" even though the binary is sitting in `D:/cargo-target/<wt>`.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "mcp_launch.py"


@pytest.fixture()
def launch(monkeypatch):
    spec = importlib.util.spec_from_file_location("mcp_launch", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for var in ("CARGO_TARGET_DIR", "CARGO_TARGET_BASE", "CARGO_TARGET_DIR_PINNED"):
        monkeypatch.delenv(var, raising=False)
    return module


def _dirs(module, binary="dcgo-harness"):
    return [p.parent.parent for p in module.candidates(binary)]


def test_explicit_cargo_target_dir_wins(launch, monkeypatch, tmp_path):
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path / "explicit"))
    monkeypatch.setenv("CARGO_TARGET_BASE", str(tmp_path / "base"))
    assert _dirs(launch)[0] == tmp_path / "explicit"


def test_derives_per_worktree_dir_from_base_when_target_dir_unset(launch, monkeypatch, tmp_path):
    """The app's case: CARGO_TARGET_BASE is a User env var, CARGO_TARGET_DIR is not."""
    root = tmp_path / "repo" / ".claude" / "worktrees" / "bold-bassi-d34dc7"
    monkeypatch.setattr(launch, "repo_root", lambda: root)
    monkeypatch.setenv("CARGO_TARGET_BASE", str(tmp_path / "cargo-target"))
    assert _dirs(launch)[0] == tmp_path / "cargo-target" / "bold-bassi-d34dc7"


def test_non_worktree_uses_repo_basename_like_bashrc(launch, monkeypatch, tmp_path):
    root = tmp_path / "digimon-deck-list-builder-1"
    monkeypatch.setattr(launch, "repo_root", lambda: root)
    monkeypatch.setenv("CARGO_TARGET_BASE", str(tmp_path / "cargo-target"))
    assert _dirs(launch)[0] == tmp_path / "cargo-target" / "digimon-deck-list-builder-1"


def test_repo_local_target_is_still_a_fallback(launch, monkeypatch, tmp_path):
    """Machines without the isolation scheme build into `<repo>/target`."""
    root = tmp_path / "repo"
    monkeypatch.setattr(launch, "repo_root", lambda: root)
    monkeypatch.setenv("CARGO_TARGET_BASE", str(tmp_path / "cargo-target"))
    assert root / "target" in _dirs(launch)


def test_release_preferred_over_debug_within_a_dir(launch, monkeypatch, tmp_path):
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path / "t"))
    first_two = [p.parent.name for p in launch.candidates("dcgo-harness")[:2]]
    assert first_two == ["release", "debug"]


def test_finds_a_binary_built_only_in_the_derived_dir(launch, monkeypatch, tmp_path):
    """End to end: the exact failure — binary exists ONLY in the per-worktree dir."""
    root = tmp_path / "repo" / ".claude" / "worktrees" / "wt1"
    monkeypatch.setattr(launch, "repo_root", lambda: root)
    monkeypatch.setenv("CARGO_TARGET_BASE", str(tmp_path / "cargo-target"))
    exe = ".exe" if launch.os.name == "nt" else ""
    built = tmp_path / "cargo-target" / "wt1" / "debug" / f"dcgo-harness{exe}"
    built.parent.mkdir(parents=True)
    built.write_bytes(b"")
    assert next(p for p in launch.candidates("dcgo-harness") if p.is_file()) == built
