"""Behavioral tests for BT23-059 Justimon: Blitz Arm (Digimon, Lv.6, Black, CS/Cyborg).

Card text:
<Blocker> (This Digimon can block in the blocker timing.)
[On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing
1 Option card in the battle area, delete 1 of your opponent's Digimon with
the lowest play cost.
[All Turns] [Once Per Turn] When Option cards in the battle area are trashed,
this Digimon unsuspends. Then, your opponent's Digimon's effects don't affect
this Digimon for the turn.

Alt Digivolve:
[Digivolve] [Justimon: Accel Arm]/[Justimon: Critical Arm]: Cost 1
[Digivolve] Lv.5 w/[CS] trait: Cost 3
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


DECK = ["BT23-059"] * 4 + ["BT22-099"] * 4 + ["BT22-017"] * 10 + ["ST5-05"] * 16 + ["BT2-052"] * 16


@pytest.mark.behavioral
class TestBT23059JustimonBlitzArm:
    """Tests for BT23-059 Justimon: Blitz Arm."""

    # -- Blocker --

    def test_has_blocker(self, debug_runner):
        """Should have Blocker keyword."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(None)

        blocker = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker) >= 1, "Should have Blocker"

    # -- Alt-digi: [Justimon: Accel Arm]/[Critical Arm]: Cost 1 --

    def test_alt_digi_from_justimon_variants(self, debug_runner):
        """Should have alt-digi from Justimon: Accel Arm or Critical Arm for cost 1."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(EffectTiming.NoTiming)

        alt_cost1 = [e for e in effects if getattr(e, '_alt_digi_cost', None) == 1]
        assert len(alt_cost1) == 1, "Should have 1 alt-digi for cost 1"
        assert getattr(alt_cost1[0], '_alt_digi_name', None) == 'Justimon: Accel Arm'

    # -- Alt-digi: Lv.5 w/[CS] trait: Cost 3 --

    def test_alt_digi_lv5_cs_trait(self, debug_runner):
        """Should have alt-digi from Lv.5 with [CS] trait for cost 3.
        The _alt_digi_trait must be set to 'CS' so the validator correctly
        restricts to CS trait Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(EffectTiming.NoTiming)

        alt_cost3 = [e for e in effects if getattr(e, '_alt_digi_cost', None) == 3
                     and getattr(e, '_alt_digi_level', None) == 5]
        assert len(alt_cost3) == 1, "Should have 1 alt-digi for Lv.5 cost 3"
        assert getattr(alt_cost3[0], '_alt_digi_trait', None) == 'CS', \
            "Alt-digi Lv.5 cost 3 should require CS trait"

    def test_alt_digi_cs_validation(self, debug_runner):
        """Validator should accept CS Lv.5 and reject non-CS Lv.5 for cost 3 alt-digi."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)

        # CS Lv.5 should be accepted
        cs_perm = runner.place_on_field(1, ["BT22-012"])  # RizeGreymon, CS, Lv.5
        assert _check_alt_digivolve(card, cs_perm), \
            "Should accept CS Lv.5 Digimon for alt-digi"

        # Non-CS Lv.5 should be rejected
        non_cs_perm = runner.place_on_field(1, ["BT2-060"])  # Megadramon, Lv.5, no CS
        assert not _check_alt_digivolve(card, non_cs_perm), \
            "Should reject non-CS Lv.5 Digimon for alt-digi"

    # -- On Play / When Digivolving / When Attacking: Trash Option, Delete --

    def test_on_play_effect_exists(self, debug_runner):
        """Should have OnEnterFieldAnyone effect with is_on_play."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(None)

        on_play = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)]
        assert len(on_play) == 1, "Should have 1 On Play effect"

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have OnEnterFieldAnyone effect with is_when_digivolving."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(None)

        when_digi = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)]
        assert len(when_digi) == 1, "Should have 1 When Digivolving effect"

    def test_when_attacking_effect_exists(self, debug_runner):
        """Should have OnUseAttack effect with is_on_attack."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-059", runner.game.player1)
        effects = card.effect_list(None)

        on_atk = [e for e in effects if e.timing == EffectTiming.OnUseAttack
                  and getattr(e, 'is_on_attack', False)]
        assert len(on_atk) == 1, "Should have 1 When Attacking effect"

    def test_once_per_turn_across_triggers(self, debug_runner):
        """The [Once Per Turn] should be shared across On Play, When Digivolving,
        and When Attacking triggers."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        # Place Justimon on field and give it an Option
        perm = runner.place_on_field(1, ["BT23-059"])
        opt_perm = runner.place_on_field(1, ["BT22-099"], turn_played=-2)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        on_play = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)]
        when_digi = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)]
        on_atk = [e for e in effects if e.timing == EffectTiming.OnUseAttack
                  and getattr(e, 'is_on_attack', False)]

        assert len(on_play) == 1
        assert len(when_digi) == 1
        assert len(on_atk) == 1

        # All three should have max_count_per_turn = 1
        assert on_play[0].max_count_per_turn == 1, "On Play should be once per turn"
        assert when_digi[0].max_count_per_turn == 1, "When Digivolving should be once per turn"
        assert on_atk[0].max_count_per_turn == 1, "When Attacking should be once per turn"

        # They should share the same hash so the once-per-turn counter is shared
        hashes = {on_play[0].hash_string, when_digi[0].hash_string, on_atk[0].hash_string}
        assert len(hashes) == 1, \
            f"All three effects should share the same hash_string for shared once-per-turn, got {hashes}"

        # Shared state: all three should be blocked after one fires.
        # First verify all three pass their can_use_condition initially
        ctx = {'game': game, 'player': game.player1, 'permanent': perm}
        assert on_play[0].can_use_condition(ctx)
        assert when_digi[0].can_use_condition(ctx)
        assert on_atk[0].can_use_condition(ctx)

        # Run the On Play process (which consumes the shared counter)
        original_own = game.effect_select_own_permanent
        original_opp = game.effect_select_opponent_permanent

        def mock_own(player, callback, filter_fn=None, is_optional=False, prompt=""):
            callback(opt_perm)

        def mock_opp(player, callback, filter_fn=None, is_optional=False):
            pass  # No opponent digimon; just don't target

        game.effect_select_own_permanent = mock_own
        game.effect_select_opponent_permanent = mock_opp

        on_play[0].on_process_callback(ctx)

        game.effect_select_own_permanent = original_own
        game.effect_select_opponent_permanent = original_opp

        # After one activation, all three should now be blocked by shared counter
        assert not on_play[0].can_use_condition(ctx), \
            "On Play should be blocked after activation (shared once-per-turn)"
        assert not when_digi[0].can_use_condition(ctx), \
            "When Digivolving should be blocked after On Play fires (shared once-per-turn)"
        assert not on_atk[0].can_use_condition(ctx), \
            "When Attacking should be blocked after On Play fires (shared once-per-turn)"

    # -- [All Turns] Unsuspend when Option trashed --

    def test_unsuspend_effect_fires_when_option_trashed(self, debug_runner):
        """When an Option is trashed from battle area (as part of the cost
        payment), Justimon should unsuspend and gain effect immunity."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        # Place Justimon suspended
        perm = runner.place_on_field(1, ["BT23-059"], is_suspended=True)
        assert perm.is_suspended, "Setup: Justimon should start suspended"

        # Place an Option
        opt_perm = runner.place_on_field(1, ["BT22-099"], turn_played=-2)

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        # Find the On Play effect
        on_play = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)][0]

        # Mock selections
        original_own = game.effect_select_own_permanent
        original_opp = game.effect_select_opponent_permanent

        def mock_own(player, callback, filter_fn=None, is_optional=False, prompt=""):
            callback(opt_perm)

        def mock_opp(player, callback, filter_fn=None, is_optional=False):
            pass  # No target

        game.effect_select_own_permanent = mock_own
        game.effect_select_opponent_permanent = mock_opp

        # Fire the On Play process — this trashes the option and should
        # cascade into the unsuspend trigger
        on_play.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        game.effect_select_own_permanent = original_own
        game.effect_select_opponent_permanent = original_opp

        # Justimon should now be unsuspended
        assert not perm.is_suspended, \
            "Justimon should be unsuspended after Option was trashed"

    def test_unsuspend_effect_once_per_turn(self, debug_runner):
        """The unsuspend trigger is [Once Per Turn]."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT23-059"])
        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)

        unsuspend_effects = [e for e in effects
                             if 'Unsuspend' in (e.effect_name or '')
                             and e.max_count_per_turn == 1]
        assert len(unsuspend_effects) >= 1, \
            "Should have unsuspend effect with once-per-turn"

    def test_trash_option_then_delete_lowest_cost(self, debug_runner):
        """Process: trash 1 Option from own battle area, then delete opponent's
        lowest play cost Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        game = runner.game

        # Place Justimon on field
        perm = runner.place_on_field(1, ["BT23-059"])

        # Place an Option on player1's battle area
        opt_perm = runner.place_on_field(1, ["BT22-099"], turn_played=-2)  # CS Option

        # Place opponent Digimon with different play costs
        opp_cheap = runner.place_on_field(2, ["ST5-05"])  # Commandramon, cost 3
        opp_expensive = runner.place_on_field(2, ["BT2-056"])  # Numemon, cost 8(?)

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.timing == EffectTiming.OnEnterFieldAnyone
                   and getattr(e, 'is_on_play', False)][0]

        # Track calls
        select_own_calls = []
        select_opp_calls = []
        original_own = game.effect_select_own_permanent
        original_opp = game.effect_select_opponent_permanent

        def mock_own(player, callback, filter_fn=None, is_optional=False, prompt=""):
            select_own_calls.append({'filter_fn': filter_fn})
            # Auto-select the Option
            callback(opt_perm)

        def mock_opp(player, callback, filter_fn=None, is_optional=False):
            select_opp_calls.append({'filter_fn': filter_fn})

        game.effect_select_own_permanent = mock_own
        game.effect_select_opponent_permanent = mock_opp

        on_play.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        # Should have selected an option to trash
        assert len(select_own_calls) == 1
        own_filter = select_own_calls[0]['filter_fn']
        assert own_filter(opt_perm), "Filter should accept Option permanents"
        assert not own_filter(perm), "Filter should reject non-Option permanents"

        # Option should have been removed from battle area and cards moved to trash
        assert opt_perm not in game.player1.battle_area, \
            "Option should be removed from battle area"

        # Should then try to select opponent's lowest cost Digimon
        assert len(select_opp_calls) == 1

        game.effect_select_own_permanent = original_own
        game.effect_select_opponent_permanent = original_opp
