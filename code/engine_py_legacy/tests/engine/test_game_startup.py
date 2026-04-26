"""Tests that verify games can start with decks from all promoted card sets.

Catches issues like SyntaxErrors in generated/frozen scripts that would
prevent CardDatabase from loading and crash game creation in the UI.
"""

import ast
import os
import pytest
import numpy as np

from digimon_gym.digimon_gym import greedy_policy
from engine_py_legacy.engine.runners.headless_game import HeadlessGame
from engine_py_legacy.engine.runners.interactive_game import InteractiveGame
from engine_py_legacy.engine.data.enums import GamePhase, PlayerType
from engine_py_legacy.engine.game import ACTION_SPACE_SIZE


# ─── Constants ──────────────────────────────────────────────────────

SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts"
)

# Promoted sets and representative cards (egg + non-egg) for deck building.
# Each tuple: (set_id, egg_card_id, non_egg_card_id)
PROMOTED_SETS = [
    ("bt8", "BT8-085", "BT8-001"),
    ("bt10", "BT10-087", "BT10-001"),
    ("bt11", "BT11-089", "BT11-001"),
    ("bt13", "BT13-094", "BT13-001"),
    ("bt15", "BT15-082", "BT15-001"),
    ("bt16", "BT16-084", "BT16-001"),
    ("bt19", "BT19-079", "BT19-001"),
    ("bt22", "BT22-083", "BT22-001"),
    ("ex5", "EX5-064", "EX5-001"),
    ("ex6", "EX6-063", "EX6-001"),
    ("ex8", "EX8-065", "EX8-001"),
    ("st12", None, "ST12-01"),  # ST12 has no eggs
]

DEFAULT_EGG = "ST1-01"
DEFAULT_NON_EGG = "ST1-03"


def make_set_deck(egg_id, non_egg_id):
    """Build a 50-card deck using the given egg and non-egg card IDs."""
    egg = egg_id or DEFAULT_EGG
    return [egg] * 5 + [non_egg_id] * 45


def resolve_opening_mulligan(runner) -> None:
    """Advance through opening mulligan decisions by keeping hand."""
    guard = 0
    while runner.game.current_phase == GamePhase.Mulligan and guard < 4:
        mask = runner.get_action_mask()
        valid = np.where(mask > 0.5)[0]
        action = int(valid[0]) if len(valid) else 0
        runner.step(action)
        guard += 1



# ─── Frozen script syntax validation ────────────────────────────────

def get_frozen_script_files():
    """Yield (set_id, filepath) for every frozen .py script."""
    for set_id, _, _ in PROMOTED_SETS:
        set_dir = os.path.join(SCRIPTS_DIR, set_id)
        if not os.path.isdir(set_dir):
            continue
        for fname in sorted(os.listdir(set_dir)):
            if fname.endswith(".py") and fname != "__init__.py":
                yield set_id, os.path.join(set_dir, fname)


FROZEN_SCRIPTS = list(get_frozen_script_files())


@pytest.mark.parametrize(
    "set_id,filepath",
    FROZEN_SCRIPTS,
    ids=[f"{s}/{os.path.basename(f)}" for s, f in FROZEN_SCRIPTS],
)
def test_frozen_script_syntax(set_id, filepath):
    """Verify each frozen script is syntactically valid Python."""
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()
    # ast.parse raises SyntaxError if the source is invalid
    ast.parse(source, filename=filepath)


# ─── Game startup with promoted set decks ────────────────────────────

@pytest.mark.parametrize(
    "set_id,egg_id,non_egg_id",
    PROMOTED_SETS,
    ids=[s for s, _, _ in PROMOTED_SETS],
)
def test_headless_game_starts_with_set_deck(set_id, egg_id, non_egg_id):
    """Verify a HeadlessGame can be constructed with cards from each promoted set."""
    deck = make_set_deck(egg_id, non_egg_id)
    game = HeadlessGame(deck, deck)
    assert not game.is_game_over
    assert game.game.current_phase == GamePhase.Mulligan


@pytest.mark.parametrize(
    "set_id,egg_id,non_egg_id",
    PROMOTED_SETS,
    ids=[s for s, _, _ in PROMOTED_SETS],
)
def test_interactive_game_starts_human_vs_agent(set_id, egg_id, non_egg_id):
    """Verify InteractiveGame with Human vs greedy Agent starts with each set."""
    deck = make_set_deck(egg_id, non_egg_id)
    game = InteractiveGame(
        deck, deck,
        player1_type=PlayerType.Human,
        player2_type=PlayerType.Agent,
        player2_policy="greedy",
    )
    assert not game.is_game_over
    assert game.player1_type == PlayerType.Human
    assert game.player2_type == PlayerType.Agent
    assert game.game.current_phase == GamePhase.Mulligan


# ─── Greedy agent integration ───────────────────────────────────────

def test_greedy_agent_plays_through_mulligan():
    """Verify a greedy agent can play through the mulligan phase."""
    deck = [DEFAULT_EGG] * 5 + [DEFAULT_NON_EGG] * 45
    game = InteractiveGame(
        deck, deck,
        player1_type=PlayerType.Agent,
        player2_type=PlayerType.Agent,
        player1_policy="greedy",
        player2_policy="greedy",
        agent_action_delay_ms=0,
    )
    assert game.game.current_phase == GamePhase.Mulligan

    # Run agent steps through mulligan
    for _ in range(10):
        state = game.run_step()
        if game.game.current_phase != GamePhase.Mulligan:
            break

    assert game.game.current_phase != GamePhase.Mulligan


def test_greedy_agent_completes_game():
    """Verify two greedy agents can play a full game to completion."""
    deck = [DEFAULT_EGG] * 5 + [DEFAULT_NON_EGG] * 45
    game = HeadlessGame(deck, deck)

    def greedy_wrapper(g, mask):
        class _Env:
            def __init__(self):
                self.game = g
            def get_action_mask(self):
                return mask
        return greedy_policy(_Env())

    winner = game.run_until_conclusion(max_turns=300, policy_fn=greedy_wrapper)
    assert winner in (0, 1, 2)
    assert game.is_game_over


def test_human_vs_greedy_agent_run_step():
    """Verify run_step pauses for human and auto-plays for greedy agent."""
    deck = [DEFAULT_EGG] * 5 + [DEFAULT_NON_EGG] * 45
    game = InteractiveGame(
        deck, deck,
        player1_type=PlayerType.Human,
        player2_type=PlayerType.Agent,
        player2_policy="greedy",
        agent_action_delay_ms=0,
    )
    resolve_opening_mulligan(game)

    # Force P2 (agent) turn
    game.game.turn_player = game.game.player2
    game.game.opponent_player = game.game.player1
    game.game.player1.is_my_turn = False
    game.game.player2.is_my_turn = True
    game.game.current_phase = GamePhase.Breeding

    state = game.run_step()
    assert "currentPhase" in state
    # Agent should have acted and returned to human turn
    assert game.is_current_player_human() or game.is_game_over


def test_interactive_game_action_mask_valid():
    """Verify action mask is correctly shaped during human turn."""
    deck = [DEFAULT_EGG] * 5 + [DEFAULT_NON_EGG] * 45
    game = InteractiveGame(
        deck, deck,
        player1_type=PlayerType.Human,
        player2_type=PlayerType.Agent,
        agent_action_delay_ms=0,
    )
    mask = game.get_action_mask()
    assert mask.shape == (ACTION_SPACE_SIZE,)
    assert mask.dtype == np.float32
    # At least one action should be valid
    assert np.any(mask > 0.5)


# ─── Multi-set deck test ────────────────────────────────────────────

def test_headless_game_with_mixed_set_deck():
    """Verify a game starts with a deck mixing cards from multiple promoted sets."""
    non_egg_cards = [nid for _, _, nid in PROMOTED_SETS]
    # Build a 50-card deck: 5 ST1 eggs + 45 non-eggs cycling through sets
    deck = [DEFAULT_EGG] * 5
    for i in range(45):
        deck.append(non_egg_cards[i % len(non_egg_cards)])
    game = HeadlessGame(deck, deck)
    assert not game.is_game_over
    assert game.game.current_phase == GamePhase.Mulligan
