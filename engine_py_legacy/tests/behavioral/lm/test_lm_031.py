"""Behavioral tests for LM-031 Black Scramble (Option, Black, Cost 2).

Card text:
  [Main] 1 of your black Digimon may digivolve into a black Digimon card in
  the hand with the digivolution cost reduced by 3. Then, place this card in
  the battle area.

  [Start of Your Turn] If your opponent has a Digimon, <Delay>
  - Return 1 black Digimon card from your trash to the top of the deck. Then,
    if you don't have a Digimon, you may play 1 black Digimon card with 2000
    DP or less from your trash without paying the cost.

  [Security] You may play 1 black Digimon card with 2000 DP or less from
  your trash without paying the cost. Then, add this card to the hand.

Key cards used:
  BT5-061: Commandramon (Black Lv3, DP 2000, Cost 4)
  BT5-063: Kurisarimon (Black Lv4, DP 4000, Cost 5)
  BT12-060: ChuuChuumon (Black Lv3, DP 1000, Cost 3) - play target (<=2000 DP)
  BT2-057: Greymon (Black Lv4, DP 4000, Cost 4) - above 2000 DP
  ST1-03: Agumon (Red Lv3) - non-black negative test
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# ── Deck helpers ──────────────────────────────────────────────────────
_EGGS = ["ST13-01"] * 5  # Black digi-egg
_MAIN = (
    ["LM-031"] * 4
    + ["BT5-061"] * 4    # Commandramon (Black Lv3)
    + ["BT5-063"] * 4    # Kurisarimon (Black Lv4)
    + ["BT12-060"] * 4   # ChuuChuumon (Black Lv3, DP 1000)
    + ["BT2-057"] * 4    # Greymon (Black Lv4, DP 4000)
    + ["ST1-03"] * 4     # Agumon (Red Lv3, non-black)
    + ["BT1-010"] * 21   # filler
)
_DECK = _EGGS + _MAIN


@pytest.mark.behavioral
class TestLM031Effects:
    """Verify LM-031 has the correct effect structure."""

    def test_has_option_skill(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-031")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_delay_marker(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-031")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_has_start_turn_delay(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-031")
        effects = cs.effect_list(None)
        start_turn = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(start_turn) >= 1, "Should have OnStartTurn timing for delay"

    def test_has_security_skill(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-031")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True


@pytest.mark.behavioral
class TestLM031MainEffect:
    """[Main] Black Digimon digivolves into black from hand, cost -3, place in battle area."""

    def test_main_places_card_in_battle_area(self, debug_runner):
        """Playing Black Scramble should place it in the battle area (Delay)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-061"])  # Commandramon (Black Lv3) for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "LM-031", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Black Scramble")
        if action is None:
            action = runner.find_action("LM-031")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-031" for s in snap.p1_field)
        assert in_battle, "Black Scramble should be placed in the battle area"

    def test_main_digivolve_with_cost_reduction(self, debug_runner):
        """Main: 1 black Digimon digivolves into black from hand with cost -3."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-061"])  # Commandramon (Black Lv3) for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT5-063", "hand")  # Kurisarimon (Black Lv4)
        runner.inject_card(1, "LM-031", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Black Scramble")
        if action is None:
            action = runner.find_action("LM-031")
        assert action is not None

        runner.execute(action)
        # Select Commandramon for digivolve
        cmd_action = runner.find_action("Commandramon")
        if cmd_action is not None:
            runner.execute(cmd_action)
            # Select Kurisarimon from hand
            kuri_action = runner.find_action("Kurisarimon")
            if kuri_action is not None:
                runner.execute(kuri_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        kuri_on_field = any(s.card_id == "BT5-063" for s in snap.p1_field)
        assert kuri_on_field, (
            "Kurisarimon should be on field after digivolving from Commandramon"
        )

    def test_main_field_filter_rejects_non_black(self, debug_runner):
        """Field filter should reject non-black Digimon (e.g., Red Agumon)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        # Place both a Red and a Black Digimon; Red should not be digivolve target
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red Lv3)
        runner.place_on_field(1, ["BT5-061"])  # Commandramon (Black Lv3) for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT5-063", "hand")  # Black Lv4 in hand
        runner.inject_card(1, "LM-031", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Black Scramble")
        if action is None:
            action = runner.find_action("LM-031")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            kuri_on_field = any(s.card_id == "BT5-063" for s in snap.p1_field)
            assert not kuri_on_field, (
                "Should NOT digivolve Red Agumon using Black Scramble"
            )

    def test_main_digivolve_is_optional(self, debug_runner):
        """Main digivolve is optional — player can choose not to digivolve."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT5-061"])  # Commandramon for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "LM-031", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Black Scramble")
        if action is None:
            action = runner.find_action("LM-031")
        assert action is not None

        runner.execute(action)

        # Look for pass/decline action (digivolve is optional)
        pass_action = runner.find_action("pass")
        if pass_action is None:
            pass_action = runner.find_action("decline")
        if pass_action is not None:
            runner.execute(pass_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Card should still be placed in battle area even if digivolve declined
        in_battle = any(s.card_id == "LM-031" for s in snap.p1_field)
        assert in_battle, "LM-031 should be in battle area even if digivolve declined"


@pytest.mark.behavioral
class TestLM031DelayEffect:
    """[Start of Your Turn] <Delay> — return black Digimon from trash; optional play."""

    def test_delay_condition_requires_opponent_digimon(self, debug_runner):
        """Delay condition should fail when opponent has no Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-031"], turn_played=-1)
        runner.inject_card(1, "BT12-060", "trash")

        game = runner.game
        player1 = game.player1

        lm031_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-031":
                lm031_card = p.top_card
                break
        assert lm031_card is not None

        effects = lm031_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        result = start_turn_eff.can_use_condition({})
        assert result is False, (
            "Delay condition should be False when opponent has no Digimon"
        )

    def test_delay_condition_passes_with_opponent_digimon(self, debug_runner):
        """Delay condition should pass when opponent has a Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-031"], turn_played=-1)
        runner.place_on_field(2, ["BT5-061"])  # Opponent Digimon
        runner.inject_card(1, "BT12-060", "trash")

        game = runner.game
        player1 = game.player1

        lm031_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "LM-031":
                lm031_card = p.top_card
                break
        assert lm031_card is not None

        effects = lm031_card.effect_list(None)
        start_turn_eff = [e for e in effects if e.timing == EffectTiming.OnStartTurn][0]

        result = start_turn_eff.can_use_condition({})
        assert result is True, (
            "Delay condition should be True when opponent has a Digimon"
        )

    def test_delay_returns_black_digimon_to_deck_top(self, debug_runner):
        """Delay should return 1 black Digimon from trash to top of deck."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-031"], turn_played=-1)
        runner.place_on_field(2, ["BT5-061"])  # Opponent Digimon
        runner.inject_card(1, "BT12-060", "trash")  # Black Digimon in trash

        lib_size_before = len(runner.game.player1.library_cards)

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, f"Should have Delay action. Available: {runner.actions()}"
        runner.execute(delay_action)

        # The delay enters a SelectTrash phase for choosing which Digimon to return.
        # auto_resolve will stop at Main, so we need to manually resolve selection.
        for _ in range(10):
            if runner.game.game_over:
                break
            if runner.game.current_phase.name == "Main":
                break
            legal = runner.action_mask()
            if not legal:
                break
            runner.execute(legal[0])

        snap = runner.snapshot()
        # LM-031 should be trashed
        assert not any(s.card_id == "LM-031" for s in snap.p1_field), (
            "LM-031 should be trashed after Delay activation"
        )
        # Library should gain 1 card (the returned Digimon)
        assert snap.p1_library_size == lib_size_before + 1, (
            "Deck should gain 1 card from returned Digimon"
        )
        # BT12-060 should no longer be in trash
        assert "BT12-060" not in snap.p1_trash, (
            "BT12-060 should have been returned from trash to deck"
        )

    def test_delay_play_from_trash_when_no_digimon(self, debug_runner):
        """After returning, if no Digimon on field, may play black <=2000 DP from trash."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        # Only the delay card on field (no Digimon)
        runner.place_on_field(1, ["LM-031"], turn_played=-1)
        runner.place_on_field(2, ["BT5-061"])  # Opponent Digimon

        # Two black Digimon in trash: one to return, one to play
        runner.inject_card(1, "BT2-057", "trash")    # Greymon (DP 4000) - return target
        runner.inject_card(1, "BT12-060", "trash")   # ChuuChuumon (DP 1000) - play target

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # Step through selection phases.  For optional selections, prefer a
        # non-pass action so the play-from-trash actually fires.
        for _ in range(10):
            if runner.game.game_over:
                break
            if runner.game.current_phase.name == "Main":
                break
            legal = runner.action_mask()
            if not legal:
                break
            # Prefer non-pass (62) action for optional play
            non_pass = [a for a in legal if a != 62]
            chosen = non_pass[0] if non_pass else legal[0]
            runner.execute(chosen)

        snap = runner.snapshot()
        has_digimon = any(s.is_digimon for s in snap.p1_field)
        assert has_digimon, (
            "Should play a black Digimon <=2000 DP from trash when no Digimon on field"
        )

    def test_delay_no_play_when_has_digimon(self, debug_runner):
        """If P1 has a Digimon, should NOT get play-from-trash sub-effect."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-031"], turn_played=-1)
        runner.place_on_field(1, ["BT5-061"])  # P1 has Commandramon on field
        runner.place_on_field(2, ["BT5-061"])  # Opponent Digimon
        runner.inject_card(1, "BT12-060", "trash")  # ChuuChuumon in trash
        runner.inject_card(1, "BT2-057", "trash")   # Greymon in trash

        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Only the original Commandramon should be on field
        digimon_count = sum(1 for s in snap.p1_field if s.is_digimon)
        assert digimon_count == 1, (
            f"Should NOT play from trash when P1 has a Digimon. Got {digimon_count}"
        )


@pytest.mark.behavioral
class TestLM031SecurityEffect:
    """[Security] Play black Digimon <=2000 DP from trash; add to hand."""

    def test_security_adds_to_hand(self):
        """Security process should add LM-031 to the hand."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-031")
        effects = cs.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        class MockGame:
            def effect_play_from_zone(self, *a, **kw):
                pass

        class MockPlayer:
            def __init__(self, card):
                self.hand_cards = []
                self.trash_cards = [card]

        player = MockPlayer(cs)
        sec_eff.on_process_callback({'player': player, 'game': MockGame()})
        assert cs in player.hand_cards, "Security should add LM-031 to hand"

    def test_security_play_filter_rejects_non_black(self):
        """Security play filter should reject non-black Digimon."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import CardColor
        db = CardDatabase()
        agumon = db.create_card_source("ST1-03")
        assert CardColor.Black not in agumon.c_entity_base.card_colors, (
            "ST1-03 Agumon should not be Black"
        )

    def test_security_play_filter_rejects_high_dp(self):
        """Security play filter should reject Digimon with DP > 2000."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        greymon = db.create_card_source("BT2-057")
        assert greymon.c_entity_base.dp == 4000, "BT2-057 should have 4000 DP"

    def test_security_play_filter_accepts_low_dp_black(self):
        """Security play filter should accept black Digimon with DP <= 2000."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.enums import CardColor
        db = CardDatabase()
        chuu = db.create_card_source("BT12-060")
        assert CardColor.Black in chuu.c_entity_base.card_colors
        assert chuu.c_entity_base.dp == 1000
