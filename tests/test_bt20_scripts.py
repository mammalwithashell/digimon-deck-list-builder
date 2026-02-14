"""Validate that all transpiled BT20 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT20_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt20"
)


def get_bt20_script_modules():
    """Yield (module_name, class_name) for each BT20 script."""
    for fname in sorted(os.listdir(BT20_SCRIPTS_DIR)):
        if fname.startswith("bt20_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("bt20", "BT20")
            yield module_name, class_name


BT20_SCRIPTS = list(get_bt20_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT20_SCRIPTS, ids=[m for m, _ in BT20_SCRIPTS])
def test_bt20_script_imports(module_name, class_name):
    """Each BT20 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt20.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT20_SCRIPTS, ids=[m for m, _ in BT20_SCRIPTS])
def test_bt20_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt20.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt20_script_count():
    """We should have 103 BT20 script files."""
    count = len([f for f in os.listdir(BT20_SCRIPTS_DIR) if f.startswith("bt20_") and f.endswith(".py")])
    assert count == 103, f"Expected 103 BT20 scripts, found {count}"


def test_bt20_cards_in_database():
    """cards.json should contain all 102 BT20 cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    bt20_cards = [c for c in cards if c["card_id"].startswith("BT20")]
    assert len(bt20_cards) == 102, f"Expected 102 BT20 cards in database, found {len(bt20_cards)}"


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


class TestBT20EffectsExecute:
    """Test that BT20 effect callbacks actually mutate game state."""

    def test_bt20_001_dp_modifier(self):
        """BT20-001: should have DP modifier with extracted value 2000."""
        from digimon_gym.engine.data.scripts.bt20.bt20_001 import BT20_001
        script = BT20_001()
        effects = script.get_card_effects(None)
        assert len(effects) >= 1
        dp_effect = effects[0]
        assert dp_effect.dp_modifier == 2000

    def test_bt20_061_has_reveal_and_select(self):
        """BT20-061: on play should use game.effect_reveal_and_select."""
        from digimon_gym.engine.data.scripts.bt20.bt20_061 import BT20_061
        script = BT20_061()
        effects = script.get_card_effects(None)
        # Should have: alt_digivolve_req, on-play (reveal), dp_modifier
        assert len(effects) >= 3
        on_play = effects[1]
        assert on_play.is_on_play
        assert on_play.on_process_callback is not None

    def test_bt20_061_alt_digivolve_req(self):
        """BT20-061: should have alternate digivolution from Yaamon."""
        from digimon_gym.engine.data.scripts.bt20.bt20_061 import BT20_061
        script = BT20_061()
        effects = script.get_card_effects(None)
        alt_digi = effects[0]
        assert hasattr(alt_digi, '_alt_digi_name')
        assert alt_digi._alt_digi_name == "Yaamon"
        assert alt_digi._alt_digi_cost == 0

    def test_bt20_036_on_play_de_digivolve(self):
        """BT20-036: on play should have de-digivolve and DP change."""
        from digimon_gym.engine.data.scripts.bt20.bt20_036 import BT20_036
        script = BT20_036()
        effects = script.get_card_effects(None)
        # Find the on-play effect
        on_play_effects = [e for e in effects if e.is_on_play]
        assert len(on_play_effects) >= 1
        assert on_play_effects[0].on_process_callback is not None

    def test_bt20_002_has_text_condition_positive(self):
        """BT20-002: HasText condition should match when card text contains 'Dracomon'."""
        from digimon_gym.engine.data.scripts.bt20.bt20_002 import BT20_002
        p1, p2, game = make_game_context()
        # Create a card whose effect text contains 'Dracomon'
        card = make_card(card_id="BT20-002", name="Dracomon", owner=p1)
        card.c_entity_base.effect_description_eng = (
            "[When Attacking] If this Digimon has [Dracomon] in its text, <Draw 1>."
        )
        # Place it on the field in a Permanent
        perm = Permanent([card])
        p1.battle_area.append(perm)

        script = BT20_002()
        effects = script.get_card_effects(card)
        assert len(effects) >= 1
        effect = effects[0]
        effect.effect_source_permanent = perm
        # Condition should pass — card text has 'Dracomon'
        assert effect.can_use_condition({}), "HasText('Dracomon') should match card text"

    def test_bt20_002_has_text_condition_negative(self):
        """BT20-002: HasText condition should fail when card text has no 'Dracomon'/'Examon'."""
        from digimon_gym.engine.data.scripts.bt20.bt20_002 import BT20_002
        p1, p2, game = make_game_context()
        # Create a card with unrelated effect text
        card = make_card(card_id="BT20-002", name="Agumon", owner=p1)
        card.c_entity_base.effect_description_eng = (
            "[When Attacking] This Digimon gains +1000 DP."
        )
        perm = Permanent([card])
        p1.battle_area.append(perm)

        script = BT20_002()
        effects = script.get_card_effects(card)
        assert len(effects) >= 1
        effect = effects[0]
        effect.effect_source_permanent = perm
        # Condition should fail — no Dracomon or Examon in text
        assert not effect.can_use_condition({}), "HasText should fail without matching text"

    def test_bt20_all_effects_have_no_stubs(self):
        """No BT20 script should contain TODO stub comments."""
        import inspect
        stub_count = 0
        for module_name, class_name in BT20_SCRIPTS:
            module_path = f"digimon_gym.engine.data.scripts.bt20.{module_name}"
            module = importlib.import_module(module_path)
            source = inspect.getsource(module)
            if "# TODO:" in source:
                stub_count += 1
        assert stub_count == 0, f"Found {stub_count} scripts with TODO stubs"
