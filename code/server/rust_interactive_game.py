"""Rust-engine interactive runner for PvP WebSocket games.

Wraps `digimon_engine.RustHeadlessGame` to present the method surface the
WebSocket layer (`ws_games.py` / `ws_manager.py`) expects, replacing the
retired Python `InteractiveGame` so PvP runs on the Rust engine with zero
`engine_py_legacy` imports (excise-legacy-engine-from-hosted-api).

PvP is human-vs-human, so there is no bot/policy loop here — that lives in
`games.py` for the REST agent path. This runner is purely: apply a human
action, expose per-player state / mask / drained events, and concede. It
follows the established Rust-backed `/games` conventions:
  * state is `to_ui_json()` (camelCase; redacted per-recipient by `state_filter`);
  * the action mask drives legal actions — there are no server-side action
    descriptions (the frontend decodes actions from the mask + the 2192-action
    space), so `describe_actions` returns `{}`;
  * events are drained from `get_events_since_last_step()`.

The Python 1/2 player-id convention is preserved end-to-end (RustHeadlessGame
already speaks 1/2 at the binding boundary, per rule 20).
"""

from __future__ import annotations

import secrets
from typing import Any, List, Optional

from digimon_engine import RustHeadlessGame


class _Winner:
    """Minimal shim so the WS layer can read `runner.game.winner.player_id`."""

    def __init__(self, player_id: int) -> None:
        self.player_id = player_id


class _GameShim:
    """Presents the `runner.game.*` surface the WS layer reads off the runner."""

    def __init__(self, rust: RustHeadlessGame) -> None:
        self._rust = rust

    def to_ui_json(self) -> dict:
        return self._rust.to_ui_json()

    @property
    def current_player_id(self) -> int:
        return self._rust.current_player_id

    def describe_actions(self, player_id: int) -> dict:
        # The Rust-backed path renders actions from the mask + the frontend's
        # action-space knowledge; no server-side descriptions (matches games.py).
        return {}

    @property
    def winner(self) -> Optional[_Winner]:
        wid = self._rust.winner_id
        return _Winner(wid) if wid else None

    @property
    def game_over(self) -> bool:
        return self._rust.is_game_over


class RustInteractiveGame:
    """PvP interactive runner on the Rust engine — a drop-in for the WS layer.

    Accepts (and ignores) the legacy `InteractiveGame` keyword args
    (`player1_type` / `player2_type` / `agent_action_delay_ms`): PvP is
    human-vs-human so there is no bot policy or pacing here.
    """

    def __init__(
        self,
        deck1_ids: List[str],
        deck2_ids: List[str],
        *,
        seed: Optional[int] = None,
        **_legacy_kwargs: Any,
    ) -> None:
        if seed is None:
            seed = secrets.randbits(64)
        # record_actions=True so the game can be persisted via
        # POST /games/{id}/recordings and replayed through RustReplayRunner.
        self._rust = RustHeadlessGame(deck1_ids, deck2_ids, False, True, False, seed)
        self.game = _GameShim(self._rust)
        self._events: List[dict] = []

    def get_action_mask(self):
        """Action mask for the current decision player (numpy float32 array)."""
        return self._rust.get_action_mask()

    @property
    def is_game_over(self) -> bool:
        return self._rust.is_game_over

    def step(self, action_id: int) -> None:
        self._rust.step(int(action_id))
        self._events.extend(self._rust.get_events_since_last_step())

    def surrender(self, player_id: int) -> None:
        self._rust.concede(int(player_id))
        self._events.extend(self._rust.get_events_since_last_step())

    def get_last_log(self) -> List[str]:
        # The Rust engine's human-readable log stream is not yet surfaced over
        # PyO3 (same as the /games path); events carry the structured stream.
        return []

    def get_last_events(self) -> List[dict]:
        return list(self._events)

    def clear_log(self) -> None:
        pass

    def clear_events(self) -> None:
        self._events.clear()
