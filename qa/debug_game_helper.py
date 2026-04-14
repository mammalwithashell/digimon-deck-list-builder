"""Helper module for running debug game QA.

Greedy baselines run in-process via engine directly (fast).
Targeted debug games use the HTTP API for fine-grained control.
"""

import json
import traceback
from pathlib import Path

import requests

BASE_URL = "http://localhost:8000"
DECK_LIBRARY_PATH = Path(__file__).parent.parent / "digimon_gym" / "engine" / "data" / "deck_library.json"


def load_deck(archetype_name: str) -> list[str]:
    """Load the first decklist for an archetype from deck_library.json."""
    with open(DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)
    arch = library["archetypes"].get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists for archetype: {archetype_name}")
    return json.loads(arch["decklists"][0]["decklist"])


# ---------------------------------------------------------------------------
# Greedy baseline — runs in-process for speed (no HTTP round-trips)
# ---------------------------------------------------------------------------

def run_greedy_baseline(archetype1: str, archetype2: str) -> dict:
    """Run a greedy baseline game in-process using the engine directly.

    Returns dict with: matchup, completed, turns, winner, steps, crash_error
    """
    try:
        deck1 = load_deck(archetype1)
        deck2 = load_deck(archetype2)

        from digimon_gym.digimon_gym import DigimonEnv, greedy_policy

        env = DigimonEnv()
        env.reset(options={"deck1": deck1, "deck2": deck2})

        done = False
        steps = 0
        max_steps = 500

        while not done and steps < max_steps:
            action = greedy_policy(env)
            _, _, done, _, _ = env.step(action)
            steps += 1

        game = env.runner.game
        winner_id = game.winner.player_id if game.winner else 0
        turns = game.turn_count

        return {
            "matchup": f"{archetype1} vs {archetype2}",
            "completed": done,
            "turns": turns,
            "winner": winner_id,
            "steps": steps,
            "crash_error": None if done else f"Exceeded max_steps ({max_steps})",
        }
    except Exception as e:
        return {
            "matchup": f"{archetype1} vs {archetype2}",
            "completed": False,
            "turns": 0,
            "winner": None,
            "steps": 0,
            "crash_error": f"{type(e).__name__}: {e}\n{traceback.format_exc()}",
        }


# ---------------------------------------------------------------------------
# Targeted debug games — via HTTP API for deterministic control
# ---------------------------------------------------------------------------

def create_debug_game(
    deck1: list[str],
    deck2: list[str],
    starting_hand1: list[str] | None = None,
    starting_hand2: list[str] | None = None,
    initial_memory: int = 10,
    skip_shuffle: bool = True,
) -> dict:
    """Create a debug game with seeded hands for targeted testing."""
    resp = requests.post(f"{BASE_URL}/debug/games", json={
        "deck1": deck1,
        "deck2": deck2,
        "player1_type": "human",
        "player2_type": "agent",
        "player1_policy": "greedy",
        "player2_policy": "greedy",
        "agent_action_delay_ms": 0,
        "skip_shuffle": skip_shuffle,
        "auto_mulligan": "keep",
        "initial_memory": initial_memory,
        "starting_hand1": starting_hand1 or [],
        "starting_hand2": starting_hand2 or [],
    })
    resp.raise_for_status()
    return resp.json()


def get_actions(game_id: str) -> dict:
    """Get available actions with descriptions."""
    resp = requests.get(f"{BASE_URL}/games/{game_id}/actions")
    resp.raise_for_status()
    return resp.json()


def get_action_mask(game_id: str) -> list:
    """Get the action mask."""
    resp = requests.get(f"{BASE_URL}/games/{game_id}/action-mask")
    resp.raise_for_status()
    return resp.json()["action_mask"]


def execute_action(game_id: str, action: int) -> dict:
    """Execute a single action."""
    resp = requests.post(
        f"{BASE_URL}/games/{game_id}/actions",
        json={"action": action},
    )
    resp.raise_for_status()
    return resp.json()


def step_game(game_id: str) -> dict:
    """Advance game until next human turn or game over."""
    resp = requests.post(f"{BASE_URL}/games/{game_id}/steps")
    resp.raise_for_status()
    return resp.json()


def get_internal_state(game_id: str) -> dict:
    """Get full internal state including hidden info."""
    resp = requests.get(f"{BASE_URL}/debug/games/{game_id}/internal-state")
    resp.raise_for_status()
    return resp.json()


def inject_card(game_id: str, player_id: int, card_id: str, zone: str = "hand") -> dict:
    """Inject a card into a player's zone."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/inject-card",
        json={"player_id": player_id, "card_id": card_id, "zone": zone},
    )
    resp.raise_for_status()
    return resp.json()


def set_memory(game_id: str, memory: int) -> dict:
    """Set the memory gauge."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/set-memory",
        json={"memory": memory},
    )
    resp.raise_for_status()
    return resp.json()


def delete_game(game_id: str):
    """Clean up a game."""
    requests.delete(f"{BASE_URL}/games/{game_id}")


def find_action_for_card(actions: dict, card_id: str, action_type: str = "play") -> int | None:
    """Find the action index that plays/digivolves a specific card.

    action_type: 'play', 'digivolve', 'use', 'attack', or substring match
    """
    for idx_str, desc in actions.get("actions", {}).items():
        desc_lower = desc.lower()
        card_lower = card_id.lower()
        if card_lower in desc_lower and action_type.lower() in desc_lower:
            return int(idx_str)
    return None


def find_action_by_substring(actions: dict, substring: str) -> int | None:
    """Find the first action containing the given substring."""
    for idx_str, desc in actions.get("actions", {}).items():
        if substring.lower() in desc.lower():
            return int(idx_str)
    return None


def place_on_field(
    game_id: str, player_id: int, card_ids: list[str],
    is_suspended: bool = False, turn_played: int = -1,
) -> dict:
    """Place a digivolution stack on a player's field."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/place-on-field",
        json={
            "player_id": player_id, "card_ids": card_ids,
            "is_suspended": is_suspended, "turn_played": turn_played,
        },
    )
    resp.raise_for_status()
    return resp.json()


def place_in_breeding(game_id: str, player_id: int, card_ids: list[str]) -> dict:
    """Place a digivolution stack in a player's breeding area."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/place-in-breeding",
        json={"player_id": player_id, "card_ids": card_ids},
    )
    resp.raise_for_status()
    return resp.json()


def bulk_setup(game_id: str, config: dict) -> dict:
    """Apply bulk state manipulation to a game."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/bulk-setup",
        json=config,
    )
    resp.raise_for_status()
    return resp.json()


def clear_zone(game_id: str, player_id: int, zone: str) -> dict:
    """Clear all cards from a player's zone."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/clear-zone",
        json={"player_id": player_id, "zone": zone},
    )
    resp.raise_for_status()
    return resp.json()


def set_phase(game_id: str, phase: str) -> dict:
    """Set the game phase."""
    resp = requests.post(
        f"{BASE_URL}/debug/games/{game_id}/set-phase",
        json={"phase": phase},
    )
    resp.raise_for_status()
    return resp.json()


def advance_past_breeding(game_id: str) -> dict:
    """Pass the breeding phase to reach main phase.

    Returns the action result from passing breeding, or None if already in main.
    """
    actions = get_actions(game_id)
    # Check if we're in breeding - look for "Pass breeding" action
    pass_breeding = find_action_by_substring(actions, "pass breeding")
    if pass_breeding is not None:
        return execute_action(game_id, pass_breeding)
    return None


def run_targeted_test(
    archetype1: str,
    archetype2: str,
    card_id: str,
    description: str,
    setup_fn=None,
    verify_fn=None,
    starting_hand1: list[str] | None = None,
    initial_memory: int = 10,
) -> dict:
    """Run a targeted debug game to test a specific card.

    setup_fn(game_id): Optional function to inject cards / set up board state.
    verify_fn(game_id, state_before, state_after, action_result): Check post-action state.
        Should return (passed: bool, details: str).

    Returns dict with card_id, description, passed, details.
    """
    result = {
        "card_id": card_id,
        "description": description,
        "passed": False,
        "details": "",
    }

    try:
        deck1 = load_deck(archetype1)
        deck2 = load_deck(archetype2)

        hand = starting_hand1 or [card_id]
        game_data = create_debug_game(
            deck1, deck2,
            starting_hand1=hand,
            initial_memory=initial_memory,
        )
        game_id = game_data["game_id"]

        # Run setup
        if setup_fn:
            setup_fn(game_id)

        # Get internal state before
        state_before = get_internal_state(game_id)

        # Get available actions
        actions = get_actions(game_id)

        # Try to find and execute the relevant action for the card
        action_idx = find_action_for_card(actions, card_id, "play")
        if action_idx is None:
            action_idx = find_action_for_card(actions, card_id, "digivolve")
        if action_idx is None:
            action_idx = find_action_for_card(actions, card_id, "use")
        if action_idx is None:
            action_idx = find_action_by_substring(actions, card_id)

        if action_idx is not None:
            action_result = execute_action(game_id, action_idx)

            # Resolve follow-up selection phases (up to 20 steps)
            for _ in range(20):
                if action_result.get("is_game_over"):
                    break
                state = action_result.get("state", {})
                if state.get("gameOver"):
                    break
                mask = get_action_mask(game_id)
                legal = [i for i, m in enumerate(mask) if m == 1]
                if not legal:
                    break
                # Auto-select first legal action for follow-up effects
                action_result = execute_action(game_id, legal[0])

            # Verify
            state_after = get_internal_state(game_id)
            if verify_fn:
                passed, details = verify_fn(game_id, state_before, state_after, action_result)
                result["passed"] = passed
                result["details"] = details
            else:
                # Default: just check no crash
                result["passed"] = True
                result["details"] = (
                    f"Action executed without crash. "
                    f"Memory: {state_after['memory']}, Phase: {state_after['phase']}"
                )
        else:
            # Card action not directly available
            hand = state_before.get("hand_p1", [])
            result["details"] = (
                f"No action found for {card_id}. "
                f"Available actions: {json.dumps(actions.get('actions', {}), indent=2)[:500]}. "
                f"Hand: {hand}"
            )
            mask = get_action_mask(game_id)
            legal = [i for i, m in enumerate(mask) if m == 1]
            if legal:
                result["details"] += f"\nLegal action indices: {legal[:20]}"

        delete_game(game_id)

    except Exception as e:
        result["details"] = f"CRASH: {type(e).__name__}: {e}\n{traceback.format_exc()}"

    return result
