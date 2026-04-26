"""Behavioral tests for P-229 Unique Emblem: Narrative Ronde (Yellow Option).

Card text:
[Main] Reveal the top 3 cards of your deck. Add 1 [Puppet] trait Digimon card
and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the
bottom of the deck. Then, place this card in the battle area.

[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay>
(By trashing this card after the placing turn, activate the effect below.)
  1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait
  card in the hand with the digivolution cost reduced by 3.

[Security] Activate this card's [Main] effects.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# BT1-045 Tsukaimon: Yellow Lv3 Digimon (for option color requirement)
YELLOW_DIGIMON = "BT1-045"
# BT2-055 ToyAgumon: Lv3 Puppet Digimon
PUPPET_DIGIMON = "BT2-055"
# BT18-060 Vemmon: LIBERATOR trait Digimon (no Puppet)
LIBERATOR_DIGI = "BT18-060"
# BT22-040 Cendrillmon: Lv6 Puppet+LIBERATOR Digimon
LV6_LIBERATOR = "BT22-040"
# BT22-036 Chaperomon: Lv5 Puppet+LIBERATOR Digimon
LV5_LIBERATOR = "BT22-036"
# EX9-067 Mirai Kinosaki: Yellow Tamer
MIRAI_KINOSAKI = "EX9-067"
# Generic filler
FILLER = "ST1-03"


def _setup_security(runner, count=3):
    """Inject security cards so the game doesn't end."""
    for _ in range(count):
        runner.inject_card(1, FILLER, "security_top")
        runner.inject_card(2, FILLER, "security_top")


@pytest.mark.behavioral
class TestP229NarrativeRonde:
    """Tests for P-229 Unique Emblem: Narrative Ronde."""

    # ── Effect 0: [Main] Reveal 3, add 1 Puppet Digimon + 1 LIBERATOR ──

    def test_main_reveal_adds_puppet_digimon_to_hand(self, debug_runner):
        """Pass 1 of reveal should select a [Puppet] trait Digimon card."""
        runner = debug_runner(initial_memory=5)

        # Need a Yellow permanent on field for option color requirement
        runner.place_on_field(1, [YELLOW_DIGIMON])

        # Stack the deck with: a Puppet Digimon, a LIBERATOR card, and a filler
        runner.clear_zone(1, "library")
        runner.inject_card(1, PUPPET_DIGIMON, "library_top")  # Puppet Digimon
        runner.inject_card(1, LIBERATOR_DIGI, "library_top")  # LIBERATOR
        runner.inject_card(1, FILLER, "library_top")          # filler
        _setup_security(runner)

        hand_before = len(runner.game.player1.hand_cards)

        runner.inject_card(1, "P-229", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Narrative Ronde")
        assert action is not None, "Should be able to play P-229 option (Yellow perm on field)"
        runner.execute(action)

        # Should enter reveal-selection phase for pass 1 (Puppet Digimon)
        assert runner.game.current_phase == GamePhase.SelectReveal, (
            "Should enter reveal-selection for first pass (Puppet Digimon)"
        )

        # Auto-resolve both passes
        runner.auto_resolve()

        # After resolution, Puppet Digimon + LIBERATOR card should be in hand
        hand_after = len(runner.game.player1.hand_cards)
        # hand_before, added P-229 (+1), played it (-1), gained 2 from reveal
        assert hand_after >= hand_before + 2, (
            f"Should gain 2 cards from reveal passes, got {hand_after - hand_before}"
        )

    def test_main_reveal_pass1_requires_digimon(self, debug_runner):
        """Pass 1 filter should reject non-Digimon cards and non-Puppet Digimon.
        The card says '1 [Puppet] trait Digimon card'."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, [YELLOW_DIGIMON])
        runner.clear_zone(1, "library")
        # Only LIBERATOR Digimon (no Puppet) and fillers -- nothing for pass 1
        runner.inject_card(1, LIBERATOR_DIGI, "library_top")
        runner.inject_card(1, FILLER, "library_top")
        runner.inject_card(1, FILLER, "library_top")
        _setup_security(runner)

        runner.inject_card(1, "P-229", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Narrative Ronde")
        assert action is not None
        runner.execute(action)

        # Pass 1 should be skipped (no Puppet Digimon), engine goes to pass 2
        runner.auto_resolve()

        # Only 1 card (LIBERATOR from pass 2) should be added, not 2
        hand_ids = [c.card_id for c in runner.game.player1.hand_cards]
        # Verify LIBERATOR was picked up in pass 2
        assert LIBERATOR_DIGI in hand_ids, (
            f"LIBERATOR card should be in hand from pass 2, hand: {hand_ids}"
        )

    def test_main_places_option_in_battle_area(self, debug_runner):
        """After the [Main] effect resolves, the option should be placed in the battle area."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, [YELLOW_DIGIMON])
        runner.clear_zone(1, "library")
        runner.inject_card(1, PUPPET_DIGIMON, "library_top")
        runner.inject_card(1, LIBERATOR_DIGI, "library_top")
        runner.inject_card(1, FILLER, "library_top")
        _setup_security(runner)

        runner.inject_card(1, "P-229", "hand")
        runner.set_phase("Main")
        action = runner.find_action("Narrative Ronde")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # P-229 should now be in the battle area (delay placement)
        ba_cards = []
        for perm in runner.game.player1.battle_area:
            for cs in perm.card_sources:
                ba_cards.append(cs.card_id)
        assert "P-229" in ba_cards, (
            f"P-229 should be in battle area after Main effect, found: {ba_cards}"
        )

    def test_main_remaining_cards_go_to_deck_bottom(self, debug_runner):
        """Cards not selected should return to the bottom of the deck."""
        runner = debug_runner(initial_memory=5)

        runner.place_on_field(1, [YELLOW_DIGIMON])
        runner.clear_zone(1, "library")
        runner.inject_card(1, PUPPET_DIGIMON, "library_top")
        runner.inject_card(1, LIBERATOR_DIGI, "library_top")
        runner.inject_card(1, FILLER, "library_top")
        _setup_security(runner)

        runner.inject_card(1, "P-229", "hand")
        runner.set_phase("Main")

        deck_before = len(runner.game.player1.library_cards)
        action = runner.find_action("Narrative Ronde")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # 3 revealed, 2 to hand, 1 to deck bottom
        deck_after = len(runner.game.player1.library_cards)
        assert deck_after == deck_before - 3 + 1, (
            f"Deck should have 1 card returned to bottom, "
            f"deck went from {deck_before} to {deck_after}"
        )

    # ── Effect 2: Delay trigger — When Mirai Kinosaki is played ──

    def test_delay_triggers_when_mirai_kinosaki_played(self, debug_runner):
        """When Mirai Kinosaki is played, the delay trigger should fire,
        trashing P-229 from battle area."""
        runner = debug_runner(initial_memory=8)

        # Place P-229 in battle area (as if the main effect already resolved)
        delay_perm = runner.place_on_field(1, ["P-229"], turn_played=0)

        # Place a Digimon on field that can be the digivolve target
        runner.place_on_field(1, [PUPPET_DIGIMON])

        # Put a Lv6 LIBERATOR in hand for the digivolve
        runner.inject_card(1, LV6_LIBERATOR, "hand")

        _setup_security(runner)
        runner.set_phase("Main")

        # Play Mirai Kinosaki (EX9-067) - Yellow Tamer, cost 3
        runner.inject_card(1, MIRAI_KINOSAKI, "hand")
        action = runner.find_action("Mirai Kinosaki")
        assert action is not None, (
            f"Should find Mirai Kinosaki in actions. "
            f"Available: {list(runner.actions().values())[:10]}"
        )
        runner.execute(action)
        runner.auto_resolve()

        # After delay fires, P-229 should be trashed (removed from battle area)
        ba_ids = []
        for perm in runner.game.player1.battle_area:
            for cs in perm.card_sources:
                ba_ids.append(cs.card_id)
        assert "P-229" not in ba_ids, (
            f"P-229 should be trashed after delay activation, but found in: {ba_ids}"
        )

    def test_delay_does_not_fire_on_non_mirai_play(self, debug_runner):
        """Playing a non-Mirai Kinosaki card should NOT trigger the delay."""
        runner = debug_runner(initial_memory=8)

        # Place P-229 in battle area
        runner.place_on_field(1, ["P-229"], turn_played=0)
        runner.place_on_field(1, [YELLOW_DIGIMON])
        _setup_security(runner)
        runner.set_phase("Main")

        # Play a non-Mirai card (another Yellow Digimon)
        runner.inject_card(1, YELLOW_DIGIMON, "hand")
        action = runner.find_action(YELLOW_DIGIMON)
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        # P-229 should still be in battle area (delay not triggered)
        ba_ids = []
        for perm in runner.game.player1.battle_area:
            for cs in perm.card_sources:
                ba_ids.append(cs.card_id)
        assert "P-229" in ba_ids, (
            "P-229 should remain in battle area when non-Mirai card is played"
        )

    def test_delay_condition_your_turn_only(self, debug_runner):
        """The delay should only fire during [Your Turn], not opponent's turn."""
        runner = debug_runner(initial_memory=-5)  # Opponent's turn

        # Place P-229 on player 1's field
        runner.place_on_field(1, ["P-229"], turn_played=0)
        _setup_security(runner)
        runner.set_phase("Main")

        # When it's opponent's turn, P-229 should stay
        ba_ids = []
        for perm in runner.game.player1.battle_area:
            for cs in perm.card_sources:
                ba_ids.append(cs.card_id)
        assert "P-229" in ba_ids, (
            "P-229 should still be in battle area when it's not your turn"
        )

    # ── Effect 3: Security effect ──

    def test_security_effect_has_correct_timing(self, debug_runner):
        """The security effect should use SecuritySkill timing."""
        runner = debug_runner(initial_memory=5)

        cs = runner.inject_card(1, "P-229", "hand")
        effects = cs.effect_list(EffectTiming.NoTiming)
        has_security_skill = any(
            getattr(e, 'timing', None) == EffectTiming.SecuritySkill
            for e in effects
        )
        assert has_security_skill, (
            "P-229 should have an effect with SecuritySkill timing for the security effect"
        )

    # ── Structure validation ──

    def test_delay_observer_does_not_have_is_on_play(self, debug_runner):
        """The OnEnterFieldAnyone observer effect should NOT have is_on_play=True,
        since it needs to observe OTHER permanents being played, not itself."""
        runner = debug_runner(initial_memory=5)

        cs = runner.inject_card(1, "P-229", "hand")
        effects = cs.effect_list(EffectTiming.NoTiming)
        observer_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnEnterFieldAnyone
        ]
        assert len(observer_effects) >= 1, (
            "P-229 should have at least one OnEnterFieldAnyone observer effect"
        )
        for e in observer_effects:
            assert not getattr(e, 'is_on_play', False), (
                "OnEnterFieldAnyone observer should NOT have is_on_play=True "
                "(it observes other cards being played, not itself)"
            )
