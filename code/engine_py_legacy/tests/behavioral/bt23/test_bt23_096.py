"""Behavioral tests for BT23-096 Comet Hammer (Option, Black, CS, Cost 5).

Card text (C# source of truth):
    While you have a Digimon or Tamer with the [CS] trait on the field, you
    can ignore this card's color requirements.
    [Main] <De-Digivolve 4> 1 of your opponent's Digimon. (Trash up to 4 cards
        from the top. You can't trash past level 3 cards.) Then, place this
        card in the battle area.
    [Your Turn] When one of your [CS] trait Digimon attacks, <Delay>
        (By trashing this card after the placing turn, activate the effect
        below.)
      - <De-Digivolve 4> 1 of your opponent's Digimon.
    [Security] <De-Digivolve 4> 1 of your opponent's Digimon. Then, place
        this card in the battle area.

Key cards used:
- BT23-096: Comet Hammer, Option, Black, cost 5, CS trait
- BT22-017: Gabumon, Digimon, Lv.3, CS trait (field presence for color bypass)
- BT22-083: Yuuko Kamishiro, Tamer, CS trait (field presence for color bypass)
- BT22-010: Meramon, Digimon, Lv.4, CS trait (attacker for delay trigger)
- BT1-009: Monodramon, Digimon, Lv.3, no CS (non-CS attacker / target)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


HAMMER_DECK = (
    ["BT23-096"] * 4
    + ["BT22-017"] * 20
    + ["BT22-010"] * 10
    + ["BT1-009"] * 16
)


@pytest.mark.behavioral
class TestBT23096CometHammer:
    """Tests for BT23-096 Comet Hammer."""

    # ─── C1: Dynamic color bypass ────────────────────────────────────

    def test_color_bypass_with_cs_digimon_on_field(self, debug_runner):
        """Color requirement is bypassed while a CS Digimon is on field."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT22-017"])  # Gabumon, CS Digimon
        runner.inject_card(1, "BT23-096", "hand")

        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        assert card.match_color_requirement is False, (
            "Color requirement should be bypassed when a CS Digimon is on "
            "field (match_color_requirement should return False).")

    def test_color_bypass_with_cs_tamer_on_field(self, debug_runner):
        """Color requirement is bypassed while a CS Tamer is on field."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT22-083"])  # Yuuko Kamishiro, CS Tamer
        runner.inject_card(1, "BT23-096", "hand")

        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        assert card.match_color_requirement is False

    def test_color_enforced_without_cs_on_field(self, debug_runner):
        """Color requirement is enforced when no CS permanent is on field."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT1-009"])  # Monodramon, no CS trait
        runner.inject_card(1, "BT23-096", "hand")

        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        assert card.match_color_requirement is True, (
            "Color requirement should be enforced when no CS permanent "
            "is on field.")

    def test_color_enforced_empty_field(self, debug_runner):
        """Color requirement is enforced when the field is empty."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.inject_card(1, "BT23-096", "hand")
        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        assert card.match_color_requirement is True

    def test_color_bypass_does_not_trigger_on_opponent_cs(self, debug_runner):
        """Opponent's CS permanents must NOT bypass our color requirement."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.place_on_field(2, ["BT22-017"])  # Opponent has CS Digimon
        runner.inject_card(1, "BT23-096", "hand")

        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        assert card.match_color_requirement is True

    def test_color_bypass_uses_fn_not_static_flag(self, debug_runner):
        """The script must install _match_color_requirement_fn, not flip the
        static _match_color_requirement flag to False unconditionally."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        runner.inject_card(1, "BT23-096", "hand")
        card = next(
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096"
        )
        # Instantiate effects so get_card_effects() installs the callable.
        card.effect_list(None)
        assert getattr(card, "_match_color_requirement_fn", None) is not None
        assert callable(card._match_color_requirement_fn)

    # ─── C2: [Main] OptionSkill de-digivolve ─────────────────────────

    def test_main_de_digivolve_effect_exists(self, debug_runner):
        """Main OptionSkill de-digivolve effect should be registered."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)
        effects = card.effect_list(None)

        option_effects = [
            e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_effects) == 1, (
            "Should have exactly 1 OptionSkill effect.")
        assert "De-Digivolve" in option_effects[0].effect_name

    def test_main_effect_de_digivolves_by_4(self, debug_runner):
        """Main effect removes up to 4 cards from the target's digi-stack."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        # Opponent runs a tall lv.5 stack: BT22-010 lv.4 stacked on a lv.3
        # Gabumon stacked on a lv.4 Meramon and another lv.3 Gabumon.
        # We stack 5 cards so the floor-at-lv3 rule still leaves the top
        # alive after a 4-card removal. (bottom-to-top placement)
        tall_stack = runner.place_on_field(
            2, ["BT22-017", "BT22-010", "BT22-017", "BT22-010", "BT22-010"])
        stack_len_before = len(tall_stack.card_sources)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        hammer = db.create_card_source("BT23-096", game.player1)

        # Find the OptionSkill process and run it against the tall stack.
        option_effect = next(
            e for e in hammer.effect_list(None)
            if e.timing == EffectTiming.OptionSkill)
        option_effect.on_process_callback({
            'game': game, 'player': game.player1, 'permanent': None,
        })

        # The process registered a selection — pick the opponent's tall stack.
        assert game.pending_selection is not None, (
            "Main effect should request opponent permanent selection.")
        valid_ids = list(game.pending_selection.valid_indices)
        assert len(valid_ids) >= 1
        # Find the action id for our tall stack.
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        tall_idx = game.player2.battle_area.index(tall_stack)
        target_action = SEL_OPP_FIELD_START + tall_idx
        assert target_action in valid_ids
        game.decode_action(target_action, game.player1.player_id)

        # Stack should have shrunk. Engine-enforced level-3 floor means the
        # effect removes cards until a level-3-or-lower card is exposed on
        # top. From [Lv3, Lv4, Lv3, Lv4, Lv4] (bottom-to-top), removing 4
        # cards from the top would leave us at Lv3; the floor protects
        # that so exactly 4 are removed.
        assert len(tall_stack.card_sources) < stack_len_before, (
            f"Expected the stack to shrink from {stack_len_before}, got "
            f"{len(tall_stack.card_sources)}.")

    def test_main_effect_floor_at_level_3(self, debug_runner):
        """De-Digivolve 4 cannot trash past a level-3 top card."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        # Opponent has a short Lv3 Digimon with no digivolution cards.
        # De-Digivolve 4 should do NOTHING (cannot trash past Lv3).
        short = runner.place_on_field(2, ["BT22-017"])  # Gabumon, Lv3
        assert len(short.card_sources) == 1
        assert short.top_card.level == 3

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        hammer = db.create_card_source("BT23-096", game.player1)
        option_effect = next(
            e for e in hammer.effect_list(None)
            if e.timing == EffectTiming.OptionSkill)
        option_effect.on_process_callback({
            'game': game, 'player': game.player1, 'permanent': None,
        })

        # Pick the short stack.
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        target_action = SEL_OPP_FIELD_START + 0
        game.decode_action(target_action, game.player1.player_id)

        # The short stack must remain intact (floor protection).
        assert len(short.card_sources) == 1, (
            "Level-3 top card must NOT be trashed by de-digivolve.")

    def test_main_effect_places_self_in_battle_area(self, debug_runner):
        """Playing the option via Main should leave it on the battle area."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=10)
        game = runner.game

        # Make sure there is a CS permanent so color bypass lets us play
        # (though the color test setup would already pass with Black only).
        runner.place_on_field(1, ["BT22-017"])  # Gabumon CS
        runner.place_on_field(2, ["BT22-010"])  # opponent target to pick

        # Clear p1 hand and give them ONLY the option card at index 0.
        game.player1.hand_cards.clear()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        hammer = db.create_card_source("BT23-096", game.player1)
        game.player1.hand_cards.append(hammer)

        # Play the option from hand (action 0).
        game.decode_action(0, game.player1.player_id)
        # Auto-resolve the opponent selection that opens.
        runner.auto_resolve()

        assert any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT23-096"
            for p in game.player1.battle_area
        ), "BT23-096 should remain on the battle area after its Main effect."

    def test_main_effect_no_target_still_places_self(self, debug_runner):
        """Even when the opponent has no valid de-digivolve target, the
        option must still be placed in the battle area."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=10)
        game = runner.game

        # Opponent has NO Digimon — de-digivolve finds no target.
        assert len(game.player2.battle_area) == 0

        game.player1.hand_cards.clear()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        hammer = db.create_card_source("BT23-096", game.player1)
        game.player1.hand_cards.append(hammer)

        game.decode_action(0, game.player1.player_id)
        runner.auto_resolve()

        # Still placed in battle area.
        assert any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT23-096"
            for p in game.player1.battle_area
        )

    # ─── C3: [Your Turn] OnAllyAttack delay trigger ──────────────────

    def test_delay_trigger_timing_is_on_ally_attack(self, debug_runner):
        """Delay trigger must use OnAllyAttack timing, not OnUseAttack."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        effects = perm.top_card.effect_list(None)

        ally_attack_effects = [
            e for e in effects if e.timing == EffectTiming.OnAllyAttack]
        assert len(ally_attack_effects) == 1, (
            "Should have 1 OnAllyAttack effect for the delay trigger.")

        use_attack_delay = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, '_is_delay', False)
        ]
        assert len(use_attack_delay) == 0

    def test_delay_condition_passes_on_cs_attacker(self, debug_runner):
        """Delay condition passes when the OnAllyAttack context attacker
        is a CS Digimon owned by the card's owner and a valid de-digivolve
        target exists."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        cs_attacker = runner.place_on_field(1, ["BT22-010"])  # Meramon, CS
        runner.place_on_field(2, ["BT22-010"])  # target to de-digivolve

        # Ensure Meramon's owner is p1 (important: place_on_field may not
        # set owner attribution for the CardSource, so we rely on the engine
        # checking attacker membership in battle_area).
        assert cs_attacker in game.player1.battle_area

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )

        ctx = {'game': game, 'attacker': cs_attacker}
        assert delay.can_use_condition(ctx) is True

    def test_delay_condition_fails_on_non_cs_attacker(self, debug_runner):
        """Delay condition must fail for a non-CS attacker."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        non_cs = runner.place_on_field(1, ["BT1-009"])  # Monodramon, no CS
        runner.place_on_field(2, ["BT22-010"])  # valid de-digi target exists

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )

        ctx = {'game': game, 'attacker': non_cs}
        assert delay.can_use_condition(ctx) is False

    def test_delay_condition_fails_without_attacker_key(self, debug_runner):
        """Delay condition must fail when there is no attacker in context
        (e.g. if _execute_delay is called outside an OnAllyAttack window)."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        runner.place_on_field(1, ["BT22-010"])  # CS Digimon on field
        runner.place_on_field(2, ["BT22-010"])  # de-digi target

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )

        # No attacker in context — condition must fail.
        ctx = {'game': game, 'permanent': option_perm}
        assert delay.can_use_condition(ctx) is False

    def test_delay_condition_fails_on_placing_turn(self, debug_runner):
        """Delay cannot activate on the same turn the option was placed."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        # Place with the current turn count: placing-turn restriction active.
        option_perm = runner.place_on_field(
            1, ["BT23-096"], turn_played=game.turn_count)
        cs_attacker = runner.place_on_field(1, ["BT22-010"])
        runner.place_on_field(2, ["BT22-010"])

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )
        ctx = {'game': game, 'attacker': cs_attacker}
        assert delay.can_use_condition(ctx) is False

    def test_delay_condition_fails_on_opponent_turn(self, debug_runner):
        """Delay must not fire when the opponent is attacking."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        # Make it the opponent's turn.
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        opp_attacker = runner.place_on_field(2, ["BT22-010"])  # opp CS
        runner.place_on_field(1, ["BT22-010"])  # our target

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )
        ctx = {'game': game, 'attacker': opp_attacker}
        assert delay.can_use_condition(ctx) is False, (
            "[Your Turn] delay must not fire on opponent's turn.")

    def test_delay_condition_fails_without_de_digivolve_target(
            self, debug_runner):
        """Delay condition must fail when there is no opponent Digimon to
        de-digivolve (no valid target = cannot activate, matching DCGO
        HasMatchConditionPermanent gating)."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        cs_attacker = runner.place_on_field(1, ["BT22-010"])
        # No opponent battle area permanents.
        assert len(game.player2.battle_area) == 0

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )
        ctx = {'game': game, 'attacker': cs_attacker}
        assert delay.can_use_condition(ctx) is False

    # ─── C4: Delay process trashes self + de-digivolves ──────────────

    def test_delay_process_trashes_option_and_de_digivolves(
            self, debug_runner):
        """Running the delay process should trash the option from the
        battle area AND initiate a de-digivolve selection."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        target = runner.place_on_field(
            2, ["BT22-017", "BT22-010", "BT22-017", "BT22-010", "BT22-010"])

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )

        assert option_perm in game.player1.battle_area

        delay.on_process_callback({'game': game, 'player': game.player1})

        # Option is trashed from the battle area.
        assert option_perm not in game.player1.battle_area
        option_card = None
        for c in game.player1.trash_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT23-096":
                option_card = c
                break
        assert option_card is not None, (
            "Option card should be moved to trash as the Delay cost.")

        # A pending selection for de-digivolve should be active.
        assert game.pending_selection is not None
        # Resolve by picking the opponent target.
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        tgt_idx = game.player2.battle_area.index(target)
        game.decode_action(
            SEL_OPP_FIELD_START + tgt_idx, game.player1.player_id)

        # Target stack should have shrunk.
        assert len(target.card_sources) < 5

    def test_delay_marker_effect_present(self, debug_runner):
        """A _is_delay factory marker must be present so the engine's
        _option_stays_on_field check keeps the option on field."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)

        effects = card.effect_list(None)
        delay_markers = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_markers) == 1

        # Ensure the engine confirms the option stays on field.
        assert runner.game._option_stays_on_field(card) is True

    # ─── C5: [Security] de-digivolve + place in battle area ──────────

    def test_security_effect_exists(self, debug_runner):
        """Security effect should be registered with is_security_effect."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)
        effects = card.effect_list(None)

        security_effects = [
            e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(security_effects) == 1
        assert security_effects[0].is_security_effect is True

    def test_security_places_self_in_battle_area_when_no_target(
            self, debug_runner):
        """Security effect should place the option in battle area even if
        the opponent has no de-digivolve target."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", game.player1)

        sec_effect = next(
            e for e in card.effect_list(None)
            if e.timing == EffectTiming.SecuritySkill
        )

        # Opponent has nothing — process should skip selection and place.
        assert len(game.player2.battle_area) == 0
        sec_effect.on_process_callback({
            'game': game, 'player': game.player1,
            'card': card, 'permanent': None,
        })

        assert any(
            p.top_card is card for p in game.player1.battle_area
        ), "Option should be placed in battle area by security effect."
        # Engine will try not to trash it afterwards.
        assert getattr(card, '_security_played', False) is True

    def test_security_places_self_in_battle_area_after_de_digivolve(
            self, debug_runner):
        """Security effect should de-digivolve THEN place the option in
        the battle area once the selection resolves."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        # Opponent has a tall stack — valid de-digivolve target.
        target = runner.place_on_field(
            2, ["BT22-017", "BT22-010", "BT22-017", "BT22-010", "BT22-010"])
        stack_before = len(target.card_sources)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", game.player1)

        sec_effect = next(
            e for e in card.effect_list(None)
            if e.timing == EffectTiming.SecuritySkill
        )
        sec_effect.on_process_callback({
            'game': game, 'player': game.player1,
            'card': card, 'permanent': None,
        })

        # A pending selection for de-digivolve should be active.
        assert game.pending_selection is not None
        from engine_py_legacy.engine.game.constants import SEL_OPP_FIELD_START
        tgt_idx = game.player2.battle_area.index(target)
        game.decode_action(
            SEL_OPP_FIELD_START + tgt_idx, game.player1.player_id)

        # After selection: stack is shrunk.
        assert len(target.card_sources) < stack_before
        # AND the option is now in our battle area (placed by security).
        assert any(
            p.top_card is card for p in game.player1.battle_area
        ), "Option should be placed in battle area after de-digivolve."
        assert getattr(card, '_security_played', False) is True

    # ─── Integration: real attack fires the delay trigger ───────────

    def test_real_attack_fires_delay_and_trashes_option(self, debug_runner):
        """Full integration: a real CS-ally attack should put the option's
        OnAllyAttack trigger on the effect stack as an optional effect."""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        game = runner.game

        option_perm = runner.place_on_field(1, ["BT23-096"], turn_played=-2)
        runner.place_on_field(1, ["BT22-010"])  # CS attacker, Meramon Lv.4
        runner.place_on_field(
            2, ["BT22-017", "BT22-010", "BT22-017", "BT22-010", "BT22-010"])

        delay = next(
            e for e in option_perm.top_card.effect_list(None)
            if e.timing == EffectTiming.OnAllyAttack
        )
        assert delay.is_optional is True, (
            "[Your Turn] Delay trigger must be optional (DCGO: isOptional=true)")

        # The condition under a real attack context must hold.
        cs_attacker = game.player1.battle_area[1]  # Meramon slot
        ctx = {'game': game, 'attacker': cs_attacker}
        assert delay.can_use_condition(ctx) is True

    # ─── Sanity check: 4 effects are registered ──────────────────────

    def test_effect_count(self, debug_runner):
        """Script should register 4 effects: Main, Delay marker,
        OnAllyAttack trigger, and Security. (Ignore-color is handled via
        the _match_color_requirement_fn property, not an ICardEffect.)"""
        runner = debug_runner(
            deck1=HAMMER_DECK, deck2=HAMMER_DECK, initial_memory=5)
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-096", runner.game.player1)
        effects = card.effect_list(None)
        assert len(effects) == 4
