"""Guardrail: the non-PvP hosted-API support surfaces and the retained
``code/tools`` CLI must import with zero ``engine_py_legacy`` dependencies.

Pins the contract from changes shrink-legacy-engine-surface (non-PvP surfaces +
tools) and excise-legacy-engine-from-hosted-api (the PvP cutover). Deck
legality, replay, recordings, state-redaction, the admin router (script-promotion
retired), AND the live PvP/WebSocket runtime (``ws_games``/``ws_manager``/
``lobby``, now on ``RustInteractiveGame``) all run on the Rust engine. The whole
``code/server/`` is legacy-free — `server.api` itself must assemble with
``engine_py_legacy`` blocked.

Imports run in a FRESH subprocess with ``engine_py_legacy`` forced unimportable
(``sys.modules[...] = None``), so module caching from other tests can't mask a
regression. A negative control proves the block bites.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

# Without the engine wheel there is nothing to guard (the Rust binding is the
# whole point), so skip cleanly.
pytest.importorskip("digimon_engine")

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CODE_DIR = _REPO_ROOT / "code"

# Non-PvP server support surfaces + the retained light tools (no torch).
_PROBE = r"""
import sys

sys.modules["engine_py_legacy"] = None

# Negative control: the block must actually bite.
try:
    import engine_py_legacy  # noqa: F401
except ImportError:
    pass
else:
    raise SystemExit("negative control failed: engine_py_legacy import did not raise")

# Non-PvP hosted-API support surfaces.
import server.routers.simulations  # noqa: F401
import server.routers.replays  # noqa: F401
import server.routers.recordings  # noqa: F401
import server.routers.state  # noqa: F401
import server.db.routers.training  # noqa: F401
import server.db.routers.decks  # noqa: F401
import server.db.routers.admin_ai  # noqa: F401  (script_promotion lane retired)

# Relocated redaction module (was engine_py_legacy.engine.state_filter).
from server.state_filter import filter_state_for_player  # noqa: F401

# PvP/WebSocket runtime — migrated to the Rust interactive runner.
import server.routers.ws_games  # noqa: F401
import server.routers.ws_manager  # noqa: F401
import server.routers.lobby  # noqa: F401
import server.routers.matchmaking  # noqa: F401
from server.rust_interactive_game import RustInteractiveGame  # noqa: F401

# The whole hosted API assembles without the sunset Python engine.
import server.api  # noqa: F401

# Retained code/tools CLI (the light, non-torch ones).
import tools.resolve_deck  # noqa: F401
import tools.meta_loader  # noqa: F401
import tools.ingest_cards  # noqa: F401
import tools.card_features  # noqa: F401

print("SUPPORT_SURFACES_LEGACY_FREE_OK")
"""


def _run_probe(probe: str) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join(
        p for p in [str(_CODE_DIR), env.get("PYTHONPATH", "")] if p
    )
    return subprocess.run(
        [sys.executable, "-c", probe],
        cwd=str(_REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
    )


def test_support_surfaces_and_tools_import_without_legacy():
    result = _run_probe(_PROBE)
    assert result.returncode == 0, (
        "non-PvP support surfaces / tools failed to import with engine_py_legacy "
        f"blocked.\n--- STDOUT ---\n{result.stdout}\n--- STDERR ---\n{result.stderr}"
    )
    assert "SUPPORT_SURFACES_LEGACY_FREE_OK" in result.stdout, (
        f"probe did not reach the success sentinel.\nSTDOUT:\n{result.stdout}\n"
        f"STDERR:\n{result.stderr}"
    )


def test_autoencoder_tool_imports_without_legacy():
    """train_card_autoencoder is retained (warm-start), repointed off legacy
    (`tools.card_features`). Guarded separately because it pulls in torch."""
    pytest.importorskip("torch")
    probe = (
        'import sys\n'
        'sys.modules["engine_py_legacy"] = None\n'
        'import tools.train_card_autoencoder  # noqa: F401\n'
        'print("AUTOENCODER_LEGACY_FREE_OK")\n'
    )
    result = _run_probe(probe)
    assert result.returncode == 0, (
        "train_card_autoencoder failed to import with engine_py_legacy blocked.\n"
        f"--- STDOUT ---\n{result.stdout}\n--- STDERR ---\n{result.stderr}"
    )
    assert "AUTOENCODER_LEGACY_FREE_OK" in result.stdout
