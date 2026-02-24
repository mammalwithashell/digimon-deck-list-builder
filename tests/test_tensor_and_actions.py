"""Tests for game state tensor, action mask, action decoder, and card registry."""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import (
    Game, TENSOR_SIZE, ACTION_SPACE_SIZE, FIELD_SLOTS, SLOT_SIZE,
    MAX_HAND, MAX_TRASH, MAX_SECURITY, MAX_SOURCES,
)
from digimon_gym.engine.data.enums import GamePhase, CardKind, CardColor
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.evo_cost import EvoCost
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect


# ─── Helpers ─────────────────────────────────────────────────────────

def make_card(card_id="BT14-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None,
              evo_costs=None):
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
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


class MockCardSourceWithEffects(CardSource):
    """CardSource that returns custom effects instead of querying CardDatabase."""

    def __init__(self):
        super().__init__()
        self._mock_effects = []

    def effect_list(self, timing):
        return self._mock_effects


def make_card_with_effects(card_id="BT14-001", name="TestDigimon", kind=CardKind.Digimon,
                           dp=5000, level=4, play_cost=5, colors=None, owner=None,
                           effects=None):
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)
    cs._mock_effects = effects or []
    for eff in cs._mock_effects:
        eff.effect_source_card = cs
    return cs


def setup_game_at_phase(phase: GamePhase, memory: int = 5) -> Game:
    """Create a Game positioned at the given phase with some cards."""
    game = Game()
    game.current_phase = phase
    game.memory = memory
    game.turn_count = 2
    game.turn_player = game.player1
    game.opponent_player = game.player2
    game.player1.is_my_turn = True
    return game


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test to ensure isolation."""
    CardRegistry.reset()
    CardRegistry.initialize_from_list([
        "BT14-001", "BT14-002", "BT14-003", "BT14-010", "BT14-020",
        "BT14-030", "BT14-050", "BT14-074", "BT14-090", "BT14-100",
        "BT24-001", "BT24-050", "BT24-100",
    ])
    yield
    CardRegistry.reset()


# ─── CardRegistry Tests ─────────────────────────────────────────────

class TestCardRegistry:
    def test_padding_id_is_zero(self):
        assert CardRegistry.PADDING_ID == 0

    def test_get_id_known(self):
        # Sorted alphabetically: BT14-001=1, BT14-002=2, ...
        assert CardRegistry.get_id("BT14-001") == 1
        assert CardRegistry.get_id("BT14-002") == 2

    def test_get_id_unknown(self):
        assert CardRegistry.get_id("UNKNOWN-999") == 0

    def test_get_id_empty(self):
        assert CardRegistry.get_id("") == 0

    def test_get_string_id(self):
        assert CardRegistry.get_string_id(1) == "BT14-001"

    def test_get_string_id_unknown(self):
        assert CardRegistry.get_string_id(9999) is None

    def test_count(self):
        assert CardRegistry.count() == 13

    def test_deterministic_ordering(self):
        """IDs should be sorted alphabetically for determinism."""
        ids = []
        for i in range(1, CardRegistry.count() + 1):
            ids.append(CardRegistry.get_string_id(i))
        assert ids == sorted(ids)

    def test_roundtrip(self):
        for cid in ["BT14-001", "BT14-074", "BT24-100"]:
            int_id = CardRegistry.get_id(cid)
            assert CardRegistry.get_string_id(int_id) == cid

    def test_initialize_from_cards_json(self):
        """Integration: initialize from the real cards.json file."""
        CardRegistry.reset()
        CardRegistry.initialize()
        assert CardRegistry.count() > 100
        assert CardRegistry.get_id("BT14-001") > 0
        assert CardRegistry.get_id("BT24-001") > 0


# ─── Tensor Tests ────────────────────────────────────────────────────

class TestBoardStateTensor:
    def test_tensor_size(self):
        game = setup_game_at_phase(GamePhase.Main)
        tensor = game.get_board_state_tensor(1)
        assert len(tensor) == TENSOR_SIZE

    def test_global_data(self):
        game = setup_game_at_phase(GamePhase.Main, memory=7)
        game.turn_count = 5
        tensor = game.get_board_state_tensor(1)

        assert tensor[0] == 5.0   # turn count
        assert tensor[1] == float(GamePhase.Main.value)  # phase
        assert tensor[2] == 7.0   # memory for player 1

    def test_memory_relative_to_player(self):
        game = setup_game_at_phase(GamePhase.Main, memory=4)
        t1 = game.get_board_state_tensor(1)
        t2 = game.get_board_state_tensor(2)

        assert t1[2] == 4.0   # P1's view: +4
        assert t2[2] == -4.0  # P2's view: -4

    def test_empty_board_all_zeros(self):
        game = setup_game_at_phase(GamePhase.Main, memory=0)
        game.turn_count = 0
        tensor = game.get_board_state_tensor(1)

        # Everything after global data should be 0
        assert all(v == 0.0 for v in tensor[3:])

    def test_battle_area_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        card = make_card("BT14-001", "TestMon", dp=5000, level=4, owner=p1)
        perm = Permanent([card])
        perm.suspend()
        p1.battle_area.append(perm)

        tensor = game.get_board_state_tensor(1)

        # My field starts at index 10
        base = 10
        assert tensor[base + 0] == CardRegistry.get_norm_id("BT14-001")  # card ID
        assert tensor[base + 1] == 5000.0  # DP
        assert tensor[base + 2] == 1.0     # suspended
        assert tensor[base + 6] == 1.0     # source count
        assert tensor[base + 7] == CardRegistry.get_norm_id("BT14-001")  # source[0] card_id
        assert tensor[base + 8] == -1.0   # source[0] opt_state: no OPT effects
        assert tensor[base + 9] == 0.0    # source[0] dp_contribution: no DP effect

    def test_digivolution_stack_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        base = make_card("BT14-002", "Rookie", dp=3000, level=3, owner=p1)
        top = make_card("BT14-010", "Champion", dp=6000, level=4, owner=p1)
        perm = Permanent([base, top])
        p1.battle_area.append(perm)

        tensor = game.get_board_state_tensor(1)

        slot_base = 10
        assert tensor[slot_base + 0] == CardRegistry.get_norm_id("BT14-010")  # top card
        assert tensor[slot_base + 1] == 6000.0  # DP
        assert tensor[slot_base + 6] == 2.0     # 2 sources
        assert tensor[slot_base + 7] == CardRegistry.get_norm_id("BT14-002")   # source[0] card_id
        assert tensor[slot_base + 8] == -1.0    # source[0] opt_state: no OPT
        assert tensor[slot_base + 9] == 0.0     # source[0] dp_contribution
        assert tensor[slot_base + 10] == CardRegistry.get_norm_id("BT14-010")  # source[1] card_id
        assert tensor[slot_base + 11] == -1.0   # source[1] opt_state: no OPT
        assert tensor[slot_base + 12] == 0.0    # source[1] dp_contribution

    def test_opponent_field_offset(self):
        game = setup_game_at_phase(GamePhase.Main)
        p2 = game.player2

        card = make_card("BT14-020", "OppMon", dp=7000, owner=p2)
        perm = Permanent([card])
        p2.battle_area.append(perm)

        tensor = game.get_board_state_tensor(1)

        # Opp field starts at 382
        base = 382
        assert tensor[base + 0] == CardRegistry.get_norm_id("BT14-020")
        assert tensor[base + 1] == 7000.0

    def test_hand_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        p1.hand_cards.append(make_card("BT14-003", owner=p1))
        p1.hand_cards.append(make_card("BT14-050", owner=p1))

        tensor = game.get_board_state_tensor(1)

        # My hand starts at 754
        assert tensor[754] == CardRegistry.get_norm_id("BT14-003")
        assert tensor[755] == CardRegistry.get_norm_id("BT14-050")
        assert tensor[756] == 0.0  # padding

    def test_trash_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        for i in range(3):
            p1.trash_cards.append(make_card(f"BT14-00{i+1}", owner=p1))

        tensor = game.get_board_state_tensor(1)

        # My trash starts at 794
        assert tensor[794] == CardRegistry.get_norm_id("BT14-001")
        assert tensor[795] == CardRegistry.get_norm_id("BT14-002")
        assert tensor[796] == CardRegistry.get_norm_id("BT14-003")

    def test_security_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        p1.security_cards.append(make_card("BT14-090", owner=p1))

        tensor = game.get_board_state_tensor(1)

        # My security starts at 884
        assert tensor[884] == CardRegistry.get_norm_id("BT14-090")
        assert tensor[885] == 0.0

    def test_breeding_encoding(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        egg = make_card("BT14-001", "DigiEgg", kind=CardKind.DigiEgg, dp=0, level=2, owner=p1)
        p1.breeding_area = Permanent([egg])

        tensor = game.get_board_state_tensor(1)

        # My breeding starts at 904
        assert tensor[904] == CardRegistry.get_norm_id("BT14-001")
        assert tensor[910] == 1.0  # source count

    def test_ui_memory_gauge_is_player1_oriented(self):
        game = setup_game_at_phase(GamePhase.Main, memory=2)
        # Make player2 the active turn player with +2 relative memory.
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        ui = game.to_ui_json()
        assert ui["memoryGauge"] == -2
        assert ui["player1"]["memory"] == -2
        assert ui["player2"]["memory"] == 2

    def test_ui_json_includes_stack_inspector_breakdowns(self):
        game = setup_game_at_phase(GamePhase.Main, memory=3)
        p1 = game.player1

        inherited_effect = ICardEffect()
        inherited_effect.is_inherited_effect = True
        inherited_effect.dp_modifier = 1000
        inherited_effect._is_blocker = True
        inherited_effect.set_can_use_condition(lambda ctx: True)

        top_effect = ICardEffect()
        top_effect.is_inherited_effect = False
        top_effect.dp_modifier = 500
        top_effect.set_can_use_condition(lambda ctx: True)

        base = make_card_with_effects(
            card_id="BT14-001",
            name="Base",
            level=3,
            dp=1000,
            owner=p1,
            effects=[inherited_effect],
        )
        base.c_entity_base.inherited_effect_description_eng = "[Inherited] Gain +1000 DP."

        top = make_card_with_effects(
            card_id="BT14-010",
            name="Top",
            level=4,
            dp=5000,
            owner=p1,
            effects=[top_effect],
        )
        top.c_entity_base.effect_description_eng = "[Your Turn] This Digimon gets +500 DP."

        perm = Permanent([base, top])
        perm._owner_game = game
        perm.grant_keyword("_is_rush", duration=-1)
        perm.change_dp(300)
        p1.battle_area.append(perm)

        ui = game.to_ui_json()
        battle_perm = ui["player1"]["battleArea"][0]

        assert battle_perm["mainEffectText"] == "[Your Turn] This Digimon gets +500 DP."
        assert battle_perm["inheritedEffects"][0]["cardId"] == "BT14-001"
        assert battle_perm["inheritedEffects"][0]["text"] == "[Inherited] Gain +1000 DP."
        assert "blocker" in battle_perm["keywordBreakdown"]["innate"]
        assert "rush" in battle_perm["keywordBreakdown"]["gained"]
        assert battle_perm["sources"][0]["dpContribution"] == 1000.0
        assert battle_perm["sources"][1]["dpContribution"] == 500.0
        assert battle_perm["dpBreakdown"]["base"] == 5000
        assert battle_perm["dpBreakdown"]["temporary"] == 300.0
        assert battle_perm["dpBreakdown"]["total"] == 6800


# ─── Mulligan Setup Tests ─────────────────────────────────────────────

class TestMulliganSetup:
    @staticmethod
    def _seed_decks(game: Game):
        seeded_ids = [
            "BT14-001", "BT14-002", "BT14-003", "BT14-010",
            "BT14-020", "BT14-030", "BT14-050", "BT14-074",
            "BT14-090", "BT14-100",
        ]
        for player in (game.player1, game.player2):
            player.library_cards = [
                make_card(seeded_ids[i % len(seeded_ids)], owner=player)
                for i in range(50)
            ]
            player.digitama_library_cards = [
                make_card("BT14-001", kind=CardKind.DigiEgg, level=2, owner=player)
                for _ in range(4)
            ]

    def test_start_game_enters_mulligan_with_no_security(self):
        game = Game()
        self._seed_decks(game)

        game.start_game()

        assert game.current_phase == GamePhase.Mulligan
        assert len(game.player1.hand_cards) == 5
        assert len(game.player2.hand_cards) == 5
        assert len(game.player1.security_cards) == 0
        assert len(game.player2.security_cards) == 0

    def test_mulligan_mask_uses_action_zero_and_one_only(self):
        game = Game()
        self._seed_decks(game)
        game.start_game()

        mask = game.get_action_mask(game.current_player_id)
        assert mask[0] == 1.0
        assert mask[1] == 1.0
        assert mask[62] == 0.0
        assert sum(1 for v in mask if v == 1.0) == 2

    def test_keep_then_keep_completes_setup_and_enters_breeding(self):
        game = Game()
        self._seed_decks(game)
        game.start_game()

        game.decode_action(0, game.current_player_id)  # first player keeps
        assert game.current_phase == GamePhase.Mulligan
        game.decode_action(0, game.current_player_id)  # second player keeps

        assert game.current_phase == GamePhase.Breeding
        assert len(game.player1.security_cards) == 5
        assert len(game.player2.security_cards) == 5

    def test_mulligan_tensor_has_opening_hand_and_empty_security(self):
        game = Game()
        self._seed_decks(game)
        game.start_game()

        tensor = game.get_board_state_tensor(1)
        assert any(v != 0.0 for v in tensor[754:759])  # opening hand cards
        assert all(v == 0.0 for v in tensor[884:894])  # security not set yet


# ─── Action Mask Tests ───────────────────────────────────────────────

class TestActionMask:
    def test_mask_size(self):
        game = setup_game_at_phase(GamePhase.Main)
        mask = game.get_action_mask(1)
        assert len(mask) == ACTION_SPACE_SIZE

    def test_main_phase_pass_always_valid(self):
        game = setup_game_at_phase(GamePhase.Main)
        mask = game.get_action_mask(1)
        assert mask[62] == 1.0

    def test_main_phase_no_hand_no_play(self):
        game = setup_game_at_phase(GamePhase.Main)
        mask = game.get_action_mask(1)
        # No hand cards -> play actions 0-29 all invalid
        assert all(mask[i] == 0.0 for i in range(30))

    def test_main_phase_play_card_gauge_limit(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1
        p1.hand_cards.append(make_card("BT14-001", play_cost=3, owner=p1))
        p1.hand_cards.append(make_card("BT14-002", play_cost=8, owner=p1))
        p1.hand_cards.append(make_card("BT14-003", play_cost=16, owner=p1))

        mask = game.get_action_mask(1)
        # Cards are playable as long as cost doesn't exceed memory + 10
        # (gauge caps at -10, so cost must be <= current_memory + 10)
        assert mask[0] == 1.0  # cost 3: 5-3 = 2, fine
        assert mask[1] == 1.0  # cost 8: 5-8 = -3, fine (within gauge)
        assert mask[2] == 0.0  # cost 16: 5-16 = -11, exceeds gauge max of -10

    def test_main_phase_attack_security(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        card = make_card("BT14-001", dp=5000, level=4, owner=p1)
        perm = Permanent([card])
        p1.battle_area.append(perm)

        mask = game.get_action_mask(1)

        # Attacker 0, target 12 (security) -> action 100 + 0*15 + 12 = 112
        assert mask[112] == 1.0

    def test_main_phase_attack_suspended_digimon(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1, p2 = game.player1, game.player2

        attacker = make_card("BT14-001", dp=5000, level=4, owner=p1)
        p1.battle_area.append(Permanent([attacker]))

        target = make_card("BT14-002", dp=3000, level=3, owner=p2)
        target_perm = Permanent([target])
        target_perm.suspend()
        p2.battle_area.append(target_perm)

        mask = game.get_action_mask(1)

        # Attacker 0, target 0 -> action 100
        assert mask[100] == 1.0

    def test_main_phase_no_attack_on_unsuspended(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1, p2 = game.player1, game.player2

        attacker = make_card("BT14-001", dp=5000, level=4, owner=p1)
        p1.battle_area.append(Permanent([attacker]))

        target = make_card("BT14-002", dp=3000, level=3, owner=p2)
        p2.battle_area.append(Permanent([target]))  # not suspended

        mask = game.get_action_mask(1)

        # Target 0 not suspended -> can't attack it
        assert mask[100] == 0.0

    def test_main_phase_suspended_attacker_cant_attack(self):
        game = setup_game_at_phase(GamePhase.Main)
        p1 = game.player1

        card = make_card("BT14-001", dp=5000, level=4, owner=p1)
        perm = Permanent([card])
        perm.suspend()
        p1.battle_area.append(perm)

        mask = game.get_action_mask(1)
        # All attack actions for slot 0 should be invalid
        assert all(mask[100 + i] == 0.0 for i in range(15))

    def test_main_phase_digivolve_valid(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1

        # Hand card: level 4 Red with evo_costs requiring Red Lv3
        evo = make_card("BT14-010", "Champion", dp=6000, level=4,
                        play_cost=5, colors=[CardColor.Red], owner=p1,
                        evo_costs=[EvoCost(CardColor.Red, 3, 2)])
        p1.hand_cards.append(evo)

        # Field: level 3 Red
        base_card = make_card("BT14-003", "Rookie", dp=3000, level=3,
                              colors=[CardColor.Red], owner=p1)
        p1.battle_area.append(Permanent([base_card]))

        mask = game.get_action_mask(1)

        # hand 0, field 0 -> 400 + 0*15 + 0 = 400
        assert mask[400] == 1.0

    def test_main_phase_digivolve_wrong_level(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1

        # Hand card: level 5 with evo_costs requiring Red Lv4
        evo = make_card("BT14-010", level=5, colors=[CardColor.Red], owner=p1,
                        evo_costs=[EvoCost(CardColor.Red, 4, 3)])
        p1.hand_cards.append(evo)

        # Field: level 3 (need level 4 to evolve to 5)
        base_card = make_card("BT14-003", level=3, colors=[CardColor.Red], owner=p1)
        p1.battle_area.append(Permanent([base_card]))

        mask = game.get_action_mask(1)
        assert mask[400] == 0.0

    def test_main_phase_digivolve_wrong_color(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1

        # Hand card: Blue Lv4 with evo_costs requiring Blue Lv3
        evo = make_card("BT14-010", level=4, colors=[CardColor.Blue], owner=p1,
                        evo_costs=[EvoCost(CardColor.Blue, 3, 2)])
        p1.hand_cards.append(evo)

        # Field: Red Lv3 (wrong color for Blue evo requirement)
        base_card = make_card("BT14-003", level=3, colors=[CardColor.Red], owner=p1)
        p1.battle_area.append(Permanent([base_card]))

        mask = game.get_action_mask(1)
        assert mask[400] == 0.0

    def test_main_phase_digivolve_breeding_valid(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1

        # Breeding base: Lv2 red egg
        egg = make_card("BT14-001", kind=CardKind.DigiEgg, level=2, colors=[CardColor.Red], owner=p1)
        p1.breeding_area = Permanent([egg])

        # Hand card: Lv3 with evo requirement from red Lv2
        evo = make_card(
            "BT14-003",
            level=3,
            colors=[CardColor.Red],
            owner=p1,
            evo_costs=[EvoCost(CardColor.Red, 2, 0)],
        )
        p1.hand_cards.append(evo)

        mask = game.get_action_mask(1)
        # 400 + hand[0]*15 + breeding_slot[12]
        assert mask[412] == 1.0

    def test_breeding_phase_hatch(self):
        game = setup_game_at_phase(GamePhase.Breeding)
        p1 = game.player1
        p1.digitama_library_cards.append(make_card("BT14-001", kind=CardKind.DigiEgg, owner=p1))

        mask = game.get_action_mask(1)
        assert mask[60] == 1.0  # hatch
        assert mask[61] == 0.0  # can't move (empty breeding)
        assert mask[62] == 1.0  # pass

    def test_breeding_phase_move(self):
        game = setup_game_at_phase(GamePhase.Breeding)
        p1 = game.player1
        card = make_card("BT14-003", level=3, owner=p1)
        p1.breeding_area = Permanent([card])

        mask = game.get_action_mask(1)
        assert mask[60] == 0.0  # can't hatch (breeding occupied)
        assert mask[61] == 1.0  # move (level >= 3)
        assert mask[62] == 1.0  # pass

    def test_breeding_phase_cant_move_low_level(self):
        game = setup_game_at_phase(GamePhase.Breeding)
        p1 = game.player1
        card = make_card("BT14-001", kind=CardKind.DigiEgg, level=2, owner=p1)
        p1.breeding_area = Permanent([card])

        mask = game.get_action_mask(1)
        assert mask[61] == 0.0  # can't move (level < 3)

    def test_breeding_phase_auto_skips_when_only_pass_exists(self):
        game = setup_game_at_phase(GamePhase.Breeding)
        p1 = game.player1
        # No hatch (empty egg deck), no move (Lv2), no training keyword.
        p1.breeding_area = Permanent([make_card("BT14-001", kind=CardKind.DigiEgg, level=2, owner=p1)])
        p1.digitama_library_cards = []

        game.phase_breeding()
        assert game.current_phase == GamePhase.Main

    def test_mask_player_perspective(self):
        """Mask should be from the requesting player's perspective."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        # Player 2 has a hand card, player 1 does not
        game.player2.hand_cards.append(make_card("BT14-001", play_cost=3, owner=game.player2))

        mask1 = game.get_action_mask(1)
        mask2 = game.get_action_mask(2)

        assert mask1[0] == 0.0  # P1 has no hand cards
        assert mask2[0] == 1.0  # P2 has an affordable card


# ─── Action Decoder Tests ───────────────────────────────────────────

class TestActionDecoder:
    def test_decode_pass_turn(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        game.decode_action(62, 1)
        assert game.current_phase == GamePhase.Start or game.game_over  # advanced past End

    def test_decode_play_card(self):
        game = setup_game_at_phase(GamePhase.Main, memory=10)
        p1 = game.player1
        card = make_card("BT14-001", play_cost=3, dp=5000, level=4, owner=p1)
        p1.hand_cards.append(card)

        assert len(p1.battle_area) == 0
        game.decode_action(0, 1)  # play hand[0]

        assert len(p1.battle_area) == 1
        assert len(p1.hand_cards) == 0

    def test_decode_attack_security(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1, p2 = game.player1, game.player2

        attacker = make_card("BT14-001", dp=5000, level=4, owner=p1)
        p1.battle_area.append(Permanent([attacker]))

        # Give opponent security
        p2.security_cards.append(make_card("BT14-002", dp=1000, owner=p2))

        # Action 112 = attack slot 0 -> security (100 + 0*15 + 12)
        game.decode_action(112, 1)

        assert p1.battle_area[0].is_suspended
        assert len(p2.security_cards) == 0

    def test_decode_attack_digimon(self):
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1, p2 = game.player1, game.player2

        attacker = make_card("BT14-001", dp=8000, level=4, owner=p1)
        p1.battle_area.append(Permanent([attacker]))

        target = make_card("BT14-002", dp=3000, level=3, owner=p2)
        target_perm = Permanent([target])
        target_perm.suspend()
        p2.battle_area.append(target_perm)

        # Action 100 = attack slot 0 -> target slot 0 (100 + 0*15 + 0)
        game.decode_action(100, 1)

        assert p1.battle_area[0].is_suspended
        assert len(p2.battle_area) == 0  # defender deleted
        assert len(p2.trash_cards) == 1

    def test_decode_digivolve(self):
        game = setup_game_at_phase(GamePhase.Main, memory=10)
        p1 = game.player1

        base_card = make_card("BT14-003", "Rookie", dp=3000, level=3,
                              colors=[CardColor.Red], owner=p1)
        perm = Permanent([base_card])
        p1.battle_area.append(perm)

        evo = make_card("BT14-010", "Champion", dp=6000, level=4,
                        play_cost=5, colors=[CardColor.Red], owner=p1)
        p1.hand_cards.append(evo)

        # Action 400 = digivolve hand[0] onto field[0] (400 + 0*15 + 0)
        game.decode_action(400, 1)

        assert len(p1.hand_cards) == 0
        assert len(p1.battle_area) == 1
        assert p1.battle_area[0].level == 4
        assert len(p1.battle_area[0].card_sources) == 2

    def test_decode_digivolve_breeding(self):
        game = setup_game_at_phase(GamePhase.Main, memory=10)
        p1 = game.player1

        egg = make_card("BT14-001", "Egg", kind=CardKind.DigiEgg, level=2,
                        colors=[CardColor.Red], owner=p1)
        p1.breeding_area = Permanent([egg])

        evo = make_card("BT14-003", "Rookie", dp=3000, level=3,
                        colors=[CardColor.Red], owner=p1,
                        evo_costs=[EvoCost(CardColor.Red, 2, 0)])
        p1.hand_cards.append(evo)

        # 400 + hand[0]*15 + breeding[12]
        game.decode_action(412, 1)

        assert p1.breeding_area is not None
        assert p1.breeding_area.level == 3
        assert len(p1.hand_cards) == 0
        assert len(p1.breeding_area.card_sources) == 2

    def test_decode_hatch(self):
        game = setup_game_at_phase(GamePhase.Breeding, memory=5)
        p1 = game.player1

        egg = make_card("BT14-001", "DigiEgg", kind=CardKind.DigiEgg, level=2, owner=p1)
        p1.digitama_library_cards.append(egg)

        game.decode_action(60, 1)
        assert p1.breeding_area is not None
        assert game.current_phase == GamePhase.Main

    def test_decode_move(self):
        game = setup_game_at_phase(GamePhase.Breeding, memory=5)
        p1 = game.player1

        card = make_card("BT14-003", "Rookie", level=3, dp=3000, owner=p1)
        p1.breeding_area = Permanent([card])

        game.decode_action(61, 1)
        assert p1.breeding_area is None
        assert len(p1.battle_area) == 1
        assert game.current_phase == GamePhase.Main

    def test_decode_move_then_hatch_is_not_allowed_same_turn(self):
        game = setup_game_at_phase(GamePhase.Breeding, memory=5)
        p1 = game.player1
        p1.digitama_library_cards.append(
            make_card("BT14-001", "Egg", kind=CardKind.DigiEgg, level=2, owner=p1)
        )
        p1.breeding_area = Permanent([make_card("BT14-003", "Rookie", level=3, owner=p1)])

        game.decode_action(61, 1)
        egg_count_after_move = len(p1.digitama_library_cards)
        game.decode_action(60, 1)

        assert game.current_phase == GamePhase.Main
        assert p1.breeding_area is None
        assert len(p1.battle_area) == 1
        assert len(p1.digitama_library_cards) == egg_count_after_move

    def test_decode_breeding_pass(self):
        game = setup_game_at_phase(GamePhase.Breeding, memory=5)
        game.decode_action(62, 1)
        assert game.current_phase == GamePhase.Main

    def test_decode_second_attacker(self):
        """Action for second attacker uses correct formula."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1, p2 = game.player1, game.player2

        a0 = make_card("BT14-001", dp=5000, level=4, owner=p1)
        a1 = make_card("BT14-002", dp=6000, level=4, owner=p1)
        p1.battle_area.append(Permanent([a0]))
        p1.battle_area.append(Permanent([a1]))

        p2.security_cards.append(make_card("BT14-003", dp=1000, owner=p2))
        p2.security_cards.append(make_card("BT14-010", dp=1000, owner=p2))

        # Attacker 1, security: 100 + 1*15 + 12 = 127
        game.decode_action(127, 1)

        assert not p1.battle_area[0].is_suspended  # attacker 0 untouched
        assert p1.battle_area[1].is_suspended       # attacker 1 attacked
        assert len(p2.security_cards) == 1


# ─── Integration: Full Round-Trip ────────────────────────────────────

class TestTensorMaskDecoderRoundTrip:
    def test_mask_actions_are_valid_decoder_inputs(self):
        """Every action flagged valid in the mask should be decodable."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        p1 = game.player1

        p1.hand_cards.append(make_card("BT14-001", play_cost=3, dp=5000, level=4, owner=p1))
        p1.battle_area.append(Permanent([make_card("BT14-002", dp=6000, level=4, owner=p1)]))

        mask = game.get_action_mask(1)
        valid_actions = [i for i, v in enumerate(mask) if v > 0.5]

        assert len(valid_actions) > 0
        assert 62 in valid_actions  # pass always valid

        # Each valid action should be decodable without error
        for action_id in valid_actions:
            g = setup_game_at_phase(GamePhase.Main, memory=5)
            g.player1.hand_cards.append(make_card("BT14-001", play_cost=3, dp=5000, level=4, owner=g.player1))
            g.player1.battle_area.append(Permanent([make_card("BT14-002", dp=6000, level=4, owner=g.player1)]))
            g.decode_action(action_id, 1)  # should not raise

    def test_tensor_changes_after_action(self):
        """Tensor should reflect state changes after actions."""
        game = setup_game_at_phase(GamePhase.Main, memory=10)
        p1 = game.player1

        card = make_card("BT14-001", play_cost=3, dp=5000, level=4, owner=p1)
        p1.hand_cards.append(card)

        t_before = game.get_board_state_tensor(1)
        assert t_before[754] != 0.0  # hand has a card

        game.decode_action(0, 1)  # play hand[0]

        t_after = game.get_board_state_tensor(1)
        assert t_after[754] == 0.0   # hand now empty
        assert t_after[10] != 0.0    # field has the card

    def test_mask_zero_after_action_depletes(self):
        """After playing the only hand card, play mask should zero out."""
        game = setup_game_at_phase(GamePhase.Main, memory=10)
        p1 = game.player1
        p1.hand_cards.append(make_card("BT14-001", play_cost=3, owner=p1))

        mask_before = game.get_action_mask(1)
        assert mask_before[0] == 1.0

        game.decode_action(0, 1)

        mask_after = game.get_action_mask(1)
        assert mask_after[0] == 0.0
