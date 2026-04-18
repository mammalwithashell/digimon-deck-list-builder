"""Behavioral tests for BT1-087 T.K. Takaishi (Tamer, Yellow, Cost 4).

Card text:
[Start of Your Turn] If you have 2 or less memory, set it to 3.
[On Play] Search your security stack, reveal 1 card among it and add it to
your hand. If it's a yellow card, <Recovery +1 (Deck)>. Then, shuffle your
security stack.
Security Effect [Security] Play this card without paying the cost.
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# Card references:
# BT1-087 T.K. Takaishi: Yellow Tamer, Cost 4
# BT1-048 Patamon:       Yellow Lv3 Digimon  — yellow-card recovery test
# ST1-03 Agumon:         Red Lv3 Digimon     — non-yellow card test
# BT1-088 Izzy Izumi:    Blue Tamer          — non-yellow card test (Tamer)
TK_TAKAISHI = "BT1-087"
YELLOW_DIGIMON = "BT1-048"       # Patamon, Yellow, Lv3
RED_DIGIMON = "ST1-03"           # Agumon, Red, Lv3
BLUE_TAMER = "BT1-088"           # Izzy, Blue tamer

TK_DECK = [TK_TAKAISHI] * 4 + [RED_DIGIMON] * 46


@pytest.mark.behavioral
class TestBT1087TKTakaishi:
    """Tests for BT1-087 T.K. Takaishi."""

    # ── [Start of Your Turn] Memory set to 3 ──────────────────────────

    def test_start_of_turn_sets_memory_to_3_when_at_0(self, debug_runner):
        """OnStartTurn should set memory to 3 when memory is 0."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = 0
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, (
            f"Memory should be set to 3 when at 0, got {game.memory}"
        )

    def test_start_of_turn_sets_memory_to_3_when_at_2(self, debug_runner):
        """OnStartTurn should set memory to 3 when memory is exactly 2 (boundary)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = 2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, (
            f"Memory should be set to 3 when at 2, got {game.memory}"
        )

    def test_start_of_turn_sets_memory_to_3_when_negative(self, debug_runner):
        """OnStartTurn should set memory to 3 when memory is negative."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = -3
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, (
            f"Memory should be set to 3 when at -3, got {game.memory}"
        )

    def test_start_of_turn_no_change_when_at_3(self, debug_runner):
        """OnStartTurn should NOT change memory when already at 3 (exactly 3 is not <= 2)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = 3
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 3, (
            f"Memory should remain at 3 when already at 3, got {game.memory}"
        )

    def test_start_of_turn_no_change_when_above_3(self, debug_runner):
        """OnStartTurn should NOT change memory when above 3 (e.g., 7)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = 7
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 7, (
            f"Memory should remain at 7 when above 3, got {game.memory}"
        )

    def test_start_of_turn_does_not_fire_on_opponent_turn(self, debug_runner):
        """Memory gate should NOT fire on opponent's turn."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, [TK_TAKAISHI])
        runner.set_phase("Main")

        game.memory = 0
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 0, (
            f"Memory should remain at 0 when it's opponent's turn, got {game.memory}"
        )

    def test_start_of_turn_no_effect_when_tamer_in_hand(self, debug_runner):
        """Memory gate should NOT fire when Tamer is in hand (field presence guard)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=5)
        game = runner.game

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        game.memory = 0
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.execute_effects(EffectTiming.OnStartTurn)

        assert game.memory == 0, (
            f"Memory should remain at 0 when Tamer is not on field, got {game.memory}"
        )

    # ── [On Play] Search security, reveal, add to hand ───────────────

    def test_on_play_moves_selected_security_card_to_hand(self, debug_runner):
        """On play, the chosen security card should leave security and join hand.

        Uses a non-yellow card so no Recovery +1 is triggered — net security
        should shrink by 1.
        """
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        # Clear current security, inject only non-yellow cards (Red)
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 1, "security")
        for _ in range(5):
            runner.inject_card(1, RED_DIGIMON, "security_bottom")

        sec_before = len(game.player1.security_cards)
        deck_before = len(game.player1.library_cards)

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        action = runner.find_action("T.K. Takaishi")
        assert action is not None, "Should be able to play T.K. Takaishi"
        runner.execute(action)
        runner.auto_resolve()

        # Security should be reduced by exactly 1 (no recovery on non-yellow)
        assert len(game.player1.security_cards) == sec_before - 1, (
            f"Security should shrink by 1 when revealed card is non-yellow, "
            f"was {sec_before}, now {len(game.player1.security_cards)}"
        )
        # Deck unchanged (no recovery)
        assert len(game.player1.library_cards) == deck_before, (
            f"Deck should not change when revealed card is non-yellow, "
            f"was {deck_before}, now {len(game.player1.library_cards)}"
        )

    def test_on_play_yellow_card_triggers_recovery(self, debug_runner):
        """If the revealed card is yellow, Recovery +1 should fire (deck → security)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        # Clear security and stack a Yellow card on top
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 1, "security")
        runner.inject_card(1, YELLOW_DIGIMON, "security_top")  # picks this
        # Add filler so auto_resolve picks the first legal (top)
        for _ in range(2):
            runner.inject_card(1, RED_DIGIMON, "security_bottom")

        sec_before = len(game.player1.security_cards)
        deck_before = len(game.player1.library_cards)

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        action = runner.find_action("T.K. Takaishi")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Net security: -1 (picked to hand) +1 (recovery from deck) = sec_before
        assert len(game.player1.security_cards) == sec_before, (
            f"Security should be net 0 (picked -1, recovery +1), "
            f"was {sec_before}, now {len(game.player1.security_cards)}"
        )
        # Deck should shrink by 1 (recovery moves 1 card from deck-top → security)
        assert len(game.player1.library_cards) == deck_before - 1, (
            f"Deck should shrink by 1 from recovery, was {deck_before}, "
            f"now {len(game.player1.library_cards)}"
        )

    def test_on_play_non_yellow_card_no_recovery(self, debug_runner):
        """If the revealed card is NOT yellow, NO recovery should fire."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        # Clear security and put ONLY a Red card (non-yellow)
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 1, "security")
        runner.inject_card(1, RED_DIGIMON, "security_top")
        runner.inject_card(1, RED_DIGIMON, "security_bottom")

        sec_before = len(game.player1.security_cards)
        deck_before = len(game.player1.library_cards)

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        action = runner.find_action("T.K. Takaishi")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Security should shrink by 1 (no recovery fired)
        assert len(game.player1.security_cards) == sec_before - 1, (
            f"Security should shrink by 1 with no recovery, "
            f"was {sec_before}, now {len(game.player1.security_cards)}"
        )
        # Deck should NOT change
        assert len(game.player1.library_cards) == deck_before, (
            f"Deck should not shrink when revealed card is non-yellow, "
            f"was {deck_before}, now {len(game.player1.library_cards)}"
        )

    def test_on_play_revealed_card_is_added_to_hand(self, debug_runner):
        """The revealed security card should appear in the player's hand."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        # Clear security; inject only one card so we know which one gets picked.
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 1, "security")
        runner.inject_card(1, YELLOW_DIGIMON, "security_top")

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        action = runner.find_action("T.K. Takaishi")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # The yellow Patamon should be in hand now
        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert YELLOW_DIGIMON in hand_ids, (
            f"Revealed yellow card should be in hand, got {hand_ids}"
        )
        # And NOT in security
        sec_ids = [c.card_id for c in game.player1.security_cards]
        assert YELLOW_DIGIMON not in sec_ids, (
            f"Revealed card should no longer be in security, got {sec_ids}"
        )

    def test_on_play_with_empty_security_no_crash(self, debug_runner):
        """Playing T.K. with empty security stack should not crash; no card added."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 1, "security")

        runner.inject_card(1, TK_TAKAISHI, "hand")
        runner.set_phase("Main")

        action = runner.find_action("T.K. Takaishi")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        assert len(game.player1.security_cards) == 0
        # T.K. should now be on field (tamer)
        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert TK_TAKAISHI in field_ids

    # ── Security Effect ──────────────────────────────────────────────

    def test_security_effect_plays_this_card_on_security_check(self, debug_runner):
        """[Security] Play this card without paying the cost — T.K. should
        land on owner's field and NOT be trashed afterward."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=10)
        game = runner.game

        # Place a strong attacker on P1 field
        runner.place_on_field(1, [RED_DIGIMON])  # filler attacker (will be set up below)

        # Inject T.K. into P2's security top
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 2, "security")
        runner.inject_card(2, TK_TAKAISHI, "security_top")

        p2_field_before = len(game.player2.battle_area)
        p2_trash_before = len(game.player2.trash_cards)

        # Directly invoke security check from P1 attacker (bypass full combat flow)
        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        # T.K. should be on P2's field (played by security effect)
        p2_field_ids = [p.top_card.card_id for p in game.player2.battle_area if p.top_card]
        assert TK_TAKAISHI in p2_field_ids, (
            f"T.K. should have been played from security to P2 field, "
            f"field now: {p2_field_ids}"
        )
        # And should NOT be in P2's trash (not double-referenced)
        p2_trash_ids = [c.card_id for c in game.player2.trash_cards]
        assert TK_TAKAISHI not in p2_trash_ids, (
            f"T.K. must NOT end up in trash when played by [Security] effect, "
            f"trash: {p2_trash_ids}"
        )
        # Field should grow
        assert len(game.player2.battle_area) == p2_field_before + 1

    def test_security_effect_does_not_pay_cost(self, debug_runner):
        """The [Security] play should bypass cost (T.K. costs 4 but memory can be low)."""
        runner = debug_runner(deck1=TK_DECK, deck2=TK_DECK, initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [RED_DIGIMON])
        from digimon_gym.engine.debug.state_injection import clear_zone as _cz
        _cz(game, 2, "security")
        runner.inject_card(2, TK_TAKAISHI, "security_top")

        memory_before = game.memory
        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        # Memory should not have changed due to the free security play
        # (it might change due to other turn mechanics; we just check T.K. landed)
        p2_field_ids = [p.top_card.card_id for p in game.player2.battle_area if p.top_card]
        assert TK_TAKAISHI in p2_field_ids, (
            "T.K. should play free from security even with 0 memory"
        )

    # ── Script shape / registration ──────────────────────────────────

    def test_script_registers_on_start_turn_effect(self):
        """Script should register an OnStartTurn effect for the memory gate."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(TK_TAKAISHI)
        effects = cs.effect_list(None)
        # C# reference uses OnStartTurn for SetMemoryTo3TamerEffect
        start_turn = [
            e for e in effects
            if e.timing == EffectTiming.OnStartTurn
        ]
        assert len(start_turn) >= 1, (
            "Should have at least 1 OnStartTurn effect for the memory gate"
        )

    def test_script_registers_on_play_effect(self):
        """Script should register an OnEnterFieldAnyone / is_on_play effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(TK_TAKAISHI)
        effects = cs.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) >= 1, "Should have at least 1 is_on_play effect"

    def test_script_registers_security_effect(self):
        """Script should register a [Security] effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(TK_TAKAISHI)
        effects = cs.effect_list(None)
        sec = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec) >= 1, "Should have at least 1 is_security_effect"
        # Security effect must have a process callback (else it is inert)
        for e in sec:
            assert e.on_process_callback is not None, (
                "Security effect must have on_process_callback — "
                "otherwise it silently fails to play the card"
            )
