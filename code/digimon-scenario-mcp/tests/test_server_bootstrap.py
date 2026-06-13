"""Server assembly + dispatch coverage (no network)."""

from __future__ import annotations

import json

import pytest

from digimon_scenario_mcp.server import (
    TOOL_DEFINITIONS,
    TOOL_HANDLERS,
    ServerContext,
    build_server,
)


def _ctx(tmp_path) -> ServerContext:
    return ServerContext(
        scenarios_dir=tmp_path,
        e2e_dir=tmp_path,
        browser_base_url="http://127.0.0.1:8000",
        desktop_base_url=None,
    )


def test_every_tool_has_a_handler() -> None:
    names = {t.name for t in TOOL_DEFINITIONS}
    assert names == set(TOOL_HANDLERS), "tool definitions and handlers must align"
    # Spec-required tools are present.
    for required in (
        "stage_scenario",
        "read_state",
        "capture_snapshot",
        "evaluate",
        "emit_playwright_spec",
        "save_fixture",
        "add_assertion",
    ):
        assert required in names


def test_build_server_constructs() -> None:
    server = build_server(_ctx_path())
    assert server is not None


def _ctx_path() -> ServerContext:
    import tempfile
    from pathlib import Path

    return ServerContext(
        scenarios_dir=Path(tempfile.gettempdir()),
        e2e_dir=Path(tempfile.gettempdir()),
        browser_base_url="http://127.0.0.1:8000",
        desktop_base_url=None,
    )


@pytest.mark.asyncio
async def test_add_assertion_handler_round_trips(tmp_path) -> None:
    handler = TOOL_HANDLERS["add_assertion"]
    result = await handler(
        _ctx(tmp_path),
        {"fixture": {"assertions": {"engine": []}}, "assertion": {"kind": "memory_equals", "value": 2}},
    )
    assert result["ok"] is True
    assert result["fixture"]["assertions"]["engine"][0]["value"] == 2


@pytest.mark.asyncio
async def test_evaluate_rejects_desktop_target(tmp_path) -> None:
    handler = TOOL_HANDLERS["evaluate"]
    result = await handler(_ctx(tmp_path), {"target": "desktop", "assertions": []})
    assert result["ok"] is False
    assert "browser-only" in result["error"]


@pytest.mark.asyncio
async def test_save_and_list_via_handlers(tmp_path) -> None:
    save = TOOL_HANDLERS["save_fixture"]
    listed = TOOL_HANDLERS["list_fixtures"]
    s = await save(_ctx(tmp_path), {"name": "h1", "fixture": {"title": "H1"}})
    assert s["ok"] is True
    out = await listed(_ctx(tmp_path), {})
    assert any(f["name"] == "h1" for f in out["fixtures"])
