"""Behavioral tests for BT15-034 Salamon (Lv.3, Yellow Mammal, Vaccine).

Card text (from cards.json):

Main:
[Start of Your Main Phase] If you have 3 or more security cards, you may add the top
card of your security stack to the hand. If you have 2 or fewer, you may place 1
yellow Digimon card with the [Vaccine] trait from your hand at the top or bottom
of your security stack.

Inherited:
[All Turns] [Once Per Turn] When a card is removed from your security stack,
1 of your opponent's Digimon gets -2000 DP for the turn.

Verified properties:
1. OnStartMainPhase main effect exists, marked optional, NOT Once Per Turn
2. Main-phase condition requires the card to be on field + owner's turn
3. Main effect has two mutually-exclusive branches gated on security count
4. Hand filter only accepts yellow Vaccine Digimon
5. Top/Bottom security placement is an agent choice (not hardcoded)
6. OnLoseSecurity inherited effect is Once Per Turn with correct hash
7. OnLoseSecurity fires only when OWN security is removed (event_player gate)
8. DP debuff uses register_modifier with CHANGE_DP + end_of_turn expiry
9. DP target is selected (not auto-picked lowest)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase, CardColor
from digimon_gym.engine.interfaces.modifiers import ModifierType


SALAMON = "BT15-034"
YELLOW_VACCINE_LV3 = "BT1-046"   # Kudamon — yellow Vaccine Lv.3
YELLOW_NON_VACCINE = "AD1-015"   # Beowolfmon — yellow non-Vaccine Lv.5
NON_YELLOW_VACCINE = "AD1-001"   # Greymon — red Vaccine Lv.4
OPP_DIGIMON = "ST1-07"           # Greymon — opponent target for DP debuff
OPP_DIGIMON_2 = "ST1-03"         # Agumon — lowest-DP target for DP debuff


def _salamon_effects(perm):
    """Return effects from the Salamon CardSource (top card only)."""
    top = perm.top_card
    assert top is not None
    assert top.c_entity_base and top.c_entity_base.card_id == SALAMON
    return top.effect_list(None)


def _find_main_effect(perm):
    for e in _salamon_effects(perm):
        if e.timing == EffectTiming.OnStartMainPhase:
            return e
    return None


def _find_inherited_lose_sec_effect(perm):
    for e in _salamon_effects(perm):
        if (e.timing == EffectTiming.OnLoseSecurity
                and getattr(e, 'is_inherited_effect', False)):
            return e
    return None


# =====================================================================
#  Main-phase effect (Start of Your Main Phase)
# =====================================================================

@pytest.mark.behavioral
class TestBT15034MainEffectStructure:

    def test_main_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        assert effect is not None, \
            "BT15-034 should have OnStartMainPhase effect"

    def test_main_effect_is_optional(self, debug_runner):
        """Card text uses 'may' — effect must be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        assert effect.is_optional, \
            "Main effect should be optional ('may' in both branches)"

    def test_main_effect_not_inherited(self, debug_runner):
        """Main effect is NOT below the inheritance line."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        assert not getattr(effect, 'is_inherited_effect', False), \
            "Main Start-of-Main effect is above the line, not inherited"

    def test_main_effect_not_once_per_turn(self, debug_runner):
        """C# uses maxUsePerTurn=-1 (unlimited). Card text has no OPT clause."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        assert effect.max_count_per_turn != 1, \
            "Main effect has no Once Per Turn restriction"


@pytest.mark.behavioral
class TestBT15034MainEffectCondition:

    def test_requires_field_presence(self, debug_runner):
        """Effect should be inert when Salamon is not on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        game = runner.game
        # Remove Salamon from field
        game.player1.battle_area.remove(perm)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), \
            "Main effect should require Salamon to be on field"

    def test_requires_owner_turn(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        game = runner.game
        # Force opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert not effect.can_use_condition(context), \
            "Main effect should only fire on owner's turn"

    def test_active_on_owner_turn_with_security(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_main_effect(perm)
        game = runner.game
        # Owner's turn is already set after place_on_field by default
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
        }
        assert effect.can_use_condition(context), \
            "Main effect should be active while Salamon is on owner's field"


@pytest.mark.behavioral
class TestBT15034MainEffectProcessBranch3Plus:
    """If you have 3 or more security cards, you may add top of security to hand."""

    def test_add_top_security_to_hand_when_3_plus(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game

        # Ensure 5 security cards, record the top one (index 0 = top)
        assert len(game.player1.security_cards) >= 3
        top_sec = game.player1.security_cards[0]
        sec_before = len(game.player1.security_cards)
        hand_before = len(game.player1.hand_cards)

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        # Resolve any optional accept/decline branch and select target
        runner.auto_resolve(max_steps=5)

        assert top_sec in game.player1.hand_cards, \
            "Top security card should have moved into hand"
        assert top_sec not in game.player1.security_cards, \
            "Top security card should have been removed from security"
        assert len(game.player1.security_cards) == sec_before - 1
        assert len(game.player1.hand_cards) == hand_before + 1

    def test_add_top_uses_index_zero(self, debug_runner):
        """Engine convention: security_cards[0] = top (where security_attack pops from)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game

        # Place a distinctive card at the top via the documented API
        runner.inject_card(1, NON_YELLOW_VACCINE, "security_top")
        expected_top_id = game.player1.security_cards[0].c_entity_base.card_id
        assert expected_top_id == NON_YELLOW_VACCINE

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })
        runner.auto_resolve(max_steps=5)

        hand_ids = [c.c_entity_base.card_id for c in game.player1.hand_cards]
        assert NON_YELLOW_VACCINE in hand_ids, \
            "The actual top card (index 0) should have been added to hand"


@pytest.mark.behavioral
class TestBT15034MainEffectProcessBranch2OrFewer:
    """If you have 2 or fewer security cards, place a yellow Vaccine Digimon
    from hand at the top or bottom of the security stack."""

    def _reduce_security_to(self, runner, count):
        p1 = runner.game.player1
        while len(p1.security_cards) > count:
            p1.security_cards.pop(0)

    def test_hand_filter_accepts_yellow_vaccine(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        self._reduce_security_to(runner, 2)

        # Give player1 a yellow Vaccine Digimon + non-matching cards
        runner.inject_card(1, YELLOW_VACCINE_LV3, "hand")
        runner.inject_card(1, YELLOW_NON_VACCINE, "hand")
        runner.inject_card(1, NON_YELLOW_VACCINE, "hand")

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # Should now be awaiting a hand-card selection
        assert game.current_phase == GamePhase.SelectHand, \
            f"Expected SelectHand phase, got {game.current_phase}"
        ps = game.pending_selection
        assert ps is not None

        # Valid selections must all be the yellow Vaccine card only
        from digimon_gym.engine.game.constants import SEL_HAND_START
        valid_cards = []
        for v in ps.valid_indices:
            idx = v - SEL_HAND_START
            if 0 <= idx < len(game.player1.hand_cards):
                valid_cards.append(game.player1.hand_cards[idx])
        valid_ids = [c.c_entity_base.card_id for c in valid_cards]
        assert YELLOW_VACCINE_LV3 in valid_ids, \
            "Yellow Vaccine card should be selectable"
        assert YELLOW_NON_VACCINE not in valid_ids, \
            "Yellow non-Vaccine card must NOT be selectable"
        assert NON_YELLOW_VACCINE not in valid_ids, \
            "Non-yellow Vaccine card must NOT be selectable"

    def test_no_action_when_no_valid_hand_card(self, debug_runner):
        """Branch silently does nothing when hand has no yellow Vaccine card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        self._reduce_security_to(runner, 2)

        # Clear hand, then add only non-matching cards
        game.player1.hand_cards.clear()
        runner.inject_card(1, YELLOW_NON_VACCINE, "hand")
        runner.inject_card(1, NON_YELLOW_VACCINE, "hand")

        sec_before = len(game.player1.security_cards)
        hand_before = len(game.player1.hand_cards)

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # Should NOT enter a selection phase (nothing to pick)
        assert game.current_phase != GamePhase.SelectHand, \
            "Should not enter SelectHand when no valid card exists"
        assert len(game.player1.security_cards) == sec_before
        assert len(game.player1.hand_cards) == hand_before

    def test_top_bottom_choice_offered(self, debug_runner):
        """Player must be offered a top/bottom branch after selecting the card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        self._reduce_security_to(runner, 2)

        game.player1.hand_cards.clear()
        runner.inject_card(1, YELLOW_VACCINE_LV3, "hand")

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # 1) Expected to enter SelectHand
        assert game.current_phase == GamePhase.SelectHand
        ps = game.pending_selection
        assert ps is not None and ps.valid_indices

        # Pick the first valid action (the yellow Vaccine card)
        game.decode_action(ps.valid_indices[0], game.current_player_id)

        # 2) After the hand selection, a branch choice for top/bottom should appear
        assert game.current_phase == GamePhase.SelectEffectChoice, \
            f"Expected SelectEffectChoice after hand pick, got {game.current_phase}"
        ps2 = game.pending_selection
        assert ps2 is not None
        assert len(ps2.valid_indices) == 2, \
            "Should offer exactly 2 branches (top / bottom)"

    def test_place_card_on_top_of_security(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        self._reduce_security_to(runner, 2)

        game.player1.hand_cards.clear()
        runner.inject_card(1, YELLOW_VACCINE_LV3, "hand")
        sec_before = len(game.player1.security_cards)

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # Pick the card
        assert game.current_phase == GamePhase.SelectHand
        game.decode_action(game.pending_selection.valid_indices[0],
                           game.current_player_id)

        # Pick branch 0 = top
        assert game.current_phase == GamePhase.SelectEffectChoice
        game.decode_action(game.pending_selection.valid_indices[0],
                           game.current_player_id)

        assert len(game.player1.security_cards) == sec_before + 1
        # Top of stack = index 0
        top_card = game.player1.security_cards[0]
        assert top_card.c_entity_base.card_id == YELLOW_VACCINE_LV3, \
            "Card should be placed at TOP (index 0) of security"
        # And it must have left the hand
        assert not any(
            c.c_entity_base.card_id == YELLOW_VACCINE_LV3
            for c in game.player1.hand_cards
        )

    def test_place_card_on_bottom_of_security(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        self._reduce_security_to(runner, 2)

        game.player1.hand_cards.clear()
        runner.inject_card(1, YELLOW_VACCINE_LV3, "hand")
        sec_before = len(game.player1.security_cards)

        effect = _find_main_effect(perm)
        effect.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': perm,
        })

        # Pick the card
        game.decode_action(game.pending_selection.valid_indices[0],
                           game.current_player_id)
        # Pick branch 1 = bottom
        assert game.current_phase == GamePhase.SelectEffectChoice
        game.decode_action(game.pending_selection.valid_indices[1],
                           game.current_player_id)

        assert len(game.player1.security_cards) == sec_before + 1
        # Bottom of stack = last index
        bottom_card = game.player1.security_cards[-1]
        assert bottom_card.c_entity_base.card_id == YELLOW_VACCINE_LV3, \
            "Card should be placed at BOTTOM (last index) of security"


# =====================================================================
#  Inherited OnLoseSecurity effect
# =====================================================================

@pytest.mark.behavioral
class TestBT15034InheritedLoseSecurityStructure:

    def test_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_inherited_lose_sec_effect(perm)
        assert effect is not None, \
            "BT15-034 should have an inherited OnLoseSecurity effect"

    def test_is_inherited(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_inherited_lose_sec_effect(perm)
        assert effect.is_inherited_effect is True

    def test_once_per_turn(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_inherited_lose_sec_effect(perm)
        assert effect.max_count_per_turn == 1, \
            "Inherited DP debuff should be Once Per Turn"

    def test_hash_string_matches_dcgo(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_inherited_lose_sec_effect(perm)
        assert effect.hash_string == "-2000DP_BT15_034"


@pytest.mark.behavioral
class TestBT15034InheritedLoseSecurityCondition:

    def test_field_presence_required(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        effect = _find_inherited_lose_sec_effect(perm)
        game = runner.game
        game.player1.battle_area.remove(perm)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        }
        assert not effect.can_use_condition(context), \
            "Should not activate without field presence"

    def test_fires_when_own_security_lost(self, debug_runner):
        """DCGO: player == card.Owner — trigger when our own security is removed."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        effect = _find_inherited_lose_sec_effect(perm)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,  # own security lost
        }
        assert effect.can_use_condition(context), \
            "Should fire when Salamon's owner loses security"

    def test_does_not_fire_when_opponent_security_lost(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        effect = _find_inherited_lose_sec_effect(perm)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player2,  # opponent lost security
        }
        assert not effect.can_use_condition(context), \
            "Must NOT fire when the opponent loses security"


@pytest.mark.behavioral
class TestBT15034InheritedLoseSecurityProcess:

    def test_offers_target_selection(self, debug_runner):
        """Effect must ask player which opponent Digimon to debuff."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        # Give opponent 2 Digimon so selection is meaningful (not auto)
        runner.place_on_field(2, [OPP_DIGIMON])
        runner.place_on_field(2, [OPP_DIGIMON_2])

        effect = _find_inherited_lose_sec_effect(perm)
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        })

        assert game.current_phase == GamePhase.SelectTarget, \
            f"Expected SelectTarget phase, got {game.current_phase}"
        ps = game.pending_selection
        assert ps is not None
        # Both opponent digimon selectable (not just lowest DP)
        assert len(ps.valid_indices) == 2, \
            f"Player must pick either digimon (got {len(ps.valid_indices)} options)"

    def test_applies_minus_2000_dp_via_modifier(self, debug_runner):
        """DP debuff must be applied via register_modifier (CHANGE_DP, end_of_turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        target = runner.place_on_field(2, [OPP_DIGIMON])  # Greymon 4000 DP

        dp_before = target.dp
        effect = _find_inherited_lose_sec_effect(perm)
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        })
        runner.auto_resolve(max_steps=5)

        assert target.dp == dp_before - 2000, \
            f"Target DP should drop by 2000 (was {dp_before}, now {target.dp})"

    def test_debuff_expires_end_of_turn(self, debug_runner):
        """Debuff must use expiry='end_of_turn' — value must be restored after turn end."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        target = runner.place_on_field(2, [OPP_DIGIMON])  # Greymon 4000 DP
        dp_base = target.dp

        effect = _find_inherited_lose_sec_effect(perm)
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        })
        runner.auto_resolve(max_steps=5)
        assert target.dp == dp_base - 2000

        # Check modifier registered with end_of_turn expiry on the target
        change_dp_entries = game.modifiers._modifiers.get(
            ModifierType.CHANGE_DP, [])
        relevant = [
            e for e in change_dp_entries
            if e.is_active(target, None)
        ]
        assert any(e.expiry == 'end_of_turn' for e in relevant), \
            "CHANGE_DP modifier must have expiry='end_of_turn'"

    def test_does_not_apply_to_own_digimon(self, debug_runner):
        """Effect targets OPPONENT's Digimon only."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [SALAMON])
        game = runner.game
        own = runner.place_on_field(1, [OPP_DIGIMON])
        opp = runner.place_on_field(2, [OPP_DIGIMON])

        effect = _find_inherited_lose_sec_effect(perm)
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_player': game.player1,
        })

        # In SelectTarget, only opponent permanents should be selectable
        assert game.current_phase == GamePhase.SelectTarget
        ps = game.pending_selection
        # Selected indices must all correspond to opponent Digimon
        from digimon_gym.engine.game import Game as _G  # noqa
        # Simpler: advance one step and check outcomes
        game.decode_action(ps.valid_indices[0], game.current_player_id)
        runner.auto_resolve(max_steps=3)
        # own DP unchanged; opponent DP dropped
        assert own.dp == own.top_card.base_dp, \
            "Own Digimon DP should not change"
        assert opp.dp == opp.top_card.base_dp - 2000, \
            "Opponent Digimon DP should drop by 2000"
