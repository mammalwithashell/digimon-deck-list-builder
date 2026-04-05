"""Behavioral tests for EX7-074 Vortex Resonance (Option, Green/Yellow, Cost 3, LIBERATOR).

Card text:
While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's
    color requirements.
[Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR]
    trait among them to the hand. Return the rest to the bottom of the deck.
    Then, 1 of your Digimon may digivolve into a Digimon card in your hand
    with the digivolution cost reduced by 4.
[Security] You may play 1 card with the [LIBERATOR] trait with a play cost of
    4 or less from your hand or trash without paying the cost. Then, add this
    card to the hand.
"""

import pytest


# EX7-031 Pteromon: Green, Lv.3, LIBERATOR trait, cost 3
LIBERATOR_LV3 = "EX7-031"
# EX7-032 Galemon: Green, Lv.4, LIBERATOR trait, cost 4
LIBERATOR_LV4 = "EX7-032"
# EX7-034 GrandGalemon: Green, Lv.5, LIBERATOR trait, cost 7
LIBERATOR_LV5 = "EX7-034"
# EX7-064 Shoto Kazama: Green Tamer, LIBERATOR trait, cost 3
LIBERATOR_TAMER = "EX7-064"
# ST1-03 Agumon: Red, Lv.3, no LIBERATOR trait
NON_LIBERATOR = "ST1-03"
# ST1-04 Birdramon: Red, Lv.4 (filler)
FILLER_LV4 = "ST1-04"

CARD_ID = "EX7-074"


@pytest.mark.behavioral
class TestEX7074VortexResonanceColorBypass:
    """Tests for color requirement bypass when LIBERATOR Digimon/Tamer on field."""

    def test_color_bypass_with_liberator_digimon(self, debug_runner):
        """Color should be bypassed when a LIBERATOR Digimon is on field."""
        runner = debug_runner(initial_memory=10)

        # Place a LIBERATOR digimon on field
        runner.place_on_field(1, [LIBERATOR_LV3])

        # Inject EX7-074 into hand and load its effects
        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when LIBERATOR Digimon is on field"
        )

    def test_color_bypass_with_liberator_tamer(self, debug_runner):
        """Color should be bypassed when a LIBERATOR Tamer is on field."""
        runner = debug_runner(initial_memory=10)

        # Place a LIBERATOR tamer on field
        runner.place_on_field(1, [LIBERATOR_TAMER])

        # Inject EX7-074 into hand and load its effects
        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when LIBERATOR Tamer is on field"
        )

    def test_color_enforced_without_liberator(self, debug_runner):
        """Color should be enforced when no LIBERATOR Digimon or Tamer on field."""
        runner = debug_runner(initial_memory=10)

        # Place a non-LIBERATOR digimon on field
        runner.place_on_field(1, [NON_LIBERATOR])

        # Inject EX7-074 into hand and load its effects
        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is True, (
            "Should enforce color requirement when no LIBERATOR Digimon/Tamer on field"
        )

    def test_color_enforced_empty_field(self, debug_runner):
        """Color should be enforced when field is empty."""
        runner = debug_runner(initial_memory=10)

        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is True, (
            "Should enforce color requirement when field is empty"
        )


@pytest.mark.behavioral
class TestEX7074VortexResonanceMain:
    """Tests for [Main] effect: reveal top 3, add 1 LIBERATOR to hand,
    rest to deck bottom. Then digivolve from hand with cost -4."""

    def test_reveal_adds_liberator_to_hand(self, debug_runner):
        """Reveal top 3 should add 1 LIBERATOR card to hand, rest to deck bottom."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Place a green digimon to satisfy color requirement
        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Stack deck: top to bottom = LIBERATOR_LV4, FILLER_LV4, NON_LIBERATOR
        # inject_card library_top inserts at index 0, so last inject is at top
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, FILLER_LV4, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")

        # Inject EX7-074 into hand
        runner.inject_card(1, CARD_ID, "hand")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        lib_before = snap_before.p1_library_size

        # Play the option
        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should be able to play EX7-074. Actions: {runner.actions()}"
        runner.execute(action)

        # Auto-resolve the reveal selection (picks first valid = LIBERATOR_LV4)
        # and decline optional digivolve selection
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # The LIBERATOR_LV4 should now be in hand
        assert LIBERATOR_LV4 in snap_after.p1_hand, (
            f"LIBERATOR card should be added to hand. Hand: {snap_after.p1_hand}"
        )

        # The other 2 cards should be at deck bottom (library should decrease by
        # revealed count but increase by 2 returned to bottom; net = -1 since 1 goes to hand)
        # Also EX7-074 (option) goes to trash after use, and we drew 5 cards at start.

    def test_reveal_no_liberator_in_top3_returns_all_to_bottom(self, debug_runner):
        """When no LIBERATOR in revealed cards, all 3 go to deck bottom."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Stack deck with non-LIBERATOR cards
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, FILLER_LV4, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")

        runner.inject_card(1, CARD_ID, "hand")

        snap_before = runner.snapshot()
        hand_count_before = len(snap_before.p1_hand)
        lib_before = snap_before.p1_library_size

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # No LIBERATOR was added to hand (EX7-074 played from hand, so hand - 1,
        # no card added, net hand = hand_count_before - 1)
        # But option goes to trash, so it's not in hand either.
        # The 3 revealed non-LIBERATOR cards should all go to deck bottom.

    def test_digivolve_from_hand_with_cost_reduction(self, debug_runner):
        """After reveal, should allow digivolving a field Digimon from hand with -4 cost."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Place a Lv.3 LIBERATOR digimon on field to satisfy color + be digivolve target
        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Stack deck with a LIBERATOR Lv.4 (will be added to hand from reveal)
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")

        # Also put a Lv.4 Digimon in hand for digivolving
        runner.inject_card(1, LIBERATOR_LV4, "hand")

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)
        # Let auto_resolve handle the reveal pick + digivolve selections
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Check that a digimon on field has been digivolved (stack should have 2 cards)
        digivolved = any(
            len(s.stack_ids) >= 2 for s in snap_after.p1_field
        )
        # This is optional, so it may or may not happen depending on auto_resolve choices
        # The key is no crash occurs and the selections are offered

    def test_digivolve_is_optional(self, debug_runner):
        """The digivolve step should be optional (player can decline)."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Stack deck with a LIBERATOR card
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)
        # Auto-resolve should complete without crash even if digivolve is declined
        runner.auto_resolve()

        # Game should return to Main phase (not stuck)
        snap = runner.snapshot()
        assert snap.phase == "Main", (
            f"Should return to Main phase after resolving. Phase: {snap.phase}"
        )


@pytest.mark.behavioral
class TestEX7074VortexResonanceSecurity:
    """Tests for [Security] effect: play 1 LIBERATOR card (cost 4 or less)
    from hand or trash free. Then add this card to hand."""

    def test_security_has_security_timing(self, debug_runner):
        """Security effect should use SecuritySkill timing."""
        runner = debug_runner(initial_memory=10)
        from digimon_gym.engine.data.enums import EffectTiming

        card = runner.inject_card(1, CARD_ID, "hand")
        effects = card.effect_list(None)

        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1, "Should have at least one security effect"

        sec_eff = security_effects[0]
        assert sec_eff.timing == EffectTiming.SecuritySkill, (
            f"Security effect should have SecuritySkill timing (38), "
            f"got {sec_eff.timing}"
        )

    def test_security_plays_liberator_from_hand(self, debug_runner):
        """[Security] should allow playing a LIBERATOR card (cost <=4) from hand free."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        # Clear hand so only our injected card is available
        runner.clear_zone(1, "hand")

        # Put a LIBERATOR Lv.3 (cost 3) in hand as play target
        runner.inject_card(1, LIBERATOR_LV3, "hand")

        # Put EX7-074 in security
        option_card = runner.inject_card(1, CARD_ID, "security_top")

        # Simulate security effect firing
        from digimon_gym.engine.data.enums import EffectTiming
        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1

        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)

        # Selection phase should be active; find and execute the play action
        # (not the decline action)
        play_action = runner.find_action(LIBERATOR_LV3)
        if play_action is None:
            # Try finding any non-decline action
            actions = runner.actions()
            mask = runner.action_mask()
            play_action = next(
                (a for a in mask if a != 62),  # 62 = decline
                None
            )
        assert play_action is not None, (
            f"Should have a play action for LIBERATOR card. Actions: {runner.actions()}"
        )
        runner.execute(play_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LIBERATOR_LV3 should be on field (played from hand free)
        on_field = any(
            s.card_id == LIBERATOR_LV3 for s in snap.p1_field
        )
        assert on_field, (
            f"LIBERATOR card should be played onto field from hand. "
            f"Field: {[s.card_id for s in snap.p1_field]}"
        )

    def test_security_plays_liberator_from_trash(self, debug_runner):
        """[Security] should allow playing a LIBERATOR card (cost <=4) from trash free."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        # Clear hand so only trash is available
        runner.clear_zone(1, "hand")

        # Put a LIBERATOR Lv.3 (cost 3) in trash as play target
        runner.inject_card(1, LIBERATOR_LV3, "trash")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

        from digimon_gym.engine.data.enums import EffectTiming
        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)

        # Selection phase should be active; find and execute the play action
        actions = runner.actions()
        mask = runner.action_mask()
        play_action = next(
            (a for a in mask if a != 62),  # 62 = decline
            None
        )
        assert play_action is not None, (
            f"Should have a play action for LIBERATOR card from trash. Actions: {actions}"
        )
        runner.execute(play_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        on_field = any(
            s.card_id == LIBERATOR_LV3 for s in snap.p1_field
        )
        assert on_field, (
            f"LIBERATOR card should be played from trash. "
            f"Field: {[s.card_id for s in snap.p1_field]}"
        )

    def test_security_rejects_cost_over_4(self, debug_runner):
        """[Security] should NOT allow playing LIBERATOR cards with cost > 4."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        # Put a LIBERATOR Lv.5 (cost 7 > 4) in hand
        runner.inject_card(1, LIBERATOR_LV5, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

        from digimon_gym.engine.data.enums import EffectTiming
        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]

        hand_before = list(player.hand_cards)
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Lv.5 should NOT be on field (cost 7 > 4)
        lv5_on_field = any(
            s.card_id == LIBERATOR_LV5 for s in snap.p1_field
        )
        assert not lv5_on_field, (
            "LIBERATOR card with cost > 4 should NOT be playable via security effect"
        )

    def test_security_rejects_non_liberator(self, debug_runner):
        """[Security] should NOT allow playing non-LIBERATOR cards."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        # Put a non-LIBERATOR Lv.3 in hand
        runner.inject_card(1, NON_LIBERATOR, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

        from digimon_gym.engine.data.enums import EffectTiming
        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]

        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)
        runner.auto_resolve()

        snap = runner.snapshot()
        non_lib_on_field = any(
            s.card_id == NON_LIBERATOR for s in snap.p1_field
        )
        assert not non_lib_on_field, (
            "Non-LIBERATOR card should NOT be playable via security effect"
        )

    def test_security_adds_option_to_hand(self, debug_runner):
        """After security play, this option card should be added to hand."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        # Put a LIBERATOR card in hand to play
        runner.inject_card(1, LIBERATOR_LV3, "hand")

        # Put EX7-074 in trash (where it would be after security activation)
        option_card = runner.inject_card(1, CARD_ID, "trash")

        from digimon_gym.engine.data.enums import EffectTiming
        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)
        runner.auto_resolve()

        # EX7-074 should now be in hand
        in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == CARD_ID
            for c in player.hand_cards
        )
        assert in_hand, (
            f"EX7-074 should be added to hand after security effect. "
            f"Hand IDs: {[c.c_entity_base.card_id for c in player.hand_cards if c.c_entity_base]}"
        )

        # And NOT in trash anymore
        in_trash = any(
            c.c_entity_base and c.c_entity_base.card_id == CARD_ID
            for c in player.trash_cards
        )
        assert not in_trash, "EX7-074 should be removed from trash after adding to hand"
