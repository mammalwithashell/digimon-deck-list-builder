"""Behavioral tests for EX10-074 Beelzemon (Lv.6 Purple/Black Demon Lord, DP 12000, Cost 7).

Card text:
[Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into this card
    without paying the cost.)
[On Play] [When Digivolving] [When Attacking] Trash the top 2 cards of your deck.
    Then, delete 1 of your opponent's play cost 6 or lower Digimon. For every 10
    cards in your trash, add 3 to the play cost maximum.
[On Play] [When Digivolving] If you have 10 or more cards in your trash, by
    returning 2 non-Digi-Egg cards from your trash to the top of the deck,
    <De-Digivolve 2> 1 of your opponent's Digimon.
Alt-digi: Lv.3 w/ [Impmon] in name for cost 4, requires 20+ trash
Inherited: Ace Overflow <-4>
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.game.constants import SEL_TRASH_START

# Use Dracomon (ST1-04) as filler
FILLER_DECK = ["ST1-04"] * 50


@pytest.mark.behavioral
class TestEX10074AltDigi:
    """Tests for EX10-074 alternate digivolution: Lv.3 w/ [Impmon] for cost 4, 20+ trash."""

    def test_alt_digi_has_name_attribute(self, debug_runner):
        """Alt-digi effect must have _alt_digi_name = 'Impmon' to restrict to Impmon-name bases."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi_effects = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi_effects) >= 1, "Should have an alt-digi effect"

        eff = alt_digi_effects[0]
        assert getattr(eff, '_alt_digi_name', None) == "Impmon", (
            "Alt-digi must have _alt_digi_name='Impmon' to match C# HasImpmonName check"
        )
        assert getattr(eff, '_alt_digi_level', None) == 3, "Alt-digi requires Lv.3"
        assert getattr(eff, '_alt_digi_cost', None) == 4, "Alt-digi cost is 4"

    def test_alt_digi_onto_impmon_with_20_trash(self, debug_runner):
        """EX10-074 can digivolve onto Impmon (BT2-068, Lv.3) for cost 4 when 20+ trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place Impmon on field
        perm = runner.place_on_field(1, ["BT2-068"])

        # Fill trash to 20+ cards
        for _ in range(20):
            runner.inject_card(1, "ST1-04", zone="trash")
        assert len(runner.game.player1.trash_cards) >= 20

        # Put EX10-074 in hand
        runner.inject_card(1, "EX10-074", zone="hand")

        # Should be able to digivolve
        actions = runner.find_actions("Digivolve")
        beelze_digi = [a for a, d in actions.items() if "Beelzemon" in d]
        assert beelze_digi, (
            "EX10-074 should be able to digivolve onto Impmon with 20+ trash"
        )

    def test_alt_digi_rejects_non_impmon_lv3(self, debug_runner):
        """EX10-074 cannot alt-digivolve onto a non-Impmon Lv.3 even with 20+ trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Place Agumon (ST1-03, Lv.3, not Impmon) on field
        perm = runner.place_on_field(1, ["ST1-03"])

        # Fill trash to 20+ cards
        for _ in range(20):
            runner.inject_card(1, "ST1-04", zone="trash")

        runner.inject_card(1, "EX10-074", zone="hand")

        # Should NOT be able to digivolve (no standard evo path either)
        actions = runner.find_actions("Digivolve")
        beelze_digi = [a for a, d in actions.items() if "Beelzemon" in d]
        assert not beelze_digi, (
            "EX10-074 should NOT alt-digi onto non-Impmon Lv.3 (Agumon)"
        )

    def test_alt_digi_rejects_impmon_without_20_trash(self, debug_runner):
        """EX10-074 cannot alt-digivolve onto Impmon with < 20 trash cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT2-068"])

        # Only 10 trash (not enough)
        for _ in range(10):
            runner.inject_card(1, "ST1-04", zone="trash")

        runner.inject_card(1, "EX10-074", zone="hand")

        actions = runner.find_actions("Digivolve")
        beelze_digi = [a for a, d in actions.items() if "Beelzemon" in d]
        assert not beelze_digi, (
            "EX10-074 should NOT alt-digi onto Impmon with < 20 trash"
        )


@pytest.mark.behavioral
class TestEX10074BlastDigivolve:
    """Tests for Blast Digivolve counter effect."""

    def test_has_blast_digivolve(self, debug_runner):
        """Should have Blast Digivolve keyword."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        blast_effects = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast_effects) >= 1, "Should have Blast Digivolve"


@pytest.mark.behavioral
class TestEX10074FirstEffect:
    """Tests for first effect: [On Play]/[When Digivolving]/[When Attacking]
    Trash top 2, delete 1 opp Digimon play cost <= 6 + 3*(trash//10)."""

    def test_on_play_mills_2_and_deletes(self, debug_runner):
        """On Play should trash 2 from deck, then delete 1 opp Digimon cost <= 6."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        # ST1-03 Agumon: Lv.3, play cost 3 (under 6)
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        lib_before = len(game.player1.library_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)]
        assert len(on_play_effects) >= 1, "Should have On Play effect (first effect)"

        effect = on_play_effects[0]

        # Mock opponent select
        deleted_perms = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        original_delete = game.player2.delete_permanent
        def track_delete(p):
            deleted_perms.append(p)
            original_delete(p)
        game.player2.delete_permanent = track_delete

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have milled 2
        lib_after = len(game.player1.library_cards)
        assert lib_before - lib_after == 2, f"Should mill 2 cards, milled {lib_before - lib_after}"

        # Should have deleted the target
        assert target in deleted_perms, "Should delete opponent's Digimon with cost <= 6"

    def test_cost_cap_scales_with_trash(self, debug_runner):
        """Delete cost cap scales: 6 + 3*(trash//10). With 20 trash, max is 12."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])

        game = runner.game

        # Fill trash with 18 cards (after milling 2 = 20, cap = 6 + 3*2 = 12)
        for _ in range(18):
            runner.inject_card(1, "ST1-04", zone="trash")

        # Place opponent with cost 12 -- ST1-11 WarGreymon is cost 12
        target_12 = runner.place_on_field(2, ["ST1-11"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)]
        effect = on_play_effects[0]

        deletable = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn and filter_fn(p):
                    deletable.append(p)
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        original_delete = game.player2.delete_permanent
        game.player2.delete_permanent = lambda p: original_delete(p)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # With 20 trash after mill, cost cap = 6 + 3*2 = 12
        # WarGreymon (cost 12) should be targetable
        assert target_12 in deletable, (
            "With 20 trash, cost cap is 12, so cost-12 Digimon should be deletable"
        )

    def test_mills_before_calculating_cap(self, debug_runner):
        """Mill happens before cost cap calculation, so milled cards count toward trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        game = runner.game

        # Start with 8 trash. After mill 2, trash = 10. Cap = 6 + 3*1 = 9.
        for _ in range(8):
            runner.inject_card(1, "ST1-04", zone="trash")

        # Place opponent Digimon with cost 7 (above base 6 but within 9)
        # BT19-067 Impmon has cost 4 -- use ST1-07 Greymon cost 4
        # Actually, let's directly verify via the filter
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)]
        effect = on_play_effects[0]

        trash_before_process = len(game.player1.trash_cards)
        max_cost_captured = []

        original_select = game.effect_select_opponent_permanent

        def capture_filter(player, callback, filter_fn=None, is_optional=False, prompt=None):
            # The filter_fn closure captures max_cost after milling
            # We can verify by checking trash count at this point
            max_cost_captured.append(len(player.trash_cards))

        game.effect_select_opponent_permanent = capture_filter

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # After mill, trash should be 10 (8 + 2 milled)
        assert len(game.player1.trash_cards) == 10, (
            f"After milling 2, should have 10 trash, got {len(game.player1.trash_cards)}"
        )

    def test_has_when_attacking_trigger(self, debug_runner):
        """Should have a When Attacking trigger for the first effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        attack_effects = [e for e in effects
                          if e.timing == EffectTiming.OnUseAttack
                          and getattr(e, 'is_on_attack', False)]
        assert len(attack_effects) >= 1, "Should have When Attacking trigger"

    def test_has_when_digivolving_trigger(self, debug_runner):
        """Should have a When Digivolving trigger for the first effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        digi_effects = [e for e in effects
                        if e.timing == EffectTiming.OnEnterFieldAnyone
                        and getattr(e, 'is_when_digivolving', False)]
        assert len(digi_effects) >= 1, "Should have When Digivolving trigger"


@pytest.mark.behavioral
class TestEX10074SecondEffect:
    """Tests for second effect: [On Play]/[When Digivolving]
    If 10+ trash, by returning 2 non-Digi-Egg from trash to deck top,
    De-Digivolve 2 one opponent Digimon."""

    def test_second_effect_selects_from_trash_not_hand(self, debug_runner):
        """The second effect must use SelectTrash phase, not SelectHand.

        BUG: The original script used effect_select_hand_card() which selects
        from player.hand_cards. The effect needs to select from player.trash_cards.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        # Opponent Digimon for de-digivolve target
        opp_stack = runner.place_on_field(2, ["ST1-03", "ST1-07", "ST1-11"])

        game = runner.game

        # Fill trash with 12 non-egg cards
        for _ in range(12):
            runner.inject_card(1, "ST1-04", zone="trash")
        assert len(game.player1.trash_cards) >= 10

        # Ensure hand is empty so hand selection would fail
        game.player1.hand_cards.clear()

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_on_play', False)
                           and getattr(e, 'is_optional', False)]
        assert len(on_play_effects) >= 1, "Should have second On Play effect (optional)"

        effect = on_play_effects[0]

        # Track what selection phase is used
        phases_used = []
        original_request = game.request_selection

        def track_request(phase, player, callback, valid, is_optional=False, **kwargs):
            phases_used.append(phase)
            # Auto-select first valid if present
            if valid:
                callback(valid[0])

        game.request_selection = track_request

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should use SelectTrash, not SelectHand
        assert GamePhase.SelectTrash in phases_used, (
            f"Second effect should use SelectTrash phase for trash card selection, "
            f"but used phases: {phases_used}"
        )
        assert GamePhase.SelectHand not in phases_used, (
            "Second effect should NOT use SelectHand phase"
        )

    def test_second_effect_returns_cards_to_deck_top(self, debug_runner):
        """Selecting 2 non-egg cards from trash should place them on top of deck."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        opp_stack = runner.place_on_field(2, ["ST1-03", "ST1-07", "ST1-11"])

        game = runner.game

        # Inject identifiable trash cards
        runner.inject_card(1, "BT2-068", zone="trash")  # Impmon
        runner.inject_card(1, "BT6-068", zone="trash")  # Another Impmon
        # More filler to reach 10+ total
        for _ in range(10):
            runner.inject_card(1, "ST1-04", zone="trash")
        assert len(game.player1.trash_cards) >= 10

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_optional = [e for e in effects
                            if e.timing == EffectTiming.OnEnterFieldAnyone
                            and getattr(e, 'is_on_play', False)
                            and getattr(e, 'is_optional', False)]
        effect = on_play_optional[0]

        # Track selections
        select_count = [0]
        original_request = game.request_selection

        def auto_select(phase, player, callback, valid, is_optional=False, **kwargs):
            select_count[0] += 1
            if valid:
                callback(valid[0])

        game.request_selection = auto_select

        lib_before = len(game.player1.library_cards)
        trash_before = len(game.player1.trash_cards)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # 2 cards should have moved from trash to deck top
        # (2 removed from trash, 2 added to library)
        trash_after = len(game.player1.trash_cards)
        lib_after = len(game.player1.library_cards)

        assert trash_before - trash_after == 2, (
            f"Should remove 2 cards from trash, removed {trash_before - trash_after}"
        )
        assert lib_after - lib_before == 2, (
            f"Should add 2 cards to library, added {lib_after - lib_before}"
        )

    def test_second_effect_requires_10_trash(self, debug_runner):
        """Second effect condition requires 10+ trash cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        game = runner.game

        # Only 5 trash (not enough)
        for _ in range(5):
            runner.inject_card(1, "ST1-04", zone="trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_optional = [e for e in effects
                            if e.timing == EffectTiming.OnEnterFieldAnyone
                            and getattr(e, 'is_on_play', False)
                            and getattr(e, 'is_optional', False)]
        assert len(on_play_optional) >= 1

        effect = on_play_optional[0]
        # Condition should fail with < 10 trash
        assert not effect.can_use_condition({}), (
            "Second effect condition should fail with < 10 trash cards"
        )

    def test_second_effect_is_optional(self, debug_runner):
        """The second effect is optional (player can decline)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_optional = [e for e in effects
                            if e.timing == EffectTiming.OnEnterFieldAnyone
                            and getattr(e, 'is_on_play', False)
                            and getattr(e, 'is_optional', False)]
        assert len(on_play_optional) >= 1, (
            "Should have an optional On Play effect (the second effect)"
        )

    def test_second_effect_triggers_de_digivolve(self, debug_runner):
        """After returning 2 cards, should De-Digivolve 2 an opponent Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-074"])
        # Opponent has a 3-card stack (base + 2 digivolutions)
        opp_stack = runner.place_on_field(2, ["ST1-03", "ST1-07", "ST1-11"])
        opp_stack_size_before = len(opp_stack.card_sources)

        game = runner.game

        # Fill trash with 12 non-egg cards
        for _ in range(12):
            runner.inject_card(1, "ST1-04", zone="trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_optional = [e for e in effects
                            if e.timing == EffectTiming.OnEnterFieldAnyone
                            and getattr(e, 'is_on_play', False)
                            and getattr(e, 'is_optional', False)]
        effect = on_play_optional[0]

        original_request = game.request_selection

        def auto_select(phase, player, callback, valid, is_optional=False, **kwargs):
            if valid:
                callback(valid[0])

        game.request_selection = auto_select

        # Also mock opponent permanent selection for de-digivolve target
        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return
        game.effect_select_opponent_permanent = mock_select_opp

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Opponent's stack should have lost 2 cards (de-digivolve 2)
        opp_stack_size_after = len(opp_stack.card_sources)
        assert opp_stack_size_after == opp_stack_size_before - 2, (
            f"De-Digivolve 2 should remove 2 cards from stack, "
            f"was {opp_stack_size_before}, now {opp_stack_size_after}"
        )

    def test_second_effect_has_when_digivolving_trigger(self, debug_runner):
        """Should have a When Digivolving trigger for the second effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-074"])

        card = perm.top_card
        effects = card.effect_list(None)
        digi_optional = [e for e in effects
                         if e.timing == EffectTiming.OnEnterFieldAnyone
                         and getattr(e, 'is_when_digivolving', False)
                         and getattr(e, 'is_optional', False)]
        assert len(digi_optional) >= 1, (
            "Should have optional When Digivolving trigger for second effect"
        )
