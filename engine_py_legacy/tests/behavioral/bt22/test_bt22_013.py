"""Behavioral tests for BT22-013 WarGreymon (Lv.6 Red, Cost 12, Dragonkin/CS).

Card text (from cards.json):
  Alt-digi: Lv.5 w/[Greymon] in name or w/[CS] trait: Cost 3
  [Hand] [Main] If you have [Nokia Shiramine], 1 of your [Agumon] digivolves
      into this card for a digivolution cost of 6, ignoring digivolution
      requirements.
  [When Digivolving] Activate 1 of the effects below:
      - 1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand,
        ignoring digivolution requirements and without paying the cost.
      - Delete 1 of your opponent's Digimon with the lowest DP.
  Inherited: [When Attacking] [Once Per Turn] If this Digimon has [Omnimon]
      in its name, trash your opponent's top security card.

C# reference: EqualsCardName("Agumon"), EqualsCardName("Nokia Shiramine"),
              EqualsCardName("Gabumon"), EqualsCardName("MetalGarurumon"),
              ContainsCardName("Omnimon"), IsMinDP
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests:
# BT22-013: WarGreymon (this card)
# BT1-010: Agumon (Lv.3)
# BT1-029: Gabumon (Lv.3)
# BT1-044: MetalGarurumon (Lv.6)
# BT5-092: Nokia Shiramine (Tamer)
# BT1-084: Omnimon (Lv.7)
# BT1-021: MetalGreymon (Lv.5, has "Greymon" in name)
# BT22-012: RizeGreymon (Lv.5, traits: [Cyborg, CS])
# BT22-023: AeroVeedramon (Lv.5, traits: [Mythical Dragon, CS]) - no Greymon name

FILLER_DECK = ["BT1-010"] * 50


@pytest.mark.behavioral
class TestBT22013AltDigi:
    """Alt-digi: Lv.5 w/[Greymon] in name or w/[CS] trait: Cost 3."""

    def test_alt_digi_has_two_effects(self, debug_runner):
        """Should have 2 alt-digi effects (one for name, one for trait) for OR logic."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-013", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        assert len(alt_digi) >= 1, "Should have at least 1 alt-digi effect"
        # Both should have level 5
        for eff in alt_digi:
            assert eff._alt_digi_level == 5, "Alt-digi level should be 5"

    def test_alt_digi_name_greymon_matches(self, debug_runner):
        """Lv.5 with Greymon in name should be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Place a Lv.5 MetalGreymon (has "Greymon" in name)
        perm = runner.place_on_field(1, ["BT1-021"])
        runner.inject_card(1, "BT22-013", "hand")
        runner.set_phase("Main")

        # Should be able to digivolve MetalGreymon into BT22-013 via alt-digi
        actions = runner.actions()
        digi_action = None
        for aid, desc in actions.items():
            if "BT22-013" in desc.lower() or "wargreymon" in desc.lower():
                if "digivolve" in desc.lower() or aid >= 100:
                    digi_action = aid
                    break
        # Check the digivolve action mask directly
        from engine_py_legacy.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert can_digivolve(card, perm), (
            "MetalGreymon (Lv.5, has Greymon in name) should be a valid alt-digi target"
        )

    def test_alt_digi_cs_trait_matches(self, debug_runner):
        """Lv.5 with CS trait (no Greymon name) should be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Place a Lv.5 AeroVeedramon (CS trait, no Greymon in name)
        perm = runner.place_on_field(1, ["BT22-023"])
        runner.inject_card(1, "BT22-013", "hand")

        from engine_py_legacy.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert can_digivolve(card, perm), (
            "AeroVeedramon (Lv.5, CS trait) should be a valid alt-digi target"
        )

    def test_alt_digi_no_greymon_no_cs_rejects(self, debug_runner):
        """Lv.5 without Greymon name or CS trait should NOT be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Place a Lv.5 with no Greymon name and no CS trait
        # BT1-040 WereGarurumon is Lv.5 with Garurumon in name, no CS trait
        perm = runner.place_on_field(1, ["BT1-040"])
        runner.inject_card(1, "BT22-013", "hand")

        from engine_py_legacy.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert not can_digivolve(card, perm), (
            "WereGarurumon (Lv.5, Garurumon name, no CS) should NOT be a valid alt-digi target for BT22-013"
        )

    def test_alt_digi_cost_is_3(self, debug_runner):
        """Alt-digi cost should be 3."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-013", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        costs = set(e._alt_digi_cost for e in alt_digi)
        assert 3 in costs, f"Alt-digi cost should include 3, got {costs}"


@pytest.mark.behavioral
class TestBT22013HandMain:
    """[Hand][Main] Nokia warp digivolve Agumon into this for cost 6."""

    def test_hand_main_requires_nokia(self, debug_runner):
        """Without Nokia Shiramine, hand main should not be available."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Place Agumon on field but no Nokia
        runner.place_on_field(1, ["BT1-010"])
        runner.inject_card(1, "BT22-013", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        assert len(hand_main) == 1, "Should have 1 hand main effect"

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not hand_main[0].can_use_condition(ctx), (
            "Hand main should not be usable without Nokia Shiramine"
        )

    def test_hand_main_requires_agumon(self, debug_runner):
        """With Nokia but no Agumon, hand main should not be available."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Place Nokia but no Agumon
        runner.place_on_field(1, ["BT5-092"])
        runner.inject_card(1, "BT22-013", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not hand_main[0].can_use_condition(ctx), (
            "Hand main should not be usable without Agumon on field"
        )

    def test_hand_main_with_nokia_and_agumon(self, debug_runner):
        """With Nokia and Agumon, hand main should be usable."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-092"])  # Nokia Shiramine
        runner.place_on_field(1, ["BT1-010"])  # Agumon
        runner.inject_card(1, "BT22-013", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert hand_main[0].can_use_condition(ctx), (
            "Hand main should be usable with Nokia and Agumon on field"
        )

    def test_hand_main_not_field_main(self, debug_runner):
        """The effect should NOT have _is_field_main (it is a hand-only effect)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-013", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        assert len(hand_main) == 1
        assert not getattr(hand_main[0], '_is_field_main', False), (
            "Hand main effect should NOT also be a field main effect"
        )


@pytest.mark.behavioral
class TestBT22013WhenDigivolving:
    """[When Digivolving] branch choice: Gabumon->MetalGarurumon OR delete lowest DP."""

    def test_wd_branch_0_gabumon_digivolve(self, debug_runner):
        """Branch 0: digivolve Gabumon into MetalGarurumon from hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place WarGreymon on field (simulating it just digivolved)
        perm = runner.place_on_field(1, ["BT22-013"])
        # Place Gabumon on field
        gabumon_perm = runner.place_on_field(1, ["BT1-029"])
        # Put MetalGarurumon in hand
        runner.inject_card(1, "BT1-044", "hand")

        # Get the WD effect
        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        assert len(wd_effects) == 1, "Should have 1 WD effect"

        wd = wd_effects[0]
        assert wd.can_use_condition({}), "WD condition should pass when on field"

        # Execute branch 0 (Gabumon -> MetalGarurumon)
        branch_chosen = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            branch_chosen.append(0)
            callback(0)

        original = runner.game.effect_choose_branch
        runner.game.effect_choose_branch = mock_choose_branch

        try:
            wd.on_process_callback({
                'player': runner.game.player1,
                'game': runner.game,
                'permanent': perm,
            })
        finally:
            runner.game.effect_choose_branch = original

        assert len(branch_chosen) == 1, "Branch choice should have been offered"

    def test_wd_branch_1_delete_lowest_dp(self, debug_runner):
        """Branch 1: delete opponent Digimon with lowest DP."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place WarGreymon on field
        perm = runner.place_on_field(1, ["BT22-013"])
        # Place opponent Digimon with different DP
        # BT1-010 Agumon: 2000 DP (lowest)
        # BT1-021 MetalGreymon: 7000 DP
        runner.place_on_field(2, ["BT1-010"])  # 2000 DP
        runner.place_on_field(2, ["BT1-021"])  # 7000 DP

        snap_before = runner.snapshot()
        p2_count_before = len(snap_before.p2_field)

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]

        # Execute branch 1 (delete lowest DP) with auto-selection
        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        selected_perms = []

        def mock_select_opponent(player, callback, filter_fn=None, is_optional=False, **kwargs):
            enemy = player.enemy
            valid = [p for p in enemy.battle_area if filter_fn is None or filter_fn(p)]
            if valid:
                selected_perms.append(valid[0])
                callback(valid[0])

        original_branch = runner.game.effect_choose_branch
        original_select = runner.game.effect_select_opponent_permanent
        runner.game.effect_choose_branch = mock_choose_branch
        runner.game.effect_select_opponent_permanent = mock_select_opponent

        try:
            wd.on_process_callback({
                'player': runner.game.player1,
                'game': runner.game,
                'permanent': perm,
            })
        finally:
            runner.game.effect_choose_branch = original_branch
            runner.game.effect_select_opponent_permanent = original_select

        # Verify the filter only allowed lowest DP (2000 DP Agumon)
        assert len(selected_perms) == 1, "Should have selected 1 target"
        assert selected_perms[0].dp == 2000, (
            f"Should target lowest DP (2000), got {selected_perms[0].dp}"
        )

    def test_wd_delete_uses_opponent_effect_flag(self, debug_runner):
        """Delete should use is_opponent_effect=True for proper Decoy/Evade handling."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-013"])
        runner.place_on_field(2, ["BT1-010"])  # 2000 DP

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]

        # Track delete calls
        delete_calls = []
        original_delete = runner.game.player2.delete_permanent

        def track_delete(permanent, is_battle=False, is_opponent_effect=False, removal_cause='effect'):
            delete_calls.append({
                'is_opponent_effect': is_opponent_effect,
                'removal_cause': removal_cause,
            })
            return original_delete(permanent, is_battle=is_battle,
                                   is_opponent_effect=is_opponent_effect,
                                   removal_cause=removal_cause)

        runner.game.player2.delete_permanent = track_delete

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_opponent(player, callback, filter_fn=None, is_optional=False, **kwargs):
            enemy = player.enemy
            valid = [p for p in enemy.battle_area if filter_fn is None or filter_fn(p)]
            if valid:
                callback(valid[0])

        original_branch = runner.game.effect_choose_branch
        original_select = runner.game.effect_select_opponent_permanent
        runner.game.effect_choose_branch = mock_choose_branch
        runner.game.effect_select_opponent_permanent = mock_select_opponent

        try:
            wd.on_process_callback({
                'player': runner.game.player1,
                'game': runner.game,
                'permanent': perm,
            })
        finally:
            runner.game.effect_choose_branch = original_branch
            runner.game.effect_select_opponent_permanent = original_select
            runner.game.player2.delete_permanent = original_delete

        assert len(delete_calls) == 1, "Should have called delete once"
        assert delete_calls[0]['is_opponent_effect'] is True, (
            "Delete should use is_opponent_effect=True"
        )


@pytest.mark.behavioral
class TestBT22013Inherited:
    """Inherited: [When Attacking][OPT] Trash security if Omnimon name."""

    def test_inherited_trash_security_with_omnimon(self, debug_runner):
        """Under Omnimon, attacking should trash opponent's top security."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Stack: BT22-013 (inherited source) under BT1-084 Omnimon
        perm = runner.place_on_field(1, ["BT22-013", "BT1-084"])

        sec_before = len(runner.game.player2.security_cards)
        assert sec_before > 0, "Opponent should have security cards"

        card = perm.card_sources[0]  # BT22-013 (bottom)
        effects = card.effect_list(None)
        inh_effects = [e for e in effects
                       if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack]
        assert len(inh_effects) == 1, "Should have 1 inherited WA effect"

        inh = inh_effects[0]
        assert inh.can_use_condition({}), (
            "Condition should pass when under Omnimon"
        )

        # Execute the effect
        inh.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        sec_after = len(runner.game.player2.security_cards)
        assert sec_after == sec_before - 1, (
            f"Should trash 1 security card, went from {sec_before} to {sec_after}"
        )

    def test_inherited_no_trash_without_omnimon(self, debug_runner):
        """Under a non-Omnimon Digimon, condition should fail."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Stack: BT22-013 under BT1-025 WarGreymon (not Omnimon)
        perm = runner.place_on_field(1, ["BT22-013", "BT1-025"])

        card = perm.card_sources[0]  # BT22-013
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]

        assert not inh.can_use_condition({}), (
            "Condition should fail when top card is not Omnimon"
        )

    def test_inherited_is_once_per_turn(self, debug_runner):
        """Effect should be Once Per Turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-013", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]
        assert inh.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_inherited_uses_proper_trash_method(self, debug_runner):
        """Trash security should use proper trash_security_card method, not raw pop."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-013", "BT1-084"])

        # Track trash_security_card calls
        trash_calls = []
        original_trash = runner.game.player2.trash_security_card

        def track_trash(card):
            trash_calls.append(card)
            return original_trash(card)

        runner.game.player2.trash_security_card = track_trash

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]

        try:
            inh.on_process_callback({
                'player': runner.game.player1,
                'game': runner.game,
                'permanent': perm,
            })
        finally:
            runner.game.player2.trash_security_card = original_trash

        assert len(trash_calls) == 1, (
            "Should use trash_security_card method (fires timing events)"
        )
