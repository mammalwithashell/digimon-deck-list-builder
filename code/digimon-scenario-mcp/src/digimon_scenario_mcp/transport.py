"""HTTP transports for the two stage/inspect surfaces, plus the pure
request-shaping helpers (unit-testable without a live server).

- ``browser`` → the hosted-API ``/debug`` + ``/games`` routes (multi-game,
  addressed by ``game_id``).
- ``desktop`` → the feature-gated Tauri bridge (single implicit game, no id),
  auto-discovered from the dotfile the bridge drops.

The two return different response shapes; the MCP server normalizes them.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any, Optional

import httpx


# ─── Pure helpers (no I/O) ───────────────────────────────────────────


def build_debug_create_body(fixture: dict[str, Any]) -> dict[str, Any]:
    """Map a scenario fixture into a POST /debug/games request body."""
    state = fixture.get("state", {}) or {}
    decks = fixture.get("decks", {}) or {}
    return {
        "deck1": decks.get("1", []),
        "deck2": decks.get("2", []),
        "seed": fixture.get("seed"),
        "phase": state.get("phase", "Main"),
        "initial_memory": state.get("memory", 0),
        "first_player": state.get("first_player", 1),
        "turn": state.get("turn"),
        "zones": fixture.get("zones", {}) or {},
    }


def desktop_discovery_path() -> Path:
    """Mirror of the bridge's `dirs::data_dir()/digimon-tcg/debug_bridge.json`.

    `dirs::data_dir()` resolves to roaming AppData on Windows, Application
    Support on macOS, and `$XDG_DATA_HOME`/`~/.local/share` on Linux.
    """
    if sys.platform.startswith("win"):
        base = os.environ.get("APPDATA")
        root = Path(base) if base else Path.home() / "AppData" / "Roaming"
    elif sys.platform == "darwin":
        root = Path.home() / "Library" / "Application Support"
    else:
        xdg = os.environ.get("XDG_DATA_HOME")
        root = Path(xdg) if xdg else Path.home() / ".local" / "share"
    return root / "digimon-tcg" / "debug_bridge.json"


def resolve_desktop_base_url(explicit: Optional[str] = None) -> str:
    """Resolve the desktop bridge base URL: explicit > env > discovery file >
    default `http://127.0.0.1:5174`."""
    if explicit:
        return explicit.rstrip("/")
    env = os.environ.get("DIGIMON_DEBUG_BRIDGE_URL")
    if env:
        return env.rstrip("/")
    path = desktop_discovery_path()
    if path.exists():
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            url = data.get("base_url")
            if isinstance(url, str) and url:
                return url.rstrip("/")
        except (json.JSONDecodeError, OSError):
            pass
    return "http://127.0.0.1:5174"


# ─── Clients ─────────────────────────────────────────────────────────


class BrowserClient:
    """Talks to the hosted-API /debug + /games surface."""

    def __init__(self, base_url: str, timeout: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self._http = httpx.Client(base_url=self.base_url, timeout=timeout)

    def stage(self, fixture: dict[str, Any]) -> dict[str, Any]:
        body = build_debug_create_body(fixture)
        r = self._http.post("/debug/games", json=body)
        r.raise_for_status()
        return r.json()

    def read_state(self, game_id: str) -> dict[str, Any]:
        r = self._http.get(f"/debug/games/{game_id}/internal-state")
        r.raise_for_status()
        return r.json()

    def ui_state(self, game_id: str) -> dict[str, Any]:
        r = self._http.get(f"/games/{game_id}/state")
        r.raise_for_status()
        return r.json()

    def get_mask(self, game_id: str) -> dict[str, Any]:
        r = self._http.get(f"/games/{game_id}/action-mask")
        r.raise_for_status()
        return r.json()

    def step(self, game_id: str, action: int) -> dict[str, Any]:
        r = self._http.post(f"/debug/games/{game_id}/step", params={"action": action})
        r.raise_for_status()
        return r.json()

    def set_memory(self, game_id: str, memory: int) -> dict[str, Any]:
        r = self._http.post(f"/debug/games/{game_id}/set-memory", json={"memory": memory})
        r.raise_for_status()
        return r.json()

    def inject_card(self, game_id: str, player_id: int, card_id: str, zone: str) -> dict[str, Any]:
        r = self._http.post(
            f"/debug/games/{game_id}/inject-card",
            json={"player_id": player_id, "card_id": card_id, "zone": zone},
        )
        r.raise_for_status()
        return r.json()

    def capture(self, game_id: str) -> dict[str, Any]:
        r = self._http.post(f"/games/{game_id}/export-scenario")
        r.raise_for_status()
        return r.json()

    def evaluate(self, game_id: str, assertions: list[dict[str, Any]]) -> dict[str, Any]:
        r = self._http.post(f"/debug/games/{game_id}/evaluate", json={"assertions": assertions})
        r.raise_for_status()
        return r.json()

    def close(self) -> None:
        self._http.close()


class DesktopClient:
    """Talks to the Tauri debug bridge (single implicit game, no game id)."""

    def __init__(self, base_url: Optional[str] = None, timeout: float = 30.0) -> None:
        self.base_url = resolve_desktop_base_url(base_url)
        self._http = httpx.Client(base_url=self.base_url, timeout=timeout)

    def stage(self, fixture: dict[str, Any]) -> dict[str, Any]:
        r = self._http.post("/stage", json=fixture)
        r.raise_for_status()
        return r.json()

    def read_state(self, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.get("/internal-state")
        r.raise_for_status()
        return r.json()

    def ui_state(self, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.get("/state")
        r.raise_for_status()
        return r.json()

    def get_mask(self, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.get("/mask")
        r.raise_for_status()
        return r.json()

    def step(self, action: int, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.post("/step", json={"action": action})
        r.raise_for_status()
        return r.json()

    def set_memory(self, memory: int, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.post("/set-memory", json={"memory": memory})
        r.raise_for_status()
        return r.json()

    def inject_card(
        self, player_id: int, card_id: str, zone: str, game_id: Optional[str] = None
    ) -> dict[str, Any]:
        r = self._http.post(
            "/inject-card", json={"player_id": player_id, "card_id": card_id, "zone": zone}
        )
        r.raise_for_status()
        return r.json()

    def capture(self, game_id: Optional[str] = None) -> dict[str, Any]:
        r = self._http.post("/export-scenario")
        r.raise_for_status()
        return r.json()

    def health(self) -> bool:
        try:
            r = self._http.get("/health")
            return r.status_code == 200
        except httpx.HTTPError:
            return False

    def close(self) -> None:
        self._http.close()
