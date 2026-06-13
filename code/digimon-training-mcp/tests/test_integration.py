"""Integration tests — spawn ``python -m digimon_training_mcp`` as a subprocess
and exercise the MCP protocol over stdio.

These are slower than the in-process tests (one subprocess spawn per test) but
verify the full JSON-RPC envelope, the stdio framing, argparse, the
ancestor-walk path resolution, and that tool responses make it back through
the SDK's ``CallToolResult`` wrap unchanged.

The subprocess gets the synthetic-repo paths via ``--runs-dir`` / ``--models-dir``
flags so the test doesn't depend on the host's runs/ or models/ layout.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List

import pytest


def _send(proc: subprocess.Popen, request: Dict[str, Any]) -> None:
    line = json.dumps(request, separators=(",", ":")) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()


def _recv(proc: subprocess.Popen) -> Dict[str, Any]:
    line = proc.stdout.readline()
    if not line:
        raise RuntimeError("MCP server closed stdout unexpectedly")
    return json.loads(line)


def _extract_tool_payload(response: Dict[str, Any]) -> Dict[str, Any]:
    """Tool results come back wrapped as ``{result: {content: [{type, text}]}}``;
    the ``text`` field is a JSON-encoded handler return value."""
    content = response["result"]["content"]
    assert len(content) == 1
    assert content[0]["type"] == "text"
    return json.loads(content[0]["text"])


@pytest.fixture
def server_proc(synthetic_repo):
    """Spawn the server as a subprocess pointed at the synthetic fixture."""
    env = os.environ.copy()
    env["PYTHONUNBUFFERED"] = "1"
    proc = subprocess.Popen(
        [
            sys.executable, "-m", "digimon_training_mcp",
            "--runs-dir", str(synthetic_repo["runs_dir"]),
            "--models-dir", str(synthetic_repo["models_dir"]),
            "--log-level", "ERROR",
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=env,
    )
    try:
        # Handshake: initialize + initialized notification.
        _send(proc, {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "integration-test", "version": "0"},
            },
        })
        init_response = _recv(proc)
        assert init_response["result"]["serverInfo"]["name"] == "digimon-training-mcp"
        _send(proc, {"jsonrpc": "2.0", "method": "notifications/initialized"})
        yield proc
    finally:
        try:
            proc.stdin.close()
        except Exception:
            pass
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)


def _call_tool(proc: subprocess.Popen, call_id: int, name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
    _send(proc, {
        "jsonrpc": "2.0", "id": call_id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments},
    })
    return _extract_tool_payload(_recv(proc))


def test_tools_list_returns_all_tools(server_proc):
    _send(server_proc, {"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    resp = _recv(server_proc)
    tools = resp["result"]["tools"]
    names = {t["name"] for t in tools}
    assert names == {
        "list_runs", "run_summary", "run_metric", "run_tags",
        "run_recordings", "run_checkpoints", "run_deck_pool",
        "run_per_game_evals", "run_anchored_evals",
        "champion_standings", "run_elo_ladder", "run_exploitability",
    }


def test_list_runs_over_stdio(server_proc, synthetic_repo):
    payload = _call_tool(server_proc, 10, "list_runs", {})
    assert payload["ok"] is True
    names = [r["name"] for r in payload["runs"]]
    assert "active_run" in names
    assert "stale_run" in names


def test_run_summary_over_stdio(server_proc):
    payload = _call_tool(server_proc, 11, "run_summary", {"name": "active_run", "tail_evals": 5})
    assert payload["ok"] is True
    assert payload["header"]["algorithm"] == "MaskablePPO"
    assert payload["panics"]["total"] == 4
    assert payload["panics"]["by_family"].get("G-DSL-OUTER-TAIL-NESTED-PARK") == 2


def test_run_metric_over_stdio(server_proc):
    payload = _call_tool(server_proc, 12, "run_metric",
                         {"name": "active_run", "tag": "pilot/win_rate"})
    assert payload["ok"] is True
    assert payload["tag"] == "pilot/win_rate"
    assert len(payload["values"]) == 5


def test_run_metric_multi_tag_over_stdio(server_proc):
    payload = _call_tool(server_proc, 13, "run_metric",
                         {"name": "active_run", "tag": ["pilot/win_rate", "train/loss"]})
    assert payload["ok"] is True
    assert set(payload["series"].keys()) == {"pilot/win_rate", "train/loss"}


def test_run_tags_over_stdio(server_proc):
    payload = _call_tool(server_proc, 14, "run_tags", {"name": "active_run"})
    assert payload["ok"] is True
    assert "pilot/win_rate" in payload["tags"]


def test_run_recordings_crash_filter_over_stdio(server_proc):
    payload = _call_tool(server_proc, 15, "run_recordings",
                         {"name": "active_run", "filter": "crash"})
    assert payload["ok"] is True
    assert payload["count"] == 15
    assert all(r["reason"] == "crash" for r in payload["recordings"])


def test_run_checkpoints_over_stdio(server_proc):
    payload = _call_tool(server_proc, 16, "run_checkpoints", {"name": "active_run"})
    assert payload["ok"] is True
    steps = [c["step"] for c in payload["checkpoints"]]
    assert steps == [100_000, 200_000, 300_000]


def test_run_deck_pool_over_stdio(server_proc):
    payload = _call_tool(server_proc, 17, "run_deck_pool", {"name": "active_run"})
    assert payload["ok"] is True
    assert payload["deck_count"] == 8
    assert len(payload["archetypes"]) == 4


def test_unknown_run_returns_structured_error(server_proc):
    payload = _call_tool(server_proc, 18, "run_summary", {"name": "does_not_exist"})
    assert payload["ok"] is False
    assert "not found" in payload["error"]


def test_recording_paths_are_readable_files(server_proc):
    """Spec scenario: paths returned by run_recordings should be loadable by
    the engine MCP's load_recording. We test the same precondition here —
    paths exist and are valid JSON files."""
    payload = _call_tool(server_proc, 19, "run_recordings",
                         {"name": "active_run", "filter": "crash", "limit": 1})
    assert payload["count"] == 1
    path = Path(payload["recordings"][0]["path"])
    assert path.is_file()
    artifact = json.loads(path.read_text(encoding="utf-8"))
    assert artifact["outcome"]["reason"] == "crash"
