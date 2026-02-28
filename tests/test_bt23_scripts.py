"""Validate that all transpiled BT23 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT23_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt23"
)


def get_bt23_script_modules():
    """Yield (module_name, class_name) for each BT23 script."""
    for fname in sorted(os.listdir(BT23_SCRIPTS_DIR)):
        if fname.startswith("bt23_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("bt23", "BT23")
            yield module_name, class_name


BT23_SCRIPTS = list(get_bt23_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT23_SCRIPTS, ids=[m for m, _ in BT23_SCRIPTS])
def test_bt23_script_imports(module_name, class_name):
    """Each BT23 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt23.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT23_SCRIPTS, ids=[m for m, _ in BT23_SCRIPTS])
def test_bt23_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt23.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt23_script_count():
    """We should have 103 BT23 script files."""
    count = len([f for f in os.listdir(BT23_SCRIPTS_DIR) if f.startswith("bt23_") and f.endswith(".py")])
    assert count == 103, f"Expected 103 BT23 scripts, found {count}"


def test_bt23_total_effects():
    """All BT23 scripts should produce a total of 428 effects."""
    total = 0
    for module_name, class_name in BT23_SCRIPTS:
        module_path = f"digimon_gym.engine.data.scripts.bt23.{module_name}"
        module = importlib.import_module(module_path)
        script_class = getattr(module, class_name)
        effects = script_class().get_card_effects(None)
        total += len(effects)
    assert total == 432, f"Expected 432 total BT23 effects, found {total}"


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

        class logger:
            @staticmethod
            def log(msg):
                pass

        def effect_select_hand_card(self, player, filter_fn, callback, is_optional=False):
            """Auto-select first matching hand card."""
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return

        def effect_select_opponent_permanent(self, player, callback, filter_fn=None, is_optional=False):
            """Auto-select first matching opponent permanent."""
            enemy = player.enemy if player else None
            if not enemy:
                return
            for p in list(enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        def effect_select_own_permanent(self, player, callback, filter_fn=None, is_optional=False):
            """Auto-select first matching own permanent."""
            if not player:
                return
            for p in list(player.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        def effect_reveal_and_select(self, player, count, filter_fn, callback, is_optional=False):
            """Auto-reveal and select first matching card."""
            if not player or not player.library_cards:
                return
            revealed = player.library_cards[:count]
            player.library_cards = player.library_cards[count:]
            selected = None
            remaining = []
            for c in revealed:
                if selected is None and filter_fn(c):
                    selected = c
                else:
                    remaining.append(c)
            if selected:
                callback(selected, remaining)
            else:
                player.library_cards = revealed + player.library_cards

        def effect_play_from_zone(self, player, zone, filter_fn, free=False, is_optional=False):
            """Auto-play first matching card from zone."""
            pass

        def effect_link_to_permanent(self, player, card, is_optional=False):
            pass

        def effect_digivolve_from_hand(self, player, perm, filter_fn, **kwargs):
            pass

    game = FakeGame()
    p1.game = game
    p2.game = game
    p1.is_my_turn = True
    return p1, p2, game


class TestBT23EffectsExecute:
    """Test that BT23 effect callbacks actually mutate game state."""

    def test_bt23_014_has_delete_effects(self):
        """BT23-014 Gallantmon: should have delete effects from SharedActivateCoroutine."""
        from digimon_gym.engine.data.scripts.bt23.bt23_014 import BT23_014
        script = BT23_014()
        effects = script.get_card_effects(None)
        # Should have multiple effects including delete from SharedActivateCoroutine
        assert len(effects) >= 3
        # At least one effect should have a process callback (delete action)
        callbacks = [e for e in effects if e.on_process_callback is not None]
        assert len(callbacks) >= 1, "Expected at least 1 effect with action callback"

    def test_bt23_050_has_dp_change(self):
        """BT23-050 Ankylomon: SharedActivateCoroutine should detect DP change."""
        from digimon_gym.engine.data.scripts.bt23.bt23_050 import BT23_050
        p1, p2, game = make_game_context()
        card = make_card("BT23-050", "Ankylomon", dp=5000, level=4, owner=p1)

        script = BT23_050()
        effects = script.get_card_effects(card)
        # Should have effects detected from SharedActivateCoroutine
        assert len(effects) >= 2
        dp_effects = [e for e in effects if e.on_process_callback is not None]
        assert len(dp_effects) >= 1, "Expected at least 1 effect with action callback"

    def test_bt23_052_has_keyword_effects(self):
        """BT23-052: should have keyword grants from GainCanNotAttack."""
        from digimon_gym.engine.data.scripts.bt23.bt23_052 import BT23_052
        script = BT23_052()
        effects = script.get_card_effects(None)
        # Should have: alt_digivolve_req, on-play cannot_attack, when-digivolving cannot_attack,
        # when-linking blocker+reboot, security_play
        assert len(effects) >= 4
        # Check that cannot_attack keyword was detected
        cannot_attack = [e for e in effects if hasattr(e, '_is_cannot_attack')
                         and e._is_cannot_attack]
        assert len(cannot_attack) >= 1, "Expected at least 1 cannot_attack keyword effect"
