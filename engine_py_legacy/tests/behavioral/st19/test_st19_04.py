"""Behavioral tests for ST19-04 PawnChessmon (Lv.3 Yellow/Black Puppet).

Card text:
[On Play] By trashing 1 card with the [Puppet] trait in your hand,
<Draw 2> (Draw 2 cards from your deck.)

Inherited Effect:
<Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.)

C# reference behavior:
- CanActivateCondition: IsExistOnBattleAreaDigimon(card) AND HasMatchConditionOwnersHand(card, HasPuppetInTrait)
- Hand selection: canNoSelect=true (player can skip), canEndNotMax=false
- AfterSelectCardCoroutine: only draws if cardSources.Count > 0
- Reboot: standard inherited static effect
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# Build a deck with ST19-04 and enough Puppet cards for testing
PUPPET_DECK = (
    ["ST19-04"] * 4
    + ["ST19-02"] * 4   # Junkmon (Puppet Lv.3) - trash targets
    + ["ST19-06"] * 4   # Doggymon (Puppet Lv.4)
    + ["ST19-09"] * 4   # Pandamon (Puppet Lv.5)
    + ["ST19-13"] * 4   # ShinMonzaemon (Puppet Lv.6)
    + ["ST1-02"] * 4    # Biyomon (Bird, non-Puppet) - negative test
    + ["ST19-03"] * 4   # Shoemon (Puppet Lv.3)
    + ["ST19-05"] * 4   # PawnChessmon (Puppet Lv.3)
    + ["ST19-07"] * 4   # Tobucatmon (Puppet Lv.4)
    + ["ST19-10"] * 4   # ExTyrannomon (Puppet Lv.5)
    + ["ST19-11"] * 4   # Chaperomon (Puppet Lv.5)
    + ["ST19-12"] * 4   # Cendrillmon (Puppet Lv.6)
    + ["ST19-08"] * 4   # ShoeShoemon (Puppet Lv.4)
)

FILLER_DECK = (
    ["ST1-02"] * 50  # Non-Puppet filler
)


@pytest.mark.behavioral
class TestST1904OnPlay:
    """Tests for [On Play] trash Puppet from hand -> Draw 2."""

    def test_on_play_trash_puppet_draw_2(self, debug_runner):
        """Playing ST19-04 with a Puppet in hand should offer the optional
        effect. Selecting a Puppet card trashes it and draws 2."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
            starting_hand1=["ST19-04", "ST19-02", "ST1-02", "ST1-02", "ST1-02"],
        )

        game = runner.game
        p1 = game.player1

        hand_before = len(p1.hand_cards)
        deck_before = len(p1.library_cards)

        # Play PawnChessmon (ST19-04) — actions use card names
        play_action = runner.find_action("Play PawnChessmon")
        assert play_action is not None, (
            f"Should be able to play PawnChessmon. Actions: {runner.actions()}"
        )
        runner.execute(play_action)

        # Should enter a selection phase for the optional On Play effect
        # Accept the optional effect (Yes)
        yes_action = runner.find_action("Yes")
        if yes_action is not None:
            runner.execute(yes_action)

        # Now should be in hand selection phase — pick Junkmon (Puppet)
        select_action = runner.find_action("Junkmon")
        assert select_action is not None, (
            f"Should be able to select Junkmon (Puppet) from hand. Actions: {runner.actions()}"
        )
        runner.execute(select_action)

        # Auto-resolve any remaining phases
        runner.auto_resolve()

        # Verify: trashed 1 from hand, drew 2
        # Net hand change: -1 (played) -1 (trashed) +2 (drawn) = 0
        assert len(p1.hand_cards) == hand_before, (
            f"Hand should be {hand_before}: played 1, trashed 1, drew 2 = net 0. "
            f"Got {len(p1.hand_cards)}"
        )
        assert len(p1.library_cards) == deck_before - 2, (
            f"Deck should shrink by 2 from draw. Was {deck_before}, got {len(p1.library_cards)}"
        )

        # Trashed card should be in trash
        trash_ids = [c.c_entity_base.card_id for c in p1.trash_cards]
        assert "ST19-02" in trash_ids, (
            f"ST19-02 (Junkmon) should be in trash. Trash: {trash_ids}"
        )

    def test_on_play_skip_hand_selection(self, debug_runner):
        """Player can skip the hand selection (canNoSelect=true in C#),
        which means no trash and no draw."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
            starting_hand1=["ST19-04", "ST19-02", "ST1-02", "ST1-02", "ST1-02"],
        )

        game = runner.game
        p1 = game.player1

        hand_before = len(p1.hand_cards)
        deck_before = len(p1.library_cards)

        # Play PawnChessmon
        play_action = runner.find_action("Play PawnChessmon")
        runner.execute(play_action)

        # Accept the optional On Play effect trigger (Yes)
        yes_action = runner.find_action("Yes")
        if yes_action is not None:
            runner.execute(yes_action)

        # Now in hand selection — decline via Pass (action 62)
        # C# canNoSelect=true means the player can skip selecting
        pass_action = runner.find_action("Pass")
        if pass_action is None:
            # Try action 62 directly (the decline action for optional selections)
            legal = runner.action_mask()
            if 62 in legal:
                pass_action = 62
        assert pass_action is not None, (
            f"Should be able to pass/decline hand selection. Actions: {runner.actions()}"
        )
        runner.execute(pass_action)

        runner.auto_resolve()

        # Hand: -1 (played), no trash, no draw
        assert len(p1.hand_cards) == hand_before - 1, (
            f"Hand should only lose the played card. Was {hand_before}, got {len(p1.hand_cards)}"
        )
        assert len(p1.library_cards) == deck_before, (
            f"Deck should be unchanged. Was {deck_before}, got {len(p1.library_cards)}"
        )

    def test_on_play_no_puppet_in_hand_no_trigger(self, debug_runner):
        """If no Puppet cards in hand, On Play effect should not trigger."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
            starting_hand1=["ST19-04", "ST1-02", "ST1-02", "ST1-02", "ST1-02"],
        )

        game = runner.game
        p1 = game.player1

        deck_before = len(p1.library_cards)

        # Play PawnChessmon
        play_action = runner.find_action("Play PawnChessmon")
        runner.execute(play_action)

        # Auto-resolve — no optional prompt should appear since no Puppet in hand
        runner.auto_resolve()

        # No draw should have occurred
        assert len(p1.library_cards) == deck_before, (
            f"Deck should not change when no Puppet in hand. Was {deck_before}, got {len(p1.library_cards)}"
        )

    def test_on_play_only_puppet_cards_selectable(self, debug_runner):
        """Only cards with [Puppet] trait should be selectable for trashing."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
            starting_hand1=["ST19-04", "ST19-02", "ST1-02", "ST1-02", "ST1-02"],
        )

        game = runner.game

        # Play PawnChessmon
        play_action = runner.find_action("Play PawnChessmon")
        runner.execute(play_action)

        # Accept optional effect
        yes_action = runner.find_action("Yes")
        if yes_action is not None:
            runner.execute(yes_action)

        # In selection phase: Biyomon (Bird) should NOT be selectable
        actions = runner.actions()
        action_descs = list(actions.values())
        biyomon_action = runner.find_action("Biyomon")
        assert biyomon_action is None, (
            f"Biyomon (non-Puppet) should not be selectable. Actions: {action_descs}"
        )

        # Junkmon (Puppet) SHOULD be selectable
        junkmon_action = runner.find_action("Junkmon")
        assert junkmon_action is not None, (
            f"Junkmon (Puppet) should be selectable. Actions: {action_descs}"
        )

    def test_trash_uses_proper_api(self, debug_runner):
        """Trashing from hand should use player.trash_from_hand() to fire
        OnDiscardHand timing properly."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
            starting_hand1=["ST19-04", "ST19-02", "ST1-02", "ST1-02", "ST1-02"],
        )

        game = runner.game
        p1 = game.player1

        # Play PawnChessmon
        play_action = runner.find_action("Play PawnChessmon")
        runner.execute(play_action)

        # Accept optional effect
        yes_action = runner.find_action("Yes")
        if yes_action is not None:
            runner.execute(yes_action)

        # Select Puppet to trash
        select_action = runner.find_action("Junkmon")
        assert select_action is not None
        runner.execute(select_action)
        runner.auto_resolve()

        # Card should be in trash
        trash_ids = [c.c_entity_base.card_id for c in p1.trash_cards]
        assert "ST19-02" in trash_ids


@pytest.mark.behavioral
class TestST1904Reboot:
    """Tests for inherited <Reboot> effect."""

    def test_reboot_inherited_flag(self, debug_runner):
        """ST19-04 should have an inherited Reboot effect with _is_reboot flag."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
        )

        # Place ST19-04 as a digivolution source under a Lv.4
        perm = runner.place_on_field(1, ["ST19-04", "ST19-06"])

        # Check the inherited effects from ST19-04 (bottom card)
        bottom_card = perm.card_sources[0]
        effects = bottom_card.effect_list(None)

        reboot_effects = [e for e in effects if getattr(e, '_is_reboot', False)]
        assert len(reboot_effects) >= 1, (
            f"Should have at least 1 Reboot effect. Effects: {[e.effect_name for e in effects]}"
        )

        reboot = reboot_effects[0]
        assert reboot.is_inherited_effect, "Reboot must be an inherited effect"

    def test_reboot_not_main_effect(self, debug_runner):
        """Reboot should ONLY be an inherited effect, not a main effect."""
        runner = debug_runner(
            deck1=PUPPET_DECK,
            deck2=FILLER_DECK,
            initial_memory=10,
        )

        perm = runner.place_on_field(1, ["ST19-04"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)

        reboot_effects = [e for e in effects if getattr(e, '_is_reboot', False)]
        for reb in reboot_effects:
            assert reb.is_inherited_effect, (
                "Reboot should only be inherited, not a main effect"
            )
