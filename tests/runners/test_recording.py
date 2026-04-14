"""Tests for game recording: GameRecorder, runner integration, and API endpoints."""

import json

import pytest
import numpy as np
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.engine.recording import (
    GameRecorder,
    InitialState,
    PlayerInitialState,
    RecordedAction,
    TensorSnapshot,
)
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.data.enums import GamePhase, PlayerType
from digimon_gym.engine.game import ACTION_SPACE_SIZE, TENSOR_SIZE
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base


# ── Helpers ──────────────────────────────────────────────────────────

def make_test_deck():
    """Return a valid deck of card IDs for testing."""
    return ["ST1-01"] * 5 + ["ST1-03"] * 45


# ── DB fixtures for API tests ────────────────────────────────────────

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
    from digimon_gym.api import app

    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


# ── GameRecorder Unit Tests ──────────────────────────────────────────

class TestGameRecorderUnit:
    def test_initial_state_defaults(self):
        recorder = GameRecorder()
        assert recorder.initial_state is None
        assert recorder.actions == []
        assert recorder.tensor_snapshots == []
        assert recorder.record_tensors is False

    def test_record_tensors_flag(self):
        recorder = GameRecorder(record_tensors=True)
        assert recorder.record_tensors is True

    def test_to_dict_empty(self):
        recorder = GameRecorder()
        d = recorder.to_dict()
        assert d["initial_state"] is None
        assert d["actions"] == []
        assert d["total_actions"] == 0
        assert d["tensor_snapshots_count"] == 0


# ── HeadlessGame Recording Tests ─────────────────────────────────────

class TestHeadlessRecording:
    def test_no_recording_by_default(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        assert game.recorder is None
        assert game.get_recording() is None

    def test_record_actions_creates_recorder(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        assert game.recorder is not None
        assert game.recorder.record_tensors is False

    def test_record_tensors_creates_recorder(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_tensors=True)
        assert game.recorder is not None
        assert game.recorder.record_tensors is True

    def test_initial_state_captured(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        assert game.recorder.initial_state is not None
        initial = game.recorder.initial_state
        assert initial.first_player_id in (1, 2)
        assert initial.player1.player_id == 1
        assert initial.player2.player_id == 2

    def test_initial_state_deck_lists(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        initial = game.recorder.initial_state
        assert initial.player1.deck_list == deck
        assert initial.player2.deck_list == deck

    def test_initial_state_zone_orders(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        initial = game.recorder.initial_state
        # Initial state is captured at Mulligan time (before security setup).
        # Security stacks are set up later during _finalize_opening_setup().
        p1 = initial.player1
        assert len(p1.initial_hand) == 5  # Draw 5
        assert len(p1.security_order) == 0  # Not yet set at capture time
        assert len(p1.digitama_library_order) >= 0  # Egg deck (ST1-01 are eggs)
        # Library = remaining cards after hand (security not yet drawn)
        total_zones = (len(p1.library_order) + len(p1.digitama_library_order)
                       + len(p1.security_order) + len(p1.initial_hand))
        assert total_zones == len(deck)

    def test_record_actions_during_game(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        # Step with pass action
        game.step(62)  # ACTION_PASS (breeding pass)
        assert len(game.recorder.actions) == 1
        action = game.recorder.actions[0]
        assert action.step_number == 1
        assert action.action_id == 62
        assert action.player_id in (1, 2)

    def test_run_until_conclusion_records_actions(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.run_until_conclusion(max_turns=200)
        recording = game.get_recording()
        assert recording is not None
        assert recording["total_actions"] > 0
        # Should have initial state
        assert recording["initial_state"] is not None
        # All actions should have valid structure
        for action in recording["actions"]:
            assert "step" in action
            assert "player_id" in action
            assert "action_id" in action
            assert "phase" in action
            assert "memory_before" in action
            assert "memory_after" in action
            assert "turn" in action

    def test_run_until_conclusion_last_action_marks_game_over(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.run_until_conclusion(max_turns=200)
        recording = game.get_recording()
        # The last action (or near last) should mark the game as over
        last_action = recording["actions"][-1]
        assert last_action["is_game_over"] is True
        assert last_action["winner_id"] in (1, 2)

    def test_tensor_recording_disabled_by_default(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.step(62)
        recording = game.get_recording()
        assert recording["tensor_snapshots_count"] == 0
        assert "tensor_snapshots" not in recording

    def test_tensor_recording_enabled(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_tensors=True)
        game.step(62)
        recording = game.get_recording()
        assert recording["tensor_snapshots_count"] > 0
        assert "tensor_snapshots" in recording
        snapshot = recording["tensor_snapshots"][0]
        assert len(snapshot["tensor"]) == TENSOR_SIZE
        assert len(snapshot["action_mask"]) == ACTION_SPACE_SIZE

    def test_to_dict_serializable(self):
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.run_until_conclusion(max_turns=50)
        recording = game.get_recording()
        # Should be JSON-serializable
        json_str = json.dumps(recording)
        roundtrip = json.loads(json_str)
        assert roundtrip["total_actions"] == recording["total_actions"]

    def test_backward_compat_no_recorder(self):
        """Existing code path without recording must still work."""
        deck = make_test_deck()
        game = HeadlessGame(deck, deck)
        game.run_until_conclusion(max_turns=200)
        assert game.is_game_over
        assert game.winner_id in (1, 2)
        assert game.get_recording() is None


# ── InteractiveGame Recording Tests ──────────────────────────────────

class TestInteractiveRecording:
    def test_recorder_created_automatically(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        assert game.recorder is not None

    def test_initial_state_captured(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        initial = game.get_initial_state_dict()
        assert "first_player_id" in initial
        assert "player1" in initial
        assert "player2" in initial

    def test_initial_state_has_deck_lists(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        initial = game.get_initial_state_dict()
        assert initial["player1"]["deck_list"] == deck
        assert initial["player2"]["deck_list"] == deck

    def test_initial_state_has_zone_orders(self):
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Human, PlayerType.Agent)
        initial = game.get_initial_state_dict()
        p1 = initial["player1"]
        assert "library_order" in p1
        assert "digitama_library_order" in p1
        assert "security_order" in p1
        assert "initial_hand" in p1
        assert len(p1["initial_hand"]) == 5
        # Security stacks are set after mulligans resolve, not at capture time
        assert len(p1["security_order"]) == 0

    def test_interactive_step_still_works(self):
        """InteractiveGame step() must still work without recording actions."""
        deck = make_test_deck()
        game = InteractiveGame(deck, deck, PlayerType.Agent, PlayerType.Agent)
        game.step(62)  # pass
        assert not game.is_game_over


# ── API Endpoint Tests ────────────────────────────────────────────────

class TestAPIRecordingEndpoints:
    async def test_health_check(self, client: AsyncClient):
        resp = await client.get("/")
        assert resp.status_code == 200

    async def test_action_context_in_response(self, client: AsyncClient):
        """POST /game/{id}/action should include action_context."""
        # Create a headless game (both agents)
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
        })
        assert resp.status_code == 200
        game_id = resp.json()["game_id"]

        # Take an action
        resp = await client.post(f"/game/{game_id}/action", json={"action": 62})
        assert resp.status_code == 200
        data = resp.json()
        assert "action_context" in data
        ctx = data["action_context"]
        assert ctx["action_id"] == 62
        assert "player_id" in ctx
        assert "phase" in ctx
        assert "memory_before" in ctx
        assert "memory_after" in ctx
        assert "turn" in ctx

    async def test_headless_recording_disabled_by_default(self, client: AsyncClient):
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
        })
        game_id = resp.json()["game_id"]
        resp = await client.get(f"/game/{game_id}/recording")
        assert resp.status_code == 404

    async def test_headless_recording_enabled(self, client: AsyncClient):
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
            "record_actions": True,
        })
        game_id = resp.json()["game_id"]

        # Take a few actions (pass)
        for _ in range(3):
            await client.post(f"/game/{game_id}/action", json={"action": 62})

        resp = await client.get(f"/game/{game_id}/recording")
        assert resp.status_code == 200
        recording = resp.json()
        assert recording["total_actions"] == 3
        assert recording["initial_state"] is not None

    async def test_save_and_retrieve_recording(self, client: AsyncClient):
        # Create game with recording
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
            "record_actions": True,
        })
        game_id = resp.json()["game_id"]
        await client.post(f"/game/{game_id}/action", json={"action": 62})

        # Save recording
        resp = await client.post(f"/game/{game_id}/recording/save")
        assert resp.status_code == 200
        recording_id = resp.json()["recording_id"]

        # List recordings
        resp = await client.get("/games/recordings")
        assert resp.status_code == 200
        recordings = resp.json()
        assert len(recordings) >= 1
        assert any(r["id"] == recording_id for r in recordings)

        # Get specific recording
        resp = await client.get(f"/games/recordings/{recording_id}")
        assert resp.status_code == 200
        data = resp.json()
        assert data["total_actions"] == 1

    async def test_bug_report_submission(self, client: AsyncClient):
        recording_bundle = {
            "game_id": "test-game-123",
            "recording_metadata": {
                "first_player_id": 1,
                "player1": {"deck_list": ["ST1-01"] * 5},
                "player2": {"deck_list": ["ST1-03"] * 5},
            },
            "actions": [
                {"action_id": 62, "player_id": 1, "phase": "Breeding"},
            ],
            "client_version": "1.0.0",
        }
        resp = await client.post("/games/report", json={
            "description": "Card played but didn't appear on board",
            "recording": recording_bundle,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "submitted"
        assert "report_id" in data

    async def test_interactive_game_no_recording_endpoint(self, client: AsyncClient):
        """Interactive games should not use the /recording endpoint."""
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
            "player1_type": "human",
            "player2_type": "agent",
        })
        game_id = resp.json()["game_id"]
        resp = await client.get(f"/game/{game_id}/recording")
        assert resp.status_code == 400  # "Recording endpoint is for headless games"

    async def test_interactive_game_has_recording_metadata(self, client: AsyncClient):
        """Interactive game creation should include recording_metadata."""
        resp = await client.post("/game/create", json={
            "deck1": make_test_deck(),
            "deck2": make_test_deck(),
            "player1_type": "human",
            "player2_type": "agent",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert "recording_metadata" in data
        meta = data["recording_metadata"]
        assert "first_player_id" in meta
        assert "player1" in meta
        assert "player2" in meta
        assert "deck_list" in meta["player1"]
        assert "library_order" in meta["player1"]
        assert "security_order" in meta["player1"]
        assert "initial_hand" in meta["player1"]

    async def test_nonexistent_recording_404(self, client: AsyncClient):
        resp = await client.get("/games/recordings/nonexistent-id")
        assert resp.status_code == 404
