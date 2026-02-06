"""Tests for digivolution validation (evo_costs) and ACE Overflow.

Covers:
  A. can_digivolve() unit tests (8 tests)
  B. Action mask integration (5 tests)
  C. ACE properties on CEntity_Base (4 tests)
  D. ACE Overflow enforcement in Player (5 tests)
  E. Backward compatibility (3 tests)
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import (
    Game, ACTION_SPACE_SIZE, FIELD_SLOTS,
)
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming,
)
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.evo_cost import EvoCost
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect
from digimon_gym.engine.validation.digivolve_validator import can_digivolve


# ─── Helpers ─────────────────────────────────────────────────────────

def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None,
              evo_costs=None):
    """Create a CardSource with given attributes and optional evo_costs."""
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


def make_ace_card(card_id="ACE-001", name="AceMon", dp=8000, level=5,
                  colors=None, owner=None, overflow_cost=3):
    """Create a CardSource that is an ACE card with Overflow."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Digimon
    entity.dp = dp
    entity.level = level
    entity.play_cost = 4
    entity.card_colors = colors or [CardColor.Red]
    entity.inherited_effect_description_eng = (
        f"Ace Overflow ＜-{overflow_cost}＞ (As this card moves from the field "
        "or under a card to an area other than those, lose "
        f"{overflow_cost} memory.)"
    )
    entity.evo_costs = [EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


class BlastDigivolveEffect(ICardEffect):
    """Mock effect that grants Blast Digivolve (counter)."""
    def __init__(self):
        super().__init__()
        self._is_blast_digivolve = True
        self.is_counter_effect = True
        self.is_inherited_effect = False
        self.timing = EffectTiming.NoTiming


class MockCardSourceWithEffects(CardSource):
    """CardSource that returns custom effects instead of querying CardDatabase."""
    def __init__(self):
        super().__init__()
        self._mock_effects = []

    def effect_list(self, timing):
        return self._mock_effects


def make_blast_digivolve_card(card_id="BLAST-001", name="BlastMon", dp=8000,
                               level=5, colors=None, owner=None):
    """Create a MockCardSource with Blast Digivolve effect and proper evo_costs."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Digimon
    entity.dp = dp
    entity.level = level
    entity.play_cost = 7
    entity.card_colors = colors or [CardColor.Red]
    # ACE cards have evo_costs — they can digivolve normally too
    entity.evo_costs = [EvoCost(
        card_color=colors[0] if colors else CardColor.Red,
        level=level - 1,
        memory_cost=3,
    )]
    entity.inherited_effect_description_eng = (
        "Ace Overflow ＜-3＞ (As this card moves from the field "
        "or under a card to an area other than those, lose 3 memory.)"
    )
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)
    cs._mock_effects = [BlastDigivolveEffect()]
    return cs


def setup_game_at_phase(phase: GamePhase, memory: int = 5) -> Game:
    """Create a Game positioned at the given phase."""
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
        "TEST-001", "TEST-002", "TEST-003", "ACE-001", "BLAST-001",
        "FIELD-001", "EVO-001", "BASE-001",
    ])
    yield
    CardRegistry.reset()


# ═══════════════════════════════════════════════════════════════════════
# A. can_digivolve() Unit Tests (8 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestCanDigivolve:
    def test_basic_valid_digivolve(self):
        """Red Lv5 with evo_costs [Red, Lv4] can digivolve onto Red Lv4."""
        evo_card = make_card(
            name="RedLv5", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        base_card = make_card(name="RedLv4", level=4, colors=[CardColor.Red])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is True

    def test_wrong_level(self):
        """Red Lv5 with evo_costs [Red, Lv4] cannot digivolve onto Red Lv3."""
        evo_card = make_card(
            name="RedLv5", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        base_card = make_card(name="RedLv3", level=3, colors=[CardColor.Red])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is False

    def test_wrong_color(self):
        """Red Lv5 with evo_costs [Red, Lv4] cannot digivolve onto Blue Lv4."""
        evo_card = make_card(
            name="RedLv5", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        base_card = make_card(name="BlueLv4", level=4, colors=[CardColor.Blue])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is False

    def test_multicolor_base_matches(self):
        """Red evo_costs can digivolve onto Red/Blue multi-color base."""
        evo_card = make_card(
            name="RedLv5", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        base_card = make_card(
            name="RedBlueLv4", level=4,
            colors=[CardColor.Red, CardColor.Blue],
        )
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is True

    def test_empty_evo_costs_cannot_digivolve(self):
        """Card with empty evo_costs cannot digivolve normally."""
        evo_card = make_card(
            name="NoEvoCosts", level=5, colors=[CardColor.Red],
            evo_costs=[],  # Explicitly empty
        )
        base_card = make_card(name="RedLv4", level=4, colors=[CardColor.Red])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is False

    def test_non_digimon_cannot_digivolve(self):
        """Option cards cannot digivolve."""
        evo_card = make_card(
            name="OptionCard", kind=CardKind.Option, level=0,
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=2)],
        )
        base_card = make_card(name="RedLv4", level=4, colors=[CardColor.Red])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is False

    def test_empty_permanent_no_top_card(self):
        """Cannot digivolve onto a permanent with no top card."""
        evo_card = make_card(
            name="RedLv5", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
        )
        empty_perm = Permanent([])  # No cards in stack

        assert can_digivolve(evo_card, empty_perm) is False

    def test_multiple_evo_costs_one_matches(self):
        """Card with multiple evo_costs — one matches, should succeed."""
        evo_card = make_card(
            name="MultiEvo", level=5, colors=[CardColor.Red, CardColor.Blue],
            evo_costs=[
                EvoCost(card_color=CardColor.Red, level=4, memory_cost=3),
                EvoCost(card_color=CardColor.Blue, level=4, memory_cost=2),
            ],
        )
        base_card = make_card(name="BlueLv4", level=4, colors=[CardColor.Blue])
        base_perm = Permanent([base_card])

        assert can_digivolve(evo_card, base_perm) is True


# ═══════════════════════════════════════════════════════════════════════
# B. Action Mask Integration (5 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDigivolveMask:
    def test_main_mask_uses_evo_costs(self):
        """Main phase mask uses evo_costs for digivolution validation."""
        game = setup_game_at_phase(GamePhase.Main)

        # Hand: Red Lv5 with evo_costs [Red, Lv4, 3mem]
        evo_card = make_card(
            card_id="EVO-001", name="EvoMon", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
            owner=game.player1,
        )
        game.player1.hand_cards.append(evo_card)

        # Field: Red Lv4 digimon
        base_card = make_card(
            card_id="BASE-001", name="BaseMon", level=4, colors=[CardColor.Red],
            owner=game.player1,
        )
        base_perm = Permanent([base_card])
        game.player1.battle_area.append(base_perm)

        mask = game.get_action_mask(1)
        # hand=0, field=0 → action 400 + 0*15 + 0 = 400
        assert mask[400] == 1.0

    def test_main_mask_rejects_empty_evo_costs(self):
        """Card with no evo_costs should NOT appear in digivolve mask."""
        game = setup_game_at_phase(GamePhase.Main)

        # Hand: Lv5 digimon with NO evo_costs
        no_evo_card = make_card(
            card_id="EVO-001", name="NoEvoMon", level=5, colors=[CardColor.Red],
            evo_costs=[],
            owner=game.player1,
        )
        game.player1.hand_cards.append(no_evo_card)

        # Field: Red Lv4 digimon
        base_card = make_card(
            card_id="BASE-001", name="BaseMon", level=4, colors=[CardColor.Red],
            owner=game.player1,
        )
        base_perm = Permanent([base_card])
        game.player1.battle_area.append(base_perm)

        mask = game.get_action_mask(1)
        # Should NOT be valid
        assert mask[400] == 0.0

    def test_main_mask_rejects_wrong_evo_color(self):
        """Evo_costs requiring Red should not match Blue base."""
        game = setup_game_at_phase(GamePhase.Main)

        # Hand: Red Lv5 with evo_costs requiring Red base
        evo_card = make_card(
            card_id="EVO-001", name="RedEvoMon", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
            owner=game.player1,
        )
        game.player1.hand_cards.append(evo_card)

        # Field: Blue Lv4 (wrong color for evo requirement)
        base_card = make_card(
            card_id="BASE-001", name="BlueMon", level=4, colors=[CardColor.Blue],
            owner=game.player1,
        )
        base_perm = Permanent([base_card])
        game.player1.battle_area.append(base_perm)

        mask = game.get_action_mask(1)
        assert mask[400] == 0.0

    def test_counter_mask_uses_evo_costs(self):
        """CounterTiming mask validates blast digivolve using evo_costs."""
        game = setup_game_at_phase(GamePhase.Main)

        # Set up a pending attack so we can get to CounterTiming
        attacker_card = make_card(
            name="Attacker", dp=7000, level=4, owner=game.player1,
        )
        attacker = Permanent([attacker_card])
        game.player1.battle_area.append(attacker)

        # Opponent field: Red Lv4
        field_card = make_card(
            card_id="FIELD-001", name="FieldMon", dp=5000, level=4,
            colors=[CardColor.Red], owner=game.player2,
        )
        field_perm = Permanent([field_card])
        game.player2.battle_area.append(field_perm)

        # Opponent hand: blast digivolve card with proper evo_costs
        blast_card = make_blast_digivolve_card(
            card_id="BLAST-001", name="BlastMon", dp=8000, level=5,
            colors=[CardColor.Red], owner=game.player2,
        )
        game.player2.hand_cards.append(blast_card)

        # Trigger attack → should reach CounterTiming
        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.CounterTiming

        mask = game.get_action_mask(game.player2.player_id)
        # Blast card at hand=0, field=0 → action 400
        assert mask[400] == 1.0
        assert mask[62] == 1.0  # Pass always valid

    def test_opponent_has_counter_uses_evo_costs(self):
        """_opponent_has_counter_options uses evo_costs validation."""
        game = setup_game_at_phase(GamePhase.Main)

        # Opponent field: Blue Lv4 (NOT Red)
        field_card = make_card(
            card_id="FIELD-001", name="BlueMon", dp=5000, level=4,
            colors=[CardColor.Blue], owner=game.player2,
        )
        field_perm = Permanent([field_card])
        game.player2.battle_area.append(field_perm)

        # Opponent hand: blast digivolve card requiring Red base
        blast_card = make_blast_digivolve_card(
            card_id="BLAST-001", name="RedBlast", dp=8000, level=5,
            colors=[CardColor.Red], owner=game.player2,
        )
        game.player2.hand_cards.append(blast_card)

        # Should return False — Red evo_cost doesn't match Blue base
        assert game._opponent_has_counter_options() is False


# ═══════════════════════════════════════════════════════════════════════
# C. ACE Properties on CEntity_Base (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestAceProperties:
    def test_is_ace_true_with_overflow_text(self):
        """Card with 'Ace Overflow' in inherited effect is ACE."""
        entity = CEntity_Base()
        entity.inherited_effect_description_eng = (
            "Ace Overflow ＜-3＞ (As this card moves from the field "
            "or under a card to an area other than those, lose 3 memory.)"
        )
        assert entity.is_ace is True

    def test_is_ace_false_regular_card(self):
        """Regular card without Ace Overflow text is not ACE."""
        entity = CEntity_Base()
        entity.inherited_effect_description_eng = "[Your Turn] This Digimon gets +2000 DP."
        assert entity.is_ace is False

    def test_ace_overflow_cost_parses_unicode(self):
        """Correctly parses Ace Overflow ＜-3＞ with Unicode brackets."""
        entity = CEntity_Base()
        entity.inherited_effect_description_eng = (
            "Ace Overflow ＜-3＞ (As this card moves from the field "
            "or under a card to an area other than those, lose 3 memory.)"
        )
        assert entity.ace_overflow_cost == 3

    def test_ace_overflow_cost_zero_for_non_ace(self):
        """Non-ACE card returns 0 for ace_overflow_cost."""
        entity = CEntity_Base()
        entity.inherited_effect_description_eng = ""
        assert entity.ace_overflow_cost == 0


# ═══════════════════════════════════════════════════════════════════════
# D. ACE Overflow Enforcement (5 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestAceOverflowEnforcement:
    def test_delete_ace_triggers_lose_memory(self):
        """Deleting a permanent with an ACE card triggers memory loss."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        ace_card = make_ace_card(owner=game.player1, overflow_cost=3)
        perm = Permanent([ace_card])
        game.player1.battle_area.append(perm)

        game.player1.delete_permanent(perm)

        # Player1 is turn_player, lose_memory(3) → memory -= 3
        assert game.memory == 2  # 5 - 3

    def test_delete_non_ace_no_memory_change(self):
        """Deleting a regular permanent doesn't change memory."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        card = make_card(name="NormalMon", owner=game.player1)
        perm = Permanent([card])
        game.player1.battle_area.append(perm)

        game.player1.delete_permanent(perm)

        assert game.memory == 5  # Unchanged

    def test_ace_in_digivolution_stack_triggers(self):
        """ACE card in digivolution stack (under top card) triggers on deletion."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        # Bottom card: ACE
        ace_card = make_ace_card(name="AceBottom", owner=game.player1)
        # Top card: normal
        top_card = make_card(name="TopMon", level=6, owner=game.player1)
        perm = Permanent([ace_card, top_card])
        game.player1.battle_area.append(perm)

        game.player1.delete_permanent(perm)

        # ACE in the stack triggers overflow
        assert game.memory == 2  # 5 - 3

    def test_bounce_under_cards_trigger_ace(self):
        """Bouncing: digivolution cards going to trash trigger ACE, top card does not."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)
        # Bottom card: ACE
        ace_card = make_ace_card(name="AceBottom", owner=game.player1)
        # Top card: normal
        top_card = make_card(name="TopMon", level=6, owner=game.player1)
        perm = Permanent([ace_card, top_card])
        game.player1.battle_area.append(perm)

        game.player1.bounce_permanent_to_hand(perm)

        # ACE under-card triggers overflow
        assert game.memory == 2  # 5 - 3
        # Top card should be in hand
        assert top_card in game.player1.hand_cards
        # ACE card should be in trash
        assert ace_card in game.player1.trash_cards

    def test_battle_deletion_with_ace_adjusts_memory(self):
        """Battle deletion of an ACE permanent adjusts memory correctly."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)

        # Give players security and library for attack resolution
        for _ in range(5):
            game.player1.security_cards.append(make_card(owner=game.player1))
            game.player2.security_cards.append(make_card(owner=game.player2))
        for _ in range(10):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        # Attacker: 10000 DP
        attacker_card = make_card(
            name="StrongAttacker", dp=10000, level=5, owner=game.player1,
        )
        attacker = Permanent([attacker_card])
        game.player1.battle_area.append(attacker)

        # Target: ACE card with 8000 DP (will lose the battle)
        ace_target = make_ace_card(
            name="AceTarget", dp=8000, level=5,
            owner=game.player2, overflow_cost=3,
        )
        ace_perm = Permanent([ace_target])
        game.player2.battle_area.append(ace_perm)

        mem_before = game.memory
        game.resolve_attack(attacker, ace_perm)

        # Opponent's ACE was deleted → opponent loses 3 memory
        # lose_memory on opponent (player2) when player1 is turn_player:
        #   add_memory(-3) → memory -= (-3) → memory += 3
        # So memory should go UP by 3 (opponent's loss benefits turn player)
        assert game.memory == mem_before + 3


# ═══════════════════════════════════════════════════════════════════════
# E. Backward Compatibility (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestBackwardCompatibility:
    def test_valid_evo_costs_still_work(self):
        """Cards with proper evo_costs can still digivolve (happy path)."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)

        # Hand card with evo_costs
        evo_card = make_card(
            card_id="EVO-001", name="EvoMon", level=5, colors=[CardColor.Red],
            evo_costs=[EvoCost(card_color=CardColor.Red, level=4, memory_cost=3)],
            owner=game.player1,
        )
        game.player1.hand_cards.append(evo_card)

        # Field permanent
        base_card = make_card(
            card_id="BASE-001", name="BaseMon", level=4, colors=[CardColor.Red],
            owner=game.player1,
        )
        base_perm = Permanent([base_card])
        game.player1.battle_area.append(base_perm)

        # Should have digivolve in mask
        mask = game.get_action_mask(1)
        assert mask[400] == 1.0

        # Execute digivolve
        game.decode_action(400, 1)

        # Card should be stacked on permanent
        assert evo_card in base_perm.card_sources
        assert evo_card not in game.player1.hand_cards

    def test_attack_flow_unchanged_without_ace(self):
        """Attack flow unchanged for non-ACE cards."""
        game = setup_game_at_phase(GamePhase.Main, memory=5)

        for _ in range(5):
            game.player1.security_cards.append(make_card(owner=game.player1))
            game.player2.security_cards.append(make_card(owner=game.player2))
        for _ in range(10):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        attacker_card = make_card(name="Attacker", dp=7000, owner=game.player1)
        attacker = Permanent([attacker_card])
        game.player1.battle_area.append(attacker)

        target_card = make_card(name="Target", dp=3000, owner=game.player2)
        target = Permanent([target_card])
        game.player2.battle_area.append(target)

        mem_before = game.memory
        game.resolve_attack(attacker, target)

        # No ACE → no memory change from overflow
        assert game.memory == mem_before
        assert target not in game.player2.battle_area

    def test_default_evo_costs_empty_on_entity(self):
        """CEntity_Base defaults to empty evo_costs list."""
        entity = CEntity_Base()
        assert entity.evo_costs == []
        # Card with no evo_costs cannot digivolve
        cs = CardSource()
        cs.set_base_data(entity, None)
        perm = Permanent([make_card(level=3)])
        assert can_digivolve(cs, perm) is False
