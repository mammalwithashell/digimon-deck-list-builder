"""Tests for DNA Digivolution.

Covers:
  A. DnaCost / DnaRequirement data model (3 tests)
  B. xros_req text parser (6 tests)
  C. DNA validation functions (10 tests)
  D. DNA action mask integration (5 tests)
  E. DNA digivolve execution via decode_action (6 tests)
  F. Player.dna_digivolve stacking behavior (4 tests)
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import Game, ACTION_SPACE_SIZE, FIELD_SLOTS
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming,
)
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.evo_cost import EvoCost, DnaCost, DnaRequirement
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.validation.digivolve_validator import (
    can_dna_digivolve, has_valid_dna_targets,
    get_valid_dna_first_targets, get_valid_dna_second_targets,
    get_dna_stacking_order,
)
from digimon_gym.engine.data.card_database import parse_xros_req


# ─── Helpers ─────────────────────────────────────────────────────────

def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None,
              evo_costs=None, dna_costs=None):
    """Create a CardSource with given attributes."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    if evo_costs is not None:
        entity.evo_costs = evo_costs
    if dna_costs is not None:
        entity.dna_costs = dna_costs
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_dna_card(card_id="DNA-001", name="DnaMon", dp=10000, level=5,
                  play_cost=8, colors=None, owner=None,
                  req1_color=CardColor.Blue, req1_level=4, req1_name="",
                  req2_color=CardColor.Green, req2_level=4, req2_name="",
                  memory_cost=0, evo_costs=None):
    """Create a CardSource with DNA digivolution requirements."""
    dna_cost = DnaCost(
        requirement1=DnaRequirement(
            level=req1_level,
            card_color=req1_color,
            name_contains=req1_name,
        ),
        requirement2=DnaRequirement(
            level=req2_level,
            card_color=req2_color,
            name_contains=req2_name,
        ),
        memory_cost=memory_cost,
    )
    return make_card(
        card_id=card_id, name=name, dp=dp, level=level,
        play_cost=play_cost, colors=colors or [CardColor.Blue, CardColor.Green],
        owner=owner, evo_costs=evo_costs or [], dna_costs=[dna_cost],
    )


def setup_game_at_phase(phase: GamePhase, memory: int = 5) -> Game:
    """Create a Game positioned at the given phase."""
    game = Game()
    game.current_phase = phase
    game.memory = memory
    game.turn_count = 2
    game.turn_player = game.player1
    game.opponent_player = game.player2
    game.player1.is_my_turn = True
    # Give players library cards for draw operations
    for _ in range(10):
        game.player1.library_cards.append(
            make_card(card_id="LIB-001", name="LibCard", owner=game.player1)
        )
        game.player2.library_cards.append(
            make_card(card_id="LIB-002", name="LibCard", owner=game.player2)
        )
    return game


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test to ensure isolation."""
    CardRegistry.reset()
    CardRegistry.initialize_from_list([
        "TEST-001", "TEST-002", "TEST-003", "DNA-001", "DNA-002",
        "BLUE-001", "GREEN-001", "BASE-001", "BASE-002",
        "LIB-001", "LIB-002", "BT16-025", "EX6-062",
    ])
    yield
    CardRegistry.reset()


# ═══════════════════════════════════════════════════════════════════════
# A. DnaCost / DnaRequirement Data Model (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDnaDataModel:
    def test_dna_requirement_defaults(self):
        """DnaRequirement defaults to no color and no name constraint."""
        req = DnaRequirement(level=4)
        assert req.level == 4
        assert req.card_color is None
        assert req.name_contains == ""

    def test_dna_cost_fields(self):
        """DnaCost holds two requirements and a memory cost."""
        req1 = DnaRequirement(level=4, card_color=CardColor.Blue)
        req2 = DnaRequirement(level=4, card_color=CardColor.Green)
        cost = DnaCost(requirement1=req1, requirement2=req2, memory_cost=0)
        assert cost.requirement1.card_color == CardColor.Blue
        assert cost.requirement2.card_color == CardColor.Green
        assert cost.memory_cost == 0

    def test_entity_base_has_dna_costs(self):
        """CEntity_Base initializes with empty dna_costs list."""
        entity = CEntity_Base()
        assert entity.dna_costs == []


# ═══════════════════════════════════════════════════════════════════════
# B. xros_req Text Parser (6 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestXrosReqParser:
    def test_parse_simple_color_level(self):
        """Parse '[DNA Digivolve] Blue Lv.4 + Green Lv.4: Cost 0'."""
        result = parse_xros_req("[DNA Digivolve] Blue Lv.4 + Green Lv.4: Cost 0")
        assert len(result) == 1
        dna = result[0]
        assert dna.requirement1.card_color == CardColor.Blue
        assert dna.requirement1.level == 4
        assert dna.requirement2.card_color == CardColor.Green
        assert dna.requirement2.level == 4
        assert dna.memory_cost == 0

    def test_parse_name_constraint(self):
        """Parse DNA with name-based requirements."""
        text = (
            "[DNA Digivolve] Lv.6 w/[Greymon] in name "
            "+ Lv.6 w/[Garurumon] in name : Cost 0"
        )
        result = parse_xros_req(text)
        assert len(result) == 1
        dna = result[0]
        assert dna.requirement1.level == 6
        assert dna.requirement1.name_contains == "Greymon"
        assert dna.requirement1.card_color is None
        assert dna.requirement2.level == 6
        assert dna.requirement2.name_contains == "Garurumon"
        assert dna.memory_cost == 0

    def test_parse_with_cost(self):
        """Parse DNA with non-zero cost."""
        result = parse_xros_req("[DNA Digivolve] Red Lv.5 + Blue Lv.5: Cost 3")
        assert len(result) == 1
        assert result[0].memory_cost == 3

    def test_parse_empty_string(self):
        """Empty string returns no results."""
        assert parse_xros_req("") == []

    def test_parse_non_dna_xros_req(self):
        """Non-DNA xros_req text returns no results."""
        result = parse_xros_req("[Digivolve] [Omnimon]: Cost 3")
        assert result == []

    def test_parse_mixed_dna_and_other(self):
        """Parse text containing both DNA and regular digivolve entries."""
        text = (
            "[Digivolve] Lv.6 w/[CS] trait: Cost 5 \r\n"
            "[DNA Digivolve] Lv.6 w/[Greymon] in name "
            "+ Lv.6 w/[Garurumon] in name : Cost 0\r\n"
            "Stack the 2 specified Digimon and digivolve unsuspended."
        )
        result = parse_xros_req(text)
        assert len(result) == 1
        assert result[0].requirement1.name_contains == "Greymon"
        assert result[0].requirement2.name_contains == "Garurumon"


# ═══════════════════════════════════════════════════════════════════════
# C. DNA Validation Functions (10 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDnaValidation:
    def test_basic_dna_valid(self):
        """Blue Lv4 + Green Lv4 satisfies DNA requirement."""
        dna_card = make_dna_card()
        blue_perm = Permanent([make_card(name="BlueMon", level=4, colors=[CardColor.Blue])])
        green_perm = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])

        assert can_dna_digivolve(dna_card, blue_perm, green_perm) is True

    def test_dna_reversed_order_valid(self):
        """Order of permanents doesn't matter for validation."""
        dna_card = make_dna_card()
        blue_perm = Permanent([make_card(name="BlueMon", level=4, colors=[CardColor.Blue])])
        green_perm = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])

        assert can_dna_digivolve(dna_card, green_perm, blue_perm) is True

    def test_dna_wrong_colors(self):
        """Red + Yellow doesn't match Blue + Green requirement."""
        dna_card = make_dna_card()
        red_perm = Permanent([make_card(name="RedMon", level=4, colors=[CardColor.Red])])
        yellow_perm = Permanent([make_card(name="YellowMon", level=4, colors=[CardColor.Yellow])])

        assert can_dna_digivolve(dna_card, red_perm, yellow_perm) is False

    def test_dna_wrong_level(self):
        """Blue Lv3 + Green Lv4 doesn't match Lv4 + Lv4 requirement."""
        dna_card = make_dna_card()
        blue_perm = Permanent([make_card(name="BlueLv3", level=3, colors=[CardColor.Blue])])
        green_perm = Permanent([make_card(name="GreenLv4", level=4, colors=[CardColor.Green])])

        assert can_dna_digivolve(dna_card, blue_perm, green_perm) is False

    def test_dna_same_permanent_invalid(self):
        """Cannot DNA digivolve with the same permanent twice."""
        dna_card = make_dna_card()
        perm = Permanent([make_card(name="DualMon", level=4,
                                     colors=[CardColor.Blue, CardColor.Green])])

        assert can_dna_digivolve(dna_card, perm, perm) is False

    def test_dna_name_constraint(self):
        """DNA with name requirement matches correctly."""
        dna_card = make_dna_card(
            req1_color=None, req1_level=6, req1_name="Greymon",
            req2_color=None, req2_level=6, req2_name="Garurumon",
        )
        greymon = Permanent([make_card(name="MetalGreymon", level=6, colors=[CardColor.Red])])
        garurumon = Permanent([make_card(name="WereGarurumon", level=6, colors=[CardColor.Blue])])

        assert can_dna_digivolve(dna_card, greymon, garurumon) is True

    def test_dna_name_constraint_mismatch(self):
        """DNA with name requirement rejects wrong names."""
        dna_card = make_dna_card(
            req1_color=None, req1_level=6, req1_name="Greymon",
            req2_color=None, req2_level=6, req2_name="Garurumon",
        )
        greymon = Permanent([make_card(name="MetalGreymon", level=6, colors=[CardColor.Red])])
        angemon = Permanent([make_card(name="MagnaAngemon", level=6, colors=[CardColor.Yellow])])

        assert can_dna_digivolve(dna_card, greymon, angemon) is False

    def test_dna_multicolor_perm_matches(self):
        """Dual-color permanent can satisfy either color requirement."""
        dna_card = make_dna_card()  # Blue Lv4 + Green Lv4
        dual_perm = Permanent([make_card(name="DualMon", level=4,
                                          colors=[CardColor.Blue, CardColor.Green])])
        green_perm = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])

        # dual_perm matches Blue req, green_perm matches Green req
        assert can_dna_digivolve(dna_card, dual_perm, green_perm) is True

    def test_has_valid_dna_targets(self):
        """has_valid_dna_targets returns True when two valid permanents exist."""
        dna_card = make_dna_card()
        blue_perm = Permanent([make_card(name="BlueMon", level=4, colors=[CardColor.Blue])])
        green_perm = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])

        assert has_valid_dna_targets(dna_card, [blue_perm, green_perm]) is True

    def test_has_valid_dna_targets_insufficient(self):
        """has_valid_dna_targets returns False with only one valid permanent."""
        dna_card = make_dna_card()
        blue_perm = Permanent([make_card(name="BlueMon", level=4, colors=[CardColor.Blue])])

        assert has_valid_dna_targets(dna_card, [blue_perm]) is False


# ═══════════════════════════════════════════════════════════════════════
# D. DNA Action Mask Integration (5 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDnaActionMask:
    def test_dna_mask_shown_when_valid(self):
        """DNA digivolve action appears in mask when valid targets exist."""
        game = setup_game_at_phase(GamePhase.Main)

        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        mask = game.get_action_mask(1)
        # hand=0 → action 63
        assert mask[63] == 1.0

    def test_dna_mask_not_shown_without_targets(self):
        """DNA digivolve action not in mask when insufficient targets."""
        game = setup_game_at_phase(GamePhase.Main)

        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.append(dna_card)

        # Only one valid target (Blue) — need both Blue and Green
        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        game.player1.battle_area.append(blue_perm)

        mask = game.get_action_mask(1)
        assert mask[63] == 0.0

    def test_dna_mask_not_shown_for_non_dna_card(self):
        """Regular digimon without dna_costs doesn't show DNA action."""
        game = setup_game_at_phase(GamePhase.Main)

        regular_card = make_card(
            card_id="TEST-001", name="RegularMon", level=5,
            colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
            owner=game.player1,
        )
        game.player1.hand_cards.append(regular_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        mask = game.get_action_mask(1)
        assert mask[63] == 0.0

    def test_dna_mask_multiple_hand_cards(self):
        """Multiple DNA cards in hand show correct mask positions."""
        game = setup_game_at_phase(GamePhase.Main)

        regular = make_card(
            card_id="TEST-001", name="RegularMon", level=5,
            owner=game.player1, evo_costs=[],
        )
        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.extend([regular, dna_card])

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        mask = game.get_action_mask(1)
        # hand=0 (regular) → no DNA
        assert mask[63] == 0.0
        # hand=1 (dna) → DNA action at 64
        assert mask[64] == 1.0

    def test_dna_mask_selection_phase(self):
        """After initiating DNA digivolve, SelectMaterial phase shows valid targets."""
        game = setup_game_at_phase(GamePhase.Main)

        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        red_perm = Permanent([make_card(
            card_id="BASE-001", name="RedMon", level=4,
            colors=[CardColor.Red], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm, red_perm])

        # Initiate DNA digivolve with hand card 0
        game.decode_action(63, 1)

        # Should be in SelectMaterial phase
        assert game.current_phase == GamePhase.SelectMaterial

        mask = game.get_action_mask(1)
        # Blue (idx 0) and Green (idx 1) should be valid first targets
        assert mask[0] == 1.0
        assert mask[1] == 1.0
        # Red (idx 2) should not be valid
        assert mask[2] == 0.0


# ═══════════════════════════════════════════════════════════════════════
# E. DNA Digivolve Execution via decode_action (6 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDnaExecution:
    def test_full_dna_digivolve_flow(self):
        """Complete DNA digivolve flow: initiate → select first → select second → execute."""
        game = setup_game_at_phase(GamePhase.Main)
        hand_before = len(game.player1.hand_cards)

        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        # Step 1: Initiate DNA with hand card 0
        game.decode_action(63, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Step 2: Select first target (Blue at index 0)
        game.decode_action(0, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Step 3: Select second target (Green at index 1)
        game.decode_action(1, 1)

        # Should be back in Main phase
        assert game.current_phase == GamePhase.Main

        # DNA card removed from hand
        assert dna_card not in game.player1.hand_cards

        # Both original permanents removed, one new permanent created
        assert blue_perm not in game.player1.battle_area
        assert green_perm not in game.player1.battle_area
        assert len(game.player1.battle_area) == 1

        # New permanent has DNA card on top
        new_perm = game.player1.battle_area[0]
        assert new_perm.top_card is dna_card

        # Digivolution bonus: drew 1 card
        assert len(game.player1.hand_cards) == hand_before + 1

    def test_dna_digivolve_unsuspended(self):
        """DNA digivolved Digimon is placed unsuspended."""
        game = setup_game_at_phase(GamePhase.Main)

        dna_card = make_dna_card(card_id="DNA-001", owner=game.player1)
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        # Suspend both — DNA result should still be unsuspended
        blue_perm.suspend()
        green_perm.suspend()
        game.player1.battle_area.extend([blue_perm, green_perm])

        game.decode_action(63, 1)  # Initiate
        game.decode_action(0, 1)   # First target
        game.decode_action(1, 1)   # Second target

        new_perm = game.player1.battle_area[0]
        assert new_perm.is_suspended is False

    def test_dna_digivolve_zero_cost(self):
        """DNA digivolve with Cost 0 doesn't change memory."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)

        dna_card = make_dna_card(
            card_id="DNA-001", memory_cost=0, owner=game.player1,
        )
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        game.decode_action(63, 1)
        game.decode_action(0, 1)
        game.decode_action(1, 1)

        assert game.memory == 5  # Unchanged

    def test_dna_digivolve_with_cost(self):
        """DNA digivolve with Cost 3 subtracts memory."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)

        dna_card = make_dna_card(
            card_id="DNA-001", memory_cost=3, owner=game.player1,
        )
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        game.decode_action(63, 1)
        game.decode_action(0, 1)
        game.decode_action(1, 1)

        assert game.memory == 2  # 5 - 3

    def test_dna_digivolve_negative_memory_triggers_turn_end(self):
        """DNA digivolve can send memory negative, triggering turn switch."""
        game = setup_game_at_phase(GamePhase.Main, memory=1)
        # Give player2 library/security for turn transition
        for _ in range(5):
            game.player2.security_cards.append(
                make_card(card_id="LIB-002", owner=game.player2))

        dna_card = make_dna_card(
            card_id="DNA-001", memory_cost=3, owner=game.player1,
        )
        game.player1.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([blue_perm, green_perm])

        game.decode_action(63, 1)
        game.decode_action(0, 1)
        game.decode_action(1, 1)

        # Memory 1 - 3 = -2 → check_turn_end → switch_turn (negates to 2)
        # But DNA digivolve still executed successfully
        assert dna_card not in game.player1.hand_cards
        assert len(game.player1.battle_area) == 1
        # The digivolve itself was successful even though turn ended
        new_perm = game.player1.battle_area[0]
        assert new_perm.top_card is dna_card

    def test_dna_both_normal_and_dna_on_same_card(self):
        """Card with both evo_costs and dna_costs can use either path."""
        game = setup_game_at_phase(GamePhase.Main)

        # Card with both normal evo (Red Lv4) and DNA (Blue Lv4 + Green Lv4)
        dna_card = make_dna_card(
            card_id="DNA-001", owner=game.player1,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        game.player1.hand_cards.append(dna_card)

        red_perm = Permanent([make_card(
            card_id="BASE-001", name="RedMon", level=4,
            colors=[CardColor.Red], owner=game.player1,
        )])
        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=game.player1,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=game.player1,
        )])
        game.player1.battle_area.extend([red_perm, blue_perm, green_perm])

        mask = game.get_action_mask(1)

        # Normal digivolve: hand=0, field=0 (red) → action 400
        assert mask[400] == 1.0
        # DNA digivolve: hand=0 → action 63
        assert mask[63] == 1.0


# ═══════════════════════════════════════════════════════════════════════
# F. Player.dna_digivolve Stacking Behavior (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDnaStacking:
    def test_stacking_order_correct(self):
        """DNA stacking: bottom sources first, then top sources, then DNA card."""
        game = setup_game_at_phase(GamePhase.Main)
        player = game.player1

        dna_card = make_dna_card(card_id="DNA-001", owner=player)

        # Blue permanent with digivolution stack (egg → rookie → blue lv4)
        blue_egg = make_card(card_id="BASE-001", name="BlueEgg", level=2,
                              kind=CardKind.DigiEgg, colors=[CardColor.Blue], owner=player)
        blue_rookie = make_card(card_id="BASE-002", name="BlueRookie", level=3,
                                 colors=[CardColor.Blue], owner=player)
        blue_top = make_card(card_id="BLUE-001", name="BlueLv4", level=4,
                              colors=[CardColor.Blue], owner=player)
        blue_perm = Permanent([blue_egg, blue_rookie, blue_top])

        # Green permanent (single card)
        green_top = make_card(card_id="GREEN-001", name="GreenLv4", level=4,
                               colors=[CardColor.Green], owner=player)
        green_perm = Permanent([green_top])

        player.battle_area.extend([blue_perm, green_perm])

        # Get stacking order
        stacking = get_dna_stacking_order(dna_card, blue_perm, green_perm)
        assert stacking is not None
        top_perm, bottom_perm, cost = stacking

        # req1=Blue goes on top, req2=Green goes on bottom
        assert top_perm is blue_perm
        assert bottom_perm is green_perm

        # Execute DNA digivolve
        player.dna_digivolve(top_perm, bottom_perm, dna_card, cost)

        # New permanent should have: green_top, blue_egg, blue_rookie, blue_top, dna_card
        new_perm = player.battle_area[0]
        assert len(new_perm.card_sources) == 5
        assert new_perm.card_sources[0] is green_top      # bottom
        assert new_perm.card_sources[1] is blue_egg
        assert new_perm.card_sources[2] is blue_rookie
        assert new_perm.card_sources[3] is blue_top
        assert new_perm.card_sources[4] is dna_card        # top

    def test_both_permanents_removed(self):
        """Both source permanents are removed from battle area."""
        game = setup_game_at_phase(GamePhase.Main)
        player = game.player1

        dna_card = make_dna_card(card_id="DNA-001", owner=player)
        dna_cost = dna_card.c_entity_base.dna_costs[0]

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=player,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=player,
        )])
        player.battle_area.extend([blue_perm, green_perm])
        assert len(player.battle_area) == 2

        player.hand_cards.append(dna_card)
        player.dna_digivolve(blue_perm, green_perm, dna_card, dna_cost)

        assert blue_perm not in player.battle_area
        assert green_perm not in player.battle_area
        assert len(player.battle_area) == 1

    def test_dna_digivolve_draws_card(self):
        """DNA digivolve triggers digivolution bonus (draw 1)."""
        game = setup_game_at_phase(GamePhase.Main)
        player = game.player1

        dna_card = make_dna_card(card_id="DNA-001", owner=player)
        dna_cost = dna_card.c_entity_base.dna_costs[0]
        player.hand_cards.append(dna_card)

        blue_perm = Permanent([make_card(
            card_id="BLUE-001", name="BlueMon", level=4,
            colors=[CardColor.Blue], owner=player,
        )])
        green_perm = Permanent([make_card(
            card_id="GREEN-001", name="GreenMon", level=4,
            colors=[CardColor.Green], owner=player,
        )])
        player.battle_area.extend([blue_perm, green_perm])

        hand_before = len(player.hand_cards) - 1  # -1 for the DNA card that will be used
        player.dna_digivolve(blue_perm, green_perm, dna_card, dna_cost)

        # Should have drawn 1 card (hand = before - dna_card + 1 draw)
        assert len(player.hand_cards) == hand_before + 1

    def test_stacking_order_name_based(self):
        """Name-based DNA requirements determine correct stacking order."""
        dna_card = make_dna_card(
            card_id="DNA-001",
            req1_color=None, req1_level=6, req1_name="Greymon",
            req2_color=None, req2_level=6, req2_name="Garurumon",
        )

        greymon = make_card(card_id="BASE-001", name="MetalGreymon", level=6,
                             colors=[CardColor.Red])
        garurumon = make_card(card_id="BASE-002", name="WereGarurumon", level=6,
                               colors=[CardColor.Blue])
        greymon_perm = Permanent([greymon])
        garurumon_perm = Permanent([garurumon])

        # Greymon is req1 (top), Garurumon is req2 (bottom)
        stacking = get_dna_stacking_order(dna_card, greymon_perm, garurumon_perm)
        assert stacking is not None
        top_perm, bottom_perm, _ = stacking
        assert top_perm is greymon_perm
        assert bottom_perm is garurumon_perm

        # Reversed input order should give same stacking
        stacking2 = get_dna_stacking_order(dna_card, garurumon_perm, greymon_perm)
        assert stacking2 is not None
        top_perm2, bottom_perm2, _ = stacking2
        assert top_perm2 is greymon_perm
        assert bottom_perm2 is garurumon_perm


# ═══════════════════════════════════════════════════════════════════════
# G. Real Card Tests: BT16-025 Paildramon & EX6-062 UltimateChaosmon
# ═══════════════════════════════════════════════════════════════════════

# --- BT16-025 Paildramon ---
# Level 5, Blue/Green, DP 8000, Play Cost 8
# [DNA Digivolve] Blue Lv.4 + Green Lv.4 : Cost 0
# Also has normal evo: Blue Lv.4 cost 4, Green Lv.4 cost 4

BT16_025_XROS_REQ = (
    "[DNA Digivolve] Blue Lv.4 + Green Lv.4 : Cost 0 "
    "Stack the 2 specified Digimon and digivolve unsuspended."
)

# --- EX6-062 UltimateChaosmon ---
# Level 7, White, DP 16000, Play Cost 16
# [DNA Digivolve] Yellow/Black Lv.6 + Green/Purple Lv.6 : Cost 0

EX6_062_XROS_REQ = (
    "[DNA Digivolve] Yellow/Black Lv.6 + Green/Purple Lv.6 : Cost 0 "
    "Stack the 2 specified Digimon and digivolve unsuspended."
)


def make_paildramon(owner=None):
    """BT16-025 Paildramon with parsed xros_req DNA costs + normal evo."""
    dna_costs = parse_xros_req(BT16_025_XROS_REQ)
    return make_card(
        card_id="BT16-025", name="Paildramon", dp=8000, level=5,
        play_cost=8, colors=[CardColor.Blue, CardColor.Green],
        owner=owner,
        evo_costs=[
            EvoCost(card_color=CardColor.Blue, level=4, memory_cost=4),
            EvoCost(card_color=CardColor.Green, level=4, memory_cost=4),
        ],
        dna_costs=dna_costs,
    )


def make_ultimate_chaosmon(owner=None):
    """EX6-062 UltimateChaosmon with parsed xros_req DNA costs + normal evo."""
    dna_costs = parse_xros_req(EX6_062_XROS_REQ)
    return make_card(
        card_id="EX6-062", name="UltimateChaosmon", dp=16000, level=7,
        play_cost=16, colors=[CardColor.White],
        owner=owner,
        evo_costs=[
            EvoCost(card_color=CardColor.Yellow, level=6, memory_cost=7),
            EvoCost(card_color=CardColor.Green, level=6, memory_cost=7),
        ],
        dna_costs=dna_costs,
    )


class TestBT16Paildramon:
    """Tests using BT16-025 Paildramon: [DNA Digivolve] Blue Lv.4 + Green Lv.4: Cost 0."""

    def test_paildramon_xros_req_parsed(self):
        """The xros_req text is parsed into correct DnaCost."""
        card = make_paildramon()
        assert len(card.c_entity_base.dna_costs) == 1
        dna = card.c_entity_base.dna_costs[0]
        assert dna.requirement1.card_color == CardColor.Blue
        assert dna.requirement1.level == 4
        assert dna.requirement2.card_color == CardColor.Green
        assert dna.requirement2.level == 4
        assert dna.memory_cost == 0

    def test_paildramon_dna_valid_with_blue_green(self):
        """Paildramon can DNA digivolve from Blue Lv4 + Green Lv4."""
        paildramon = make_paildramon()

        # ExVeemon (Blue Lv4)
        exveemon = Permanent([make_card(
            card_id="BLUE-001", name="ExVeemon", level=4, dp=5000,
            colors=[CardColor.Blue],
        )])
        # Stingmon (Green Lv4)
        stingmon = Permanent([make_card(
            card_id="GREEN-001", name="Stingmon", level=4, dp=5000,
            colors=[CardColor.Green],
        )])
        assert can_dna_digivolve(paildramon, exveemon, stingmon) is True

    def test_paildramon_dna_invalid_two_blues(self):
        """Paildramon cannot DNA from two Blue Lv4 (need Blue + Green)."""
        paildramon = make_paildramon()

        blue1 = Permanent([make_card(name="Blue1", level=4, colors=[CardColor.Blue])])
        blue2 = Permanent([make_card(name="Blue2", level=4, colors=[CardColor.Blue])])

        assert can_dna_digivolve(paildramon, blue1, blue2) is False

    def test_paildramon_dna_valid_with_dual_color_perm(self):
        """A Blue/Green dual-color Lv4 can serve as either requirement."""
        paildramon = make_paildramon()

        # Dual color Blue/Green Lv4 (e.g., Lighdramon)
        dual = Permanent([make_card(
            name="DualMon", level=4, colors=[CardColor.Blue, CardColor.Green],
        )])
        green = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])

        # dual can match Blue req, green matches Green req
        assert can_dna_digivolve(paildramon, dual, green) is True

    def test_paildramon_also_has_normal_evo(self):
        """Paildramon can also normal-digivolve onto a Blue or Green Lv4."""
        from digimon_gym.engine.validation.digivolve_validator import can_digivolve
        paildramon = make_paildramon()

        blue_lv4 = Permanent([make_card(name="BlueMon", level=4, colors=[CardColor.Blue])])
        assert can_digivolve(paildramon, blue_lv4) is True

        green_lv4 = Permanent([make_card(name="GreenMon", level=4, colors=[CardColor.Green])])
        assert can_digivolve(paildramon, green_lv4) is True

    def test_paildramon_stacking_order(self):
        """Paildramon DNA: Blue sources on top, Green sources on bottom."""
        paildramon = make_paildramon()

        exveemon = make_card(name="ExVeemon", level=4, colors=[CardColor.Blue])
        stingmon = make_card(name="Stingmon", level=4, colors=[CardColor.Green])
        exv_perm = Permanent([exveemon])
        stg_perm = Permanent([stingmon])

        stacking = get_dna_stacking_order(paildramon, exv_perm, stg_perm)
        assert stacking is not None
        top, bottom, cost = stacking
        # req1=Blue → ExVeemon on top, req2=Green → Stingmon on bottom
        assert top is exv_perm
        assert bottom is stg_perm
        assert cost.memory_cost == 0

    def test_paildramon_full_game_flow(self):
        """Full game flow: DNA digivolve into Paildramon at Cost 0."""
        game = setup_game_at_phase(GamePhase.Main, memory=3)
        player = game.player1

        paildramon = make_paildramon(owner=player)
        player.hand_cards.append(paildramon)

        exveemon = Permanent([make_card(
            card_id="BLUE-001", name="ExVeemon", level=4, dp=5000,
            colors=[CardColor.Blue], owner=player,
        )])
        stingmon = Permanent([make_card(
            card_id="GREEN-001", name="Stingmon", level=4, dp=5000,
            colors=[CardColor.Green], owner=player,
        )])
        player.battle_area.extend([exveemon, stingmon])

        # Initiate DNA (hand idx 0)
        game.decode_action(63, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Select ExVeemon (field idx 0)
        game.decode_action(0, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Select Stingmon (field idx 1)
        game.decode_action(1, 1)

        # Result
        assert game.current_phase == GamePhase.Main
        assert game.memory == 3  # Cost 0 → unchanged
        assert len(player.battle_area) == 1

        new_perm = player.battle_area[0]
        assert new_perm.top_card is paildramon
        assert new_perm.is_suspended is False
        # Stack: stingmon (bottom), exveemon, paildramon (top)
        assert len(new_perm.card_sources) == 3

    def test_paildramon_dna_invalid_wrong_level(self):
        """Paildramon cannot DNA from Blue Lv3 + Green Lv4."""
        paildramon = make_paildramon()

        blue_lv3 = Permanent([make_card(name="BlueLv3", level=3, colors=[CardColor.Blue])])
        green_lv4 = Permanent([make_card(name="GreenLv4", level=4, colors=[CardColor.Green])])

        assert can_dna_digivolve(paildramon, blue_lv3, green_lv4) is False


class TestEX6UltimateChaosmon:
    """Tests using EX6-062 UltimateChaosmon: [DNA Digivolve] Yellow/Black Lv.6 + Green/Purple Lv.6: Cost 0."""

    def test_chaosmon_xros_req_parsed(self):
        """The xros_req text is parsed into correct DnaCost."""
        card = make_ultimate_chaosmon()
        assert len(card.c_entity_base.dna_costs) == 1
        dna = card.c_entity_base.dna_costs[0]
        # "Yellow/Black" → parser picks first color: Yellow
        assert dna.requirement1.card_color == CardColor.Yellow
        assert dna.requirement1.level == 6
        # "Green/Purple" → parser picks first color: Green
        assert dna.requirement2.card_color == CardColor.Green
        assert dna.requirement2.level == 6
        assert dna.memory_cost == 0

    def test_chaosmon_dna_yellow_green(self):
        """UltimateChaosmon DNA: Yellow Lv6 + Green Lv6 is valid."""
        chaosmon = make_ultimate_chaosmon()

        yellow_lv6 = Permanent([make_card(
            name="YellowMega", level=6, dp=12000, colors=[CardColor.Yellow],
        )])
        green_lv6 = Permanent([make_card(
            name="GreenMega", level=6, dp=12000, colors=[CardColor.Green],
        )])

        assert can_dna_digivolve(chaosmon, yellow_lv6, green_lv6) is True

    def test_chaosmon_dna_reversed_order(self):
        """Order of permanents doesn't matter for validation."""
        chaosmon = make_ultimate_chaosmon()

        yellow_lv6 = Permanent([make_card(
            name="YellowMega", level=6, colors=[CardColor.Yellow],
        )])
        green_lv6 = Permanent([make_card(
            name="GreenMega", level=6, colors=[CardColor.Green],
        )])

        assert can_dna_digivolve(chaosmon, green_lv6, yellow_lv6) is True

    def test_chaosmon_dna_wrong_level(self):
        """Lv5 materials don't match Lv6 requirement."""
        chaosmon = make_ultimate_chaosmon()

        yellow_lv5 = Permanent([make_card(
            name="YellowUlt", level=5, colors=[CardColor.Yellow],
        )])
        green_lv5 = Permanent([make_card(
            name="GreenUlt", level=5, colors=[CardColor.Green],
        )])

        assert can_dna_digivolve(chaosmon, yellow_lv5, green_lv5) is False

    def test_chaosmon_dna_wrong_colors(self):
        """Blue + Red doesn't match Yellow/Black + Green/Purple."""
        chaosmon = make_ultimate_chaosmon()

        blue_lv6 = Permanent([make_card(
            name="BlueMega", level=6, colors=[CardColor.Blue],
        )])
        red_lv6 = Permanent([make_card(
            name="RedMega", level=6, colors=[CardColor.Red],
        )])

        assert can_dna_digivolve(chaosmon, blue_lv6, red_lv6) is False

    def test_chaosmon_stacking_order(self):
        """Yellow on top (req1), Green on bottom (req2)."""
        chaosmon = make_ultimate_chaosmon()

        yellow = make_card(name="YellowMega", level=6, colors=[CardColor.Yellow])
        green = make_card(name="GreenMega", level=6, colors=[CardColor.Green])
        yellow_perm = Permanent([yellow])
        green_perm = Permanent([green])

        stacking = get_dna_stacking_order(chaosmon, yellow_perm, green_perm)
        assert stacking is not None
        top, bottom, cost = stacking
        assert top is yellow_perm  # Yellow matches req1 → top
        assert bottom is green_perm  # Green matches req2 → bottom

    def test_chaosmon_full_game_flow(self):
        """Full game flow: DNA digivolve into UltimateChaosmon."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        player = game.player1

        chaosmon = make_ultimate_chaosmon(owner=player)
        player.hand_cards.append(chaosmon)

        yellow_mega = Permanent([make_card(
            card_id="BASE-001", name="YellowMega", level=6, dp=12000,
            colors=[CardColor.Yellow], owner=player,
        )])
        green_mega = Permanent([make_card(
            card_id="BASE-002", name="GreenMega", level=6, dp=12000,
            colors=[CardColor.Green], owner=player,
        )])
        player.battle_area.extend([yellow_mega, green_mega])

        # Initiate
        game.decode_action(63, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Select Yellow (field idx 0)
        game.decode_action(0, 1)
        assert game.current_phase == GamePhase.SelectMaterial

        # Select Green (field idx 1)
        game.decode_action(1, 1)

        # Verify
        assert game.current_phase == GamePhase.Main
        assert game.memory == 5  # Cost 0
        assert len(player.battle_area) == 1

        new_perm = player.battle_area[0]
        assert new_perm.top_card is chaosmon
        assert new_perm.is_suspended is False
        assert new_perm.dp == 16000
        # Stack: green_mega (bottom), yellow_mega, chaosmon (top)
        assert len(new_perm.card_sources) == 3

    def test_chaosmon_deep_digivolution_stacks(self):
        """DNA with multi-card digivolution stacks combines correctly."""
        game = setup_game_at_phase(GamePhase.Main)
        player = game.player1

        chaosmon = make_ultimate_chaosmon(owner=player)

        # Yellow stack: egg → rookie → champion → ultimate (lv6)
        y_egg = make_card(name="YellowEgg", level=2, kind=CardKind.DigiEgg,
                           colors=[CardColor.Yellow], owner=player)
        y_rook = make_card(name="YellowRookie", level=3, dp=2000,
                            colors=[CardColor.Yellow], owner=player)
        y_champ = make_card(name="YellowChamp", level=4, dp=5000,
                             colors=[CardColor.Yellow], owner=player)
        y_ult = make_card(card_id="BASE-001", name="YellowMega", level=6, dp=12000,
                           colors=[CardColor.Yellow], owner=player)
        yellow_perm = Permanent([y_egg, y_rook, y_champ, y_ult])

        # Green stack: rookie → champion → ultimate (lv6)
        g_rook = make_card(name="GreenRookie", level=3, dp=2000,
                            colors=[CardColor.Green], owner=player)
        g_champ = make_card(name="GreenChamp", level=4, dp=5000,
                             colors=[CardColor.Green], owner=player)
        g_ult = make_card(card_id="BASE-002", name="GreenMega", level=6, dp=12000,
                           colors=[CardColor.Green], owner=player)
        green_perm = Permanent([g_rook, g_champ, g_ult])

        player.battle_area.extend([yellow_perm, green_perm])
        player.hand_cards.append(chaosmon)

        dna_cost = chaosmon.c_entity_base.dna_costs[0]
        stacking = get_dna_stacking_order(chaosmon, yellow_perm, green_perm)
        top, bottom, cost = stacking

        player.dna_digivolve(top, bottom, chaosmon, cost)

        new_perm = player.battle_area[0]
        # Stack: green sources (3) + yellow sources (4) + chaosmon (1) = 8
        assert len(new_perm.card_sources) == 8
        # Bottom: green stack
        assert new_perm.card_sources[0] is g_rook
        assert new_perm.card_sources[1] is g_champ
        assert new_perm.card_sources[2] is g_ult
        # Middle: yellow stack
        assert new_perm.card_sources[3] is y_egg
        assert new_perm.card_sources[4] is y_rook
        assert new_perm.card_sources[5] is y_champ
        assert new_perm.card_sources[6] is y_ult
        # Top: chaosmon
        assert new_perm.card_sources[7] is chaosmon
        assert new_perm.top_card is chaosmon
