"""Tests for keyword mechanics: Retaliation, Piercing, Jamming, Rush, SA+/-,
Blitz, Collision, restriction keywords, linked card keywords, granted keywords,
deletion prevention, and game-over state cleanup.

Covers ~30 tests validating engine combat resolution correctness.
"""

import os
import sys
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from digimon_gym.engine.game import Game, PendingAttack, ACTION_SPACE_SIZE
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming, AttackResolution,
)
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect


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
    game.pending_attack = PendingAttack(
        attacker=attacker,
        original_target=target,
        effective_target=target,
    )
    game._resolve_battle()


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before each test."""
    CardRegistry.reset()
    CardRegistry.initialize_from_list([
        "TEST-001", "KW-001", "ATK-001", "TGT-001",
    ])
    yield
    CardRegistry.reset()


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
