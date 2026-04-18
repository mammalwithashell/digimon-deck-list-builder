"""Behavioral tests for EX7-049 Metallicdramon (Lv.6 Black Sky Dragon).

Card text (from cards.json):
  [On Play] [When Attacking] <De-Digivolve 4> 1 of your opponent's Digimon.
      (Trash up to 4 cards from the top. You can't trash past level 3 cards.)
  [When Digivolving] None of your opponent's level 4 or lower Digimon can
      digivolve until the end of their turn.
  [All Turns] [Once Per Turn] When this Digimon would leave battle area other
      than by one of your effects, you may play 1 Digimon card with the
      [Rock Dragon] or [Earth Dragon] trait from your trash without paying
      the cost.

NOTE: The user request specified CANNOT_ATTACK + CANNOT_BLOCK for the When
Digivolving clause. The actual card text in cards.json says "can't digivolve",
so tests validate CANNOT_DIGIVOLVE per faithful implementation policy.
"""

import pytest

from digimon_gym.engine.data.enums import GamePhase
from digimon_gym.engine.interfaces.modifiers import ModifierType

# Card IDs
METALLICDRAMON = "EX7-049"   # Card under test (Lv.6 Black Sky Dragon)
MEGADRAMON = "BT2-060"       # Black Lv.5 (valid digivolve base for Metallicdramon)
FILLER = "BT1-003"           # Koromon (Lv.2 digi-egg for filler)
AGUMON_LV3 = "ST1-03"        # Agumon (Lv.3 Red)
GREYMON_LV4 = "ST1-07"       # Greymon (Lv.4 Red)
METALGREYMON_LV5 = "ST1-09"  # MetalGreymon (Lv.5 Red)
WARGREYMON_LV6 = "ST1-11"    # WarGreymon (Lv.6 Red)
VORVOMON = "BT2-011"         # Lv.3 Rock Dragon Digimon (for trash test)
GROUNDRAMON = "BT1-020"      # Lv.5 Earth Dragon Digimon (for trash test)

FILLER_CARD = "ST1-03"       # Main deck filler


def _build_deck(extras=None):
    """50-card deck: extras + filler."""
    cards = list(extras or [])
    while len(cards) < 50:
        cards.append(FILLER_CARD)
    return cards


# ════════════════════════════════════════════════════════════════════════
# Effect 0 & 1: [On Play] / [When Attacking] <De-Digivolve 4>
# ════════════════════════════════════════════════════════════════════════

@pytest.mark.behavioral
class TestEX7049DeDigivolve:
    """Tests for [On Play] / [When Attacking] <De-Digivolve 4> effect."""

    def test_on_play_de_digivolve_removes_4_cards(self, debug_runner):
        """On Play should De-Digivolve 4: trash exactly 4 cards from the top
        of an opponent's 5-card stack, leaving 1 card."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=20)

        # Opponent: 5-card stack (Lv.3 -> Lv.4 -> Lv.5 -> Lv.5 -> Lv.5)
        opp_perm = runner.place_on_field(2, [
            AGUMON_LV3, GREYMON_LV4, METALGREYMON_LV5,
            METALGREYMON_LV5, METALGREYMON_LV5
        ])
        assert len(opp_perm.card_sources) == 5

        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        assert play is not None, f"Should be able to play. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The opp stack should now be exactly 1 card (Lv.3 base remains)
        opp_field = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_field) == 1, "Opponent should still have 1 Digimon on field"
        assert len(opp_field[0].stack_ids) == 1, (
            f"De-Digivolve 4 on a 5-card stack should leave 1 card, "
            f"got {len(opp_field[0].stack_ids)}"
        )
        # 4 cards should be in opponent's trash
        assert len(snap.p2_trash) == 4, (
            f"4 cards should be trashed by De-Digivolve 4, got {len(snap.p2_trash)}"
        )

    def test_on_play_de_digivolve_stops_at_level_3(self, debug_runner):
        """De-Digivolve 4 can't trash past level 3 cards. A 3-card stack
        (Lv.3 + 2 above) should only lose 2 cards, not 4."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=20)

        # Opponent: 3-card stack (Lv.3 -> Lv.4 -> Lv.5)
        opp_perm = runner.place_on_field(2, [
            AGUMON_LV3, GREYMON_LV4, METALGREYMON_LV5
        ])
        assert len(opp_perm.card_sources) == 3

        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_field = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_field) == 1
        # Should stop at Lv.3 — only 2 cards trashed (Lv.4 and Lv.5)
        assert len(opp_field[0].stack_ids) == 1, (
            f"Should stop at Lv.3 base, got {len(opp_field[0].stack_ids)} cards"
        )
        assert len(snap.p2_trash) == 2, (
            f"Only 2 cards above Lv.3 base should be trashed, got {len(snap.p2_trash)}"
        )

    def test_on_play_de_digivolve_single_card_digimon(self, debug_runner):
        """De-Digivolve on a single-card Digimon (no evo stack) should
        remove 0 cards — the base card stays."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=20)

        # Opponent: single Lv.4 Digimon (no digivolution cards underneath)
        opp_perm = runner.place_on_field(2, [GREYMON_LV4])
        assert len(opp_perm.card_sources) == 1

        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_field = [s for s in snap.p2_field if s.is_digimon]
        assert len(opp_field) == 1
        assert len(opp_field[0].stack_ids) == 1, (
            "Single-card Digimon should remain untouched by De-Digivolve"
        )
        assert len(snap.p2_trash) == 0, (
            "No cards should be trashed from a single-card Digimon"
        )

    def test_on_play_trashed_cards_go_to_opponent_trash(self, debug_runner):
        """De-Digivolve cards must go to the opponent's trash, not ours."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=20)

        opp_perm = runner.place_on_field(2, [
            AGUMON_LV3, GREYMON_LV4, METALGREYMON_LV5
        ])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Trashed cards are in P2 trash, not P1 trash
        assert len(snap.p2_trash) >= 2, "De-digivolved cards should be in opponent's trash"
        # P1 trash should be empty (no own cards trashed)
        assert len(snap.p1_trash) == 0, (
            f"P1 trash should be empty, got {len(snap.p1_trash)} cards"
        )

    def test_when_attacking_de_digivolve_4(self, debug_runner):
        """When Attacking should also trigger De-Digivolve 4."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=5)

        # Place Metallicdramon on our field (already played, can attack)
        our_perm = runner.place_on_field(1, [METALLICDRAMON], turn_played=-1)

        # Opponent: 5-card stack
        opp_perm = runner.place_on_field(2, [
            AGUMON_LV3, GREYMON_LV4, METALGREYMON_LV5,
            METALGREYMON_LV5, METALGREYMON_LV5
        ])
        assert len(opp_perm.card_sources) == 5

        runner.set_phase("Main")

        # Find attack action
        attack = runner.find_action("Attack")
        assert attack is not None, f"Should be able to attack. Actions: {runner.actions()}"
        runner.execute(attack)
        runner.auto_resolve()

        # After attack + De-Digivolve 4, opponent stack should be reduced
        remaining = len(opp_perm.card_sources)
        assert remaining == 1, (
            f"De-Digivolve 4 on attack should leave 1 card, got {remaining}"
        )


# ════════════════════════════════════════════════════════════════════════
# Effect 2: [When Digivolving] Opponent Lv.4 or lower can't digivolve
# ════════════════════════════════════════════════════════════════════════

@pytest.mark.behavioral
class TestEX7049WhenDigivolving:
    """Tests for [When Digivolving] CANNOT_DIGIVOLVE on opponent's Lv.4- Digimon.

    Actual card text: "None of your opponent's level 4 or lower Digimon
    can digivolve until the end of their turn."
    """

    def test_lv4_gets_cannot_digivolve(self, debug_runner):
        """Opponent's Lv.4 Digimon should get CANNOT_DIGIVOLVE modifier
        when Metallicdramon digivolves."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        # Opponent has a Lv.4 Digimon
        opp_perm = runner.place_on_field(2, [GREYMON_LV4])
        assert opp_perm.level == 4

        # Place Black Lv.5 on our field, digivolve into Metallicdramon
        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, (
            f"Should be able to digivolve into Metallicdramon. Actions: {runner.actions()}"
        )
        runner.execute(digivolve)
        runner.auto_resolve()

        # Check CANNOT_DIGIVOLVE modifier on opponent's Lv.4
        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {"permanent": opp_perm}
        )
        assert has_block, (
            "Opponent's Lv.4 Digimon should have CANNOT_DIGIVOLVE after "
            "Metallicdramon's When Digivolving effect"
        )

    def test_lv3_gets_cannot_digivolve(self, debug_runner):
        """Opponent's Lv.3 Digimon should also get CANNOT_DIGIVOLVE
        (level 4 or LOWER)."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        opp_perm = runner.place_on_field(2, [AGUMON_LV3])
        assert opp_perm.level == 3

        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None
        runner.execute(digivolve)
        runner.auto_resolve()

        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {"permanent": opp_perm}
        )
        assert has_block, (
            "Opponent's Lv.3 Digimon should have CANNOT_DIGIVOLVE"
        )

    def test_lv5_not_affected(self, debug_runner):
        """Opponent's Lv.5 Digimon should NOT get CANNOT_DIGIVOLVE (above Lv.4)."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        opp_perm = runner.place_on_field(2, [METALGREYMON_LV5])
        assert opp_perm.level == 5

        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None
        runner.execute(digivolve)
        runner.auto_resolve()

        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {"permanent": opp_perm}
        )
        assert not has_block, (
            "Opponent's Lv.5 Digimon should NOT have CANNOT_DIGIVOLVE"
        )

    def test_lv6_not_affected(self, debug_runner):
        """Opponent's Lv.6 Digimon should NOT get CANNOT_DIGIVOLVE."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        opp_perm = runner.place_on_field(2, [WARGREYMON_LV6])
        assert opp_perm.level == 6

        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None
        runner.execute(digivolve)
        runner.auto_resolve()

        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {"permanent": opp_perm}
        )
        assert not has_block, (
            "Opponent's Lv.6 Digimon should NOT have CANNOT_DIGIVOLVE"
        )

    def test_modifier_expiry_end_of_opponent_turn(self, debug_runner):
        """The CANNOT_DIGIVOLVE modifier should expire 'end_of_opponent_turn'
        (card text says 'until the end of their turn')."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        opp_perm = runner.place_on_field(2, [GREYMON_LV4])
        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        runner.execute(digivolve)
        runner.auto_resolve()

        # Check that the modifier uses 'end_of_opponent_turn' expiry
        entries = runner.game.modifiers._modifiers.get(ModifierType.CANNOT_DIGIVOLVE, [])
        matching = [e for e in entries if e.is_active(opp_perm, {})]
        assert len(matching) >= 1, "Should have at least 1 CANNOT_DIGIVOLVE entry"
        assert matching[0].expiry == 'end_of_opponent_turn', (
            f"Modifier expiry should be 'end_of_opponent_turn', got '{matching[0].expiry}'"
        )

    def test_multiple_lv4_digimon_all_affected(self, debug_runner):
        """ALL opponent Lv.4 or lower Digimon should get the modifier,
        not just one."""
        deck = _build_deck([METALLICDRAMON] * 4 + [MEGADRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=10)

        opp_perm1 = runner.place_on_field(2, [GREYMON_LV4])
        opp_perm2 = runner.place_on_field(2, [AGUMON_LV3])

        runner.place_on_field(1, [MEGADRAMON])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        runner.execute(digivolve)
        runner.auto_resolve()

        has1 = runner.game.modifiers.has_modifier(
            opp_perm1, ModifierType.CANNOT_DIGIVOLVE, {}
        )
        has2 = runner.game.modifiers.has_modifier(
            opp_perm2, ModifierType.CANNOT_DIGIVOLVE, {}
        )
        assert has1, "Opponent's Lv.4 should have CANNOT_DIGIVOLVE"
        assert has2, "Opponent's Lv.3 should have CANNOT_DIGIVOLVE"

    def test_not_triggered_on_play(self, debug_runner):
        """CANNOT_DIGIVOLVE should NOT trigger on play, only on digivolve."""
        deck = _build_deck([METALLICDRAMON] * 4)
        runner = debug_runner(deck1=deck, deck2=deck, initial_memory=20)

        opp_perm = runner.place_on_field(2, [GREYMON_LV4])
        runner.inject_card(1, METALLICDRAMON, "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Metallicdramon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        has_block = runner.game.modifiers.has_modifier(
            opp_perm, ModifierType.CANNOT_DIGIVOLVE, {}
        )
        assert not has_block, (
            "CANNOT_DIGIVOLVE should NOT be applied when playing (not digivolving)"
        )


# ════════════════════════════════════════════════════════════════════════
# Effect 3: [All Turns] [Once Per Turn] When leave, play from trash
# ════════════════════════════════════════════════════════════════════════

def _select_non_decline(runner):
    """Pick the non-decline action in a SelectTarget phase.
    Returns True if a card was selected, False otherwise."""
    if runner.game.current_phase != GamePhase.SelectTarget:
        return False
    legal = runner.action_mask()
    play_actions = [a for a in legal if a != 62]
    if not play_actions:
        return False
    runner.execute(play_actions[0])
    runner.auto_resolve()
    return True


@pytest.mark.behavioral
class TestEX7049WhenLeaveField:
    """Tests for [All Turns] [Once Per Turn] When this Digimon would leave
    battle area other than by one of your effects, play Rock/Earth Dragon
    from trash."""

    def test_triggers_on_opponent_effect_removal(self, debug_runner):
        """Should trigger when removed by opponent's effect (not own effect)."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, [METALLICDRAMON])
        runner.inject_card(1, VORVOMON, "trash")

        p1 = runner.game.player1
        p1.delete_permanent(perm, is_opponent_effect=True)

        selected = _select_non_decline(runner)
        assert selected, "Should create a selection to play from trash"

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert VORVOMON in field_ids, (
            f"Vorvomon (Rock Dragon) should be played from trash. Field: {field_ids}"
        )

    def test_does_not_trigger_on_own_effect_removal(self, debug_runner):
        """Should NOT trigger when removed by own effects."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, [METALLICDRAMON])
        runner.inject_card(1, VORVOMON, "trash")

        p1 = runner.game.player1
        p1.delete_permanent(perm, is_opponent_effect=False)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert VORVOMON not in field_ids, (
            f"Vorvomon should NOT be played on own-effect removal. Field: {field_ids}"
        )

    def test_plays_earth_dragon_from_trash(self, debug_runner):
        """Should accept Earth Dragon trait Digimon from trash."""
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, [METALLICDRAMON])
        runner.inject_card(1, GROUNDRAMON, "trash")

        p1 = runner.game.player1
        p1.delete_permanent(perm, is_opponent_effect=True)

        selected = _select_non_decline(runner)
        assert selected, "Should trigger for Earth Dragon trash target"

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert GROUNDRAMON in field_ids, (
            f"Groundramon (Earth Dragon) should be played from trash. Field: {field_ids}"
        )

    def test_effect_is_optional(self, debug_runner):
        """'you may play' means the effect is optional."""
        from digimon_gym.engine.data.enums import EffectTiming
        runner = debug_runner(initial_memory=5)

        perm = runner.place_on_field(1, [METALLICDRAMON])
        effects = perm.effect_list(EffectTiming.WhenRemoveField)
        leave_effects = [e for e in effects
                         if "Rock Dragon" in (e.effect_name or "")
                         or "Play Rock" in (e.effect_name or "")]
        assert len(leave_effects) >= 1, (
            f"Should have WhenRemoveField effect. "
            f"Effects: {[(e.effect_name, e.timing) for e in effects]}"
        )
        assert leave_effects[0].is_optional, (
            "The effect should be optional ('you may play')"
        )

    def test_once_per_turn(self, debug_runner):
        """Effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=5)

        perm1 = runner.place_on_field(1, [METALLICDRAMON])
        runner.inject_card(1, VORVOMON, "trash")

        p1 = runner.game.player1
        p1.delete_permanent(perm1, is_opponent_effect=True)

        selected = _select_non_decline(runner)
        assert selected, "First trigger should fire"

        # Second Metallicdramon + second trash target
        perm2 = runner.place_on_field(1, [METALLICDRAMON])
        runner.inject_card(1, GROUNDRAMON, "trash")

        p1.delete_permanent(perm2, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        field_ids = [s.card_id for s in snap.p1_field]
        assert GROUNDRAMON not in field_ids, (
            f"Second trigger should NOT fire (once per turn). Field: {field_ids}"
        )
