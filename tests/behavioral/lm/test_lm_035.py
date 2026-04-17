"""Behavioral tests for LM-035 Amber Memory Boost! (Option, Yellow, Cost 3).

Card text (source of truth):
  Purple also meets this card's color requirements.
  [Main] Reveal the top 3 cards of your deck. Add 1 yellow or purple Digimon
      card among them to the hand. Return the rest to the bottom of deck.
      Then, place this card in the battle area.
  [Main] <Delay> (By trashing this card after the placing turn, activate
      the effect below.)
      - Gain 2 memory.
  [Security] Place this card in the battle area.

Clauses tested:
  C1. Color bypass: Purple on field allows play; no Y/P means cannot play;
      Red-only field cannot play.
  C2. Main effect: reveal 3, add 1 Y/P Digimon to hand, rest to deck bottom,
      and place self in battle area (delay).
  C3. Delay: can't activate same turn; activation trashes card and grants
      +2 memory.
  C4. Security: place self in battle area (effect_play_from_security path).
"""

from __future__ import annotations

import pytest

from digimon_gym.engine.data.enums import CardColor, EffectTiming


# ── Deck helpers ────────────────────────────────────────────────────────
# Yellow level-3 singleton-color: BT1-048 Patamon
# Purple level-3 singleton-color: BT2-068 Impmon
# Red    level-3 singleton-color: ST1-03  Agumon
# Yellow level-4: BT1-053 Angemon
# Purple level-4: BT2-073 Wizardmon
# Non-Digimon yellow Option fillers are not needed; we use a vanilla Option
# BT2-066 (Purple Digimon) and generic filler BT1-010 for padding.

_DECK = (
    ["LM-035"] * 4
    + ["BT1-048"] * 4    # Yellow Patamon (Lv3, singleton color)
    + ["BT2-068"] * 4    # Purple Impmon (Lv3, singleton color)
    + ["ST1-03"] * 4     # Red Agumon (Lv3) — negative test
    + ["BT1-053"] * 4    # Yellow Angemon (Lv4)
    + ["BT1-010"] * 30   # Filler (Red Agumon)
)


# ======================================================================
# C1 — Partial color bypass: "Purple also meets this card's color requirements"
# ======================================================================

@pytest.mark.behavioral
class TestLM035ColorBypass:
    """Purple also meets LM-035's color requirements."""

    def test_match_color_requirement_fn_exists(self, debug_runner):
        """LM-035 should set _match_color_requirement_fn for dynamic partial bypass."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        # Trigger effect_list so the script runs and sets the fn
        cs.effect_list(None)
        fn = getattr(cs, '_match_color_requirement_fn', None)
        assert fn is not None, (
            "LM-035 must set _match_color_requirement_fn for partial color bypass "
            "(Purple also meets color requirements)."
        )

    def test_color_requirement_without_owner_enforces_yellow(self, debug_runner):
        """With empty field, color requirement defaults to enforced (no Y/P on field)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        game = runner.game
        player1 = game.player1

        # Clear field and hand
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        # Trigger effect_list
        lm035 = player1.hand_cards[-1]
        lm035.effect_list(None)

        # No yellow/purple on field → should NOT be playable
        actions = runner.actions()
        lm035_play = None
        for aid, desc in actions.items():
            if "Amber Memory Boost" in desc or "LM-035" in desc:
                lm035_play = aid
                break
        assert lm035_play is None, (
            f"LM-035 should NOT be playable with no yellow/purple on field. "
            f"Actions: {actions}"
        )

    def test_playable_with_yellow_digimon_on_field(self, debug_runner):
        """With Yellow Digimon on field, LM-035 is playable (normal color req)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon
        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None, (
            f"LM-035 should be playable with Yellow Digimon on field. "
            f"Actions: {runner.actions()}"
        )

    def test_playable_with_purple_digimon_on_field(self, debug_runner):
        """C1: Purple Digimon on field allows LM-035 to be played (partial bypass)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT2-068"])  # Purple Impmon
        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None, (
            f"LM-035 should be playable with ONLY Purple Digimon on field "
            f"(partial color bypass). Actions: {runner.actions()}"
        )

    def test_not_playable_with_red_only_field(self, debug_runner):
        """C1: Red-only field (no yellow, no purple) should NOT allow LM-035."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["ST1-03"])  # Red Agumon
        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is None, (
            f"LM-035 should NOT be playable with only Red Digimon on field. "
            f"Actions: {runner.actions()}"
        )


# ======================================================================
# C2 — [Main] reveal 3, add 1 yellow/purple Digimon, rest to bottom, place self
# ======================================================================

@pytest.mark.behavioral
class TestLM035MainEffect:
    """[Main] reveal 3; add 1 Y/P Digimon; rest to bottom; place in battle area."""

    def test_main_effect_timing(self, debug_runner):
        """Main effect must use OptionSkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        effects = cs.effect_list(None)
        main = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main) >= 1, (
            f"Must have OptionSkill main effect. Timings: "
            f"{[(e.effect_name, e.timing) for e in effects]}"
        )

    def test_main_effect_places_card_in_battle_area(self, debug_runner):
        """After playing main effect, LM-035 stays in battle area (Delay)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon

        # Stack deck top: 3 cards, at least one yellow/purple digimon
        runner.clear_zone(1, "library")
        runner.inject_card(1, "BT1-048", "library_top")  # Yellow Digimon at slot 2
        runner.inject_card(1, "ST1-03", "library_top")   # Red at slot 1
        runner.inject_card(1, "BT2-068", "library_top")  # Purple at slot 0
        # Pad the deck so we can return rest to bottom
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")

        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None, f"LM-035 not in actions: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-035" for s in snap.p1_field)
        assert in_battle, "LM-035 should be placed in battle area after main effect"

    def test_main_effect_yellow_selectable(self, debug_runner):
        """Reveal filter should include yellow Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon on field

        runner.clear_zone(1, "library")
        # Top 3: Yellow Patamon (BT1-048), Red filler, Red filler
        runner.inject_card(1, "BT1-010", "library_top")  # #2 Red
        runner.inject_card(1, "BT1-010", "library_top")  # #1 Red
        runner.inject_card(1, "BT1-048", "library_top")  # #0 Yellow Patamon
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")

        hand_before = len(runner.game.player1.hand_cards)

        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Hand should contain Yellow Patamon now (selected from reveal)
        hand_ids = [
            c.c_entity_base.card_id for c in runner.game.player1.hand_cards
        ]
        assert "BT1-048" in hand_ids, (
            f"Yellow Patamon should be in hand after reveal select. Hand: {hand_ids}"
        )

    def test_main_effect_purple_selectable(self, debug_runner):
        """Reveal filter should also include purple Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon on field

        runner.clear_zone(1, "library")
        # Top 3: Purple Impmon, Red filler, Red filler
        runner.inject_card(1, "BT1-010", "library_top")  # #2 Red
        runner.inject_card(1, "BT1-010", "library_top")  # #1 Red
        runner.inject_card(1, "BT2-068", "library_top")  # #0 Purple Impmon
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")

        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        hand_ids = [
            c.c_entity_base.card_id for c in runner.game.player1.hand_cards
        ]
        assert "BT2-068" in hand_ids, (
            f"Purple Impmon should be in hand after reveal select. Hand: {hand_ids}"
        )

    def test_main_effect_rest_returned_to_bottom(self, debug_runner):
        """Non-selected revealed cards should go to the bottom of the deck."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon on field

        runner.clear_zone(1, "library")
        # Top 3: Yellow Patamon (selected), Red Agumon (#1), Red Agumon (#2) — go to bottom
        # Then filler for remaining deck
        runner.inject_card(1, "ST1-03", "library_bottom")  # filler so top-3 are set
        # But we want the top 3 EXACTLY set — use library_top in reverse
        runner.clear_zone(1, "library")
        # Fill with 10 filler cards first as bottom padding
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")
        # Now top 3 in reverse order
        runner.inject_card(1, "ST1-03", "library_top")   # slot 2 — non-selected
        runner.inject_card(1, "ST1-03", "library_top")   # slot 1 — non-selected
        runner.inject_card(1, "BT1-048", "library_top")  # slot 0 — selected

        lib_size_before = len(runner.game.player1.library_cards)

        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Library should shrink by 1 (the selected card left the deck)
        lib_after = runner.game.player1.library_cards
        assert len(lib_after) == lib_size_before - 1, (
            f"Library should shrink by exactly 1 (selected card). "
            f"before={lib_size_before} after={len(lib_after)}"
        )

        # The last 2 cards of the deck should be the two ST1-03 (Red Agumon)
        # placed to bottom.
        last_two = [c.c_entity_base.card_id for c in lib_after[-2:]]
        assert last_two.count("ST1-03") == 2, (
            f"Two non-selected Red Agumons should be at deck bottom. "
            f"Last two: {last_two}"
        )

    def test_main_effect_no_yp_in_reveal_still_bottoms_cards(self, debug_runner):
        """If reveal has no Y/P Digimon, cards still placed at bottom (no selection)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "hand")
        runner.place_on_field(1, ["BT1-048"])  # Yellow Patamon on field

        runner.clear_zone(1, "library")
        # Pad library bottom
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")
        # Top 3 are all Red (no yellow/purple)
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")
        runner.inject_card(1, "ST1-03", "library_top")

        lib_size_before = len(runner.game.player1.library_cards)

        runner.inject_card(1, "LM-035", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Amber Memory Boost")
        if action is None:
            action = runner.find_action("LM-035")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # No valid target → no card added to hand, all 3 go to bottom
        lib_after = runner.game.player1.library_cards
        assert len(lib_after) == lib_size_before, (
            f"Library should be same size if no valid selection. "
            f"before={lib_size_before} after={len(lib_after)}"
        )

        # LM-035 should still be placed in battle area regardless
        snap = runner.snapshot()
        in_battle = any(s.card_id == "LM-035" for s in snap.p1_field)
        assert in_battle, (
            "LM-035 should still be placed in battle area even if no Y/P in reveal"
        )


# ======================================================================
# C3 — [Main] <Delay> Gain 2 memory
# ======================================================================

@pytest.mark.behavioral
class TestLM035DelayEffect:
    """<Delay> — trash self after placing turn → gain 2 memory."""

    def test_delay_marker_exists(self, debug_runner):
        """LM-035 must have a delay marker effect with _is_delay=True."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        effects = cs.effect_list(None)
        delay = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay) >= 1, "Must have Delay marker"

    def test_delay_effect_has_callback(self, debug_runner):
        """The effect following the delay marker should have a process callback."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        effects = cs.effect_list(None)
        # Find the delay marker, then check the next effect
        for idx, e in enumerate(effects):
            if getattr(e, '_is_delay', False):
                assert idx + 1 < len(effects), "Must have delay body after marker"
                body = effects[idx + 1]
                assert body.on_process_callback is not None, (
                    "Delay body effect must have a process callback"
                )
                return
        pytest.fail("No delay marker found")

    def test_delay_activation_trashes_card_and_grants_memory(self, debug_runner):
        """Activating delay on LM-035 trashes it and grants +2 memory."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        runner.place_on_field(1, ["LM-035"], turn_played=-1)  # placed on prior turn

        memory_before = runner.game.memory
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None, (
            f"Should have Delay action. Actions: {runner.actions()}"
        )
        runner.execute(delay_action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # LM-035 should no longer be in battle area
        assert not any(s.card_id == "LM-035" for s in snap.p1_field), (
            "LM-035 should be trashed after delay activation"
        )
        # LM-035 should be in trash
        trash_ids = [
            c.c_entity_base.card_id for c in runner.game.player1.trash_cards
        ]
        assert "LM-035" in trash_ids, (
            f"LM-035 should be in trash after delay. Trash: {trash_ids}"
        )
        # Memory should have increased by 2 (active player perspective)
        memory_delta = runner.game.memory - memory_before
        assert memory_delta == 2, (
            f"Memory should increase by 2, got delta={memory_delta} "
            f"(before={memory_before}, after={runner.game.memory})"
        )

    def test_delay_not_activatable_same_turn(self, debug_runner):
        """Delay cannot be activated on the same turn the card is placed."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3, skip_shuffle=True)

        # Place with turn_played = current turn → should be same-turn (no delay)
        runner.place_on_field(
            1, ["LM-035"],
            turn_played=runner.game.turn_count,
        )
        runner.set_phase("Main")

        # No delay action should be available
        delay_action = runner.find_action("Delay")
        assert delay_action is None, (
            f"Delay should NOT be activatable same turn. Actions: {runner.actions()}"
        )

    def test_delay_condition_does_not_check_permanent_of_this_card(self, debug_runner):
        """Delay-body condition must not return False based on permanent_of_this_card.

        The engine's _execute_delay removes the permanent BEFORE invoking the
        callback, so the condition must not reject based on a missing permanent
        (BT22-099 / LM-030 discovery).
        """
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3)
        game = runner.game
        cs = db.create_card_source("LM-035", game.player1)
        effects = cs.effect_list(None)

        # Find the delay body (effect after delay marker)
        delay_body = None
        for idx, e in enumerate(effects):
            if getattr(e, '_is_delay', False):
                delay_body = effects[idx + 1]
                break
        assert delay_body is not None

        # After the engine trashes the card, permanent_of_this_card() is None.
        # Simulate that state: card is owned by player1 but has no permanent.
        # The condition should NOT return False solely because of that.
        game.turn_player = game.player1  # ensure is_my_turn is True

        # card has no permanent in battle area → permanent_of_this_card() is None
        # but the Delay body must still be callable.
        result = delay_body.can_use_condition({
            'game': game,
            'player': game.player1,
            'card': cs,
        })
        # Either True (good — will run) or the condition is trivially True
        assert result is True or result is None, (
            f"Delay body condition must not reject based on missing permanent. "
            f"Got {result}"
        )


# ======================================================================
# C4 — [Security] Place self in battle area
# ======================================================================

@pytest.mark.behavioral
class TestLM035SecurityEffect:
    """[Security] Place this card in the battle area."""

    def test_security_effect_exists(self, debug_runner):
        """Must have SecuritySkill effect flagged as security effect."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1, "Must have SecuritySkill effect"
        assert sec[0].is_security_effect is True, (
            "Security effect must have is_security_effect=True"
        )

    def test_security_effect_uses_effect_play_from_security(self, debug_runner):
        """The process callback must use game.effect_play_from_security."""
        import inspect
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("LM-035")
        effects = cs.effect_list(None)
        sec = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec) >= 1
        cb = sec[0].on_process_callback
        assert cb is not None, "Security effect must have a process callback"

        # Check that the callback source references effect_play_from_security.
        try:
            src = inspect.getsource(cb)
        except (OSError, TypeError):
            src = ""
        assert "effect_play_from_security" in src, (
            "Security callback should call game.effect_play_from_security "
            "so On Play effects fire and the card stays on field "
            f"(instead of being trashed post-security-attack). Source: {src}"
        )

    def test_security_places_card_in_battle_area(self, debug_runner):
        """After security attack, LM-035 should end up in the battle area."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10, skip_shuffle=True)

        # Give P2 an attacker, P1 nothing but LM-035 on top of security.
        runner.clear_zone(1, "field")
        runner.clear_zone(1, "security")
        # Deck filler so any reveal during play is safe
        for _ in range(10):
            runner.inject_card(1, "BT1-010", "library_bottom")
        # Security top = LM-035 (will be revealed on attack)
        runner.inject_card(1, "LM-035", "security_top")

        runner.place_on_field(2, ["BT2-068"])  # P2 Purple Impmon attacker
        # Suspend the attacker to simulate attacking state? — we'll use actual attack flow.
        # Instead of full attack flow, invoke security check directly.

        # Use game.security_attack or the effect's callback
        game = runner.game
        p1 = game.player1
        lm035_sec = p1.security_cards[0]

        # Manually fire the security effect (mimicking what happens on security attack)
        effects = lm035_sec.effect_list(None)
        sec_eff = [e for e in effects if e.timing == EffectTiming.SecuritySkill][0]

        # Remove from security first (engine does this pre-callback)
        p1.security_cards.remove(lm035_sec)
        ctx = {'player': p1, 'game': game, 'card': lm035_sec}
        sec_eff.on_process_callback(ctx)

        # LM-035 should now be in battle area, with _security_played set.
        in_battle = any(
            p.top_card and p.top_card.c_entity_base.card_id == "LM-035"
            for p in p1.battle_area
        )
        assert in_battle, (
            f"LM-035 should be in battle area after security effect. "
            f"Battle area: {[p.top_card.c_entity_base.card_id for p in p1.battle_area if p.top_card]}"
        )
        # The _security_played flag prevents the security_attack routine from
        # trashing it.
        assert getattr(lm035_sec, '_security_played', False) is True, (
            "Card should be marked _security_played so it won't be trashed"
        )
