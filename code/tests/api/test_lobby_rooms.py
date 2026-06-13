"""Room-match lobby lifecycle tests (add-room-match-pvp).

Covers the two-phase room flow: create (5-digit numeric code) → join
(seat reservation, no deck) → per-seat deck locking → host first-player
choice → host-gated start (constructs the Rust game) → seat-aware polled
state. Plus leave/cancel semantics, TTL expiry, and the rule-14 redaction
regression over a real RustHeadlessGame.to_ui_json() payload.
"""
from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from server.db.database import get_db
from server.db.models import Base


@pytest.fixture
async def db_engine():
    engine = create_async_engine(
        "sqlite+aiosqlite:///:memory:",
        echo=False,
        connect_args={"check_same_thread": False},
    )
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
async def client(db_engine):
    from server.api import app
    from server.routers.lobby import pending_games, code_to_game
    from server.routers.state import active_games

    pending_games.clear()
    code_to_game.clear()
    active_games.clear()

    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()
    pending_games.clear()
    code_to_game.clear()
    active_games.clear()


async def _guest(client: AsyncClient) -> tuple[str, dict]:
    """Mint a guest (the room flow is accounts-free) and return (user_id, headers)."""
    resp = await client.post("/auth/guest")
    assert resp.status_code == 201
    body = resp.json()
    return body["user_id"], {"Authorization": f"Bearer {body['access_token']}"}


def _legal_deck() -> list[str]:
    """50-card main deck of BT14 Digimon that passes the standard ban list."""
    ids = [f"BT14-{i:03d}" for i in range(10, 23)]
    deck: list[str] = []
    for cid in ids:
        for _ in range(4):
            deck.append(cid)
            if len(deck) == 50:
                return deck
    return deck


async def _create_room(client: AsyncClient, headers: dict) -> tuple[str, str]:
    resp = await client.post("/lobby/create", json={"is_public": False}, headers=headers)
    assert resp.status_code == 200
    body = resp.json()
    return body["game_id"], body["join_code"]


class TestRoomLifecycle:
    async def test_join_code_is_five_digit_numeric(self, client: AsyncClient):
        _, h = await _guest(client)
        _, code = await _create_room(client, h)
        assert len(code) == 5
        assert code.isdigit()

    async def test_full_happy_path_create_join_decks_start(self, client: AsyncClient):
        from server.routers.state import active_games
        from server.routers.ws_manager import manager
        from digimon_engine import RustHeadlessGame

        host_id, h_host = await _guest(client)
        joiner_id, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)

        # Join reserves the seat without a deck and without starting
        rj = await client.post(f"/lobby/join/{code}", headers=h_join)
        assert rj.status_code == 200
        assert rj.json()["your_seat"] == 2
        assert game_id not in active_games

        # Host polls state: joiner present, not deck-ready
        rs = (await client.get(f"/lobby/{game_id}/state", headers=h_host)).json()
        assert rs["joiner_display_name"] is not None
        assert rs["joiner_deck_ready"] is False
        assert rs["your_seat"] == 1
        assert rs["started"] is False

        # Both seats lock decks
        rd1 = await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_host)
        assert rd1.status_code == 200
        rd2 = await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join)
        assert rd2.status_code == 200
        assert rd2.json()["host_deck_ready"] is True
        assert rd2.json()["joiner_deck_ready"] is True

        # Host starts
        rstart = await client.post(f"/lobby/{game_id}/start", headers=h_host)
        assert rstart.status_code == 200
        assert rstart.json()["started"] is True

        # Rust game is live; WS seat validation knows the joiner
        assert isinstance(active_games.get(game_id), RustHeadlessGame)
        assert manager.get_settings(game_id).joiner_user_id == joiner_id

        # Joiner polls state: started, knows their seat
        rs2 = (await client.get(f"/lobby/{game_id}/state", headers=h_join)).json()
        assert rs2["started"] is True
        assert rs2["your_seat"] == 2

        # The join code is freed once started
        rj2 = await client.post(f"/lobby/join/{code}", headers=h_join)
        assert rj2.status_code == 404

    async def test_seat_exclusivity_and_idempotent_rejoin(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        _, h_third = await _guest(client)
        game_id, code = await _create_room(client, h_host)

        assert (await client.post(f"/lobby/join/{code}", headers=h_join)).status_code == 200
        # Re-join by the seated user is idempotent
        again = await client.post(f"/lobby/join/{code}", headers=h_join)
        assert again.status_code == 200
        assert again.json()["game_id"] == game_id
        # Third user is rejected
        assert (await client.post(f"/lobby/join/{code}", headers=h_third)).status_code == 409

    async def test_host_cannot_join_own_room(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, code = await _create_room(client, h_host)
        assert (await client.post(f"/lobby/join/{code}", headers=h_host)).status_code == 400

    async def test_non_participant_cannot_lock_deck(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, h_other = await _guest(client)
        game_id, _ = await _create_room(client, h_host)
        resp = await client.put(
            f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_other
        )
        assert resp.status_code == 403

    async def test_premature_start_rejected(self, client: AsyncClient):
        from server.routers.state import active_games

        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_host)

        # No joiner yet
        assert (await client.post(f"/lobby/{game_id}/start", headers=h_host)).status_code == 409

        # Joiner seated but no joiner deck
        await client.post(f"/lobby/join/{code}", headers=h_join)
        assert (await client.post(f"/lobby/{game_id}/start", headers=h_host)).status_code == 409
        assert game_id not in active_games

        # Joiner cannot start even when ready
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join)
        assert (await client.post(f"/lobby/{game_id}/start", headers=h_join)).status_code == 403

    async def test_first_player_choice_host_only_and_visible(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.post(f"/lobby/join/{code}", headers=h_join)

        # Joiner cannot set it
        rj = await client.put(
            f"/lobby/{game_id}/first-player", json={"first_player": "2"}, headers=h_join
        )
        assert rj.status_code == 403

        rh = await client.put(
            f"/lobby/{game_id}/first-player", json={"first_player": "2"}, headers=h_host
        )
        assert rh.status_code == 200
        state = (await client.get(f"/lobby/{game_id}/state", headers=h_join)).json()
        assert state["first_player"] == "2"

    async def test_room_seed_is_host_only_visible_and_clearable(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.post(f"/lobby/join/{code}", headers=h_join)

        joiner_seed = await client.put(
            f"/lobby/{game_id}/seed", json={"seed": "12345"}, headers=h_join
        )
        assert joiner_seed.status_code == 403

        set_seed = await client.put(
            f"/lobby/{game_id}/seed", json={"seed": "18446744073709551615"}, headers=h_host
        )
        assert set_seed.status_code == 200
        state = set_seed.json()
        assert state["seed"] == "18446744073709551615"
        assert state["seed_mode"] == "explicit"
        assert state["first_player_seed_locked"] is True

        visible_to_joiner = (await client.get(f"/lobby/{game_id}/state", headers=h_join)).json()
        assert visible_to_joiner["seed"] == "18446744073709551615"

        cleared = await client.put(f"/lobby/{game_id}/seed", json={"seed": ""}, headers=h_host)
        assert cleared.status_code == 200
        assert cleared.json()["seed"] is None
        assert cleared.json()["seed_mode"] == "generated"
        assert cleared.json()["first_player_seed_locked"] is False

    async def test_invalid_room_seed_is_rejected(self, client: AsyncClient):
        _, h_host = await _guest(client)
        game_id, _ = await _create_room(client, h_host)

        for seed in ["-1", "1.5", "oops", "18446744073709551616"]:
            resp = await client.put(f"/lobby/{game_id}/seed", json={"seed": seed}, headers=h_host)
            assert resp.status_code == 422
            assert "base-10" in resp.text or "u64" in resp.text

    async def test_explicit_room_seed_starts_unchanged_and_controls_first_player(
        self, client: AsyncClient
    ):
        from server.routers.state import active_games

        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.post(f"/lobby/join/{code}", headers=h_join)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_host)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join)
        await client.put(
            f"/lobby/{game_id}/first-player", json={"first_player": "2"}, headers=h_host
        )
        await client.put(f"/lobby/{game_id}/seed", json={"seed": "4242"}, headers=h_host)

        started = await client.post(f"/lobby/{game_id}/start", headers=h_host)
        assert started.status_code == 200
        assert started.json()["seed"] == "4242"
        assert started.json()["seed_mode"] == "explicit"
        assert active_games[game_id].current_player_id == 1

    @pytest.mark.parametrize("choice,expected_pid", [("1", 1), ("2", 2)])
    async def test_first_player_choice_maps_to_created_game(
        self, client: AsyncClient, choice: str, expected_pid: int
    ):
        """Pins the seed-parity direction (Game::new rotates turn order by
        seed % 2): host choice '1'/'2' must make that seat the first
        decision player of the constructed game."""
        from server.routers.state import active_games

        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.post(f"/lobby/join/{code}", headers=h_join)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_host)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join)
        await client.put(
            f"/lobby/{game_id}/first-player", json={"first_player": choice}, headers=h_host
        )
        assert (await client.post(f"/lobby/{game_id}/start", headers=h_host)).status_code == 200

        runner = active_games[game_id]
        # The first decision of a fresh game (mulligan) belongs to the
        # first player; current_player_id is the Python 1/2 decision player.
        assert runner.current_player_id == expected_pid
        state = (await client.get(f"/lobby/{game_id}/state", headers=h_host)).json()
        assert state["seed"] is not None
        assert int(state["seed"]) % 2 == (0 if choice == "1" else 1)

    async def test_leave_and_cancel(self, client: AsyncClient):
        _, h_host = await _guest(client)
        _, h_join = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        await client.post(f"/lobby/join/{code}", headers=h_join)
        await client.put(f"/lobby/{game_id}/deck", json={"deck": _legal_deck()}, headers=h_join)

        # Joiner leaves: seat + deck cleared, room back to waiting
        rl = await client.post(f"/lobby/{game_id}/leave", headers=h_join)
        assert rl.status_code == 200
        state = (await client.get(f"/lobby/{game_id}/state", headers=h_host)).json()
        assert state["joiner_display_name"] is None
        assert state["joiner_deck_ready"] is False

        # Seat can be re-taken after leave
        assert (await client.post(f"/lobby/join/{code}", headers=h_join)).status_code == 200

        # Host cancels: code and state both 404 afterwards
        rc = await client.delete(f"/lobby/{game_id}", headers=h_host)
        assert rc.status_code == 200
        assert (await client.post(f"/lobby/join/{code}", headers=h_join)).status_code == 404
        assert (await client.get(f"/lobby/{game_id}/state", headers=h_host)).status_code == 404

    async def test_state_requires_auth(self, client: AsyncClient):
        _, h_host = await _guest(client)
        game_id, _ = await _create_room(client, h_host)
        resp = await client.get(f"/lobby/{game_id}/state")
        assert resp.status_code == 401

    async def test_ttl_prunes_stale_rooms(self, client: AsyncClient):
        from server.routers.lobby import _LOBBY_TTL, pending_games

        _, h_host = await _guest(client)
        game_id, code = await _create_room(client, h_host)
        # Age the room past the TTL
        pending_games[game_id].created_at = (
            datetime.now(timezone.utc) - _LOBBY_TTL - timedelta(minutes=1)
        )
        # Any prune-triggering call sweeps it
        _, h_other = await _guest(client)
        await _create_room(client, h_other)
        assert game_id not in pending_games
        assert (await client.post(f"/lobby/join/{code}", headers=h_host)).status_code == 404


class TestRule14Redaction:
    """Rule 14 over the new state source: opponent handIds AND handCards are
    redacted, and securityIds never leave the server, on a real
    RustHeadlessGame.to_ui_json() payload."""

    def _live_state(self):
        from digimon_engine import RustHeadlessGame

        game = RustHeadlessGame(_legal_deck(), _legal_deck(), False, False, False, 42)
        return game.to_ui_json()

    def test_player_filter_redacts_opponent_hand_and_all_security(self):
        from server.state_filter import filter_state_for_player

        full = self._live_state()
        # Sanity: the raw payload really contains hidden info to redact
        assert full["player1"]["handIds"] or full["player2"]["handIds"]

        for me, opp in ((1, "player2"), (2, "player1")):
            filtered = filter_state_for_player(full, me)
            assert filtered[opp]["handIds"] == []
            assert filtered[opp]["handCards"] == []
            assert filtered["player1"]["securityIds"] == []
            assert filtered["player2"]["securityIds"] == []
            # Own hand survives
            mine = "player1" if me == 1 else "player2"
            assert filtered[mine]["handIds"] == full[mine]["handIds"]

    def test_spectator_filter_redacts_both_hands(self):
        from server.state_filter import filter_state_for_spectator

        full = self._live_state()
        filtered = filter_state_for_spectator(full, "hidden")
        for key in ("player1", "player2"):
            assert filtered[key]["handIds"] == []
            assert filtered[key]["handCards"] == []
            assert filtered[key]["securityIds"] == []


def test_pvp_path_imports_no_legacy_engine():
    """Guardrail (rule 22): the PvP path must not import engine_py_legacy.

    Checks actual import statements (docstrings may mention the module by
    name when documenting this very rule)."""
    import ast
    import importlib

    for mod_name in (
        "server.routers.lobby",
        "server.routers.matchmaking",
        "server.routers.ws_games",
        "server.routers.ws_manager",
        "server.state_filter",
    ):
        mod = importlib.import_module(mod_name)
        tree = ast.parse(open(mod.__file__, encoding="utf-8").read())
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names = [a.name for a in node.names]
            elif isinstance(node, ast.ImportFrom):
                names = [node.module or ""]
            else:
                continue
            for name in names:
                assert not name.startswith("engine_py_legacy"), (
                    f"{mod_name} imports {name}"
                )
