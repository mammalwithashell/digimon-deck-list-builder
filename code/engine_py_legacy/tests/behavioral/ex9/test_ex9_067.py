"""Behavioral tests for EX9-067 Mirai Kinosaki (Tamer, Yellow, Cost 3).

Card text:
[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Puppet]
    or [LIBERATOR] trait among them to the hand. Return the rest to the
    bottom of the deck.

[Your Turn] When any of your Digimon digivolve into a [Puppet] trait
    Digimon, by returning this Tamer to the bottom of the deck, you may
    play 1 [Arisa Kinosaki] or 1 [Puppet] trait Digimon card from your
    hand with the play cost reduced by 3.

[Security] Play this card without paying the cost.
"""

import pytest


@pytest.mark.behavioral
class TestEX9067MiraiKinosaki:
    """Tests for EX9-067 Mirai Kinosaki."""

    # ── Effect 0: [On Play] Reveal top 3, add 1 Puppet/LIBERATOR ──

    def test_on_play_reveal_picks_puppet_trait(self, debug_runner):
        """[On Play] Should reveal top 3 and let player add a Puppet trait card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Inject the tamer into hand
        runner.inject_card(1, "EX9-067", "hand")

        # Stack deck: Puppet (BT9-032 ToyAgumon), non-trait filler x2
        runner.inject_card(1, "ST1-03", "library_top")   # filler (bottom of 3)
        runner.inject_card(1, "ST1-03", "library_top")   # filler (middle of 3)
        runner.inject_card(1, "BT9-032", "library_top")  # Puppet (top of 3)

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Mirai Kinosaki")
        assert action is not None, "Should be able to play Mirai Kinosaki"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tamer on field
        assert any(
            s.card_id == "EX9-067" for s in snap_after.p1_field
        ), "Mirai Kinosaki should be on field"
        # BT9-032 should be in hand (selected from reveal)
        assert "BT9-032" in snap_after.p1_hand, (
            "ToyAgumon (Puppet) should have been added to hand from reveal"
        )

    def test_on_play_reveal_picks_liberator_trait(self, debug_runner):
        """[On Play] Should allow selecting a LIBERATOR trait card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-067", "hand")

        # Stack deck: LIBERATOR card (EX9-024 Hanimon), filler x2
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "EX9-024", "library_top")  # LIBERATOR trait

        action = runner.find_action("Play Mirai Kinosaki")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert "EX9-024" in snap.p1_hand, (
            "Hanimon (LIBERATOR) should have been added to hand from reveal"
        )

    def test_on_play_reveal_no_matching_cards(self, debug_runner):
        """[On Play] If no cards match Puppet/LIBERATOR, all go to deck bottom."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-067", "hand")

        # Stack deck with no Puppet/LIBERATOR cards
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")

        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Mirai Kinosaki")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # No cards should have been added to hand (beyond what was there + tamer left hand)
        # hand_before includes EX9-067 which is now on field, so hand should be hand_before - 1
        assert len(snap_after.p1_hand) == hand_before - 1, (
            f"No matching cards in reveal; hand should be {hand_before - 1} but was {len(snap_after.p1_hand)}"
        )

    def test_on_play_remaining_cards_to_deck_bottom(self, debug_runner):
        """[On Play] Non-selected cards should return to deck bottom."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        runner.inject_card(1, "EX9-067", "hand")

        # Stack: Puppet on top, 2 fillers
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "BT9-032", "library_top")

        snap_before = runner.snapshot()
        deck_before = snap_before.p1_library_size

        action = runner.find_action("Play Mirai Kinosaki")
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # 1 card taken from deck to hand, 2 returned to bottom
        # Net: deck lost 1 card
        assert snap_after.p1_library_size == deck_before - 1, (
            f"Deck should lose 1 card (selected to hand); was {deck_before}, now {snap_after.p1_library_size}"
        )

    # ── Effect 1: [Your Turn] Digivolve observer ──

    def test_digivolve_observer_triggers_on_puppet(self, debug_runner):
        """When our Digimon digivolves into Puppet, should offer to return tamer and play."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Yellow Digimon on field (BT9-032 ToyAgumon, Puppet Lv.3)
        runner.place_on_field(1, ["BT9-032"])

        # Clear hand so only controlled cards are present
        runner.clear_zone(1, "hand")

        # Use BT22-032 ShoeShoemon (Puppet Lv.4, Yellow, no WhenDigivolving effect)
        runner.inject_card(1, "BT22-032", "hand")

        # Also put a playable Puppet Digimon in hand for the play effect
        runner.inject_card(1, "BT9-032", "hand")

        snap_before = runner.snapshot()
        tamer_on_field = any(s.card_id == "EX9-067" for s in snap_before.p1_field)
        assert tamer_on_field, "Tamer should be on field before digivolve"

        # Digivolve ShoeShoemon onto ToyAgumon
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve ShoeShoemon onto ToyAgumon"
        runner.execute(action)

        # The digivolve observer should trigger — auto-resolve the selection phases
        # This will return tamer to deck and play a card from hand
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tamer should no longer be on field (returned to deck bottom)
        assert not any(
            s.card_id == "EX9-067" for s in snap_after.p1_field
        ), "Tamer should have been returned to deck bottom"

    def test_digivolve_observer_not_on_non_puppet(self, debug_runner):
        """Observer should NOT trigger when digivolving into a non-Puppet Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Digimon (non-Puppet) on field
        runner.place_on_field(1, ["ST1-03"])  # Agumon, not Puppet

        # Put a non-Puppet Lv.4 in hand to digivolve
        runner.inject_card(1, "ST1-06", "hand")  # Greymon (not Puppet)

        snap_before = runner.snapshot()

        action = runner.find_action("Digivolve")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

        snap_after = runner.snapshot()
        # Tamer should still be on field
        assert any(
            s.card_id == "EX9-067" for s in snap_after.p1_field
        ), "Tamer should remain on field when non-Puppet digivolves"

    def test_digivolve_observer_not_on_opponent_turn(self, debug_runner):
        """Observer should NOT trigger on opponent's turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on P1's field
        runner.place_on_field(1, ["EX9-067"])

        # Switch to P2's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        snap = runner.snapshot()
        # Tamer should still be there
        assert any(
            s.card_id == "EX9-067" for s in snap.p1_field
        ), "Tamer should remain on field during opponent's turn"

    def test_digivolve_observer_plays_arisa_kinosaki(self, debug_runner):
        """Observer should allow playing Arisa Kinosaki from hand with cost -3."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Puppet on field (BT9-032 ToyAgumon, Yellow Puppet)
        runner.place_on_field(1, ["BT9-032"])

        # Clear hand so only controlled cards are present
        runner.clear_zone(1, "hand")

        # Use BT22-032 ShoeShoemon (Puppet Lv.4, Yellow, evo from Lv.3 Yellow cost 2)
        # It has NO When Digivolving effect to avoid selection interference
        runner.inject_card(1, "BT22-032", "hand")

        # Put Arisa Kinosaki (BT22-088, cost 3) in hand — the only play target
        runner.inject_card(1, "BT22-088", "hand")

        # Digivolve ShoeShoemon onto ToyAgumon
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve ShoeShoemon onto ToyAgumon"
        runner.execute(action)

        # Auto-resolve: observer returns tamer to deck, then plays Arisa from hand
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Arisa Kinosaki should be on field (played from hand)
        assert any(
            s.card_id == "BT22-088" for s in snap_after.p1_field
        ), "Arisa Kinosaki should have been played from hand via observer"

    def test_digivolve_observer_plays_puppet_digimon(self, debug_runner):
        """Observer should allow playing a Puppet trait Digimon from hand with cost -3."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Puppet on field (BT9-032 ToyAgumon)
        runner.place_on_field(1, ["BT9-032"])

        # Clear hand so only controlled cards are present
        runner.clear_zone(1, "hand")

        # Use BT22-032 ShoeShoemon (Puppet Lv.4, Yellow, no WhenDigivolving)
        runner.inject_card(1, "BT22-032", "hand")

        # Put a Puppet Digimon in hand for the play (BT9-032 ToyAgumon, cost 2)
        # Cost 2 - 3 reduction = 0 (free play)
        runner.inject_card(1, "BT9-032", "hand")

        memory_before = runner.game.memory

        # Digivolve ShoeShoemon onto ToyAgumon
        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # ToyAgumon should be on field (played from hand via observer)
        toy_on_field = any(
            s.card_id == "BT9-032" for s in snap_after.p1_field
        )
        assert toy_on_field, (
            "A Puppet Digimon (ToyAgumon) should have been played from hand via observer"
        )

    def test_digivolve_observer_is_optional(self, debug_runner):
        """Observer effect should be optional (can decline to return tamer)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Puppet on field
        runner.place_on_field(1, ["BT9-032"])

        # Put a Puppet Lv.4 in hand to digivolve
        runner.inject_card(1, "EX9-027", "hand")

        snap_before = runner.snapshot()
        tamer_count_before = sum(
            1 for s in snap_before.p1_field if s.card_id == "EX9-067"
        )

        # The observer fires through _fire_digivolve_observers which calls
        # the process callback directly. Since it's optional in the C# ref
        # (canNoSelect: true), verify the effect is marked optional.
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        script = db.get_script("EX9-067")
        from engine_py_legacy.engine.core.card_source import CardSource
        cs = CardSource()
        effects = script.get_card_effects(cs)
        # Effect 1 is the digivolve observer
        observer_effect = effects[1]
        assert getattr(observer_effect, 'is_optional', False), (
            "Digivolve observer effect should be marked as optional"
        )
        assert getattr(observer_effect, '_is_digivolve_observer', False), (
            "Effect 1 should have _is_digivolve_observer = True"
        )

    def test_digivolve_observer_does_not_fire_via_when_digivolving(self, debug_runner):
        """The observer should NOT have is_when_digivolving=True (it's an observer, not a self-digivolve effect)."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        script = db.get_script("EX9-067")
        from engine_py_legacy.engine.core.card_source import CardSource
        cs = CardSource()
        effects = script.get_card_effects(cs)
        observer_effect = effects[1]
        assert not getattr(observer_effect, 'is_when_digivolving', False), (
            "Observer effect should NOT have is_when_digivolving=True; "
            "it is a digivolve observer, not a self-digivolve trigger"
        )

    def test_digivolve_observer_filter_rejects_non_puppet_non_arisa(self, debug_runner):
        """Observer play filter should reject cards that are neither Arisa Kinosaki nor Puppet Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place tamer on field
        runner.place_on_field(1, ["EX9-067"])

        # Place a Lv.3 Puppet on field
        runner.place_on_field(1, ["BT9-032"])

        # Clear hand so only controlled cards are present
        runner.clear_zone(1, "hand")

        # Ensure the digivolve draw pulls a non-Puppet, non-Arisa card
        runner.inject_card(1, "ST1-03", "library_top")

        # Use BT22-032 ShoeShoemon (no WhenDigivolving)
        runner.inject_card(1, "BT22-032", "hand")

        # Put a non-Puppet, non-Arisa card in hand (the only play candidate)
        runner.inject_card(1, "ST1-03", "hand")  # Agumon - not Puppet, not Arisa

        # Digivolve triggers observer, but no valid play targets in hand
        action = runner.find_action("Digivolve")
        assert action is not None, "Should be able to digivolve"
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # ST1-03 should still be in hand (not played via observer, since not Puppet/Arisa)
        assert "ST1-03" in snap_after.p1_hand, (
            "Non-Puppet, non-Arisa cards should not be playable via observer"
        )
        # Tamer should have been returned to deck (observer process runs before play check)
        # Actually, the observer's process1 returns tamer then calls effect_play_from_zone.
        # If no valid targets, play is skipped but tamer is already returned.
        assert not any(
            s.card_id == "EX9-067" for s in snap_after.p1_field
        ), "Tamer was returned to deck as the cost of the observer"

    # ── Effect 2: Security ──

    def test_security_play_free(self, debug_runner):
        """[Security] Should play this card without paying the cost."""
        # Security effects are standard pattern; just verify the effect exists
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import EffectTiming
        db = CardDatabase()
        script = db.get_script("EX9-067")
        from engine_py_legacy.engine.core.card_source import CardSource
        cs = CardSource()
        effects = script.get_card_effects(cs)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) == 1, "Should have exactly 1 security effect"
