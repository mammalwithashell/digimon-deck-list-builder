"""Behavioral tests for EX10-051 Mummymon (Lv.5 Purple/Black Digimon, Undead).

Card text:
    [On Play] By trashing 1 card in your hand, <De-Digivolve 1> 1 of your
        opponent's Digimon.
    [On Deletion] You may play 1 Tamer card with [Myotismon] in its text
        from your trash without paying the cost. This effect can't play
        cards with the same name as any of your Tamers.

Key cards used in test setups:
- EX10-051 Mummymon         Lv.5 Purple/Black, cost 6, DP 6000 (the card under test)
- BT1-009 Agumon            Lv.3 Red, for placing opponent digimon targets
- BT1-015 Greymon           Lv.4 Red, for building opponent digi-stacks
- AD1-004 WarGreymon        Lv.6 Red, for building opponent digi-stacks
- BT8-093 Yukio Oikawa      Tamer, "Myotismon" in text (valid C2 candidate)
- EX10-065 Yukio Oikawa     Tamer, "Myotismon" in text (same-name as BT8-093)
- BT16-089 Arukenimon & Mummymon  Tamer, "Myotismon" in text (valid C2 candidate)
- BT2-089 Tai Kamiya        Tamer, NO Myotismon (non-matching control)
- ST1-03 Agumon             Lv.3 Red Digimon (simple neutral body)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


MUMMYMON = "EX10-051"
YUKIO_OIKAWA_BT8 = "BT8-093"
YUKIO_OIKAWA_EX10 = "EX10-065"
ARUKENIMON_MUMMYMON_TAMER = "BT16-089"
TAI_KAMIYA = "BT2-089"


def _get_effects(card):
    """Return all effects defined by EX10-051's script."""
    return list(card.effect_list(EffectTiming.NoTiming))


def _find_c1_on_play(card):
    for eff in _get_effects(card):
        if eff.is_on_play and "EX10-051" in (eff.effect_name or ""):
            return eff
    return None


def _find_c2_on_deletion(card):
    for eff in _get_effects(card):
        if eff.is_on_deletion and "EX10-051" in (eff.effect_name or ""):
            return eff
    return None


# ─────────────────────────────────────────────────────────────────
# Effect-structure tests (no gameplay, just script shape)
# ─────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestEX10051ScriptStructure:
    def test_has_c1_on_play_effect(self, debug_runner):
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        eff = _find_c1_on_play(perm.top_card)
        assert eff is not None, "Should have C1 [On Play] effect"
        assert eff.is_on_play is True
        assert eff.timing == EffectTiming.OnEnterFieldAnyone

    def test_c1_is_optional_at_effect_level(self, debug_runner):
        """C1 must be optional so the agent can choose not to pay the by-cost."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        eff = _find_c1_on_play(perm.top_card)
        assert eff is not None
        assert eff.is_optional is True, (
            "C1 must be optional — 'by trashing' is a voluntary by-cost")

    def test_has_c2_on_deletion_effect(self, debug_runner):
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        eff = _find_c2_on_deletion(perm.top_card)
        assert eff is not None, "Should have C2 [On Deletion] effect"
        assert eff.is_on_deletion is True
        assert eff.timing == EffectTiming.OnDestroyedAnyone

    def test_c2_is_optional(self, debug_runner):
        """C2 text says 'You may play', which is explicitly optional."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        eff = _find_c2_on_deletion(perm.top_card)
        assert eff is not None
        assert eff.is_optional is True


# ─────────────────────────────────────────────────────────────────
# C1 [On Play] tests
# ─────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestEX10051OnPlayDeDigivolve:
    """C1: By trashing 1 hand card, De-Digivolve 1 opp Digimon."""

    def _setup_play_mummymon(self, debug_runner, opp_stack, p1_hand_filler=None,
                              initial_memory=6):
        """Build a runner with Mummymon in hand for player 1 and a stacked
        opponent Digimon on player 2's field."""
        runner = debug_runner(initial_memory=initial_memory)
        # Opponent has a stacked Digimon so de-digivolve has a target and room
        runner.place_on_field(2, opp_stack)
        # Player 1 has Mummymon + some filler hand card to trash
        runner.inject_card(1, MUMMYMON, "hand")
        filler_id = p1_hand_filler or "ST1-03"
        runner.inject_card(1, filler_id, "hand")
        runner.set_phase("Main")
        return runner

    def test_on_play_creates_hand_selection_phase(self, debug_runner):
        """Playing Mummymon while an opponent Digimon exists must first prompt
        the player to select a hand card to trash (the by-cost)."""
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015", "AD1-004"])
        play = runner.find_action("Play Mummymon")
        assert play is not None, (
            f"Should be able to play Mummymon. Actions: {runner.actions()}")
        runner.execute(play)

        # Should now be selecting a hand card (the by-cost)
        game = runner.game
        assert game.pending_selection is not None or game.current_phase == GamePhase.SelectHand, (
            f"Should enter SelectHand after Play. Phase: {game.current_phase}")
        if game.pending_selection:
            assert game.current_phase == GamePhase.SelectHand

    def test_on_play_trashes_selected_hand_card(self, debug_runner):
        """Player picks a hand card; that specific card ends up in trash."""
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015", "AD1-004"])
        p1 = runner.game.player1

        play = runner.find_action("Play Mummymon")
        runner.execute(play)

        # Pick the first legal hand index (the non-Mummymon filler)
        game = runner.game
        assert game.current_phase == GamePhase.SelectHand, (
            f"Phase should be SelectHand after Play. Got: {game.current_phase}")
        assert game.pending_selection is not None
        valid = list(game.pending_selection.valid_indices)
        assert len(valid) >= 1, "Should offer at least one hand card to trash"

        # Track the target card (filler ST1-03 is at some index)
        chosen_idx = valid[0]
        chosen_card = p1.hand_cards[chosen_idx]  # SEL_HAND_START == 0
        chosen_card_id = chosen_card.c_entity_base.card_id

        game.decode_action(chosen_idx, p1.player_id)

        # After trashing, the chosen card should be in trash
        trash_ids = [c.c_entity_base.card_id for c in p1.trash_cards]
        assert chosen_card_id in trash_ids, (
            f"Chosen card {chosen_card_id} should be in trash. Trash: {trash_ids}")

    def test_on_play_then_opens_opponent_select_phase(self, debug_runner):
        """After trashing the hand card, the player selects an opponent Digimon."""
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015", "AD1-004"])

        play = runner.find_action("Play Mummymon")
        runner.execute(play)
        game = runner.game

        # Resolve the SelectHand step
        assert game.current_phase == GamePhase.SelectHand
        hand_valid = list(game.pending_selection.valid_indices)
        game.decode_action(hand_valid[0], game.player1.player_id)

        # Now should be in SelectTarget selecting an opponent permanent
        assert game.current_phase == GamePhase.SelectTarget, (
            f"After trashing, should enter SelectTarget. "
            f"Phase: {game.current_phase}")
        assert game.pending_selection is not None
        # Should include the opponent stacked digimon
        assert len(game.pending_selection.valid_indices) >= 1

    def test_on_play_full_flow_removes_top_digivolution_card(self, debug_runner):
        """Full flow: play Mummymon, trash hand, pick opp digimon, de-digivolve 1.
        De-Digivolve 1 pops the TOP card of the opponent stack (down to Lv.3 floor).
        """
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015", "AD1-004"])  # Greymon(Lv4) -> WarGreymon(Lv6)
        p2 = runner.game.player2

        # Record initial stack — top is WarGreymon (AD1-004)
        opp_perm = p2.battle_area[0]
        assert opp_perm.top_card.c_entity_base.card_id == "AD1-004"
        initial_stack_size = len(opp_perm.card_sources)
        assert initial_stack_size == 2

        play = runner.find_action("Play Mummymon")
        runner.execute(play)
        game = runner.game

        # Step 1: pick a hand card
        game.decode_action(
            list(game.pending_selection.valid_indices)[0],
            game.player1.player_id)

        # Step 2: pick the opponent digimon target
        assert game.current_phase == GamePhase.SelectTarget
        target_action = list(game.pending_selection.valid_indices)[0]
        game.decode_action(target_action, game.player1.player_id)

        # Stack should have lost its top (WarGreymon) which now goes to p2 trash.
        # Remaining top should be Greymon (Lv.4, BT1-015)
        assert len(opp_perm.card_sources) == initial_stack_size - 1, (
            f"De-Digivolve 1 should remove 1 card. "
            f"Got sources: {[c.c_entity_base.card_id for c in opp_perm.card_sources]}")
        assert opp_perm.top_card.c_entity_base.card_id == "BT1-015", (
            f"New top should be Greymon (Lv.4) after de-digi. "
            f"Got: {opp_perm.top_card.c_entity_base.card_id}")
        # Removed card should be in OPPONENT's trash, not player's
        p2_trash_ids = [c.c_entity_base.card_id for c in p2.trash_cards]
        assert "AD1-004" in p2_trash_ids, (
            f"De-digivolved card should go to opponent trash. p2 trash: {p2_trash_ids}")
        p1_trash_ids = [c.c_entity_base.card_id for c in runner.game.player1.trash_cards]
        assert "AD1-004" not in p1_trash_ids, (
            "De-digivolved card must not go to player 1 trash")

    def test_on_play_does_not_de_digivolve_past_level_3(self, debug_runner):
        """De-Digivolve 1 must stop if it would go below Lv.3 (engine-enforced floor).
        If opp has a Lv.4 with no sources (just the top), de-digi cannot proceed —
        removing the top would leave nothing, so it doesn't pop."""
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015"])  # flat Lv.4, no sources under
        p2 = runner.game.player2
        opp_perm = p2.battle_area[0]
        assert len(opp_perm.card_sources) == 1

        play = runner.find_action("Play Mummymon")
        runner.execute(play)
        game = runner.game

        # Pick hand
        game.decode_action(
            list(game.pending_selection.valid_indices)[0],
            game.player1.player_id)

        # Pick target
        assert game.current_phase == GamePhase.SelectTarget
        game.decode_action(
            list(game.pending_selection.valid_indices)[0],
            game.player1.player_id)

        # Stack must still be 1 card — can't go past Lv.3 floor, and single-card
        # stacks cannot be de-digivolved at all.
        assert len(opp_perm.card_sources) == 1, (
            f"Flat Lv.4 cannot be de-digivolved. Sources: "
            f"{[c.c_entity_base.card_id for c in opp_perm.card_sources]}")
        # Nothing should be trashed
        p2_trash_ids = [c.c_entity_base.card_id for c in p2.trash_cards]
        assert "BT1-015" not in p2_trash_ids, (
            "Flat Lv.4 should not be trashed by de-digivolve")

    def test_on_play_declines_fully_with_optional(self, debug_runner):
        """Agent must have a decline path (is_optional=True on the effect itself)."""
        runner = self._setup_play_mummymon(
            debug_runner, opp_stack=["BT1-015", "AD1-004"])

        eff = _find_c1_on_play(runner.game.player1.hand_cards[
            [c.c_entity_base.card_id for c in runner.game.player1.hand_cards].index(MUMMYMON)
        ])
        assert eff is not None
        assert eff.is_optional is True

    def test_on_play_does_not_leak_to_other_cards(self, debug_runner):
        """When another card is played (not Mummymon), Mummymon's C1 must not fire.
        Engine guards this via played_card identity in _effect_matches_timing,
        but we verify behavior end-to-end by playing a non-Mummymon Digimon."""
        runner = debug_runner(initial_memory=10)
        # Mummymon ON FIELD (so it has a host permanent)
        runner.place_on_field(1, [MUMMYMON])
        # Opponent stacked target
        runner.place_on_field(2, ["BT1-015", "AD1-004"])
        p2 = runner.game.player2
        opp_perm = p2.battle_area[0]
        initial_stack = len(opp_perm.card_sources)

        # Put a different card in hand
        runner.inject_card(1, "ST1-03", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None
        runner.execute(play)
        # Resolve any pending follow-ups (there shouldn't be any for ST1-03)
        runner.auto_resolve()

        # Opponent stack should be unchanged — Mummymon's C1 must not fire
        assert len(opp_perm.card_sources) == initial_stack, (
            f"Mummymon's C1 should not fire when a different card is played. "
            f"Stack: {[c.c_entity_base.card_id for c in opp_perm.card_sources]}")


# ─────────────────────────────────────────────────────────────────
# C2 [On Deletion] tests
# ─────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestEX10051OnDeletionPlayTamer:
    """C2: On deletion, play 1 Tamer with [Myotismon] in text from trash free,
    excluding cards sharing a name with an existing own Tamer."""

    def test_on_deletion_opens_select_target_for_trash_tamer(self, debug_runner):
        """Deleting Mummymon with a valid Myotismon-text Tamer in trash opens
        a SelectTarget phase so the player can pick which Tamer to play."""
        runner = debug_runner(initial_memory=0)
        # Clear any residual field so no duplicate-name issues arise
        perm = runner.place_on_field(1, [MUMMYMON])
        # Put a valid Myotismon-text Tamer in trash
        yukio = runner.game.player1.hand_cards  # scratch
        runner.inject_card(1, YUKIO_OIKAWA_BT8, "trash")
        # Something irrelevant to rule out false positives
        runner.inject_card(1, "ST1-03", "trash")

        # Delete Mummymon (by effect, opponent-style)
        runner.game.player1.delete_permanent(perm, is_opponent_effect=True)

        game = runner.game
        assert game.pending_selection is not None, (
            "C2 should open a pending selection for playing a trash Tamer")
        # Phase is SelectTarget (the play-from-zone entry)
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Should enter SelectTarget. Got: {game.current_phase}")
        # Should be optional per the 'may' wording
        assert game.pending_selection.is_optional is True

    def test_on_deletion_valid_indices_include_myotismon_tamer(self, debug_runner):
        """The trash index for a valid Myotismon-text Tamer must be in the
        pending selection's valid_indices."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        runner.inject_card(1, YUKIO_OIKAWA_BT8, "trash")
        p1 = runner.game.player1

        yukio_idx_in_trash = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == YUKIO_OIKAWA_BT8)
        expected_action = SEL_TRASH_START + yukio_idx_in_trash

        p1.delete_permanent(perm, is_opponent_effect=True)

        game = runner.game
        assert game.pending_selection is not None
        assert expected_action in game.pending_selection.valid_indices, (
            f"Expected trash action {expected_action} for Yukio Oikawa in valid set "
            f"{list(game.pending_selection.valid_indices)}")

    def test_on_deletion_excludes_non_myotismon_tamers(self, debug_runner):
        """Tamers without 'Myotismon' in their text must NOT be offered."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        runner.inject_card(1, TAI_KAMIYA, "trash")  # no Myotismon
        p1 = runner.game.player1
        tai_idx = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == TAI_KAMIYA)

        p1.delete_permanent(perm, is_opponent_effect=True)
        game = runner.game
        # Either no pending selection (nothing valid), or the non-matching idx
        # is not included.
        bad_action = SEL_TRASH_START + tai_idx
        if game.pending_selection is not None:
            assert bad_action not in game.pending_selection.valid_indices, (
                "Non-Myotismon-text Tamer must not be selectable")

    def test_on_deletion_excludes_non_tamer_cards(self, debug_runner):
        """Non-Tamer cards in trash must NOT be offered (even if they have
        'Myotismon' in their text)."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        # BT3-087 is a Digimon named Mummymon whose text references Myotismon
        # via MaloMyotismon — NOT a tamer, so must not be selectable.
        runner.inject_card(1, "BT3-087", "trash")
        p1 = runner.game.player1
        bt3_idx = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == "BT3-087")

        p1.delete_permanent(perm, is_opponent_effect=True)
        game = runner.game
        bad_action = SEL_TRASH_START + bt3_idx
        if game.pending_selection is not None:
            assert bad_action not in game.pending_selection.valid_indices, (
                "Non-Tamer cards with 'Myotismon' in text must not be selectable")

    def test_on_deletion_excludes_cards_sharing_name_with_own_tamer(
            self, debug_runner):
        """A candidate with the same card name as an existing Tamer must NOT
        be offered. BT8-093 and EX10-065 are both named 'Yukio Oikawa'."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=0)
        # Put Yukio Oikawa (BT8-093) as an existing Tamer on field
        runner.place_on_field(1, [YUKIO_OIKAWA_BT8])
        mummymon_perm = runner.place_on_field(1, [MUMMYMON])
        # Put the OTHER Yukio Oikawa (EX10-065) in trash — same name
        runner.inject_card(1, YUKIO_OIKAWA_EX10, "trash")
        # Also put a valid non-matching-name Myotismon-text Tamer in trash
        runner.inject_card(1, ARUKENIMON_MUMMYMON_TAMER, "trash")

        p1 = runner.game.player1
        blocked_idx = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == YUKIO_OIKAWA_EX10)
        allowed_idx = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == ARUKENIMON_MUMMYMON_TAMER)

        blocked_action = SEL_TRASH_START + blocked_idx
        allowed_action = SEL_TRASH_START + allowed_idx

        p1.delete_permanent(mummymon_perm, is_opponent_effect=True)
        game = runner.game
        assert game.pending_selection is not None, (
            "Should still have a valid candidate (Arukenimon & Mummymon tamer)")
        valid = list(game.pending_selection.valid_indices)
        assert blocked_action not in valid, (
            f"EX10-065 Yukio Oikawa must be excluded (same name as own Tamer). "
            f"Valid: {valid}")
        assert allowed_action in valid, (
            f"Arukenimon & Mummymon tamer must still be selectable. "
            f"Valid: {valid}")

    def test_on_deletion_plays_selected_tamer_for_free(self, debug_runner):
        """Executing the pending selection should actually play the Tamer to
        the field without paying its cost."""
        from engine_py_legacy.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        runner.inject_card(1, YUKIO_OIKAWA_BT8, "trash")
        p1 = runner.game.player1
        initial_memory = runner.game.memory

        # Delete Mummymon
        p1.delete_permanent(perm, is_opponent_effect=True)
        game = runner.game
        assert game.pending_selection is not None
        valid = list(game.pending_selection.valid_indices)
        # Pick the Yukio Oikawa trash action
        yukio_idx = next(
            i for i, c in enumerate(p1.trash_cards)
            if c.c_entity_base.card_id == YUKIO_OIKAWA_BT8)
        action = SEL_TRASH_START + yukio_idx
        assert action in valid
        game.decode_action(action, p1.player_id)

        # Yukio Oikawa should be on p1's field now
        tamer_ids_on_field = [
            p.top_card.c_entity_base.card_id for p in p1.battle_area if p.is_tamer]
        assert YUKIO_OIKAWA_BT8 in tamer_ids_on_field, (
            f"Yukio Oikawa should be on field after C2. "
            f"Tamer field: {tamer_ids_on_field}")
        # Memory should be unchanged (free play)
        assert game.memory == initial_memory, (
            f"Free play should not change memory. "
            f"Before: {initial_memory}, After: {game.memory}")
        # Yukio Oikawa should be removed from trash
        trash_ids = [c.c_entity_base.card_id for c in p1.trash_cards]
        assert YUKIO_OIKAWA_BT8 not in trash_ids, (
            "Played tamer should be removed from trash")

    def test_on_deletion_no_fire_when_no_valid_candidates(self, debug_runner):
        """If there's no valid candidate in trash, no selection should be
        created and the game should not enter a weird stuck state."""
        runner = debug_runner(initial_memory=0)
        perm = runner.place_on_field(1, [MUMMYMON])
        # Only a non-matching card in trash
        runner.inject_card(1, TAI_KAMIYA, "trash")

        runner.game.player1.delete_permanent(perm, is_opponent_effect=True)
        # No pending selection should block the game
        assert runner.game.pending_selection is None, (
            "No valid candidate means no pending selection should be created")


# ─────────────────────────────────────────────────────────────────
# DCGO parity tests
# ─────────────────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestEX10051DCGOParity:
    """Extra faithfulness tests mirroring DCGO C# semantics."""

    def test_c1_trash_uses_trash_from_hand_api(self):
        """The script's on_trashed callback must call player.trash_from_hand
        so that OnDiscardHand timing fires (DCGO IDiscardHand)."""
        import inspect
        from engine_py_legacy.engine.data.scripts.ex10.ex10_051 import EX10_051
        source = inspect.getsource(EX10_051)
        assert 'trash_from_hand' in source, (
            "C1 should use player.trash_from_hand so OnDiscardHand fires.")

    def test_c1_uses_target_owner_trash(self):
        """De-digivolved cards must go to the target permanent's owner trash,
        not to a hardcoded reference to player.enemy."""
        import inspect
        from engine_py_legacy.engine.data.scripts.ex10.ex10_051 import EX10_051
        source = inspect.getsource(EX10_051)
        assert 'target_perm.owner' in source or 'target_owner' in source, (
            "De-digivolved cards should route via target_perm.owner.")

    def test_c2_filter_checks_all_battle_area_permanents(self):
        """The same-name exclusion must scan ALL battle-area permanents,
        mirroring DCGO's GetBattleAreaPermanents() which returns every slot."""
        import inspect
        from engine_py_legacy.engine.data.scripts.ex10.ex10_051 import EX10_051
        source = inspect.getsource(EX10_051)
        # Must not gate by is_tamer when building the name set
        assert 'is_tamer and p.top_card' not in source, (
            "Name-exclusion set must not be gated on is_tamer only.")
        assert 'existing_own_names' in source or 'for p in player.battle_area' in source
