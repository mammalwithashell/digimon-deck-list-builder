"""Behavioral tests for BT22-026 MetalGarurumon (Lv.6 Blue, Cost 12, Cyborg/CS).

Card text (from cards.json):
  Alt-digi: Lv.5 w/[Garurumon] in name or w/[CS] trait: Cost 3
  [Hand] [Main] If you have [Nokia Shiramine], 1 of your [Gabumon] digivolves
      into this card for a digivolution cost of 6, ignoring digivolution
      requirements.
  [When Digivolving] Activate 1 of the effects below:
      - 1 of your [Agumon] may digivolve into [WarGreymon] in the hand,
        ignoring digivolution requirements and without paying the cost.
      - Return 1 of your opponent's Digimon with the lowest level to the hand.
  Inherited: [When Attacking] [Once Per Turn] If this Digimon has [Omnimon]
      in its name, it unsuspends.

C# reference: EqualsCardName("Gabumon"), EqualsCardName("Nokia Shiramine"),
              EqualsCardName("Agumon"), EqualsCardName("WarGreymon"),
              ContainsCardName("Omnimon"), IsMinLevel
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Card IDs used in tests:
# BT22-026: MetalGarurumon (this card)
# BT1-010: Agumon (Lv.3)
# BT1-029: Gabumon (Lv.3)
# BT1-025: WarGreymon (Lv.6)
# BT5-092: Nokia Shiramine (Tamer)
# BT1-084: Omnimon (Lv.7)
# BT1-040: WereGarurumon (Lv.5, has "Garurumon" in name)
# BT22-023: AeroVeedramon (Lv.5, traits: [Mythical Dragon, CS])
# BT22-024: MarineBullmon (Lv.5, traits: [Mollusk, LIBERATOR]) - no Garurumon, no CS
# BT1-021: MetalGreymon (Lv.5) - has Greymon name, no CS trait

FILLER_DECK = ["BT1-029"] * 50


@pytest.mark.behavioral
class TestBT22026AltDigi:
    """Alt-digi: Lv.5 w/[Garurumon] in name or w/[CS] trait: Cost 3."""

    def test_alt_digi_garurumon_name_matches(self, debug_runner):
        """Lv.5 with Garurumon in name should be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # BT1-040 WereGarurumon is Lv.5 with "Garurumon" in name
        perm = runner.place_on_field(1, ["BT1-040"])
        runner.inject_card(1, "BT22-026", "hand")

        from digimon_gym.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert can_digivolve(card, perm), (
            "WereGarurumon (Lv.5, has Garurumon in name) should be a valid alt-digi target"
        )

    def test_alt_digi_cs_trait_matches(self, debug_runner):
        """Lv.5 with CS trait (no Garurumon name) should be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # BT22-023 AeroVeedramon: Lv.5, CS trait, no Garurumon name
        perm = runner.place_on_field(1, ["BT22-023"])
        runner.inject_card(1, "BT22-026", "hand")

        from digimon_gym.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert can_digivolve(card, perm), (
            "AeroVeedramon (Lv.5, CS trait) should be a valid alt-digi target"
        )

    def test_alt_digi_no_garurumon_no_cs_rejects(self, debug_runner):
        """Lv.5 without Garurumon name or CS trait should NOT be valid alt-digi target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # BT1-021 MetalGreymon: Lv.5, "Greymon" in name, no CS trait
        perm = runner.place_on_field(1, ["BT1-021"])
        runner.inject_card(1, "BT22-026", "hand")

        from digimon_gym.engine.validation.digivolve_validator import can_digivolve
        card = runner.game.player1.hand_cards[-1]
        assert not can_digivolve(card, perm), (
            "MetalGreymon (Lv.5, Greymon name, no CS) should NOT be a valid alt-digi target for BT22-026"
        )

    def test_alt_digi_cost_is_3(self, debug_runner):
        """Alt-digi cost should be 3."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-026", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        alt_digi = [e for e in effects if hasattr(e, '_alt_digi_cost') and e._alt_digi_cost is not None]
        costs = set(e._alt_digi_cost for e in alt_digi)
        assert 3 in costs, f"Alt-digi cost should include 3, got {costs}"


@pytest.mark.behavioral
class TestBT22026HandMain:
    """[Hand][Main] Nokia warp digivolve Gabumon into this for cost 6."""

    def test_hand_main_requires_nokia(self, debug_runner):
        """Without Nokia Shiramine, hand main should not be available."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT1-029"])  # Gabumon
        runner.inject_card(1, "BT22-026", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        assert len(hand_main) == 1

        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not hand_main[0].can_use_condition(ctx), (
            "Hand main should not be usable without Nokia Shiramine"
        )

    def test_hand_main_requires_gabumon(self, debug_runner):
        """With Nokia but no Gabumon, hand main should not be available."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-092"])  # Nokia only
        runner.inject_card(1, "BT22-026", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert not hand_main[0].can_use_condition(ctx), (
            "Hand main should not be usable without Gabumon on field"
        )

    def test_hand_main_with_nokia_and_gabumon(self, debug_runner):
        """With Nokia and Gabumon, hand main should be usable."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-092"])  # Nokia Shiramine
        runner.place_on_field(1, ["BT1-029"])  # Gabumon
        runner.inject_card(1, "BT22-026", "hand")
        runner.set_phase("Main")

        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        ctx = {'game': runner.game, 'player': runner.game.player1}
        assert hand_main[0].can_use_condition(ctx), (
            "Hand main should be usable with Nokia and Gabumon on field"
        )

    def test_hand_main_not_field_main(self, debug_runner):
        """The effect should NOT have _is_field_main (it is a hand-only effect)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-026", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        hand_main = [e for e in effects if getattr(e, '_is_hand_main', False)]
        assert len(hand_main) == 1
        assert not getattr(hand_main[0], '_is_field_main', False), (
            "Hand main effect should NOT also be a field main effect"
        )


@pytest.mark.behavioral
class TestBT22026WhenDigivolving:
    """[When Digivolving] branch choice: Agumon->WarGreymon OR bounce lowest level."""

    def test_wd_branch_0_agumon_digivolve(self, debug_runner):
        """Branch 0: digivolve Agumon into WarGreymon from hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-026"])
        agumon_perm = runner.place_on_field(1, ["BT1-010"])
        runner.inject_card(1, "BT1-025", "hand")  # WarGreymon in hand

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]

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

    def test_wd_branch_1_bounce_lowest_level(self, debug_runner):
        """Branch 1: return opponent Digimon with lowest level to hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-026"])
        # Opponent: BT1-010 Agumon (Lv.3) and BT1-021 MetalGreymon (Lv.5)
        runner.place_on_field(2, ["BT1-010"])  # Lv.3
        runner.place_on_field(2, ["BT1-021"])  # Lv.5

        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects if e.is_when_digivolving][0]

        selected_perms = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

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

        assert len(selected_perms) == 1, "Should have selected 1 target"
        assert selected_perms[0].level == 3, (
            f"Should target lowest level (3), got {selected_perms[0].level}"
        )


@pytest.mark.behavioral
class TestBT22026Inherited:
    """Inherited: [When Attacking][OPT] Unsuspend if Omnimon name."""

    def test_inherited_unsuspend_with_omnimon(self, debug_runner):
        """Under Omnimon, attacking should unsuspend this Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Stack: BT22-026 under BT1-084 Omnimon, suspended (as if attacking)
        perm = runner.place_on_field(1, ["BT22-026", "BT1-084"], is_suspended=True)

        assert perm.is_suspended, "Should start suspended"

        card = perm.card_sources[0]  # BT22-026
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]

        assert inh.can_use_condition({}), "Condition should pass when under Omnimon"

        inh.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        assert not perm.is_suspended, "Omnimon should unsuspend after attacking"

    def test_inherited_no_unsuspend_without_omnimon(self, debug_runner):
        """Under a non-Omnimon Digimon, condition should fail."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-026", "BT1-025"], is_suspended=True)

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]

        assert not inh.can_use_condition({}), (
            "Condition should fail when top card is WarGreymon (not Omnimon)"
        )

    def test_inherited_is_once_per_turn(self, debug_runner):
        """Effect should be Once Per Turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.inject_card(1, "BT22-026", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]
        assert inh.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_inherited_unsuspend_noop_when_not_suspended(self, debug_runner):
        """If already unsuspended, effect should be harmless."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT22-026", "BT1-084"], is_suspended=False)

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        inh = [e for e in effects
               if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack][0]

        # Should not crash
        inh.on_process_callback({
            'player': runner.game.player1,
            'game': runner.game,
            'permanent': perm,
        })

        assert not perm.is_suspended, "Should remain unsuspended"
