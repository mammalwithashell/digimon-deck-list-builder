"""Behavioral tests for EX10-012 MetalSeadramon (Lv.6 Blue Cyborg/Dark Masters, DP 11000, Cost 11).

Card text:
[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters]
    in their texts, you may play this card with the play cost reduced by 5.
    At turn end, delete the Digimon this effect played.
[On Play] [When Attacking] 1 of your opponent's Digimon and 1 of their Tamers
    can't suspend until their turn ends.
[All Turns] This Digimon can only digivolve into [Apocalymon].
[On Deletion] If you have no blue face-up security cards, place this Digimon
    face up as the bottom security card.
Inherited: [Security] If this card was face-up, you may play 1 level 5 or lower
    card with [Dark Masters] in its text from your hand or trash without paying
    the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestEX10012MetalSeadramon:
    """Tests for EX10-012 MetalSeadramon."""

    # ----------------------------------------------------------------
    # Clause 1: [Hand] [Main] Cost reduction by 5 (conditional)
    # ----------------------------------------------------------------

    def test_cost_reduction_condition_passes_empty_field(self, debug_runner):
        """Cost reduction condition should pass when player has no Digimon on field."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Inject EX10-012 into hand
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        effects = card.effect_list(None)
        bpc_effects = [e for e in effects
                       if getattr(e, 'timing', None) == EffectTiming.BeforePayCost]
        assert len(bpc_effects) >= 1, "Should have BeforePayCost effect"

        bpc = bpc_effects[0]
        ctx = {'card_source': card, 'player': player}
        assert bpc.can_use_condition(ctx), \
            "Cost reduction should be available with empty field"

    def test_cost_reduction_condition_passes_with_dark_masters_digimon(self, debug_runner):
        """Cost reduction should pass when field has only Dark Masters Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a Dark Masters digimon on field (BT15-027 Scorpiomon has Dark Masters in text)
        runner.place_on_field(1, ["BT15-027"])
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        effects = card.effect_list(None)
        bpc = [e for e in effects
               if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        ctx = {'card_source': card, 'player': player}
        assert bpc.can_use_condition(ctx), \
            "Cost reduction should pass with only Dark Masters Digimon on field"

    def test_cost_reduction_condition_fails_with_non_dark_masters(self, debug_runner):
        """Cost reduction should fail when field has a non-Dark-Masters Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place a non-Dark-Masters digimon (ST1-03 Agumon)
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        effects = card.effect_list(None)
        bpc = [e for e in effects
               if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        ctx = {'card_source': card, 'player': player}
        assert not bpc.can_use_condition(ctx), \
            "Cost reduction should fail with non-Dark-Masters Digimon on field"

    def test_cost_reduction_leak_guard(self, debug_runner):
        """BeforePayCost should NOT activate for other cards (leak guard)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX10-012"])
        runner.inject_card(1, "ST1-03", "hand")

        game = runner.game
        player = game.player1
        # Get the EX10-012 card's BeforePayCost effect
        card_012 = perm.top_card
        effects = card_012.effect_list(None)
        bpc = [e for e in effects
               if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        # Try with a DIFFERENT card_source (ST1-03)
        other_card = [c for c in player.hand_cards
                      if c.c_entity_base and c.c_entity_base.card_id == "ST1-03"][0]
        ctx = {'card_source': other_card, 'player': player}
        assert not bpc.can_use_condition(ctx), \
            "BeforePayCost should not activate for other cards"

    def test_cost_reduction_actually_reduces_cost(self, debug_runner):
        """Playing EX10-012 should actually cost 6 (11 - 5) when condition is met."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        # Calculate cost (should be 6 = 11 - 5)
        cost = game.calculate_play_cost(player, card)
        assert cost == 6, f"Expected cost 6 (11-5), got {cost}"

    def test_cost_reduction_uses_cost_reduction_property(self, debug_runner):
        """The BeforePayCost effect should use cost_reduction property, not _temp_play_cost_reduction."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        effects = card.effect_list(None)
        bpc = [e for e in effects
               if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        assert getattr(bpc, 'cost_reduction', 0) == 5, \
            "BeforePayCost effect should have cost_reduction=5"

    # ----------------------------------------------------------------
    # Clause 1b: End of turn delete
    # ----------------------------------------------------------------

    def test_eot_delete_flag_set_on_commit(self, debug_runner):
        """After playing via cost reduction, the card should be flagged for EOT deletion."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.inject_card(1, "EX10-012", "hand")

        game = runner.game
        player = game.player1
        card = [c for c in player.hand_cards
                if c.c_entity_base and c.c_entity_base.card_id == "EX10-012"][0]

        effects = card.effect_list(None)
        bpc = [e for e in effects
               if getattr(e, 'timing', None) == EffectTiming.BeforePayCost][0]

        # Simulate the process callback firing (commit phase)
        ctx = {'player': player, 'game': game, 'card_source': card}
        if bpc.on_process_callback:
            bpc.on_process_callback(ctx)
        assert getattr(card, '_ex10_eot_delete', False), \
            "Card should be flagged for EOT deletion after cost reduction commit"

    def test_eot_delete_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect that deletes self when flagged."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        eot_effects = [e for e in effects
                       if getattr(e, 'timing', None) == EffectTiming.OnEndTurn]
        assert len(eot_effects) >= 1, "Should have OnEndTurn delete effect"

    # ----------------------------------------------------------------
    # Clause 2: [On Play] [When Attacking] can't suspend
    # ----------------------------------------------------------------

    def test_on_play_cant_suspend_effect_exists(self, debug_runner):
        """Should have an OnEnterFieldAnyone effect for On Play can't suspend."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play
                   and 'suspend' in (getattr(e, 'effect_name', '') or '').lower()]
        assert len(on_play) == 1, "Should have On Play can't suspend effect"

    def test_when_attacking_cant_suspend_effect_exists(self, debug_runner):
        """Should have an OnUseAttack effect for When Attacking can't suspend."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects
                     if e.timing == EffectTiming.OnUseAttack
                     and e.is_on_attack]
        assert len(on_attack) == 1, "Should have When Attacking effect"

    def test_on_play_selects_digimon_and_tamer(self, debug_runner):
        """On Play should select 1 opponent Digimon and 1 opponent Tamer."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])
        opp_digi = runner.place_on_field(2, ["ST1-03"])
        opp_tamer = runner.place_on_field(2, ["BT1-085"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        selections = []

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    selections.append(('select', p))
                    callback(p)
                    return

        game.effect_select_opponent_permanent = mock_select_opp

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should have made 2 selections (1 digimon, 1 tamer)
        assert len(selections) == 2, f"Expected 2 selections (digimon+tamer), got {len(selections)}"

    def test_cant_suspend_uses_end_of_opponent_turn_expiry(self, debug_runner):
        """Can't suspend modifier should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])
        opp_digi = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        registered_modifiers = []
        orig_register = game.register_modifier

        def track_register(target, mod_type, **kwargs):
            entry = orig_register(target, mod_type, **kwargs)
            registered_modifiers.append({
                'target': target,
                'type': mod_type,
                'expiry': kwargs.get('expiry', 'permanent'),
            })
            return entry

        game.register_modifier = track_register

        def mock_select_opp(player, callback, filter_fn=None, is_optional=False, prompt=None):
            for p in player.enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    callback(p)
                    return

        game.effect_select_opponent_permanent = mock_select_opp

        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        suspend_mods = [m for m in registered_modifiers
                        if m['type'] == ModifierType.CANNOT_SUSPEND]
        assert len(suspend_mods) >= 1, "Should register CANNOT_SUSPEND modifier"
        for m in suspend_mods:
            assert m['expiry'] == 'end_of_opponent_turn', \
                f"CANNOT_SUSPEND should expire at end_of_opponent_turn, got {m['expiry']}"

    # ----------------------------------------------------------------
    # Clause 3: [All Turns] Can only digivolve into [Apocalymon]
    # ----------------------------------------------------------------

    def test_digivolve_restriction_exists(self, debug_runner):
        """Should have an effect that registers CANNOT_DIGIVOLVE modifier on self."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)

        # Look for the digivolve restriction effect by name/description
        digi_restrict = [e for e in effects
                         if 'apocalymon' in (getattr(e, 'effect_name', '') or '').lower()
                         or ('digivolve' in (getattr(e, 'effect_description', '') or '').lower()
                             and 'Apocalymon' in (getattr(e, 'effect_description', '') or ''))]

        assert len(digi_restrict) >= 1, \
            "Should have [All Turns] digivolve restriction effect"

        # Fire the effect's process callback and check modifier was registered
        game = runner.game
        digi_effect = digi_restrict[0]
        if digi_effect.on_process_callback:
            digi_effect.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })

        has_restriction = game.modifiers.has_modifier(perm, ModifierType.CANNOT_DIGIVOLVE)
        assert has_restriction, \
            "CANNOT_DIGIVOLVE modifier should be registered on self after effect fires"

    # ----------------------------------------------------------------
    # Clause 4: [On Deletion] Place face-up as bottom security
    # ----------------------------------------------------------------

    def test_on_deletion_effect_exists(self, debug_runner):
        """Should have an OnDestroyedAnyone effect with is_on_deletion."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        deletion_effects = [e for e in effects
                            if e.timing == EffectTiming.OnDestroyedAnyone
                            and e.is_on_deletion]
        assert len(deletion_effects) == 1, "Should have On Deletion effect"

    def test_on_deletion_condition_no_blue_face_up_security(self, debug_runner):
        """On Deletion should activate if no blue face-up security exists."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        game = runner.game
        player = game.player1
        card = perm.top_card

        # Clear any existing face-up security to test clean state
        player.face_up_security.clear()

        effects = card.effect_list(None)
        deletion = [e for e in effects
                    if e.timing == EffectTiming.OnDestroyedAnyone
                    and e.is_on_deletion][0]

        # Simulate post-deletion context (permanent removed from field)
        # On Deletion fires after removal, so permanent_of_this_card() = None
        ctx = {'player': player, 'game': game, 'permanent': perm}
        assert deletion.can_use_condition(ctx), \
            "On Deletion condition should pass when no blue face-up security"

    def test_on_deletion_condition_fails_with_blue_face_up_security(self, debug_runner):
        """On Deletion should NOT activate if blue face-up security card exists."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        game = runner.game
        player = game.player1
        card = perm.top_card

        # Clear existing face-up security first
        player.face_up_security.clear()

        # Add a blue card as face-up security (security_top inserts at index 0)
        sec_cs = runner.inject_card(1, "EX10-012", "security_top")
        player.face_up_security.add(sec_cs)

        effects = card.effect_list(None)
        deletion = [e for e in effects
                    if e.timing == EffectTiming.OnDestroyedAnyone
                    and e.is_on_deletion][0]

        ctx = {'player': player, 'game': game, 'permanent': perm}
        assert not deletion.can_use_condition(ctx), \
            "On Deletion condition should fail when blue face-up security exists"

    def test_on_deletion_places_as_bottom_security(self, debug_runner):
        """On Deletion process should place card as bottom security face-up."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        game = runner.game
        player = game.player1
        card = perm.top_card

        effects = card.effect_list(None)
        deletion = [e for e in effects
                    if e.timing == EffectTiming.OnDestroyedAnyone
                    and e.is_on_deletion][0]

        initial_sec_count = len(player.security_cards)
        deletion.on_process_callback({'player': player, 'game': game})

        assert len(player.security_cards) == initial_sec_count + 1, \
            "Should add 1 card to security"
        assert player.security_cards[-1] is card, \
            "Card should be at bottom of security (appended)"
        assert card in player.face_up_security, \
            "Card should be face-up in security"

    # ----------------------------------------------------------------
    # Clause 5: Inherited [Security] Play Lv5 Dark Masters from hand/trash
    # ----------------------------------------------------------------

    def test_security_effect_is_inherited(self, debug_runner):
        """Security effect should be marked as inherited."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        security_effects = [e for e in effects
                            if e.timing == EffectTiming.SecuritySkill
                            and getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have Security effect"

        sec = security_effects[0]
        assert sec.is_inherited_effect, "Security effect should be inherited"
        assert sec.is_optional, "Security effect should be optional"

    def test_security_filter_requires_digimon(self, debug_runner):
        """Security effect filter should only allow Digimon (per C# reference)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX10-012"])

        card = perm.top_card
        effects = card.effect_list(None)
        sec = [e for e in effects
               if e.timing == EffectTiming.SecuritySkill
               and getattr(e, 'is_security_effect', False)][0]

        # Mock the play_from_zone to capture the filter
        game = runner.game
        player = game.player1
        captured_filter = {}

        def mock_play_from_zone(player, zone, filter_fn, free=True, is_optional=True, prompt=""):
            captured_filter['fn'] = filter_fn

        game.effect_play_from_zone = mock_play_from_zone

        # Make the card face-up in security for condition to pass
        player.face_up_security.add(card)

        sec.on_process_callback({'player': player, 'game': game})

        assert 'fn' in captured_filter, "Should call effect_play_from_zone"

        # Create mock card sources to test filter
        class MockCard:
            def __init__(self, is_digi_egg=False, level=None, card_text='', is_digimon=True):
                self.is_digi_egg = is_digi_egg
                self.level = level
                self.card_text = card_text
                self.is_digimon = is_digimon

        fn = captured_filter['fn']

        # Lv5 Digimon with Dark Masters text -> should pass
        assert fn(MockCard(level=5, card_text='Dark Masters', is_digimon=True)), \
            "Lv5 Digimon with Dark Masters should pass"

        # Lv6 Digimon with Dark Masters -> should fail (too high)
        assert not fn(MockCard(level=6, card_text='Dark Masters', is_digimon=True)), \
            "Lv6 Digimon should fail"

        # Lv5 Digimon without Dark Masters -> should fail
        assert not fn(MockCard(level=5, card_text='Some other text', is_digimon=True)), \
            "Digimon without Dark Masters text should fail"

        # Lv4 non-Digimon (Option/Tamer) with Dark Masters -> should fail per C#
        assert not fn(MockCard(level=4, card_text='Dark Masters', is_digimon=False)), \
            "Non-Digimon should fail (C# requires IsDigimon)"
