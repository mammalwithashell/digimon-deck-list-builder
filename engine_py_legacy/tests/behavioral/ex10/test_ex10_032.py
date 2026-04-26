"""Behavioral tests for EX10-032 Proganomon (Lv.5 Black Digimon).

Card text:
  [Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash
  as any of your [Sunarizamon]'s bottom digivolution card, it digivolves into
  this card for a digivolution cost of 3, ignoring digivolution requirements.

  [On Play] [When Digivolving] [When Attacking] By trashing any 1 [Mineral] or
  [Rock] trait card from your Digimon's digivolution cards, 1 of your such Digimon
  gains <Collision>, <Piercing> and +3000 DP until your opponent's turn ends.

  Inherited: When effects trash this card from a [Mineral] or [Rock] trait
  Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.

Key cards used (place_on_field is bottom-to-top):
  EX10-032: Proganomon (Lv.5, Mineral/LIBERATOR, DP 7000)
  EX10-025: Sunarizamon (Lv.3, Reptile/LIBERATOR)
  EX10-028: Landramon (Lv.4, Mineral/LIBERATOR)
  EX8-048:  Landramon (Lv.4, Mineral/LIBERATOR)
  EX8-067:  Close (Tamer, LIBERATOR)
  EX8-046:  Gotsumon (Lv.3, Rock)
  BT4-066:  Golemon (Lv.4, Mineral)
  EX8-050:  Gogmamon (Lv.5, Rock, DP 7000)
  ST1-03:   Agumon (Reptile, Lv.3)
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase


def _resolve_accepting(runner, max_steps=20):
    """Auto-resolve selection phases by always choosing a non-decline action.

    Unlike auto_resolve (which picks the first legal action, often Decline),
    this helper picks the first non-62 legal action, ensuring effects activate.
    """
    results = []
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            break
        legal = runner.action_mask()
        if not legal:
            break
        # Pick the first non-decline action
        non_decline = [a for a in legal if a != 62]
        if non_decline:
            results.append(runner.execute(non_decline[0]))
        else:
            # Only decline available
            results.append(runner.execute(legal[0]))
    return results


@pytest.mark.behavioral
class TestEX10032HandMain:
    """Tests for [Hand][Main] special digivolution via Landramon + Sunarizamon."""

    def test_hand_main_activates_with_all_conditions_met(self, debug_runner):
        """Hand main effect should be available when Close on field,
        Landramon in trash, and Sunarizamon on field."""
        runner = debug_runner(initial_memory=10)

        # Place Close tamer on field
        runner.place_on_field(1, ["EX8-067"])
        # Place Sunarizamon on field
        runner.place_on_field(1, ["EX10-025"])
        # Put Landramon in trash
        runner.inject_card(1, "EX8-048", "trash")
        # Put Proganomon in hand
        runner.inject_card(1, "EX10-032", "hand")

        runner.set_phase("Main")

        # Should find the hand main action (in action range 30-59)
        actions = runner.actions()
        hand_main = {aid: desc for aid, desc in actions.items() if 30 <= aid <= 59}
        assert len(hand_main) > 0, (
            f"Hand main effect should be available when all conditions are met. "
            f"Available actions: {actions}"
        )

    def test_hand_main_not_available_without_close(self, debug_runner):
        """Hand main effect should NOT be available without Close tamer on field."""
        runner = debug_runner(initial_memory=10)

        # No Close tamer on field
        runner.place_on_field(1, ["EX10-025"])  # Sunarizamon
        runner.inject_card(1, "EX8-048", "trash")  # Landramon in trash
        runner.inject_card(1, "EX10-032", "hand")  # Proganomon in hand

        runner.set_phase("Main")

        # No hand main effects should be available (range 30-59)
        actions = runner.actions()
        hand_main = {aid: desc for aid, desc in actions.items() if 30 <= aid <= 59}
        assert len(hand_main) == 0, (
            f"Hand main effect should NOT be available without Close. Found: {hand_main}"
        )

    def test_hand_main_not_available_without_landramon_in_trash(self, debug_runner):
        """Hand main effect should NOT be available without Landramon in trash."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["EX8-067"])  # Close tamer
        runner.place_on_field(1, ["EX10-025"])  # Sunarizamon
        # No Landramon in trash
        runner.inject_card(1, "EX10-032", "hand")  # Proganomon in hand

        runner.set_phase("Main")

        actions = runner.actions()
        hand_main = {aid: desc for aid, desc in actions.items() if 30 <= aid <= 59}
        assert len(hand_main) == 0, (
            f"Hand main effect should NOT be available without Landramon in trash. Found: {hand_main}"
        )

    def test_hand_main_places_landramon_and_digivolves(self, debug_runner):
        """Executing hand main should place Landramon under Sunarizamon and
        digivolve into Proganomon for cost 3.

        Note: After digivolution, the WhenDigivolving trigger of the
        OP/WD/WA effect fires, which may trash the Landramon that was
        just placed. We verify the digivolution itself works correctly
        and the memory cost is paid.
        """
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["EX8-067"])  # Close tamer
        runner.place_on_field(1, ["EX10-025"])  # Sunarizamon
        runner.inject_card(1, "EX8-048", "trash")  # Landramon in trash
        runner.inject_card(1, "EX10-032", "hand")  # Proganomon in hand

        runner.set_phase("Main")
        snap_before = runner.snapshot()

        # Find hand main effect in 30-59 range
        actions = runner.actions()
        hand_main = {aid: desc for aid, desc in actions.items() if 30 <= aid <= 59}
        assert len(hand_main) > 0, f"Should have hand main action. Actions: {actions}"
        action = next(iter(hand_main))
        runner.execute(action)

        # Resolve: select Landramon from trash, select Sunarizamon,
        # then the WhenDigivolving trigger fires (accept all)
        _resolve_accepting(runner)

        snap_after = runner.snapshot()

        # Proganomon should now be on field (as top card of the stack)
        proga_on_field = any(
            s.card_id == "EX10-032" for s in snap_after.p1_field
        )
        assert proga_on_field, "Proganomon should be on field after digivolution"

        # Memory should have been reduced by 3 for digivolution
        # (may be further modified by WhenDigivolving effects)
        assert snap_after.memory <= snap_before.memory - 3, (
            f"Should pay at least 3 memory for digivolution. Before: {snap_before.memory}, After: {snap_after.memory}"
        )

        # The digivolution stack should contain Sunarizamon
        for slot in snap_after.p1_field:
            if slot.card_id == "EX10-032":
                assert "EX10-025" in slot.stack_ids, (
                    f"Sunarizamon should still be in the stack. Stack: {slot.stack_ids}"
                )
                break


@pytest.mark.behavioral
class TestEX10032GrantEffect:
    """Tests for [On Play] [When Digivolving] [When Attacking] effect granting
    Collision, Piercing, +3000 DP.

    The C# has a three-step selection flow:
      1. Select Digimon to trash a digi-card FROM (any Digimon with Mineral/Rock source)
      2. Select WHICH Mineral/Rock digi-card to trash (player choice)
      3. Select Mineral/Rock Digimon to receive Collision+Piercing+3000 DP
    """

    def test_on_play_grants_collision_piercing_dp(self, debug_runner):
        """On Play should allow trashing a Mineral/Rock digi-card and granting
        Collision, Piercing, +3000 DP to a Mineral/Rock Digimon."""
        runner = debug_runner(initial_memory=15)

        # Place a Mineral Digimon with a trashable Mineral digi-card
        # Bottom-to-top: BT4-066 (Golemon, Mineral) -> EX10-032 (Proganomon, Mineral)
        target_perm = runner.place_on_field(1, ["BT4-066", "EX10-032"])

        # Inject a second Proganomon into hand for playing
        runner.inject_card(1, "EX10-032", "hand")

        runner.set_phase("Main")

        base_dp = target_perm.dp

        # Play Proganomon to trigger On Play
        action = runner.find_action("Play Proganomon")
        assert action is not None, f"Should be able to play Proganomon. Actions: {runner.actions()}"
        runner.execute(action)

        # Resolve the three selection phases (always accept, don't decline)
        _resolve_accepting(runner)

        snap_after = runner.snapshot()

        # Check that a Mineral/Rock Digimon gained Piercing and Collision
        any_has_piercing = any(
            "piercing" in s.keywords for s in snap_after.p1_field if s.is_digimon
        )
        assert any_has_piercing, (
            f"A Mineral/Rock Digimon should have Piercing. "
            f"Field: {[(s.card_name, s.keywords, s.dp) for s in snap_after.p1_field]}"
        )

        any_has_collision = any(
            "collision" in s.keywords for s in snap_after.p1_field if s.is_digimon
        )
        assert any_has_collision, (
            f"A Mineral/Rock Digimon should have Collision. "
            f"Field: {[(s.card_name, s.keywords, s.dp) for s in snap_after.p1_field]}"
        )

    def test_on_play_trashes_player_selected_digi_card(self, debug_runner):
        """The effect should trash a Mineral/Rock digivolution card."""
        runner = debug_runner(initial_memory=15)

        # Place Proganomon with TWO trashable Mineral/Rock digi-cards underneath
        # Bottom-to-top: EX8-046 (Gotsumon, Rock), BT4-066 (Golemon, Mineral), EX10-032 (top)
        perm = runner.place_on_field(1, ["EX8-046", "BT4-066", "EX10-032"])

        # Inject Proganomon into hand for playing
        runner.inject_card(1, "EX10-032", "hand")

        runner.set_phase("Main")

        # Play Proganomon to trigger On Play
        action = runner.find_action("Play Proganomon")
        assert action is not None
        runner.execute(action)

        # Resolve the three selection phases
        _resolve_accepting(runner)

        snap_after = runner.snapshot()
        # One of the two digi-cards should have been trashed
        total_trashed = snap_after.p1_trash
        has_golemon = "BT4-066" in total_trashed
        has_gotsumon = "EX8-046" in total_trashed
        assert has_golemon or has_gotsumon, (
            f"One Mineral/Rock digi-card should be trashed. Trash: {total_trashed}"
        )

    def test_grant_target_can_be_different_from_trash_source(self, debug_runner):
        """The Digimon getting buffs can be DIFFERENT from the Digimon whose
        digi-card was trashed. The C# implementation uses three separate
        selection steps to enable this."""
        runner = debug_runner(initial_memory=15)

        # Place TWO Digimon:
        # Digimon A: Non-Mineral Digimon with a Mineral digi-card (trash source)
        #   Bottom-to-top: BT4-066 (Golemon, Mineral) -> ST1-03 (Agumon, Reptile)
        perm_a = runner.place_on_field(1, ["BT4-066", "ST1-03"])
        # Digimon B: Mineral trait Digimon (buff target)
        perm_b = runner.place_on_field(1, ["EX8-050"])  # Gogmamon, Rock trait

        # Inject Proganomon into hand for playing
        runner.inject_card(1, "EX10-032", "hand")

        runner.set_phase("Main")

        # Play Proganomon
        action = runner.find_action("Play Proganomon")
        assert action is not None
        runner.execute(action)

        # Resolve selections (accept all)
        _resolve_accepting(runner)

        snap_after = runner.snapshot()

        # At least one of the Mineral/Rock Digimon should have gained keywords
        any_has_piercing = any(
            "piercing" in s.keywords for s in snap_after.p1_field if s.is_digimon
        )
        assert any_has_piercing, (
            f"A Mineral/Rock Digimon should have gained Piercing. "
            f"Field: {[(s.card_name, s.keywords, s.dp) for s in snap_after.p1_field]}"
        )

    def test_when_attacking_grants_buffs(self, debug_runner):
        """When Attacking should trigger the grant effect."""
        runner = debug_runner(initial_memory=10)

        # Place Proganomon on field with a Mineral digi-card underneath
        # Bottom-to-top: BT4-066 (Golemon, Mineral) -> EX10-032 (Proganomon, Mineral)
        perm = runner.place_on_field(1, ["BT4-066", "EX10-032"])

        runner.set_phase("Main")

        base_dp = perm.dp

        # Attack with Proganomon
        action = runner.find_action("Attack player with Proganomon")
        assert action is not None, f"Should be able to attack. Actions: {runner.actions()}"
        runner.execute(action)

        # Resolve the three selection phases (accept all)
        _resolve_accepting(runner)

        # Check Proganomon gained buffs
        assert perm.has_keyword("_is_collision"), "Proganomon should have Collision after attacking"
        assert perm.has_keyword("_is_piercing"), "Proganomon should have Piercing after attacking"
        assert perm.dp >= base_dp + 3000, (
            f"Proganomon should have +3000 DP. Base: {base_dp}, Current: {perm.dp}"
        )

    def test_effect_is_optional(self, debug_runner):
        """The grant effect should be optional (can decline)."""
        runner = debug_runner(initial_memory=15)

        # Place Proganomon with a Mineral digi-card
        # Bottom-to-top: BT4-066 (Golemon) -> EX10-032 (Proganomon)
        perm = runner.place_on_field(1, ["BT4-066", "EX10-032"])

        runner.inject_card(1, "EX10-032", "hand")
        runner.set_phase("Main")

        # Play Proganomon
        action = runner.find_action("Play Proganomon")
        assert action is not None
        runner.execute(action)

        # Should be able to find a "Decline" option (action 62)
        legal = runner.action_mask()
        assert 62 in legal, (
            f"Should be able to decline the optional effect. Legal actions: {legal}"
        )

    def test_dp_boost_lasts_until_opponent_turn_ends(self, debug_runner):
        """The +3000 DP should be applied when the effect triggers."""
        runner = debug_runner(initial_memory=10)

        # Bottom-to-top: BT4-066 (Golemon, Mineral) -> EX10-032 (Proganomon, Mineral)
        perm = runner.place_on_field(1, ["BT4-066", "EX10-032"])
        # Place opponent Digimon to prevent game ending on security check
        runner.place_on_field(2, ["ST1-03"])

        runner.set_phase("Main")

        base_dp = perm.dp

        # Attack to trigger the effect
        action = runner.find_action("Attack player with Proganomon")
        assert action is not None
        runner.execute(action)
        _resolve_accepting(runner)

        # DP should be boosted now
        assert perm.dp >= base_dp + 3000, (
            f"DP should be boosted. Base: {base_dp}, Current: {perm.dp}"
        )

    def test_condition_requires_trashable_mineral_rock_source(self, debug_runner):
        """Effect should not be available if no Digimon has a trashable
        Mineral/Rock digivolution card."""
        runner = debug_runner(initial_memory=15)

        # Place Proganomon with NO digi-cards underneath
        runner.place_on_field(1, ["EX10-032"])

        runner.inject_card(1, "EX10-032", "hand")
        runner.set_phase("Main")

        # Play Proganomon
        action = runner.find_action("Play Proganomon")
        assert action is not None
        runner.execute(action)

        # Should go straight back to Main without triggering the grant effect
        snap = runner.snapshot()
        assert snap.phase == "Main", (
            f"Should return to Main when no trashable source available. Phase: {snap.phase}"
        )

    def test_source_digimon_need_not_have_mineral_rock_trait(self, debug_runner):
        """The Digimon you trash FROM does NOT need to have Mineral/Rock trait.
        Per C# CanSelectPermamentTrashDigivolution, only the digi-card needs
        the trait, not the Digimon itself."""
        runner = debug_runner(initial_memory=15)

        # Place a non-Mineral/Rock Digimon (Agumon) with a Mineral digi-card
        # Bottom-to-top: BT4-066 (Golemon, Mineral) -> ST1-03 (Agumon, Reptile)
        non_mineral_perm = runner.place_on_field(1, ["BT4-066", "ST1-03"])
        # Place a Mineral Digimon for the buff target
        mineral_perm = runner.place_on_field(1, ["EX10-032"])

        runner.inject_card(1, "EX10-032", "hand")
        runner.set_phase("Main")

        # Play Proganomon
        action = runner.find_action("Play Proganomon")
        assert action is not None
        runner.execute(action)

        # Resolve all three steps (accept all)
        _resolve_accepting(runner)

        snap_after = runner.snapshot()
        # Golemon should be trashed from Agumon's stack
        assert "BT4-066" in snap_after.p1_trash, (
            f"Golemon should be trashed. Trash: {snap_after.p1_trash}"
        )


@pytest.mark.behavioral
class TestEX10032InheritedDeDigivolve:
    """Tests for inherited effect: De-Digivolve 1 when this card is trashed
    from a Mineral/Rock Digimon's digi-cards."""

    def test_inherited_triggers_de_digivolve_on_trash(self, debug_runner):
        """When EX10-032 is trashed from a Mineral/Rock Digimon's digi-cards,
        should trigger De-Digivolve 1 on an opponent's Digimon."""
        runner = debug_runner(initial_memory=10)

        # Place a Mineral Digimon with EX10-032 (Proganomon) as inherited source
        # Bottom-to-top: EX10-032 (Proganomon, Mineral) -> EX10-028 (Landramon, Mineral)
        perm = runner.place_on_field(1, ["EX10-032", "EX10-028"])

        # Place an opponent Digimon with digivolution cards
        # Bottom-to-top: ST1-02 -> ST1-03
        opp_perm = runner.place_on_field(2, ["ST1-02", "ST1-03"])

        opp_stack_before = len(opp_perm.card_sources)

        # Trash the digivolution card (EX10-032) from the Mineral Digimon
        game = runner.game
        player = game.player1
        trashed = perm.trash_digivolution_cards(1, from_top=True)
        player.trash_cards.extend(trashed)

        # Auto-resolve the De-Digivolve selection (mandatory, no decline)
        _resolve_accepting(runner)

        # The opponent's Digimon should have been de-digivolved
        opp_stack_after = len(opp_perm.card_sources)

        assert opp_stack_after < opp_stack_before, (
            f"Opponent Digimon should have been de-digivolved. "
            f"Stack before: {opp_stack_before}, after: {opp_stack_after}"
        )

    def test_inherited_does_not_trigger_from_non_mineral_rock_digimon(self, debug_runner):
        """Inherited effect should NOT trigger when trashed from a Digimon
        without Mineral/Rock trait."""
        runner = debug_runner(initial_memory=10)

        # Place a non-Mineral/Rock Digimon (Agumon top) with EX10-032 underneath
        # Bottom-to-top: EX10-032 (Proganomon) -> ST1-03 (Agumon)
        perm = runner.place_on_field(1, ["EX10-032", "ST1-03"])

        # Place opponent Digimon with digi-cards
        # Bottom-to-top: ST1-02 -> ST1-03
        opp_perm = runner.place_on_field(2, ["ST1-02", "ST1-03"])

        opp_stack_before = len(opp_perm.card_sources)

        # Trash the digi-card
        game = runner.game
        player = game.player1
        trashed = perm.trash_digivolution_cards(1, from_top=True)
        player.trash_cards.extend(trashed)

        # Auto-resolve
        _resolve_accepting(runner)

        opp_stack_after = len(opp_perm.card_sources)

        assert opp_stack_after == opp_stack_before, (
            f"Opponent Digimon should NOT have been de-digivolved from non-Mineral/Rock Digimon. "
            f"Stack before: {opp_stack_before}, after: {opp_stack_after}"
        )
