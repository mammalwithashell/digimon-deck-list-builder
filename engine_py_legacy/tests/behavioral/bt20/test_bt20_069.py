"""Behavioral tests for BT20-069 Punkmon (Lv.4 Purple/Red Dark Dragon LIBERATOR, DP 5000, Cost 5).

Card text:
[On Play] [When Digivolving] Trash 1 card in your hand. Then, until the end of
your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.
Inherited: [Your Turn] This Digimon gets +2000 DP.

Alt Digivolve: Lv.3 with [Evil] trait for cost 2
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT20069Punkmon:
    """Tests for BT20-069 Punkmon."""

    # ── Helper to get On Play / When Digivolving effects ──────────────

    def _get_on_play_effect(self, perm):
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play]
        assert len(on_play) == 1, "Should have exactly 1 On Play effect"
        return on_play[0]

    def _get_when_digivolving_effect(self, perm):
        card = perm.top_card
        effects = card.effect_list(None)
        wd = [e for e in effects
              if e.timing == EffectTiming.OnEnterFieldAnyone
              and e.is_when_digivolving]
        assert len(wd) == 1, "Should have exactly 1 When Digivolving effect"
        return wd[0]

    def _trigger_effect(self, game, perm, effect, target, hand_card_to_trash=None):
        """Trigger an On Play / When Digivolving effect, mocking the selections.

        Mocks:
        1. effect_select_hand_card -> picks hand_card_to_trash (or first hand card)
        2. effect_select_own_permanent -> picks target
        """
        player = perm.top_card.owner

        # Mock hand card selection (trash)
        original_select_hand = game.effect_select_hand_card

        def mock_select_hand(p, filter_fn, callback, is_optional=False, prompt=None):
            card_to_trash = hand_card_to_trash or player.hand_cards[0]
            callback(card_to_trash)

        game.effect_select_hand_card = mock_select_hand

        # Mock permanent selection (grant keywords)
        original_select_perm = game.effect_select_own_permanent

        def mock_select_perm(p, callback, filter_fn=None, is_optional=False, prompt=None):
            callback(target)

        game.effect_select_own_permanent = mock_select_perm

        effect.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Restore mocks
        game.effect_select_hand_card = original_select_hand
        game.effect_select_own_permanent = original_select_perm

    # ── On Play tests ─────────────────────────────────────────────────

    def test_on_play_trashes_hand_card(self, debug_runner):
        """[On Play] Should trash 1 card from hand."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])  # Agumon
        game = runner.game

        hand_before = len(game.player1.hand_cards)
        trash_before = len(game.player1.trash_cards)

        effect = self._get_on_play_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        assert len(game.player1.hand_cards) == hand_before - 1, \
            "Should have trashed 1 card from hand"
        assert len(game.player1.trash_cards) == trash_before + 1, \
            "Trashed card should be in trash"

    def test_on_play_grants_blocker_to_selected_digimon(self, debug_runner):
        """[On Play] Should grant Blocker to selected Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])  # Agumon
        game = runner.game

        effect = self._get_on_play_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        assert target.has_keyword('_is_blocker'), \
            "Selected Digimon should have Blocker"

    def test_on_play_grants_retaliation_to_selected_digimon(self, debug_runner):
        """[On Play] Should grant Retaliation to selected Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])  # Agumon
        game = runner.game

        effect = self._get_on_play_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        assert target.has_keyword('_is_retaliation'), \
            "Selected Digimon should have Retaliation"

    def test_on_play_keywords_last_until_end_of_opponent_turn(self, debug_runner):
        """[On Play] Blocker and Retaliation should persist through opponent's turn
        and expire at the start of the player's next turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])  # Agumon
        game = runner.game

        # Current turn_count is 1 (P1's turn)
        current_turn = game.turn_count

        effect = self._get_on_play_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        # Keywords should be active during current turn (T)
        assert target.has_keyword('_is_blocker'), \
            "Blocker should be active during current turn"
        assert target.has_keyword('_is_retaliation'), \
            "Retaliation should be active during current turn"

        # Simulate opponent's turn (T+1): keywords should still be active
        target.clear_expired_grants(current_turn + 1)
        assert target.has_keyword('_is_blocker'), \
            "Blocker should persist through opponent's turn"
        assert target.has_keyword('_is_retaliation'), \
            "Retaliation should persist through opponent's turn"

        # Simulate start of our next turn (T+2): keywords should expire
        target.clear_expired_grants(current_turn + 2)
        assert not target.has_keyword('_is_blocker'), \
            "Blocker should expire at start of player's next turn"
        assert not target.has_keyword('_is_retaliation'), \
            "Retaliation should expire at start of player's next turn"

    def test_on_play_keywords_not_permanent(self, debug_runner):
        """[On Play] Keywords should NOT be permanent (duration != -1)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        effect = self._get_on_play_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        # Check the granted keywords have a proper expiry (not -1)
        blocker_expiry = target._granted_keywords.get('_is_blocker')
        retaliation_expiry = target._granted_keywords.get('_is_retaliation')

        assert blocker_expiry is not None, "Blocker should be in granted keywords"
        assert retaliation_expiry is not None, "Retaliation should be in granted keywords"
        assert blocker_expiry != -1, "Blocker should not be permanent (must have expiry)"
        assert retaliation_expiry != -1, "Retaliation should not be permanent (must have expiry)"

    def test_on_play_can_target_self(self, debug_runner):
        """[On Play] Punkmon can select itself for Blocker + Retaliation."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        game = runner.game

        effect = self._get_on_play_effect(perm)
        # Target Punkmon itself
        self._trigger_effect(game, perm, effect, perm)

        assert perm.has_keyword('_is_blocker'), \
            "Punkmon should be able to grant Blocker to itself"
        assert perm.has_keyword('_is_retaliation'), \
            "Punkmon should be able to grant Retaliation to itself"

    # ── When Digivolving tests ────────────────────────────────────────

    def test_when_digivolving_grants_blocker_and_retaliation(self, debug_runner):
        """[When Digivolving] Should grant Blocker and Retaliation to selected Digimon."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        effect = self._get_when_digivolving_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        assert target.has_keyword('_is_blocker'), \
            "When Digivolving should grant Blocker"
        assert target.has_keyword('_is_retaliation'), \
            "When Digivolving should grant Retaliation"

    def test_when_digivolving_trashes_hand_card(self, debug_runner):
        """[When Digivolving] Should trash 1 card from hand."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        hand_before = len(game.player1.hand_cards)
        trash_before = len(game.player1.trash_cards)

        effect = self._get_when_digivolving_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        assert len(game.player1.hand_cards) == hand_before - 1
        assert len(game.player1.trash_cards) == trash_before + 1

    def test_when_digivolving_keywords_expire_correctly(self, debug_runner):
        """[When Digivolving] Keywords should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        target = runner.place_on_field(1, ["ST1-03"])
        game = runner.game

        current_turn = game.turn_count

        effect = self._get_when_digivolving_effect(perm)
        self._trigger_effect(game, perm, effect, target)

        # Active during current turn
        assert target.has_keyword('_is_blocker')
        assert target.has_keyword('_is_retaliation')

        # Still active during opponent's turn
        target.clear_expired_grants(current_turn + 1)
        assert target.has_keyword('_is_blocker')
        assert target.has_keyword('_is_retaliation')

        # Expired at start of our next turn
        target.clear_expired_grants(current_turn + 2)
        assert not target.has_keyword('_is_blocker')
        assert not target.has_keyword('_is_retaliation')

    # ── Inherited effect tests ────────────────────────────────────────

    def test_inherited_dp_boost_during_your_turn(self, debug_runner):
        """Inherited: [Your Turn] This Digimon gets +2000 DP."""
        runner = debug_runner(initial_memory=5)

        # Place Punkmon as inherited source under a Lv.5 Digimon
        # BT20-069 (Lv.4, DP 5000) on bottom, ST1-07 (Greymon, Lv.4, DP 5000) on top
        # Actually we need a Lv.5 on top. Use a generic setup.
        perm = runner.place_on_field(1, ["BT20-069", "ST1-11"])  # WarGreymon Lv.6 on top

        game = runner.game
        # P1's turn -> inherited should be active
        assert game.player1.is_my_turn, "Should be P1's turn"

        # WarGreymon (ST1-11) has DP 12000. With inherited +2000 = 14000
        base_dp = perm.top_card.c_entity_base.dp  # ST1-11 base DP
        assert perm.dp == base_dp + 2000, \
            f"Inherited should add +2000 DP during your turn (expected {base_dp + 2000}, got {perm.dp})"

    def test_inherited_dp_not_during_opponent_turn(self, debug_runner):
        """Inherited: DP boost should NOT apply during opponent's turn."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069", "ST1-11"])

        game = runner.game
        base_dp = perm.top_card.c_entity_base.dp

        # Verify boost during our turn
        assert perm.dp == base_dp + 2000

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        assert perm.dp == base_dp, \
            "Inherited DP boost should not apply during opponent's turn"

    # ── Alt digivolve tests ───────────────────────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt digivolve should be from Lv.3 with [Evil] trait for cost 2."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        card = perm.top_card
        effects = card.effect_list(None)

        alt_digi_effects = [e for e in effects
                            if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi_effects) == 1, "Should have exactly 1 alt digi effect"

        alt = alt_digi_effects[0]
        assert alt._alt_digi_cost == 2, "Alt digi cost should be 2"
        assert alt._alt_digi_trait == "Evil", "Alt digi trait should be Evil"
        assert alt._alt_digi_level == 3, "Alt digi level should be 3"

    # ── Selection filter tests ────────────────────────────────────────

    def test_only_digimon_can_be_selected_for_keywords(self, debug_runner):
        """The permanent selection should only allow Digimon (not Tamers)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        game = runner.game

        effect = self._get_on_play_effect(perm)

        # Track what filter_fn is passed to effect_select_own_permanent
        captured_filter = {}

        def mock_select_perm(p, callback, filter_fn=None, is_optional=False, prompt=None):
            captured_filter['fn'] = filter_fn
            # Don't call callback to avoid side effects
        game.effect_select_own_permanent = mock_select_perm

        # Mock hand selection to proceed to permanent selection
        def mock_select_hand(p, filter_fn, callback, is_optional=False, prompt=None):
            callback(p.hand_cards[0])
        game.effect_select_hand_card = mock_select_hand

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert 'fn' in captured_filter, "Should have called effect_select_own_permanent"
        filter_fn = captured_filter['fn']

        # Digimon should pass the filter
        assert filter_fn(perm), "Digimon permanents should pass the filter"

    def test_selection_is_mandatory(self, debug_runner):
        """Both the hand trash and permanent selection should be mandatory."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, ["BT20-069"])
        game = runner.game

        effect = self._get_on_play_effect(perm)

        # Track is_optional for both selections
        captured = {'hand_optional': None, 'perm_optional': None}

        def mock_select_hand(p, filter_fn, callback, is_optional=False, prompt=None):
            captured['hand_optional'] = is_optional
            callback(p.hand_cards[0])
        game.effect_select_hand_card = mock_select_hand

        def mock_select_perm(p, callback, filter_fn=None, is_optional=False, prompt=None):
            captured['perm_optional'] = is_optional
            callback(perm)  # Select something to complete the flow
        game.effect_select_own_permanent = mock_select_perm

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert captured['hand_optional'] is False, \
            "Hand card trash should be mandatory"
        assert captured['perm_optional'] is False, \
            "Permanent selection should be mandatory"
