"""Behavioral tests for BT16-025 Paildramon.

Card text:
  Lv.5 | Blue/Green | Dragonkin | DP:8000 | Cost:8 | Attribute: Free
  Partition (Blue Lv.4 & Green Lv.4)
  [When Digivolving] Suspend all of your opponent's Digimon with as many or
  fewer digivolution cards as this Digimon. Then, if DNA digivolving, none of
  their Digimon can unsuspend until the end of their turn.
  [When Attacking] [Once Per Turn] Suspend 1 of your opponent's unsuspended
  Digimon. If this effect didn't suspend, unsuspend this Digimon.

  Inherited: Partition (Blue Lv.4 & Green Lv.4)

Key faithfulness points tested:
  - When Digivolving: suspends ALL opponent Digimon with <= this Digimon's
    digi-card count
  - Digi-card count = len(card_sources) - 1 (cards below top)
  - Does NOT suspend opponent Digimon with MORE digi-cards
  - DNA clause: ALL opponent Digimon get CANNOT_UNSUSPEND until end of their turn
  - When Attacking: Once Per Turn, suspend 1 opponent unsuspended Digimon
  - When Attacking: If no suspension happened, unsuspend self
  - Partition effects exist (non-inherited and inherited)
"""

import pytest
from engine_py_legacy.engine.interfaces.modifiers import ModifierType
from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
BT16_025 = "BT16-025"          # Card under test (Paildramon, Lv.5 Blue/Green)
AGUMON_ST1 = "ST1-03"          # Agumon (Lv.3, Red)
GREYMON_ST1 = "ST1-07"         # Greymon (Lv.4, Red)
METALGREYMON_ST1 = "ST1-09"    # MetalGreymon (Lv.5, Red)


@pytest.mark.behavioral
class TestBT16025Partition:
    """Partition effects should exist on both the card and as inherited."""

    def test_has_partition_effect(self, debug_runner):
        """Should have a non-inherited Partition effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_025])

        top = perm.top_card
        all_effects = top.effect_list(None)
        partitions = [e for e in all_effects if getattr(e, '_is_partition', False)
                      and not getattr(e, 'is_inherited_effect', False)]
        assert len(partitions) >= 1, "Should have a non-inherited Partition effect"

    def test_has_inherited_partition_effect(self, debug_runner):
        """Should have an inherited Partition effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_025])

        top = perm.top_card
        all_effects = top.effect_list(None)
        inherited_partitions = [e for e in all_effects
                                if getattr(e, '_is_partition', False)
                                and getattr(e, 'is_inherited_effect', False)]
        assert len(inherited_partitions) >= 1, "Should have an inherited Partition effect"


@pytest.mark.behavioral
class TestBT16025WhenDigivolving:
    """[When Digivolving] Suspend opponent Digimon with <= this Digimon's digi-cards."""

    def test_suspends_opponent_with_fewer_digi_cards(self, debug_runner):
        """Should suspend opponent Digimon with fewer digivolution cards."""
        runner = debug_runner(initial_memory=10)

        # Place BT16-025 with 2 digi-cards (3 cards total: egg/lv3 + lv4 + BT16-025 on top)
        perm = runner.place_on_field(1, [AGUMON_ST1, GREYMON_ST1, BT16_025])
        my_digi_count = len(perm.card_sources) - 1  # = 2

        # Place opponent Digimon with 1 digi-card (2 cards in stack)
        opp1 = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])
        opp1_digi = len(opp1.card_sources) - 1  # = 1
        assert opp1_digi <= my_digi_count

        game = runner.game

        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        assert opp1.is_suspended is True, (
            f"Opponent Digimon with {opp1_digi} digi-cards should be suspended "
            f"(this Digimon has {my_digi_count})"
        )

    def test_suspends_opponent_with_equal_digi_cards(self, debug_runner):
        """Should suspend opponent Digimon with EQUAL digivolution card count."""
        runner = debug_runner(initial_memory=10)

        # Both have 1 digi-card
        perm = runner.place_on_field(1, [AGUMON_ST1, BT16_025])  # 1 digi-card
        opp = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])  # 1 digi-card

        game = runner.game
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        assert opp.is_suspended is True, (
            "Opponent Digimon with equal digi-card count should be suspended"
        )

    def test_does_not_suspend_opponent_with_more_digi_cards(self, debug_runner):
        """Should NOT suspend opponent Digimon with MORE digivolution cards."""
        runner = debug_runner(initial_memory=10)

        # P1 Paildramon with 1 digi-card
        perm = runner.place_on_field(1, [AGUMON_ST1, BT16_025])  # 1 digi-card

        # Opponent has 3 digi-cards (4 total)
        opp = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1, AGUMON_ST1])
        opp_digi = len(opp.card_sources) - 1  # = 3

        game = runner.game
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        assert opp.is_suspended is False, (
            f"Opponent Digimon with {opp_digi} digi-cards should NOT be suspended "
            f"(this Digimon has 1)"
        )

    def test_suspends_opponent_with_zero_digi_cards(self, debug_runner):
        """Opponent Digimon with 0 digi-cards (single card) should always
        be suspended, since 0 <= any count."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [AGUMON_ST1, BT16_025])  # 1 digi-card
        opp = runner.place_on_field(2, [AGUMON_ST1])  # 0 digi-cards

        game = runner.game
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        assert opp.is_suspended is True, (
            "Opponent with 0 digi-cards should be suspended"
        )


@pytest.mark.behavioral
class TestBT16025DNAClause:
    """DNA clause: none of opponent's Digimon can unsuspend until end of their turn."""

    def test_dna_applies_cannot_unsuspend_to_all_opponent_digimon(self, debug_runner):
        """When DNA digivolving, ALL opponent Digimon should get CANNOT_UNSUSPEND."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [AGUMON_ST1, BT16_025])

        # Place 3 opponent Digimon
        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])
        opp3 = runner.place_on_field(2, [METALGREYMON_ST1])

        game = runner.game
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': True,
        })

        # All opponent Digimon should have CANNOT_UNSUSPEND
        for opp_perm in [opp1, opp2, opp3]:
            has_mod = game.modifiers.has_modifier(opp_perm, ModifierType.CANNOT_UNSUSPEND)
            assert has_mod, (
                f"All opponent Digimon should have CANNOT_UNSUSPEND when DNA. "
                f"Missing on permanent with top card: "
                f"{opp_perm.top_card.c_entity_base.card_id if opp_perm.top_card else 'unknown'}"
            )

    def test_no_dna_no_cannot_unsuspend(self, debug_runner):
        """Without DNA, should NOT apply CANNOT_UNSUSPEND."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [AGUMON_ST1, BT16_025])
        opp1 = runner.place_on_field(2, [AGUMON_ST1])

        game = runner.game
        top = perm.top_card
        wd_effect = [e for e in top.effect_list(None) if getattr(e, 'is_when_digivolving', False)][0]

        wd_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'is_dna_digivolve': False,
        })

        has_mod = game.modifiers.has_modifier(opp1, ModifierType.CANNOT_UNSUSPEND)
        assert not has_mod, (
            "Without DNA, opponent Digimon should NOT have CANNOT_UNSUSPEND"
        )


@pytest.mark.behavioral
class TestBT16025WhenAttacking:
    """[When Attacking] [Once Per Turn] Suspend 1 opp unsuspended Digimon.
    If didn't suspend, unsuspend self."""

    def test_suspends_one_opponent_digimon(self, debug_runner):
        """Should suspend one unsuspended opponent Digimon when attacking."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_025])
        opp1 = runner.place_on_field(2, [AGUMON_ST1])
        opp2 = runner.place_on_field(2, [GREYMON_ST1])

        game = runner.game

        top = perm.top_card
        attack_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_attack', False)
        ]
        assert len(attack_effects) >= 1, "Should have a When Attacking effect"

        attack_effect = attack_effects[0]

        attack_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Exactly 1 of the opponent Digimon should be suspended
        suspended_count = sum(1 for p in game.player2.battle_area if p.is_suspended)
        assert suspended_count == 1, (
            f"Exactly 1 opponent Digimon should be suspended, got {suspended_count}"
        )

    def test_unsuspends_self_when_no_targets(self, debug_runner):
        """If no opponent unsuspended Digimon, this Digimon should unsuspend."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_025])
        perm.suspend()  # Simulate being suspended from declaring attack
        assert perm.is_suspended is True

        # No opponent Digimon at all
        game = runner.game

        top = perm.top_card
        attack_effect = [e for e in top.effect_list(None) if getattr(e, 'is_on_attack', False)][0]

        attack_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert perm.is_suspended is False, (
            "With no valid targets, this Digimon should unsuspend itself"
        )

    def test_unsuspends_self_when_all_opponents_already_suspended(self, debug_runner):
        """If all opponent Digimon are already suspended, should unsuspend self."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_025])
        perm.suspend()

        # Place opponent Digimon that are already suspended
        opp1 = runner.place_on_field(2, [AGUMON_ST1], is_suspended=True)
        opp2 = runner.place_on_field(2, [GREYMON_ST1], is_suspended=True)

        game = runner.game
        top = perm.top_card
        attack_effect = [e for e in top.effect_list(None) if getattr(e, 'is_on_attack', False)][0]

        attack_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert perm.is_suspended is False, (
            "With all opponents already suspended, should unsuspend self"
        )

    def test_does_not_unsuspend_self_when_suspended_target(self, debug_runner):
        """If a target WAS suspended, should NOT unsuspend self."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_025])
        perm.suspend()

        opp1 = runner.place_on_field(2, [AGUMON_ST1])  # unsuspended target

        game = runner.game
        top = perm.top_card
        attack_effect = [e for e in top.effect_list(None) if getattr(e, 'is_on_attack', False)][0]

        attack_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # The opponent should be suspended
        assert opp1.is_suspended is True, "Target should be suspended"
        # Self should remain suspended (did suspend a target)
        assert perm.is_suspended is True, (
            "Should NOT unsuspend self when a target was suspended"
        )

    def test_once_per_turn_limit(self, debug_runner):
        """The effect should be limited to once per turn."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_025])

        top = perm.top_card
        attack_effects = [e for e in top.effect_list(None) if getattr(e, 'is_on_attack', False)]
        assert len(attack_effects) >= 1

        attack_effect = attack_effects[0]
        assert attack_effect.max_count_per_turn == 1, (
            f"When Attacking effect should be Once Per Turn, "
            f"got max_count_per_turn={attack_effect.max_count_per_turn}"
        )
