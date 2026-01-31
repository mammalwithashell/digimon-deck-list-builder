import pytest
import numpy as np
import pytest
import numpy as np
from python_impl.digimon_gym import GameState, greedy_policy, ACTION_TRASH_CARD_START, ACTION_PASS_TURN, ACTION_PLAY_CARD_START, ACTION_HATCH
from python_impl.engine.data.enums import PendingAction, GamePhase
from python_impl.engine.core.card_source import CardSource
from python_impl.engine.core.entity_base import CEntity_Base
from python_impl.engine.data.enums import CardKind

# Mock Card Helper
def create_mock_card(name="Test Card", kind=CardKind.Digimon):
    entity = CEntity_Base()
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.level = 3
    entity.play_cost = 3

    card = CardSource()
    card.c_entity_base = entity
    return card

# Helper to setup a playable state
def setup_playable_state(env):
    # 1. Add cards to hand
    player = env.game.turn_player
    for i in range(5):
        player.hand_cards.append(create_mock_card(f"Card {i}"))

    # 2. Add eggs
    player.digitama_library_cards.append(create_mock_card("Egg", CardKind.DigiEgg))

    # 3. Set Phase to Main
    env.game.current_phase = GamePhase.Main

def test_rl_gym_initial_state():
    env = GameState()
    env.reset()
    setup_playable_state(env)

    # Check mask
    mask = env.get_action_mask()

    # Should allow Play (hand > 0), Hatch (eggs > 0), Pass
    assert mask[ACTION_PASS_TURN] == True
    assert mask[ACTION_HATCH] == True
    assert mask[ACTION_PLAY_CARD_START] == True

    # Should NOT allow Trash (unless effect pending)
    assert mask[ACTION_TRASH_CARD_START] == False

def test_rl_gym_trash_pending():
    env = GameState()
    env.reset()
    setup_playable_state(env)

    # Force pending action
    env.game.pending_action = PendingAction.TRASH_CARD

    mask = env.get_action_mask()

    # Should allow Trash
    assert mask[ACTION_TRASH_CARD_START] == True

    # Should NOT allow Pass or Play
    assert mask[ACTION_PASS_TURN] == False
    assert mask[ACTION_PLAY_CARD_START] == False

    # Execute Trash Action (Index 0)
    action = ACTION_TRASH_CARD_START + 0
    obs, reward, done, info = env.step(action)

    # Verify pending action cleared
    assert env.game.pending_action == PendingAction.NO_ACTION

    # Verify card moved to trash
    # setup_playable_state added 5 cards.
    # After trash, hand should be 4, trash should be 1.
    assert len(env.game.turn_player.hand_cards) == 4
    assert len(env.game.turn_player.trash_cards) == 1

def test_greedy_policy():
    env = GameState()
    env.reset()
    setup_playable_state(env)

    # Default state: Should probably Hatch or Play
    action = greedy_policy(env)
    assert action in [ACTION_HATCH, ACTION_PASS_TURN] or (action >= ACTION_PLAY_CARD_START and action <= ACTION_PLAY_CARD_START+9)

    # Force Trash
    env.game.pending_action = PendingAction.TRASH_CARD
    action = greedy_policy(env)
    assert action >= ACTION_TRASH_CARD_START and action <= ACTION_TRASH_CARD_START + 9
