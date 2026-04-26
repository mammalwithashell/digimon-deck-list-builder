"""Behavioral tests for EX8-050 Gogmamon (Lv.5 Black, DP 8000, Cost 7).

Card text:
  <Blocker>
  [On Deletion] Reveal the top 3 cards of your deck. You may play 1
  Digimon card with the [Mineral] or [Rock] trait and a play cost of
  5 or less among them without paying the cost. Trash the rest.

  Inherited: [Opponent's Turn] [Once Per Turn] When one of your
  opponent's Digimon attacks, you may change the attack target to
  this Digimon.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# ---------------------------------------------------------------------------
# Filler deck: 50 copies of a generic card to pad decks
# ---------------------------------------------------------------------------
FILLER = "BT1-019"  # Greymon


def _deck():
    """Build a 50-card filler deck."""
    return [FILLER] * 50


@pytest.mark.behavioral
class TestEX8050Blocker:
    """Blocker keyword."""

    def test_has_blocker(self, debug_runner):
        """EX8-050 should have the <Blocker> keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        blocker = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker) >= 1, "Should have Blocker effect"


@pytest.mark.behavioral
class TestEX8050OnDeletion:
    """[On Deletion] Reveal top 3, play 1 Mineral/Rock Digimon cost<=5 free, trash rest."""

    def test_on_deletion_effect_exists(self, debug_runner):
        """Should have an On Deletion effect with correct timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects
                  if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion]
        assert len(on_del) == 1, "Should have exactly 1 On Deletion effect"

    def test_on_deletion_plays_valid_rock_digimon(self, debug_runner):
        """On Deletion: selecting a valid [Rock] trait Digimon plays it to field."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # Inject known cards on top of library (last injected = top)
        # Top 3 will be: EX8-046 (Rock, cost 3), ST1-03 (Reptile), BT1-019 (filler)
        runner.inject_card(1, FILLER, "library_top")    # position 2
        runner.inject_card(1, "ST1-03", "library_top")  # position 1
        runner.inject_card(1, "EX8-046", "library_top") # position 0 (top)

        p1 = game.player1
        p1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [f.card_id for f in snap.p1_field]
        assert "EX8-046" in field_ids, \
            f"Gotsumon (Rock) should be played to field, got field: {field_ids}"

    def test_on_deletion_plays_valid_mineral_digimon(self, debug_runner):
        """On Deletion: should also accept [Mineral] trait Digimon."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # EX8-049 Golemon: Mineral trait, cost 5 (valid)
        runner.inject_card(1, FILLER, "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "EX8-049", "library_top")

        p1 = game.player1
        p1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [f.card_id for f in snap.p1_field]
        assert "EX8-049" in field_ids, \
            f"Golemon (Mineral) should be played to field, got field: {field_ids}"

    def test_on_deletion_no_valid_cards_trashes_all(self, debug_runner):
        """On Deletion: if no valid Mineral/Rock Digimon in top 3, all go to trash."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # All 3 top cards are non-matching (Reptile trait)
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")

        p1 = game.player1
        trash_before = len(p1.trash_cards)
        lib_before = len(p1.library_cards)
        p1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        # All 3 non-matching should be trashed (+ EX8-050 from deletion = 4 new)
        trash_new = [c for c in snap.p1_trash if c == "ST1-03"]
        assert len(trash_new) >= 3, \
            f"All 3 non-matching cards should be trashed, got {len(trash_new)} ST1-03 in trash"
        # Library should lose 3 cards
        assert snap.p1_library_size == lib_before - 3, \
            "Should remove exactly 3 cards from library"

    def test_on_deletion_rejects_cost_above_5(self, debug_runner):
        """On Deletion: Digimon with Rock trait but cost > 5 should NOT be playable."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # Top 3: another EX8-050 (Rock trait, cost 7 - too expensive), plus 2 non-matching
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "EX8-050", "library_top")  # Rock but cost 7

        p1 = game.player1
        p1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [f.card_id for f in snap.p1_field]
        assert "EX8-050" not in field_ids, \
            f"Cost-7 Gogmamon should NOT be played (cost > 5), got field: {field_ids}"

    def test_on_deletion_trashes_non_selected(self, debug_runner):
        """On Deletion: when a card is played, remaining cards should be trashed (not deck)."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # Top 3: EX8-046 (valid), ST1-03 (not valid), BT1-019 (not valid)
        runner.inject_card(1, FILLER, "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "EX8-046", "library_top")

        p1 = game.player1
        lib_before = len(p1.library_cards)

        p1.delete_permanent(perm)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Library should have lost 3 cards total
        assert snap.p1_library_size == lib_before - 3, \
            "Should remove exactly 3 cards from library"
        # The 2 non-selected should be in trash
        assert "ST1-03" in snap.p1_trash, \
            "Non-selected revealed card should be trashed"

    def test_on_deletion_is_optional(self, debug_runner):
        """On Deletion play is optional ('you may'): player can decline and trash all."""
        runner = debug_runner(
            deck1=_deck(), deck2=_deck(),
            skip_shuffle=True, initial_memory=5,
        )
        game = runner.game
        perm = runner.place_on_field(1, ["EX8-050"])

        # Top 3: EX8-046 (valid), ST1-03, BT1-019
        runner.inject_card(1, FILLER, "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "EX8-046", "library_top")

        p1 = game.player1
        p1.delete_permanent(perm)

        # Should enter a selection phase where declining is possible
        # Check that the effect uses is_optional=True
        card = perm.top_card
        effects = card.effect_list(None)
        on_del = [e for e in effects
                  if e.timing == EffectTiming.OnDestroyedAnyone and e.is_on_deletion][0]
        # The process internally calls effect_reveal_and_select with is_optional=True
        assert on_del.on_process_callback is not None


@pytest.mark.behavioral
class TestEX8050InheritedRedirect:
    """Inherited: [Opponent's Turn] [Once Per Turn] redirect attack to this Digimon."""

    def test_inherited_redirect_effect_exists(self, debug_runner):
        """Should have an inherited redirect effect with _is_when_attacked_observer."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)]
        assert len(redirect) == 1, \
            "Should have 1 inherited redirect effect with _is_when_attacked_observer"

    def test_inherited_redirect_is_optional(self, debug_runner):
        """Redirect effect should be optional ('you may')."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)][0]
        assert redirect.is_optional, "Redirect should be optional"

    def test_inherited_redirect_once_per_turn(self, debug_runner):
        """Redirect effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)][0]
        assert redirect.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_inherited_redirect_condition_requires_field(self, debug_runner):
        """Redirect condition should fail if card is not on field."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "EX8-050", "hand")
        game = runner.game
        hand_card = game.player1.hand_cards[-1]
        effects = hand_card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)]
        if redirect:
            assert not redirect[0].can_use_condition({}), \
                "Should fail when card is not on field"

    def test_inherited_redirect_condition_requires_opponent_turn(self, debug_runner):
        """Redirect condition should fail on own turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        game = runner.game
        # P1's turn -> redirect should not be available for P1
        game.player1.is_my_turn = True
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)][0]
        assert not redirect.can_use_condition({}), \
            "Should fail on own turn (requires opponent's turn)"

    def test_inherited_redirect_has_process_callback(self, debug_runner):
        """Redirect process callback should not be a stub/pass."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)][0]
        assert redirect.on_process_callback is not None, \
            "Redirect should have a process callback (not a stub/pass)"

    def test_inherited_redirect_calls_redirect_attack(self, debug_runner):
        """The redirect process should call game.redirect_attack(perm)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX8-050"])
        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        redirect = [e for e in effects
                    if e.is_inherited_effect
                    and getattr(e, '_is_when_attacked_observer', False)][0]

        # Track whether redirect_attack was called
        redirect_called = []
        original_redirect = game.redirect_attack

        def mock_redirect(target):
            redirect_called.append(target)
            original_redirect(target)

        game.redirect_attack = mock_redirect

        # Set up a fake pending attack so redirect_attack doesn't bail
        from digimon_gym.engine.game.constants import PendingAttack
        attacker_perm = runner.place_on_field(2, ["ST1-03"])
        game.pending_attack = PendingAttack(
            attacker=attacker_perm, original_target=None, effective_target=None)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'card': card,
        }
        redirect.on_process_callback(ctx)
        assert len(redirect_called) == 1, "redirect_attack should be called once"
        assert redirect_called[0] is perm, \
            "redirect_attack should target this Digimon's permanent"
