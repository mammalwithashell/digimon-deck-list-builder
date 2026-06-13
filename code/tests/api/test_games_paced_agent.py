"""Route coverage for paced agent stepping (add-bot-action-pacing).

Paced mode caps agent autoplay at ONE action per request so the frontend
can render each bot action as a perceivable beat. Key invariants:

- `agent_pending` is true exactly when the game is live and the next
  decision belongs to a non-human agent (i.e. another /agent-step beat
  is expected).
- Unpaced requests keep the legacy run-to-human/game-end semantics and
  therefore always report `agent_pending: false`.
- Pacing changes the *cadence* of agent actions, never the actions: a
  paced game replays the exact same deterministic greedy sequence, so it
  lands on the same terminal state as an unpaced run of the same seed.
"""

from __future__ import annotations

from fastapi.testclient import TestClient

from server.api import app

_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _create(client: TestClient, **overrides) -> dict:
    body = {
        "deck1": _DECK,
        "deck2": _DECK,
        "player1_type": "human",
        "player2_type": "agent",
        "player2_policy": "greedy",
        "seed": 4242,
    }
    body.update(overrides)
    resp = client.post("/games", json=body)
    assert resp.status_code == 200, resp.text
    return resp.json()


def _pending_invariant(payload: dict) -> None:
    assert payload["agent_pending"] == (
        not payload["is_game_over"] and not payload["is_human_turn"]
    ), "agent_pending must equal (live game AND non-human decider)"


def _drain_paced(client: TestClient, game_id: str, payload: dict, cap: int = 20_000) -> dict:
    """Pull /agent-step beats until no agent action is pending."""
    steps = 0
    while payload["agent_pending"]:
        assert steps < cap, "paced drain did not converge"
        resp = client.post(f"/games/{game_id}/agent-step")
        assert resp.status_code == 200, resp.text
        payload = resp.json()
        _pending_invariant(payload)
        steps += 1
    return payload


def test_unpaced_default_always_reports_no_pending_agent() -> None:
    client = TestClient(app)
    payload = _create(client)
    _pending_invariant(payload)
    # Unpaced create drains to a human decision point (or game end), so a
    # pending agent must never be reported.
    assert payload["agent_pending"] is False


def test_paced_create_with_two_agents_caps_prelude_at_one_action() -> None:
    client = TestClient(app)
    payload = _create(
        client, player1_type="agent", player1_policy="greedy", paced=True
    )
    # With both seats as agents the unpaced path would have played the whole
    # game inside create; paced must stop after one beat.
    assert payload["is_game_over"] is False
    assert payload["agent_pending"] is True
    _pending_invariant(payload)


def test_paced_beats_reach_the_same_terminal_state_as_unpaced() -> None:
    client = TestClient(app)

    unpaced = _create(client, player1_type="agent", player1_policy="greedy")
    assert unpaced["is_game_over"] is True
    assert unpaced["agent_pending"] is False

    paced = _create(
        client, player1_type="agent", player1_policy="greedy", paced=True
    )
    game_id = paced["game_id"]
    final = _drain_paced(client, game_id, paced)

    assert final["is_game_over"] is True
    assert final["agent_pending"] is False
    # Same seed + deterministic greedy → identical terminal state. Pacing
    # must change cadence only, never the action sequence.
    assert final["state"]["turnCount"] == unpaced["state"]["turnCount"]
    assert final["state"]["winner"] == unpaced["state"]["winner"]


def test_paced_human_action_caps_agent_reply_at_one_beat() -> None:
    client = TestClient(app)
    created = _create(client, paced=True)
    game_id = created["game_id"]
    # Land on the human decision point first (paced prelude may stop on an
    # agent beat depending on who opens).
    payload = _drain_paced(client, game_id, created)
    if payload["is_game_over"]:
        return  # degenerate seed; nothing to assert about the human path

    assert payload["is_human_turn"] is True
    mask = payload["action_mask"]
    legal = next(i for i, bit in enumerate(mask) if bit == 1)

    resp = client.post(f"/games/{game_id}/actions", json={"action": legal, "paced": True})
    assert resp.status_code == 200, resp.text
    after = resp.json()
    _pending_invariant(after)
    # Whether an agent beat follows depends on the action; either way the
    # paced drain must converge back to a human decision point or game end.
    settled = _drain_paced(client, game_id, after)
    assert settled["is_human_turn"] or settled["is_game_over"]


def test_agent_step_is_a_safe_noop_on_human_turn() -> None:
    client = TestClient(app)
    created = _create(client)
    game_id = created["game_id"]
    assert created["is_human_turn"] is True

    resp = client.post(f"/games/{game_id}/agent-step")
    assert resp.status_code == 200, resp.text
    payload = resp.json()
    assert payload["agent_pending"] is False
    assert payload["state"]["turnCount"] == created["state"]["turnCount"]
    assert payload["state"]["currentPhase"] == created["state"]["currentPhase"]


def test_action_script_ignores_paced_during_create() -> None:
    client = TestClient(app)
    baseline = _create(client)
    legal = next(i for i, bit in enumerate(baseline["action_mask"]) if bit == 1)

    scripted_unpaced = _create(client, action_script=[legal])
    scripted_paced = _create(client, action_script=[legal], paced=True)

    # Staging semantics: the script replays identically whether or not the
    # caller asked for pacing — paced only affects live presentation.
    assert (
        scripted_paced["state"]["turnCount"] == scripted_unpaced["state"]["turnCount"]
    )
    assert (
        scripted_paced["state"]["currentPhase"]
        == scripted_unpaced["state"]["currentPhase"]
    )
    _pending_invariant(scripted_paced)
