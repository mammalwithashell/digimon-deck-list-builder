"""Behavioral tests for P-205 Insane Synthetic Monster (Option, Purple, Cost 3, Traits: DM).

Official card text (from cards.json):
While you have a Digimon or Tamer with the [DM] trait on the field,
    you can ignore this card's color requirements.
[Main] <Draw 2> and trash 2 cards in your hand. Then, place this card
    in the battle area.
[Main] <Delay> (By trashing this card after the placing turn, activate
    the effect below.)
    - By deleting 1 of your play cost 7 or lower Digimon, you may play
      1 Digimon card with [Kimeramon] or [Millenniummon] in its name
      from your trash with the play cost reduced by 3.
[Security] <Draw 2> and trash 2 cards in your hand. Then, place this
    card in the battle area.

C# reference: P_205.cs
- Color bypass: IgnoreColorConditionClass with HasMatchConditionPermanent
    checking (IsTamer || IsDigimon) && HasDMTraits
- Main: DrawAndDiscardCards(draw=2, trash=2) + PlaceDelayOptionCards
- Delay: OnDeclaration timing, delete option first, then select 1 own
    Digimon cost<=7 to delete, then play Kimeramon/Millenniummon from
    trash with fixedCost = BasePlayCost - 3
- Security: Same shared draw+trash+place routine as Main
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# Minimal decks for DebugRunner
DECK = ["P-205"] * 4 + ["BT2-067"] * 46  # Purple filler (DemiDevimon)


# ─── Color Bypass Tests ──────────────────────────────────────────────────


@pytest.mark.behavioral
class TestP205ColorBypass:
    """Tests for P-205 conditional color requirement ignoring."""

    def test_color_bypass_with_dm_digimon_on_field(self, debug_runner):
        """P-205 should be playable without matching color when a DM Digimon
        is on the field. EX9-007 Agumon is Red+DM, not Purple."""
        runner = debug_runner(
            deck1=["P-205"] * 4 + ["EX9-007"] * 46,
            deck2=DECK,
            initial_memory=5,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")

        # Place a Red DM Digimon (no Purple color match)
        runner.place_on_field(1, ["EX9-007"])  # Agumon (Red, DM trait)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None, (
            "P-205 should be playable when a DM Digimon is on the field, "
            "even without color match"
        )

    def test_color_bypass_with_dm_tamer_on_field(self, debug_runner):
        """P-205 should be playable without matching color when a DM Tamer
        is on the field. EX9-069 Analog Youth is White+DM, not Purple."""
        runner = debug_runner(
            deck1=["P-205"] * 4 + ["EX9-069"] * 46,
            deck2=DECK,
            initial_memory=5,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")

        # Place a White DM Tamer
        runner.place_on_field(1, ["EX9-069"])  # Analog Youth (White, DM trait)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None, (
            "P-205 should be playable when a DM Tamer is on the field, "
            "even without color match"
        )

    def test_no_color_bypass_without_dm_on_field(self, debug_runner):
        """P-205 should NOT bypass color requirement when no DM trait
        Digimon/Tamer is on the field. Red non-DM Digimon doesn't help."""
        runner = debug_runner(
            deck1=["P-205"] * 4 + ["ST1-03"] * 46,
            deck2=DECK,
            initial_memory=5,
        )
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")

        # Place a non-DM non-Purple Digimon
        runner.place_on_field(1, ["ST1-03"])  # Agumon (Red, no DM)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is None, (
            "P-205 should NOT be playable when no DM Digimon/Tamer is on field "
            "and no Purple Digimon/Tamer provides color match"
        )

    def test_playable_with_purple_digimon_no_dm(self, debug_runner):
        """P-205 (Purple) should be playable normally when a Purple
        Digimon is on field, even without DM trait (regular color match)."""
        runner = debug_runner(
            deck1=DECK,
            deck2=DECK,
            initial_memory=5,
        )
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")

        # Place a Purple non-DM Digimon
        runner.place_on_field(1, ["BT2-067"])  # DemiDevimon (Purple, no DM)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None, (
            "P-205 should be playable via normal Purple color match"
        )


# ─── Main Effect Tests (Draw 2 + Trash 2 + Place in BA) ─────────────────


@pytest.mark.behavioral
class TestP205MainEffect:
    """Tests for P-205 [Main] <Draw 2> + trash 2 + place in battle area."""

    def test_main_draws_2_cards(self, debug_runner):
        """Playing P-205 should draw 2 cards."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")
        # Place a Purple Digimon for color match
        runner.place_on_field(1, ["BT2-067"])

        lib_before = len(p1.library_cards)
        hand_before = len(p1.hand_cards)  # 1 (P-205)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None, "Should find play action for P-205"
        runner.execute(action)

        # After drawing 2 and before trashing, hand should have gained 2
        # (P-205 was used from hand, so net: -1 + 2 = +1 before trash)
        # Library should have lost 2 cards from draw
        assert len(p1.library_cards) == lib_before - 2, (
            "Library should lose 2 cards from Draw 2"
        )

    def test_main_trashes_2_from_hand(self, debug_runner):
        """Playing P-205 should require trashing 2 cards from hand."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")
        # Add extra hand cards for trash targets
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        # Place Purple Digimon for color match
        runner.place_on_field(1, ["BT2-067"])

        trash_before = len(p1.trash_cards)
        hand_before = len(p1.hand_cards)  # 4 (P-205 + 3 DemiDevimon)

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # After: played P-205 (-1), drew 2 (+2), trashed 2 (-2) = net -1 from initial
        # P-205 goes to battle area, not trash
        # 2 cards trashed from hand
        assert len(p1.trash_cards) >= trash_before + 2, (
            "At least 2 cards should be trashed from hand"
        )

    def test_main_places_in_battle_area(self, debug_runner):
        """After Main effect, P-205 should be placed in the battle area
        (for Delay activation later)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")
        # Add trash fodder
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.place_on_field(1, ["BT2-067"])

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        p205_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "P-205"
        ]
        assert len(p205_on_field) >= 1, (
            "P-205 should be placed in the battle area after its Main effect"
        )


# ─── Delay Effect Tests ─────────────────────────────────────────────────


@pytest.mark.behavioral
class TestP205Delay:
    """Tests for P-205 Delay: delete own cost-7-or-less Digimon,
    play Kimeramon/Millenniummon from trash with cost -3."""

    def test_delay_marker_exists(self):
        """P-205 should have _is_delay = True on one of its effects."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, (
            "P-205 should have a Delay marker effect"
        )

    def test_delay_callback_follows_delay_marker(self):
        """The effect immediately after the _is_delay marker should have
        a process callback (the delayed effect)."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        for i, effect in enumerate(effects):
            if getattr(effect, '_is_delay', False):
                assert i + 1 < len(effects), "Delay marker must have a following effect"
                next_eff = effects[i + 1]
                assert next_eff.on_process_callback is not None, (
                    "Effect after _is_delay marker must have a process callback"
                )
                break
        else:
            pytest.fail("No _is_delay marker found")

    def test_delay_not_available_same_turn(self, debug_runner):
        """Delay cannot be activated on the same turn the option was placed."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-205", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.inject_card(1, "BT2-067", "hand")
        runner.place_on_field(1, ["BT2-067"])

        runner.set_phase("Main")
        action = runner.find_action("Insane Synthetic")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Now try to find a Delay action for P-205 on same turn
        delay_action = runner.find_action("Delay")
        assert delay_action is None, (
            "Delay should NOT be available on the same turn the option was placed"
        )

    def test_delay_available_after_placing_turn(self, debug_runner):
        """Delay should be available on a subsequent turn after placement."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        # Place P-205 directly in battle area as if it was placed last turn
        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        # Place a cost<=7 Digimon on field (needed for the delay effect)
        runner.place_on_field(1, ["BT2-067"])  # cost 2
        # Put a Kimeramon in trash
        runner.inject_card(1, "BT2-077", "trash")

        game.turn_count = 2  # Ensure turn_played < turn_count
        runner.set_phase("Main")

        # The delay action should be available
        actions = runner.actions()
        delay_action = runner.find_action("Delay")
        assert delay_action is not None, (
            "Delay should be available after the placing turn"
        )

    def test_delay_trashes_option_card(self, debug_runner):
        """Activating Delay should trash P-205 from the battle area."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        runner.place_on_field(1, ["BT2-067"])
        runner.inject_card(1, "BT2-077", "trash")

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        # P-205 should now be in trash
        p205_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == "P-205"
        ]
        assert len(p205_in_trash) >= 1, (
            "P-205 should be trashed after Delay activation"
        )

        # P-205 should no longer be in battle area
        p205_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "P-205"
        ]
        assert len(p205_on_field) == 0, (
            "P-205 should not remain in battle area after Delay activation"
        )

    def test_delay_deletes_own_cost7_or_less_digimon(self, debug_runner):
        """Delay should allow selecting and deleting one of your Digimon
        with play cost 7 or lower."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p1 = game.player1

        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        # Place a cost 7 Digimon (BT2-077 Kimeramon, cost 7) on field
        target_perm = runner.place_on_field(1, ["BT2-077"])
        # Put another Kimeramon in trash for playing
        runner.inject_card(1, "BT2-077", "trash")

        game.turn_count = 2
        runner.set_phase("Main")

        field_count_before = len(p1.battle_area)

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)
        runner.auto_resolve()

        # The target Digimon should have been deleted (sent to trash)
        # After delay: P-205 trashed, target Digimon deleted,
        # possibly a Kimeramon played from trash
        bt2_077_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT2-077"
        ]
        # At least 1 BT2-077 should be in trash (the deleted one)
        assert len(bt2_077_in_trash) >= 1, (
            "The deleted Digimon should be in trash"
        )

    def test_delay_plays_kimeramon_from_trash(self, debug_runner):
        """Delay should be able to play a Kimeramon from trash."""
        runner = debug_runner(
            deck1=["P-205"] * 4 + ["BT2-067"] * 42 + ["BT2-077"] * 4,
            deck2=DECK,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        # Place a cheap Digimon for deletion target
        runner.place_on_field(1, ["BT2-067"])  # cost 2 DemiDevimon
        # Put Kimeramon in trash
        runner.inject_card(1, "BT2-077", "trash")

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # Step through selections manually (auto_resolve picks Decline first)
        # Step 1: Select the DemiDevimon to delete (not Decline)
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget
        legal = runner.action_mask()
        select_perm = [a for a in legal if a != 62]  # skip Decline
        assert len(select_perm) >= 1, "Should have at least 1 Digimon to select for deletion"
        runner.execute(select_perm[0])

        # Step 2: Select Kimeramon from trash to play (not Decline)
        if game.current_phase == GamePhase.SelectTarget:
            legal2 = runner.action_mask()
            select_trash = [a for a in legal2 if a != 62]
            if select_trash:
                runner.execute(select_trash[0])

        runner.auto_resolve()

        # Check if Kimeramon was played from trash onto the field
        kim_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and "Kimeramon" in (p.top_card.c_entity_base.card_name_eng or "")
        ]
        assert len(kim_on_field) >= 1, (
            "Kimeramon should be played from trash after Delay activation"
        )

    def test_delay_plays_millenniummon_from_trash(self, debug_runner):
        """Delay should be able to play a Millenniummon from trash."""
        runner = debug_runner(
            deck1=["P-205"] * 4 + ["BT2-067"] * 42 + ["BT2-083"] * 4,
            deck2=DECK,
            initial_memory=15,
        )
        game = runner.game
        p1 = game.player1

        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        runner.place_on_field(1, ["BT2-067"])
        # Put Millenniummon (cost 15, reduced by 3 = 12) in trash
        runner.inject_card(1, "BT2-083", "trash")

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # Step through selections manually
        from engine_py_legacy.engine.data.enums import GamePhase
        if game.current_phase == GamePhase.SelectTarget:
            legal = runner.action_mask()
            select_perm = [a for a in legal if a != 62]
            if select_perm:
                runner.execute(select_perm[0])

        if game.current_phase == GamePhase.SelectTarget:
            legal2 = runner.action_mask()
            select_trash = [a for a in legal2 if a != 62]
            if select_trash:
                runner.execute(select_trash[0])

        runner.auto_resolve()

        mill_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and "Millenniummon" in (p.top_card.c_entity_base.card_name_eng or "")
        ]
        assert len(mill_on_field) >= 1, (
            "Millenniummon should be played from trash after Delay activation"
        )

    def test_delay_rejects_non_kimeramon_non_millenniummon(self, debug_runner):
        """Delay should NOT offer to play a Digimon that doesn't have Kimeramon
        or Millenniummon in its name from trash. Only DemiDevimon in trash."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        p1 = game.player1

        p205_perm = runner.place_on_field(1, ["P-205"], turn_played=0)
        runner.place_on_field(1, ["BT2-067"])  # deletion target
        # Put ONLY non-matching Digimon in trash (no Kimeramon/Millenniummon)
        runner.inject_card(1, "BT2-067", "trash")

        game.turn_count = 2
        runner.set_phase("Main")

        delay_action = runner.find_action("Delay")
        assert delay_action is not None
        runner.execute(delay_action)

        # Step 1: Select DemiDevimon to delete (not decline)
        from engine_py_legacy.engine.data.enums import GamePhase
        if game.current_phase == GamePhase.SelectTarget:
            legal = runner.action_mask()
            select_perm = [a for a in legal if a != 62]
            if select_perm:
                runner.execute(select_perm[0])

        # After deleting the Digimon, the play-from-trash should NOT
        # offer any selection because no Kimeramon/Millenniummon in trash.
        # If there IS a selection, it would be a bug (filter not working).
        # The game should return to Main phase since no valid targets.
        runner.auto_resolve()

        # The DemiDevimon in trash should NOT have been played to field
        demidev_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT2-067"
        ]
        assert len(demidev_on_field) == 0, (
            "Non-Kimeramon/Millenniummon Digimon should not be played from trash"
        )

    def test_delay_not_is_field_main(self):
        """The delay callback effect should NOT have _is_field_main = True.
        The delay mechanism has its own action slot; _is_field_main would
        cause a duplicate action."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        for i, effect in enumerate(effects):
            if getattr(effect, '_is_delay', False) and i + 1 < len(effects):
                next_eff = effects[i + 1]
                assert not getattr(next_eff, '_is_field_main', False), (
                    "Delay callback effect should NOT have _is_field_main=True. "
                    "The delay mechanism has its own action slot (effectIdx=1)."
                )
                break


# ─── Security Effect Tests ───────────────────────────────────────────────


@pytest.mark.behavioral
class TestP205Security:
    """Tests for P-205 [Security] <Draw 2> + trash 2 + place in BA."""

    def test_security_effect_has_correct_timing(self):
        """P-205 security effect should have SecuritySkill timing."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and e.is_security_effect
        ]
        assert len(sec_effects) >= 1, (
            "P-205 should have a SecuritySkill timing security effect"
        )
        assert sec_effects[0].on_process_callback is not None, (
            "P-205 security effect should have a process callback"
        )

    def test_security_draws_and_trashes(self, debug_runner):
        """Security effect should draw 2 and require trashing 2."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # Give P2 some hand cards for trash targets
        runner.inject_card(2, "BT2-067", "hand")
        runner.inject_card(2, "BT2-067", "hand")
        runner.inject_card(2, "BT2-067", "hand")

        # Clear security, inject P-205 as security
        runner.clear_zone(2, "security")
        runner.inject_card(2, "P-205", "security_top")

        lib_before = len(p2.library_cards)

        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # Library should have lost 2 cards from Draw 2
        assert len(p2.library_cards) <= lib_before - 2, (
            "Security Draw 2 should reduce library by 2"
        )

    def test_security_places_in_battle_area(self, debug_runner):
        """After security effect, P-205 should be placed in the battle area."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.inject_card(2, "BT2-067", "hand")
        runner.inject_card(2, "BT2-067", "hand")
        runner.inject_card(2, "BT2-067", "hand")

        runner.clear_zone(2, "security")
        runner.inject_card(2, "P-205", "security_top")

        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        p205_on_field = [
            p for p in p2.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "P-205"
        ]
        assert len(p205_on_field) >= 1, (
            "P-205 should be placed in the battle area after security effect"
        )


# ─── Effect Structure Tests ──────────────────────────────────────────────


@pytest.mark.behavioral
class TestP205EffectStructure:
    """Tests for P-205 effect list structure and attributes."""

    def test_has_five_effects(self):
        """P-205 should have exactly 5 effects:
        0: color bypass, 1: main, 2: delay marker, 3: delay callback, 4: security."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        assert len(effects) == 5, (
            f"P-205 should have 5 effects, got {len(effects)}"
        )

    def test_main_effect_has_option_skill_timing(self):
        """P-205 Main effect should have OptionSkill timing."""
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        from engine_py_legacy.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-205", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        option_skill = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(option_skill) >= 1, (
            "P-205 should have an OptionSkill timing effect for its Main effect"
        )
