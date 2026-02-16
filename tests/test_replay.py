"""Tests for ReplayRunner: zone restoration, step-through, seek, verification, and API endpoints."""

import json
import os
import sys

import pytest
import numpy as np
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.recording import GameRecorder, ReplayStepResult
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.replay_runner import ReplayRunner
from digimon_gym.engine.data.enums import GamePhase
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.game import TENSOR_SIZE, ACTION_SPACE_SIZE
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base


# ── Helpers ──────────────────────────────────────────────────────────

def make_test_deck():
    """Return a valid deck of card IDs for testing."""
    return ["ST1-01"] * 5 + ["ST1-03"] * 45


def record_a_game(max_turns=200):
    """Play a full game with recording enabled and return the recording dict."""
    deck = make_test_deck()
    game = HeadlessGame(deck, deck, record_actions=True)
    game.run_until_conclusion(max_turns=max_turns)
    return game.get_recording()


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test to ensure isolation."""
    CardRegistry.reset()
    yield
    CardRegistry.reset()


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


# ── ReplayRunner Unit Tests ──────────────────────────────────────────

class TestReplayRunnerConstruction:
    def test_missing_initial_state_raises(self):
        """Recording without initial_state should raise ValueError."""
        with pytest.raises(ValueError, match="initial_state"):
            ReplayRunner({"actions": []})

    def test_empty_actions_is_valid(self):
        """Recording with initial_state but no actions is valid."""
        recording = record_a_game()
        recording["actions"] = []
        runner = ReplayRunner(recording)
        assert runner.total_steps == 0
        assert runner.current_step == 0
        assert runner.is_complete is True


class TestReplayZoneRestoration:
    def test_hand_size_after_construction(self):
        """After zone restoration, each player should have 5 cards in hand."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        assert len(runner.game.player1.hand_cards) == 5
        assert len(runner.game.player2.hand_cards) == 5

    def test_security_size_after_construction(self):
        """After zone restoration, each player should have 5 security cards."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        assert len(runner.game.player1.security_cards) == 5
        assert len(runner.game.player2.security_cards) == 5

    def test_first_player_matches(self):
        """First player should match the recording's first_player_id."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        expected = recording["initial_state"]["first_player_id"]
        assert runner.game.turn_player.player_id == expected

    def test_total_cards_preserved(self):
        """Total cards across all zones should equal the original deck size."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        for player in [runner.game.player1, runner.game.player2]:
            total = (
                len(player.library_cards)
                + len(player.digitama_library_cards)
                + len(player.security_cards)
                + len(player.hand_cards)
            )
            assert total == 50  # 5 eggs + 45 cards

    def test_initial_phase_is_breeding(self):
        """After construction, game should be parked at Breeding phase."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        assert runner.game.current_phase == GamePhase.Breeding


class TestReplayStep:
    def test_step_advances_current_step(self):
        """Stepping should increment current_step."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        assert runner.current_step == 0
        runner.step()
        assert runner.current_step == 1

    def test_step_returns_replay_step_result(self):
        """Step should return a ReplayStepResult with populated fields."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        result = runner.step()
        assert isinstance(result, ReplayStepResult)
        assert result.step_number == 1
        assert result.action_id == recording["actions"][0]["action_id"]
        assert result.player_id in (1, 2)
        assert "TurnCount" in result.state  # game.to_json() key

    def test_step_raises_when_complete(self):
        """Step should raise IndexError after all actions replayed."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        runner.run_to_completion()
        with pytest.raises(IndexError, match="No more actions"):
            runner.step()

    def test_step_changes_game_state(self):
        """After stepping, the game state should reflect the action."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        state_before = runner.get_state()
        runner.step()
        state_after = runner.get_state()
        # Some state should have changed (phase, memory, or turn)
        assert state_before != state_after or runner.total_steps == 1


class TestReplaySeek:
    def test_seek_forward(self):
        """Seek to step 5 should advance current_step to 5."""
        recording = record_a_game()
        if len(recording["actions"]) < 5:
            pytest.skip("Not enough actions in recording")
        runner = ReplayRunner(recording)
        result = runner.seek(5)
        assert runner.current_step == 5
        assert result.step_number == 5

    def test_seek_backward(self):
        """Seek backward should rebuild from scratch."""
        recording = record_a_game()
        if len(recording["actions"]) < 10:
            pytest.skip("Not enough actions in recording")
        runner = ReplayRunner(recording)
        runner.seek(10)
        assert runner.current_step == 10
        result = runner.seek(3)
        assert runner.current_step == 3
        assert result.step_number == 3

    def test_seek_out_of_range_raises(self):
        """Seek to invalid step should raise ValueError."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        with pytest.raises(ValueError):
            runner.seek(0)
        with pytest.raises(ValueError):
            runner.seek(runner.total_steps + 1)

    def test_seek_to_same_step(self):
        """Seeking to current step should rebuild and replay."""
        recording = record_a_game()
        if len(recording["actions"]) < 3:
            pytest.skip("Not enough actions")
        runner = ReplayRunner(recording)
        runner.seek(3)
        # Seek to same step should still work (rebuild + replay)
        result = runner.seek(3)
        assert runner.current_step == 3


class TestReplayRunToCompletion:
    def test_run_to_completion(self):
        """run_to_completion should replay all actions."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        result = runner.run_to_completion()
        assert runner.is_complete is True
        assert runner.current_step == runner.total_steps
        assert result.is_game_over is True

    def test_run_to_completion_empty_raises(self):
        """run_to_completion with no actions should raise IndexError."""
        recording = record_a_game()
        recording["actions"] = []
        runner = ReplayRunner(recording)
        with pytest.raises(IndexError, match="No actions"):
            runner.run_to_completion()


class TestReplayRoundTrip:
    def test_winner_matches_original(self):
        """Replayed game should have the same winner as the original."""
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.run_until_conclusion(max_turns=200)
        original_winner = game.winner_id
        recording = game.get_recording()

        runner = ReplayRunner(recording)
        result = runner.run_to_completion()
        assert result.winner_id == original_winner

    def test_turn_count_matches_original(self):
        """Replayed game should end on the same turn as the original."""
        deck = make_test_deck()
        game = HeadlessGame(deck, deck, record_actions=True)
        game.run_until_conclusion(max_turns=200)
        original_turns = game.game.turn_count
        recording = game.get_recording()

        runner = ReplayRunner(recording)
        runner.run_to_completion()
        assert runner.game.turn_count == original_turns


class TestReplayVerification:
    def test_verify_all_steps_pass(self):
        """With verify=True, a clean recording should have no verification errors."""
        recording = record_a_game()
        runner = ReplayRunner(recording, verify=True)
        result = runner.run_to_completion()
        # Final step should have verification_ok
        assert result.verification_ok is True
        assert result.verification_errors == []

    def test_verify_each_step(self):
        """All individual steps should pass verification."""
        recording = record_a_game()
        runner = ReplayRunner(recording, verify=True)
        while not runner.is_complete:
            result = runner.step()
            assert result.verification_ok is True, (
                f"Step {result.step_number} failed: {result.verification_errors}"
            )

    def test_tampered_memory_detected(self):
        """Modifying recorded memory_after should trigger verification error."""
        recording = record_a_game()
        if not recording["actions"]:
            pytest.skip("No actions in recording")
        # Tamper with the first action's memory_after
        recording["actions"][0]["memory_after"] = -999
        runner = ReplayRunner(recording, verify=True)
        result = runner.step()
        assert result.verification_ok is False
        assert any("memory_after" in e for e in result.verification_errors)


class TestReplayStateInspection:
    def test_get_state(self):
        """get_state should return a valid game state dict."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        state = runner.get_state()
        assert "TurnCount" in state
        assert "CurrentPhase" in state
        assert "Player1" in state
        assert "Player2" in state

    def test_get_board_tensor(self):
        """get_board_tensor should return 981-element float32 array."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        tensor = runner.get_board_tensor(1)
        assert isinstance(tensor, np.ndarray)
        assert tensor.dtype == np.float32
        assert len(tensor) == TENSOR_SIZE

    def test_get_action_mask(self):
        """get_action_mask should return ACTION_SPACE_SIZE-element float32 array."""
        recording = record_a_game()
        runner = ReplayRunner(recording)
        mask = runner.get_action_mask()
        assert isinstance(mask, np.ndarray)
        assert mask.dtype == np.float32
        assert len(mask) == ACTION_SPACE_SIZE


# ── API Endpoint Tests ───────────────────────────────────────────────

class TestReplayAPIEndpoints:
    async def test_create_replay(self, client: AsyncClient):
        """POST /games/replay should create a replay session."""
        recording = record_a_game()
        resp = await client.post("/games/replay", json={
            "recording": recording,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert "replay_id" in data
        assert data["total_steps"] == recording["total_actions"]
        assert "initial_state" in data

    async def test_create_replay_with_verify(self, client: AsyncClient):
        """POST /games/replay with verify=True should succeed."""
        recording = record_a_game()
        resp = await client.post("/games/replay", json={
            "recording": recording,
            "verify": True,
        })
        assert resp.status_code == 200

    async def test_create_replay_invalid_recording(self, client: AsyncClient):
        """POST /games/replay with bad recording should return 400."""
        resp = await client.post("/games/replay", json={
            "recording": {"actions": []},
        })
        assert resp.status_code == 400

    async def test_replay_step(self, client: AsyncClient):
        """POST /games/replay/{id}/step should advance one action."""
        recording = record_a_game()
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        resp = await client.post(f"/games/replay/{replay_id}/step")
        assert resp.status_code == 200
        data = resp.json()
        assert data["step_number"] == 1
        assert "action_id" in data
        assert "state" in data

    async def test_replay_step_complete_returns_400(self, client: AsyncClient):
        """Step after replay is complete should return 400."""
        recording = record_a_game()
        recording["actions"] = recording["actions"][:1]  # Only 1 action
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        # Step once (completes replay)
        await client.post(f"/games/replay/{replay_id}/step")
        # Step again should fail
        resp = await client.post(f"/games/replay/{replay_id}/step")
        assert resp.status_code == 400

    async def test_replay_seek(self, client: AsyncClient):
        """POST /games/replay/{id}/seek should jump to specified step."""
        recording = record_a_game()
        if recording["total_actions"] < 5:
            pytest.skip("Not enough actions")
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        resp = await client.post(f"/games/replay/{replay_id}/seek", json={"step": 5})
        assert resp.status_code == 200
        assert resp.json()["step_number"] == 5

    async def test_replay_seek_backward(self, client: AsyncClient):
        """Seek backward should work (rebuilds from scratch)."""
        recording = record_a_game()
        if recording["total_actions"] < 10:
            pytest.skip("Not enough actions")
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        # Seek forward to 10
        await client.post(f"/games/replay/{replay_id}/seek", json={"step": 10})
        # Seek backward to 3
        resp = await client.post(f"/games/replay/{replay_id}/seek", json={"step": 3})
        assert resp.status_code == 200
        assert resp.json()["step_number"] == 3

    async def test_replay_seek_invalid_step(self, client: AsyncClient):
        """Seek with invalid step should return 400."""
        recording = record_a_game()
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        resp = await client.post(f"/games/replay/{replay_id}/seek", json={"step": 0})
        assert resp.status_code == 400

        resp = await client.post(
            f"/games/replay/{replay_id}/seek",
            json={"step": recording["total_actions"] + 100},
        )
        assert resp.status_code == 400

    async def test_delete_replay(self, client: AsyncClient):
        """DELETE /games/replay/{id} should destroy the session."""
        recording = record_a_game()
        resp = await client.post("/games/replay", json={"recording": recording})
        replay_id = resp.json()["replay_id"]

        resp = await client.delete(f"/games/replay/{replay_id}")
        assert resp.status_code == 200
        assert resp.json()["status"] == "deleted"

        # Step should now 404
        resp = await client.post(f"/games/replay/{replay_id}/step")
        assert resp.status_code == 404

    async def test_replay_nonexistent_404(self, client: AsyncClient):
        """Operations on nonexistent replay should return 404."""
        resp = await client.post("/games/replay/nonexistent/step")
        assert resp.status_code == 404

    async def test_stateless_recording_state(self, client: AsyncClient):
        """GET /games/recordings/{id}/state?step=N should return state at step N."""
        # First create and save a recording
        deck = make_test_deck()
        resp = await client.post("/game/create", json={
            "deck1": deck,
            "deck2": deck,
            "record_actions": True,
        })
        game_id = resp.json()["game_id"]

        # Take some actions
        for _ in range(5):
            await client.post(f"/game/{game_id}/action", json={"action": 62})

        # Save recording
        resp = await client.post(f"/game/{game_id}/recording/save")
        assert resp.status_code == 200
        recording_id = resp.json()["recording_id"]

        # Stateless replay to step 3
        resp = await client.get(f"/games/recordings/{recording_id}/state", params={"step": 3})
        assert resp.status_code == 200
        state = resp.json()
        assert "TurnCount" in state

    async def test_stateless_recording_state_step_0(self, client: AsyncClient):
        """GET /games/recordings/{id}/state?step=0 should return initial state."""
        deck = make_test_deck()
        resp = await client.post("/game/create", json={
            "deck1": deck,
            "deck2": deck,
            "record_actions": True,
        })
        game_id = resp.json()["game_id"]
        await client.post(f"/game/{game_id}/action", json={"action": 62})

        resp = await client.post(f"/game/{game_id}/recording/save")
        recording_id = resp.json()["recording_id"]

        resp = await client.get(f"/games/recordings/{recording_id}/state", params={"step": 0})
        assert resp.status_code == 200
        state = resp.json()
        assert state["TurnCount"] == 1

    async def test_stateless_recording_not_found(self, client: AsyncClient):
        """GET /games/recordings/nonexistent/state should return 404."""
        resp = await client.get("/games/recordings/nonexistent-id/state", params={"step": 0})
        assert resp.status_code == 404
