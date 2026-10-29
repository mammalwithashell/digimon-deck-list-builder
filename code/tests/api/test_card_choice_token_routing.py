"""Transport boundary tests; fresh native-wheel enforcement is tested separately."""
from types import SimpleNamespace

import pytest
from fastapi import HTTPException

from server.routers import games, ws_games
from server.routers.schemas import GameActionRequest


TOKEN = {"prompt_id": "18446744073709551615", "page_generation": "9007199254740993"}


class RejectingRunner:
    is_game_over = False
    current_player_id = 1

    def __init__(self):
        self.submissions = []

    def to_ui_json(self):
        return {"memoryGauge": 3, "currentPhase": 14, "turnCount": 1}

    def get_action_mask(self):
        return [1] * 64

    def step_checked(self, action, expected):
        self.submissions.append((action, expected))
        raise ValueError("Stale card choice prompt or page")


def test_http_routes_exact_token_and_rolls_back_rejected_history(monkeypatch):
    runner = RejectingRunner()
    meta = SimpleNamespace(human_action_history=[7])
    monkeypatch.setattr(games, "_require_game", lambda _: runner)
    monkeypatch.setattr(games, "_require_meta", lambda _: meta)
    with pytest.raises(HTTPException) as error:
        games.game_action("test", GameActionRequest(action=59, expected_card_choice=TOKEN))
    assert error.value.status_code == 400
    assert "Stale card choice" in error.value.detail
    assert runner.submissions == [(59, TOKEN)]
    assert meta.human_action_history == [7]


@pytest.mark.asyncio
async def test_websocket_routes_token_and_reports_rejection_without_broadcast():
    messages = []

    async def send_json(message):
        messages.append(message)

    runner = RejectingRunner()
    await ws_games._handle_action(SimpleNamespace(send_json=send_json), "test", 1,
        {"action_id": 59, "expected_card_choice": TOKEN}, runner)
    assert runner.submissions == [(59, TOKEN)]
    assert messages == [{"type": "error", "message": "Stale card choice prompt or page"}]
