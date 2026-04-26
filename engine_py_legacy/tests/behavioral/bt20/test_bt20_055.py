"""Behavioral tests for BT20-055 Invisimon (Lv.6 Black/Blue, DP 11000, Cost 11).

Card text:
  [Security] [End of Opponent's Turn] Play this card without paying the cost.
  [On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon
    and flip your opponent's top face-down security card face up. Then, delete
    1 of your opponent's Digimon with 1 or fewer digivolution cards.
  [Your Turn] When your Digimon checks a face-up security card, you may place
    the top card of this Digimon face-up at the bottom of your security stack.

Key faithfulness points tested:
  - Effect 0: [Security] [End of Opponent's Turn] -- OnEndTurn timing, fires
    when card is in security during opponent's turn, plays without cost using
    game.effect_play_from_security (which fires On Play effects).
  - Effects 1+2: [On Play]/[When Digivolving] -- De-Digivolve 2 target,
    flip top face-down security face-up, delete Digimon with <=1 digi-cards.
    Both use effect_select_opponent_permanent for player agency.
  - Effect 3: [Your Turn] OnSecurityCheck -- triggers when owner's Digimon
    checks a face-up security card; places top digivolution card face-up
    at bottom of owner's security stack. Optional.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

INVISIMON = "BT20-055"
AGUMON = "ST1-03"       # Lv.3 Red, DP 2000
GREYMON = "ST1-07"      # Lv.4 Red, DP 5000
FILLER = [AGUMON] * 50


@pytest.mark.behavioral
class TestBT20055SecurityPlay:
    """Tests for BT20-055 Effect 0: [Security] [End of Opponent's Turn] play."""

    def test_has_security_end_of_turn_effect(self, debug_runner):
        """Should have an effect with OnEndTurn timing + is_security_effect."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(INVISIMON, runner.game.player1)
        effects = cs.effect_list(None)

        sec_eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and getattr(e, 'is_security_effect', False)
        ]
        assert len(sec_eot) == 1, "Should have exactly 1 security OnEndTurn effect"

    def test_security_condition_requires_opponent_turn(self, debug_runner):
        """Condition should only pass on opponent's turn (not owner's turn)."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(INVISIMON, runner.game.player1)

        # Put card in security
        runner.game.player1.security_cards.append(cs)

        effects = cs.effect_list(None)
        sec_eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and getattr(e, 'is_security_effect', False)
        ][0]

        # P1's turn -- should NOT trigger
        assert runner.game.player1.is_my_turn
        assert not sec_eot.can_use_condition({}), (
            "Should NOT trigger on owner's turn"
        )

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        assert sec_eot.can_use_condition({}), (
            "Should trigger on opponent's turn when card is in security"
        )

    def test_security_condition_requires_in_security(self, debug_runner):
        """Condition should fail when card is NOT in security."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER,
            initial_memory=5, first_player=2,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(INVISIMON, runner.game.player1)

        # Card in hand, NOT in security
        runner.game.player1.hand_cards.append(cs)

        effects = cs.effect_list(None)
        sec_eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and getattr(e, 'is_security_effect', False)
        ][0]

        assert not sec_eot.can_use_condition({}), (
            "Should NOT trigger when card is not in security"
        )

    def test_security_play_uses_effect_play_from_security(self, debug_runner):
        """Process callback should use game.effect_play_from_security
        so On Play effects fire correctly."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER,
            initial_memory=5, first_player=2,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(INVISIMON, runner.game.player1)

        # Put card in security
        runner.game.player1.security_cards.append(cs)

        effects = cs.effect_list(None)
        sec_eot = [
            e for e in effects
            if e.timing == EffectTiming.OnEndTurn
            and getattr(e, 'is_security_effect', False)
        ][0]

        field_before = len(runner.game.player1.battle_area)

        # Execute the process callback
        sec_eot.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        # Card should now be on the field
        assert len(runner.game.player1.battle_area) == field_before + 1, (
            "Card should be played from security to field"
        )
        # Card should no longer be in security
        assert cs not in runner.game.player1.security_cards, (
            "Card should be removed from security"
        )


@pytest.mark.behavioral
class TestBT20055OnPlayDeDigivolve:
    """Tests for BT20-055 On Play/When Digivolving: De-Digivolve, flip security, delete."""

    def test_has_on_play_effect(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [INVISIMON])

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"

    def test_has_when_digivolving_effect(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [INVISIMON])

        top = perm.top_card
        effects = top.effect_list(None)
        when_digi = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(when_digi) == 1, "Should have exactly 1 When Digivolving effect"

    def test_on_play_de_digivolves_opponent(self, debug_runner):
        """On Play should De-Digivolve 2 an opponent's Digimon."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [INVISIMON])

        # Opponent has a Digimon stack: Lv.3 -> Lv.4 -> Lv.5
        # After De-Digivolve 2, should lose top 2, left with Lv.3
        opp_perm = runner.place_on_field(2, [AGUMON, GREYMON, "BT1-021"])
        stack_before = len(opp_perm.card_sources)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        # The opponent's Digimon should have lost up to 2 cards from De-Digivolve
        assert len(opp_perm.card_sources) < stack_before, (
            f"De-Digivolve 2 should reduce stack. Before: {stack_before}, "
            f"after: {len(opp_perm.card_sources)}"
        )

    def test_on_play_flips_security_face_up(self, debug_runner):
        """On Play should flip opponent's top face-down security card face up."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [INVISIMON])

        # Give opponent security (face-down by default)
        runner.clear_zone(2, "security")
        runner.inject_card(2, AGUMON, "security_top")
        runner.inject_card(2, AGUMON, "security_top")

        # Place an opponent Digimon so De-Digivolve has a target
        runner.place_on_field(2, [AGUMON])

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        p2 = runner.game.player2
        face_up_before = len(p2.face_up_security)

        on_play.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        face_up_after = len(p2.face_up_security)
        assert face_up_after > face_up_before, (
            f"Should flip at least 1 security card face-up. "
            f"Before: {face_up_before}, after: {face_up_after}"
        )

    def test_on_play_deletes_digimon_with_few_digi_cards(self, debug_runner):
        """On Play should delete 1 opponent Digimon with <=1 digivolution cards."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=10
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [INVISIMON])

        # Place opponent Digimon with no digi-cards (single card stack, 0 digi cards)
        opp_perm1 = runner.place_on_field(2, [AGUMON])  # 0 digi-cards, eligible
        # Place another as De-Digivolve target
        opp_perm2 = runner.place_on_field(2, [AGUMON, GREYMON])  # 1 digi-card, also eligible

        # Give security for flip
        runner.inject_card(2, AGUMON, "security_top")

        p2_field_before = len(runner.game.player2.battle_area)

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ][0]

        on_play.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
        })
        runner.auto_resolve()

        p2_field_after = len(runner.game.player2.battle_area)
        # Should have deleted at least 1 Digimon (the delete step)
        # Note: De-Digivolve might also affect field count if target gets destroyed
        assert p2_field_after < p2_field_before, (
            f"Should delete opponent Digimon with <=1 digi-cards. "
            f"Before: {p2_field_before}, after: {p2_field_after}"
        )

    def test_condition_requires_field(self, debug_runner):
        """Condition should fail when Invisimon is not on the field."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )

        runner.inject_card(1, INVISIMON, "hand")
        game = runner.game

        inv_hand = [
            c for c in game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == INVISIMON
        ]
        assert len(inv_hand) >= 1

        card = inv_hand[0]
        effects = card.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        if on_play:
            assert not on_play[0].can_use_condition({}), (
                "Condition should fail when card is not on field"
            )


@pytest.mark.behavioral
class TestBT20055FaceUpSecurityCheck:
    """Tests for BT20-055 Effect 3: face-up security check -> place digi-card to security."""

    def test_has_on_security_check_effect(self, debug_runner):
        """Should have an OnSecurityCheck timing effect."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [INVISIMON])

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck]
        assert len(sec_check) == 1, "Should have exactly 1 OnSecurityCheck effect"

    def test_security_check_is_optional(self, debug_runner):
        """The face-up security check effect should be optional."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [INVISIMON])

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]
        assert sec_check.is_optional, "Effect should be optional"

    def test_condition_requires_your_turn(self, debug_runner):
        """Condition should only pass on owner's turn."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        # Place with a digi-card underneath
        perm = runner.place_on_field(1, [AGUMON, INVISIMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]

        ctx = {
            'event_player': game.player1,
            'security_was_face_up': True,
        }

        # P1's turn -- should pass
        assert game.player1.is_my_turn
        assert sec_check.can_use_condition(ctx), (
            "Should pass on owner's turn with face-up security"
        )

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        assert not sec_check.can_use_condition(ctx), (
            "Should fail on opponent's turn"
        )

    def test_condition_requires_face_up_security(self, debug_runner):
        """Condition should require the checked security card to be face-up."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [AGUMON, INVISIMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]

        # Face-down security: should fail
        ctx_face_down = {
            'event_player': game.player1,
            'security_was_face_up': False,
        }
        assert not sec_check.can_use_condition(ctx_face_down), (
            "Should NOT trigger for face-down security card"
        )

        # Face-up security: should pass
        ctx_face_up = {
            'event_player': game.player1,
            'security_was_face_up': True,
        }
        assert sec_check.can_use_condition(ctx_face_up), (
            "Should trigger for face-up security card"
        )

    def test_condition_requires_own_digimon_checking(self, debug_runner):
        """Condition should only fire when the owner's Digimon checks security."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        perm = runner.place_on_field(1, [AGUMON, INVISIMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]

        # Opponent's Digimon checking: should fail
        ctx_opp = {
            'event_player': game.player2,
            'security_was_face_up': True,
        }
        assert not sec_check.can_use_condition(ctx_opp), (
            "Should NOT trigger when opponent's Digimon checks security"
        )

    def test_condition_requires_digivolution_cards(self, debug_runner):
        """Condition should fail if Invisimon has no digivolution cards."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        # Place without digi-cards (single card stack)
        perm = runner.place_on_field(1, [INVISIMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]

        ctx = {
            'event_player': game.player1,
            'security_was_face_up': True,
        }
        assert not sec_check.can_use_condition(ctx), (
            "Should fail when permanent has no digivolution cards to give"
        )

    def test_process_places_digi_card_to_security_bottom_face_up(self, debug_runner):
        """Process should trash top digi-card and place it face-up at security bottom."""
        runner = debug_runner(
            deck1=[INVISIMON] * 4 + [AGUMON] * 46, deck2=FILLER, initial_memory=5
        )
        # Stack: Agumon (bottom) -> Invisimon (top) = 1 digi-card
        perm = runner.place_on_field(1, [AGUMON, INVISIMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        sec_check = [e for e in effects if e.timing == EffectTiming.OnSecurityCheck][0]

        sec_before = len(game.player1.security_cards)
        digi_before = len(perm.card_sources)

        sec_check.on_process_callback({
            'player': game.player1,
            'permanent': perm,
        })
        runner.auto_resolve()

        sec_after = len(game.player1.security_cards)
        digi_after = len(perm.card_sources)

        assert sec_after == sec_before + 1, (
            f"Should add 1 card to security. Before: {sec_before}, after: {sec_after}"
        )
        assert digi_after == digi_before - 1, (
            f"Should remove 1 digi-card from stack. Before: {digi_before}, after: {digi_after}"
        )

        # The added card should be face-up
        added_card = game.player1.security_cards[0]  # bottom of security (index 0)
        assert game.player1.is_security_face_up(added_card), (
            "Added card should be face-up in security"
        )
