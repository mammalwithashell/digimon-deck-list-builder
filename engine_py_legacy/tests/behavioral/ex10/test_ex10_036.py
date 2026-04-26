"""Behavioral tests for EX10-036 Magneticdramon (Lv.7 Black Rock Dragon, DP 14000, Cost 14).

Card text:
<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its
digivolution cards, it isn't deleted.)
[When Digivolving] [When Attacking] By trashing 3 [Mineral] or [Rock] trait
cards from any of your Digimon's digivolution cards, delete 1 of your opponent's
Digimon and trash their top security card.
[When Digivolving] [When Attacking] [Once Per Turn] By placing 3 [Mineral] or
[Rock] trait cards from your trash as this Digimon's bottom digivolution cards,
it unsuspends.

Alt-digi: While you have [Close], [Proganomon]: Cost 6
(Base must be Proganomon, condition requires Close tamer on field.)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestEX10036Magneticdramon:
    """Tests for EX10-036 Magneticdramon."""

    # ── Alt-Digi Tests ──────────────────────────────────────────────

    def test_alt_digi_name_is_proganomon(self, debug_runner):
        """Alt-digi _alt_digi_name should be 'Proganomon' (base requirement)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have alt-digi effect"
        assert alt_digi[0]._alt_digi_name == "Proganomon", \
            f"Alt-digi base should be Proganomon, got {alt_digi[0]._alt_digi_name!r}"

    def test_alt_digi_cost_is_6(self, debug_runner):
        """Alt-digi cost should be 6."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert alt_digi[0]._alt_digi_cost == 6

    def test_alt_digi_requires_close_on_field(self, debug_runner):
        """Alt-digi condition_fn should require Close tamer on owner's field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1

        eff = alt_digi[0]
        condition_fn = getattr(eff, '_alt_digi_condition_fn', None)
        assert condition_fn is not None, \
            "Alt-digi should have _alt_digi_condition_fn to check for Close tamer"

        # Place a Proganomon as base (what we'd digivolve from)
        base_perm = runner.place_on_field(1, ["EX10-032"])  # Proganomon

        # Without Close on field, condition should fail
        assert not condition_fn(base_perm), \
            "Alt-digi condition should fail without Close tamer on field"

        # Place Close tamer on field
        close_perm = runner.place_on_field(1, ["EX10-063"])  # Close tamer

        # With Close on field, condition should pass
        assert condition_fn(base_perm), \
            "Alt-digi condition should pass with Close tamer on field"

    # ── Fragment (3) Tests ──────────────────────────────────────────

    def test_has_fragment_3(self, debug_runner):
        """Should have Fragment (3) keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        fragment = [e for e in effects if getattr(e, '_is_fragment', False)]
        assert len(fragment) >= 1, "Should have Fragment keyword"
        assert fragment[0]._fragment_count == 3, "Fragment count should be 3"

    # ── Delete Effect (Trash 3 Sources) Tests ───────────────────────

    def test_delete_effect_uses_player_chosen_source_selection(self, debug_runner):
        """Delete effect should use SelectSource phase for player to choose
        which digivolution cards to trash (not auto-trash from top)."""
        runner = debug_runner(initial_memory=5)

        # Build Magneticdramon with Mineral/Rock sources underneath
        # EX10-028 = Landramon (Mineral), EX10-003 = Tumblemon (Rock),
        # EX8-046 = Gotsumon (Rock)
        perm = runner.place_on_field(1, [
            "EX10-003", "EX10-028", "EX8-046", "EX10-032", "EX10-036"
        ])
        # Opponent Digimon to delete
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        # Find the When Digivolving version of the delete effect
        when_digi_delete = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_when_digivolving', False)
                           and 'delete' in (e.effect_name or '').lower()]
        assert len(when_digi_delete) >= 1, "Should have When Digivolving delete effect"

        effect = when_digi_delete[0]

        # Process the effect
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should enter SelectSource phase for player to choose sources to trash
        assert game.current_phase == GamePhase.SelectSource, \
            f"Should be in SelectSource phase, got {game.current_phase}"

    def test_delete_effect_trashes_3_sources_then_deletes_and_trashes_security(self, debug_runner):
        """After trashing 3 chosen Mineral/Rock sources, should delete opponent
        Digimon and trash their top security card."""
        runner = debug_runner(initial_memory=5)

        # Stack with 4 Mineral/Rock sources under Magneticdramon
        perm = runner.place_on_field(1, [
            "EX10-003", "EX10-028", "EX8-046", "BT4-066", "EX10-036"
        ])
        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        p1 = game.player1
        p2 = game.player2

        initial_security = len(p2.security_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_delete = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_when_digivolving', False)
                           and 'delete' in (e.effect_name or '').lower()]
        effect = when_digi_delete[0]

        # Process the effect
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Should be in SelectSource phase
        assert game.current_phase == GamePhase.SelectSource

        # Select 3 sources to trash using auto_resolve
        # auto_resolve picks first legal action each time
        results = runner.auto_resolve(max_steps=20)

        # After selecting 3 sources + selecting opponent target:
        # - 3 sources should have been trashed (moved to p1 trash)
        # - Opponent Digimon should be deleted
        # - Opponent's top security should be trashed

        # Check sources were trashed from perm
        # Originally had 5 cards (4 sources + top). After trashing 3, should have 2.
        assert len(perm.card_sources) == 2, \
            f"Should have 2 cards left (top + 1 source), got {len(perm.card_sources)}"

        # Check p1 trash has the 3 trashed sources
        mineral_rock_in_trash = [c for c in p1.trash_cards
                                 if any('Mineral' in t or 'Rock' in t
                                        for t in (getattr(c, 'card_traits', []) or []))]
        assert len(mineral_rock_in_trash) >= 3, \
            f"P1 trash should have at least 3 Mineral/Rock cards, got {len(mineral_rock_in_trash)}"

        # Check opponent security was trashed
        assert len(p2.security_cards) == initial_security - 1, \
            f"Opponent should have lost 1 security card"

    def test_delete_effect_condition_requires_3_mineral_rock_sources(self, debug_runner):
        """Delete effect condition should fail if fewer than 3 Mineral/Rock
        sources exist across all own Digimon."""
        runner = debug_runner(initial_memory=5)

        # Only 2 Mineral sources
        perm = runner.place_on_field(1, ["EX10-028", "EX10-032", "EX10-036"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_delete = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_when_digivolving', False)
                           and 'delete' in (e.effect_name or '').lower()]
        effect = when_digi_delete[0]

        # With only 2 Mineral/Rock sources (not counting top card), condition should fail
        result = effect.can_use_condition({})
        assert not result, "Condition should fail with fewer than 3 Mineral/Rock sources"

    def test_delete_effect_counts_sources_across_all_digimon(self, debug_runner):
        """Delete effect should count Mineral/Rock sources across ALL own Digimon,
        not just this Digimon."""
        runner = debug_runner(initial_memory=5)

        # Magneticdramon with 1 Mineral source
        perm = runner.place_on_field(1, ["EX10-028", "EX10-036"])
        # Another Digimon with 2 Mineral sources
        other = runner.place_on_field(1, ["BT4-066", "EX8-046", "ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_delete = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_when_digivolving', False)
                           and 'delete' in (e.effect_name or '').lower()]
        effect = when_digi_delete[0]

        # 1 from this Digimon + 2 from other = 3 total
        result = effect.can_use_condition({})
        assert result, "Condition should pass with 3 Mineral/Rock sources across multiple Digimon"

    def test_delete_effect_is_optional(self, debug_runner):
        """Delete effect should be optional (player can decline)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_delete = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and getattr(e, 'is_when_digivolving', False)
                           and 'delete' in (e.effect_name or '').lower()]
        assert when_digi_delete[0].is_optional, "Delete effect should be optional"

    def test_delete_effect_when_attacking_timing(self, debug_runner):
        """Should also trigger When Attacking."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_attack_delete = [e for e in effects
                             if e.timing == EffectTiming.OnUseAttack
                             and getattr(e, 'is_on_attack', False)
                             and 'delete' in (e.effect_name or '').lower()]
        assert len(when_attack_delete) >= 1, "Should have When Attacking delete effect"

    # ── Unsuspend Effect (Place 3 from Trash) Tests ─────────────────

    def test_unsuspend_effect_selects_from_trash(self, debug_runner):
        """Unsuspend effect should use SelectTrash phase for player to choose
        Mineral/Rock cards from trash."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-036"], is_suspended=True)

        # Inject 3 Mineral/Rock cards into trash
        runner.inject_card(1, "EX10-028", "trash")  # Mineral
        runner.inject_card(1, "EX10-003", "trash")  # Rock
        runner.inject_card(1, "BT4-066", "trash")   # Mineral

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        assert len(when_digi_unsuspend) >= 1, "Should have When Digivolving unsuspend effect"

        effect = when_digi_unsuspend[0]
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should enter SelectTrash phase
        assert game.current_phase == GamePhase.SelectTrash, \
            f"Should be in SelectTrash phase, got {game.current_phase}"

    def test_unsuspend_effect_places_3_and_unsuspends(self, debug_runner):
        """After placing exactly 3 Mineral/Rock cards from trash as bottom
        digivolution cards, the Digimon should unsuspend."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-036"], is_suspended=True)
        initial_sources = len(perm.card_sources)

        # Inject 3 Mineral/Rock cards into trash
        runner.inject_card(1, "EX10-028", "trash")  # Mineral
        runner.inject_card(1, "EX10-003", "trash")  # Rock
        runner.inject_card(1, "BT4-066", "trash")   # Mineral

        game = runner.game
        p1 = game.player1

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        effect = when_digi_unsuspend[0]

        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve all 3 trash selections
        results = runner.auto_resolve(max_steps=10)

        # Should now be unsuspended
        assert not perm.is_suspended, "Digimon should be unsuspended after placing 3 cards"

        # Should have 3 additional sources at the bottom
        assert len(perm.card_sources) == initial_sources + 3, \
            f"Should have {initial_sources + 3} cards in stack, got {len(perm.card_sources)}"

        # Cards should have been removed from trash
        mineral_rock_in_trash = [c for c in p1.trash_cards
                                 if any('Mineral' in t or 'Rock' in t
                                        for t in (getattr(c, 'card_traits', []) or []))]
        assert len(mineral_rock_in_trash) == 0, \
            "All 3 Mineral/Rock cards should have been removed from trash"

    def test_unsuspend_effect_is_once_per_turn(self, debug_runner):
        """Unsuspend effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        assert len(when_digi_unsuspend) >= 1
        effect = when_digi_unsuspend[0]
        assert effect.max_count_per_turn == 1, "Unsuspend effect should be once per turn"

    def test_unsuspend_effect_condition_requires_3_mineral_rock_in_trash(self, debug_runner):
        """Unsuspend condition should fail if fewer than 3 Mineral/Rock cards in trash."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        # Only 2 Mineral cards in trash
        runner.inject_card(1, "EX10-028", "trash")
        runner.inject_card(1, "BT4-066", "trash")

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        effect = when_digi_unsuspend[0]

        result = effect.can_use_condition({})
        assert not result, "Condition should fail with only 2 Mineral/Rock cards in trash"

    def test_unsuspend_effect_when_attacking_timing(self, debug_runner):
        """Should also trigger When Attacking."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_attack_unsuspend = [e for e in effects
                                if e.timing == EffectTiming.OnUseAttack
                                and getattr(e, 'is_on_attack', False)
                                and 'unsuspend' in (e.effect_name or '').lower()]
        assert len(when_attack_unsuspend) >= 1, "Should have When Attacking unsuspend effect"

    def test_unsuspend_effect_is_optional(self, debug_runner):
        """Unsuspend effect should be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-036"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        assert when_digi_unsuspend[0].is_optional, "Unsuspend effect should be optional"

    def test_unsuspend_trash_callback_handles_sel_trash_offset(self, debug_runner):
        """Trash selection callback should correctly subtract SEL_TRASH_START
        offset from the action_id to get the actual trash index."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["EX10-036"], is_suspended=True)

        # Inject 3 Mineral/Rock cards into trash
        runner.inject_card(1, "EX10-028", "trash")  # index 0
        runner.inject_card(1, "EX10-003", "trash")  # index 1
        runner.inject_card(1, "BT4-066", "trash")   # index 2

        game = runner.game
        p1 = game.player1

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi_unsuspend = [e for e in effects
                              if e.timing == EffectTiming.OnEnterFieldAnyone
                              and getattr(e, 'is_when_digivolving', False)
                              and 'unsuspend' in (e.effect_name or '').lower()]
        effect = when_digi_unsuspend[0]

        # Trigger the effect
        effect.on_process_callback({
            'player': p1,
            'game': game,
            'permanent': perm,
        })

        # Should be in SelectTrash phase
        assert game.current_phase == GamePhase.SelectTrash

        # The valid indices should be SEL_TRASH_START + trash_index
        ps = game.pending_selection
        assert ps is not None, "Should have pending selection"
        from digimon_gym.engine.game.constants import SEL_TRASH_START
        for idx in ps.valid_indices:
            assert idx >= SEL_TRASH_START, \
                f"Valid index {idx} should be >= SEL_TRASH_START ({SEL_TRASH_START})"

        # Execute selection through the action decoder (simulates real gameplay)
        first_action = ps.valid_indices[0]
        runner.execute(first_action)

        # Verify the card was actually removed from trash and placed as digi source
        # (If the offset bug existed, the callback would access wrong index)
        assert len(perm.card_sources) >= 2, \
            "Should have placed a card as bottom digivolution source"
