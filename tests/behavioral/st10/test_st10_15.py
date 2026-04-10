"""Behavioral tests for ST10-15 Darkness Wave (Option, Purple, Cost 1).

Card text:
[Main] Trash the top 3 cards of your deck. Then, if you have a yellow
    Digimon in play, return 1 yellow or purple Digimon card from your
    trash to your hand.
[Security] Activate this card's [Main] effects.

Key cards used in tests:
- ST10-02 Salamon (Lv3, Yellow, Digimon) - yellow Digimon for field condition
- ST10-05 Angewomon (Lv5, Yellow, Digimon) - yellow Digimon for trash recovery
- ST10-04 Gatomon (Lv4, Yellow+Purple, Digimon) - multi-color field/recovery
- ST10-07 Ghostmon (Lv3, Purple, Digimon) - purple Digimon for color req + recovery
- ST10-08 Tsukaimon (Lv3, Purple, Digimon) - purple Digimon for recovery
- ST1-03 Agumon (Lv3, Red, Digimon) - non-matching color (negative test)
- ST10-14 Chaos Degradation (Yellow+Purple, Option) - NOT a Digimon (negative test)

Clause analysis:
1. [Main] Trash the top 3 cards of your deck. (mandatory, always)
2. Then, if you have a yellow Digimon in play... (conditional on yellow Digimon)
3. ...return 1 yellow or purple Digimon card from your trash to your hand.
   (exactly 1, mandatory if qualifying cards exist, yellow OR purple, must be Digimon)
4. [Security] Activate this card's [Main] effects. (reuses Main logic)
"""

import pytest

from digimon_gym.engine.data.enums import CardColor


# === Card IDs ===
DARKNESS_WAVE = "ST10-15"       # Card under test (Purple Option, cost 1)
SALAMON = "ST10-02"             # Lv3 Yellow Digimon
ANGEWOMON = "ST10-05"           # Lv5 Yellow Digimon
GATOMON = "ST10-04"             # Lv4 Yellow+Purple Digimon
GHOSTMON = "ST10-07"            # Lv3 Purple Digimon
TSUKAIMON = "ST10-08"           # Lv3 Purple Digimon
AGUMON = "ST1-03"               # Lv3 Red Digimon (no match)
CHAOS_DEGRADATION = "ST10-14"   # Yellow+Purple Option (NOT Digimon)


def _build_deck(extra_cards=None):
    """Build a minimal 50-card deck using Purple filler."""
    deck = []
    if extra_cards:
        deck.extend(extra_cards)
    while len(deck) < 50:
        deck.append(GHOSTMON)
    return deck


def _find_play_option(runner):
    """Find the action to play ST10-15 from hand."""
    action = runner.find_action("Darkness Wave")
    if action is None:
        action = runner.find_action("ST10-15")
    return action


@pytest.mark.behavioral
class TestST10_15_TrashFromDeck:
    """Clause 1: Trash the top 3 cards of your deck (mandatory, always)."""

    def test_trashes_exactly_3_cards(self, debug_runner):
        """Playing Darkness Wave should trash exactly 3 cards from deck top."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Need a Purple Digimon on field for option color requirement
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack specific cards on top of library so we can verify they are trashed
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, SALAMON, "library_top")
        runner.inject_card(1, TSUKAIMON, "library_top")

        lib_size_before = len(player.library_cards)
        trash_size_before = len(player.trash_cards)

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # Library should have 3 fewer cards (the top 3 were trashed)
        # Plus the option card itself goes to trash after use
        assert len(player.library_cards) == lib_size_before - 3, (
            f"Library should lose exactly 3 cards. "
            f"Before: {lib_size_before}, After: {len(player.library_cards)}"
        )

        # The 3 milled cards should be in trash
        trash_ids = [c.card_id for c in player.trash_cards]
        assert TSUKAIMON in trash_ids, f"Top card (Tsukaimon) should be trashed. Trash: {trash_ids}"
        assert SALAMON in trash_ids, f"2nd card (Salamon) should be trashed. Trash: {trash_ids}"
        assert AGUMON in trash_ids, f"3rd card (Agumon) should be trashed. Trash: {trash_ids}"

    def test_trashes_even_without_yellow_digimon_on_field(self, debug_runner):
        """Trashing is mandatory even if there's no yellow Digimon on field."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Purple Digimon on field (NOT yellow) - meets option color req
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack 3 specific cards on library top
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        lib_size_before = len(player.library_cards)

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # 3 cards should still be trashed even though there's no yellow Digimon
        assert len(player.library_cards) == lib_size_before - 3, (
            f"Should trash 3 even without yellow Digimon. "
            f"Before: {lib_size_before}, After: {len(player.library_cards)}"
        )

    def test_trashes_fewer_if_deck_has_less_than_3(self, debug_runner):
        """If deck has fewer than 3 cards, trash whatever is available."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Reduce library to just 1 card
        while len(player.library_cards) > 1:
            player.library_cards.pop()

        lib_size_before = len(player.library_cards)
        assert lib_size_before == 1

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # Should trash just the 1 available card (not error out)
        assert len(player.library_cards) == 0, (
            f"Should trash whatever is available. "
            f"Before: {lib_size_before}, After: {len(player.library_cards)}"
        )


@pytest.mark.behavioral
class TestST10_15_YellowDigimonCondition:
    """Clause 2: Recovery is conditional on having a yellow Digimon in play."""

    def test_no_recovery_without_yellow_digimon(self, debug_runner):
        """No recovery step if no yellow Digimon is on the field."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Only Purple Digimon on field (no yellow)
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Put a purple Digimon on top of library (would be eligible for recovery)
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, TSUKAIMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Hand should be empty -- no recovery happened
        hand_ids = [c.card_id for c in player.hand_cards]
        assert len(hand_ids) == 0, (
            f"No recovery without yellow Digimon on field. Hand: {hand_ids}"
        )

    def test_recovery_with_yellow_digimon_on_field(self, debug_runner):
        """Recovery happens when a yellow Digimon is on the field."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Yellow Digimon on field satisfies condition
        runner.place_on_field(1, [SALAMON])
        # Also need a Purple perm for option color requirement
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Put a purple Digimon on library top so it gets trashed and is recoverable
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # Tsukaimon (purple Digimon) was trashed, then should be recovered
        hand_ids = [c.card_id for c in player.hand_cards]
        assert TSUKAIMON in hand_ids, (
            f"Purple Digimon should be recovered when yellow Digimon on field. "
            f"Hand: {hand_ids}"
        )

    def test_yellow_purple_digimon_satisfies_condition(self, debug_runner):
        """A Yellow+Purple Digimon (e.g. Gatomon) satisfies the yellow condition."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Gatomon is Yellow+Purple, satisfies both option color req and yellow condition
        runner.place_on_field(1, [GATOMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack a recoverable purple Digimon on library top
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert TSUKAIMON in hand_ids, (
            f"Yellow+Purple Digimon should satisfy yellow condition. Hand: {hand_ids}"
        )


@pytest.mark.behavioral
class TestST10_15_TrashRecoveryFilter:
    """Clause 3: Return 1 yellow or purple Digimon card from trash to hand."""

    def test_recover_yellow_digimon_from_trash(self, debug_runner):
        """Can recover a yellow Digimon from trash."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Put Angewomon (yellow Digimon) on library top to be trashed
        runner.inject_card(1, ANGEWOMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert ANGEWOMON in hand_ids, (
            f"Yellow Digimon should be recoverable from trash. Hand: {hand_ids}"
        )

    def test_recover_purple_digimon_from_trash(self, debug_runner):
        """Can recover a purple Digimon from trash."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack purple Digimon on top (will be trashed then recovered)
        runner.inject_card(1, GHOSTMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert GHOSTMON in hand_ids, (
            f"Purple Digimon should be recoverable from trash. Hand: {hand_ids}"
        )

    def test_cannot_recover_non_digimon(self, debug_runner):
        """Option cards in trash (even yellow/purple) should NOT be recoverable."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Pre-place a yellow+purple Option in trash (not Digimon, should NOT be recoverable)
        runner.inject_card(1, CHAOS_DEGRADATION, "trash")

        # Stack only non-matching cards on library (Red Digimon, not yellow/purple)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Chaos Degradation is an Option, not a Digimon -- should NOT be recovered
        hand_ids = [c.card_id for c in player.hand_cards]
        assert CHAOS_DEGRADATION not in hand_ids, (
            f"Non-Digimon cards should NOT be recoverable. Hand: {hand_ids}"
        )

    def test_cannot_recover_non_yellow_non_purple_digimon(self, debug_runner):
        """Red Digimon in trash should NOT be recoverable."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Pre-place a Red Digimon in trash
        runner.inject_card(1, AGUMON, "trash")

        # Stack only non-matching cards on library
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Red Digimon should NOT be recovered
        hand_ids = [c.card_id for c in player.hand_cards]
        assert AGUMON not in hand_ids, (
            f"Red Digimon should NOT be recoverable (only yellow/purple). Hand: {hand_ids}"
        )

    def test_no_recovery_if_no_qualifying_trash(self, debug_runner):
        """If no yellow/purple Digimon in trash, no recovery phase at all."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack only Red Digimon (no yellow/purple Digimon in deck top)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        # Clear trash to ensure no pre-existing qualifying cards
        runner.clear_zone(1, "trash")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Hand should be empty -- no qualifying cards to recover
        hand_ids = [c.card_id for c in player.hand_cards]
        assert len(hand_ids) == 0, (
            f"No recovery when trash has no qualifying cards. Hand: {hand_ids}"
        )

    def test_can_recover_pre_existing_trash_card(self, debug_runner):
        """Cards already in trash before milling are also eligible for recovery."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Pre-place a purple Digimon in trash BEFORE playing the option
        runner.inject_card(1, TSUKAIMON, "trash")

        # Stack only Red cards on deck top (these won't qualify for recovery)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # The pre-existing Tsukaimon in trash should be recoverable
        hand_ids = [c.card_id for c in player.hand_cards]
        assert TSUKAIMON in hand_ids, (
            f"Pre-existing trash cards should also be eligible. Hand: {hand_ids}"
        )

    def test_returns_exactly_1_card(self, debug_runner):
        """Even with multiple qualifying cards in trash, only 1 is returned."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack 3 purple Digimon on library top (all will be trashed, all qualify)
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, TSUKAIMON, "library_top")

        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Should get exactly 1 card back in hand (not 2 or 3)
        hand_ids = [c.card_id for c in player.hand_cards]
        assert len(hand_ids) == 1, (
            f"Should return exactly 1 card to hand. Got {len(hand_ids)}: {hand_ids}"
        )


@pytest.mark.behavioral
class TestST10_15_SecurityEffect:
    """Clause 4: [Security] activates the [Main] effect."""

    def test_security_trashes_3_and_recovers(self, debug_runner):
        """Security effect should trash 3 from deck and allow recovery."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Yellow Digimon on field for recovery condition
        runner.place_on_field(1, [SALAMON])
        runner.place_on_field(1, [GHOSTMON])

        # Stack a recoverable Digimon on library top
        runner.inject_card(1, TSUKAIMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        lib_size_before = len(player.library_cards)

        # Simulate security trigger: get the script via CardDatabase and invoke
        # the security effect's process callback directly
        from digimon_gym.engine.data.card_database import CardDatabase

        db = CardDatabase()
        card_source = db.create_card_source(DARKNESS_WAVE, player)
        script = db.get_script(DARKNESS_WAVE)
        assert script is not None, "Script for ST10-15 should be loaded"

        effects = script.get_card_effects(card_source)

        # Find the SecuritySkill effect
        security_effects = [e for e in effects if e.is_security_effect]
        assert len(security_effects) == 1, (
            f"Should have exactly 1 security effect. Got {len(security_effects)}"
        )

        sec_effect = security_effects[0]
        ctx = {'player': player, 'game': game, 'card': card_source}
        sec_effect.on_process_callback(ctx)

        # Should have trashed 3 cards
        assert len(player.library_cards) == lib_size_before - 3, (
            f"Security effect should trash 3 cards. "
            f"Before: {lib_size_before}, After: {len(player.library_cards)}"
        )

        # Selection phase should be pending if there's a qualifying card
        # (auto_resolve will pick it)
        runner.auto_resolve()

        hand_ids = [c.card_id for c in player.hand_cards]
        assert TSUKAIMON in hand_ids, (
            f"Security effect should also allow recovery. Hand: {hand_ids}"
        )


@pytest.mark.behavioral
class TestST10_15_MemoryCost:
    """Verify option costs 1 memory to play."""

    def test_costs_1_memory(self, debug_runner):
        """Playing Darkness Wave should cost 1 memory."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        # Stack deck top (no yellow digimon on field, so no recovery)
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")
        runner.inject_card(1, AGUMON, "library_top")

        mem_before = game.memory
        action = _find_play_option(runner)
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        mem_after = game.memory
        cost_paid = mem_before - mem_after
        assert cost_paid == 1, (
            f"Darkness Wave should cost 1 memory. "
            f"Before: {mem_before}, After: {mem_after}, Paid: {cost_paid}"
        )


@pytest.mark.behavioral
class TestST10_15_OptionColorRequirement:
    """Verify option color requirement (Purple)."""

    def test_cannot_play_without_purple_on_field(self, debug_runner):
        """Cannot play Purple option without Purple Digimon/Tamer on field."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Only a Yellow Digimon on field (no Purple for option color req)
        runner.place_on_field(1, [SALAMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        action = _find_play_option(runner)
        assert action is None, (
            f"Should NOT be able to play Purple option without Purple on field. "
            f"Actions: {runner.actions()}"
        )

    def test_can_play_with_purple_on_field(self, debug_runner):
        """Can play Purple option when Purple Digimon is on field."""
        runner = debug_runner(
            deck1=_build_deck(), deck2=_build_deck(), initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, [GHOSTMON])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, DARKNESS_WAVE, "hand")

        action = _find_play_option(runner)
        assert action is not None, (
            f"Should be able to play with Purple Digimon on field. "
            f"Actions: {runner.actions()}"
        )
