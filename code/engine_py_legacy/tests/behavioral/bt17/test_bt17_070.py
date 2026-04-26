"""Behavioral tests for BT17-070 Gulfmon.

Card text:
  Lv.6 | Purple | Dark Animal, Virus | DP:11000 | Cost:12

  [On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters]
    in its text from your hand or trash as this Digimon's bottom digivolution
    card, delete 1 of your opponent's level 5 or lower Digimon.
  [When Attacking] By returning 7 cards from your opponent's trash to the
    bottom of the deck, unsuspend this Digimon.

  Alt-Digi: From Lv.5 w/[Dark Masters] in text: Cost 3

Key faithfulness points tested:
  - Effect 0/1: On Play / When Digivolving shared behavior
    - Optional effect ("By placing...") -- activating the cost (placing) is the
      trigger; if you choose to activate, both the placement and deletion happen
    - Player CHOOSES which zone (hand or trash) via effect_choose_branch
    - Player CHOOSES which qualifying card via selection phase (hand or trash)
    - After placing, player selects opponent's Lv.5-or-lower Digimon to delete
    - No auto-selection of placement card (RL must choose)
    - Condition: must have qualifying card in hand or trash AND opponent must
      have valid delete target (per C# -- deletion only fires if target exists)
  - Effect 2: When Attacking
    - Optional: "By returning 7 cards..." is a cost
    - Condition: opponent must have >= 7 cards in trash
    - After returning 7 cards to deck bottom, unsuspend this Digimon
    - Only triggers for THIS Digimon attacking
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# Card IDs
BT17_070 = "BT17-070"      # Gulfmon (card under test)
BT17_068 = "BT17-068"      # Mephistomon Lv.5 (has "Dark Masters" in text)
BT15_077 = "BT15-077"      # LadyDevimon Lv.5 (has "Dark Masters" in text)
ST1_03 = "ST1-03"           # Agumon Lv.3 (filler)
ST1_07 = "ST1-07"           # Greymon Lv.4 (opponent target)
ST1_09 = "ST1-09"           # MetalGreymon Lv.5 (opponent target)
BT1_025 = "BT1-025"        # WarGreymon Lv.6 (NOT level 5 or lower - invalid delete target)

FILLER_DECK = (
    ["BT3-001"] * 4
    + [ST1_03] * 10
    + [ST1_07] * 10
    + [ST1_09] * 10
    + [BT1_025] * 8
    + ["ST1-11"] * 4
    + ["ST1-02"] * 4
)


@pytest.mark.behavioral
class TestBT17070OnPlayWhenDigivolving:
    """[On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters]
    in its text from your hand or trash as this Digimon's bottom digivolution
    card, delete 1 of your opponent's level 5 or lower Digimon."""

    def test_on_play_effect_is_optional(self, debug_runner):
        """The effect should be optional (player can decline to place)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play]
        assert len(on_play) >= 1, "Should have On Play effect"
        assert on_play[0].is_optional, "On Play effect should be optional"

    def test_when_digivolving_effect_is_optional(self, debug_runner):
        """The When Digivolving effect should be optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have When Digivolving effect"
        assert when_digi[0].is_optional, "When Digivolving effect should be optional"

    def test_condition_fails_without_qualifying_cards(self, debug_runner):
        """Condition should fail when no Lv.5 Dark Masters text cards in hand or trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        # Place an opponent target so we isolate the qualifying-card condition
        runner.place_on_field(2, [ST1_03])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        # No Dark Masters text Lv.5 cards anywhere
        ctx = {}
        assert not on_play.can_use_condition(ctx), (
            "Condition should fail without qualifying Lv.5 Dark Masters text cards")

    def test_condition_passes_with_qualifying_card_in_hand(self, debug_runner):
        """Condition should pass when a qualifying Lv.5 card is in hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent target
        runner.inject_card(1, BT17_068, "hand")  # Mephistomon Lv.5 with Dark Masters text

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        ctx = {}
        assert on_play.can_use_condition(ctx), (
            "Condition should pass with qualifying card in hand")

    def test_condition_passes_with_qualifying_card_in_trash(self, debug_runner):
        """Condition should pass when a qualifying Lv.5 card is in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent target
        runner.inject_card(1, BT17_068, "trash")  # Mephistomon Lv.5 in trash

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        ctx = {}
        assert on_play.can_use_condition(ctx), (
            "Condition should pass with qualifying card in trash")

    def test_zone_selection_uses_choose_branch(self, debug_runner):
        """When both hand and trash have qualifying cards, effect should present
        zone choice via effect_choose_branch (not auto-select)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent Lv.3 target
        runner.inject_card(1, BT17_068, "hand")   # qualifying in hand
        runner.inject_card(1, BT15_077, "trash")  # qualifying in trash

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        branch_calls = []
        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card

        def mock_choose(player, num, callback, branch_labels=None, **kwargs):
            branch_calls.append({
                'num_choices': num,
                'labels': branch_labels,
            })
            callback(0)  # pick hand

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            # Auto-pick first qualifying card
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand

        assert len(branch_calls) >= 1, (
            "Should call effect_choose_branch for zone selection when both zones have cards")
        assert branch_calls[0]['num_choices'] == 2, "Should have 2 choices (hand vs trash)"

    def test_hand_selection_uses_selection_phase(self, debug_runner):
        """When choosing from hand, effect should use effect_select_hand_card
        (not auto-select the first card)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent target
        runner.inject_card(1, BT17_068, "hand")   # qualifying in hand only

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        hand_select_calls = []
        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(0)  # hand

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            hand_select_calls.append(True)
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        assert len(hand_select_calls) >= 1, (
            "Should call effect_select_hand_card for card selection (not auto-select)")

    def test_trash_selection_uses_selection_phase(self, debug_runner):
        """When choosing from trash, effect should use request_selection with
        SelectTrash phase (not auto-select)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent target
        runner.inject_card(1, BT17_068, "trash")  # qualifying in trash only

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        trash_select_calls = []
        original_choose = runner.game.effect_choose_branch
        original_request = runner.game.request_selection
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(1)  # trash

        def mock_request(phase, player, callback, valid, **kwargs):
            if phase == GamePhase.SelectTrash:
                trash_select_calls.append({'valid': valid})
                if valid:
                    callback(valid[0])
            else:
                original_request(phase, player, callback, valid, **kwargs)

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.request_selection = mock_request
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.request_selection = original_request
            runner.game.effect_select_opponent_permanent = original_select_opp

        assert len(trash_select_calls) >= 1, (
            "Should use request_selection with SelectTrash phase for trash card selection")

    def test_places_card_as_bottom_digivolution(self, debug_runner):
        """Selected card should be placed as the bottom digivolution card."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])  # opponent target
        runner.inject_card(1, BT17_068, "hand")

        stack_before = len(perm.card_sources)

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(0)  # hand

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        # Stack should have grown by 1 (card placed at bottom)
        assert len(perm.card_sources) == stack_before + 1, (
            f"Stack should grow by 1 after placing card. Before: {stack_before}, "
            f"After: {len(perm.card_sources)}")
        # The bottom card should be the placed card
        bottom_id = perm.card_sources[0].c_entity_base.card_id
        assert bottom_id == BT17_068, (
            f"Bottom card should be the placed BT17-068. Got {bottom_id}")

    def test_card_removed_from_hand_after_placing(self, debug_runner):
        """Card placed from hand should no longer be in hand."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])
        runner.inject_card(1, BT17_068, "hand")

        hand_ids_before = [c.c_entity_base.card_id for c in runner.game.player1.hand_cards]
        assert BT17_068 in hand_ids_before, "Setup: BT17-068 should be in hand"

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(0)

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        hand_ids_after = [c.c_entity_base.card_id for c in runner.game.player1.hand_cards]
        assert BT17_068 not in hand_ids_after, (
            f"BT17-068 should be removed from hand after placing. Hand: {hand_ids_after}")

    def test_card_removed_from_trash_after_placing(self, debug_runner):
        """Card placed from trash should no longer be in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])
        runner.inject_card(1, BT17_068, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        original_choose = runner.game.effect_choose_branch
        original_request = runner.game.request_selection
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(1)  # trash

        def mock_request(phase, player, callback, valid, **kwargs):
            if phase == GamePhase.SelectTrash:
                if valid:
                    callback(valid[0])
            else:
                original_request(phase, player, callback, valid, **kwargs)

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.request_selection = mock_request
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.request_selection = original_request
            runner.game.effect_select_opponent_permanent = original_select_opp

        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert BT17_068 not in trash_ids, (
            f"BT17-068 should be removed from trash after placing. Trash: {trash_ids}")

    def test_deletes_opponent_lv5_or_lower_digimon(self, debug_runner):
        """After placing, should delete 1 of opponent's Lv.5-or-lower Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        opp_target = runner.place_on_field(2, [ST1_09])  # Lv.5 MetalGreymon
        runner.inject_card(1, BT17_068, "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        deleted_perms = []
        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(0)

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, filter_fn=None, **kwargs):
            enemy = player.enemy
            for p in enemy.battle_area:
                if filter_fn is None or filter_fn(p):
                    deleted_perms.append(p)
                    callback(p)
                    return

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        assert len(deleted_perms) >= 1, (
            "Should call effect_select_opponent_permanent to delete opp Digimon")

    def test_delete_filter_rejects_lv6_digimon(self, debug_runner):
        """Delete target filter should reject Lv.6+ Digimon."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        opp_lv6 = runner.place_on_field(2, [BT1_025])  # WarGreymon Lv.6
        runner.inject_card(1, BT17_068, "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        filter_results = []
        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            callback(0)

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, filter_fn=None, **kwargs):
            # Record what the filter says about opp Lv.6
            enemy = player.enemy
            for p in enemy.battle_area:
                filter_results.append({
                    'level': p.level,
                    'accepted': filter_fn(p) if filter_fn else True
                })

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        assert len(filter_results) >= 1, "Should have checked opponent's Digimon"
        lv6_result = next((r for r in filter_results if r['level'] == 6), None)
        assert lv6_result is not None, "Should have checked the Lv.6 Digimon"
        assert not lv6_result['accepted'], (
            f"Delete filter should reject Lv.6 Digimon. Got accepted={lv6_result['accepted']}")

    def test_no_zone_choice_when_only_hand(self, debug_runner):
        """When only hand has qualifying cards, should skip zone choice
        and go directly to hand selection (per C# behavior)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])
        runner.inject_card(1, BT17_068, "hand")  # only in hand, not in trash

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        branch_calls = []
        hand_select_calls = []
        original_choose = runner.game.effect_choose_branch
        original_select_hand = runner.game.effect_select_hand_card
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            branch_calls.append(True)
            callback(0)

        def mock_select_hand(player, filter_fn, callback, **kwargs):
            hand_select_calls.append(True)
            for c in player.hand_cards:
                if filter_fn(c):
                    callback(c)
                    return

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.effect_select_hand_card = mock_select_hand
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.effect_select_hand_card = original_select_hand
            runner.game.effect_select_opponent_permanent = original_select_opp

        # When only one zone has cards, C# skips the zone choice and auto-selects that zone
        assert len(branch_calls) == 0, (
            "Should NOT call effect_choose_branch when only one zone has qualifying cards")
        assert len(hand_select_calls) >= 1, (
            "Should go directly to hand selection when only hand has cards")

    def test_no_zone_choice_when_only_trash(self, debug_runner):
        """When only trash has qualifying cards, should skip zone choice
        and go directly to trash selection."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        runner.place_on_field(2, [ST1_03])
        runner.inject_card(1, BT17_068, "trash")  # only in trash

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if e.is_on_play][0]

        branch_calls = []
        trash_select_calls = []
        original_choose = runner.game.effect_choose_branch
        original_request = runner.game.request_selection
        original_select_opp = runner.game.effect_select_opponent_permanent

        def mock_choose(player, num, callback, **kwargs):
            branch_calls.append(True)
            callback(1)

        def mock_request(phase, player, callback, valid, **kwargs):
            if phase == GamePhase.SelectTrash:
                trash_select_calls.append(True)
                if valid:
                    callback(valid[0])
            else:
                original_request(phase, player, callback, valid, **kwargs)

        def mock_select_opp(player, callback, **kwargs):
            enemy = player.enemy
            if enemy and enemy.battle_area:
                callback(enemy.battle_area[0])

        runner.game.effect_choose_branch = mock_choose
        runner.game.request_selection = mock_request
        runner.game.effect_select_opponent_permanent = mock_select_opp
        try:
            ctx = {'player': runner.game.player1, 'game': runner.game}
            on_play.on_process_callback(ctx)
        finally:
            runner.game.effect_choose_branch = original_choose
            runner.game.request_selection = original_request
            runner.game.effect_select_opponent_permanent = original_select_opp

        assert len(branch_calls) == 0, (
            "Should NOT call effect_choose_branch when only trash has qualifying cards")
        assert len(trash_select_calls) >= 1, (
            "Should go directly to trash selection when only trash has cards")


@pytest.mark.behavioral
class TestBT17070WhenAttacking:
    """[When Attacking] By returning 7 cards from your opponent's trash to the
    bottom of the deck, unsuspend this Digimon."""

    def test_when_attacking_effect_is_optional(self, debug_runner):
        """The When Attacking effect should be optional."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack]
        assert len(on_attack) >= 1, "Should have When Attacking effect"
        assert on_attack[0].is_optional, "When Attacking should be optional"

    def test_condition_requires_7_cards_in_opp_trash(self, debug_runner):
        """Condition should fail when opponent has fewer than 7 cards in trash."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        # Only 6 cards in opponent's trash
        for _ in range(6):
            runner.inject_card(2, ST1_03, "trash")

        ctx = {'attacker': perm, 'permanent': perm}
        assert not on_attack.can_use_condition(ctx), (
            "Should fail with only 6 cards in opponent's trash")

        # Add 1 more to reach 7
        runner.inject_card(2, ST1_03, "trash")
        assert on_attack.can_use_condition(ctx), (
            "Should pass with 7 cards in opponent's trash")

    def test_condition_requires_this_digimon_attacking(self, debug_runner):
        """Should only trigger when THIS Digimon is the attacker."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070])
        other_perm = runner.place_on_field(1, [BT1_025])
        for _ in range(7):
            runner.inject_card(2, ST1_03, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        # Other Digimon attacking
        ctx = {'attacker': other_perm, 'permanent': other_perm}
        assert not on_attack.can_use_condition(ctx), (
            "Should NOT trigger for a different Digimon attacking")

        # This Digimon attacking
        ctx = {'attacker': perm, 'permanent': perm}
        assert on_attack.can_use_condition(ctx), (
            "Should trigger for this Digimon attacking")

    def test_returns_7_cards_to_deck_bottom(self, debug_runner):
        """Should return exactly 7 cards from opponent's trash to deck bottom."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070], is_suspended=True)

        # Put 10 cards in opponent trash
        for _ in range(10):
            runner.inject_card(2, ST1_03, "trash")

        p2_trash_before = len(runner.game.player2.trash_cards)
        p2_library_before = len(runner.game.player2.library_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_attack.on_process_callback(ctx)

        p2_trash_after = len(runner.game.player2.trash_cards)
        p2_library_after = len(runner.game.player2.library_cards)

        assert p2_trash_after == p2_trash_before - 7, (
            f"Opponent's trash should have 7 fewer cards. "
            f"Before: {p2_trash_before}, After: {p2_trash_after}")
        assert p2_library_after == p2_library_before + 7, (
            f"Opponent's library should have 7 more cards. "
            f"Before: {p2_library_before}, After: {p2_library_after}")

    def test_unsuspends_self_after_returning(self, debug_runner):
        """Should unsuspend this Digimon after returning 7 cards."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070], is_suspended=True)
        assert perm.is_suspended, "Should start suspended"

        for _ in range(7):
            runner.inject_card(2, ST1_03, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_attack.on_process_callback(ctx)

        assert not perm.is_suspended, (
            "This Digimon should be unsuspended after returning 7 cards")

    def test_no_unsuspend_without_7_cards(self, debug_runner):
        """Should NOT unsuspend if process fires but fewer than 7 cards available."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=10)
        perm = runner.place_on_field(1, [BT17_070], is_suspended=True)

        # Only 5 cards in trash
        for _ in range(5):
            runner.inject_card(2, ST1_03, "trash")

        card = perm.top_card
        effects = card.effect_list(None)
        on_attack = [e for e in effects if e.is_on_attack][0]

        ctx = {'player': runner.game.player1, 'game': runner.game}
        on_attack.on_process_callback(ctx)

        assert perm.is_suspended, (
            "Should remain suspended when fewer than 7 cards in opp trash")
