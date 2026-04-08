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


# EX7-031 Pteromon: Green, Lv.3, LIBERATOR trait, cost 3, Bird Dragon
LIBERATOR_LV3 = "EX7-031"
# EX7-032 Galemon: Green, Lv.4, LIBERATOR trait, cost 4, Bird Dragon
LIBERATOR_LV4 = "EX7-032"
# EX7-034 GrandGalemon: Green, Lv.5, LIBERATOR trait, cost 7
LIBERATOR_LV5 = "EX7-034"
# EX7-064 Shoto Kazama: Green Tamer, LIBERATOR trait, cost 3
LIBERATOR_TAMER = "EX7-064"
# ST1-03 Agumon: Red, Lv.3, no LIBERATOR trait
NON_LIBERATOR = "ST1-03"
# ST1-04 Birdramon: Red, Lv.4 (filler, no LIBERATOR)
FILLER_LV4 = "ST1-04"

CARD_ID = "EX7-074"


@pytest.mark.behavioral
class TestEX7074ColorBypass:
    """Tests for color requirement bypass when LIBERATOR Digimon/Tamer on field."""

    def test_color_bypass_with_liberator_digimon(self, debug_runner):
        """Color should be bypassed when a LIBERATOR Digimon is on field."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [LIBERATOR_LV3])

        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when LIBERATOR Digimon is on field"
        )

    def test_color_bypass_with_liberator_tamer(self, debug_runner):
        """Color should be bypassed when a LIBERATOR Tamer is on field."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [LIBERATOR_TAMER])

        card = runner.inject_card(1, CARD_ID, "hand")
        card.effect_list(None)

        assert card.match_color_requirement is False, (
            "Should bypass color requirement when LIBERATOR Tamer is on field"
        )

    def test_color_enforced_without_liberator(self, debug_runner):
        """Color should be enforced when no LIBERATOR Digimon or Tamer on field."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, [NON_LIBERATOR])

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
class TestEX7074MainReveal:
    """Tests for [Main] effect: reveal top 3, add 1 LIBERATOR to hand,
    rest to deck bottom."""

    def test_reveal_adds_liberator_to_hand_precisely(self, debug_runner):
        """Reveal top 3: 1 LIBERATOR goes to hand, 2 others go to deck bottom."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Place a green LIBERATOR digimon to satisfy color requirement
        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Clear hand so we can track precisely
        runner.clear_zone(1, "hand")

        # Stack deck: top -> LIBERATOR_LV4, FILLER_LV4, NON_LIBERATOR
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, FILLER_LV4, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")

        lib_size_before = len(player.library_cards)

        # Inject EX7-074 into hand
        runner.inject_card(1, CARD_ID, "hand")

        # Play the option
        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None, f"Should be able to play EX7-074. Actions: {runner.actions()}"
        runner.execute(action)

        # Auto-resolve: picks LIBERATOR_LV4, declines optional digivolve
        runner.auto_resolve()

        snap = runner.snapshot()

        # LIBERATOR_LV4 should be in hand (added from reveal)
        assert LIBERATOR_LV4 in snap.p1_hand, (
            f"LIBERATOR card should be added to hand. Hand: {snap.p1_hand}"
        )

        # The 2 non-LIBERATOR cards should be at deck bottom
        # Library should lose 3 (revealed) + gain 2 (returned to bottom) = net -1
        assert snap.p1_library_size == lib_size_before - 1, (
            f"Library should lose 1 card (net). Before: {lib_size_before}, "
            f"After: {snap.p1_library_size}"
        )

        # Verify the 2 returned cards are at the bottom of the library
        bottom_two_ids = [
            c.c_entity_base.card_id for c in player.library_cards[-2:]
        ]
        assert FILLER_LV4 in bottom_two_ids, (
            f"FILLER_LV4 should be at deck bottom. Bottom 2: {bottom_two_ids}"
        )
        assert NON_LIBERATOR in bottom_two_ids, (
            f"NON_LIBERATOR should be at deck bottom. Bottom 2: {bottom_two_ids}"
        )

    def test_reveal_no_liberator_returns_all_to_bottom(self, debug_runner):
        """When no LIBERATOR in revealed cards, all 3 go to deck bottom, nothing added to hand."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")

        # Stack deck with non-LIBERATOR cards only
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, FILLER_LV4, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")

        lib_size_before = len(player.library_cards)

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()

        # No card added to hand (hand should be empty since we cleared it and
        # EX7-074 was played from hand to trash)
        assert len(snap.p1_hand) == 0, (
            f"No cards should be added to hand when no LIBERATOR in top 3. "
            f"Hand: {snap.p1_hand}"
        )

        # All 3 revealed cards go back to deck bottom: net library change = 0
        assert snap.p1_library_size == lib_size_before, (
            f"Library should not change size when all revealed go to bottom. "
            f"Before: {lib_size_before}, After: {snap.p1_library_size}"
        )

    def test_reveal_multiple_liberator_offers_selection(self, debug_runner):
        """When multiple LIBERATOR cards in top 3, player should get to choose one."""
        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")

        # Stack deck: 2 LIBERATOR cards + 1 non-LIBERATOR
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")
        runner.inject_card(1, LIBERATOR_LV3, "library_top")  # second LIBERATOR

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)

        # After playing, should be in a SelectReveal phase
        from digimon_gym.engine.data.enums import GamePhase
        assert runner.game.current_phase == GamePhase.SelectReveal, (
            f"Should be in SelectReveal phase. Phase: {runner.game.current_phase}"
        )

        # Should have at least 2 selection options (both LIBERATOR cards)
        legal = runner.action_mask()
        assert len(legal) >= 2, (
            f"Should offer at least 2 choices for 2 LIBERATOR cards. "
            f"Legal actions: {runner.actions()}"
        )

        # Auto-resolve picks first valid, then resolves rest
        runner.auto_resolve()

        snap = runner.snapshot()
        # Exactly 1 LIBERATOR should be in hand
        liberator_in_hand = [
            cid for cid in snap.p1_hand
            if cid in (LIBERATOR_LV3, LIBERATOR_LV4)
        ]
        assert len(liberator_in_hand) == 1, (
            f"Exactly 1 LIBERATOR should be added to hand. Hand: {snap.p1_hand}"
        )


@pytest.mark.behavioral
class TestEX7074MainDigivolve:
    """Tests for the digivolve step: 1 of your Digimon may digivolve into
    a Digimon card in your hand with cost -4."""

    def test_digivolve_from_hand_with_cost_reduction(self, debug_runner):
        """After reveal, should digivolve a Digimon from hand with cost -4."""
        from digimon_gym.engine.data.enums import GamePhase

        runner = debug_runner(initial_memory=10)
        player = runner.game.player1

        # Place a Lv.3 LIBERATOR digimon on field
        runner.place_on_field(1, [LIBERATOR_LV3])
        runner.set_phase("Main")

        # Stack deck with non-LIBERATOR so reveal doesn't add to hand
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")

        # Put a Lv.4 Digimon in hand for digivolving (LIBERATOR_LV4, play cost 4)
        runner.inject_card(1, LIBERATOR_LV4, "hand")

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)

        # After reveal (no LIBERATOR found, auto-resolved), we enter digivolve
        # selection. We need to manually pick the non-decline option since
        # auto_resolve picks the first legal action (which is decline for optionals).
        # Step 1: Select the Digimon target to digivolve
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget for digivolve. Phase: {runner.game.current_phase}"
        )
        legal = runner.action_mask()
        # Find the non-decline action (permanent selection)
        non_decline = [a for a in legal if a != 62]
        assert len(non_decline) >= 1, (
            f"Should have at least 1 target permanent. Legal: {runner.actions()}"
        )
        runner.execute(non_decline[0])  # Select the Lv.3 Digimon

        # Step 2: Select hand card to digivolve into
        # (may auto-resolve if only one valid card)
        runner.auto_resolve()

        snap = runner.snapshot()

        # Check that a digimon on field has been digivolved (stack should have 2 cards)
        digivolved = any(
            len(s.stack_ids) >= 2 for s in snap.p1_field
        )
        assert digivolved, (
            f"A Digimon should have been digivolved. "
            f"Field: {[(s.card_id, s.stack_ids) for s in snap.p1_field]}"
        )

    def test_digivolve_is_optional(self, debug_runner):
        """The digivolve step should be optional (player can decline)."""
        runner = debug_runner(initial_memory=10)

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

        snap = runner.snapshot()
        assert snap.phase == "Main", (
            f"Should return to Main phase after resolving. Phase: {snap.phase}"
        )

    def test_digivolve_step_only_targets_digimon(self, debug_runner):
        """Digivolve target selection should only offer Digimon, not Tamers."""
        runner = debug_runner(initial_memory=10)

        # Place both a Digimon and a Tamer
        runner.place_on_field(1, [LIBERATOR_LV3])  # Digimon
        runner.place_on_field(1, [LIBERATOR_TAMER])  # Tamer
        runner.set_phase("Main")

        # Stack deck with no LIBERATOR
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")

        # Put a Lv.4 digimon in hand
        runner.inject_card(1, LIBERATOR_LV4, "hand")

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)

        # Reveal resolves (no LIBERATOR), then digivolve selection
        # auto_resolve handles it
        runner.auto_resolve()

        snap = runner.snapshot()
        # The Tamer should still be on field unchanged (not digivolved onto)
        tamer_on_field = any(
            s.card_id == LIBERATOR_TAMER and s.is_tamer and len(s.stack_ids) == 1
            for s in snap.p1_field
        )
        assert tamer_on_field, (
            f"Tamer should remain unchanged (not a digivolve target). "
            f"Field: {[(s.card_id, s.is_tamer, s.stack_ids) for s in snap.p1_field]}"
        )

    def test_no_digivolve_offered_when_no_digimon_on_field(self, debug_runner):
        """When no Digimon on field, digivolve step should be skipped."""
        runner = debug_runner(initial_memory=10)

        # Only a Tamer on field (no Digimon) - still satisfies color bypass
        runner.place_on_field(1, [LIBERATOR_TAMER])
        runner.set_phase("Main")

        # Stack deck with LIBERATOR
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, NON_LIBERATOR, "library_top")
        runner.inject_card(1, LIBERATOR_LV4, "library_top")

        runner.inject_card(1, CARD_ID, "hand")

        action = runner.find_action("Vortex Resonance")
        if action is None:
            action = runner.find_action(CARD_ID)
        assert action is not None
        runner.execute(action)

        runner.auto_resolve()

        snap = runner.snapshot()
        # Should resolve cleanly and return to Main phase
        assert snap.phase == "Main", (
            f"Should return to Main phase. Phase: {snap.phase}"
        )


@pytest.mark.behavioral
class TestEX7074SecurityEffect:
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

    def test_security_plays_liberator_digimon_from_hand(self, debug_runner):
        """[Security] should allow playing a LIBERATOR Digimon (cost <=4) from hand free."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put a LIBERATOR Lv.3 (cost 3) in hand as play target
        runner.inject_card(1, LIBERATOR_LV3, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        assert len(security_effects) >= 1

        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)

        # Should have a selection phase for the LIBERATOR card
        runner.auto_resolve()

        snap = runner.snapshot()
        on_field = any(
            s.card_id == LIBERATOR_LV3 for s in snap.p1_field
        )
        assert on_field, (
            f"LIBERATOR Digimon should be played onto field from hand. "
            f"Field: {[s.card_id for s in snap.p1_field]}"
        )

    def test_security_plays_liberator_tamer_from_hand(self, debug_runner):
        """[Security] should allow playing a LIBERATOR Tamer (cost <=4) from hand free."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put LIBERATOR tamer (cost 3) in hand
        runner.inject_card(1, LIBERATOR_TAMER, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

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
        tamer_on_field = any(
            s.card_id == LIBERATOR_TAMER and s.is_tamer for s in snap.p1_field
        )
        assert tamer_on_field, (
            f"LIBERATOR Tamer should be played onto field from hand. "
            f"Field: {[(s.card_id, s.is_tamer) for s in snap.p1_field]}"
        )

    def test_security_plays_liberator_from_trash(self, debug_runner):
        """[Security] should allow playing a LIBERATOR card (cost <=4) from trash free."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put a LIBERATOR Lv.3 (cost 3) in trash as play target
        runner.inject_card(1, LIBERATOR_LV3, "trash")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)

        # Manually select the trash card (non-decline) since auto_resolve
        # picks action 62 (decline) first for optional selections
        legal = runner.action_mask()
        non_decline = [a for a in legal if a != 62]
        assert len(non_decline) >= 1, (
            f"Should have at least 1 trash card to play. Legal: {runner.actions()}"
        )
        runner.execute(non_decline[0])
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

        runner.clear_zone(1, "hand")

        # Put a LIBERATOR Lv.5 (cost 7 > 4) in hand
        runner.inject_card(1, LIBERATOR_LV5, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

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

        runner.clear_zone(1, "hand")

        # Put a non-LIBERATOR Lv.3 in hand
        runner.inject_card(1, NON_LIBERATOR, "hand")

        option_card = runner.inject_card(1, CARD_ID, "security_top")

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

    def test_security_play_is_optional(self, debug_runner):
        """[Security] play is optional ('You may play'). Player can decline."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put a valid LIBERATOR card in hand
        runner.inject_card(1, LIBERATOR_LV3, "hand")

        option_card = runner.inject_card(1, CARD_ID, "trash")

        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)

        # Check that there's a decline option available
        from digimon_gym.engine.data.enums import GamePhase
        if runner.game.current_phase in (
            GamePhase.SelectTarget, GamePhase.SelectReveal,
            GamePhase.SelectHand, GamePhase.SelectTrash,
        ):
            legal = runner.action_mask()
            # Action 62 is the standard decline action
            assert 62 in legal, (
                f"Should have a decline option (action 62) for optional play. "
                f"Legal actions: {runner.actions()}"
            )

    def test_security_adds_option_to_hand_from_trash(self, debug_runner):
        """After security play, this option card should be added to hand from trash."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put a LIBERATOR card in hand to play
        runner.inject_card(1, LIBERATOR_LV3, "hand")

        # Put EX7-074 in trash (where it would be after security activation)
        option_card = runner.inject_card(1, CARD_ID, "trash")

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

    def test_security_adds_option_to_hand_from_security(self, debug_runner):
        """After security effect fires, card should move from security zone to hand."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # Put EX7-074 in security (this is the typical flow when revealed from security)
        option_card = runner.inject_card(1, CARD_ID, "security_top")

        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}

        # No valid targets to play, but the "add to hand" should still work
        sec_eff.on_process_callback(ctx)
        runner.auto_resolve()

        # EX7-074 should be in hand (moved from security)
        in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == CARD_ID
            for c in player.hand_cards
        )
        assert in_hand, (
            f"EX7-074 should be added to hand from security zone. "
            f"Hand IDs: {[c.c_entity_base.card_id for c in player.hand_cards if c.c_entity_base]}"
        )

    def test_security_add_to_hand_happens_even_if_no_play(self, debug_runner):
        """The 'Then, add this card to the hand' should fire even if no card was played."""
        runner = debug_runner(initial_memory=0)
        player = runner.game.player1
        game = runner.game

        runner.clear_zone(1, "hand")

        # No valid LIBERATOR cards in hand or trash to play
        option_card = runner.inject_card(1, CARD_ID, "trash")

        effects = option_card.effect_list(None)
        security_effects = [
            e for e in effects
            if getattr(e, 'is_security_effect', False)
        ]
        sec_eff = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': option_card}
        sec_eff.on_process_callback(ctx)
        runner.auto_resolve()

        # Even with no valid play, card should still be added to hand
        in_hand = any(
            c.c_entity_base and c.c_entity_base.card_id == CARD_ID
            for c in player.hand_cards
        )
        assert in_hand, (
            f"EX7-074 should be added to hand even when no play occurred. "
            f"Hand IDs: {[c.c_entity_base.card_id for c in player.hand_cards if c.c_entity_base]}"
        )
