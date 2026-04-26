"""Behavioral tests for EX10-028 Landramon (Lv.4 Black, Mineral/LIBERATOR, DP 4000, Cost 4).

Card text:
[On Play] [When Digivolving] By trashing any 1 card with the [Mineral] or [Rock] trait
from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock]
trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends.

Inherited Effect:
When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution
cards, delete 1 of your opponent's Digimon with a play cost of 4 or less.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX10028Landramon:
    """Tests for EX10-028 Landramon."""

    # -- Clause 1: [On Play] effect (trash source -> grant keywords) --

    def test_on_play_trashes_mineral_source_and_grants_keywords(self, debug_runner):
        """[On Play] Should trash a Mineral/Rock digivolution card and grant
        Reboot + Blocker + 3000 DP to a Mineral/Rock trait Digimon."""
        runner = debug_runner(initial_memory=10)

        # Stack bottom-to-top: BT2-054 (Gotsumon, Lv.3, Rock) then EX10-028 (Landramon, Mineral)
        perm = runner.place_on_field(1, ["BT2-054", "EX10-028"])
        game = runner.game

        # Verify setup: top card is Landramon with Mineral trait
        assert perm.top_card.c_entity_base.card_id == 'EX10-028'
        assert perm.has_trait('Mineral'), "Landramon should have Mineral trait"

        # Get the On Play effect from the top card
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        assert len(on_play_effects) == 1, "Should have exactly 1 On Play effect"

        effect = on_play_effects[0]

        # Condition should pass (we have a Digimon with a trashable Rock source)
        assert effect.can_use_condition({}), \
            "Condition should pass when there is a Mineral/Rock source to trash"

        # Mock selections
        select_own_calls = []

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            select_own_calls.append({
                'filter_fn': filter_fn, 'is_optional': is_optional, 'callback': callback
            })
            # Find first matching permanent
            for p in player.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have been called twice:
        # 1st: select which Digimon to trash a source from (optional)
        # 2nd: select which Mineral/Rock Digimon to receive the buffs (mandatory)
        assert len(select_own_calls) == 2, \
            f"Should have 2 select_own calls, got {len(select_own_calls)}"

        # First selection should be optional (card text says "By trashing...")
        assert select_own_calls[0]['is_optional'] is True, \
            "Trash source selection should be optional"
        # Second selection should be mandatory
        assert select_own_calls[1]['is_optional'] is False, \
            "Grant target selection should be mandatory"

        # The BT2-054 should now be in trash
        snap = runner.snapshot()
        assert 'BT2-054' in snap.p1_trash, \
            "BT2-054 should be trashed from digivolution cards"

    def test_on_play_condition_fails_without_trashable_source(self, debug_runner):
        """[On Play] Condition should fail when no Digimon has a Mineral/Rock source."""
        runner = debug_runner(initial_memory=10)

        # Place Landramon alone (no digivolution cards = nothing to trash)
        perm = runner.place_on_field(1, ["EX10-028"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        assert not effect.can_use_condition({}), \
            "Condition should fail when no Mineral/Rock digivolution source exists"

    def test_grant_target_filter_requires_mineral_or_rock(self, debug_runner):
        """Grant target must be a Digimon with [Mineral] or [Rock] trait."""
        runner = debug_runner(initial_memory=10)

        # Landramon (Mineral) with a Rock source underneath
        perm = runner.place_on_field(1, ["BT2-054", "EX10-028"])
        # A non-Mineral/Rock Digimon should NOT be selectable as grant target
        other = runner.place_on_field(1, ["ST1-03"])  # Agumon (Reptile trait)
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        grant_targets = []
        select_call_count = [0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            select_call_count[0] += 1
            if select_call_count[0] == 1:
                # First call: select trash source permanent
                callback(perm)
            else:
                # Second call: check which permanents pass the grant filter
                for p in player.battle_area:
                    if filter_fn and filter_fn(p):
                        grant_targets.append(p)
                if grant_targets:
                    callback(grant_targets[0])

        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Landramon (Mineral) should be targetable, Agumon should not
        assert perm in grant_targets, "Landramon (Mineral) should be a valid grant target"
        assert other not in grant_targets, "Agumon (Reptile) should NOT be a valid grant target"

    # -- Clause 2: [When Digivolving] effect --

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a [When Digivolving] effect with same trash-and-grant behavior."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT2-054", "EX10-028"])

        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects
                      if e.timing == EffectTiming.OnEnterFieldAnyone
                      and e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have exactly 1 When Digivolving effect"

    # -- Clause 3: Inherited effect --

    def test_inherited_effect_triggers_on_trash_from_mineral_digimon(self, debug_runner):
        """Inherited: When THIS card is trashed from a Mineral/Rock Digimon's
        digivolution cards, should delete 1 opponent Digimon with cost 4 or less."""
        runner = debug_runner(initial_memory=10)

        # Stack bottom-to-top: EX10-028 (Landramon) under BT4-066 (Golemon, Mineral, Lv.4)
        # Golemon has Mineral trait, so the permanent has Mineral trait
        top_perm = runner.place_on_field(1, ["EX10-028", "BT4-066"])
        # Opponent has a cost-3 Digimon (ST1-03 Agumon)
        opp_target = runner.place_on_field(2, ["ST1-03"])
        game = runner.game

        # Get the inherited effect from EX10-028 (the card below top)
        landramon_cs = top_perm.card_sources[0]  # bottom card = EX10-028
        assert landramon_cs.c_entity_base.card_id == 'EX10-028', \
            "Bottom card should be EX10-028"

        effects = landramon_cs.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        assert len(inherited_effects) == 1, "EX10-028 should have 1 inherited effect"

        inh_effect = inherited_effects[0]

        # Condition should pass when:
        # 1. The permanent has Mineral trait (Golemon = Mineral) -- checked via event_permanent
        # 2. THIS card (EX10-028) was in the trashed_cards list
        context_pass = {
            'event_permanent': top_perm,
            'trashed_cards': [landramon_cs],
        }
        assert inh_effect.can_use_condition(context_pass), \
            "Inherited condition should pass when card is in trashed_cards and permanent has Mineral trait"

    def test_inherited_effect_does_not_trigger_for_other_trashed_card(self, debug_runner):
        """Inherited: Should NOT trigger when a DIFFERENT card was trashed (not this card)."""
        runner = debug_runner(initial_memory=10)

        # Stack bottom-to-top: BT2-054 (Rock), EX10-028 (Mineral), BT4-066 (Mineral) on top
        top_perm = runner.place_on_field(1, ["BT2-054", "EX10-028", "BT4-066"])
        game = runner.game

        landramon_cs = top_perm.card_sources[1]  # middle card = EX10-028
        gotsumon_cs = top_perm.card_sources[0]    # bottom card = BT2-054

        assert landramon_cs.c_entity_base.card_id == 'EX10-028'
        assert gotsumon_cs.c_entity_base.card_id == 'BT2-054'

        effects = landramon_cs.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh_effect = inherited_effects[0]

        # Trash Gotsumon (NOT Landramon) -- Landramon's inherited should NOT trigger
        context_other = {
            'event_permanent': top_perm,
            'trashed_cards': [gotsumon_cs],  # different card trashed
        }
        assert not inh_effect.can_use_condition(context_other), \
            "Inherited should NOT trigger when a different card was trashed"

    def test_inherited_effect_does_not_trigger_from_non_mineral_rock_digimon(self, debug_runner):
        """Inherited: Should NOT trigger if the Digimon doesn't have Mineral or Rock trait."""
        runner = debug_runner(initial_memory=10)

        # Stack bottom-to-top: EX10-028 under ST1-07 (Greymon, Lv.4, Dinosaur trait)
        top_perm = runner.place_on_field(1, ["EX10-028", "ST1-07"])
        game = runner.game

        landramon_cs = top_perm.card_sources[0]  # bottom = EX10-028
        assert landramon_cs.c_entity_base.card_id == 'EX10-028'

        effects = landramon_cs.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh_effect = inherited_effects[0]

        # Permanent has Dinosaur trait (not Mineral/Rock)
        context_wrong_trait = {
            'event_permanent': top_perm,
            'trashed_cards': [landramon_cs],
        }
        assert not inh_effect.can_use_condition(context_wrong_trait), \
            "Inherited should NOT trigger from a non-Mineral/Rock Digimon"

    def test_inherited_delete_filter_cost_4_or_less(self, debug_runner):
        """Inherited: Delete target must be opponent's Digimon with play cost 4 or less."""
        runner = debug_runner(initial_memory=10)

        top_perm = runner.place_on_field(1, ["EX10-028", "BT4-066"])
        # Opponent with cost 3 (should be targetable) and cost 4 (should be targetable)
        opp_low = runner.place_on_field(2, ["ST1-03"])   # Agumon, cost 3
        opp_4 = runner.place_on_field(2, ["BT1-015"])    # Greymon, cost 4

        game = runner.game

        landramon_cs = top_perm.card_sources[0]
        effects = landramon_cs.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh_effect = inherited_effects[0]

        # Process the effect
        selected_targets = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    selected_targets.append(p)
            if selected_targets:
                callback(selected_targets[0])

        game.effect_select_opponent_permanent = mock_select_opp

        inh_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # ST1-03 (cost 3) and ST1-07 (cost 4) should both be valid (cost 4 or less)
        assert opp_low in selected_targets, "Cost 3 Digimon should be targetable"
        assert opp_4 in selected_targets, "Cost 4 Digimon should be targetable (4 or less)"

    def test_inherited_delete_filter_excludes_high_cost(self, debug_runner):
        """Inherited: Should NOT target Digimon with play cost > 4."""
        runner = debug_runner(initial_memory=10)

        top_perm = runner.place_on_field(1, ["EX10-028", "BT4-066"])
        # ST1-11 WarGreymon (cost 12) -- too high
        opp_high = runner.place_on_field(2, ["ST1-11"])

        game = runner.game

        landramon_cs = top_perm.card_sources[0]
        effects = landramon_cs.effect_list(EffectTiming.OnDigivolutionCardDiscarded)
        inherited_effects = [e for e in effects if e.is_inherited_effect]
        inh_effect = inherited_effects[0]

        selected_targets = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn and filter_fn(p):
                    selected_targets.append(p)

        game.effect_select_opponent_permanent = mock_select_opp

        inh_effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        assert opp_high not in selected_targets, \
            "Cost 12 Digimon should NOT be targetable for delete"

    # -- Integration: trash fires OnDigivolutionCardDiscarded --

    def test_on_play_trash_fires_on_digi_card_discarded(self, debug_runner):
        """[On Play] Trashing a digi source should fire OnDigivolutionCardDiscarded timing.
        This ensures the inherited effect can trigger."""
        runner = debug_runner(initial_memory=10)

        # Landramon on top with Rock source below
        perm = runner.place_on_field(1, ["BT2-054", "EX10-028"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [e for e in effects
                           if e.timing == EffectTiming.OnEnterFieldAnyone
                           and e.is_on_play]
        effect = on_play_effects[0]

        execute_effects_calls = []
        original_execute = game.execute_effects

        def tracking_execute(timing, extra_context=None):
            execute_effects_calls.append((timing, extra_context))
            return original_execute(timing, extra_context)

        game.execute_effects = tracking_execute

        # Mock selections
        select_call_count = [0]

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, prompt=None):
            select_call_count[0] += 1
            for p in player.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        game.effect_select_own_permanent = mock_select_own

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Check that OnDigivolutionCardDiscarded was fired
        digi_card_trash_calls = [
            (t, ctx) for t, ctx in execute_effects_calls
            if t == EffectTiming.OnDigivolutionCardDiscarded
        ]
        assert len(digi_card_trash_calls) >= 1, \
            "Should fire OnDigivolutionCardDiscarded when trashing a digivolution source"

        # Verify the context includes trashed_cards and permanent
        call_timing, call_ctx = digi_card_trash_calls[0]
        assert 'trashed_cards' in call_ctx, "Context should include trashed_cards"
        assert 'permanent' in call_ctx, "Context should include permanent"
        assert len(call_ctx['trashed_cards']) == 1, "Should trash exactly 1 card"
