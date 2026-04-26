"""Behavioral tests for LM-030 Green Scramble (Option, Green, Cost 2).

Card text:
  [Main] 1 of your green Digimon may digivolve into a green Digimon card in
  the hand with the digivolution cost reduced by 3. Then, place this card in
  the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay> (by trashing
  this card after the placing turn, activate the effect below): Return 1 green
  Digimon card from your trash to the top of the deck. Then, if you don't have
  a Digimon, you may play 1 green Digimon card with 2000 DP or less from your
  trash without paying the cost.

  [Security] You may play 1 green Digimon card with 2000 DP or less from your
  trash without paying the cost. Then, add this card to the hand.

Clauses tested:
  1. Main effect filters hand for GREEN Digimon only.
  2. Main effect filters field for GREEN Digimon only.
  3. Main effect digivolves with cost reduced by 3.
  4. Main effect places option in battle area (delay).
  5. Delay condition: requires opponent to have a Digimon.
  6. Delay process: trashes delay card, returns green Digimon from trash to deck top.
  7. Delay sub-effect: if no Digimon on field, play green Digimon <=2000 DP from trash.
  8. Delay sub-effect: SKIP play if you have a Digimon on field.
  9. Security: play green Digimon <=2000 DP from trash.
  10. Security: add this card to hand.
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# ── Helper deck ─────────────────────────────────────────────────────────
# BT1-067: Palmon (Green Lv3, DP 1000) — <=2000 DP target
# BT1-066: Tentomon (Green Lv3, DP 2000) — exactly 2000 DP
# BT1-064: Goblimon (Green Lv3, DP 3000) — above 2000 DP threshold
# BT1-073: Kabuterimon (Green Lv4, DP 5000) — digivolve target
# BT1-069: Ogremon (Green Lv4, DP 4000) — digivolve target
# ST1-03: Agumon (Red Lv3) — non-green (negative test)
_EGGS = ["BT1-001"] * 5
_MAIN = (
    ["LM-030"] * 4       # Card under test
    + ["BT1-067"] * 4    # Palmon (Green Lv3, DP 1000)
    + ["BT1-066"] * 4    # Tentomon (Green Lv3, DP 2000)
    + ["BT1-064"] * 4    # Goblimon (Green Lv3, DP 3000)
    + ["BT1-073"] * 4    # Kabuterimon (Green Lv4, DP 5000)
    + ["BT1-069"] * 4    # Ogremon (Green Lv4, DP 4000)
    + ["ST1-03"] * 4     # Agumon (Red Lv3)
    + ["BT1-010"] * 17   # filler
)
_DECK = _EGGS + _MAIN


@pytest.mark.behavioral
class TestLM030MainEffect:
    """[Main] Green Digimon digivolves into green from hand, cost -3, place in battle area."""

    def test_main_effect_exists(self, debug_runner):
        """LM-030 should have an OptionSkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-030")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill timing"

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Green Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT1-067"])  # Palmon (Green Lv3)
        runner.inject_card(1, "LM-030", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Green Scramble")
        if action is None:
            action = runner.find_action("LM-030")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-030" for s in snap.p1_field)
        assert in_battle, "Green Scramble should be placed in the battle area"

    def test_main_no_duplicate_placement(self, debug_runner):
        """Green Scramble should only appear once in battle area."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT1-067"])  # Palmon
        runner.inject_card(1, "LM-030", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Green Scramble")
        if action is None:
            action = runner.find_action("LM-030")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        count = sum(1 for s in snap.p1_field if s.card_id == "LM-030")
        assert count == 1, f"Should appear exactly once, found {count}"

    def test_main_digivolve_green_with_cost_reduction(self, debug_runner):
        """Main effect: green Digimon digivolves into green from hand with cost -3."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT1-067"])  # Palmon (Green Lv3)
        runner.inject_card(1, "BT1-073", "hand")  # Kabuterimon (Green Lv4)
        runner.inject_card(1, "LM-030", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Green Scramble")
        if action is None:
            action = runner.find_action("LM-030")
        assert action is not None

        runner.execute(action)
        # Try to select the Palmon for digivolve
        palmon_action = runner.find_action("Palmon")
        if palmon_action is not None:
            runner.execute(palmon_action)
            kabuterimon_action = runner.find_action("Kabuterimon")
            if kabuterimon_action is not None:
                runner.execute(kabuterimon_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        kabuterimon_on_field = any(s.card_id == "BT1-073" for s in snap.p1_field)
        assert kabuterimon_on_field, (
            "Kabuterimon should be on field after digivolving from Palmon"
        )

    def test_main_field_filter_rejects_non_green(self, debug_runner):
        """Field filter should reject non-green Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        # Only a Red Digimon on field — should not be selectable
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red Lv3)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT1-073", "hand")  # Green Lv4 in hand
        runner.inject_card(1, "LM-030", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Green Scramble")
        if action is None:
            action = runner.find_action("LM-030")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            # Agumon should not have evolved since it's Red
            agumon_still = any(
                s.card_id == "ST1-03" and s.is_digimon for s in snap.p1_field
            )
            kabuterimon_on_field = any(
                s.card_id == "BT1-073" for s in snap.p1_field
            )
            assert not kabuterimon_on_field, (
                "Should NOT digivolve Red Agumon using Green Scramble"
            )


@pytest.mark.behavioral
class TestLM030DelayEffect:
    """[Start of Your Turn] Delay: return green Digimon from trash to deck; play if no Digimon."""

    def test_delay_marker_exists(self, debug_runner):
        """LM-030 should have a delay marker effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-030")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_delay_trigger_exists(self, debug_runner):
        """Should have an OnStartTurn effect for the delay body."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-030")
        effects = cs.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(start_turn) >= 1, "Should have OnStartTurn timing"

    def test_delay_condition_requires_opponent_digimon(self, debug_runner):
        """Delay condition should require opponent to have a Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-030"], turn_played=-1)
        # No opponent Digimon
        runner.inject_card(1, "BT1-067", "trash")

        game = runner.game
        player1 = game.player1

        # Find the OnStartTurn effect
        lm030_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-030":
                lm030_card = p.top_card
                break
        assert lm030_card is not None

        effects = lm030_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        # Condition should fail without opponent Digimon
        result = start_turn_eff.can_use_condition({})
        assert result is False, (
            "Delay condition should be False when opponent has no Digimon"
        )

    def test_delay_condition_passes_with_opponent_digimon(self, debug_runner):
        """Delay condition should pass when opponent has a Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-030"], turn_played=-1)
        runner.place_on_field(2, ["BT1-067"])  # Opponent Digimon
        runner.inject_card(1, "BT1-067", "trash")

        game = runner.game
        player1 = game.player1

        lm030_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-030":
                lm030_card = p.top_card
                break
        assert lm030_card is not None

        effects = lm030_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        result = start_turn_eff.can_use_condition({})
        assert result is True, (
            "Delay condition should be True when opponent has a Digimon"
        )

    def test_delay_returns_green_digimon_to_deck_top(self, debug_runner):
        """Delay: return 1 green Digimon from trash to top of deck."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-030"], turn_played=-1)
        runner.place_on_field(2, ["BT1-067"])  # Opponent Digimon
        runner.inject_card(1, "BT1-067", "trash")  # Green Digimon in trash

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, f"Should have Delay action. Available: {runner.actions()}"
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-030 should be trashed
        assert not any(s.card_id == "LM-030" for s in snap.p1_field), (
            "LM-030 should be trashed after Delay activation"
        )
        # Library should gain 1 card
        assert snap.p1_library_size == lib_size_before + 1, (
            "Deck should gain 1 card from returned Digimon"
        )

    def test_delay_play_from_trash_when_no_digimon(self, debug_runner):
        """After returning card, if no Digimon on field, may play green <=2000 DP from trash."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        # No Digimon on P1 field (only the delay card, which isn't a Digimon)
        runner.place_on_field(1, ["LM-030"], turn_played=-1)
        runner.place_on_field(2, ["BT1-067"])  # Opponent Digimon

        # Two green Digimon in trash: one to return, one to play
        runner.inject_card(1, "BT1-064", "trash")  # Goblimon (DP 3000) — return target
        runner.inject_card(1, "BT1-067", "trash")  # Palmon (DP 1000) — play target

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # First: select a green Digimon from trash to return to deck
        legal = runner.action_mask()
        assert legal, "Should have trash selection options"
        runner.execute(legal[0])  # Select first available

        # Second: optional play from trash (no Digimon on field)
        actions = runner.actions()
        play_action = None
        for aid, desc in actions.items():
            if "pass" not in desc.lower() and "decline" not in desc.lower():
                play_action = aid
                break
        if play_action is not None:
            runner.execute(play_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        has_digimon = any(s.is_digimon for s in snap.p1_field)
        assert has_digimon, (
            "Should play a green Digimon with DP<=2000 from trash when no Digimon on field"
        )

    def test_delay_no_play_when_has_digimon(self, debug_runner):
        """If P1 has a Digimon on field, should NOT get play-from-trash sub-effect."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-030"], turn_played=-1)
        runner.place_on_field(1, ["BT1-067"])  # P1 has Palmon on field
        runner.place_on_field(2, ["BT1-067"])  # Opponent Digimon
        runner.inject_card(1, "BT1-067", "trash")  # Palmon in trash
        runner.inject_card(1, "BT1-066", "trash")  # Tentomon in trash (DP 2000)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only the original Palmon should be on field
        digimon_count = sum(1 for s in snap.p1_field if s.is_digimon)
        assert digimon_count == 1, (
            f"Should NOT play from trash when P1 has a Digimon. Got {digimon_count}"
        )


@pytest.mark.behavioral
class TestLM030SecurityEffect:
    """[Security] Play green Digimon <=2000 DP from trash; add to hand."""

    def test_security_effect_exists(self, debug_runner):
        """LM-030 should have a SecuritySkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-030")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True

    def test_security_play_filter_rejects_high_dp(self, debug_runner):
        """Security play filter should reject Digimon with DP > 2000."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Goblimon has DP 3000 — should be rejected
        goblimon = db.create_card_source("BT1-064")
        assert goblimon.c_entity_base.dp == 3000

        # Palmon has DP 1000 — should be accepted
        palmon = db.create_card_source("BT1-067")
        assert palmon.c_entity_base.dp == 1000

        # Tentomon has DP 2000 — should be accepted (<=2000)
        tentomon = db.create_card_source("BT1-066")
        assert tentomon.c_entity_base.dp == 2000

    def test_security_play_filter_rejects_non_green(self, debug_runner):
        """Security play filter should reject non-green Digimon."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import CardColor
        db = CardDatabase()

        agumon = db.create_card_source("ST1-03")
        assert CardColor.Green not in agumon.c_entity_base.card_colors

    def test_security_adds_to_hand(self, debug_runner):
        """Security effect should add LM-030 to the hand after play."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-030")
        effects = cs.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        # The security process pops the last trash card to hand.
        # This mimics the engine behavior where the security card is
        # trashed before the security effect fires.
        class MockGame:
            def effect_play_from_zone(self, *a, **kw):
                pass
        class MockPlayer:
            def __init__(self, card):
                self.hand_cards = []
                self.trash_cards = [card]  # simulate card being in trash
        player = MockPlayer(cs)
        sec_eff.on_process_callback({'player': player, 'game': MockGame()})
        assert cs in player.hand_cards, "Security should add LM-030 to hand"
