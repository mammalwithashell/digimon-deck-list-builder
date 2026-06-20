"""End-to-end PvP over WebSocket on the Rust engine (add-room-match-pvp).

Drives the full room flow (two guests: create → join → decks → start) and
then a live game through `/ws/games/{id}`: mulligans via the ordinary
action path, decision-player routing (the mask goes only to the decision
player; the other seat's submissions are rejected), per-broadcast rule-14
redaction, and concede → game_over with `surrendered_by`.

Uses FastAPI's sync TestClient because httpx.AsyncClient has no WS support.
"""
from __future__ import annotations

import contextlib


def _legal_deck() -> list[str]:
    ids = [f"BT14-{i:03d}" for i in range(10, 23)]
    deck: list[str] = []
    for cid in ids:
        for _ in range(4):
            deck.append(cid)
            if len(deck) == 50:
                return deck
    return deck


def _setup():
    import asyncio as _asyncio

    from fastapi.testclient import TestClient
    from sqlalchemy.ext.asyncio import (
        AsyncSession, async_sessionmaker, create_async_engine,
    )

    from server.api import app
    from server.db.database import get_db
    from server.db.models import Base
    from server.routers.lobby import pending_games, code_to_game
    from server.routers.state import active_games

    pending_games.clear()
    code_to_game.clear()
    active_games.clear()

    loop = _asyncio.new_event_loop()
    _asyncio.set_event_loop(loop)

    engine = create_async_engine(
        "sqlite+aiosqlite:///:memory:", echo=False,
        connect_args={"check_same_thread": False},
    )

    async def _init():
        async with engine.begin() as conn:
            await conn.run_sync(Base.metadata.create_all)
    loop.run_until_complete(_init())

    session_factory = async_sessionmaker(
        engine, class_=AsyncSession, expire_on_commit=False,
    )

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    def teardown():
        app.dependency_overrides.clear()
        pending_games.clear()
        code_to_game.clear()
        active_games.clear()

        async def _drop():
            async with engine.begin() as conn:
                await conn.run_sync(Base.metadata.drop_all)
            await engine.dispose()
        loop.run_until_complete(_drop())
        loop.close()

    return client, teardown


def _guest_sync(client) -> tuple[str, str, dict]:
    resp = client.post("/auth/guest")
    assert resp.status_code == 201
    body = resp.json()
    return body["user_id"], body["access_token"], {
        "Authorization": f"Bearer {body['access_token']}"
    }


def _start_room(client) -> tuple[str, str, str]:
    """Two guests through the full room flow; returns (game_id, host_token,
    joiner_token)."""
    _, tok_host, h_host = _guest_sync(client)
    _, tok_join, h_join = _guest_sync(client)

    r = client.post("/lobby/create", json={"is_public": False}, headers=h_host)
    game_id, code = r.json()["game_id"], r.json()["join_code"]
    assert client.post(f"/lobby/join/{code}", headers=h_join).status_code == 200
    assert client.put(
        f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_host
    ).status_code == 200
    assert client.put(
        f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join
    ).status_code == 200
    assert client.post(f"/lobby/{game_id}/start", headers=h_host).status_code == 200
    return game_id, tok_host, tok_join


def _ws_url(game_id: str, token: str) -> str:
    from server.config import settings
    return (
        f"/ws/games/{game_id}?token={token}"
        f"&role=player&engine_version={settings.engine_version}"
    )


def _recv_until(ws, msg_type: str) -> dict:
    """Receive messages until one of `msg_type` arrives, skipping unrelated
    broadcasts (player_joined / spectator_count / etc.)."""
    for _ in range(20):
        msg = ws.receive_json()
        if msg["type"] == msg_type:
            return msg
    raise AssertionError(f"No {msg_type} received within 20 messages")


def _recv_state(ws) -> dict:
    return _recv_until(ws, "state_update")


def _assert_redacted(msg: dict) -> None:
    """Rule 14 on every broadcast: opponent hand redacted for the recipient,
    security never sent to anyone."""
    me = msg.get("your_player_id")
    state = msg["state"]
    assert state["player1"]["securityIds"] == []
    assert state["player2"]["securityIds"] == []
    if me is not None:
        opp = "player2" if me == 1 else "player1"
        assert state[opp]["handIds"] == []
        assert state[opp]["handCards"] == []


def _first_legal(mask: list) -> int:
    for i, bit in enumerate(mask):
        if bit >= 0.5:
            return i
    raise AssertionError("Empty action mask for the decision player")


def test_full_pvp_game_over_websocket_on_rust():
    client, teardown = _setup()
    try:
        game_id, tok_host, tok_join = _start_room(client)

        with contextlib.ExitStack() as stack:
            ws1 = stack.enter_context(client.websocket_connect(_ws_url(game_id, tok_host)))
            first1 = _recv_state(ws1)
            assert first1["your_player_id"] == 1
            _assert_redacted(first1)

            ws2 = stack.enter_context(client.websocket_connect(_ws_url(game_id, tok_join)))
            first2 = _recv_state(ws2)
            # Every player is perspective-normalized into the player1 slot, so
            # `your_player_id` is 1 for BOTH seats (add-room-match-pvp seat-2 fix).
            assert first2["your_player_id"] == 1
            _assert_redacted(first2)

            # `sockets` is keyed by ABSOLUTE seat (1 = host, 2 = joiner). Since
            # the wire ids are now normalized per-recipient, identify the
            # decision player by which seat holds the (non-empty) mask — the
            # mask only ever reaches the decision player.
            sockets = {1: ws1, 2: ws2}
            masks = {1: first1["action_mask"], 2: first2["action_mask"]}

            def decision_seat() -> int:
                return 1 if masks[1] else 2

            # Decision routing probe: the non-decision player is rejected.
            non_decider = 2 if decision_seat() == 1 else 1
            sockets[non_decider].send_json({"type": "action", "action_id": 0})
            rejection = _recv_until(sockets[non_decider], "error")
            assert "turn" in rejection["message"].lower()

            # Play through mulligans + a stretch of real turns. Each step: the
            # decision seat picks its first legal action; both seats receive a
            # redacted broadcast normalized to their own perspective; the mask
            # follows the decision seat.
            game_over = False
            for _ in range(60):
                actor = decision_seat()
                mask = masks[actor]
                assert mask, f"decision seat {actor} got an empty mask"
                sockets[actor].send_json({
                    "type": "action",
                    "action_id": _first_legal(mask),
                })
                upd = {pid: _recv_state(sockets[pid]) for pid in (1, 2)}
                for pid in (1, 2):
                    _assert_redacted(upd[pid])
                    # Every recipient sees themselves as player1.
                    assert upd[pid]["your_player_id"] == 1
                if upd[1]["is_game_over"]:
                    game_over = True
                    break
                masks = {pid: upd[pid]["action_mask"] for pid in (1, 2)}
                # Mask only reaches the single decision seat.
                assert (masks[1] == []) != (masks[2] == []), (
                    "exactly one seat should hold the action mask"
                )

            if not game_over:
                # Concede from one seat — legal regardless of whose decision it is.
                conceder = decision_seat()
                sockets[conceder].send_json({"type": "surrender"})
                # Both sockets get the final state then game_over.
                finals = {pid: _recv_state(sockets[pid]) for pid in (1, 2)}
                winner_seat = 1 if conceder == 2 else 2
                for pid in (1, 2):
                    assert finals[pid]["is_game_over"]
                    # State winner is normalized: 1 = "you won" from each view.
                    assert finals[pid]["state"]["winner"] == (1 if pid == winner_seat else 2)
                # The game_over EVENT carries absolute ids (broadcast to all).
                over1 = _recv_until(ws1, "game_over")
                over2 = _recv_until(ws2, "game_over")
                for over in (over1, over2):
                    assert over["surrendered_by"] == conceder
                    assert over["winner_id"] == winner_seat
    finally:
        teardown()


def test_spectator_gets_redacted_state():
    client, teardown = _setup()
    try:
        game_id, tok_host, _ = _start_room(client)
        _, tok_spec, _ = _guest_sync(client)
        from server.config import settings
        url = (
            f"/ws/games/{game_id}?token={tok_spec}"
            f"&role=spectator&engine_version={settings.engine_version}"
        )
        with client.websocket_connect(url) as ws:
            msg = ws.receive_json()
            assert msg["type"] == "state_update"
            state = msg["state"]
            for key in ("player1", "player2"):
                assert state[key]["handIds"] == []
                assert state[key]["handCards"] == []
                assert state[key]["securityIds"] == []
    finally:
        teardown()
