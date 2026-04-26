"""Behavioral tests for P-206 Digital Gate Open (Option, White, Cost 4).

Official card text (from cards.json):
    You can ignore this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 Digimon card and 1 Tamer
    card among them to the hand. Return the rest to the bottom of the deck.
    Then, place this card in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate the
    effect below.)
    - You may play 1 Tamer card with the same color as any of your Digimon on
      the field from your hand with the play cost reduced by 4.
    [Security] You may play 1 Digimon card with a play cost of 3 or less from
    your hand or trash without paying the cost. Then, add this card to the hand.

C# reference: DCGO/Assets/Scripts/CardEffect/P/White/P_206.cs
- Color bypass: IgnoreColorConditionClass (unconditional, CardCondition is self)
- Main: OptionSkill timing, SimplifiedRevealDeckTopCardsAndSelect 3 cards,
    2 passes (Digimon + Tamer), PlaceDelayOptionCards
- Delay: OnDeclaration timing, plays Tamer with matching color from hand
    with cost reduced by 4 (changeCostFunc cost-=4)
- Security: SecuritySkill, play Digimon cost<=3 from hand or trash without
    paying cost, then AddThisCardToHand
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase, CardColor


# Card IDs used by tests
P_206 = "P-206"                 # Card under test (Option, White, cost 4)
ST1_03 = "ST1-03"               # Agumon (Red Digimon, Lv.3, cost 3)
ST1_12 = "ST1-12"               # Tai Kamiya (Red Tamer, cost 2)
BT1_001 = "BT1-001"             # Kuwagamon (Red Lv.3, cost 3)

# Minimal deck — mostly filler Agumons
DECK = [P_206] * 4 + [ST1_03] * 46


# ─── Structural Tests ────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestP206Structure:
    """Static tests for P-206 effect list shape and flags."""

    def test_color_requirement_unconditionally_bypassed(self):
        """P-206 sets _match_color_requirement = False (unconditional bypass)."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        # Force effect initialization
        cs.effect_list(EffectTiming.NoTiming)

        assert getattr(cs, '_match_color_requirement', True) is False, (
            "P-206 should unconditionally bypass color requirements"
        )
        # match_color_requirement property must return False
        assert cs.match_color_requirement is False, (
            "P-206.match_color_requirement property should be False"
        )

    def test_has_option_skill_main_effect(self):
        """P-206 should have an OptionSkill timing effect for [Main]."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1, (
            "P-206 should have an OptionSkill effect for its [Main] reveal"
        )
        assert option_skill[0].on_process_callback is not None, (
            "OptionSkill effect must have a process callback"
        )

    def test_has_delay_marker(self):
        """P-206 should have a _is_delay marker effect (enables place-in-BA)."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, (
            "P-206 should have a _is_delay marker so the option stays on field"
        )

    def test_delay_callback_follows_delay_marker(self):
        """The effect immediately after _is_delay must have a process callback."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        found = False
        for i, effect in enumerate(effects):
            if getattr(effect, '_is_delay', False):
                assert i + 1 < len(effects), (
                    "Delay marker must be followed by the delayed effect"
                )
                next_eff = effects[i + 1]
                assert next_eff.on_process_callback is not None, (
                    "Effect after _is_delay marker must have a process callback"
                )
                found = True
                break
        assert found, "No _is_delay marker found in effect list"

    def test_delay_callback_is_not_field_main(self):
        """The delay callback effect should NOT have _is_field_main = True.

        The engine exposes delay activation via a dedicated action slot
        (effectIdx=1). If the same effect also had _is_field_main=True,
        it would be reachable via effectIdx=2 as well, creating a duplicate
        action that fires the effect without trashing the Option.
        """
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        for i, effect in enumerate(effects):
            if getattr(effect, '_is_delay', False) and i + 1 < len(effects):
                next_eff = effects[i + 1]
                assert not getattr(next_eff, '_is_field_main', False), (
                    "Delay callback effect must NOT have _is_field_main=True"
                )
                break

    def test_has_security_effect(self):
        """P-206 should have a SecuritySkill effect with callback."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(P_206, None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill and e.is_security_effect
        ]
        assert len(sec_effects) >= 1, (
            "P-206 should have a SecuritySkill + is_security_effect entry"
        )
        assert sec_effects[0].on_process_callback is not None, (
            "Security effect must have a process callback (not an inert stub)"
        )


# ─── Color Bypass Tests ──────────────────────────────────────────────────


@pytest.mark.behavioral
class TestP206ColorBypass:
    """Tests for P-206 color requirement bypass during play action."""

    def test_playable_without_white_digimon_on_field(self, debug_runner):
        """P-206 (White) should be playable even with only Red Digimon
        on field — color requirement is unconditionally bypassed."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=5,
        )
        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")
        # Place a Red Digimon — does NOT match White
        runner.place_on_field(1, [ST1_03])

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None, (
            "P-206 should be playable without a matching-color "
            "Digimon/Tamer on the field"
        )

    def test_playable_with_empty_field(self, debug_runner):
        """P-206 should be playable even with no Digimon/Tamer on field."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=5,
        )
        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")
        # No field presence at all

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None, (
            "P-206 should be playable with no field Digimon (color bypass)"
        )


# ─── [Main] Reveal Effect Tests ──────────────────────────────────────────


@pytest.mark.behavioral
class TestP206MainReveal:
    """Tests for P-206 [Main] Reveal 3 / add Digimon + Tamer / place in BA."""

    def test_main_effect_uses_reveal_multi_with_two_passes(self):
        """The OptionSkill effect should call effect_reveal_and_select_multi
        with 2 filter passes (Digimon + Tamer)."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.game import Game
        from engine_py_legacy.engine.loggers import EventLogger

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        game = Game(EventLogger())
        cs = db.create_card_source(P_206, game.player1)
        effects = cs.effect_list(EffectTiming.NoTiming)

        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1
        effect = option_skill[0]

        captured = []
        original = game.effect_reveal_and_select_multi

        def mock_reveal(player, count, passes, **kwargs):
            captured.append({'count': count, 'passes': passes, **kwargs})

        game.effect_reveal_and_select_multi = mock_reveal
        try:
            effect.on_process_callback({
                'game': game,
                'player': game.player1,
            })
        finally:
            game.effect_reveal_and_select_multi = original

        assert len(captured) == 1, "Should call effect_reveal_and_select_multi once"
        call = captured[0]
        assert call['count'] == 3, f"Should reveal 3 cards, got {call['count']}"
        assert len(call['passes']) == 2, (
            f"Should have 2 passes (Digimon + Tamer), got {len(call['passes'])}"
        )
        assert call.get('remaining_placement') == 'deck_bottom', (
            "Remaining cards should go to deck bottom"
        )

        # Verify pass 1 filter matches Digimon, pass 2 matches Tamer
        digimon_filter, digimon_dest = call['passes'][0]
        tamer_filter, tamer_dest = call['passes'][1]
        assert digimon_dest == 'hand', "Digimon pass should place to hand"
        assert tamer_dest == 'hand', "Tamer pass should place to hand"

        # Stub CardSources for filter checks
        class StubCard:
            def __init__(self, is_digimon=False, is_tamer=False):
                self.is_digimon = is_digimon
                self.is_tamer = is_tamer

        assert digimon_filter(StubCard(is_digimon=True)) is True
        assert digimon_filter(StubCard(is_tamer=True)) is False
        assert tamer_filter(StubCard(is_tamer=True)) is True
        assert tamer_filter(StubCard(is_digimon=True)) is False

    def test_main_adds_digimon_and_tamer_to_hand_from_top_three(self, debug_runner):
        """Playing P-206 should reveal top 3 and agent selects 1 Digimon + 1 Tamer."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Give only P-206 in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")

        # Stack deck top with Digimon, Tamer, Digimon (all filler)
        # Clear library first and inject known cards
        runner.clear_zone(1, "library")
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        top1 = db.create_card_source(ST1_03, p1)   # Digimon
        top2 = db.create_card_source(ST1_12, p1)   # Tamer
        top3 = db.create_card_source(ST1_03, p1)   # Digimon
        # Append ensures top1 is drawn first
        p1.library_cards.append(top1)
        p1.library_cards.append(top2)
        p1.library_cards.append(top3)
        # Add padding for deck-out safety
        for _ in range(30):
            p1.library_cards.append(db.create_card_source(ST1_03, p1))

        hand_before = len(p1.hand_cards)  # 1 (P-206)
        lib_before = len(p1.library_cards)

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None, "Should find play action for P-206"
        runner.execute(action)
        runner.auto_resolve()

        # Expected: P-206 leaves hand to BA (hand -1), library drops by 3 revealed
        # (2 added to hand, 1 to deck bottom), hand gains 2 cards (Digimon + Tamer)
        # Net hand change: -1 (P-206 played) + 2 (added) = +1
        # Net library change: +1 (deck-bottom) - 3 (revealed) = -2
        assert len(p1.library_cards) == lib_before - 2, (
            f"Library should lose 2 cards net (3 revealed, 1 returned to bottom), "
            f"had {lib_before}, now {len(p1.library_cards)}"
        )
        # Hand should contain the selected Digimon and Tamer
        hand_ids = [
            c.c_entity_base.card_id for c in p1.hand_cards
            if c.c_entity_base
        ]
        assert ST1_03 in hand_ids, (
            f"Selected Digimon should be in hand, got {hand_ids}"
        )
        assert ST1_12 in hand_ids, (
            f"Selected Tamer should be in hand, got {hand_ids}"
        )

    def test_main_remainder_goes_to_deck_bottom(self, debug_runner):
        """The revealed card that isn't added to hand should go to deck bottom."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")

        # Stack deck top: Digimon, Tamer, EXTRA Digimon (unused)
        runner.clear_zone(1, "library")
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        p1.library_cards.append(db.create_card_source(ST1_03, p1))   # Digimon (selected)
        p1.library_cards.append(db.create_card_source(ST1_12, p1))   # Tamer (selected)
        # Use a uniquely identifiable remainder card
        remainder = db.create_card_source(BT1_001, p1)
        p1.library_cards.append(remainder)
        for _ in range(30):
            p1.library_cards.append(db.create_card_source(ST1_03, p1))

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Remainder should now be at the bottom of the library
        assert len(p1.library_cards) > 0
        bottom_card = p1.library_cards[-1]
        assert bottom_card is remainder, (
            f"Remainder card BT1-001 should be at bottom of library after reveal"
        )

    def test_main_places_option_in_battle_area(self, debug_runner):
        """After the Main effect, P-206 should remain in the battle area
        (for its Delay effect). This is handled automatically via _is_delay
        marker triggering _option_stays_on_field in the engine."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # P-206 should be in the battle area, not in trash
        p206_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == P_206
        ]
        p206_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == P_206
        ]
        assert len(p206_on_field) == 1, (
            f"P-206 should be placed in the battle area after its Main effect, "
            f"found {len(p206_on_field)} on field"
        )
        assert len(p206_in_trash) == 0, (
            f"P-206 should NOT be in trash after its Main effect "
            f"(it is a Delay Option), found {len(p206_in_trash)} in trash"
        )


# ─── [Main] <Delay> Effect Tests ─────────────────────────────────────────


@pytest.mark.behavioral
class TestP206Delay:
    """Tests for P-206 Delay: play 1 Tamer matching Digimon colors, cost -4."""

    def test_delay_not_available_same_turn(self, debug_runner):
        """Delay cannot be activated on the same turn the option was placed."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, P_206, "hand")

        runner.set_phase("Main")
        action = runner.find_action("Digital Gate")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Immediately after placing: delay should NOT be available
        delay_action = runner.find_action("Delay")
        assert delay_action is None, (
            "Delay should NOT be available on the same turn the option was placed"
        )

    def test_delay_available_after_placing_turn(self, debug_runner):
        """Delay should be available on a later turn after the Option is placed."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        # Place P-206 directly on field, simulating a prior turn
        runner.place_on_field(1, [P_206], turn_played=0)
        # Place a Digimon so the Tamer filter has something to match against
        runner.place_on_field(1, [ST1_03])
        # Put a matching-color Tamer in hand
        runner.inject_card(1, ST1_12, "hand")

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, (
            "Delay should be available after the placing turn"
        )

    def test_delay_activation_trashes_option(self, debug_runner):
        """Activating Delay should trash P-206 from the battle area to trash."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        p1 = game.player1

        runner.place_on_field(1, [P_206], turn_played=0)
        runner.place_on_field(1, [ST1_03])  # Red Digimon
        runner.inject_card(1, ST1_12, "hand")  # Red Tamer (Tai Kamiya)

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        # P-206 should now be in trash
        p206_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == P_206
        ]
        p206_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == P_206
        ]
        assert len(p206_in_trash) >= 1, (
            "P-206 should be trashed after Delay activation"
        )
        assert len(p206_on_field) == 0, (
            "P-206 should NOT remain on field after Delay activation"
        )

    def test_delay_calls_effect_play_from_zone_hand_manual_reduction_4(self, debug_runner):
        """Delay callback should call effect_play_from_zone on hand with
        manual_reduction=4 and free=False."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        p206_perm = runner.place_on_field(1, [P_206], turn_played=0)
        runner.place_on_field(1, [ST1_03])

        card = p206_perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        # Find the delay callback effect (after _is_delay marker)
        delay_callback = None
        for i, eff in enumerate(effects):
            if getattr(eff, '_is_delay', False) and i + 1 < len(effects):
                delay_callback = effects[i + 1]
                break
        assert delay_callback is not None, "Delay callback not found"
        assert delay_callback.on_process_callback is not None

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append({'zone': zone, 'filter_fn': filter_fn, **kwargs})

        game.effect_play_from_zone = mock_play
        try:
            delay_callback.on_process_callback({
                'game': game,
                'player': game.player1,
                'permanent': p206_perm,
                'card': card,
            })
        finally:
            game.effect_play_from_zone = original

        assert len(captured) == 1, "Delay should call effect_play_from_zone once"
        call = captured[0]
        assert call['zone'] == 'hand', (
            f"Delay should play from hand, got {call['zone']}"
        )
        assert call.get('free') is False, (
            "Delay should NOT play for free (cost is reduced, not waived)"
        )
        assert call.get('manual_reduction') == 4, (
            f"Delay should use manual_reduction=4, got {call.get('manual_reduction')}"
        )

    def test_delay_filter_accepts_tamer_matching_field_digimon_color(self, debug_runner):
        """Delay filter should accept Tamers whose colors intersect with
        the colors of Digimon on the player's field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        p206_perm = runner.place_on_field(1, [P_206], turn_played=0)
        # Field has a Red Digimon (ST1-03 Agumon)
        runner.place_on_field(1, [ST1_03])

        card = p206_perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        delay_callback = None
        for i, eff in enumerate(effects):
            if getattr(eff, '_is_delay', False) and i + 1 < len(effects):
                delay_callback = effects[i + 1]
                break
        assert delay_callback is not None

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append(filter_fn)

        game.effect_play_from_zone = mock_play
        try:
            delay_callback.on_process_callback({
                'game': game,
                'player': game.player1,
                'permanent': p206_perm,
                'card': card,
            })
        finally:
            game.effect_play_from_zone = original

        assert len(captured) == 1
        filter_fn = captured[0]

        # ST1-12 Tai Kamiya is a Red Tamer — should match Red Digimon on field
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        red_tamer = db.create_card_source(ST1_12, game.player1)
        assert filter_fn(red_tamer) is True, (
            "Red Tamer should be accepted when a Red Digimon is on field"
        )

        # A Digimon (non-Tamer) should NOT be accepted
        red_digimon = db.create_card_source(ST1_03, game.player1)
        assert filter_fn(red_digimon) is False, (
            "Non-Tamer (Digimon) should NOT be accepted by the delay filter"
        )

    def test_delay_filter_rejects_tamer_with_no_color_match(self, debug_runner):
        """Delay filter should REJECT Tamers whose colors don't intersect
        with any Digimon on the player's field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        p206_perm = runner.place_on_field(1, [P_206], turn_played=0)
        # Place a Digimon with a non-Red color on field — use Zephagamon P-206 itself
        # Simpler: put NO Digimon on field, only a Tamer, so no color match
        # to Digimons possible.

        card = p206_perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        delay_callback = None
        for i, eff in enumerate(effects):
            if getattr(eff, '_is_delay', False) and i + 1 < len(effects):
                delay_callback = effects[i + 1]
                break
        assert delay_callback is not None

        captured = []

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append(filter_fn)

        original = game.effect_play_from_zone
        game.effect_play_from_zone = mock_play
        try:
            delay_callback.on_process_callback({
                'game': game,
                'player': game.player1,
                'permanent': p206_perm,
                'card': card,
            })
        finally:
            game.effect_play_from_zone = original

        assert len(captured) == 1
        filter_fn = captured[0]

        # With no Digimon on field, no Tamer should pass the color filter
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        red_tamer = db.create_card_source(ST1_12, game.player1)
        assert filter_fn(red_tamer) is False, (
            "Tamer should NOT match when there are no Digimon on field"
        )


# ─── [Security] Effect Tests ────────────────────────────────────────────


@pytest.mark.behavioral
class TestP206Security:
    """Tests for P-206 [Security] play Digimon cost<=3 free, add this to hand."""

    def test_security_calls_play_from_hand_or_trash_free(self, debug_runner):
        """Security effect should call effect_play_from_zone with
        'hand_or_trash' zone, free=True."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P_206, game.player2)
        effects = cs.effect_list(EffectTiming.NoTiming)
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill and e.is_security_effect
        ]
        assert len(sec_effects) == 1
        effect = sec_effects[0]

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append({'zone': zone, 'filter_fn': filter_fn, **kwargs})

        game.effect_play_from_zone = mock_play
        try:
            effect.on_process_callback({
                'game': game,
                'player': game.player2,
                'card': cs,
            })
        finally:
            game.effect_play_from_zone = original

        assert len(captured) == 1
        call = captured[0]
        assert call['zone'] == 'hand_or_trash', (
            f"Security should play from hand_or_trash, got {call['zone']}"
        )
        assert call.get('free') is True, (
            "Security should play WITHOUT paying cost"
        )

    def test_security_filter_accepts_digimon_cost_3(self, debug_runner):
        """Security filter should accept a Digimon with cost <= 3."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P_206, game.player2)
        effects = cs.effect_list(EffectTiming.NoTiming)
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill and e.is_security_effect
        ]
        effect = sec_effects[0]

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append(filter_fn)

        game.effect_play_from_zone = mock_play
        try:
            effect.on_process_callback({
                'game': game,
                'player': game.player2,
                'card': cs,
            })
        finally:
            game.effect_play_from_zone = original

        assert len(captured) == 1
        filter_fn = captured[0]

        # ST1-03 Agumon — Digimon, cost 3 — should pass
        cost3_digimon = db.create_card_source(ST1_03, game.player2)
        assert filter_fn(cost3_digimon) is True, (
            "Cost-3 Digimon should be accepted by security filter"
        )

    def test_security_filter_rejects_digimon_cost_above_3(self, debug_runner):
        """Security filter should REJECT a Digimon with cost > 3."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P_206, game.player2)
        effects = cs.effect_list(EffectTiming.NoTiming)
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill and e.is_security_effect
        ]
        effect = sec_effects[0]

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append(filter_fn)

        game.effect_play_from_zone = mock_play
        try:
            effect.on_process_callback({
                'game': game,
                'player': game.player2,
                'card': cs,
            })
        finally:
            game.effect_play_from_zone = original

        filter_fn = captured[0]

        # Create a fake cost-5 Digimon CardSource
        class StubCard:
            is_digimon = True
            is_tamer = False
            is_option = False
            get_cost_itself = 5

        assert filter_fn(StubCard()) is False, (
            "Cost-5 Digimon should be rejected (filter needs cost <= 3)"
        )

    def test_security_filter_rejects_tamers_and_options(self, debug_runner):
        """Security filter should REJECT Tamer / Option cards — only Digimon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(P_206, game.player2)
        effects = cs.effect_list(EffectTiming.NoTiming)
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill and e.is_security_effect
        ]
        effect = sec_effects[0]

        captured = []
        original = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, **kwargs):
            captured.append(filter_fn)

        game.effect_play_from_zone = mock_play
        try:
            effect.on_process_callback({
                'game': game,
                'player': game.player2,
                'card': cs,
            })
        finally:
            game.effect_play_from_zone = original

        filter_fn = captured[0]

        class StubTamer:
            is_digimon = False
            is_tamer = True
            is_option = False
            get_cost_itself = 2

        class StubOption:
            is_digimon = False
            is_tamer = False
            is_option = True
            get_cost_itself = 2

        assert filter_fn(StubTamer()) is False, "Tamer should be rejected"
        assert filter_fn(StubOption()) is False, "Option should be rejected"

    def test_security_adds_card_to_hand_end_to_end(self, debug_runner):
        """End-to-end: after a security check triggers P-206's security effect,
        the P-206 card should end up in the defender's hand (not trash)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # Put a cost-3 Digimon in P2's hand so the play-from-hand path runs
        runner.clear_zone(2, "hand")
        runner.inject_card(2, ST1_03, "hand")

        # Inject P-206 as top security card
        runner.clear_zone(2, "security")
        runner.inject_card(2, P_206, "security_top")

        # Use an attacker strong enough to deal security damage
        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=12000)
        game.player1.battle_area.append(attacker)

        hand_before = len(p2.hand_cards)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # P-206 should be in P2's hand after the security effect
        p206_in_hand = [
            c for c in p2.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == P_206
        ]
        p206_in_trash = [
            c for c in p2.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == P_206
        ]
        assert len(p206_in_hand) >= 1, (
            f"P-206 should be added to hand after security effect, "
            f"hand now has {[c.c_entity_base.card_id for c in p2.hand_cards if c.c_entity_base]}"
        )
        assert len(p206_in_trash) == 0, (
            f"P-206 should NOT be in trash after security effect (it is added to hand)"
        )
