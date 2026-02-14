"""Validate that all transpiled BT24 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT24_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt24"
)


def get_bt24_script_modules():
    """Yield (module_name, class_name) for each BT24 script."""
    for fname in sorted(os.listdir(BT24_SCRIPTS_DIR)):
        if fname.startswith("bt24_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("bt24", "BT24")
            yield module_name, class_name


BT24_SCRIPTS = list(get_bt24_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT24_SCRIPTS, ids=[m for m, _ in BT24_SCRIPTS])
def test_bt24_script_imports(module_name, class_name):
    """Each BT24 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt24.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT24_SCRIPTS, ids=[m for m, _ in BT24_SCRIPTS])
def test_bt24_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt24.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt24_script_count():
    """We should have 102 BT24 script files."""
    count = len([f for f in os.listdir(BT24_SCRIPTS_DIR) if f.startswith("bt24_") and f.endswith(".py")])
    assert count == 102, f"Expected 102 BT24 scripts, found {count}"


def test_bt24_cards_in_database():
    """cards.json should contain all 102 BT24 cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    bt24_cards = [c for c in cards if c["card_id"].startswith("BT24")]
    assert len(bt24_cards) == 102, f"Expected 102 BT24 cards in database, found {len(bt24_cards)}"


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

        def effect_link_to_permanent(self, player, card, is_optional=False):
            pass

        def effect_digivolve_from_hand(self, player, perm, filter_fn, **kwargs):
            pass

    game = FakeGame()
    p1.game = game
    p2.game = game
    p1.is_my_turn = True
    return p1, p2, game


class TestBT24EffectsExecute:
    """Test that BT24 effect callbacks actually mutate game state."""

    def test_bt24_008_on_play_draw_and_trash(self):
        """BT24-008 Elizamon: on play, draw 2 and trash 1 from hand."""
        from digimon_gym.engine.data.scripts.bt24.bt24_008 import BT24_008
        p1, p2, game = make_game_context()

        card = make_card("BT24-008", "Elizamon", dp=3000, level=3, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))
        p1.hand_cards.append(make_card("HAND-0", "HandCard", owner=p1))

        script = BT24_008()
        effects = script.get_card_effects(card)
        on_play_effect = effects[0]

        assert on_play_effect.is_on_play
        assert on_play_effect.is_optional

        hand_before = len(p1.hand_cards)
        lib_before = len(p1.library_cards)
        ctx = {"game": game, "player": p1, "permanent": perm}
        on_play_effect.on_process_callback(ctx)

        # Drew 2 from library, trashed 1 from hand -> net hand +1
        assert len(p1.library_cards) == lib_before - 2
        assert len(p1.hand_cards) == hand_before + 1

    def test_bt24_008_inherited_memory_gain(self):
        """BT24-008 Elizamon: inherited once-per-turn memory gain."""
        from digimon_gym.engine.data.scripts.bt24.bt24_008 import BT24_008
        p1, p2, game = make_game_context()

        card = make_card("BT24-008", "Elizamon", dp=3000, level=3, owner=p1)
        card.owner = p1
        card.owner.is_my_turn = True

        script = BT24_008()
        effects = script.get_card_effects(card)
        inherited = effects[1]

        assert inherited.is_inherited_effect
        assert inherited.max_count_per_turn == 1

        mem_before = game.memory
        ctx = {"game": game, "player": p1, "permanent": None}
        inherited.on_process_callback(ctx)
        assert game.memory == mem_before + 1

    def test_bt24_010_blocker_and_on_deletion(self):
        """BT24-010 Greymon: has blocker factory + on-deletion de-digivolve."""
        from digimon_gym.engine.data.scripts.bt24.bt24_010 import BT24_010
        p1, p2, game = make_game_context()

        card = make_card("BT24-010", "Greymon", dp=6000, level=4, owner=p1)
        perm = Permanent([card])

        # Opponent digimon with digivolution stack
        opp_base = make_card("OPP-BASE", "OppBase", level=3, dp=3000, owner=p2)
        opp_top = make_card("OPP-TOP", "OppTop", level=4, dp=6000, owner=p2)
        opp_perm = Permanent([opp_base, opp_top])
        p2.battle_area.append(opp_perm)

        script = BT24_010()
        effects = script.get_card_effects(card)

        # Should have: alt_digivolve_req, blocker, on-deletion (de-digivolve), raid
        assert effects[1]._is_blocker
        assert effects[2].is_on_deletion
        assert effects[3]._is_raid

        # Test de-digivolve callback
        ctx = {"game": game, "player": p1, "permanent": perm}
        effects[2].on_process_callback(ctx)

        assert len(opp_perm.card_sources) == 1
        assert opp_perm.top_card is opp_base
        assert len(p2.trash_cards) == 1

    def test_bt24_014_dp_change_and_delete(self):
        """BT24-014: when digivolving, -5000 DP and delete."""
        from digimon_gym.engine.data.scripts.bt24.bt24_014 import BT24_014
        p1, p2, game = make_game_context()

        card = make_card("BT24-014", "GeoGreymon", dp=7000, level=4, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        opp_card = make_card("OPP-001", "OppDigimon", dp=6000, level=4, owner=p2)
        opp_perm = Permanent([opp_card])
        p2.battle_area.append(opp_perm)

        script = BT24_014()
        effects = script.get_card_effects(card)
        digi_effect = effects[2]  # [0]=alt_digi, [1]=security_attack, [2]=when_digivolving

        assert digi_effect.is_when_digivolving

        ctx = {"game": game, "player": p1, "permanent": perm}
        digi_effect.on_process_callback(ctx)

        # The DP change applies -5000 to weakest opp digimon
        assert opp_perm.dp == 1000  # 6000 - 5000

    def test_bt24_057_de_digivolve_on_deletion(self):
        """BT24-057: on deletion, de-digivolve 1 opponent's digimon."""
        from digimon_gym.engine.data.scripts.bt24.bt24_057 import BT24_057
        p1, p2, game = make_game_context()

        card = make_card("BT24-057", "Digimon", dp=5000, level=4, owner=p1)

        opp_base = make_card("OPP-BASE", "OppBase", level=3, dp=3000, owner=p2)
        opp_top = make_card("OPP-TOP", "OppTop", level=4, dp=6000, owner=p2)
        opp_perm = Permanent([opp_base, opp_top])
        p2.battle_area.append(opp_perm)

        script = BT24_057()
        effects = script.get_card_effects(card)
        # effect3 is the de-digivolve on deletion
        dedigivolve = [e for e in effects if e.is_on_deletion and e.on_process_callback is not None][0]

        ctx = {"game": game, "player": p1, "permanent": None}
        dedigivolve.on_process_callback(ctx)

        assert len(opp_perm.card_sources) == 1
        assert len(p2.trash_cards) == 1

    def test_bt24_102_tamer_memory_and_draw(self):
        """BT24-102 Homeros: start of main phase, gain 1 memory and draw 1."""
        from digimon_gym.engine.data.scripts.bt24.bt24_102 import BT24_102
        p1, p2, game = make_game_context()

        card = make_card("BT24-102", "Homeros", kind=CardKind.Tamer, dp=0, level=0, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        # Opponent needs a digimon for the suspend target
        opp_card = make_card("OPP-001", "OppDigimon", dp=5000, owner=p2)
        opp_perm = Permanent([opp_card])
        p2.battle_area.append(opp_perm)

        script = BT24_102()
        effects = script.get_card_effects(card)
        main_phase_effect = effects[0]

        mem_before = game.memory
        hand_before = len(p1.hand_cards)
        ctx = {"game": game, "player": p1, "permanent": perm}
        main_phase_effect.on_process_callback(ctx)

        assert game.memory == mem_before + 1
        assert len(p1.hand_cards) == hand_before + 1

    def test_bt24_102_security_play_flag(self):
        """BT24-102 Homeros: should have security play effect."""
        from digimon_gym.engine.data.scripts.bt24.bt24_102 import BT24_102
        script = BT24_102()
        effects = script.get_card_effects(None)
        sec_effects = [e for e in effects if e.is_security_effect]
        assert len(sec_effects) == 1

    def test_bt24_014_security_attack_plus(self):
        """BT24-014: should have Security Attack +1 factory effect."""
        from digimon_gym.engine.data.scripts.bt24.bt24_014 import BT24_014
        script = BT24_014()
        effects = script.get_card_effects(None)
        sa_effects = [e for e in effects if hasattr(e, '_security_attack_modifier') and e._security_attack_modifier == 1]
        assert len(sa_effects) == 1

    def test_bt24_effect_once_per_turn_tracking(self):
        """Once-per-turn effects should track activations correctly."""
        from digimon_gym.engine.data.scripts.bt24.bt24_008 import BT24_008
        p1, p2, game = make_game_context()

        card = make_card("BT24-008", "Elizamon", dp=3000, level=3, owner=p1)
        card.owner = p1
        card.owner.is_my_turn = True

        script = BT24_008()
        effects = script.get_card_effects(card)
        inherited = effects[1]

        assert inherited.can_activate_this_turn()
        inherited.record_activation()
        assert not inherited.can_activate_this_turn()

        inherited.reset_turn_count()
        assert inherited.can_activate_this_turn()

    def test_bt24_total_effects_count(self):
        """Verify total effect count across all BT24 scripts."""
        total_effects = 0
        for module_name, class_name in BT24_SCRIPTS:
            module_path = f"digimon_gym.engine.data.scripts.bt24.{module_name}"
            module = importlib.import_module(module_path)
            script_class = getattr(module, class_name)
            instance = script_class()
            effects = instance.get_card_effects(None)
            total_effects += len(effects)
        assert total_effects == 401, f"Expected 401 total effects, got {total_effects}"
