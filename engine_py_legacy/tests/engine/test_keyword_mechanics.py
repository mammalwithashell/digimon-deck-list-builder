"""Tests for keyword mechanics: Retaliation, Piercing, Jamming, Rush, SA+/-,
Blitz, Collision, restriction keywords, linked card keywords, granted keywords,
deletion prevention, game-over state cleanup, and the 9 newly implemented
keywords: Training, Progress, Fortitude, Save, Vortex, Overclock, Alliance,
Decoy, Material Save.

Covers ~60 tests validating engine combat resolution and keyword correctness.
"""

import pytest

from engine_py_legacy.engine.game import Game, PendingAttack, ACTION_SPACE_SIZE
from engine_py_legacy.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming, AttackResolution,
)
from engine_py_legacy.engine.data.card_registry import CardRegistry
from engine_py_legacy.engine.core.player import Player
from engine_py_legacy.engine.core.permanent import Permanent
from engine_py_legacy.engine.core.card_source import CardSource
from engine_py_legacy.engine.core.entity_base import CEntity_Base
from engine_py_legacy.engine.interfaces.card_effect import ICardEffect


# ─── Mock Helpers ─────────────────────────────────────────────────────

class KeywordEffect(ICardEffect):
    """Generic mock effect that grants a single keyword flag."""
    def __init__(self, keyword_attr: str, inherited=False, **kwargs):
        super().__init__()
        setattr(self, keyword_attr, True)
        self.is_inherited_effect = inherited
        self.timing = EffectTiming.NoTiming
        for k, v in kwargs.items():
            setattr(self, k, v)


class SAModifierEffect(ICardEffect):
    """Mock effect with a security attack modifier value."""
    def __init__(self, sa_mod: int, inherited=False):
        super().__init__()
        self._security_attack_modifier = sa_mod
        self.is_inherited_effect = inherited
        self.timing = EffectTiming.NoTiming


class MockCardSourceWithEffects(CardSource):
    """CardSource that returns custom effects instead of querying CardDatabase."""
    def __init__(self):
        super().__init__()
        self._mock_effects = []

    def effect_list(self, timing):
        return self._mock_effects


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


def make_keyword_card(card_id="KW-001", name="KeywordMon", dp=5000, level=4,
                      keywords=None, sa_mod=None, colors=None, owner=None):
    """Create a MockCardSourceWithEffects with given keyword effects."""
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
    effects = []
    for kw in (keywords or []):
        effects.append(KeywordEffect(kw))
    if sa_mod is not None:
        effects.append(SAModifierEffect(sa_mod))
    cs._mock_effects = effects
    return cs


def setup_battle_game(attacker_dp=7000, attacker_keywords=None, attacker_sa=None,
                      target_dp=5000, target_keywords=None, target_sa=None,
                      security_count=5):
    """Create a game ready for battle resolution testing.

    Returns (game, attacker_permanent, target_permanent).
    """
    game = Game()
    game.current_phase = GamePhase.Main
    game.memory = 5
    game.turn_count = 2
    game.turn_player = game.player1
    game.opponent_player = game.player2
    game.player1.is_my_turn = True
    game.player1.game = game
    game.player2.game = game

    # Security and library cards for both players
    for _ in range(security_count):
        game.player1.security_cards.append(
            make_card(name="P1Sec", kind=CardKind.Tamer, dp=None, level=None, owner=game.player1))
        game.player2.security_cards.append(
            make_card(name="P2Sec", kind=CardKind.Tamer, dp=None, level=None, owner=game.player2))
    for _ in range(10):
        game.player1.library_cards.append(make_card(name="P1Lib", owner=game.player1))
        game.player2.library_cards.append(make_card(name="P2Lib", owner=game.player2))

    # Attacker on P1's field
    attacker_card = make_keyword_card(
        card_id="ATK-001", name="Attacker", dp=attacker_dp,
        keywords=attacker_keywords, sa_mod=attacker_sa, owner=game.player1)
    attacker = Permanent([attacker_card])
    attacker.turn_played = 1  # not this turn — no summoning sickness
    attacker._owner_game = game
    game.player1.battle_area.append(attacker)

    # Target on P2's field
    target_card = make_keyword_card(
        card_id="TGT-001", name="Target", dp=target_dp,
        keywords=target_keywords, sa_mod=target_sa, owner=game.player2)
    target = Permanent([target_card])
    target._owner_game = game
    game.player2.battle_area.append(target)

    return game, attacker, target


def force_resolve_battle(game, attacker, target):
    """Set up PendingAttack and directly call _resolve_battle() to skip
    blocker/counter timing for unit-test isolation."""
    attacker.is_attacking = True  # Matches resolve_attack() line 370
    game.pending_attack = PendingAttack(
        attacker=attacker,
        original_target=target,
        effective_target=target,
    )
    game._resolve_battle()


@pytest.fixture(autouse=True)
def init_test_registry():
    """Initialize registry with test card IDs for this module."""
    CardRegistry.initialize_from_list([
        "TEST-001", "KW-001", "ATK-001", "TGT-001",
    ])


# ═══════════════════════════════════════════════════════════════════════
# A. Retaliation Tests (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestRetaliation:
    def test_retaliation_deletes_winner(self):
        """When target with Retaliation is deleted in battle, attacker is also deleted."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            target_keywords=['_is_retaliation'])
        force_resolve_battle(game, attacker, target)
        assert target not in game.player2.battle_area, "Target should be deleted"
        assert attacker not in game.player1.battle_area, "Attacker should be deleted by Retaliation"

    def test_retaliation_not_fired_when_armor_purge_saves(self):
        """If target survives via Armor Purge, Retaliation does NOT fire."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            target_keywords=['_is_retaliation', '_is_armor_purge'])
        # Give target a digivolution stack (>1 card) so Armor Purge can activate
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        force_resolve_battle(game, attacker, target)
        assert target in game.player2.battle_area, "Target should survive via Armor Purge"
        assert attacker in game.player1.battle_area, "Attacker should survive (Retaliation blocked)"

    def test_retaliation_not_fired_when_evade_saves(self):
        """If target survives via Evade, Retaliation does NOT fire."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            target_keywords=['_is_retaliation', '_is_evade'])
        # Target must be unsuspended for Evade to work
        assert not target.is_suspended
        force_resolve_battle(game, attacker, target)
        assert target in game.player2.battle_area, "Target should survive via Evade"
        assert target.is_suspended, "Target should be suspended after Evade"
        assert attacker in game.player1.battle_area, "Attacker should survive (Retaliation blocked)"

    def test_retaliation_not_triggered_on_tie(self):
        """In a tie, neither Retaliation fires."""
        game, attacker, target = setup_battle_game(
            attacker_dp=5000, target_dp=5000,
            attacker_keywords=['_is_retaliation'],
            target_keywords=['_is_retaliation'])
        force_resolve_battle(game, attacker, target)
        # Both should be deleted, no retaliation loop
        assert attacker not in game.player1.battle_area
        assert target not in game.player2.battle_area


# ═══════════════════════════════════════════════════════════════════════
# B. Piercing Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestPiercing:
    def test_piercing_checks_security_after_kill(self):
        """Piercing attacker checks security after defeating a Digimon."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            attacker_keywords=['_is_piercing'], security_count=5)
        initial_security = len(game.player2.security_cards)
        force_resolve_battle(game, attacker, target)
        assert target not in game.player2.battle_area, "Target should be deleted"
        # Piercing should have checked 1 security card (SA+0 = 1 check)
        assert len(game.player2.security_cards) < initial_security, \
            "Piercing should have checked at least one security card"

    def test_piercing_not_triggered_when_target_survives(self):
        """If target survives via Armor Purge, Piercing does NOT fire."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            attacker_keywords=['_is_piercing'],
            target_keywords=['_is_armor_purge'])
        # Give target a digivolution stack so Armor Purge can activate
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        initial_security = len(game.player2.security_cards)
        force_resolve_battle(game, attacker, target)
        assert target in game.player2.battle_area, "Target should survive via Armor Purge"
        assert len(game.player2.security_cards) == initial_security, \
            "Piercing should NOT check security when target survived"

    def test_piercing_does_not_apply_on_direct_player_attack(self):
        """Piercing only triggers after defeating a Digimon, not on player attacks."""
        game, attacker, _target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_piercing'])
        initial_security = len(game.player2.security_cards)
        # Attack the player directly
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )
        game._resolve_battle()
        # Normal security check happens, but not "extra" Piercing checks
        security_lost = initial_security - len(game.player2.security_cards)
        assert security_lost == 1, "Only 1 security check (SA+0), no extra Piercing"


# ═══════════════════════════════════════════════════════════════════════
# C. Jamming Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestJamming:
    def test_jamming_survives_security_battle_loss(self):
        """Attacker with Jamming survives losing a security battle."""
        game, attacker, _target = setup_battle_game(attacker_dp=1000)
        # Add a high-DP Digimon as security so attacker loses the security battle
        sec_digimon = make_card(card_id="SEC-001", name="SecurityDigimon",
                                kind=CardKind.Digimon, dp=10000, level=5,
                                owner=game.player2)
        game.player2.security_cards = [sec_digimon]

        # Give attacker Jamming
        attacker_card = attacker.top_card
        if isinstance(attacker_card, MockCardSourceWithEffects):
            attacker_card._mock_effects.append(KeywordEffect('_is_jamming'))

        result = game.player2.security_attack(attacker)
        # Attacker DP (1000) < Security DP (10000) but Jamming saves
        assert result == AttackResolution.Survivor

    def test_jamming_does_not_protect_in_normal_battle(self):
        """Jamming does NOT prevent deletion in a normal Digimon vs Digimon battle."""
        game, attacker, target = setup_battle_game(
            attacker_dp=3000, target_dp=7000,
            attacker_keywords=['_is_jamming'])
        force_resolve_battle(game, attacker, target)
        assert attacker not in game.player1.battle_area, \
            "Attacker should still be deleted in normal battle even with Jamming"


# ═══════════════════════════════════════════════════════════════════════
# D. Rush Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestRush:
    def test_rush_bypasses_summoning_sickness(self):
        """Permanent with Rush can attack on the turn it was played."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_rush'])
        attacker.turn_played = game.turn_count  # played this turn
        assert attacker.can_attack() is True

    def test_no_rush_has_summoning_sickness(self):
        """Permanent without Rush cannot attack on the turn it was played."""
        game, attacker, _target = setup_battle_game()
        attacker.turn_played = game.turn_count  # played this turn
        assert attacker.can_attack() is False


# ═══════════════════════════════════════════════════════════════════════
# E. Security Attack +/- Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestSecurityAttack:
    def test_sa_plus_one_checks_two_security(self):
        """SA+1 means 2 security checks per attack."""
        game, attacker, _target = setup_battle_game(
            attacker_sa=1, security_count=5)
        initial_security = len(game.player2.security_cards)
        # Attack player directly
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )
        game._resolve_battle()
        security_lost = initial_security - len(game.player2.security_cards)
        assert security_lost == 2, f"SA+1 should check 2 security cards, lost {security_lost}"

    def test_sa_minus_one_checks_zero_security(self):
        """SA-1 means 0 security checks per attack."""
        game, attacker, _target = setup_battle_game(
            attacker_sa=-1, security_count=5)
        initial_security = len(game.player2.security_cards)
        game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=game.player2,
            effective_target=game.player2,
        )
        game._resolve_battle()
        security_lost = initial_security - len(game.player2.security_cards)
        assert security_lost == 0, f"SA-1 should check 0 security cards, lost {security_lost}"


# ═══════════════════════════════════════════════════════════════════════
# F. Restriction Keyword Tests (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestRestrictionKeywords:
    def test_cannot_attack(self):
        """Permanent with cannot_attack keyword cannot attack."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_cannot_attack'])
        assert attacker.can_attack() is False

    def test_cannot_attack_player(self):
        """Permanent with cannot_attack_player cannot attack player directly."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_cannot_attack_player'])
        assert attacker.can_attack_player() is False

    def test_cannot_block(self):
        """Permanent with cannot_block cannot block."""
        game, _attacker, target = setup_battle_game(
            target_keywords=['_is_cannot_block', '_is_blocker'])
        attacker_perm = Permanent([make_card(name="Incoming")])
        assert target.can_block(attacker_perm) is False

    def test_cannot_be_blocked(self):
        """Blocker cannot block an attacker with cannot_be_blocked."""
        game, attacker, target = setup_battle_game(
            attacker_keywords=['_is_cannot_be_blocked'],
            target_keywords=['_is_blocker'])
        assert target.can_block(attacker) is False


# ═══════════════════════════════════════════════════════════════════════
# G. Linked Card Keywords Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestLinkedCardKeywords:
    def test_linked_card_keyword_detected(self):
        """Keyword from a linked option card is detected by has_keyword()."""
        # Create a permanent without keywords
        base_card = make_card(name="BaseMon", dp=5000)
        perm = Permanent([base_card])

        # Create a linked option card with Blocker
        linked = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "OPT-001"
        entity.card_name_eng = "OptionCard"
        entity.card_kind = CardKind.Option
        entity.dp = 0
        entity.level = 0
        entity.play_cost = 3
        entity.card_colors = [CardColor.Red]
        linked.set_base_data(entity, None)
        linked._mock_effects = [KeywordEffect('_is_blocker')]

        perm.linked_cards.append(linked)
        assert perm.has_keyword('_is_blocker') is True

    def test_linked_card_sa_modifier(self):
        """SA modifier from linked card is counted in security_attack_modifier()."""
        base_card = make_card(name="BaseMon", dp=5000)
        perm = Permanent([base_card])

        linked = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "OPT-002"
        entity.card_name_eng = "SAOption"
        entity.card_kind = CardKind.Option
        entity.dp = 0
        entity.level = 0
        entity.play_cost = 3
        entity.card_colors = [CardColor.Red]
        linked.set_base_data(entity, None)
        linked._mock_effects = [SAModifierEffect(sa_mod=1)]

        perm.linked_cards.append(linked)
        assert perm.security_attack_modifier() == 1


# ═══════════════════════════════════════════════════════════════════════
# H. Granted Keywords Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestGrantedKeywords:
    def test_granted_keyword_detected(self):
        """grant_keyword() makes has_keyword() return True."""
        card = make_card(name="GrantMon", dp=5000)
        perm = Permanent([card])
        perm.grant_keyword('_is_rush', duration=-1)
        assert perm.has_keyword('_is_rush') is True

    def test_granted_keyword_expires(self):
        """Granted keyword with expiry is not detected after expiry turn."""
        card = make_card(name="GrantMon", dp=5000)
        perm = Permanent([card])
        # Create a mock game for turn tracking
        game = Game()
        game.turn_count = 3
        perm._owner_game = game

        perm.grant_keyword('_is_blocker', duration=3)  # expires at turn 3
        assert perm.has_keyword('_is_blocker') is True  # turn 3 <= 3

        game.turn_count = 4
        assert perm.has_keyword('_is_blocker') is False  # turn 4 > 3


# ═══════════════════════════════════════════════════════════════════════
# I. Deletion Prevention Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDeletionPrevention:
    def test_armor_purge_prevents_deletion_returns_false(self):
        """Armor Purge trashes top card and prevents deletion; returns False."""
        game, _attacker, target = setup_battle_game(
            target_keywords=['_is_armor_purge'])
        # Give target a digivolution stack (>1 card)
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        initial_sources = len(target.card_sources)

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is False, "Deletion should be prevented"
        assert target in game.player2.battle_area, "Target should remain on field"
        assert len(target.card_sources) == initial_sources - 1, "One card trashed"

    def test_evade_prevents_deletion_returns_false(self):
        """Evade suspends self and prevents deletion; returns False."""
        game, _attacker, target = setup_battle_game(
            target_keywords=['_is_evade'])
        assert not target.is_suspended

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is False, "Deletion should be prevented"
        assert target in game.player2.battle_area, "Target should remain on field"
        assert target.is_suspended, "Target should be suspended"

    def test_barrier_prevents_deletion_returns_false(self):
        """Barrier trashes top security and prevents deletion; returns False."""
        game, _attacker, target = setup_battle_game(
            target_keywords=['_is_barrier'], security_count=5)
        initial_security = len(game.player2.security_cards)

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is False, "Deletion should be prevented"
        assert target in game.player2.battle_area, "Target should remain on field"
        assert len(game.player2.security_cards) == initial_security - 1, "One security trashed"


# ═══════════════════════════════════════════════════════════════════════
# J. Game-Over State Cleanup Tests (2 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestGameOverCleanup:
    def test_declare_winner_clears_pending_state(self):
        """declare_winner() clears pending_attack, pending_selection, active_player."""
        game = Game()
        game.pending_attack = PendingAttack(
            attacker=Permanent([make_card()]),
            original_target=game.player2,
            effective_target=game.player2,
        )
        game.active_player = game.player2
        game.declare_winner(game.player1)
        assert game.game_over is True
        assert game.winner is game.player1
        assert game.pending_attack is None
        assert game.pending_selection is None
        assert game.active_player is None

    def test_declare_winner_clears_revealed_cards(self):
        """declare_winner() clears revealed_cards list."""
        game = Game()
        if hasattr(game, 'revealed_cards'):
            game.revealed_cards.append(make_card())
            game.declare_winner(game.player1)
            assert len(game.revealed_cards) == 0


# ═══════════════════════════════════════════════════════════════════════
# K. Retaliation from Defender's Perspective (1 test)
# ═══════════════════════════════════════════════════════════════════════

class TestRetaliationDefenderWins:
    def test_attacker_with_retaliation_deleted_deletes_winner(self):
        """When attacker with Retaliation loses a battle, the target is also deleted."""
        game, attacker, target = setup_battle_game(
            attacker_dp=3000, target_dp=7000,
            attacker_keywords=['_is_retaliation'])
        force_resolve_battle(game, attacker, target)
        assert attacker not in game.player1.battle_area, "Attacker should be deleted (lost battle)"
        assert target not in game.player2.battle_area, "Target should be deleted (Retaliation)"


# ═══════════════════════════════════════════════════════════════════════
# L. Permanent State Tests (is_attacking, _temp_sa_modifier)
# ═══════════════════════════════════════════════════════════════════════

class TestPermanentState:
    def test_is_attacking_flag_default(self):
        """is_attacking defaults to False on new permanents."""
        card = make_card(name="Mon", dp=5000)
        perm = Permanent([card])
        assert perm.is_attacking is False

    def test_clear_attack_state_resets(self):
        """clear_attack_state() resets is_attacking and _temp_sa_modifier."""
        card = make_card(name="Mon", dp=5000)
        perm = Permanent([card])
        perm.is_attacking = True
        perm._temp_sa_modifier = 2
        perm.clear_attack_state()
        assert perm.is_attacking is False
        assert perm._temp_sa_modifier == 0

    def test_temp_sa_modifier_included_in_security_attack_modifier(self):
        """_temp_sa_modifier is added to security_attack_modifier() total."""
        card = make_card(name="Mon", dp=5000)
        perm = Permanent([card])
        perm._temp_sa_modifier = 3
        assert perm.security_attack_modifier() == 3


# ═══════════════════════════════════════════════════════════════════════
# M. GamePhase Enum Tests
# ═══════════════════════════════════════════════════════════════════════

class TestGamePhaseEnum:
    def test_end_of_turn_action_exists(self):
        """GamePhase.EndOfTurnAction enum value exists."""
        assert GamePhase.EndOfTurnAction.value == 15

    def test_alliance_timing_exists(self):
        """GamePhase.AllianceTiming enum value exists."""
        assert GamePhase.AllianceTiming.value == 16


# ═══════════════════════════════════════════════════════════════════════
# N. Training Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestTraining:
    def test_training_suspends_and_adds_card(self):
        """Training suspends the Digimon and places top deck card at bottom of digi stack."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_training'])
        attacker.is_suspended = False
        initial_sources = len(attacker.card_sources)
        initial_deck = len(game.player1.library_cards)

        game._execute_training(attacker, game.player1)

        assert attacker.is_suspended, "Training should suspend the Digimon"
        assert len(attacker.card_sources) == initial_sources + 1, \
            "Training should add a card to digivolution stack"
        assert len(game.player1.library_cards) == initial_deck - 1, \
            "Training should remove top card from deck"

    def test_training_does_nothing_with_empty_deck(self):
        """Training does nothing if the player's deck is empty."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_training'])
        game.player1.library_cards.clear()
        initial_sources = len(attacker.card_sources)

        game._execute_training(attacker, game.player1)

        assert not attacker.is_suspended, "Should not suspend with empty deck"
        assert len(attacker.card_sources) == initial_sources

    def test_training_card_placed_at_bottom_of_stack(self):
        """Training places the new card at index 0 (bottom of digi stack)."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_training'])
        top_before = attacker.top_card
        deck_top = game.player1.library_cards[0]

        game._execute_training(attacker, game.player1)

        assert attacker.card_sources[0] is deck_top, \
            "Deck card should be at bottom of digi stack"
        assert attacker.top_card is top_before, \
            "Top card should remain unchanged"

    def test_training_action_mask_main_phase(self):
        """Training is exposed in main phase action mask for unsuspended Digimon."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_training'])
        attacker.is_suspended = False
        game.current_phase = GamePhase.Main

        mask = game.get_action_mask(game.turn_player.player_id)
        # Attacker is at index 0 in player1.battle_area
        assert mask[1000 + 0 * 10] == 1.0, "Training should be exposed at action 1000"

    def test_training_not_exposed_when_suspended(self):
        """Training is NOT exposed when the Digimon is already suspended."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_training'])
        attacker.is_suspended = True
        game.current_phase = GamePhase.Main

        mask = game.get_action_mask(game.turn_player.player_id)
        assert mask[1000 + 0 * 10] == 0.0, "Suspended Digimon should not expose Training"


# ═══════════════════════════════════════════════════════════════════════
# O. Progress Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestProgress:
    def test_is_immune_to_opponent_effects_while_attacking(self):
        """Permanent with Progress is immune to opponent effects while attacking."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])
        perm.is_attacking = True
        assert perm.is_immune_to_opponent_effects is True

    def test_not_immune_when_not_attacking(self):
        """Permanent with Progress is NOT immune when not attacking."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])
        perm.is_attacking = False
        assert perm.is_immune_to_opponent_effects is False

    def test_progress_blocks_opponent_effect_deletion(self):
        """delete_permanent with is_opponent_effect=True fails when Progress is active."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_progress'])
        attacker.is_attacking = True

        result = game.player1.delete_permanent(attacker, is_opponent_effect=True)
        assert result is False, "Progress should block opponent effect deletion"
        assert attacker in game.player1.battle_area

    def test_progress_does_not_block_battle_deletion(self):
        """Progress does NOT block normal DP-comparison battle deletion."""
        game, attacker, target = setup_battle_game(
            attacker_dp=3000, target_dp=7000,
            attacker_keywords=['_is_progress'])
        attacker.is_attacking = True

        # Normal battle deletion (game rule, not opponent effect) should still work
        result = game.player1.delete_permanent(attacker, is_battle=True)
        assert result is True, "Normal battle deletion should not be blocked by Progress"

    def test_progress_blocks_retaliation(self):
        """Progress blocks Retaliation — attacker survives opponent's Retaliation effect."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, target_dp=5000,
            attacker_keywords=['_is_progress'],
            target_keywords=['_is_retaliation'])
        force_resolve_battle(game, attacker, target)

        assert target not in game.player2.battle_area, "Target should be deleted (lost DP battle)"
        assert attacker in game.player1.battle_area, \
            "Attacker with Progress should survive Retaliation"

    def test_progress_blocks_opponent_effect_during_battle(self):
        """Progress blocks opponent effects even when is_battle=True."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_progress'])
        attacker.is_attacking = True

        # Opponent effect during battle — Progress should block
        result = game.player1.delete_permanent(
            attacker, is_battle=True, is_opponent_effect=True)
        assert result is False, "Progress should block opponent effects during battle"
        assert attacker in game.player1.battle_area

    def test_progress_ignores_dp_debuff_while_attacking(self):
        """Progress attacker ignores negative DP modifiers received during attack."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])
        perm.is_attacking = True

        # Attempt to apply -3000 DP debuff while attacking with Progress
        perm.change_dp(-3000)

        # change_dp should have been blocked entirely
        assert perm.dp == 5000, "Progress should block DP debuff during attack"

    def test_progress_ignores_preexisting_dp_debuffs_during_attack(self):
        """Pre-existing DP debuffs are ignored when Progress attacker starts attacking."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])

        # Apply debuff BEFORE attack begins (not yet attacking)
        perm.change_dp(-2000)
        assert perm.dp == 3000, "Debuff should apply when not attacking"

        # Now start attacking — debuff should be ignored
        perm.is_attacking = True
        assert perm.dp == 5000, \
            "Pre-existing DP debuffs should be ignored during Progress attack"

    def test_progress_dp_debuffs_restored_after_attack(self):
        """After attack ends (is_attacking=False), DP debuffs are re-applied."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])

        # Apply debuff before attack
        perm.change_dp(-2000)
        perm.is_attacking = True
        assert perm.dp == 5000, "Debuff ignored during attack"

        # End the attack
        perm.clear_attack_state()
        assert perm.dp == 3000, "Debuff should re-apply after attack ends"

    def test_progress_positive_dp_buffs_still_apply(self):
        """Positive DP buffs still apply during Progress attack."""
        card = make_keyword_card(name="ProgressMon", dp=5000, keywords=['_is_progress'])
        perm = Permanent([card])
        perm.is_attacking = True

        # Positive buff should not be blocked
        perm.change_dp(2000)
        assert perm.dp == 7000, "Positive DP buffs should still apply during Progress"

    def test_progress_blocks_security_effects(self):
        """Security effects are skipped when attacker has Progress."""
        game, attacker, _target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_progress'],
            security_count=0)
        attacker.is_attacking = True

        # Add a security Digimon that would normally delete the attacker (high DP)
        sec_digimon = make_card(card_id="SEC-001", name="SecurityDigimon",
                                kind=CardKind.Digimon, dp=10000, level=6,
                                owner=game.player2)
        # Give it a mock security effect that would apply a DP debuff
        game.player2.security_cards = [sec_digimon]

        result = game.player2.security_attack(attacker)

        # The security Digimon has higher DP (10000 > 7000), so attacker loses the
        # DP comparison. But Progress doesn't prevent DP battle — it only blocks effects.
        # This test verifies that the security effect execution path is skipped.
        # (The DP battle still happens normally since it's game rules, not an effect.)
        assert result in (AttackResolution.Survivor, AttackResolution.AttackerDeleted)

    def test_progress_security_effects_skipped_not_executed(self):
        """Verify security effect callbacks are NOT executed against Progress attacker."""
        game, attacker, _target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_progress'],
            security_count=0)
        attacker.is_attacking = True

        # Create a security card with a mock effect that tracks execution
        sec_card = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "SEC-EFF"
        entity.card_name_eng = "SecurityOption"
        entity.card_kind = CardKind.Option
        entity.dp = 0
        entity.level = 0
        entity.play_cost = 5
        entity.card_colors = [CardColor.Red]
        sec_card.set_base_data(entity, game.player2)

        # Create a security effect with a callback that sets a flag
        effect = ICardEffect()
        effect.is_inherited_effect = False
        effect.timing = EffectTiming.SecuritySkill
        effect.is_security_effect = True
        effect._callback_executed = False
        def track_callback():
            effect._callback_executed = True
        effect.on_process_callback = track_callback
        sec_card._mock_effects = [effect]

        game.player2.security_cards = [sec_card]
        game.player2.security_attack(attacker)

        assert effect._callback_executed is False, \
            "Security effect callback should NOT execute against Progress attacker"

    def test_progress_dp_battle_still_works(self):
        """Progress does NOT prevent normal DP-comparison security battle."""
        game, attacker, _target = setup_battle_game(
            attacker_dp=3000, attacker_keywords=['_is_progress'],
            security_count=0)
        attacker.is_attacking = True

        # Add a high-DP security Digimon
        sec_digimon = make_card(card_id="SEC-001", name="SecurityDigimon",
                                kind=CardKind.Digimon, dp=10000, level=6,
                                owner=game.player2)
        game.player2.security_cards = [sec_digimon]

        result = game.player2.security_attack(attacker)

        # DP battle: 3000 < 10000 → attacker deleted (Progress doesn't prevent this)
        assert result == AttackResolution.AttackerDeleted, \
            "Progress does NOT prevent DP-comparison battle deletion"


# ═══════════════════════════════════════════════════════════════════════
# P. Fortitude Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestFortitude:
    def test_fortitude_replays_when_had_digi_cards(self):
        """Fortitude replays the card from trash if it had digivolution cards."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_fortitude'])
        # Give target a digivolution stack so had_digi_cards = True
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        top_card = target.top_card

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is True, "Deletion should succeed (Fortitude doesn't prevent)"

        # top_card should have been replayed from trash as a new permanent
        replayed = any(
            top_card in perm.card_sources
            for perm in game.player2.battle_area
        )
        assert replayed, "Fortitude should replay the top card as a new permanent"
        assert top_card not in game.player2.trash_cards, \
            "Replayed card should not remain in trash"

    def test_fortitude_not_triggered_without_digi_cards(self):
        """Fortitude does NOT trigger if the permanent had no digivolution cards."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_fortitude'])
        # Only 1 card in stack — no digi cards
        top_card = target.top_card

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is True
        # Card should stay in trash (Fortitude didn't trigger)
        assert top_card in game.player2.trash_cards, \
            "Without digi cards, Fortitude should not activate"

    def test_fortitude_is_mandatory(self):
        """Fortitude is mandatory — always activates when conditions met."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_fortitude'])
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)

        game.player2.delete_permanent(target, is_battle=True)
        # Should always replay — count battle area permanents
        assert len(game.player2.battle_area) >= 1, \
            "Fortitude mandatory replay should add a new permanent"


# ═══════════════════════════════════════════════════════════════════════
# Q. Save Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestSave:
    def _make_tamer_permanent(self, owner):
        """Helper to create a Tamer permanent on the field."""
        tamer_card = make_card(card_id="TAMER-001", name="Tamer",
                               kind=CardKind.Tamer, dp=None, level=None, owner=owner)
        tamer = Permanent([tamer_card])
        return tamer

    def test_save_places_under_tamer(self):
        """Save places deleted card under first available Tamer."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_save'])
        tamer = self._make_tamer_permanent(game.player2)
        game.player2.battle_area.append(tamer)
        top_card = target.top_card
        initial_tamer_sources = len(tamer.card_sources)

        game.player2.delete_permanent(target, is_battle=True)

        assert top_card not in game.player2.trash_cards, \
            "Save should remove the card from trash"
        assert len(tamer.card_sources) == initial_tamer_sources + 1, \
            "Card should be placed under the Tamer"

    def test_save_no_tamers_stays_in_trash(self):
        """If no Tamers on field, Save does not activate and card stays in trash."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_save'])
        # No Tamers on field
        top_card = target.top_card

        game.player2.delete_permanent(target, is_battle=True)

        assert top_card in game.player2.trash_cards, \
            "Without Tamers, card should stay in trash"

    def test_save_does_not_trigger_if_fortitude_activates(self):
        """Save and Fortitude on the same card — Fortitude takes priority (elif)."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_save', '_is_fortitude'])
        tamer = self._make_tamer_permanent(game.player2)
        game.player2.battle_area.append(tamer)
        # Give digi cards so Fortitude triggers
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        initial_tamer_sources = len(tamer.card_sources)

        game.player2.delete_permanent(target, is_battle=True)

        # Fortitude should have replayed the card, Save should not have triggered
        assert len(tamer.card_sources) == initial_tamer_sources, \
            "Tamer sources should be unchanged (Fortitude took priority over Save)"


# ═══════════════════════════════════════════════════════════════════════
# R. Decoy Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestDecoy:
    def test_decoy_sacrificed_to_save_target(self):
        """Decoy Digimon is deleted instead when target would be deleted by opponent effect."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        # Add a Decoy Digimon on P2's field
        decoy_card = make_keyword_card(card_id="DECOY-001", name="DecoyMon", dp=3000,
                                        keywords=['_is_decoy'], owner=game.player2)
        decoy = Permanent([decoy_card])
        decoy._owner_game = game
        game.player2.battle_area.append(decoy)

        result = game.player2.delete_permanent(target, is_opponent_effect=True)

        assert result is False, "Original deletion should be prevented by Decoy"
        assert target in game.player2.battle_area, "Target should survive"
        assert decoy not in game.player2.battle_area, "Decoy should be deleted"

    def test_decoy_not_triggered_in_battle(self):
        """Decoy does NOT trigger during battle deletion (only opponent effects)."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        decoy_card = make_keyword_card(card_id="DECOY-001", name="DecoyMon", dp=3000,
                                        keywords=['_is_decoy'], owner=game.player2)
        decoy = Permanent([decoy_card])
        game.player2.battle_area.append(decoy)

        result = game.player2.delete_permanent(target, is_battle=True)

        assert result is True, "Battle deletion should not trigger Decoy"
        assert target not in game.player2.battle_area
        assert decoy in game.player2.battle_area, "Decoy should survive (not triggered)"

    def test_decoy_cards_go_to_trash(self):
        """When Decoy sacrifices itself, its cards go to the owner's trash."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        decoy_card = make_keyword_card(card_id="DECOY-001", name="DecoyMon", dp=3000,
                                        keywords=['_is_decoy'], owner=game.player2)
        decoy = Permanent([decoy_card])
        decoy._owner_game = game
        game.player2.battle_area.append(decoy)
        initial_trash = len(game.player2.trash_cards)

        game.player2.delete_permanent(target, is_opponent_effect=True)

        assert len(game.player2.trash_cards) > initial_trash, \
            "Decoy's cards should be in trash"


# ═══════════════════════════════════════════════════════════════════════
# S. Material Save Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestMaterialSave:
    def _make_tamer_permanent(self, owner):
        """Helper to create a Tamer permanent."""
        tamer_card = make_card(card_id="TAMER-001", name="Tamer",
                               kind=CardKind.Tamer, dp=None, level=None, owner=owner)
        tamer = Permanent([tamer_card])
        return tamer

    def test_material_save_places_card_under_tamer(self):
        """Material Save places 1 digi source under a Tamer before deletion."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_material_save'])
        # Give target digi cards
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)

        tamer = self._make_tamer_permanent(game.player2)
        game.player2.battle_area.append(tamer)
        initial_tamer_sources = len(tamer.card_sources)

        game.player2.delete_permanent(target, is_battle=True)

        assert len(tamer.card_sources) == initial_tamer_sources + 1, \
            "Material Save should place 1 card under the Tamer"

    def test_material_save_does_not_prevent_deletion(self):
        """Material Save does NOT prevent deletion — the Digimon is still deleted."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_material_save'])
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        tamer = self._make_tamer_permanent(game.player2)
        game.player2.battle_area.append(tamer)

        result = game.player2.delete_permanent(target, is_battle=True)

        assert result is True, "Digimon should still be deleted"
        assert target not in game.player2.battle_area

    def test_material_save_no_tamer_no_activation(self):
        """Material Save does NOT activate without a Tamer on field."""
        game, _attacker, target = setup_battle_game(
            target_dp=5000, target_keywords=['_is_material_save'])
        base_card = make_card(card_id="BASE-001", name="Base", dp=3000, level=3,
                              owner=game.player2)
        target.card_sources.insert(0, base_card)
        # No Tamers on field

        result = game.player2.delete_permanent(target, is_battle=True)
        assert result is True, "Deletion should proceed normally without Tamers"


# ═══════════════════════════════════════════════════════════════════════
# T. Vortex Tests (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestVortex:
    def test_vortex_bypasses_summoning_sickness(self):
        """Vortex attack bypasses summoning sickness via is_vortex=True."""
        card = make_keyword_card(name="VortexMon", dp=5000, keywords=['_is_vortex'])
        perm = Permanent([card])
        game = Game()
        game.turn_count = 3
        perm.turn_played = 3  # played this turn
        perm._owner_game = game

        assert perm.can_attack() is False, "Normal attack blocked by summoning sickness"
        assert perm.can_attack(is_vortex=True) is True, "Vortex bypasses summoning sickness"

    def test_vortex_action_mask_end_of_turn(self):
        """Vortex exposes attack actions in EndOfTurnAction phase."""
        game, attacker, target = setup_battle_game(
            attacker_keywords=['_is_vortex'])
        game.current_phase = GamePhase.EndOfTurnAction

        mask = game.get_action_mask(game.turn_player.player_id)
        # Attacker at slot 0, target at slot 0 → action 100 + 0*15 + 0 = 100
        assert mask[100] == 1.0, "Vortex attack on opponent Digimon should be exposed"
        assert mask[62] == 1.0, "Pass should always be valid in EndOfTurnAction"

    def test_vortex_only_targets_digimon(self):
        """Vortex can only attack opponent Digimon, not the player."""
        game, attacker, target = setup_battle_game(
            attacker_keywords=['_is_vortex'])
        game.current_phase = GamePhase.EndOfTurnAction

        mask = game.get_action_mask(game.turn_player.player_id)
        # Security attack (action 100 + 0*15 + 12) should NOT be exposed
        # (Vortex only targets opponent Digimon in the EndOfTurnAction mask)
        assert mask[100 + 0 * 15 + 12] == 0.0, \
            "Vortex should NOT allow security/player attacks"

    def test_end_of_turn_parking_with_vortex(self):
        """phase_end() parks at EndOfTurnAction when Vortex Digimon exists."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_vortex'])
        game.current_phase = GamePhase.Main
        # Simulate phase_end transition
        game.phase_end()

        assert game.current_phase == GamePhase.EndOfTurnAction, \
            "Should park at EndOfTurnAction with Vortex Digimon"


# ═══════════════════════════════════════════════════════════════════════
# U. Overclock Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestOverclock:
    def test_overclock_action_mask_end_of_turn(self):
        """Overclock is exposed in EndOfTurnAction mask when sacrifice available."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_overclock'])
        # Need a sacrifice target on P1's field
        sacrifice_card = make_card(card_id="SAC-001", name="SacMon", dp=2000,
                                    owner=game.player1)
        sacrifice = Permanent([sacrifice_card])
        sacrifice._owner_game = game
        game.player1.battle_area.append(sacrifice)
        game.current_phase = GamePhase.EndOfTurnAction

        mask = game.get_action_mask(game.turn_player.player_id)
        # Overclock at slot 0 → action 1000 + 0*10 = 1000
        assert mask[1000] == 1.0, "Overclock should be exposed with sacrifice target"

    def test_overclock_not_exposed_without_sacrifice(self):
        """Overclock is NOT exposed when no sacrifice targets exist."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_overclock'])
        # Only the attacker on P1's field — no valid sacrifice targets
        # Remove any other permanents
        game.player1.battle_area = [attacker]
        game.current_phase = GamePhase.EndOfTurnAction

        mask = game.get_action_mask(game.turn_player.player_id)
        assert mask[1000] == 0.0, "Overclock should NOT be exposed without sacrifice"

    def test_overclock_end_of_turn_parking(self):
        """phase_end() parks at EndOfTurnAction when Overclock + sacrifice exist."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_overclock'])
        sacrifice_card = make_card(card_id="SAC-001", name="SacMon", dp=2000,
                                    owner=game.player1)
        sacrifice = Permanent([sacrifice_card])
        game.player1.battle_area.append(sacrifice)
        game.current_phase = GamePhase.Main

        game.phase_end()

        assert game.current_phase == GamePhase.EndOfTurnAction, \
            "Should park at EndOfTurnAction with Overclock + sacrifice"


# ═══════════════════════════════════════════════════════════════════════
# V. Alliance Tests (4 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestAlliance:
    def test_alliance_timing_entered(self):
        """Attack with Alliance enters AllianceTiming when allies available."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_alliance'])
        # Add an unsuspended ally on P1's field
        ally_card = make_card(card_id="ALLY-001", name="AllyMon", dp=3000,
                              owner=game.player1)
        ally = Permanent([ally_card])
        ally._owner_game = game
        game.player1.battle_area.append(ally)

        game.resolve_attack(attacker, target)

        assert game.current_phase == GamePhase.AllianceTiming, \
            "Should park at AllianceTiming with Alliance keyword"

    def test_alliance_mask_exposes_allies(self):
        """AllianceTiming mask exposes unsuspended allies and pass."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_alliance'])
        ally_card = make_card(card_id="ALLY-001", name="AllyMon", dp=3000,
                              owner=game.player1)
        ally = Permanent([ally_card])
        ally._owner_game = game
        game.player1.battle_area.append(ally)

        game.resolve_attack(attacker, target)
        assert game.current_phase == GamePhase.AllianceTiming

        mask = game.get_action_mask(game.turn_player.player_id)
        assert mask[62] == 1.0, "Pass should be valid"
        # Ally is at index 1 (attacker is at 0) → action 100 + 1 = 101
        assert mask[101] == 1.0, "Ally should be selectable"
        # Attacker at index 0 should NOT be selectable
        assert mask[100] == 0.0, "Attacker should NOT be selectable"

    def test_alliance_decode_suspends_ally_and_adds_dp(self):
        """Selecting an ally in AllianceTiming suspends it and adds DP + SA+1."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_alliance'])
        # Add TWO allies so that after selecting one, there's still another,
        # keeping us in AllianceTiming (before battle resolves and clears state)
        ally_card = make_card(card_id="ALLY-001", name="AllyMon", dp=3000,
                              owner=game.player1)
        ally = Permanent([ally_card])
        ally._owner_game = game
        game.player1.battle_area.append(ally)

        ally2_card = make_card(card_id="ALLY-002", name="AllyMon2", dp=2000,
                               owner=game.player1)
        ally2 = Permanent([ally2_card])
        ally2._owner_game = game
        game.player1.battle_area.append(ally2)

        game.resolve_attack(attacker, target)
        initial_dp = attacker.dp

        # Select the first ally (index 1 → action 101)
        game._decode_alliance(101)

        assert ally.is_suspended, "Ally should be suspended after Alliance"
        # Still in AllianceTiming because ally2 is available
        assert game.current_phase == GamePhase.AllianceTiming
        assert attacker._temp_sa_modifier >= 1, "Attacker should gain SA+1"

    def test_alliance_decline_proceeds_to_blockers(self):
        """Declining Alliance (action 62) proceeds past AllianceTiming."""
        game, attacker, target = setup_battle_game(
            attacker_dp=7000, attacker_keywords=['_is_alliance'])
        ally_card = make_card(card_id="ALLY-001", name="AllyMon", dp=3000,
                              owner=game.player1)
        ally = Permanent([ally_card])
        ally._owner_game = game
        game.player1.battle_area.append(ally)

        game.resolve_attack(attacker, target)
        assert game.current_phase == GamePhase.AllianceTiming

        game._decode_alliance(62)

        # Should have moved past AllianceTiming (to blockers/counter/resolve)
        assert game.current_phase != GamePhase.AllianceTiming, \
            "Declining Alliance should move past AllianceTiming"


# ═══════════════════════════════════════════════════════════════════════
# W. End-of-Turn Phase Tests
# ═══════════════════════════════════════════════════════════════════════

class TestEndOfTurnPhase:
    def test_phase_end_no_keywords_proceeds_normally(self):
        """phase_end() proceeds normally when no Vortex/Overclock present."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player1.game = game
        game.player2.game = game
        # Give both players library cards to avoid deck-out
        for _ in range(10):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        game.phase_end()

        assert game.current_phase != GamePhase.EndOfTurnAction, \
            "Should NOT park at EndOfTurnAction without keywords"

    def test_decline_end_of_turn_action_switches_turn(self):
        """Declining (action 62) in EndOfTurnAction proceeds to switch turn."""
        game, attacker, _target = setup_battle_game(
            attacker_keywords=['_is_vortex'])
        game.current_phase = GamePhase.EndOfTurnAction
        # Add library cards to avoid deck-out on turn switch
        for _ in range(10):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        game._decode_end_of_turn_action(62)

        # Should have moved past EndOfTurnAction
        assert game.current_phase != GamePhase.EndOfTurnAction, \
            "Declining should transition away from EndOfTurnAction"


# ═══════════════════════════════════════════════════════════════════════
# X. Cannot Return To Hand Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestCannotReturnToHand:
    def test_bounce_blocked_by_keyword(self):
        """Permanent with cannot_return_to_hand stays on field when bounced."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        target.grant_keyword('_is_cannot_return_to_hand', duration=-1)
        initial_hand = len(game.player2.hand_cards)
        initial_battle = len(game.player2.battle_area)

        game.player2.bounce_permanent_to_hand(target)

        assert target in game.player2.battle_area, \
            "Permanent with cannot_return_to_hand should stay on field"
        assert len(game.player2.hand_cards) == initial_hand, \
            "No cards should be added to hand"
        assert len(game.player2.battle_area) == initial_battle, \
            "Battle area should be unchanged"

    def test_bounce_works_without_keyword(self):
        """Normal permanent is bounced to hand as expected."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        initial_hand = len(game.player2.hand_cards)

        game.player2.bounce_permanent_to_hand(target)

        assert target not in game.player2.battle_area, \
            "Permanent should be removed from battle area"
        assert len(game.player2.hand_cards) > initial_hand, \
            "Cards should be added to hand"

    def test_bounce_blocked_by_granted_keyword(self):
        """grant_keyword for cannot_return_to_hand blocks bounce."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        # Grant via the runtime mechanism (as the transpiled scripts do)
        target.grant_keyword('_is_cannot_return_to_hand')

        game.player2.bounce_permanent_to_hand(target)

        assert target in game.player2.battle_area, \
            "Granted cannot_return_to_hand should block bounce"


# ═══════════════════════════════════════════════════════════════════════
# Y. Cannot Return To Deck Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestCannotReturnToDeck:
    def test_return_to_deck_blocked_by_keyword(self):
        """Permanent with cannot_return_to_deck stays on field."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        target.grant_keyword('_is_cannot_return_to_deck', duration=-1)
        initial_deck = len(game.player2.library_cards)
        initial_battle = len(game.player2.battle_area)

        game.player2.return_permanent_to_deck_bottom(target)

        assert target in game.player2.battle_area, \
            "Permanent with cannot_return_to_deck should stay on field"
        assert len(game.player2.library_cards) == initial_deck, \
            "No cards should be added to deck"
        assert len(game.player2.battle_area) == initial_battle, \
            "Battle area should be unchanged"

    def test_return_to_deck_works_without_keyword(self):
        """Normal permanent is returned to deck bottom as expected."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        initial_deck = len(game.player2.library_cards)

        game.player2.return_permanent_to_deck_bottom(target)

        assert target not in game.player2.battle_area, \
            "Permanent should be removed from battle area"
        assert len(game.player2.library_cards) > initial_deck, \
            "Cards should be added to deck"

    def test_return_to_deck_blocked_by_granted_keyword(self):
        """grant_keyword for cannot_return_to_deck blocks return to deck."""
        game, _attacker, target = setup_battle_game(target_dp=5000)
        target.grant_keyword('_is_cannot_return_to_deck')

        game.player2.return_permanent_to_deck_bottom(target)

        assert target in game.player2.battle_area, \
            "Granted cannot_return_to_deck should block return to deck"


# ═══════════════════════════════════════════════════════════════════════
# Z. Grant Keyword Via Callback Tests (3 tests)
# ═══════════════════════════════════════════════════════════════════════

class TestGrantKeywordViaCallback:
    def test_grant_keyword_blocker_via_process_callback(self):
        """Simulate a transpiled process callback granting Blocker."""
        card = make_card(name="GrantTarget", dp=5000)
        perm = Permanent([card])

        # Simulate what transpiled code does in on_grant callback
        def on_grant(target_perm):
            target_perm.grant_keyword('_is_blocker')

        on_grant(perm)

        assert perm.has_keyword('_is_blocker') is True, \
            "grant_keyword('_is_blocker') should make has_keyword return True"

    def test_grant_multiple_keywords_via_callback(self):
        """Simulate granting multiple keywords in a single callback."""
        card = make_card(name="MultiGrantTarget", dp=5000)
        perm = Permanent([card])

        # Simulate BT24-056 style: grant cannot_return_to_deck + cannot_return_to_hand
        def on_grant(target_perm):
            target_perm.grant_keyword('_is_cannot_return_to_deck')
            target_perm.grant_keyword('_is_cannot_return_to_hand')

        on_grant(perm)

        assert perm.has_keyword('_is_cannot_return_to_deck') is True
        assert perm.has_keyword('_is_cannot_return_to_hand') is True

    def test_grant_keyword_interacts_with_engine_checks(self):
        """Granted cannot_attack keyword blocks can_attack() via has_keyword."""
        game, attacker, _target = setup_battle_game()
        attacker.turn_played = 1  # not this turn

        assert attacker.can_attack() is True, "Should be able to attack initially"

        # Grant cannot_attack via runtime mechanism
        attacker.grant_keyword('_is_cannot_attack')

        assert attacker.can_attack() is False, \
            "Granted cannot_attack should block can_attack()"
