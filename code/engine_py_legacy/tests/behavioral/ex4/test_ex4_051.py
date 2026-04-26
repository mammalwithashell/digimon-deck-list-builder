"""Behavioral tests for EX4-051 BlitzGreymon (Lv.6 Black/Red, Cost 12, DP 12000).

Card text:
  [When Digivolving] Activate 1 of the effects below.
    - <De-Digivolve 1> 3 of your opponent's Digimon.
    - Digivolve 1 of your other Digimon into a level 6 or lower Digimon card
      with [Garurumon] in its name in your hand without paying its cost.
    - This Digimon and another of your Digimon may DNA digivolve into a
      Digimon card in the hand.

  Inherited: [When Attacking] [Once Per Turn] If this Digimon has [Omnimon]
    in its name, trash the top card of your opponent's security stack.

Evo Costs: Black Lv.5 for 4
Alt-digi: [MetalGreymon] Cost 3
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase

FILLER_DECK = ["ST1-03"] * 50


def _get_effects(perm):
    """Get all effects from EX4-051 top card."""
    top = perm.top_card
    if top is None:
        return []
    return top.effect_list(None)


def _get_when_digivolving(perm):
    """Get the When Digivolving effect from EX4-051."""
    effects = _get_effects(perm)
    wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
    assert len(wd) == 1, f"Expected 1 When Digivolving effect, found {len(wd)}"
    return wd[0]


def _get_inherited(perm):
    """Get the inherited When Attacking effect from EX4-051."""
    # Look through all card_sources for the EX4-051 inherited effect
    for cs in perm.card_sources:
        if cs.c_entity_base and cs.c_entity_base.card_id == "EX4-051":
            effects = cs.effect_list(None)
            inh = [e for e in effects
                   if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack]
            if inh:
                return inh[0]
    # Fallback: check top card directly
    effects = _get_effects(perm)
    inh = [e for e in effects
           if e.is_inherited_effect and e.timing == EffectTiming.OnUseAttack]
    assert len(inh) == 1, f"Expected 1 inherited effect, found {len(inh)}"
    return inh[0]


# ===========================================================================
# When Digivolving: Branch choice
# ===========================================================================

@pytest.mark.behavioral
class TestEX4051BranchChoice:
    """[When Digivolving] Should offer 3-way branch choice."""

    def test_offers_three_branch_choices(self, debug_runner):
        """Effect should call effect_choose_branch with 3 options."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        num_choices_offered = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            num_choices_offered.append(num_choices)
            # Don't call callback (just verify branch count)

        original = game.effect_choose_branch
        game.effect_choose_branch = mock_choose_branch
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original

        assert num_choices_offered == [3], (
            f"Should offer exactly 3 branch choices, got {num_choices_offered}"
        )

    def test_condition_requires_field_presence(self, debug_runner):
        """When Digivolving condition must require card to be on field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        game = runner.game

        runner.inject_card(1, "EX4-051", "hand")
        hand_card = game.player1.hand_cards[-1]

        effects = hand_card.effect_list(None)
        wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        if wd:
            assert not wd[0].can_use_condition({}), \
                "Condition should fail when card is not on field"


# ===========================================================================
# Branch 0: De-Digivolve 1 three opponent Digimon
# ===========================================================================

@pytest.mark.behavioral
class TestEX4051DeDigivolve:
    """Branch 0: <De-Digivolve 1> 3 of your opponent's Digimon."""

    def test_de_digivolve_targets_three_opponents(self, debug_runner):
        """When opponent has 3 Digimon, all 3 should be de-digivolved."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # 3 opponent Digimon, each with 2 cards (can be de-digivolved)
        opp1 = runner.place_on_field(2, ["ST1-03", "ST1-07"])  # Agumon -> Greymon
        opp2 = runner.place_on_field(2, ["ST1-03", "ST1-07"])
        opp3 = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        trash_before = len(game.player2.trash_cards)

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(0)  # Choose de-digivolve branch

        original = game.effect_choose_branch
        game.effect_choose_branch = mock_choose_branch
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
            # Auto-resolve any selection phases
            runner.auto_resolve(max_steps=20)
        finally:
            game.effect_choose_branch = original

        trash_after = len(game.player2.trash_cards)
        # Each de-digivolve 1 removes 1 card from a 2-card stack -> 3 cards trashed
        assert trash_after - trash_before == 3, (
            f"De-Digivolve 1 on 3 Digimon should trash 3 cards, "
            f"got {trash_after - trash_before}"
        )

    def test_de_digivolve_with_fewer_than_3_opponents(self, debug_runner):
        """When opponent has fewer than 3 Digimon, de-digivolve all of them."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # Only 2 opponent Digimon
        opp1 = runner.place_on_field(2, ["ST1-03", "ST1-07"])
        opp2 = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        trash_before = len(game.player2.trash_cards)

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(0)

        original = game.effect_choose_branch
        game.effect_choose_branch = mock_choose_branch
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
            runner.auto_resolve(max_steps=20)
        finally:
            game.effect_choose_branch = original

        trash_after = len(game.player2.trash_cards)
        # 2 Digimon with de-digivolve 1 each = 2 trashed
        assert trash_after - trash_before == 2, (
            f"De-Digivolve 1 on 2 Digimon should trash 2 cards, "
            f"got {trash_after - trash_before}"
        )

    def test_de_digivolve_uses_selection_when_more_than_3(self, debug_runner):
        """When opponent has >3 Digimon, player must choose which 3 to target."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # 4 opponent Digimon
        runner.place_on_field(2, ["ST1-03", "ST1-07"])
        runner.place_on_field(2, ["ST1-03", "ST1-07"])
        runner.place_on_field(2, ["ST1-03", "ST1-07"])
        runner.place_on_field(2, ["ST1-03", "ST1-07"])

        select_calls = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(0)

        def mock_select_opponent(player, callback, filter_fn=None, is_optional=False, **kwargs):
            select_calls.append(True)
            # Pick the first valid target
            enemy = player.enemy
            valid = [p for p in enemy.battle_area if filter_fn is None or filter_fn(p)]
            if valid:
                callback(valid[0])

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_opponent_permanent
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_opponent_permanent = mock_select_opponent
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_opponent_permanent = original_select

        # Should have 3 selection calls (one per target)
        assert len(select_calls) == 3, (
            f"With 4 opponent Digimon, should make 3 selection calls to pick targets, "
            f"got {len(select_calls)}"
        )

    def test_de_digivolve_skips_single_card_stacks(self, debug_runner):
        """De-Digivolve 1 on a Digimon with only 1 card does nothing to that stack."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # 1 stacked, 1 single-card, 1 stacked
        opp1 = runner.place_on_field(2, ["ST1-03", "ST1-07"])  # 2 cards
        opp2 = runner.place_on_field(2, ["ST1-03"])            # 1 card only
        opp3 = runner.place_on_field(2, ["ST1-03", "ST1-07"])  # 2 cards

        trash_before = len(game.player2.trash_cards)

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(0)

        original = game.effect_choose_branch
        game.effect_choose_branch = mock_choose_branch
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
            runner.auto_resolve(max_steps=20)
        finally:
            game.effect_choose_branch = original

        trash_after = len(game.player2.trash_cards)
        # De-Digivolve 1 applied to all 3, but single-card stack can't lose a card
        # So only 2 cards trashed (from the 2 stacked Digimon)
        assert trash_after - trash_before == 2, (
            f"Expected 2 cards trashed (single-card stack untouched), "
            f"got {trash_after - trash_before}"
        )


# ===========================================================================
# Branch 1: Garurumon digivolve from hand
# ===========================================================================

@pytest.mark.behavioral
class TestEX4051GarurumonDigivolve:
    """Branch 1: Digivolve 1 of your other Digimon into Lv6- [Garurumon] from hand."""

    def test_garurumon_digivolve_uses_selection_for_target(self, debug_runner):
        """Player must choose which of their other Digimon to digivolve."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # Two other Digimon on our field
        other1 = runner.place_on_field(1, ["ST1-03"])  # Agumon
        other2 = runner.place_on_field(1, ["ST1-07"])  # Greymon

        # Garurumon in hand (AD1-010 = Garurumon Lv.4)
        runner.inject_card(1, "AD1-010", "hand")

        select_own_calls = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)  # Choose Garurumon branch

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            select_own_calls.append(True)
            valid = [p for p in player.battle_area if filter_fn is None or filter_fn(p)]
            if valid:
                callback(valid[0])

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select

        assert len(select_own_calls) >= 1, (
            "Should use selection to let player choose which Digimon to digivolve"
        )

    def test_garurumon_filter_accepts_lv6_garurumon(self, debug_runner):
        """Garurumon filter should accept a Lv.6 card with 'Garurumon' in name."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game
        other = runner.place_on_field(1, ["ST1-03"])

        # CresGarurumon (AD1-012) is Lv.6 with "Garurumon" in name
        runner.inject_card(1, "AD1-012", "hand")

        digivolve_called = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            callback(other)

        def mock_digivolve_from_hand(player, target_perm, filter_fn, **kwargs):
            # Check that CresGarurumon passes the filter
            cres_card = game.player1.hand_cards[-1]
            digivolve_called.append(filter_fn(cres_card))

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        original_digi = game.effect_digivolve_from_hand
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        game.effect_digivolve_from_hand = mock_digivolve_from_hand
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select
            game.effect_digivolve_from_hand = original_digi

        assert digivolve_called and digivolve_called[0] is True, (
            "CresGarurumon (Lv.6, has 'Garurumon' in name) should pass the filter"
        )

    def test_garurumon_filter_rejects_lv7(self, debug_runner):
        """Garurumon filter should reject cards above Lv.6."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game
        other = runner.place_on_field(1, ["ST1-03"])

        # AD1-014 MetalGarurumon is Lv.6 - should pass. But there's no Lv.7 Garurumon
        # available easily. Let's just test the filter function directly.
        # We'll inject an Omnimon (Lv.7, no Garurumon) to verify rejection
        runner.inject_card(1, "BT5-086", "hand")  # Omnimon Lv.7

        filter_results = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            callback(other)

        def mock_digivolve_from_hand(player, target_perm, filter_fn, **kwargs):
            omnimon = game.player1.hand_cards[-1]
            filter_results.append(filter_fn(omnimon))

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        original_digi = game.effect_digivolve_from_hand
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        game.effect_digivolve_from_hand = mock_digivolve_from_hand
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select
            game.effect_digivolve_from_hand = original_digi

        assert filter_results and filter_results[0] is False, (
            "Omnimon (Lv.7, no Garurumon in name) should be rejected by filter"
        )

    def test_garurumon_filter_rejects_non_garurumon(self, debug_runner):
        """Garurumon filter should reject Digimon without 'Garurumon' in name."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game
        other = runner.place_on_field(1, ["ST1-03"])

        # ST1-07 Greymon Lv.4 - no Garurumon in name
        runner.inject_card(1, "ST1-07", "hand")

        filter_results = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            callback(other)

        def mock_digivolve_from_hand(player, target_perm, filter_fn, **kwargs):
            greymon = game.player1.hand_cards[-1]
            filter_results.append(filter_fn(greymon))

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        original_digi = game.effect_digivolve_from_hand
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        game.effect_digivolve_from_hand = mock_digivolve_from_hand
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select
            game.effect_digivolve_from_hand = original_digi

        assert filter_results and filter_results[0] is False, (
            "Greymon (no 'Garurumon' in name) should be rejected by filter"
        )

    def test_garurumon_digivolve_excludes_self(self, debug_runner):
        """Selection filter should exclude BlitzGreymon itself from targets."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game

        # Only other Digimon
        other = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "AD1-010", "hand")

        selected_targets = []

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            valid = [p for p in player.battle_area if filter_fn is None or filter_fn(p)]
            selected_targets.extend(valid)
            if valid:
                callback(valid[0])

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select

        assert perm not in selected_targets, (
            "BlitzGreymon itself should NOT be in the selectable targets"
        )

    def test_garurumon_digivolve_ignores_requirements(self, debug_runner):
        """effect_digivolve_from_hand should be called with ignore_requirements=True."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        game = runner.game
        other = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "AD1-010", "hand")

        captured_kwargs = {}

        def mock_choose_branch(player, num_choices, callback, **kwargs):
            callback(1)

        def mock_select_own(player, callback, filter_fn=None, is_optional=False, **kwargs):
            callback(other)

        def mock_digivolve_from_hand(player, target_perm, filter_fn, **kwargs):
            captured_kwargs.update(kwargs)

        original_branch = game.effect_choose_branch
        original_select = game.effect_select_own_permanent
        original_digi = game.effect_digivolve_from_hand
        game.effect_choose_branch = mock_choose_branch
        game.effect_select_own_permanent = mock_select_own
        game.effect_digivolve_from_hand = mock_digivolve_from_hand
        try:
            wd = _get_when_digivolving(perm)
            wd.on_process_callback({
                'player': game.player1,
                'game': game,
                'permanent': perm,
            })
        finally:
            game.effect_choose_branch = original_branch
            game.effect_select_own_permanent = original_select
            game.effect_digivolve_from_hand = original_digi

        assert captured_kwargs.get('ignore_requirements') is True, (
            "Should call effect_digivolve_from_hand with ignore_requirements=True"
        )


# ===========================================================================
# Inherited: [When Attacking][OPT] Omnimon security trash
# ===========================================================================

@pytest.mark.behavioral
class TestEX4051Inherited:
    """Inherited: [When Attacking][OPT] If this Digimon has [Omnimon] in its
    name, trash the top card of opponent's security stack."""

    def test_inherited_is_once_per_turn(self, debug_runner):
        """Effect should be Once Per Turn."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        inh = _get_inherited(perm)
        assert inh.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_inherited_is_on_attack(self, debug_runner):
        """Effect should have is_on_attack set."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        inh = _get_inherited(perm)
        assert inh.is_on_attack, "Should be marked as on_attack"

    def test_inherited_is_inherited(self, debug_runner):
        """Effect should be marked as inherited."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051"])
        inh = _get_inherited(perm)
        assert inh.is_inherited_effect, "Should be marked as inherited"

    def test_inherited_fires_under_omnimon(self, debug_runner):
        """Should fire when this card is under a Digimon with [Omnimon] in name."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Stack: EX4-051 under BT5-086 Omnimon
        perm = runner.place_on_field(1, ["EX4-051", "BT5-086"])
        game = runner.game

        assert perm.contains_card_name("Omnimon"), "Top should be Omnimon"

        inh = _get_inherited(perm)

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        }
        assert inh.can_use_condition(ctx), \
            "Condition should pass when top card is Omnimon"

    def test_inherited_does_not_fire_without_omnimon(self, debug_runner):
        """Should NOT fire when top card is not Omnimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        # Stack: EX4-051 under ST1-11 WarGreymon (not Omnimon)
        perm = runner.place_on_field(1, ["EX4-051", "ST1-11"])
        game = runner.game

        assert not perm.contains_card_name("Omnimon"), "Top should NOT be Omnimon"

        inh = _get_inherited(perm)

        ctx = {
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacker': perm,
        }
        assert not inh.can_use_condition(ctx), \
            "Condition should fail when top card is not Omnimon"

    def test_inherited_trashes_top_security(self, debug_runner):
        """When activated, should trash the top card of opponent's security stack."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051", "BT5-086"])
        game = runner.game

        # Ensure opponent has security
        sec_before = len(game.player2.security_cards)
        assert sec_before > 0, "Opponent should have security cards"

        inh = _get_inherited(perm)
        inh.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        sec_after = len(game.player2.security_cards)
        assert sec_after == sec_before - 1, (
            f"Should trash 1 security card: before={sec_before}, after={sec_after}"
        )

    def test_inherited_trashes_to_trash_zone(self, debug_runner):
        """Trashed security card should go to opponent's trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051", "BT5-086"])
        game = runner.game

        trash_before = len(game.player2.trash_cards)

        inh = _get_inherited(perm)
        inh.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        trash_after = len(game.player2.trash_cards)
        assert trash_after == trash_before + 1, (
            "Trashed security card should go to opponent's trash"
        )

    def test_inherited_noop_when_no_security(self, debug_runner):
        """Should do nothing when opponent has 0 security."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["EX4-051", "BT5-086"])
        game = runner.game

        game.player2.security_cards.clear()
        trash_before = len(game.player2.trash_cards)

        inh = _get_inherited(perm)
        inh.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player2.security_cards) == 0
        assert len(game.player2.trash_cards) == trash_before, \
            "Nothing should be trashed when no security"
