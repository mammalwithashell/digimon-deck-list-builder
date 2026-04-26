"""Behavioral tests for BT14-001 Koromon (Lv.2 Digi-Egg, Red).

Card text:
Inherited Effect [Your Turn] [Once Per Turn] When a card is removed from
  your opponent's security stack, <Draw 1>.

C# reference (BT14_001.cs):
  CanUseCondition: IsExistOnBattleArea, IsOwnerTurn,
    CanTriggerWhenLoseSecurity(hashtable, player => player == card.Owner.Enemy)
  CanActivateCondition: IsExistOnBattleArea
  ActivateCoroutine: DrawClass(card.Owner, 1, activateClass).Draw()
  is_inherited_effect = true, hash = "Draw1_BT14_001", OPT (max 1)
  is_optional = false (mandatory when triggered)

Key behaviors:
- Inherited effect only (active when under a Digimon on the battle area)
- [Your Turn] restriction
- [Once Per Turn] (max_count_per_turn=1, hash "Draw1_BT14_001")
- Triggers on opponent's security removal (not own security)
- Draw 1 (mandatory -- not optional)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


KOROMON_DECK = ["BT14-001"] * 5 + ["ST1-03"] * 45
FILLER_DECK = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestBT14001InheritedFlags:
    """Verify effect metadata: inherited flag, timing, hash, OPT count."""

    def test_inherited_effect_exists(self, debug_runner):
        """BT14-001 should have exactly 1 inherited OnLoseSecurity effect."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]  # Koromon (bottom of stack)
        effects = card.effect_list(None)
        ols_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(ols_effects) == 1, "Should have exactly 1 OnLoseSecurity effect"

    def test_is_inherited(self, debug_runner):
        """The effect must be marked as inherited."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]
        assert ols.is_inherited_effect, "Effect must be inherited"

    def test_hash_string(self, debug_runner):
        """Hash string must be 'Draw1_BT14_001' per C# reference."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]
        assert ols.hash_string == "Draw1_BT14_001", \
            f"Hash string must be 'Draw1_BT14_001', got '{ols.hash_string}'"

    def test_once_per_turn_metadata(self, debug_runner):
        """max_count_per_turn should be 1 (Once Per Turn)."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]
        assert ols.max_count_per_turn == 1, \
            f"max_count_per_turn should be 1, got {ols.max_count_per_turn}"

    def test_timing_is_on_lose_security(self, debug_runner):
        """Timing must be OnLoseSecurity."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]
        assert ols.timing == EffectTiming.OnLoseSecurity


@pytest.mark.behavioral
class TestBT14001Condition:
    """Condition checks: your-turn, opponent-only security trigger."""

    def test_condition_passes_on_your_turn_opponent_security(self, debug_runner):
        """Condition should pass when: it's your turn AND opponent loses security."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent losing security
        }
        assert ols.can_use_condition(ctx), \
            "Condition should pass when opponent loses security on your turn"

    def test_condition_fails_on_own_security_loss(self, debug_runner):
        """When YOUR OWN security is removed, condition must fail."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,  # OWN security loss
        }
        assert not ols.can_use_condition(ctx), \
            "Condition should FAIL when own security is lost"

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """[Your Turn] restriction: condition must fail on opponent's turn."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        # Set it to player 2's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent loses security (but it's their turn)
        }
        assert not ols.can_use_condition(ctx), \
            "Condition should FAIL on opponent's turn"


@pytest.mark.behavioral
class TestBT14001OPT:
    """Once Per Turn: first activation allowed, second blocked."""

    def test_first_activation_allowed(self, debug_runner):
        """First activation in a turn should be allowed."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        assert ols.can_activate_this_turn(), "First activation should be allowed"

    def test_second_activation_blocked(self, debug_runner):
        """After one activation, a second should be blocked by OPT."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        ols = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        ols.record_activation()
        assert not ols.can_activate_this_turn(), \
            "Second activation should be blocked by OPT"


@pytest.mark.behavioral
class TestBT14001Integration:
    """Integration tests: fire OnLoseSecurity via engine, verify draw happens."""

    def test_draw_via_fire_timing_opponent_security(self, debug_runner):
        """Fire OnLoseSecurity through the engine dispatch for opponent security
        loss during your turn. The inherited effect should draw 1."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        hand_before = len(game.player1.hand_cards)
        assert len(game.player2.security_cards) > 0, "Opponent needs security"

        # Remove opponent's security and fire timing through engine
        sec_card = game.player2.security_cards[0]
        game.player2.security_cards.remove(sec_card)
        game.player2.trash_cards.append(sec_card)
        game.player2._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": sec_card, "player": game.player2},
        )

        assert len(game.player1.hand_cards) == hand_before + 1, \
            f"Should draw 1. Hand before={hand_before}, after={len(game.player1.hand_cards)}"

    def test_no_draw_via_fire_timing_own_security(self, debug_runner):
        """Fire OnLoseSecurity for own security loss. No draw should occur."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        hand_before = len(game.player1.hand_cards)
        assert len(game.player1.security_cards) > 0, "Player 1 needs security"

        # Remove OWN security and fire timing
        sec_card = game.player1.security_cards[0]
        game.player1.security_cards.remove(sec_card)
        game.player1.trash_cards.append(sec_card)
        game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": sec_card, "player": game.player1},
        )

        assert len(game.player1.hand_cards) == hand_before, \
            f"Should NOT draw on own security loss. Hand before={hand_before}, after={len(game.player1.hand_cards)}"

    def test_opt_via_fire_timing_two_security_losses(self, debug_runner):
        """Fire OnLoseSecurity twice for opponent. Only 1 draw should occur (OPT)."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False

        hand_before = len(game.player1.hand_cards)
        assert len(game.player2.security_cards) >= 2, "Opponent needs >= 2 security"

        # Remove opponent security twice
        for _ in range(2):
            if not game.player2.security_cards:
                break
            sec_card = game.player2.security_cards[0]
            game.player2.security_cards.remove(sec_card)
            game.player2.trash_cards.append(sec_card)
            game.player2._fire_timing(
                EffectTiming.OnLoseSecurity,
                {"lost_card": sec_card, "player": game.player2},
            )

        assert len(game.player1.hand_cards) == hand_before + 1, \
            f"Should draw only 1 (OPT). Hand before={hand_before}, after={len(game.player1.hand_cards)}"

    def test_no_draw_on_opponent_turn_via_fire_timing(self, debug_runner):
        """On opponent's turn, fire OnLoseSecurity for opponent. No draw."""
        runner = debug_runner(
            deck1=KOROMON_DECK, deck2=FILLER_DECK, initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT14-001", "ST1-03"])
        game = runner.game

        # Switch to player 2's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        hand_before = len(game.player1.hand_cards)
        assert len(game.player2.security_cards) > 0, "Opponent needs security"

        sec_card = game.player2.security_cards[0]
        game.player2.security_cards.remove(sec_card)
        game.player2.trash_cards.append(sec_card)
        game.player2._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": sec_card, "player": game.player2},
        )

        assert len(game.player1.hand_cards) == hand_before, \
            f"Should NOT draw on opponent's turn. Hand before={hand_before}, after={len(game.player1.hand_cards)}"
