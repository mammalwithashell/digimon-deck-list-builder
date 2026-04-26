"""Behavioral tests for EX10-063 Close (Tamer, Black, Play Cost 3).

Card text:
[Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
you may play 1 [Close] from your hand without paying the cost. Then, if you
don't have a Digimon, you may play 1 [Sunarizamon] from your trash without
paying the cost.

[All Turns] When effects trash any of your [Mineral] or [Rock] trait Digimon's
digivolution cards, by suspending this Tamer, gain 1 memory.

Security Effect: Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


FILLER = ["ST1-03"] * 40  # Agumon filler for decks


@pytest.mark.behavioral
class TestEX10063Close:
    """Tests for EX10-063 Close."""

    # ── Clause 1: Start of Main Phase — condition ───────────────────

    def test_condition_offered_when_on_field_own_turn(self, debug_runner):
        """Effect0 condition should pass when tamer is on field during owner's turn,
        even with no Close in hand and no Sunarizamon in trash (C# behavioral match)."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        # Place EX10-063 on field
        perm = runner.place_on_field(1, ["EX10-063"])
        runner.set_phase("Main")
        runner.set_memory(3)

        card = perm.top_card
        effects = card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(start_effects) >= 1, "Should have OnStartMainPhase effect"

        effect = start_effects[0]
        # Build context mimicking what the engine provides
        result = effect.can_use_condition({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        })
        assert result is True, (
            "Condition should be True when tamer is on field during own turn, "
            "regardless of Close in hand or Sunarizamon in trash"
        )

    def test_condition_fails_when_not_on_field(self, debug_runner):
        """Effect0 condition should fail when tamer is NOT on the field."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        # Inject Close to hand but don't place on field
        runner.inject_card(1, "EX10-063", "hand")
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        # Find the card source in hand
        close_card = None
        for c in p1.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-063":
                close_card = c
                break
        assert close_card is not None, "Should have Close in hand"

        effects = close_card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        if start_effects:
            effect = start_effects[0]
            result = effect.can_use_condition({
                'game': game,
                'player': p1,
            })
            assert result is False, "Condition should fail when not on field"

    def test_condition_fails_on_opponent_turn(self, debug_runner):
        """Effect0 condition should fail during opponent's turn."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])
        runner.set_phase("Main")

        # Switch turn to player 2
        game = runner.game
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        card = perm.top_card
        effects = card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = start_effects[0]
        result = effect.can_use_condition({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })
        assert result is False, "Condition should fail on opponent's turn"

    # ── Clause 1: Start of Main Phase — process ────────────────────

    def test_return_tamer_to_deck_bottom_and_play_close(self, debug_runner):
        """Process should return tamer to deck bottom and offer to play Close from hand."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])
        runner.inject_card(1, "EX10-063", "hand")  # Another Close in hand
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        initial_deck_size = len(p1.library_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = start_effects[0]

        # Track play_from_zone calls
        play_calls = []
        original_play = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            play_calls.append({'zone': zone, 'kwargs': kwargs})
            # Auto-select the Close from hand
            original_play(player, zone, filter_fn, **kwargs)

        game.effect_play_from_zone = mock_play

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Tamer should be returned to deck bottom
        assert perm not in p1.battle_area, "Tamer should be removed from battle area"
        # Deck size should increase (tamer returned to bottom)
        assert len(p1.library_cards) >= initial_deck_size, "Deck should have tamer at bottom"
        # Should have called play_from_zone for hand (Close)
        assert any(c['zone'] == 'hand' for c in play_calls), "Should offer to play Close from hand"

    # ── Clause 2: Then — Sunarizamon from trash ────────────────────

    def test_sunarizamon_from_trash_when_no_digimon(self, debug_runner):
        """When no Digimon on field after Close play, should offer Sunarizamon from trash."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])
        runner.inject_card(1, "EX10-025", "trash")  # Sunarizamon in trash
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1

        # Ensure no Digimon on field (only the tamer, which will be returned)
        assert not any(p.is_digimon for p in p1.battle_area if p is not perm), \
            "Setup: no Digimon on field besides the tamer"

        card = perm.top_card
        effects = card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = start_effects[0]

        play_calls = []
        original_play = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            play_calls.append({'zone': zone, 'kwargs': kwargs})

        game.effect_play_from_zone = mock_play

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Should call play_from_zone for both hand (Close) and trash (Sunarizamon)
        zones_called = [c['zone'] for c in play_calls]
        assert 'hand' in zones_called, "Should offer to play Close from hand"
        assert 'trash' in zones_called, "Should offer to play Sunarizamon from trash"

    def test_no_sunarizamon_when_digimon_on_field(self, debug_runner):
        """When Digimon is on field, should NOT offer Sunarizamon from trash."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])
        runner.place_on_field(1, ["ST1-03"])  # Agumon Digimon on field
        runner.inject_card(1, "EX10-025", "trash")  # Sunarizamon in trash
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1

        card = perm.top_card
        effects = card.effect_list(None)
        start_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = start_effects[0]

        play_calls = []
        original_play = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            play_calls.append({'zone': zone, 'kwargs': kwargs})

        game.effect_play_from_zone = mock_play

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        zones_called = [c['zone'] for c in play_calls]
        assert 'hand' in zones_called, "Should still offer Close from hand"
        assert 'trash' not in zones_called, \
            "Should NOT offer Sunarizamon from trash when Digimon is on field"

    # ── Clause 3: All Turns — digi-card trash trigger ───────────────

    def test_memory_gain_on_mineral_digi_card_trash(self, debug_runner):
        """Should gain 1 memory when a Mineral Digimon's digi-card is trashed."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        # BT4-066 Golemon (Mineral) on top, BT2-054 under it (bottom-to-top order)
        mineral_perm = runner.place_on_field(1, ["BT2-054", "BT4-066"])
        runner.set_phase("Main")
        runner.set_memory(3)

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        assert len(digi_trash_effects) >= 1, "Should have OnDigivolutionCardDiscarded effect"

        effect = digi_trash_effects[0]

        # Build context as engine would provide
        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': mineral_perm,  # The permanent whose digi-card was trashed
        }
        assert effect.can_use_condition(context) is True, \
            "Condition should pass for Mineral Digimon"

        before_memory = game.memory
        effect.on_process_callback({
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
        })
        assert game.memory == before_memory + 1, "Should gain 1 memory"

    def test_memory_gain_on_rock_digi_card_trash(self, debug_runner):
        """Should gain 1 memory when a Rock Digimon's digi-card is trashed."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        # BT2-054 Gotsumon (Rock) on top, ST1-03 under it (bottom-to-top order)
        rock_perm = runner.place_on_field(1, ["ST1-03", "BT2-054"])
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': rock_perm,
        }
        assert effect.can_use_condition(context) is True, \
            "Condition should pass for Rock Digimon"

    def test_no_memory_gain_for_non_mineral_rock(self, debug_runner):
        """Should NOT trigger for Digimon without Mineral or Rock trait."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        # ST1-03 Agumon (Reptile trait, NOT Mineral/Rock)
        reptile_perm = runner.place_on_field(1, ["ST1-03", "ST1-03"])
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': reptile_perm,
        }
        assert effect.can_use_condition(context) is False, \
            "Condition should fail for non-Mineral/Rock Digimon"

    def test_no_trigger_when_tamer_suspended(self, debug_runner):
        """Should NOT trigger when tamer is already suspended."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"], is_suspended=True)
        mineral_perm = runner.place_on_field(1, ["BT2-054", "BT4-066"])
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': mineral_perm,
        }
        assert effect.can_use_condition(context) is False, \
            "Condition should fail when tamer is already suspended"

    def test_tamer_suspended_after_trigger(self, debug_runner):
        """Tamer should be suspended after gaining memory."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        assert not tamer_perm.is_suspended, "Tamer should start unsuspended"
        effect.on_process_callback({
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
        })
        assert tamer_perm.is_suspended, "Tamer should be suspended after trigger"

    def test_trigger_on_opponent_turn(self, debug_runner):
        """[All Turns] Should trigger on opponent's turn too."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        mineral_perm = runner.place_on_field(1, ["BT2-054", "BT4-066"])
        runner.set_phase("Main")

        game = runner.game
        # Switch to opponent's turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': mineral_perm,
        }
        assert effect.can_use_condition(context) is True, \
            "[All Turns] condition should pass on opponent's turn"

    def test_no_trigger_for_opponent_digimon(self, debug_runner):
        """Should NOT trigger when opponent's Mineral/Rock Digimon digi-cards are trashed."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX10-063"])
        # Opponent's Mineral Digimon (bottom-to-top: Gotsumon under, Golemon on top)
        opp_mineral = runner.place_on_field(2, ["BT2-054", "BT4-066"])
        runner.set_phase("Main")

        game = runner.game
        p1 = game.player1
        card = tamer_perm.top_card

        effects = card.effect_list(None)
        digi_trash_effects = [e for e in effects
                              if e.timing == EffectTiming.OnDigivolutionCardDiscarded]
        effect = digi_trash_effects[0]

        context = {
            'game': game,
            'player': p1,
            'permanent': tamer_perm,
            'event_permanent': opp_mineral,
        }
        assert effect.can_use_condition(context) is False, \
            "Should NOT trigger for opponent's Digimon"

    # ── Clause 4: Security ──────────────────────────────────────────

    def test_security_effect_exists(self, debug_runner):
        """Should have a security effect to play this card."""
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])

        card = perm.top_card
        effects = card.effect_list(None)
        security_effects = [e for e in effects
                            if e.timing == EffectTiming.SecuritySkill
                            and getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have security effect"

    # ── Effect count ────────────────────────────────────────────────

    def test_has_three_effects(self, debug_runner):
        """EX10-063 should have exactly 3 effects:
        1. OnStartMainPhase (return + play Close + maybe Sunarizamon)
        2. OnDigivolutionCardDiscarded (suspend for memory)
        3. SecuritySkill (play from security)
        """
        runner = debug_runner(deck1=FILLER, deck2=FILLER, initial_memory=3)
        perm = runner.place_on_field(1, ["EX10-063"])

        card = perm.top_card
        effects = card.effect_list(None)
        assert len(effects) == 3, f"Should have 3 effects, got {len(effects)}"
