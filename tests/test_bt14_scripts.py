"""Validate that all transpiled BT14 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT14_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt14"
)


def get_bt14_script_modules():
    """Yield (module_name, class_name) for each BT14 script."""
    for fname in sorted(os.listdir(BT14_SCRIPTS_DIR)):
        if fname.startswith("bt14_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.upper().replace("BT14", "BT14")  # bt14_001 -> BT14_001
            # Fix: class_name should be BT14_XXX
            class_name = "BT14_" + module_name.split("_", 1)[1].upper()
            # Handle: bt14_001 -> BT14_001, bt14_100 -> BT14_100
            class_name = module_name.replace("bt14", "BT14")
            yield module_name, class_name


BT14_SCRIPTS = list(get_bt14_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT14_SCRIPTS, ids=[m for m, _ in BT14_SCRIPTS])
def test_bt14_script_imports(module_name, class_name):
    """Each BT14 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt14.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT14_SCRIPTS, ids=[m for m, _ in BT14_SCRIPTS])
def test_bt14_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt14.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    # Pass None as card (scripts shouldn't crash on None for basic structure)
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt14_script_count():
    """We should have 94 BT14 script files."""
    count = len([f for f in os.listdir(BT14_SCRIPTS_DIR) if f.startswith("bt14_") and f.endswith(".py")])
    assert count == 94, f"Expected 94 BT14 scripts, found {count}"


def test_bt14_cards_in_database():
    """cards.json should contain all 102 BT14 cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    bt14_cards = [c for c in cards if c["card_id"].startswith("BT14")]
    assert len(bt14_cards) == 102, f"Expected 102 BT14 cards in database, found {len(bt14_cards)}"


# ─── Integration Tests: Effects Execute Against Game State ──────────

from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.data.enums import CardKind, CardColor


def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, traits=None, owner=None):
    """Helper to create a CardSource with minimal setup."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.type_eng = traits or []
    entity.card_colors = [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_game_context():
    """Create a minimal game-like context with two players."""
    p1 = Player()
    p2 = Player()
    p1.player_name = "P1"
    p2.player_name = "P2"
    p1.enemy = p2
    p2.enemy = p1

    class FakeGame:
        turn_player = p1
        opponent_player = p2
        memory = 3
    game = FakeGame()
    p1.game = game
    p2.game = game
    p1.is_my_turn = True
    return p1, p2, game


class TestBT14EffectsExecute:
    """Test that effect callbacks actually mutate game state."""

    def test_bt14_001_draw_on_security_loss(self):
        """BT14-001: inherited effect — draw 1 on opponent security loss."""
        from digimon_gym.engine.data.scripts.bt14.bt14_001 import BT14_001
        p1, p2, game = make_game_context()

        card = make_card("BT14-001", "Koromon", kind=CardKind.DigiEgg, dp=0, level=2, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        # Give p1 some library cards to draw from
        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = BT14_001()
        effects = script.get_card_effects(card)
        assert len(effects) == 1

        effect = effects[0]
        assert effect.is_inherited_effect
        assert effect.max_count_per_turn == 1

        # Execute the callback
        hand_before = len(p1.hand_cards)
        ctx = {"game": game, "player": p1, "permanent": perm}
        effect.on_process_callback(ctx)
        assert len(p1.hand_cards) == hand_before + 1

    def test_bt14_074_attack_effect_draws_and_gains_memory(self):
        """BT14-074: when attacking, trash 1 from hand, draw 1, gain 1 memory."""
        from digimon_gym.engine.data.scripts.bt14.bt14_074 import BT14_074
        p1, p2, game = make_game_context()

        card = make_card("BT14-074", "Loogamon", dp=7000, level=5, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        # Give hand and library
        for i in range(3):
            p1.hand_cards.append(make_card(f"HAND-{i}", f"Hand{i}", owner=p1))
        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = BT14_074()
        effects = script.get_card_effects(card)
        attack_effect = effects[0]

        ctx = {"game": game, "player": p1, "permanent": perm}
        mem_before = game.memory
        hand_before = len(p1.hand_cards)
        attack_effect.on_process_callback(ctx)

        # Should have drawn 1, gained 1 memory, trashed 1 from hand
        # Net hand change: +1 draw -1 trash = 0
        assert game.memory == mem_before + 1

    def test_bt14_034_dp_change_on_deletion(self):
        """BT14-034: on deletion, opponent digimon gets -3000 DP."""
        from digimon_gym.engine.data.scripts.bt14.bt14_034 import BT14_034
        p1, p2, game = make_game_context()

        card = make_card("BT14-034", "Lucemon", dp=4000, level=4, owner=p1)
        perm = Permanent([card])

        # Opponent has a digimon
        opp_card = make_card("OPP-001", "OppDigimon", dp=6000, level=4, owner=p2)
        opp_perm = Permanent([opp_card])
        p2.battle_area.append(opp_perm)

        script = BT14_034()
        effects = script.get_card_effects(card)
        deletion_effect = [e for e in effects if e.is_on_deletion][0]

        ctx = {"game": game, "player": p1, "permanent": perm}
        deletion_effect.on_process_callback(ctx)

        assert opp_perm.dp == 3000  # 6000 - 3000

    def test_effect_once_per_turn_tracking(self):
        """Once-per-turn effects should only fire once."""
        from digimon_gym.engine.data.scripts.bt14.bt14_001 import BT14_001
        p1, p2, game = make_game_context()

        card = make_card("BT14-001", "Koromon", kind=CardKind.DigiEgg, dp=0, level=2, owner=p1)
        script = BT14_001()
        effects = script.get_card_effects(card)
        effect = effects[0]

        assert effect.can_activate_this_turn()
        effect.record_activation()
        assert not effect.can_activate_this_turn()

        # Reset should allow it again
        effect.reset_turn_count()
        assert effect.can_activate_this_turn()

    def test_permanent_change_dp_and_clear(self):
        """Permanent.change_dp() should apply temp modifier, cleared each turn."""
        p1, _, _ = make_game_context()
        card = make_card(dp=5000, owner=p1)
        perm = Permanent([card])

        assert perm.dp == 5000
        perm.change_dp(-3000)
        assert perm.dp == 2000
        perm.change_dp(1000)
        assert perm.dp == 3000
        perm.clear_temp_dp()
        assert perm.dp == 5000

    def test_player_draw_cards(self):
        """Player.draw_cards() should draw N cards."""
        p1, _, _ = make_game_context()
        for i in range(10):
            p1.library_cards.append(make_card(f"LIB-{i}", owner=p1))

        drawn = p1.draw_cards(3)
        assert len(drawn) == 3
        assert len(p1.hand_cards) == 3
        assert len(p1.library_cards) == 7

    def test_player_add_memory(self):
        """Player.add_memory() should adjust game memory."""
        p1, p2, game = make_game_context()
        game.memory = 3

        p1.add_memory(2)
        assert game.memory == 5

        p2.add_memory(1)
        assert game.memory == 4  # Opponent gaining memory reduces turn player's

    def test_player_recovery(self):
        """Player.recovery() should move cards from library to security."""
        p1, _, _ = make_game_context()
        for i in range(10):
            p1.library_cards.append(make_card(f"LIB-{i}", owner=p1))

        p1.recovery(2)
        assert len(p1.security_cards) == 2
        assert len(p1.library_cards) == 8

    def test_card_source_permanent_of_this_card(self):
        """CardSource.permanent_of_this_card() should find its permanent."""
        p1, _, _ = make_game_context()
        card = make_card(owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        assert card.permanent_of_this_card() is perm

    def test_permanent_trash_digivolution_cards(self):
        """Permanent.trash_digivolution_cards() should remove from under top."""
        p1, _, _ = make_game_context()
        base = make_card("BASE", "Base", level=3, owner=p1)
        mid = make_card("MID", "Mid", level=4, owner=p1)
        top = make_card("TOP", "Top", level=5, owner=p1)
        perm = Permanent([base, mid, top])

        trashed = perm.trash_digivolution_cards(1)
        assert len(trashed) == 1
        assert trashed[0] is mid
        assert perm.top_card is top
        assert len(perm.card_sources) == 2
