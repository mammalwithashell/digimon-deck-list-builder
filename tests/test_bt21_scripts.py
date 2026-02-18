"""Validate that all transpiled BT21 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT21_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt21"
)


def get_bt21_script_modules():
    """Yield (module_name, class_name) for each BT21 script."""
    for fname in sorted(os.listdir(BT21_SCRIPTS_DIR)):
        if fname.startswith("bt21_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("bt21", "BT21")
            yield module_name, class_name


BT21_SCRIPTS = list(get_bt21_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT21_SCRIPTS, ids=[m for m, _ in BT21_SCRIPTS])
def test_bt21_script_imports(module_name, class_name):
    """Each BT21 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt21.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT21_SCRIPTS, ids=[m for m, _ in BT21_SCRIPTS])
def test_bt21_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt21.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt21_script_count():
    """We should have 103 BT21 script files."""
    count = len([f for f in os.listdir(BT21_SCRIPTS_DIR) if f.startswith("bt21_") and f.endswith(".py")])
    assert count == 103, f"Expected 103 BT21 scripts, found {count}"


def test_bt21_cards_in_database():
    """cards.json should contain all 102 BT21 cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    if isinstance(cards, dict):
        bt21_cards = [c for cid, c in cards.items() if cid.startswith("BT21")]
    else:
        bt21_cards = [c for c in cards if c["card_id"].startswith("BT21")]
    assert len(bt21_cards) == 102, f"Expected 102 BT21 cards in database, found {len(bt21_cards)}"


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
            for c in list(player.hand_cards):
                if filter_fn(c):
                    callback(c)
                    return

        def effect_select_opponent_permanent(self, player, callback, filter_fn=None, is_optional=False):
            enemy = player.enemy if player else None
            if not enemy:
                return
            for p in list(enemy.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        def effect_select_own_permanent(self, player, callback, filter_fn=None, is_optional=False):
            if not player:
                return
            for p in list(player.battle_area):
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        def effect_reveal_and_select(self, player, count, filter_fn, callback, is_optional=False):
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


class TestBT21EffectsExecute:
    """Test that BT21 effect callbacks actually mutate game state."""

    def test_bt21_034_on_tapped_draw(self):
        """BT21-034: when this Digimon suspends, draw 1 (once per turn)."""
        from digimon_gym.engine.data.scripts.bt21.bt21_034 import BT21_034
        p1, p2, game = make_game_context()

        card = make_card("BT21-034", "Digimon", dp=5000, level=4, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = BT21_034()
        effects = script.get_card_effects(card)

        # Should have alt_digivolve_req, draw-on-suspend, jamming
        assert any(hasattr(e, "_is_jamming") and e._is_jamming for e in effects)

        # The draw effect is identified by its description containing "suspends"
        draw_effects = [e for e in effects if "suspends" in (e.effect_description or "")]
        assert len(draw_effects) >= 1

        hand_before = len(p1.hand_cards)
        lib_before = len(p1.library_cards)
        ctx = {"game": game, "player": p1, "permanent": perm}
        draw_effects[0].on_process_callback(ctx)
        assert len(p1.hand_cards) == hand_before + 1
        assert len(p1.library_cards) == lib_before - 1

    def test_bt21_036_blocker_and_armor_purge(self):
        """BT21-036: should have blocker and armor_purge factory effects."""
        from digimon_gym.engine.data.scripts.bt21.bt21_036 import BT21_036
        script = BT21_036()
        effects = script.get_card_effects(None)
        assert any(hasattr(e, "_is_blocker") and e._is_blocker for e in effects)
        assert any(hasattr(e, "_is_armor_purge") and e._is_armor_purge for e in effects)

    def test_bt21_063_save_and_dp_modifier(self):
        """BT21-063: should have save and dp_modifier factory effects."""
        from digimon_gym.engine.data.scripts.bt21.bt21_063 import BT21_063
        script = BT21_063()
        effects = script.get_card_effects(None)
        assert any(hasattr(e, "_is_save") and e._is_save for e in effects)
        assert any(hasattr(e, "_dp_modifier") and e._dp_modifier != 0 for e in effects)

    def test_bt21_069_retaliation(self):
        """BT21-069: should have retaliation factory effect."""
        from digimon_gym.engine.data.scripts.bt21.bt21_069 import BT21_069
        script = BT21_069()
        effects = script.get_card_effects(None)
        assert any(hasattr(e, "_is_retaliation") and e._is_retaliation for e in effects)

    def test_bt21_033_on_play_add_to_hand(self):
        """BT21-033: on play/digivolve, reveal top 3 and add matching to hand."""
        from digimon_gym.engine.data.scripts.bt21.bt21_033 import BT21_033
        p1, p2, game = make_game_context()

        card = make_card("BT21-033", "Digimon", dp=5000, level=4, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = BT21_033()
        effects = script.get_card_effects(card)

        # Find on-play effect
        on_play = [e for e in effects if hasattr(e, "is_on_play") and e.is_on_play]
        assert len(on_play) >= 1

        assert any(hasattr(e, "_is_jamming") and e._is_jamming for e in effects)

    def test_bt21_025_progress(self):
        """BT21-025: should have progress factory effect."""
        from digimon_gym.engine.data.scripts.bt21.bt21_025 import BT21_025
        script = BT21_025()
        effects = script.get_card_effects(None)
        assert any(hasattr(e, "_is_progress") and e._is_progress for e in effects)

    def test_bt21_087_set_memory_3(self):
        """BT21-087 Tamer: should have set_memory_3 (by name) and security_play."""
        from digimon_gym.engine.data.scripts.bt21.bt21_087 import BT21_087
        script = BT21_087()
        effects = script.get_card_effects(None)
        # set_memory_3 factory emits an effect with description "Set memory to 3"
        assert any("Set memory to 3" in (e.effect_description or "") or "Set memory to 3" in (e.effect_name or "") for e in effects)
        assert any(hasattr(e, "is_security_effect") and e.is_security_effect for e in effects)

    def test_bt21_total_effects_count(self):
        """Verify total effect count across all BT21 scripts."""
        total_effects = 0
        for module_name, class_name in BT21_SCRIPTS:
            module_path = f"digimon_gym.engine.data.scripts.bt21.{module_name}"
            module = importlib.import_module(module_path)
            script_class = getattr(module, class_name)
            instance = script_class()
            effects = instance.get_card_effects(None)
            total_effects += len(effects)
        assert total_effects == 399, f"Expected 399 total effects, got {total_effects}"
