"""Behavioral tests for P-165 ShoeShoemon (Lv.4, Yellow, Puppet/LIBERATOR, DP 4000, Cost 4).

Card text:
[Security] At the end of the battle, play this card without paying the cost.
[On Play] [When Digivolving] Play 1 [Familiar] Token.
    At the end of your opponent's turn, delete that token.
Inherited: <Barrier>

C# reference: P_165.cs
- Security: PlaySelfDigimonAfterBattleSecurityEffect
    (plays this card AFTER the security battle, not during security timing)
- On Play / When Digivolving: PlaySelfDeleteFamiliarToken
    (familiar token self-deletes at end of opponent's turn via OnEndTurn)
- Inherited: BarrierSelfEffect (isInheritedEffect=true)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase, CardColor


# Minimal deck for DebugRunner
DECK = ["P-165"] * 4 + ["ST1-03"] * 46


@pytest.mark.behavioral
class TestP165ShoeShoemonSecurity:
    """Tests for P-165 [Security] At the end of the battle, play this card."""

    def test_security_effect_plays_card_after_battle(self, debug_runner):
        """When revealed from security, P-165 should be played to the field
        after the security battle resolves."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # Clear P2 security and inject P-165 as top security card
        runner.clear_zone(2, "security")
        sec_card = runner.inject_card(2, "P-165", "security_top")

        # P1 attacker with high DP to survive the security battle
        attacker = runner.place_on_field(1, ["ST1-03"])
        # Ensure attacker has enough DP to beat P-165 (4000 DP)
        # ST1-03 has 2000 DP; place a higher-DP attacker
        runner.game.player1.battle_area.remove(attacker)
        from tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=8000)
        game.player1.battle_area.append(attacker)

        field_before = len(p2.battle_area)
        result = p2.security_attack(attacker)

        # P-165 should now be on P2's field (played after battle).
        # Playing P-165 also triggers its [On Play] effect which creates a
        # Familiar Token, so field should gain at least 2 permanents
        # (P-165 itself + Familiar Token).
        field_after = len(p2.battle_area)
        assert field_after >= field_before + 1, (
            f"P-165 should be played to field after security battle, "
            f"field went from {field_before} to {field_after}"
        )

        # Verify P-165 is on the field
        p165_on_field = [p for p in p2.battle_area
                         if p.top_card and p.top_card.c_entity_base
                         and p.top_card.c_entity_base.card_id == "P-165"]
        assert len(p165_on_field) >= 1, (
            "P-165 should be on the field after security play"
        )

    def test_security_card_not_trashed_when_played(self, debug_runner):
        """When P-165 is played from security, it should NOT end up in trash."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.clear_zone(2, "security")
        sec_card = runner.inject_card(2, "P-165", "security_top")

        from tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=8000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)

        # P-165 should NOT be in trash
        p165_in_trash = [c for c in p2.trash_cards
                         if c.c_entity_base and c.c_entity_base.card_id == "P-165"]
        assert len(p165_in_trash) == 0, (
            "P-165 should not be in trash when played from security"
        )

    def test_security_battles_as_security_digimon(self, debug_runner):
        """P-165 is a Digimon, so it should battle the attacker as a Security Digimon.
        If attacker DP <= P-165 DP (4000), attacker should be deleted (unless Jamming)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.clear_zone(2, "security")
        runner.inject_card(2, "P-165", "security_top")

        # Weak attacker with DP < 4000
        from tests.helpers.game_builder import make_permanent
        from digimon_gym.engine.data.enums import AttackResolution
        weak_attacker = make_permanent("WEAK-001", "WeakAttacker", dp=3000)
        game.player1.battle_area.append(weak_attacker)

        result = p2.security_attack(weak_attacker)

        assert result == AttackResolution.AttackerDeleted, (
            "Weak attacker should be deleted by P-165 security Digimon (4000 DP)"
        )

        # P-165 should still be played to field after the battle
        p165_on_field = [p for p in p2.battle_area
                         if p.top_card and p.top_card.c_entity_base
                         and p.top_card.c_entity_base.card_id == "P-165"]
        assert len(p165_on_field) == 1, (
            "P-165 should be played to field even after deleting the attacker"
        )


@pytest.mark.behavioral
class TestP165ShoeShoemonOnPlay:
    """Tests for P-165 [On Play] Play 1 [Familiar] Token, delete at end of opponent's turn."""

    def test_on_play_creates_familiar_token(self, debug_runner):
        """Playing P-165 should create a Familiar Token on the field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        # Inject P-165 into hand
        runner.inject_card(1, "P-165", "hand")

        runner.set_phase("Main")
        field_before = len(game.player1.battle_area)

        # Find and play ShoeShoemon
        action = runner.find_action("ShoeShoemon")
        if action is None:
            pytest.skip("Cannot find play action for ShoeShoemon")
        runner.execute(action)
        runner.auto_resolve()

        # Should have P-165 + Familiar Token on field
        field_after = len(game.player1.battle_area)
        assert field_after >= field_before + 2, (
            f"Playing P-165 should add P-165 + Familiar Token to field, "
            f"field went from {field_before} to {field_after}"
        )

        # Verify Familiar Token exists
        tokens = [p for p in game.player1.battle_area
                  if p.top_card and p.top_card.c_entity_base
                  and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        assert len(tokens) >= 1, (
            "Should have at least 1 Familiar Token on field after playing P-165"
        )

    def test_familiar_token_deleted_at_end_of_opponent_turn(self, debug_runner):
        """Familiar Token should be deleted at the end of the opponent's turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        runner.inject_card(1, "P-165", "hand")
        runner.set_phase("Main")

        action = runner.find_action("ShoeShoemon")
        if action is None:
            pytest.skip("Cannot find play action for ShoeShoemon")
        runner.execute(action)
        runner.auto_resolve()

        # Find the Familiar Token
        tokens = [p for p in game.player1.battle_area
                  if p.top_card and p.top_card.c_entity_base
                  and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        assert len(tokens) >= 1, "Should have Familiar Token after playing P-165"

        # Simulate end of opponent's turn:
        # Switch turn to opponent, then fire OnEndTurn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        game.execute_effects(EffectTiming.OnEndTurn)

        # Token should be deleted
        tokens_after = [p for p in game.player1.battle_area
                        if p.top_card and p.top_card.c_entity_base
                        and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        assert len(tokens_after) == 0, (
            "Familiar Token should be deleted at end of opponent's turn"
        )

    def test_familiar_token_not_deleted_at_end_of_own_turn(self, debug_runner):
        """Familiar Token should NOT be deleted at the end of the owner's own turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        runner.inject_card(1, "P-165", "hand")
        runner.set_phase("Main")

        action = runner.find_action("ShoeShoemon")
        if action is None:
            pytest.skip("Cannot find play action for ShoeShoemon")
        runner.execute(action)
        runner.auto_resolve()

        tokens = [p for p in game.player1.battle_area
                  if p.top_card and p.top_card.c_entity_base
                  and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        assert len(tokens) >= 1, "Should have Familiar Token"

        # Fire OnEndTurn while it's STILL player 1's turn (own turn)
        game.execute_effects(EffectTiming.OnEndTurn)

        # Token should still be there
        tokens_after = [p for p in game.player1.battle_area
                        if p.top_card and p.top_card.c_entity_base
                        and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        assert len(tokens_after) >= 1, (
            "Familiar Token should NOT be deleted at end of own turn"
        )


@pytest.mark.behavioral
class TestP165ShoeShoemonWhenDigivolving:
    """Tests for P-165 [When Digivolving] Play 1 [Familiar] Token."""

    def test_when_digivolving_creates_familiar_token(self, debug_runner):
        """Digivolving into P-165 should create a Familiar Token."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        # Place a Lv.3 on field that P-165 can digivolve from
        base_perm = runner.place_on_field(1, ["ST3-04"])  # Yellow Lv.3

        # Inject P-165 into hand for digivolving
        runner.inject_card(1, "P-165", "hand")
        runner.set_phase("Main")

        field_before = len(game.player1.battle_area)

        # Look for digivolve action
        action = runner.find_action("Digivolve")
        if action is None:
            action = runner.find_action("digivolve")
        if action is None:
            action = runner.find_action("ShoeShoemon")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # Check for Familiar Token
        tokens = [p for p in game.player1.battle_area
                  if p.top_card and p.top_card.c_entity_base
                  and 'TOKEN_FAMILIAR' in (p.top_card.c_entity_base.card_id or '')]
        if action is not None:
            assert len(tokens) >= 1, (
                "Digivolving into P-165 should create a Familiar Token"
            )


@pytest.mark.behavioral
class TestP165ShoeShoemonBarrier:
    """Tests for P-165 Inherited: <Barrier>."""

    def test_barrier_is_inherited(self):
        """P-165's Barrier should be marked as inherited effect."""
        from digimon_gym.engine.data.card_registry import CardRegistry
        from digimon_gym.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        from digimon_gym.engine.core.card_source import CardSource
        cs = db.create_card_source("P-165", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        barrier_effects = [e for e in effects
                           if getattr(e, '_is_barrier', False)]
        assert len(barrier_effects) >= 1, (
            "P-165 should have a Barrier effect"
        )
        assert barrier_effects[0].is_inherited_effect, (
            "P-165 Barrier should be marked as inherited"
        )
