"""Behavioral tests for BT17-068 Mephistomon.

Card text:
  While this card is revealed from your deck, this card is also treated as level 6.
  When this card would be played from the hand, by returning 1 [Apocalymon]
    from your trash to the bottom of the deck, reduce the play cost by 3.
  [On Deletion] If deleted by an effect, you may play 1 [Gulfmon] or 1 level 6
    Digimon with the [Dark Masters] trait from your hand or trash without
    paying the cost.
  Inherited: [When Attacking] [Once Per Turn] By placing 1 level 5 or lower
    card with [Dark Masters] in its text from your trash as this Digimon's
    bottom digivolution card, this Digimon gets +2000 DP for the turn.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# Helpers
MEPHISTOMON = "BT17-068"
APOCALYMON = "BT15-102"       # Lv.7 Digimon
GULFMON = "BT17-070"          # Lv.6 Digimon
PIEDMON = "BT15-079"          # Lv.6 Dark Masters trait
SCORPIOMON = "BT15-027"       # Lv.5 with Dark Masters in text
AGUMON = "BT1-010"            # generic Lv.3 filler


def _filler_deck(extra=None):
    """Build a minimal deck with filler cards + extras."""
    base = [AGUMON] * 50
    if extra:
        for card_id in extra:
            base[0] = card_id
            base = base  # just replace first slots
    return base


@pytest.mark.behavioral
class TestBT17068Effect0RevealedLevel:
    """Effect 0: While revealed, treated as level 6."""

    def test_revealed_level_override_attribute(self, debug_runner):
        """Effect should have _revealed_level_override = 6."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        effects = card.effect_list(None)
        has_override = any(getattr(e, '_revealed_level_override', None) == 6 for e in effects)
        assert has_override, "Should have _revealed_level_override = 6"


@pytest.mark.behavioral
class TestBT17068Effect1CostReduction:
    """Effect 1: BeforePayCost — return Apocalymon from trash, reduce cost by 3."""

    def test_cost_reduction_property_set(self, debug_runner):
        """Effect should use cost_reduction = 3 (not _temp_play_cost_reduction)."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if getattr(e, 'timing', None) == EffectTiming.BeforePayCost]
        assert len(bpc) == 1, "Should have exactly 1 BeforePayCost effect"
        assert bpc[0].cost_reduction == 3, (
            f"cost_reduction should be 3, got {bpc[0].cost_reduction}")

    def test_condition_requires_apocalymon_in_trash(self, debug_runner):
        """Condition should check for Apocalymon in trash."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        effects = card.effect_list(None)
        bpc = [e for e in effects if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        # No Apocalymon in trash -> condition fails
        ctx = {'card_source': card, 'played_card': card}
        assert not bpc.can_use_condition(ctx), (
            "Should fail without Apocalymon in trash")

        # Put Apocalymon in trash -> condition passes
        runner.inject_card(1, APOCALYMON, "trash")
        assert bpc.can_use_condition(ctx), (
            "Should pass with Apocalymon in trash")

    def test_condition_requires_card_source_is_self(self, debug_runner):
        """Condition should only activate for this card being played."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        other = runner.inject_card(1, AGUMON, "hand")
        runner.inject_card(1, APOCALYMON, "trash")

        effects = card.effect_list(None)
        bpc = [e for e in effects if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        # With a different card_source -> should fail
        ctx_other = {'card_source': other, 'played_card': other}
        assert not bpc.can_use_condition(ctx_other), (
            "Should NOT activate for another card being played")

    def test_process_returns_apocalymon_to_deck_bottom(self, debug_runner):
        """Process callback should move Apocalymon from trash to deck bottom."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        runner.inject_card(1, APOCALYMON, "trash")

        effects = card.effect_list(None)
        bpc = [e for e in effects if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        p1 = runner.game.player1
        library_before = len(p1.library_cards)
        trash_before = len(p1.trash_cards)

        ctx = {'player': p1, 'game': runner.game, 'card_source': card}
        bpc.on_process_callback(ctx)

        # Apocalymon should be at bottom of library
        assert len(p1.library_cards) == library_before + 1, "Library should grow by 1"
        bottom_card = p1.library_cards[-1]
        assert 'Apocalymon' in (getattr(bottom_card, 'card_names', []) or []), (
            "Bottom card should be Apocalymon")

        # Trash should shrink
        apoc_in_trash = [c for c in p1.trash_cards
                         if 'Apocalymon' in (getattr(c, 'card_names', []) or [])]
        assert len(apoc_in_trash) == 0, "Apocalymon should no longer be in trash"

    def test_process_does_not_set_temp_cost_reduction(self, debug_runner):
        """Process should NOT use _temp_play_cost_reduction (broken pattern)."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        runner.inject_card(1, APOCALYMON, "trash")

        effects = card.effect_list(None)
        bpc = [e for e in effects if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        p1 = runner.game.player1
        ctx = {'player': p1, 'game': runner.game, 'card_source': card}
        bpc.on_process_callback(ctx)

        # Should NOT touch _temp_play_cost_reduction
        temp_reduction = getattr(p1, '_temp_play_cost_reduction', 0)
        assert temp_reduction == 0, (
            f"Should not set _temp_play_cost_reduction, got {temp_reduction}")

    def test_calculate_play_cost_with_apocalymon_in_trash(self, debug_runner):
        """calculate_play_cost should return 5 (8 - 3) when Apocalymon is in trash."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")
        runner.inject_card(1, APOCALYMON, "trash")

        p1 = runner.game.player1
        cost = runner.game.calculate_play_cost(p1, card, source_zone="hand")
        assert cost == 5, f"Play cost should be 5 (8 - 3), got {cost}"

    def test_calculate_play_cost_without_apocalymon(self, debug_runner):
        """calculate_play_cost should return 8 when no Apocalymon in trash."""
        runner = debug_runner(initial_memory=10)
        card = runner.inject_card(1, MEPHISTOMON, "hand")

        p1 = runner.game.player1
        cost = runner.game.calculate_play_cost(p1, card, source_zone="hand")
        assert cost == 8, f"Play cost should be 8 without Apocalymon, got {cost}"


@pytest.mark.behavioral
class TestBT17068Effect2OnDeletion:
    """Effect 2: [On Deletion] If deleted by effect, play Gulfmon or Lv.6 DM from hand/trash."""

    def test_on_deletion_effect_exists(self, debug_runner):
        """Should have an On Deletion effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects if e.is_on_deletion]
        assert len(on_del) >= 1, "Should have an On Deletion effect"
        assert on_del[0].is_optional, "On Deletion should be optional"

    def test_condition_checks_removal_cause_is_effect(self, debug_runner):
        """Condition should check removal_cause == 'effect' (not is_by_effect)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        runner.inject_card(1, GULFMON, "hand")  # valid target
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects if e.is_on_deletion][0]

        # Deleted by effect (removal_cause='effect') -> should pass
        ctx_effect = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'deleted_permanent': perm,
            'removal_cause': 'effect',
        }
        assert on_del.can_use_condition(ctx_effect), (
            "Should pass when deleted by effect (removal_cause='effect')")

        # Deleted by battle (removal_cause='battle') -> should fail
        ctx_battle = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'deleted_permanent': perm,
            'removal_cause': 'battle',
        }
        assert not on_del.can_use_condition(ctx_battle), (
            "Should fail when deleted by battle")

    def test_condition_requires_valid_target_in_hand_or_trash(self, debug_runner):
        """Condition should require a valid play target (Gulfmon or Lv.6 DM) in hand or trash."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects if e.is_on_deletion][0]

        base_ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'deleted_permanent': perm,
            'removal_cause': 'effect',
        }

        # No valid target -> should fail
        assert not on_del.can_use_condition(base_ctx), (
            "Should fail without valid target")

        # Gulfmon in hand -> should pass
        runner.inject_card(1, GULFMON, "hand")
        assert on_del.can_use_condition(base_ctx), (
            "Should pass with Gulfmon in hand")

    def test_condition_accepts_lv6_dark_masters_trait(self, debug_runner):
        """Condition should accept a Lv.6 Digimon with Dark Masters trait."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects if e.is_on_deletion][0]

        # Piedmon (Lv.6, Dark Masters trait) in trash
        runner.inject_card(1, PIEDMON, "trash")

        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'deleted_permanent': perm,
            'removal_cause': 'effect',
        }
        assert on_del.can_use_condition(ctx), (
            "Should pass with Lv.6 Dark Masters Digimon in trash")

    def test_process_uses_effect_play_from_zone(self, debug_runner):
        """Process should call effect_play_from_zone to let RL agent select."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects if e.is_on_deletion][0]

        runner.inject_card(1, GULFMON, "hand")

        # Track calls to effect_play_from_zone
        calls = []
        original = runner.game.effect_play_from_zone

        def mock_play_from_zone(player, zone, filter_fn, **kwargs):
            calls.append({'zone': zone, 'free': kwargs.get('free', False)})
            original(player, zone, filter_fn, **kwargs)

        runner.game.effect_play_from_zone = mock_play_from_zone
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_del.on_process_callback(ctx)
        finally:
            runner.game.effect_play_from_zone = original

        assert len(calls) >= 1, "Should call effect_play_from_zone"
        assert calls[0]['zone'] == 'hand_or_trash', "Zone should be 'hand_or_trash'"
        assert calls[0]['free'] is True, "Should play without paying cost"


@pytest.mark.behavioral
class TestBT17068Effect3InheritedWhenAttacking:
    """Effect 3: Inherited [When Attacking] OPT place DM from trash, +2000 DP."""

    def test_inherited_effect_attributes(self, debug_runner):
        """Should be inherited, on_attack, optional, once-per-turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON])
        card = perm.top_card
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack]
        assert len(inherited_atk) == 1, "Should have exactly 1 inherited on-attack effect"
        eff = inherited_atk[0]
        assert eff.is_optional, "Should be optional"
        assert eff.max_count_per_turn == 1, "Should be once per turn"

    def test_condition_requires_dark_masters_text_lv5_or_lower_in_trash(self, debug_runner):
        """Condition should require a Lv.5- card with 'Dark Masters' in text in trash."""
        runner = debug_runner(initial_memory=10)
        # Stack Mephistomon under a Lv.6 to simulate inherited context
        perm = runner.place_on_field(1, [MEPHISTOMON, PIEDMON])
        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == MEPHISTOMON:
                card = cs
                break
        assert card is not None
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack][0]

        ctx = {'attacker': perm, 'permanent': perm}

        # No qualifying card in trash -> fail
        assert not inherited_atk.can_use_condition(ctx), (
            "Should fail without qualifying card in trash")

        # Add Scorpiomon (Lv.5, Dark Masters text) to trash
        runner.inject_card(1, SCORPIOMON, "trash")
        assert inherited_atk.can_use_condition(ctx), (
            "Should pass with Scorpiomon (Lv.5, Dark Masters text) in trash")

    def test_condition_rejects_lv6_card(self, debug_runner):
        """Condition should reject a Lv.6 card even with Dark Masters in text."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON, PIEDMON])
        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == MEPHISTOMON:
                card = cs
                break
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack][0]

        # Piedmon is Lv.6 with Dark Masters trait, but level > 5 so should fail
        runner.inject_card(1, PIEDMON, "trash")
        ctx = {'attacker': perm, 'permanent': perm}
        assert not inherited_atk.can_use_condition(ctx), (
            "Should reject Lv.6 card (not Lv.5 or lower)")

    def test_condition_only_for_this_digimon(self, debug_runner):
        """Should only trigger for the Digimon this card is under."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON, PIEDMON])
        other = runner.place_on_field(1, [AGUMON])
        runner.inject_card(1, SCORPIOMON, "trash")

        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == MEPHISTOMON:
                card = cs
                break
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack][0]

        # Other Digimon attacking -> should fail
        ctx_other = {'attacker': other, 'permanent': other}
        assert not inherited_atk.can_use_condition(ctx_other), (
            "Should not trigger for a different Digimon")

    def test_process_uses_selection_phase(self, debug_runner):
        """Process should use a selection phase (not auto-select first card)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON, PIEDMON])
        runner.inject_card(1, SCORPIOMON, "trash")
        runner.inject_card(1, MEPHISTOMON, "trash")  # another qualifying card

        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == MEPHISTOMON:
                card = cs
                break
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack][0]

        # Track selection request
        selection_requested = []
        original_request = runner.game.request_selection

        def mock_request(phase, player, callback, valid_indices=None, **kwargs):
            selection_requested.append({
                'phase': phase,
                'valid_count': len(valid_indices) if valid_indices else 0,
            })
            # Auto-select first valid
            if valid_indices:
                callback(valid_indices[0])

        runner.game.request_selection = mock_request
        try:
            ctx = {
                'player': runner.game.player1,
                'game': runner.game,
                'permanent': perm,
            }
            inherited_atk.on_process_callback(ctx)
        finally:
            runner.game.request_selection = original_request

        assert len(selection_requested) >= 1, (
            "Should use request_selection for trash card selection (not auto-select)")
        assert selection_requested[0]['phase'] == GamePhase.SelectTrash, (
            f"Phase should be SelectTrash, got {selection_requested[0]['phase']}")
        assert selection_requested[0]['valid_count'] >= 2, (
            f"Should have 2+ valid choices, got {selection_requested[0]['valid_count']}")

    def test_process_places_card_as_bottom_digi_and_grants_dp(self, debug_runner):
        """Process should place selected card from trash as bottom digi-card and +2000 DP."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [MEPHISTOMON, PIEDMON])
        runner.inject_card(1, SCORPIOMON, "trash")

        card = None
        for cs in perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == MEPHISTOMON:
                card = cs
                break
        effects = card.effect_list(None)
        inherited_atk = [e for e in effects if e.is_inherited_effect and e.is_on_attack][0]

        p1 = runner.game.player1
        stack_before = len(perm.card_sources)
        dp_before = perm.dp

        # Mock selection to pick the Scorpiomon from trash
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START
        scorpiomon_idx = None
        for i, tc in enumerate(p1.trash_cards):
            if tc.c_entity_base and tc.c_entity_base.card_id == SCORPIOMON:
                scorpiomon_idx = i
                break

        original_request = runner.game.request_selection

        def mock_request(phase, player, callback, valid_indices=None, **kwargs):
            callback(SEL_TRASH_START + scorpiomon_idx)

        runner.game.request_selection = mock_request
        try:
            ctx = {'player': p1, 'game': runner.game, 'permanent': perm}
            inherited_atk.on_process_callback(ctx)
        finally:
            runner.game.request_selection = original_request

        # Stack should grow by 1
        assert len(perm.card_sources) == stack_before + 1, (
            f"Stack should grow by 1. Before: {stack_before}, After: {len(perm.card_sources)}")

        # Scorpiomon should be at bottom of stack
        bottom = perm.card_sources[0]
        assert bottom.c_entity_base.card_id == SCORPIOMON, (
            f"Bottom card should be Scorpiomon, got {bottom.c_entity_base.card_id}")

        # Scorpiomon should no longer be in trash
        scorpiomon_in_trash = [c for c in p1.trash_cards
                               if c.c_entity_base and c.c_entity_base.card_id == SCORPIOMON]
        assert len(scorpiomon_in_trash) == 0, "Scorpiomon should not be in trash anymore"

        # DP should increase by 2000
        assert perm.dp == dp_before + 2000, (
            f"DP should increase by 2000. Before: {dp_before}, After: {perm.dp}")
