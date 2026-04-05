"""Behavioral tests for ST19-03 Shoemon (Lv.3 Yellow, Puppet/LIBERATOR).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Puppet]
trait and 1 card with the [LIBERATOR] trait among them to the hand. Return
the rest to the bottom of the deck.

Inherited Effect:
[Your Turn] All of your opponent's security Digimon get -3000 DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# --- Helper card IDs ---
SHOEMON = "ST19-03"          # Puppet + LIBERATOR (card under test)
PUPPET_ONLY_1 = "ST19-02"   # Junkmon (Puppet only)
PUPPET_ONLY_2 = "ST19-04"   # PawnChessmon (Puppet only)
LIBERATOR_ONLY = "ST19-14"  # Arisa Kinosaki (LIBERATOR only, Tamer)
BOTH_TRAITS = "ST19-08"     # ShoeShoemon (Puppet + LIBERATOR)
NO_TRAIT = "BT1-087"        # T.K. Takaishi (no Puppet/LIBERATOR traits)
FILLER = "ST19-05"          # PawnChessmon (Puppet only, used as deck filler)


def _setup_runner(debug_runner, top_cards):
    """Create a runner with SHOEMON in hand and top_cards on top of library."""
    deck1 = [SHOEMON] + [FILLER] * 49
    deck2 = [FILLER] * 50
    runner = debug_runner(
        deck1=deck1,
        deck2=deck2,
        initial_memory=5,
        starting_hand1=[SHOEMON],
    )
    # Inject desired cards on top of library
    p1 = runner.game.player1
    from digimon_gym.engine.data.card_database import CardDatabase
    db = CardDatabase()
    for card_id in reversed(top_cards):
        cs = db.create_card_source(card_id, p1)
        p1.library_cards.insert(0, cs)
    return runner


@pytest.mark.behavioral
class TestST19_03_OnPlay:
    """Tests for [On Play] reveal top 3, add 1 Puppet + 1 LIBERATOR."""

    def test_on_play_effect_exists_with_correct_timing(self, debug_runner):
        """Effect 0 must be OnEnterFieldAnyone with is_on_play flag."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SHOEMON])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) == 1, "Should have exactly 1 on-play effect"
        assert on_play[0].timing == EffectTiming.OnEnterFieldAnyone

    def test_reveal_adds_puppet_and_liberator(self, debug_runner):
        """When top 3 has 1 Puppet-only and 1 LIBERATOR-only, both are added to hand."""
        runner = _setup_runner(debug_runner, [PUPPET_ONLY_1, LIBERATOR_ONLY, PUPPET_ONLY_2])

        game = runner.game
        p1 = game.player1

        action = runner.find_action("Play")
        assert action is not None, f"No play action found. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        hand_ids = [c.c_entity_base.card_id for c in p1.hand_cards]
        assert PUPPET_ONLY_1 in hand_ids, f"Puppet card should be in hand. Hand: {hand_ids}"
        assert LIBERATOR_ONLY in hand_ids, f"LIBERATOR card should be in hand. Hand: {hand_ids}"

    def test_reveal_rest_goes_to_deck_bottom(self, debug_runner):
        """Cards not selected go to the bottom of the deck."""
        runner = _setup_runner(debug_runner, [PUPPET_ONLY_1, LIBERATOR_ONLY, PUPPET_ONLY_2])

        game = runner.game
        p1 = game.player1
        lib_size_before = len(p1.library_cards)

        action = runner.find_action("Play")
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        # 3 revealed, 2 to hand, 1 to deck bottom -> net -2
        assert len(p1.library_cards) == lib_size_before - 2, \
            f"Library should shrink by 2 (3 revealed, 1 returned). Was {lib_size_before}, now {len(p1.library_cards)}"

        # The remaining card (PUPPET_ONLY_2) should be at bottom
        bottom_id = p1.library_cards[-1].c_entity_base.card_id
        assert bottom_id == PUPPET_ONLY_2, \
            f"Remaining Puppet card should be at deck bottom. Got: {bottom_id}"

    def test_reveal_no_puppet_match_skips_puppet_pass(self, debug_runner):
        """If no Puppet cards in revealed 3, skip that pass, still pick LIBERATOR."""
        runner = _setup_runner(debug_runner, [LIBERATOR_ONLY, NO_TRAIT, NO_TRAIT])

        game = runner.game
        p1 = game.player1

        action = runner.find_action("Play")
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        hand_ids = [c.c_entity_base.card_id for c in p1.hand_cards]
        assert LIBERATOR_ONLY in hand_ids, f"LIBERATOR card should be in hand. Hand: {hand_ids}"

    def test_reveal_no_liberator_match_skips_liberator_pass(self, debug_runner):
        """If no LIBERATOR cards in revealed 3, skip that pass, still pick Puppet."""
        runner = _setup_runner(debug_runner, [PUPPET_ONLY_1, NO_TRAIT, NO_TRAIT])

        game = runner.game
        p1 = game.player1

        action = runner.find_action("Play")
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        hand_ids = [c.c_entity_base.card_id for c in p1.hand_cards]
        assert PUPPET_ONLY_1 in hand_ids, f"Puppet card should be in hand. Hand: {hand_ids}"

    def test_reveal_no_matching_cards_all_to_bottom(self, debug_runner):
        """If revealed 3 have no Puppet or LIBERATOR, all go to deck bottom."""
        runner = _setup_runner(debug_runner, [NO_TRAIT, NO_TRAIT, NO_TRAIT])

        game = runner.game
        p1 = game.player1
        lib_before = len(p1.library_cards)

        action = runner.find_action("Play")
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        # All 3 returned to bottom, net library change = 0
        assert len(p1.library_cards) == lib_before, \
            f"Library should not change when no matches. Was {lib_before}, now {len(p1.library_cards)}"

    def test_reveal_dual_trait_card_used_for_one_pass(self, debug_runner):
        """A card with both Puppet+LIBERATOR selected in pass 1 is removed from pass 2 pool."""
        runner = _setup_runner(debug_runner, [BOTH_TRAITS, NO_TRAIT, NO_TRAIT])

        game = runner.game
        p1 = game.player1

        action = runner.find_action("Play")
        runner.execute(action)
        runner.auto_resolve(max_steps=20)

        # Dual-trait card matches Puppet pass first, gets taken for hand
        # LIBERATOR pass has no remaining matches, so skipped
        hand_ids = [c.c_entity_base.card_id for c in p1.hand_cards]
        assert BOTH_TRAITS in hand_ids, \
            f"Dual-trait card should be in hand. Hand: {hand_ids}"


@pytest.mark.behavioral
class TestST19_03_Inherited:
    """Tests for inherited [Your Turn] opponent security Digimon -3000 DP."""

    def test_inherited_effect_structure(self, debug_runner):
        """Effect 1 must be inherited with EffectTiming.NoTiming, dp_modifier=-3000."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SHOEMON, "ST19-04"])

        # Shoemon is the bottom card source
        shoemon_src = perm.card_sources[0]
        assert shoemon_src.c_entity_base.card_id == SHOEMON

        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited) == 1, f"Should have 1 inherited effect, got {len(inherited)}"

        eff = inherited[0]
        assert eff.dp_modifier == -3000, f"DP modifier should be -3000, got {eff.dp_modifier}"
        assert getattr(eff, '_applies_to_opponent_security_digimon', False), \
            "Must have _applies_to_opponent_security_digimon flag"
        assert eff.timing == EffectTiming.NoTiming, \
            f"Inherited DP effect should use NoTiming, got {eff.timing}"

    def test_inherited_condition_requires_owner_turn(self, debug_runner):
        """Inherited DP reduction only active on owner's turn."""
        runner = debug_runner(initial_memory=5, first_player=1)
        perm = runner.place_on_field(1, [SHOEMON, "ST19-04"])

        shoemon_src = perm.card_sources[0]
        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)][0]

        assert runner.game.player1.is_my_turn, "P1 should be turn player"
        assert inherited.can_use_condition({}), "Condition should pass on owner's turn"

    def test_inherited_condition_fails_on_opponent_turn(self, debug_runner):
        """Inherited DP reduction inactive on opponent's turn."""
        runner = debug_runner(initial_memory=5, first_player=2)
        perm = runner.place_on_field(1, [SHOEMON, "ST19-04"])

        shoemon_src = perm.card_sources[0]
        effects = shoemon_src.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)][0]

        assert not runner.game.player1.is_my_turn, "P1 should NOT be turn player"
        assert not inherited.can_use_condition({}), \
            "Condition should fail on opponent's turn"
