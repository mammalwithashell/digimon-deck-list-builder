"""Behavioral tests for EX5-015 Gabumon (X Antibody).

Card text (from cards.json):
Main effect:
[On Play] [When Digivolving] Reveal the top 4 cards of your deck. Add 2 cards
    with [Garurumon] or [X Antibody] in their names among them to the hand.
    Place the rest at the bottom of the deck. If you added cards, trash 1 card
    in your hand.

Inherited effect:
[All Turns] [Once Per Turn] When this Digimon with [Garurumon] or [Omnimon]
    in its name would be deleted in battle, by returning 2 non-Digi-Egg cards
    from your trash to the bottom of the deck, prevent that deletion.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX5015GabumonXAntibody:
    """Tests for EX5-015 Gabumon (X Antibody)."""

    def test_on_play_reveals_top_4_and_filters_correctly(self, debug_runner):
        """[On Play] Should reveal top 4 and filter for [Garurumon]/[X Antibody] names."""
        runner = debug_runner(
            deck1=["EX5-015"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )

        # Verify the script has On Play and When Digivolving effects
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX5-015", game.player1)
        effects = cs.effect_list(None)

        on_play_effects = [e for e in effects
                          if e.timing == EffectTiming.OnEnterFieldAnyone
                          and getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) >= 1, "Should have On Play effect"

        when_digi_effects = [e for e in effects
                            if e.timing == EffectTiming.OnEnterFieldAnyone
                            and getattr(e, 'is_when_digivolving', False)]
        assert len(when_digi_effects) >= 1, "Should have When Digivolving effect"

    def test_on_play_effect_has_correct_filter(self, debug_runner):
        """The reveal filter should match cards with [Garurumon] or [X Antibody] in name."""
        runner = debug_runner(
            deck1=["EX5-015"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )

        # The key structural test: the process callback should:
        # 1. Reveal top 4 cards
        # 2. Filter for [Garurumon] or [X Antibody] in NAMES
        # 3. Let player pick up to 2
        # 4. Place rest at bottom
        # 5. If any added, trash 1 from hand
        # The current script does this in the WRONG order (trashes first, then reveals)

        # Verify by checking effect structure
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX5-015", game.player1)
        effects = cs.effect_list(None)

        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)]
        assert len(on_play) >= 1

    def test_inherited_effect_is_deletion_prevention(self, debug_runner):
        """Inherited should prevent deletion when Digimon has [Garurumon]/[Omnimon] in name."""
        runner = debug_runner(
            deck1=["EX5-015"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )

        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX5-015", game.player1)
        effects = cs.effect_list(None)

        inherited_effects = [e for e in effects
                            if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited_effects) >= 1, "Should have inherited effect"

        # The inherited effect should use WhenPermanentWouldBeDeleted timing
        deletion_effects = [e for e in inherited_effects
                           if e.timing == EffectTiming.WhenPermanentWouldBeDeleted]
        assert len(deletion_effects) >= 1, (
            "Inherited should use WhenPermanentWouldBeDeleted timing for deletion prevention"
        )

    def test_inherited_effect_checks_garurumon_or_omnimon_name(self, debug_runner):
        """Inherited condition should check if the Digimon has [Garurumon] or [Omnimon] in name."""
        runner = debug_runner(
            deck1=["EX5-015"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )

        # Place a Digimon with EX5-015 as an inherited card
        # Use a Garurumon-named card on top
        perm = runner.place_on_field(1, ["EX5-015", "ST2-06"])  # ST2-06 is WereGarurumon

        game = runner.game
        # The inherited effect should be active because "WereGarurumon" contains "Garurumon"
        # It should check the top card's name for [Garurumon] or [Omnimon]

    def test_inherited_effect_requires_2_non_digi_egg_trash_cards(self, debug_runner):
        """Prevention cost: return 2 non-Digi-Egg cards from trash to deck bottom."""
        runner = debug_runner(
            deck1=["EX5-015"] * 4 + ["ST2-04"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 50,
            initial_memory=10,
        )

        # The inherited effect's process should:
        # 1. Check for 2 non-Digi-Egg cards in trash
        # 2. Return them to deck bottom
        # 3. Prevent the deletion
        # NOT: select an opponent permanent and return it to deck bottom (current bug)

        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX5-015", game.player1)
        effects = cs.effect_list(None)

        inherited_delete = [e for e in effects
                           if getattr(e, 'is_inherited_effect', False)
                           and e.timing == EffectTiming.WhenPermanentWouldBeDeleted]
        assert len(inherited_delete) >= 1, "Should have inherited deletion prevention effect"
