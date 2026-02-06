"""Tests for phase decoders: Block, Counter, Selection, and attack resolution phases.

Covers:
  A. Permanent.can_block() (6 tests)
  B. Backward compatibility — attack without blockers/counters (4 tests)
  C. BlockTiming decoder (8 tests)
  D. CounterTiming decoder (7 tests)
  E. Selection framework (5 tests)
  F. Integration (3 tests)
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import (
    Game, PendingAttack, PendingSelection,
    ACTION_SPACE_SIZE, FIELD_SLOTS,
)
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming, AttackResolution,
)
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.evo_cost import EvoCost
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect


# ─── Helpers ─────────────────────────────────────────────────────────

def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None):
    """Create a CardSource with given attributes."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


class BlockerEffect(ICardEffect):
    """Mock effect that grants <Blocker>."""
    def __init__(self, inherited=False):
        super().__init__()
        self._is_blocker = True
        self.is_inherited_effect = inherited
        self.timing = EffectTiming.NoTiming


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


def make_blocker_card(card_id="BLOCKER-001", name="BlockerMon", dp=6000,
                      level=4, colors=None, owner=None, inherited=False):
    """Create a CardSource that has the <Blocker> keyword."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Digimon
    entity.dp = dp
    entity.level = level
    entity.play_cost = 5
    entity.card_colors = colors or [CardColor.Red]
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)
    cs._mock_effects = [BlockerEffect(inherited=inherited)]
    return cs


def make_blast_digivolve_card(card_id="BLAST-001", name="BlastMon", dp=8000,
                               level=5, colors=None, owner=None):
    """Create a CardSource with Blast Digivolve and proper evo_costs."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Digimon
    entity.dp = dp
    entity.level = level
    entity.play_cost = 7
    entity.card_colors = colors or [CardColor.Red]
    # ACE/Blast cards have evo_costs — required for can_digivolve() validation
    primary_color = colors[0] if colors else CardColor.Red
    entity.evo_costs = [EvoCost(card_color=primary_color, level=level - 1, memory_cost=3)]
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)
    cs._mock_effects = [BlastDigivolveEffect()]
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


def setup_attack_game():
    """Create a Game with an attacker on P1's field and set at Main phase.

    Returns (game, attacker_permanent).
    """
    game = setup_game_at_phase(GamePhase.Main, memory=5)
    # Give both players some security and library cards
    for _ in range(5):
        game.player1.security_cards.append(make_card(name="P1Sec", owner=game.player1))
        game.player2.security_cards.append(make_card(name="P2Sec", owner=game.player2))
    for _ in range(10):
        game.player1.library_cards.append(make_card(name="P1Lib", owner=game.player1))
        game.player2.library_cards.append(make_card(name="P2Lib", owner=game.player2))

    attacker_card = make_card(card_id="ATK-001", name="Attacker", dp=7000,
                              level=4, owner=game.player1)
    attacker = Permanent([attacker_card])
    game.player1.battle_area.append(attacker)
    return game, attacker


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test to ensure isolation."""
    CardRegistry.reset()
    CardRegistry.initialize_from_list([
        "TEST-001", "BLOCKER-001", "BLAST-001", "ATK-001", "TARGET-001",
    ])
    yield
    CardRegistry.reset()


# ═══════════════════════════════════════════════════════════════════════
# A. Permanent.can_block() Tests (6 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestCanBlock:
    def test_blocker_effect_returns_true(self):
        """Digimon with <Blocker> can block."""
        blocker_card = make_blocker_card()
        perm = Permanent([blocker_card])
        attacker_card = make_card(name="Attacker", dp=5000)
        attacker = Permanent([attacker_card])
        assert perm.can_block(attacker) is True

    def test_normal_digimon_returns_false(self):
        """Digimon without <Blocker> cannot block."""
        card = make_card(name="NormalMon")
        perm = Permanent([card])
        attacker = Permanent([make_card(name="Attacker")])
        assert perm.can_block(attacker) is False

    def test_suspended_blocker_returns_false(self):
        """Suspended Digimon with <Blocker> cannot block."""
        blocker_card = make_blocker_card()
        perm = Permanent([blocker_card])
        perm.suspend()
        attacker = Permanent([make_card(name="Attacker")])
        assert perm.can_block(attacker) is False

    def test_tamer_with_blocker_flag_returns_false(self):
        """Tamers cannot block even if they somehow have a blocker flag."""
        entity = CEntity_Base()
        entity.card_id = "TAMER-001"
        entity.card_name_eng = "TamerBlocker"
        entity.card_kind = CardKind.Tamer
        entity.dp = 0
        entity.level = 0
        entity.play_cost = 3
        entity.card_colors = [CardColor.Red]
        cs = MockCardSourceWithEffects()
        cs.set_base_data(entity, None)
        cs._mock_effects = [BlockerEffect(inherited=False)]
        perm = Permanent([cs])
        attacker = Permanent([make_card(name="Attacker")])
        assert perm.can_block(attacker) is False

    def test_inherited_blocker_works(self):
        """Inherited <Blocker> from digivolution source works."""
        # Bottom card: has inherited blocker
        bottom = make_blocker_card(card_id="BOTTOM-001", name="BottomMon",
                                   inherited=True)
        # Top card: normal digimon (no blocker)
        top = make_card(card_id="TOP-001", name="TopMon", level=5, dp=8000)
        perm = Permanent([bottom, top])
        attacker = Permanent([make_card(name="Attacker")])
        assert perm.can_block(attacker) is True

    def test_top_card_non_inherited_blocker_works(self):
        """Non-inherited <Blocker> from top card works."""
        # Bottom card: normal
        bottom = make_card(card_id="BOTTOM-002", name="BottomMon", level=3)
        # Top card: has non-inherited blocker
        top = make_blocker_card(card_id="TOP-002", name="TopBlocker", level=4,
                                inherited=False)
        perm = Permanent([bottom, top])
        attacker = Permanent([make_card(name="Attacker")])
        assert perm.can_block(attacker) is True


# ═══════════════════════════════════════════════════════════════════════
# B. Backward Compatibility Tests (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestBackwardCompatibility:
    def test_attack_without_blockers_resolves_immediately(self):
        """Attack against player with no blockers goes straight to resolution."""
        game, attacker = setup_attack_game()

        # Attack player (security attack)
        game.resolve_attack(attacker, game.player2)

        # Should be back in Main phase (or End if memory crossed)
        assert game.current_phase in (GamePhase.Main, GamePhase.End,
                                       GamePhase.Start, GamePhase.Breeding)
        assert game.pending_attack is None
        assert game.active_player is None

    def test_attack_vs_digimon_resolves_immediately(self):
        """Attack against digimon with no blockers resolves immediately."""
        game, attacker = setup_attack_game()
        target_card = make_card(card_id="TARGET-001", name="Target", dp=3000,
                                owner=game.player2)
        target = Permanent([target_card])
        game.player2.battle_area.append(target)

        game.resolve_attack(attacker, target)

        # Target should be deleted (7000 > 3000)
        assert target not in game.player2.battle_area
        assert game.pending_attack is None
        assert game.active_player is None

    def test_pending_attack_cleaned_up_after_resolution(self):
        """PendingAttack and active_player are None after battle resolution."""
        game, attacker = setup_attack_game()

        game.resolve_attack(attacker, game.player2)

        assert game.pending_attack is None
        assert game.active_player is None

    def test_attacker_is_suspended_after_attack(self):
        """Attacker should be suspended after declaring an attack."""
        game, attacker = setup_attack_game()
        assert not attacker.is_suspended

        game.resolve_attack(attacker, game.player2)

        assert attacker.is_suspended


# ═══════════════════════════════════════════════════════════════════════
# C. BlockTiming Tests (8 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestBlockTiming:
    def _setup_with_blocker(self):
        """Set up game where opponent has a blocker on field."""
        game, attacker = setup_attack_game()
        blocker_card = make_blocker_card(name="OpponentBlocker", dp=6000,
                                         owner=game.player2)
        blocker = Permanent([blocker_card])
        game.player2.battle_area.append(blocker)
        return game, attacker, blocker

    def test_attack_with_blocker_enters_block_timing(self):
        """When opponent has a blocker, attack enters BlockTiming phase."""
        game, attacker, blocker = self._setup_with_blocker()

        game.resolve_attack(attacker, game.player2)

        assert game.current_phase == GamePhase.BlockTiming
        assert game.pending_attack is not None
        assert game.pending_attack.attacker is attacker

    def test_active_player_set_to_opponent_during_block(self):
        """During BlockTiming, active_player is the opponent (defender)."""
        game, attacker, blocker = self._setup_with_blocker()

        game.resolve_attack(attacker, game.player2)

        assert game.active_player is game.player2
        assert game.current_player_id == game.player2.player_id

    def test_block_mask_includes_valid_blockers(self):
        """Action mask during BlockTiming includes valid blocker slots."""
        game, attacker, blocker = self._setup_with_blocker()

        game.resolve_attack(attacker, game.player2)
        mask = game.get_action_mask(game.player2.player_id)

        # Pass is always valid
        assert mask[62] == 1.0
        # Blocker is at index 0 in opponent's battle_area → action 100
        assert mask[100] == 1.0

    def test_block_mask_excludes_suspended_blockers(self):
        """Suspended blockers should not appear in the block mask."""
        game, attacker, blocker = self._setup_with_blocker()
        blocker.suspend()  # Can't block while suspended

        game.resolve_attack(attacker, game.player2)

        # No blockers available → should skip to counter/resolve
        # (since no valid blockers, attack shouldn't enter BlockTiming)
        assert game.current_phase != GamePhase.BlockTiming

    def test_decline_block_proceeds_to_resolve(self):
        """Action 62 (pass) during BlockTiming declines and resolves battle."""
        game, attacker, blocker = self._setup_with_blocker()

        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.BlockTiming

        # Decline block
        game.decode_action(62, game.player2.player_id)

        # Should resolve (no counter options either for plain cards)
        assert game.pending_attack is None
        assert game.active_player is None

    def test_selecting_blocker_redirects_target(self):
        """Selecting a blocker changes the effective target."""
        game, attacker, blocker = self._setup_with_blocker()

        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.BlockTiming

        # Block with blocker at index 0
        game.decode_action(100, game.player2.player_id)

        # Blocker should be suspended
        assert blocker.is_suspended

    def test_blocker_suspends_on_block(self):
        """The blocker is suspended when it blocks."""
        game, attacker, blocker = self._setup_with_blocker()
        assert not blocker.is_suspended

        game.resolve_attack(attacker, game.player2)
        game.decode_action(100, game.player2.player_id)

        assert blocker.is_suspended

    def test_battle_uses_blocker_dp_for_comparison(self):
        """When blocked, battle resolution compares against blocker's DP."""
        game, attacker, blocker = self._setup_with_blocker()
        # Attacker: 7000 DP, Blocker: 6000 DP → blocker should be deleted
        assert attacker.dp == 7000
        assert blocker.dp == 6000

        game.resolve_attack(attacker, game.player2)
        game.decode_action(100, game.player2.player_id)

        # Blocker (6000) < Attacker (7000) → blocker should be deleted
        assert blocker not in game.player2.battle_area


# ═══════════════════════════════════════════════════════════════════════
# D. CounterTiming Tests (7 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestCounterTiming:
    def _setup_with_counter(self):
        """Set up game where opponent has a blast digivolve card in hand
        and a valid target on field."""
        game, attacker = setup_attack_game()
        # Field target: level 4 Red digimon
        field_card = make_card(card_id="FIELD-001", name="FieldMon", dp=5000,
                               level=4, colors=[CardColor.Red], owner=game.player2)
        field_perm = Permanent([field_card])
        game.player2.battle_area.append(field_perm)

        # Hand card: level 5 Red blast digivolve
        blast_card = make_blast_digivolve_card(
            card_id="BLAST-001", name="BlastMon", dp=8000,
            level=5, colors=[CardColor.Red], owner=game.player2,
        )
        game.player2.hand_cards.append(blast_card)

        return game, attacker, field_perm, blast_card

    def test_counter_timing_reached_after_no_block(self):
        """CounterTiming is reached when there are counter options."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()

        game.resolve_attack(attacker, game.player2)

        assert game.current_phase == GamePhase.CounterTiming
        assert game.active_player is game.player2
        assert game.pending_attack is not None

    def test_counter_mask_shows_blast_digivolve_options(self):
        """Counter timing mask shows valid blast digivolve actions."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()

        game.resolve_attack(attacker, game.player2)
        mask = game.get_action_mask(game.player2.player_id)

        # Pass is always valid
        assert mask[62] == 1.0
        # Blast card is at hand index 0, field target at index 0
        # Action = 400 + hand*15 + field = 400 + 0*15 + 0 = 400
        assert mask[400] == 1.0

    def test_decline_counter_resolves_battle(self):
        """Declining counter (action 62) resolves the battle."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()

        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.CounterTiming

        game.decode_action(62, game.player2.player_id)

        assert game.pending_attack is None
        assert game.active_player is None

    def test_blast_digivolve_moves_card_from_hand(self):
        """Blast digivolve removes the card from hand."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()

        game.resolve_attack(attacker, game.player2)
        assert blast_card in game.player2.hand_cards

        # Blast digivolve: hand=0, field=0 → action 400
        game.decode_action(400, game.player2.player_id)

        assert blast_card not in game.player2.hand_cards

    def test_blast_digivolve_changes_dp(self):
        """Blast digivolve changes the field permanent's DP."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()
        assert field_perm.dp == 5000  # Before digivolve

        game.resolve_attack(attacker, game.player2)
        game.decode_action(400, game.player2.player_id)

        # After blast digivolve, top card is blast_card with 8000 DP
        assert field_perm.dp == 8000

    def test_blast_digivolve_affects_battle_outcome(self):
        """Blast digivolve can change the battle outcome by changing DP."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()
        # Attacker: 7000 DP, field: 5000 (without counter) → 8000 (with counter)

        game.resolve_attack(attacker, field_perm)

        # Should reach CounterTiming (opponent has blast digivolve)
        assert game.current_phase == GamePhase.CounterTiming

        # Blast digivolve → field becomes 8000 DP
        game.decode_action(400, game.player2.player_id)

        # Now attacker (7000) < field_perm (8000) → attacker should be deleted
        assert attacker not in game.player1.battle_area

    def test_state_cleaned_up_after_counter_resolution(self):
        """All interrupt state is cleaned up after counter + battle resolution."""
        game, attacker, field_perm, blast_card = self._setup_with_counter()

        game.resolve_attack(attacker, game.player2)
        game.decode_action(400, game.player2.player_id)

        assert game.pending_attack is None
        assert game.active_player is None
        assert game.pending_selection is None


# ═══════════════════════════════════════════════════════════════════════
# E. Selection Framework Tests (5 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestSelectionFramework:
    def test_request_selection_transitions_phase(self):
        """request_selection changes the game phase."""
        game = setup_game_at_phase(GamePhase.Main)

        callback_called = []
        game.request_selection(
            phase=GamePhase.SelectTarget,
            player=game.player1,
            callback=lambda idx: callback_called.append(idx),
            valid_indices=[100, 101, 102],
        )

        assert game.current_phase == GamePhase.SelectTarget
        assert game.pending_selection is not None
        assert game.active_player is game.player1

    def test_selection_callback_fires_on_valid_selection(self):
        """Callback fires when a valid selection is made."""
        game = setup_game_at_phase(GamePhase.Main)

        callback_called = []
        game.request_selection(
            phase=GamePhase.SelectTarget,
            player=game.player1,
            callback=lambda idx: callback_called.append(idx),
            valid_indices=[100, 101, 102],
        )

        game.decode_action(101, game.player1.player_id)

        assert callback_called == [101]

    def test_selection_restores_phase_after_completion(self):
        """Phase restores to previous phase after selection completes."""
        game = setup_game_at_phase(GamePhase.Main)

        game.request_selection(
            phase=GamePhase.SelectTarget,
            player=game.player1,
            callback=lambda idx: None,
            valid_indices=[100],
        )

        game.decode_action(100, game.player1.player_id)

        assert game.current_phase == GamePhase.Main
        assert game.pending_selection is None
        assert game.active_player is None

    def test_trash_selection_validates_index(self):
        """Trash selection decoder validates against trash size."""
        game = setup_game_at_phase(GamePhase.Main)
        # Add 3 cards to trash
        for i in range(3):
            game.player1.trash_cards.append(
                make_card(name=f"TrashCard{i}", owner=game.player1)
            )

        callback_called = []
        game.request_selection(
            phase=GamePhase.SelectTrash,
            player=game.player1,
            callback=lambda idx: callback_called.append(idx),
        )

        # Valid index
        game.decode_action(1, game.player1.player_id)
        assert callback_called == [1]

    def test_source_selection_decodes_correctly(self):
        """Source selection decoder correctly extracts field_idx and source_idx."""
        game = setup_game_at_phase(GamePhase.Main)
        # Add a permanent with 3 sources (digivolution stack)
        cards = [
            make_card(card_id="SRC-001", name="Src1", level=3, owner=game.player1),
            make_card(card_id="SRC-002", name="Src2", level=4, owner=game.player1),
            make_card(card_id="SRC-003", name="Src3", level=5, owner=game.player1),
        ]
        perm = Permanent(cards)
        game.player1.battle_area.append(perm)

        callback_called = []
        game.request_selection(
            phase=GamePhase.SelectSource,
            player=game.player1,
            callback=lambda idx: callback_called.append(idx),
        )

        # Action 2000 + field_idx*10 + source_idx = 2000 + 0*10 + 1 = 2001
        game.decode_action(2001, game.player1.player_id)
        assert callback_called == [2001]


# ═══════════════════════════════════════════════════════════════════════
# F. Integration Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestIntegration:
    def test_full_attack_block_counter_resolve_flow(self):
        """Full flow: attack → block → counter → resolve."""
        game, attacker = setup_attack_game()

        # Put a blocker on opponent's field
        blocker_card = make_blocker_card(name="Blocker", dp=6000,
                                         owner=game.player2)
        blocker = Permanent([blocker_card])
        game.player2.battle_area.append(blocker)

        # Put a blast digivolve target + card for opponent
        # (but blocker will take the hit, so counter won't change target outcome)
        field_card = make_card(card_id="FIELD-002", name="FieldMon2", dp=4000,
                               level=4, colors=[CardColor.Red], owner=game.player2)
        field_perm = Permanent([field_card])
        game.player2.battle_area.append(field_perm)

        blast_card = make_blast_digivolve_card(
            card_id="BLAST-002", name="BlastMon2", dp=9000,
            level=5, colors=[CardColor.Red], owner=game.player2,
        )
        game.player2.hand_cards.append(blast_card)

        # Start attack → should enter BlockTiming
        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.BlockTiming

        # Decline block → should enter CounterTiming
        game.decode_action(62, game.player2.player_id)
        assert game.current_phase == GamePhase.CounterTiming

        # Decline counter → resolve battle
        game.decode_action(62, game.player2.player_id)
        assert game.pending_attack is None
        assert game.active_player is None

    def test_mask_decoder_roundtrip_block_timing(self):
        """Valid mask actions can be decoded during BlockTiming."""
        game, attacker = setup_attack_game()
        blocker_card = make_blocker_card(name="Blocker", dp=6000,
                                         owner=game.player2)
        blocker = Permanent([blocker_card])
        game.player2.battle_area.append(blocker)

        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.BlockTiming

        mask = game.get_action_mask(game.player2.player_id)

        # All valid actions in mask should be decodable without error
        valid_actions = [i for i, v in enumerate(mask) if v == 1.0]
        assert len(valid_actions) >= 1  # At least pass (62)
        assert 62 in valid_actions

        # Decode one valid action (pass)
        game.decode_action(62, game.player2.player_id)
        # Should not crash and should progress the game state

    def test_mask_decoder_roundtrip_counter_timing(self):
        """Valid mask actions can be decoded during CounterTiming."""
        game, attacker = setup_attack_game()

        # Field target for blast digivolve
        field_card = make_card(card_id="FIELD-003", name="FieldMon3", dp=5000,
                               level=4, colors=[CardColor.Red], owner=game.player2)
        field_perm = Permanent([field_card])
        game.player2.battle_area.append(field_perm)

        blast_card = make_blast_digivolve_card(
            card_id="BLAST-003", name="BlastMon3", dp=8000,
            level=5, colors=[CardColor.Red], owner=game.player2,
        )
        game.player2.hand_cards.append(blast_card)

        game.resolve_attack(attacker, game.player2)
        assert game.current_phase == GamePhase.CounterTiming

        mask = game.get_action_mask(game.player2.player_id)

        valid_actions = [i for i, v in enumerate(mask) if v == 1.0]
        assert 62 in valid_actions  # Pass always valid
        assert 400 in valid_actions  # Blast digivolve option

        # Decode the blast digivolve
        game.decode_action(400, game.player2.player_id)

        # Should have resolved
        assert game.pending_attack is None
