"""Behavioral tests for BT21-013 Agunimon (Lv.4 Red Dragonkin/Hybrid/LIBERATOR).

Card text:
[When Digivolving] You may place 1 [Hybrid] or [Hero] trait Digimon card from
your hand or trash as this Digimon's bottom digivolution card or under any of
your red Tamers with inherited effects.
[When Attacking] This Digimon may digivolve into a red Lv.5 Digimon card in
your hand with the digivolution cost reduced by 2.
Inherited: [Your Turn] This Digimon gets +2000 DP.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestBT21013WhenDigivolving:
    """[When Digivolving] place 1 Hybrid/Hero Digimon from hand/trash under this or red Tamer."""

    def test_wd_effect_timing_and_flags(self, debug_runner):
        """WD effect must use OnEnterFieldAnyone timing with is_when_digivolving flag."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])
        top_card = perm.top_card
        effects = top_card.effect_list(None)

        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) == 1, "Should have exactly 1 WD effect"

        wd = wd_effects[0]
        assert wd.is_optional, "WD effect must be optional ('You may')"

    def test_wd_places_hybrid_from_hand(self, debug_runner):
        """WD effect should offer Hybrid-form Digimon from hand as valid targets."""
        runner = debug_runner(initial_memory=10)
        # Place Agunimon on field
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])

        # Inject a Hybrid-form Digimon into hand (AD1-002 Aldamon: Hybrid form, Wizard type)
        runner.inject_card(1, "AD1-002", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        # Fire the process callback
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should enter a selection phase with valid options including the Hybrid card
        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget phase for choosing a Hybrid/Hero Digimon"
        )

    def test_wd_places_hero_from_hand(self, debug_runner):
        """WD effect should offer Hero-trait Digimon from hand as valid targets."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])

        # AD1-003 WarGrowlmon has Hero trait (in type_eng)
        runner.inject_card(1, "AD1-003", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget phase for Hero-trait Digimon"
        )

    def test_wd_places_from_trash(self, debug_runner):
        """WD effect should also offer Hybrid/Hero Digimon from trash."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])

        # Put a Hybrid Digimon in trash
        runner.inject_card(1, "AD1-002", "trash")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should enter SelectTarget phase for Hybrid Digimon from trash"
        )

    def test_wd_rejects_non_hybrid_non_hero(self, debug_runner):
        """WD effect should NOT offer Digimon without Hybrid or Hero trait."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])

        # Clear hand and inject only a non-Hybrid, non-Hero Digimon
        runner.clear_zone(1, "hand")
        # AD1-001 Greymon: Lv.4 red, no Hybrid form, no Hero type
        runner.inject_card(1, "AD1-001", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        from digimon_gym.engine.data.enums import GamePhase
        # With no valid targets, should NOT enter selection phase
        assert game.current_phase != GamePhase.SelectTarget, (
            "Should not enter selection when no Hybrid/Hero Digimon available"
        )

    def test_wd_card_placed_as_bottom_digi_card(self, debug_runner):
        """Selected card should be placed as bottom digivolution card (index 0 in card_sources)."""
        runner = debug_runner(initial_memory=10)
        # Stack: BT1-009 (bottom), BT21-013 (top)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])
        initial_stack_size = len(perm.card_sources)

        # Inject a Hero Digimon into hand
        runner.inject_card(1, "AD1-003", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Select the Hero Digimon from hand
        results = runner.auto_resolve(max_steps=5)

        # The card should now be at the bottom of the stack
        assert len(perm.card_sources) == initial_stack_size + 1, (
            "Stack should have one more card after placing"
        )
        bottom_card = perm.card_sources[0]
        assert bottom_card.c_entity_base.card_id == "AD1-003", (
            "AD1-003 should be placed as the bottom digivolution card"
        )

    def test_wd_card_removed_from_hand(self, debug_runner):
        """Selected card should be removed from hand after placement."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "AD1-003", "hand")

        game = runner.game
        hand_before = len(game.player1.hand_cards)

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve selections (pick card, pick destination)
        runner.auto_resolve(max_steps=5)

        assert len(game.player1.hand_cards) == hand_before - 1, (
            "Card should be removed from hand after placement"
        )

    def test_wd_dest_includes_red_tamer(self, debug_runner):
        """Destination selection should include red Tamers on the field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT1-009", "BT21-013"])
        # Place a red tamer (BT1-085 Tai Kamiya)
        tamer_perm = runner.place_on_field(1, ["BT1-085"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "AD1-003", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # First selection: pick the card from hand
        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget

        # Select first available card
        legal = runner.action_mask()
        runner.execute(legal[0])

        # Now should be selecting destination (this Digimon or red Tamer)
        # Both perm (Agunimon) and tamer_perm should be valid destinations
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should enter destination selection phase"
        )
        # Should have at least 2 options (this Digimon + red Tamer)
        legal2 = runner.action_mask()
        assert len(legal2) >= 2, (
            f"Should have at least 2 destination options (self + red tamer), got {len(legal2)}"
        )


@pytest.mark.behavioral
class TestBT21013WhenAttacking:
    """[When Attacking] digivolve into red Lv.5 from hand with cost -2."""

    def test_wa_effect_uses_on_use_attack_timing(self, debug_runner):
        """WA effect MUST use OnUseAttack timing (not OnAllyAttack)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])
        top_card = perm.top_card
        effects = top_card.effect_list(None)

        wa_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
        ]
        assert len(wa_effects) == 1, (
            "Should have exactly 1 OnUseAttack effect (not OnAllyAttack)"
        )

        # Verify NO OnAllyAttack effects exist for this card
        ally_attack_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnAllyAttack
        ]
        assert len(ally_attack_effects) == 0, (
            "Must NOT use OnAllyAttack timing for [When Attacking] on self"
        )

        wa = wa_effects[0]
        assert wa.is_optional, "WA digivolve effect must be optional ('may')"

    def test_wa_filters_red_lv5_digimon(self, debug_runner):
        """WA digivolve filter should accept red Lv.5 Digimon from hand."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])

        runner.clear_zone(1, "hand")
        # AD1-002 Aldamon: red Lv.5 Hybrid Digimon
        runner.inject_card(1, "AD1-002", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa = [e for e in effects if e.timing == EffectTiming.OnUseAttack][0]

        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should enter selection for digivolve
        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            "Should offer red Lv.5 Digimon for digivolve"
        )

    def test_wa_rejects_non_red_lv5(self, debug_runner):
        """WA digivolve filter should reject non-red Lv.5 Digimon."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])

        runner.clear_zone(1, "hand")
        # BT1-044 Kabuterimon: green Lv.4 (not red, not Lv.5)
        runner.inject_card(1, "BT1-044", "hand")

        game = runner.game
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa = [e for e in effects if e.timing == EffectTiming.OnUseAttack][0]

        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        from digimon_gym.engine.data.enums import GamePhase
        assert game.current_phase != GamePhase.SelectTarget, (
            "Should not offer non-red/non-Lv.5 Digimon for digivolve"
        )

    def test_wa_cost_reduction_is_2(self, debug_runner):
        """WA digivolve should reduce cost by 2 (not 1).

        effect_digivolve_from_hand uses play_cost (get_cost_itself) as the base,
        not the normal evo cost. AD1-002 play_cost=8, so with cost_reduction=2
        the effective cost is 6.
        """
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "AD1-002", "hand")

        game = runner.game
        memory_before = game.memory

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa = [e for e in effects if e.timing == EffectTiming.OnUseAttack][0]

        wa.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Select the card to digivolve into
        runner.auto_resolve(max_steps=5)

        # effect_digivolve_from_hand uses get_cost_itself (play_cost) as base
        # AD1-002 play_cost=8, cost_reduction=2 -> effective cost = 6
        import json
        from digimon_gym.data_paths import CARDS_JSON
        with open(CARDS_JSON, 'r', encoding='utf-8') as f:
            cards = json.load(f)
        ad1_002 = cards.get('AD1-002', {})
        play_cost = ad1_002.get('play_cost', 0)
        expected_paid = max(0, play_cost - 2)
        actual_paid = memory_before - game.memory
        assert actual_paid == expected_paid, (
            f"Should pay {expected_paid} (play_cost {play_cost} - 2 reduction), "
            f"but paid {actual_paid}"
        )


@pytest.mark.behavioral
class TestBT21013Inherited:
    """Inherited: [Your Turn] This Digimon gets +2000 DP."""

    def test_inherited_dp_modifier_value(self, debug_runner):
        """Inherited effect should provide +2000 DP."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013"])
        top_card = perm.top_card
        effects = top_card.effect_list(None)

        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited) == 1, "Should have exactly 1 inherited effect"

        inh = inherited[0]
        assert inh.dp_modifier == 2000, f"Inherited DP modifier should be 2000, got {inh.dp_modifier}"

    def test_inherited_active_on_your_turn(self, debug_runner):
        """Inherited DP modifier should be active during owner's turn."""
        runner = debug_runner(initial_memory=10)
        # Place BT21-013 as a digivolution source under a Lv.5 Digimon
        perm = runner.place_on_field(1, ["BT21-013", "BT1-020"])

        game = runner.game
        # Ensure it's player 1's turn
        game.turn_player = game.player1
        game.player1.is_my_turn = True

        # Get inherited effect from BT21-013 (bottom card)
        bt21_013_card = perm.card_sources[0]
        effects = bt21_013_card.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited) == 1

        inh = inherited[0]
        assert inh.can_use_condition({}), (
            "Inherited condition should pass on owner's turn"
        )

    def test_inherited_inactive_on_opponent_turn(self, debug_runner):
        """Inherited DP modifier should NOT be active during opponent's turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT21-013", "BT1-020"])

        game = runner.game
        # Set to opponent's turn
        game.turn_player = game.player2
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        bt21_013_card = perm.card_sources[0]
        effects = bt21_013_card.effect_list(None)
        inherited = [e for e in effects if getattr(e, 'is_inherited_effect', False)]
        assert len(inherited) == 1

        inh = inherited[0]
        assert not inh.can_use_condition({}), (
            "Inherited condition should fail on opponent's turn"
        )
