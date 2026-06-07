"""Guardrail: the RL training entrypoint chain must import with zero
``engine_py_legacy`` (and zero hosted-``server``) dependencies.

This pins the cloud-image contract from change make-training-build-legacy-free.
The training build (``Dockerfile.training``) ships neither ``engine_py_legacy``
nor ``code/server/``, so a top-level import of either anywhere in the training
chain crashes the image at startup — and the prior CI smoke (``--dry-run``)
returned before the real import chain, so such breakage shipped green.

The imports run in a FRESH subprocess (so module caching from other tests in the
same session cannot mask a regression), with both packages forced unimportable
via ``sys.modules[...] = None`` — mirroring exactly how the image fails. A
negative control inside the probe proves the block actually bites.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

# The training stack is Rust-only; without the engine wheel (or the heavy
# training deps) there is nothing to guard, so skip cleanly.
pytest.importorskip("digimon_engine")
pytest.importorskip("sb3_contrib")

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CODE_DIR = _REPO_ROOT / "code"

# Block engine_py_legacy + server, prove the block bites (negative control),
# then import the full training entrypoint chain.
_PROBE = r"""
import sys

sys.modules["engine_py_legacy"] = None
sys.modules["server"] = None

# Negative control: with the block in place, importing engine_py_legacy MUST
# raise. If it doesn't, the guardrail is not actually testing anything.
try:
    import engine_py_legacy  # noqa: F401
except ImportError:
    pass
else:
    raise SystemExit("negative control failed: engine_py_legacy import did not raise")

# The real contract: the training entrypoint chain imports cleanly.
import digimon_gym.digimon_gym  # noqa: F401
import digimon_gym.agents.pilot_training  # noqa: F401
import tools.run_training_job  # noqa: F401

# The env must also construct on the Rust backend without legacy/server.
from digimon_gym.digimon_gym import DigimonEnv

env = DigimonEnv()
obs, info = env.reset(seed=0)
assert obs.shape[0] > 0 and info["action_mask"].shape[0] > 0
assert type(env.runner).__name__ == "RustHeadlessGame"

print("LEGACY_FREE_OK")
"""


def _run_probe() -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join(
        p for p in [str(_CODE_DIR), env.get("PYTHONPATH", "")] if p
    )
    env["DIGIMON_BACKEND"] = "rust"
    return subprocess.run(
        [sys.executable, "-c", _PROBE],
        cwd=str(_REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
    )


def test_training_chain_imports_without_engine_py_legacy_or_server():
    """The training entrypoint chain imports and runs on Rust with
    engine_py_legacy + server forced unimportable."""
    result = _run_probe()
    assert result.returncode == 0, (
        "training chain failed to import/run with engine_py_legacy + server "
        f"blocked.\n--- STDOUT ---\n{result.stdout}\n--- STDERR ---\n{result.stderr}"
    )
    assert "LEGACY_FREE_OK" in result.stdout, (
        f"probe did not reach the success sentinel.\nSTDOUT:\n{result.stdout}\n"
        f"STDERR:\n{result.stderr}"
    )
