"""Behavioral tests for P-215 Icemon | Lv.4 (Blue/Black, Ice-Snow).

Card text:
[When Moving] [On Play] [When Digivolving] By placing 1 level 4 or lower
[Ice-Snow], [Mineral] or [Rock] trait card from your hand or trash as this
Digimon's bottom digivolution card, until your opponent's turn ends, their
effects can't return 1 of your [Ice-Snow], [Mineral] or [Rock] trait Digimon
to hands or decks or affect it with De-Digivolve effects.

Inherited: Blocker

Issues being tested:
  1. Trash selection must use SelectTrash phase (no auto-pick)
  2. When both hand and trash have candidates, player chooses zone
  3. Protection must target exactly 1 chosen Digimon (not all)
  4. Hand-only path uses effect_select_hand_card (correct, keep)
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase


# --- Test card IDs ---
ICEMON = "P-215"             # Lv.4 Blue/Black, Ice-Snow trait (card under test)
FRIGIMON = "BT1-032"         # Lv.4 Blue, Ice-Snow trait (valid placement target)
HYOGAMON = "BT11-026"        # Lv.4 Blue, Ice-Snow trait (second qualifying Digimon)
GOTSUMON_ROCK = "BT2-054"    # Lv.3 Black, Rock trait (valid placement from trash)
FILLER = "ST1-03"            # Lv.3 Red filler (Agumon) -- no qualifying trait
BLUE_LV3 = "ST2-04"          # Lv.3 Blue filler (Gomamon)


def _setup_security(runner, count=3):
    """Inject security cards so the game does not end prematurely."""
    for _ in range(count):
        runner.inject_card(1, FILLER, "security_top")
        runner.inject_card(2, FILLER, "security_top")


@pytest.mark.behavioral
class TestP215TrashSelection:
    """Issue 1: Trash selection must present a player choice, not auto-pick."""

    def test_trash_only_opens_select_trash_phase(self, debug_runner):
        """When only trash has valid candidates (no hand candidates), the
        script must open a SelectTrash phase for the player to choose,
        rather than auto-selecting the first matching card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Put qualifying cards in trash only
        runner.inject_card(1, GOTSUMON_ROCK, "trash")
        runner.inject_card(1, FRIGIMON, "trash")

        # Inject P-215 into hand for playing
        runner.inject_card(1, ICEMON, "hand")

        # Play P-215
        action = runner.find_action("Play Icemon")
        assert action is not None, "Should be able to play Icemon"
        runner.execute(action)

        # Navigate through phases, accepting the optional effect, then
        # verify we reach SelectTrash
        found_trash_select = False
        for _ in range(10):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.Main:
                break
            if phase == GamePhase.SelectTrash:
                found_trash_select = True
                # Pick one to continue
                legal = runner.action_mask()
                if legal:
                    runner.execute(legal[0])
                continue
            # For other phases, pick first non-zero (non-pass) action if available
            legal = runner.action_mask()
            if not legal:
                break
            # Prefer non-pass to accept the optional effect
            non_pass = [a for a in legal if a != 0]
            runner.execute(non_pass[0] if non_pass else legal[0])

        assert found_trash_select, (
            "When only trash has qualifying cards, the script must open a "
            "SelectTrash phase for player choice, not auto-pick. "
            f"Current phase: {runner.game.current_phase.name}"
        )

    def test_trash_selection_allows_choice_between_multiple(self, debug_runner):
        """When trash has multiple qualifying cards, the player must be able
        to choose which one to place (not just auto-pick the first)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Put two different qualifying cards in trash
        runner.inject_card(1, GOTSUMON_ROCK, "trash")
        runner.inject_card(1, FRIGIMON, "trash")

        # Inject P-215 into hand for playing
        runner.inject_card(1, ICEMON, "hand")

        # Play P-215
        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # Resolve phases until we hit SelectTrash or Main
        found_trash_select = False
        for _ in range(10):
            if runner.game.game_over:
                break
            if runner.game.current_phase == GamePhase.Main:
                break
            if runner.game.current_phase == GamePhase.SelectTrash:
                # We found the selection phase -- verify there are multiple choices
                legal = runner.action_mask()
                assert len(legal) >= 2, (
                    f"SelectTrash should offer at least 2 choices for 2 qualifying "
                    f"trash cards, got {len(legal)} legal actions"
                )
                found_trash_select = True
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_trash_select, (
            "When only trash has qualifying cards, the script must open a "
            "SelectTrash phase for player choice, not auto-pick. "
            f"Current phase: {runner.game.current_phase.name}"
        )


@pytest.mark.behavioral
class TestP215HandTrashChoice:
    """Issue 2: When both hand and trash have candidates, player picks zone."""

    def test_both_zones_offers_branch_choice(self, debug_runner):
        """When hand AND trash both have qualifying cards, the player must
        be offered a choice between zones (via effect_choose_branch or
        equivalent), not default to hand only."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Put a qualifying card in trash
        runner.inject_card(1, GOTSUMON_ROCK, "trash")

        # Inject P-215 + a qualifying hand card
        runner.inject_card(1, ICEMON, "hand")
        runner.inject_card(1, FRIGIMON, "hand")

        # Play P-215
        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # Navigate through phases to find the branch choice
        found_branch = False
        found_select = False
        for _ in range(10):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.Main:
                break
            if phase == GamePhase.SelectEffectChoice:
                # This is the branch choice between hand and trash
                found_branch = True
                legal = runner.action_mask()
                assert len(legal) == 2, (
                    f"Branch choice should offer exactly 2 options "
                    f"(hand/trash), got {len(legal)}"
                )
                break
            if phase in (GamePhase.SelectHand, GamePhase.SelectTrash):
                found_select = True
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_branch, (
            "When both hand and trash have valid cards, should get a branch "
            f"choice phase (SelectEffectChoice). Found select={found_select}, "
            f"current phase={runner.game.current_phase.name}"
        )


@pytest.mark.behavioral
class TestP215ProtectionTargetSelection:
    """Issue 3: Protection must target exactly 1 chosen Digimon."""

    def test_protection_targets_one_not_all(self, debug_runner):
        """Card says 'their effects can't return 1 of your [trait] Digimon'.
        The player must choose which ONE Digimon gets protection, not all."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Place two Ice-Snow Digimon on field first
        perm1 = runner.place_on_field(1, [FRIGIMON])    # Ice-Snow #1
        perm2 = runner.place_on_field(1, [HYOGAMON])    # Ice-Snow #2

        # Put a qualifying card in hand for the placement cost
        runner.inject_card(1, GOTSUMON_ROCK, "hand")

        # Inject P-215 into hand
        runner.inject_card(1, ICEMON, "hand")

        # Play P-215
        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # Navigate through: optional activation -> hand selection -> target selection
        # We need to find a SelectTarget phase where we pick which Digimon to protect
        found_target_select = False
        for _ in range(15):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.Main:
                break
            if phase == GamePhase.SelectTarget:
                # This should be the protection target selection
                legal = runner.action_mask()
                # Should have multiple choices (at least Frigimon, Hyogamon, and possibly Icemon itself)
                if len(legal) >= 2:
                    found_target_select = True
                    # Select one specific target (pick the first)
                    runner.execute(legal[0])
                    continue
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_target_select, (
            "Should get a SelectTarget phase to choose which Digimon gets "
            f"protection. Current phase: {runner.game.current_phase.name}"
        )

    def test_only_protected_digimon_has_modifiers(self, debug_runner):
        """After selecting 1 Digimon for protection, only that Digimon should
        have the CANNOT_RETURN_TO_HAND / CANNOT_RETURN_TO_DECK /
        IMMUNE_FROM_DE_DIGIVOLVE modifiers -- not the others."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Place two Ice-Snow Digimon on field
        perm1 = runner.place_on_field(1, [FRIGIMON])    # slot 0
        perm2 = runner.place_on_field(1, [HYOGAMON])    # slot 1

        # Qualifying card in hand
        runner.inject_card(1, GOTSUMON_ROCK, "hand")

        # Inject and play P-215
        runner.inject_card(1, ICEMON, "hand")
        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # Auto-resolve everything (this will pick first legal action each time)
        runner.auto_resolve()

        # After full resolution, check modifiers on field Digimon
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        game = runner.game
        player = game.player1

        # Count how many Digimon have the protection modifiers
        protected_count = 0
        for field_perm in player.battle_area:
            if not field_perm.is_digimon:
                continue
            has_protection = game.modifiers.has_modifier(
                field_perm, ModifierType.CANNOT_RETURN_TO_HAND
            )
            if has_protection:
                protected_count += 1

        # Only ONE Digimon should be protected, not all of them
        assert protected_count == 1, (
            f"Exactly 1 Digimon should have protection modifiers, "
            f"but {protected_count} have them. Card says '1 of your' Digimon."
        )


@pytest.mark.behavioral
class TestP215HandOnlyPath:
    """The hand selection path (existing) should work correctly."""

    def test_hand_only_uses_select_hand(self, debug_runner):
        """When only hand has qualifying cards, use effect_select_hand_card."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Qualifying card in hand only (no trash candidates)
        runner.inject_card(1, FRIGIMON, "hand")
        runner.inject_card(1, ICEMON, "hand")

        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # Navigate until SelectHand phase
        found_hand_select = False
        for _ in range(10):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.Main:
                break
            if phase == GamePhase.SelectHand:
                found_hand_select = True
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        assert found_hand_select, (
            "When only hand has qualifying cards, should enter SelectHand phase. "
            f"Current phase: {runner.game.current_phase.name}"
        )


@pytest.mark.behavioral
class TestP215EffectOptional:
    """The effect is optional -- player can decline."""

    def test_effect_can_be_declined(self, debug_runner):
        """The effect uses 'By placing...' which is optional. Player can
        decline and no card is placed, no protection granted."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")
        runner.clear_zone(1, "hand")
        _setup_security(runner)

        # Set up with qualifying cards available
        runner.inject_card(1, FRIGIMON, "hand")
        runner.inject_card(1, ICEMON, "hand")

        before_hand_count = len(runner.game.player1.hand_cards)

        action = runner.find_action("Play Icemon")
        assert action is not None
        runner.execute(action)

        # The effect is optional -- find and use Pass/decline action if offered
        # Look for the optional activation and decline it
        for _ in range(10):
            if runner.game.game_over:
                break
            phase = runner.game.current_phase
            if phase == GamePhase.Main:
                break
            legal = runner.action_mask()
            if not legal:
                break
            # Try to find a "Pass" or "No" action (action 0 is often pass)
            pass_action = runner.find_action("Pass")
            if pass_action is not None and pass_action in legal:
                runner.execute(pass_action)
                continue
            # If 0 is legal and is pass
            if 0 in legal:
                runner.execute(0)
                continue
            runner.execute(legal[0])

        # After declining, Icemon should be on field with just 1 card in stack
        snap = runner.snapshot()
        icemon_slot = None
        for slot in snap.p1_field:
            if slot.card_id == ICEMON:
                icemon_slot = slot
                break

        # If Icemon is on field, verify no extra card was placed
        if icemon_slot is not None:
            assert len(icemon_slot.stack_ids) == 1, (
                f"After declining the optional effect, Icemon should have only "
                f"itself in the stack (1 card), got {len(icemon_slot.stack_ids)}"
            )


@pytest.mark.behavioral
class TestP215InheritedBlocker:
    """Inherited effect: Blocker."""

    def test_inherited_blocker_flag(self, debug_runner):
        """The inherited effect should set _is_blocker = True."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(ICEMON)
        effects = cs.effect_list(None)
        blocker_effects = [e for e in effects if e.is_inherited_effect and e._is_blocker]
        assert len(blocker_effects) >= 1, (
            "P-215 should have an inherited Blocker effect"
        )
