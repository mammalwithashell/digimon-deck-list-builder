"""Validate that all transpiled P (Promo) card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

P_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "p"
)


def get_p_script_modules():
    """Yield (module_name, class_name) for each P script."""
    for fname in sorted(os.listdir(P_SCRIPTS_DIR)):
        if fname.startswith("p_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = "P" + module_name[1:]
            yield module_name, class_name


P_SCRIPTS = list(get_p_script_modules())


@pytest.mark.parametrize("module_name,class_name", P_SCRIPTS, ids=[m for m, _ in P_SCRIPTS])
def test_p_script_imports(module_name, class_name):
    """Each P script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.p.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", P_SCRIPTS, ids=[m for m, _ in P_SCRIPTS])
def test_p_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.p.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_p_script_count():
    """We should have 227 P script files."""
    count = len([f for f in os.listdir(P_SCRIPTS_DIR) if f.startswith("p_") and f.endswith(".py")])
    assert count == 227, f"Expected 227 P scripts, found {count}"


def test_p_cards_in_database():
    """cards.json should contain all 231 P cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    if isinstance(cards, dict):
        p_cards = [c for cid, c in cards.items() if cid.startswith("P-")]
    else:
        p_cards = [c for c in cards if c["card_id"].startswith("P-")]
    assert len(p_cards) == 231, f"Expected 231 P cards in database, found {len(p_cards)}"


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


class TestPEffectsExecute:
    """Test that P effect callbacks actually mutate game state."""

    def test_p_001_on_play_delete(self):
        """P-001 Agumon: on play, delete opponent Digimon with 3000 DP or less."""
        from digimon_gym.engine.data.scripts.p.p_001 import P_001
        p1, p2, game = make_game_context()

        card = make_card("P-001", "Agumon", dp=2000, level=3, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        # Opponent digimon with 3000 DP (should be deletable)
        opp_card = make_card("OPP-001", "OppDigimon", dp=3000, level=3, owner=p2)
        opp_perm = Permanent([opp_card])
        p2.battle_area.append(opp_perm)

        script = P_001()
        effects = script.get_card_effects(card)
        on_play_effect = effects[0]

        assert on_play_effect.is_on_play

        ctx = {"game": game, "player": p1, "permanent": perm}
        on_play_effect.on_process_callback(ctx)

        assert len(p2.battle_area) == 0
        assert len(p2.trash_cards) == 1

    def test_p_051_when_digivolving_draw(self):
        """P-051 MetalGarurumon: when digivolving, draw 2."""
        from digimon_gym.engine.data.scripts.p.p_051 import P_051
        p1, p2, game = make_game_context()

        card = make_card("P-051", "MetalGarurumon", dp=12000, level=6, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = P_051()
        effects = script.get_card_effects(card)
        digi_effect = effects[0]

        assert digi_effect.is_when_digivolving

        hand_before = len(p1.hand_cards)
        lib_before = len(p1.library_cards)
        ctx = {"game": game, "player": p1, "permanent": perm}
        digi_effect.on_process_callback(ctx)

        assert len(p1.hand_cards) == hand_before + 2
        assert len(p1.library_cards) == lib_before - 2

    def test_p_091_factory_keywords(self):
        """P-091 Saberdramon: has Raid and Retaliation factory effects."""
        from digimon_gym.engine.data.scripts.p.p_091 import P_091
        script = P_091()
        effects = script.get_card_effects(None)

        assert len(effects) == 3
        assert effects[0]._is_raid
        assert effects[1]._is_retaliation
        assert effects[2].is_inherited_effect
        assert effects[2].is_on_deletion

    def test_p_012_tamer_security_play(self):
        """P-012 Tai Kamiya: has security play effect."""
        from digimon_gym.engine.data.scripts.p.p_012 import P_012
        script = P_012()
        effects = script.get_card_effects(None)

        sec_effects = [e for e in effects if e.is_security_effect]
        assert len(sec_effects) == 1

    def test_p_024_option_draw(self):
        """P-024 Tai's Growing Up!: option draw 3."""
        from digimon_gym.engine.data.scripts.p.p_024 import P_024
        p1, p2, game = make_game_context()

        for i in range(5):
            p1.library_cards.append(make_card(f"LIB-{i}", f"Lib{i}", owner=p1))

        script = P_024()
        effects = script.get_card_effects(None)
        option_effect = effects[0]

        hand_before = len(p1.hand_cards)
        ctx = {"game": game, "player": p1, "permanent": None}
        option_effect.on_process_callback(ctx)

        assert len(p1.hand_cards) == hand_before + 3

    def test_p_031_blocker_and_recovery(self):
        """P-031 MagnaAngemon: has Blocker factory + recovery on play."""
        from digimon_gym.engine.data.scripts.p.p_031 import P_031
        script = P_031()
        effects = script.get_card_effects(None)

        blocker_effects = [e for e in effects if hasattr(e, '_is_blocker') and e._is_blocker]
        assert len(blocker_effects) == 1

    def test_p_total_effects_count(self):
        """Verify total effect count across all P scripts."""
        total_effects = 0
        for module_name, class_name in P_SCRIPTS:
            module_path = f"digimon_gym.engine.data.scripts.p.{module_name}"
            module = importlib.import_module(module_path)
            script_class = getattr(module, class_name)
            instance = script_class()
            effects = instance.get_card_effects(None)
            total_effects += len(effects)
        assert total_effects == 650, f"Expected 650 total effects, got {total_effects}"
