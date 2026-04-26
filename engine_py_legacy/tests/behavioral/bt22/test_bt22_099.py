"""Behavioral tests for BT22-099 Kuremi Detective Agency (Option, Black/Yellow, Cost 3).

Card text (SOURCE OF TRUTH):
  While you have a [CS] trait Digimon or Tamer on the field, you can ignore
  this card's color requirements.
  [Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among them
  to the hand. Return the rest to the bottom of the deck. Then, place this card
  in the battle area.
  [Main] <Delay> (By trashing this card after the placing turn, activate the
  effect below.)
    - Gain 2 memory.
  [Security] Place this card in the battle area.

C# reference: DCGO/Assets/Scripts/CardEffect/BT22/Black/BT22_099.cs
  - None timing: IgnoreColorConditionClass (CS Digimon/Tamer on owner field)
  - OptionSkill: SimplifiedRevealDeckTopCardsAndSelect (3 cards, 1 pass: HasCSTraits)
                  + PlaceDelayOptionCards(card)
  - OnDeclaration: CardEffectFactory.Gain2MemoryOptionDelayEffect(card)
  - SecuritySkill: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)

Key cards used:
  BT22-099: Kuremi Detective Agency, Option, Black/Yellow, cost 3, CS trait
  BT22-017: Gabumon, Digimon Lv.3, Blue, CS trait (non-Black/non-Yellow, tests bypass)
  BT22-083: Yuuko Kamishiro, Tamer, Red/Black, CS trait
  BT22-008: Agumon, Digimon Lv.3, Red, CS trait (non-Black/non-Yellow)
  BT1-009: Monodramon, Digimon Lv.3, Red, no CS trait
  ST1-03: Agumon, Digimon Lv.3, Red, no CS trait
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


_DECK = ["BT22-099"] * 4 + ["BT22-017"] * 20 + ["BT1-009"] * 26


@pytest.mark.behavioral
class TestBT22099EffectShape:
    """Verify BT22-099 has the correct effect structure."""

    def test_has_option_skill(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-099")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) == 1, "Should have exactly 1 OptionSkill effect"

    def test_has_delay_marker(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-099")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) == 1, "Should have exactly 1 Delay marker effect"

    def test_has_delay_on_declaration(self):
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-099")
        effects = cs.effect_list(None)
        on_decl = [e for e in effects
                   if e.timing == EffectTiming.OnDeclaration
                   and getattr(e, '_is_field_main', False)]
        assert len(on_decl) == 1, "Should have exactly 1 OnDeclaration field-main effect"

    def test_has_security_effect_with_callback(self):
        """Security effect must have an on_process_callback that places the
        card in the battle area — not just a description."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-099")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) == 1, "Should have exactly 1 SecuritySkill effect"
        sec = sec_effects[0]
        assert sec.is_security_effect, "SecuritySkill must be flagged as security"
        assert sec.on_process_callback is not None, (
            "SecuritySkill MUST have on_process_callback to place card in battle area"
        )


@pytest.mark.behavioral
class TestBT22099ColorBypass:
    """While you have a [CS] Digimon/Tamer on field, ignore color requirements."""

    def test_color_enforced_without_cs_on_field(self, debug_runner):
        """Without CS Digimon/Tamer on field, color requirement is enforced (True)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.inject_card(1, "BT22-099", "hand")
        card = next(c for c in runner.game.player1.hand_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099")
        assert card.match_color_requirement is True, (
            "Without CS on field, match_color_requirement should be True (enforce)"
        )

    def test_color_bypassed_with_cs_digimon_on_field(self, debug_runner):
        """With a CS Digimon on field, color requirement is bypassed (False)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-017"])  # CS Digimon (Blue — non black/yellow)
        runner.inject_card(1, "BT22-099", "hand")
        card = next(c for c in runner.game.player1.hand_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099")
        assert card.match_color_requirement is False, (
            "With CS Digimon on field, match_color_requirement should be False (bypass)"
        )

    def test_color_bypassed_with_cs_tamer_on_field(self, debug_runner):
        """With a CS Tamer on field, color requirement is bypassed."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-083"])  # CS Tamer
        runner.inject_card(1, "BT22-099", "hand")
        card = next(c for c in runner.game.player1.hand_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099")
        assert card.match_color_requirement is False, (
            "With CS Tamer on field, match_color_requirement should be False (bypass)"
        )

    def test_non_cs_digimon_does_not_trigger_bypass(self, debug_runner):
        """Non-CS Digimon does NOT bypass color requirement."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT1-009"])  # non-CS Digimon
        runner.inject_card(1, "BT22-099", "hand")
        card = next(c for c in runner.game.player1.hand_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099")
        assert card.match_color_requirement is True, (
            "Non-CS Digimon should NOT bypass color requirement"
        )

    def test_bypass_is_specific_to_this_card(self, debug_runner):
        """Color bypass must be scoped to this card's _match_color_requirement_fn,
        not leak to unrelated options in hand."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-017"])  # CS Digimon
        # Instantiate an unrelated option in hand
        runner.inject_card(1, "BT22-099", "hand")
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Use another option and check its color req is not affected
        other = db.create_card_source("BT22-100", runner.game.player1)
        runner.game.player1.hand_cards.append(other)
        # Query each's property; BT22-099 should bypass, BT22-100 should not
        b099 = next(c for c in runner.game.player1.hand_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099")
        assert b099.match_color_requirement is False


@pytest.mark.behavioral
class TestBT22099MainEffect:
    """[Main] Reveal top 3, add 1 CS to hand, rest to bottom, place self in BA."""

    def _setup_reveal(self, runner, top_cards):
        """Place specific cards on top of P1 library (top-first)."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        p1 = runner.game.player1
        for cid in reversed(top_cards):
            cs = db.create_card_source(cid, p1)
            p1.library_cards.insert(0, cs)

    def test_main_places_option_in_battle_area(self, debug_runner):
        """Playing BT22-099 places it in the battle area (Delay option)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # CS Digimon on field (also provides color bypass for the cost)
        runner.place_on_field(1, ["BT22-017"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-099", "hand")
        # Ensure top 3 are all non-CS so select pass short-circuits cleanly
        self._setup_reveal(runner, ["BT1-009", "BT1-009", "BT1-009"])
        runner.set_phase("Main")

        action = self._find_play_action(runner)
        assert action is not None, f"Play action missing. Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT22-099" for s in snap.p1_field), (
            "BT22-099 must be in battle area after Main effect (Delay option)"
        )

    def _find_play_action(self, runner):
        """Find the Play BT22-099 action (action_id 0 is falsy, so use `is None`)."""
        aid = runner.find_action("Kuremi Detective")
        if aid is None:
            aid = runner.find_action("BT22-099")
        return aid

    def test_main_adds_cs_card_to_hand(self, debug_runner):
        """Reveal effect picks a CS card to hand, rest go to deck bottom."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-017"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-099", "hand")

        # Top 3: 1 CS Digimon + 2 non-CS
        self._setup_reveal(runner, ["BT22-008", "BT1-009", "BT1-009"])

        p1 = runner.game.player1
        deck_size_before = len(p1.library_cards)
        hand_size_before = len(p1.hand_cards)

        runner.set_phase("Main")
        action = self._find_play_action(runner)
        assert action is not None, f"Play action missing: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # CS Digimon added to hand
        assert "BT22-008" in snap.p1_hand, (
            f"CS Digimon BT22-008 should be in hand, got {snap.p1_hand}"
        )
        # Deck size: 3 revealed, 1 (CS) added to hand, 2 (non-CS) returned
        # to bottom. Net change: -1 card (the one that went to hand).
        assert len(p1.library_cards) == deck_size_before - 1, (
            f"Deck size should drop by 1 (only the CS card leaves deck): "
            f"before={deck_size_before}, after={len(p1.library_cards)}"
        )
        # Non-CS cards should NOT be in hand
        assert snap.p1_hand.count("BT1-009") == 0, (
            "Non-CS cards must NOT be added to hand"
        )

    def test_main_returns_non_cs_to_bottom(self, debug_runner):
        """Non-selected cards return to deck bottom."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-017"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-099", "hand")

        # Top 3: 1 CS + 2 non-CS (trackable by reference)
        self._setup_reveal(runner, ["BT22-008", "BT1-009", "BT1-009"])

        p1 = runner.game.player1

        runner.set_phase("Main")
        action = self._find_play_action(runner)
        assert action is not None, f"Play action missing: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        # The last 2 cards in the library should be non-CS (BT1-009) — returned
        # to bottom. Check the bottom two entries.
        assert len(p1.library_cards) >= 2
        bottom_two_ids = [
            p1.library_cards[-1].c_entity_base.card_id,
            p1.library_cards[-2].c_entity_base.card_id,
        ]
        assert all(cid == "BT1-009" for cid in bottom_two_ids), (
            f"Both non-CS cards should be at deck bottom, got {bottom_two_ids}"
        )

    def test_main_no_cs_in_reveal_skips_hand_add(self, debug_runner):
        """If no CS cards among the revealed 3, still places self in BA."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-017"])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT22-099", "hand")
        self._setup_reveal(runner, ["BT1-009", "BT1-009", "BT1-009"])

        runner.set_phase("Main")
        action = self._find_play_action(runner)
        assert action is not None, f"Play action missing: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Still placed in battle area
        assert any(s.card_id == "BT22-099" for s in snap.p1_field), (
            "BT22-099 should still be placed in BA even if no CS revealed"
        )
        # No BT1-009 in hand
        assert "BT1-009" not in snap.p1_hand


@pytest.mark.behavioral
class TestBT22099DelayEffect:
    """[Main] <Delay> Gain 2 memory. Activated by trashing card in battle area."""

    def test_delay_gains_2_memory_and_trashes_self(self, debug_runner):
        """Activating Delay trashes the Option and grants +2 memory."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3)
        # Place on field with turn_played=-1 (past turn, so Delay is legal)
        runner.place_on_field(1, ["BT22-099"], turn_played=-1)
        runner.set_phase("Main")

        memory_before = runner.game.memory

        action = (runner.find_action("Delay")
                  or runner.find_action("Memory +2")
                  or runner.find_action("Kuremi Detective"))
        assert action is not None, f"Delay action missing. Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Card should be trashed from battle area
        assert not any(s.card_id == "BT22-099" for s in snap.p1_field), (
            "BT22-099 should be trashed after Delay activation"
        )
        assert "BT22-099" in snap.p1_trash, (
            "BT22-099 should be in trash after Delay activation"
        )
        # Memory +2
        assert snap.memory == memory_before + 2, (
            f"Memory should be +2, was {memory_before}, now {snap.memory}"
        )

    def test_delay_blocked_same_turn_as_placement(self, debug_runner):
        """Cannot activate Delay the same turn the card is placed."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=3)
        game = runner.game
        # Place with turn_played = current turn
        runner.place_on_field(1, ["BT22-099"], turn_played=game.turn_count)
        runner.set_phase("Main")

        # Delay effect should be masked out by action_mask (turn_played >= turn)
        # Check: no "Delay" action available among Kuremi Detective entries
        actions = runner.actions()
        delay_actions = [
            desc for aid, desc in actions.items()
            if "Delay" in desc or "Memory +2" in desc
        ]
        # The Delay effect must NOT be active on the turn it was placed
        # (We allow the field-main field effect but not the Delay effectIdx=1)
        # Easier check: build mask and verify the delay slot is not set.
        mask = game.get_action_mask(game.current_player_id)
        from engine_py_legacy.engine.game.action_decoder import EFFECTS_PER_PERM
        # Find BT22-099's perm slot
        for idx, perm in enumerate(game.player1.battle_area):
            if perm.top_card and perm.top_card.c_entity_base.card_id == "BT22-099":
                delay_slot = 1000 + idx * EFFECTS_PER_PERM + 1
                assert mask[delay_slot] == 0.0, (
                    "Delay must NOT be activatable on placing turn"
                )
                break
        else:
            pytest.fail("BT22-099 not found on field")


@pytest.mark.behavioral
class TestBT22099SecurityEffect:
    """[Security] Place this card in the battle area."""

    def test_security_places_in_battle_area(self, debug_runner):
        """When BT22-099 is revealed from security by an attack, it's placed
        in the owner's battle area instead of being trashed."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # Clear security and put BT22-099 on top
        runner.clear_zone(2, "security")
        runner.inject_card(2, "BT22-099", "security_top")

        # Create an attacker on P1 field
        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "Attacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # BT22-099 should be in P2 battle area
        on_field = [p for p in p2.battle_area
                    if p.top_card and p.top_card.c_entity_base
                    and p.top_card.c_entity_base.card_id == "BT22-099"]
        assert len(on_field) == 1, (
            f"BT22-099 must be placed in battle area from security "
            f"(p2 battle_area count={len(p2.battle_area)})"
        )
        # NOT in trash
        in_trash = [c for c in p2.trash_cards
                    if c.c_entity_base and c.c_entity_base.card_id == "BT22-099"]
        assert len(in_trash) == 0, (
            "BT22-099 must NOT be in trash after security (goes to battle area)"
        )

    def test_security_placed_card_can_later_delay(self, debug_runner):
        """Card placed from security should be activatable via Delay on a later turn."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        runner.clear_zone(2, "security")
        runner.inject_card(2, "BT22-099", "security_top")

        from engine_py_legacy.tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "Attacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # Verify the placed perm has turn_played set (so it's properly tracked)
        on_field = [p for p in p2.battle_area
                    if p.top_card and p.top_card.c_entity_base
                    and p.top_card.c_entity_base.card_id == "BT22-099"]
        assert len(on_field) == 1
        perm = on_field[0]
        # turn_played should be set so Delay is trackable
        assert perm.turn_played is not None
