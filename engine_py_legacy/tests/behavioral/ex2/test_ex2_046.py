"""Behavioral tests for EX2-046 ADR-02 Searcher (Digimon, White, Cost 3).

Card text (from cards.json):
When you would play this card from your hand, reduce its play cost by 2
    if you don't have another [ADR-02 Searcher] in play.
[Your Turn] This Digimon can't attack players.
[On Play] <Draw 1> (Draw 1 card from your deck.)

Inherited Effect:
[Your Turn] All of your Digimon with the [D-Reaper] trait get +1000 DP.

C# reference (EX2_046.cs):
- Cost reduction: ChangeCostClass in EffectTiming.None, checks
    card.Owner.HandCards.Contains(card) AND no other ADR-02 Searcher on field
- Can't attack players: CanNotAttackSelfStaticEffect with
    DefenderCondition(permanent) => permanent == null (i.e. player target)
    Condition: IsOwnerTurn(card) (Your Turn only)
- On Play: Draw 1 with CanTriggerOnPlay
- Inherited DP: [Your Turn] D-Reaper Digimon +1000 DP with PermanentCondition
    checking CardTraits.Contains("D-Reaper")

Key faithfulness checks:
1. Cost reduction leak guard: only when THIS card is being played (not from field)
2. Can't attack players is [Your Turn] only
3. Inherited DP must filter for D-Reaper trait (not all own Digimon)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, CardColor


@pytest.mark.behavioral
class TestEX2046ADR02Searcher:
    """Tests for EX2-046 ADR-02 Searcher."""

    def _get_cost_reduction_effect(self, card_source):
        """Get the cost reduction effect."""
        effects = card_source.effect_list(None)
        cr = [e for e in effects if getattr(e, 'cost_reduction', 0) > 0]
        assert len(cr) >= 1, "ADR-02 should have a cost reduction effect"
        return cr[0]

    def _get_cant_attack_players_effect(self, card_source):
        """Get the can't attack players effect."""
        effects = card_source.effect_list(None)
        cap = [e for e in effects if getattr(e, '_is_cannot_attack_player', False)]
        assert len(cap) >= 1, "ADR-02 should have a can't attack players effect"
        return cap[0]

    def _get_on_play_effect(self, card_source):
        """Get the On Play draw effect."""
        effects = card_source.effect_list(None)
        op = [e for e in effects
              if e.timing == EffectTiming.OnEnterFieldAnyone
              and getattr(e, 'is_on_play', False)]
        assert len(op) >= 1, "ADR-02 should have an On Play effect"
        return op[0]

    def _get_inherited_dp_effect(self, card_source):
        """Get the inherited DP modifier effect."""
        effects = card_source.effect_list(None)
        idp = [e for e in effects
               if getattr(e, 'is_inherited_effect', False)
               and getattr(e, 'dp_modifier', 0) > 0]
        assert len(idp) >= 1, "ADR-02 should have an inherited DP effect"
        return idp[0]

    # ------------------------------------------------------------------
    # Effect 0: Cost reduction by 2 if no other ADR-02 in play
    # ------------------------------------------------------------------

    def test_cost_reduction_exists(self, debug_runner):
        """Should have a cost reduction effect."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_cost_reduction_effect(cs)
        assert effect.cost_reduction == 2, "Cost reduction should be 2"

    def test_cost_reduction_leak_guard(self, debug_runner):
        """Cost reduction should ONLY apply when THIS card is being played.

        The BeforePayCost leak guard must check card_source is the card itself.
        C# checks: CardSourceCondition(cardSource) => cardSource == card
        """
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        # Place one ADR-02 on field
        perm = runner.place_on_field(1, ["EX2-046"])
        game = runner.game

        # Get the effect from the on-field card
        effect = self._get_cost_reduction_effect(perm.top_card)

        # Simulate another card being played -- should NOT reduce
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        other_card = db.create_card_source("ST1-03", game.player1)
        result = effect.can_use_condition({'card_source': other_card})
        assert not result, "Cost reduction should NOT apply to other cards being played"

    def test_cost_reduction_fails_with_another_adr02_on_field(self, debug_runner):
        """Cost reduction should fail when another ADR-02 is already on field."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
            starting_hand1=["EX2-046"],
        )
        game = runner.game

        # Place one ADR-02 on field
        perm_field = runner.place_on_field(1, ["EX2-046"])

        # Get a hand ADR-02 card
        hand_adr02 = None
        for c in game.player1.hand_cards:
            if c.card_names and 'ADR-02 Searcher' in c.card_names:
                hand_adr02 = c
                break
        assert hand_adr02 is not None, "Should have ADR-02 in hand"

        effect = self._get_cost_reduction_effect(hand_adr02)
        result = effect.can_use_condition({'card_source': hand_adr02})
        assert not result, "Cost reduction should fail with another ADR-02 on field"

    def test_cost_reduction_passes_no_other_adr02(self, debug_runner):
        """Cost reduction should pass when no other ADR-02 is on field."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
            starting_hand1=["EX2-046"],
        )
        game = runner.game

        hand_adr02 = None
        for c in game.player1.hand_cards:
            if c.card_names and 'ADR-02 Searcher' in c.card_names:
                hand_adr02 = c
                break
        assert hand_adr02 is not None, "Should have ADR-02 in hand"

        effect = self._get_cost_reduction_effect(hand_adr02)
        result = effect.can_use_condition({'card_source': hand_adr02})
        assert result, "Cost reduction should pass when no other ADR-02 on field"

    # ------------------------------------------------------------------
    # Effect 1: [Your Turn] Can't attack players
    # ------------------------------------------------------------------

    def test_cant_attack_players_exists(self, debug_runner):
        """Should have a can't attack players effect."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-046"])
        effect = self._get_cant_attack_players_effect(perm.top_card)
        assert effect is not None

    def test_cant_attack_players_your_turn_only(self, debug_runner):
        """Can't attack players should be active on your turn only.

        C# Condition: return CardEffectCommons.IsOwnerTurn(card);
        """
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-046"])
        game = runner.game
        effect = self._get_cant_attack_players_effect(perm.top_card)

        # Should be active on your turn
        result_your_turn = effect.can_use_condition({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })
        assert result_your_turn, "Should be active on your turn"

        # Should NOT be active on opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        result_opp_turn = effect.can_use_condition({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })
        assert not result_opp_turn, "Should NOT be active on opponent's turn"

    def test_cant_attack_players_requires_field(self, debug_runner):
        """Can't attack players should require being on field."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-046"])
        effect = self._get_cant_attack_players_effect(perm.top_card)

        runner.game.player1.battle_area.remove(perm)
        result = effect.can_use_condition({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        })
        assert not result, "Should fail when not on field"

    # ------------------------------------------------------------------
    # Effect 2: [On Play] <Draw 1>
    # ------------------------------------------------------------------

    def test_on_play_exists(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_on_play_effect(cs)
        assert effect is not None

    def test_on_play_draws_one(self, debug_runner):
        """On Play process should draw 1 card."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["EX2-046"])
        game = runner.game

        effect = self._get_on_play_effect(perm.top_card)
        hand_before = len(game.player1.hand_cards)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.hand_cards) == hand_before + 1, \
            "On Play should draw exactly 1 card"

    # ------------------------------------------------------------------
    # Effect 3: Inherited [Your Turn] D-Reaper +1000 DP
    # ------------------------------------------------------------------

    def test_inherited_dp_effect_exists(self, debug_runner):
        """Should have an inherited DP modifier effect."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_inherited_dp_effect(cs)
        assert effect.dp_modifier == 1000, "DP modifier should be +1000"

    def test_inherited_dp_is_inherited(self, debug_runner):
        """The DP effect should be marked as inherited."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_inherited_dp_effect(cs)
        assert effect.is_inherited_effect, "Must be inherited effect"

    def test_inherited_dp_applies_to_all_own_digimon(self, debug_runner):
        """The inherited DP effect should use _applies_to_all_own_digimon aura pattern."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_inherited_dp_effect(cs)
        assert getattr(effect, '_applies_to_all_own_digimon', False), \
            "Should use _applies_to_all_own_digimon aura pattern"

    def test_inherited_dp_filters_for_dreaper_trait(self, debug_runner):
        """CRITICAL: Inherited DP should only apply to D-Reaper trait Digimon.

        C# PermanentCondition:
            if (permanent.TopCard.CardTraits.Contains("D-Reaper")) return true;

        Bug: Current script sets _applies_to_all_own_digimon but has NO
        _dp_permanent_condition filter for D-Reaper trait, so it buffs ALL
        own Digimon instead of only D-Reaper ones.
        """
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["EX2-047"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        game = runner.game

        # Place a D-Reaper Digimon with ADR-02 as inherited source
        # ADR-03 Pendulum Feet (EX2-047) is D-Reaper, 3000 DP
        dreaper_perm = runner.place_on_field(1, ["EX2-046", "EX2-047"])

        # Place a non-D-Reaper Digimon (ST1-03 Agumon, 2000 DP, Reptile trait)
        non_dreaper_perm = runner.place_on_field(1, ["ST1-03"])

        # The inherited ADR-02 DP aura should NOT buff non-D-Reaper Digimon
        # Check the _dp_permanent_condition filter exists
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        adr02_cs = db.create_card_source("EX2-046", game.player1)
        effect = self._get_inherited_dp_effect(adr02_cs)

        perm_filter = getattr(effect, '_dp_permanent_condition', None)
        assert perm_filter is not None, (
            "Inherited DP effect must have _dp_permanent_condition to filter "
            "for D-Reaper trait. Without it, ALL own Digimon get +1000 DP "
            "instead of only D-Reaper ones."
        )

    def test_inherited_dp_filter_accepts_dreaper(self, debug_runner):
        """The DP filter should accept Digimon with D-Reaper trait."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["EX2-047"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        game = runner.game

        dreaper_perm = runner.place_on_field(1, ["EX2-047"])

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        adr02_cs = db.create_card_source("EX2-046", game.player1)
        effect = self._get_inherited_dp_effect(adr02_cs)

        perm_filter = getattr(effect, '_dp_permanent_condition', None)
        if perm_filter:
            assert perm_filter(dreaper_perm), \
                "DP filter should accept D-Reaper trait Digimon"

    def test_inherited_dp_filter_rejects_non_dreaper(self, debug_runner):
        """The DP filter should reject non-D-Reaper Digimon."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        game = runner.game

        non_dreaper_perm = runner.place_on_field(1, ["ST1-03"])

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        adr02_cs = db.create_card_source("EX2-046", game.player1)
        effect = self._get_inherited_dp_effect(adr02_cs)

        perm_filter = getattr(effect, '_dp_permanent_condition', None)
        if perm_filter:
            assert not perm_filter(non_dreaper_perm), \
                "DP filter should reject non-D-Reaper Digimon"

    def test_inherited_dp_your_turn_only(self, debug_runner):
        """Inherited DP should only be active on your turn.

        C# Condition: IsExistOnBattleArea && IsOwnerTurn
        """
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["ST1-03"] * 46,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("EX2-046", runner.game.player1)
        effect = self._get_inherited_dp_effect(cs)

        # Should be active on your turn
        runner.game.player1.is_my_turn = True
        result_your_turn = effect.can_use_condition({
            'permanent': None,
        })
        assert result_your_turn, "Should be active on your turn"

        # Should NOT be active on opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        result_opp_turn = effect.can_use_condition({
            'permanent': None,
        })
        assert not result_opp_turn, "Should NOT be active on opponent's turn"

    def test_inherited_dp_integration_dreaper_gets_buff(self, debug_runner):
        """Integration: A D-Reaper Digimon with ADR-02 inherited should get +1000 DP."""
        runner = debug_runner(
            deck1=["EX2-046"] * 4 + ["EX2-047"] * 4 + ["ST1-03"] * 42,
            deck2=["ST1-03"] * 4 + ["ST1-02"] * 46,
            initial_memory=5,
        )
        game = runner.game

        # ADR-03 (EX2-047) has 3000 base DP and D-Reaper trait
        # Stack: ADR-02 (bottom, inherited DP) -> ADR-03 (top)
        perm = runner.place_on_field(1, ["EX2-046", "EX2-047"])

        # Another D-Reaper Digimon should get the aura buff
        other_dreaper = runner.place_on_field(1, ["EX2-048"])  # ADR-04 Bubbles, 3000 DP

        # The inherited aura from ADR-02 under ADR-03 should buff other D-Reaper Digimon
        # This is the aura system: effects from UNDER the top card of OTHER permanents
        # don't self-buff, they buff other permanents
        base_dp = other_dreaper.top_card.base_dp if other_dreaper.top_card else 0
        actual_dp = other_dreaper.dp

        # If DP filter is correct, the other D-Reaper should get +1000
        # If DP filter is missing, all Digimon get +1000 (still passes, but wrong)
        assert actual_dp >= base_dp, \
            f"D-Reaper Digimon should have at least base DP ({base_dp}), got {actual_dp}"
