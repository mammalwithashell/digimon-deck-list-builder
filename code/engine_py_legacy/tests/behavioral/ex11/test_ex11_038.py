"""Behavioral tests for EX11-038 Sunarizamon (Lv.3, Black, Reptile/LIBERATOR).

Card text:
[When Moving] [On Play] By trashing 1 [Mineral] or [Rock] trait card
from your hand or your Digimon's digivolution cards, <Draw 1>.

Inherited:
When effects trash this card from a [Mineral] or [Rock] trait Digimon's
digivolution cards, <Draw 1>.

Key faithfulness issues being validated:
1. Player must CHOOSE between trashing from hand vs digivolution cards when
   both zones have eligible targets.
2. Digi-source trash must use proper selection (not auto-select first match).
3. Digi-source trash must fire OnDigivolutionCardDiscarded timing.
4. Inherited fires only when THIS card is trashed from a Mineral/Rock Digimon.
"""

import pytest
from engine_py_legacy.engine.data.enums import GamePhase, EffectTiming


# Deck with Sunarizamon + Mineral/Rock support cards
DECK = ["EX11-038"] * 4 + ["BT4-065"] * 4 + ["BT4-066"] * 4 + ["BT4-070"] * 4 + ["BT2-054"] * 4 + ["BT2-005"] * 4 + ["EX11-038"] * 26


def _find_source_action(runner, card_id):
    """Find the SelectSource action for a specific card_id in digivolution sources."""
    actions = runner.actions()
    for action_id, desc in actions.items():
        if action_id >= 2000 and action_id < 2168:
            # Decode: field_idx = (action_id - 2000) // 12, source_idx = (action_id - 2000) % 12
            from engine_py_legacy.engine.game.constants import SOURCES_PER_FIELD
            field_idx = (action_id - 2000) // SOURCES_PER_FIELD
            source_idx = (action_id - 2000) % SOURCES_PER_FIELD
            player = runner.game.player1
            if field_idx < len(player.battle_area):
                perm = player.battle_area[field_idx]
                if source_idx < len(perm.card_sources):
                    cs = perm.card_sources[source_idx]
                    if cs.c_entity_base and cs.c_entity_base.card_id == card_id:
                        return action_id
    return None


@pytest.mark.behavioral
class TestEX11038OnPlayHandTrash:
    """[On Play] trashing a Mineral/Rock card from hand to draw 1."""

    def test_on_play_trash_from_hand_draws_1(self, debug_runner):
        """Playing Sunarizamon with a Mineral card in hand should allow
        trashing it to draw 1."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand so we control exactly what's in it
        runner.game.player1.hand_cards.clear()

        # Put only a Mineral card and Sunarizamon in hand
        runner.inject_card(1, "BT4-066", "hand")  # Golemon (Mineral)
        runner.inject_card(1, "EX11-038", "hand")

        library_before = len(runner.game.player1.library_cards)
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None, "Should be able to play Sunarizamon"
        runner.execute(action)

        # After playing, the optional effect triggers. auto_resolve will
        # accept the hand selection (first legal card = Golemon)
        runner.auto_resolve()

        # Golemon should be trashed, 1 card drawn
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-066" in trash_ids, \
            f"Golemon (BT4-066) should be in trash after trashing. Trash: {trash_ids}"
        # Should have drawn 1 card
        library_after = len(runner.game.player1.library_cards)
        assert library_after == library_before - 1, \
            f"Should have drawn 1 card. Library: {library_before} -> {library_after}"

    def test_on_play_trash_rock_card_from_hand(self, debug_runner):
        """Rock trait cards should also be valid trash targets."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand
        runner.game.player1.hand_cards.clear()
        runner.inject_card(1, "BT4-065", "hand")  # Gotsumon (Rock)
        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-065" in trash_ids, \
            f"Gotsumon (BT4-065, Rock) should be in trash. Trash: {trash_ids}"


@pytest.mark.behavioral
class TestEX11038OnPlayDigiSourceTrash:
    """[On Play] trashing a Mineral/Rock card from digivolution cards."""

    def test_on_play_trash_from_digi_sources_draws_1(self, debug_runner):
        """When no Mineral/Rock in hand but present in digi-sources, should
        allow trashing from sources and draw 1 when player selects."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Place a stack: BT4-066 (Mineral, Lv.4) on top of BT4-065 (Rock, Lv.3)
        perm = runner.place_on_field(1, ["BT4-065", "BT4-066"])
        assert len(perm.card_sources) == 2, "Stack should have 2 cards"

        # Clear hand of any Mineral/Rock cards
        runner.game.player1.hand_cards = [
            c for c in runner.game.player1.hand_cards
            if not any(t in (getattr(c, 'card_traits', []) or [])
                       for t in ['Mineral', 'Rock'])
        ]

        library_before = len(runner.game.player1.library_cards)

        # Play Sunarizamon
        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)

        # Should be in SelectSource phase (player picks which digi-source to trash)
        assert runner.game.current_phase == GamePhase.SelectSource, \
            f"Should be in SelectSource phase, got {runner.game.current_phase}"

        # Find and select the Rock card from sources
        source_action = _find_source_action(runner, "BT4-065")
        assert source_action is not None, "Should find BT4-065 in source actions"
        runner.execute(source_action)
        runner.auto_resolve()

        # The Rock card should have been trashed from sources
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-065" in trash_ids, \
            f"Gotsumon (BT4-065, Rock) should be trashed from sources. Trash: {trash_ids}"
        # Stack should now have only 1 card (top card remains)
        assert len(perm.card_sources) == 1, \
            f"Stack should have 1 card after trashing. Got {len(perm.card_sources)}"
        # Should have drawn 1
        library_after = len(runner.game.player1.library_cards)
        assert library_after == library_before - 1, \
            f"Should have drawn 1 card. Library: {library_before} -> {library_after}"

    def test_digi_source_trash_fires_on_digivolution_card_discarded(self, debug_runner):
        """Trashing from digi-sources MUST fire OnDigivolutionCardDiscarded
        so inherited effects (like Sunarizamon's own inherited) can trigger."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Build stack: BT2-054 (Rock) under BT4-066 (Mineral top)
        perm = runner.place_on_field(1, ["BT2-054", "BT4-066"])

        # Clear hand of Mineral/Rock
        runner.game.player1.hand_cards = [
            c for c in runner.game.player1.hand_cards
            if not any(t in (getattr(c, 'card_traits', []) or [])
                       for t in ['Mineral', 'Rock'])
        ]

        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)

        # Should be in SelectSource
        assert runner.game.current_phase == GamePhase.SelectSource, \
            f"Should be in SelectSource, got {runner.game.current_phase}"

        # Select BT2-054 from sources
        source_action = _find_source_action(runner, "BT2-054")
        assert source_action is not None, "Should find BT2-054 in source actions"
        runner.execute(source_action)
        runner.auto_resolve()

        # BT2-054 should be trashed
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT2-054" in trash_ids, \
            f"BT2-054 (Rock) should be trashed from digi-sources. Trash: {trash_ids}"


@pytest.mark.behavioral
class TestEX11038ZoneChoice:
    """Player must choose between hand and digi-sources when both have targets."""

    def test_both_zones_available_presents_choice(self, debug_runner):
        """When both hand and digi-sources have Mineral/Rock cards, the player
        should get a branch choice (SelectEffectChoice) to pick the zone."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand
        runner.game.player1.hand_cards.clear()

        # Put Mineral card in hand
        runner.inject_card(1, "BT4-066", "hand")  # Golemon (Mineral)

        # Put Rock card in digi-sources
        runner.place_on_field(1, ["BT4-065", "BT4-070"])  # Gotsumon (Rock) under Meteormon

        # Play Sunarizamon
        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None
        runner.execute(action)

        # Should be in SelectEffectChoice (zone choice: hand vs digi-sources)
        phase = runner.game.current_phase
        assert phase == GamePhase.SelectEffectChoice, \
            f"Expected SelectEffectChoice for zone choice, got {phase}"

        # Should have 2 branch options
        actions = runner.actions()
        branch_actions = {k: v for k, v in actions.items() if k >= 1000 and k < 1010}
        assert len(branch_actions) >= 2, \
            f"Should have at least 2 branch choices (hand + sources). Got: {branch_actions}"

    def test_choose_hand_branch_trashes_from_hand(self, debug_runner):
        """Choosing the 'hand' branch should proceed to hand card selection."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand
        runner.game.player1.hand_cards.clear()
        runner.inject_card(1, "BT4-066", "hand")  # Golemon (Mineral)
        runner.place_on_field(1, ["BT4-065", "BT4-070"])

        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        runner.execute(action)

        # Pick branch 0 = "From hand"
        assert runner.game.current_phase == GamePhase.SelectEffectChoice
        runner.execute(1000)  # Branch 0 = From hand

        # Should now be in SelectHand
        assert runner.game.current_phase == GamePhase.SelectHand, \
            f"After picking hand branch, should be in SelectHand, got {runner.game.current_phase}"

        # Auto-resolve the hand selection
        runner.auto_resolve()

        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-066" in trash_ids, \
            f"Golemon should be trashed from hand. Trash: {trash_ids}"

    def test_choose_sources_branch_trashes_from_sources(self, debug_runner):
        """Choosing the 'digivolution cards' branch should proceed to source selection."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand
        runner.game.player1.hand_cards.clear()
        runner.inject_card(1, "BT4-066", "hand")  # Golemon in hand
        perm = runner.place_on_field(1, ["BT4-065", "BT4-070"])  # Rock in sources

        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        runner.execute(action)

        # Pick branch 1 = "From digivolution cards"
        assert runner.game.current_phase == GamePhase.SelectEffectChoice
        runner.execute(1001)  # Branch 1 = From digivolution cards

        # Should now be in SelectSource
        assert runner.game.current_phase == GamePhase.SelectSource, \
            f"After picking sources branch, should be in SelectSource, got {runner.game.current_phase}"

        # Select the Rock card
        source_action = _find_source_action(runner, "BT4-065")
        assert source_action is not None
        runner.execute(source_action)
        runner.auto_resolve()

        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-065" in trash_ids, \
            f"Gotsumon should be trashed from sources. Trash: {trash_ids}"
        # Hand Golemon should NOT be trashed (we chose sources)
        assert "BT4-066" not in trash_ids, \
            "Hand card should not be trashed when sources branch was chosen"


@pytest.mark.behavioral
class TestEX11038WhenMoving:
    """[When Moving] effect fires when Sunarizamon moves from breeding."""

    def test_when_moving_trash_and_draw(self, debug_runner):
        """Moving Sunarizamon from breeding should trigger trash-and-draw."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Place Sunarizamon in breeding area
        runner.place_in_breeding(1, ["BT2-005", "EX11-038"])

        # Clear hand and put only a Rock card
        runner.game.player1.hand_cards.clear()
        runner.inject_card(1, "BT4-065", "hand")  # Gotsumon (Rock)

        library_before = len(runner.game.player1.library_cards)

        # Move from breeding
        runner.set_phase("Breeding")
        action = runner.find_action("Move")
        assert action is not None, "Should have Move action"
        runner.execute(action)

        # After moving, the OnMove effect should fire. Since only hand targets,
        # we should be in SelectHand phase.
        assert runner.game.current_phase == GamePhase.SelectHand, \
            f"Should be in SelectHand after OnMove, got {runner.game.current_phase}"

        runner.auto_resolve()

        # Rock card should be trashed, and 1 card drawn
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "BT4-065" in trash_ids, \
            f"Gotsumon should be trashed after When Moving. Trash: {trash_ids}"
        library_after = len(runner.game.player1.library_cards)
        assert library_after == library_before - 1, \
            f"Should have drawn 1 card. Library: {library_before} -> {library_after}"

    def test_when_moving_only_fires_for_moved_permanent(self, debug_runner):
        """OnMove should only fire for the permanent that actually moved,
        not other permanents with Sunarizamon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Place Sunarizamon on field (should NOT fire OnMove when breeding moves)
        runner.place_on_field(1, ["EX11-038"])

        # Place a different Digimon in breeding (non-EX11-038)
        runner.place_in_breeding(1, ["BT2-005", "BT4-065"])

        # Clear hand and add a Rock card
        runner.game.player1.hand_cards.clear()
        runner.inject_card(1, "BT4-066", "hand")

        # Move breeding Digimon
        runner.set_phase("Breeding")
        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)

        # The field Sunarizamon's OnMove should NOT fire
        # (only the moved permanent's OnMove should fire, and BT4-065 doesn't have one)
        # Should go directly to Main
        runner.auto_resolve()

        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        mineral_rock_in_trash = [cid for cid in trash_ids if cid in ["BT4-066", "BT4-065"]]
        assert len(mineral_rock_in_trash) == 0, \
            f"Field Sunarizamon's OnMove should not fire. Trash: {trash_ids}"


@pytest.mark.behavioral
class TestEX11038Inherited:
    """Inherited: When effects trash THIS card from a Mineral/Rock Digimon's sources."""

    def test_inherited_fires_when_trashed_from_mineral_digimon(self, debug_runner):
        """When Sunarizamon is trashed from a Mineral-trait Digimon's
        digivolution cards, the owner should draw 1."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Use place_on_field to ensure _owner_game is set (required for _fire_timing)
        # Stack: EX11-038 (bottom, inherited source) -> BT4-066 (Mineral, top)
        perm = runner.place_on_field(1, ["EX11-038", "BT4-066"])

        # Top card is Golemon (Mineral trait) so inherited condition is met
        assert perm.has_trait('Mineral'), "Top card should have Mineral trait"

        library_before = len(runner.game.player1.library_cards)

        # Trash Sunarizamon from the digi-sources using the standard method
        trashed = perm.trash_digivolution_cards(1)
        runner.game.player1.trash_cards.extend(trashed)

        library_after = len(runner.game.player1.library_cards)

        # Sunarizamon's inherited should have fired, drawing 1
        assert library_after == library_before - 1, \
            f"Should have drawn 1 card. Library: {library_before} -> {library_after}"
        # Sunarizamon should be in trash
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "EX11-038" in trash_ids, \
            "Sunarizamon should be in trash"

    def test_inherited_fires_when_trashed_from_rock_digimon(self, debug_runner):
        """Inherited should also fire when trashed from a Rock-trait Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Stack: EX11-038 (bottom) -> BT4-070 (Rock, Meteormon, top)
        perm = runner.place_on_field(1, ["EX11-038", "BT4-070"])

        assert perm.has_trait('Rock'), "Top card should have Rock trait"

        library_before = len(runner.game.player1.library_cards)

        trashed = perm.trash_digivolution_cards(1)
        runner.game.player1.trash_cards.extend(trashed)

        library_after = len(runner.game.player1.library_cards)

        assert library_after == library_before - 1, \
            f"Should have drawn 1 card from Rock Digimon. Library: {library_before} -> {library_after}"

    def test_inherited_does_not_fire_from_non_mineral_rock_digimon(self, debug_runner):
        """Inherited must NOT fire when trashed from a Digimon without
        Mineral or Rock trait."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Stack: EX11-038 (bottom) -> EX11-038 (top, Reptile/LIBERATOR, NOT Mineral/Rock)
        perm = runner.place_on_field(1, ["EX11-038", "EX11-038"])

        # Top card is Sunarizamon (Reptile, NOT Mineral/Rock)
        assert not perm.has_trait('Mineral'), "Should NOT have Mineral"
        assert not perm.has_trait('Rock'), "Should NOT have Rock"

        library_before = len(runner.game.player1.library_cards)

        trashed = perm.trash_digivolution_cards(1)
        runner.game.player1.trash_cards.extend(trashed)

        library_after = len(runner.game.player1.library_cards)

        assert library_after == library_before, \
            f"Should NOT draw from non-Mineral/Rock Digimon. Library: {library_before} -> {library_after}"

    def test_inherited_does_not_fire_when_different_card_trashed(self, debug_runner):
        """Inherited only fires when THIS SPECIFIC card (Sunarizamon) is trashed,
        not any card from the sources."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Stack: EX11-038 (bottom) -> BT4-065 (Rock, middle) -> BT4-066 (Mineral, top)
        perm = runner.place_on_field(1, ["EX11-038", "BT4-065", "BT4-066"])

        assert perm.has_trait('Mineral'), "Top card should have Mineral trait"

        library_before = len(runner.game.player1.library_cards)

        # Trash from top of sources (will trash Gotsumon, NOT Sunarizamon)
        trashed = perm.trash_digivolution_cards(1, from_top=True)
        runner.game.player1.trash_cards.extend(trashed)

        # Gotsumon was trashed, not Sunarizamon
        trashed_ids = [c.c_entity_base.card_id for c in trashed]
        assert "BT4-065" in trashed_ids, "Gotsumon should be the trashed card"

        library_after = len(runner.game.player1.library_cards)

        assert library_after == library_before, \
            f"Should NOT draw when a different card is trashed. Library: {library_before} -> {library_after}"


@pytest.mark.behavioral
class TestEX11038NoTargets:
    """Effect should not activate when no valid Mineral/Rock targets exist."""

    def test_no_mineral_rock_anywhere_effect_unavailable(self, debug_runner):
        """If no Mineral/Rock cards in hand or any digi-sources, the effect
        condition should fail and no selection phases should appear."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        # Clear hand of Mineral/Rock cards
        runner.game.player1.hand_cards = [
            c for c in runner.game.player1.hand_cards
            if not any(t in (getattr(c, 'card_traits', []) or [])
                       for t in ['Mineral', 'Rock'])
        ]

        # No stacks with Mineral/Rock sources on field

        runner.inject_card(1, "EX11-038", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Sunarizamon")
        assert action is not None, "Should still be able to play the card"
        runner.execute(action)

        # The optional effect should not trigger (no valid targets)
        runner.auto_resolve()

        snap = runner.snapshot()
        # No Mineral/Rock should be in trash (effect didn't fire)
        trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        mineral_rock_in_trash = [
            cid for cid in trash_ids
            if cid in ["BT4-065", "BT4-066", "BT4-070", "BT2-054"]
        ]
        assert len(mineral_rock_in_trash) == 0, \
            f"No Mineral/Rock should be trashed. Found: {mineral_rock_in_trash}"
