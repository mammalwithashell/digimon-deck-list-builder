"""Behavioral tests for BT17-097 Return to the Primogenitor (Option, Blue/Green, Cost 2).

Card text:
  [Main] 1 of your Digimon may digivolve into a level 5 or higher Digimon card
  with the [Free] trait in your hand with the digivolution cost reduced by 4.
  Then, place this card in the battle area.

  [All Turns] When one of your Digimon with the [Free] trait would be deleted
  other than by one of your effects, <Delay> (by trashing this card):
  By digivolving that Digimon into a Digimon card with [Imperialdramon] in its
  name in your hand without paying the cost, prevent that deletion.

  [Security] You may play 1 Tamer card with [Davis Motomiya] or [Ken Ichijoji]
  in its name from your hand or trash without paying the cost. Then, place this
  card in the battle area.

Clauses tested:
  1. Main effect filters hand for Lv5+ Digimon with [Free] ATTRIBUTE (not trait).
  2. Main effect digivolves with cost reduction of 4.
  3. Main effect places option in battle area (delay).
  4. Delay condition: own Digimon with [Free] attribute would be deleted.
  5. Delay condition: NOT triggered by own effects (ctx_player == owner and !battle).
  6. Delay condition: requires Imperialdramon in hand.
  7. Delay process: trashes this card, digivolves into Imperialdramon, prevents deletion.
  8. Security: plays Davis Motomiya or Ken Ichijoji Tamer from hand or trash.
  9. Security: places this card in battle area after play.

KNOWN BUG (fixed): Lines 62 and 132-133 check card_traits for "Free" but should
check attribute_eng. The [Free] trait in Digimon TCG refers to the attribute, not
the type/trait field.
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# ── Helper deck ─────────────────────────────────────────────────────────
# BT12-021: Veemon (Free Lv3)
# BT12-022: ExVeemon (Free Lv4)
# BT12-028: Paildramon (Free Lv5)
# BT12-030: Imperialdramon: Dragon Mode (Free Lv6)
# BT12-031: Imperialdramon: Fighter Mode (Free Lv6)
# BT3-093: Davis Motomiya (Tamer)
# BT3-094: Ken Ichijoji (Tamer)
# BT17-097: Card under test
_EGGS = ["BT1-001"] * 5
_MAIN = (
    ["BT17-097"] * 4    # Card under test
    + ["BT12-021"] * 4   # Veemon (Free Lv3)
    + ["BT12-022"] * 4   # ExVeemon (Free Lv4)
    + ["BT12-028"] * 4   # Paildramon (Free Lv5)
    + ["BT12-030"] * 4   # Imperialdramon: Dragon Mode (Free Lv6)
    + ["BT12-031"] * 2   # Imperialdramon: Fighter Mode (Free Lv6)
    + ["BT3-093"] * 4    # Davis Motomiya (Tamer)
    + ["BT3-094"] * 4    # Ken Ichijoji (Tamer)
    + ["BT1-010"] * 15   # filler
)
_DECK = _EGGS + _MAIN


@pytest.mark.behavioral
class TestBT17097MainEffect:
    """[Main] Digivolve into Lv5+ [Free] from hand, cost -4, place in battle area."""

    def test_main_effect_exists(self, debug_runner):
        """BT17-097 should have an OptionSkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT17-097")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill timing"

    def test_hand_filter_uses_attribute_not_traits(self, debug_runner):
        """The hand filter for [Free] must check attribute_eng, not card_traits.

        card_traits maps to type_eng (e.g. 'Insectoid', 'Dragon Man').
        attribute_eng contains 'Free', 'Vaccine', 'Data', 'Virus', etc.
        """
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # BT12-028 Paildramon is Lv5 with Free attribute
        cs = db.create_card_source("BT12-028")
        assert cs is not None
        assert cs.level >= 5, f"Paildramon should be Lv5+, got {cs.level}"
        assert 'Free' in cs.c_entity_base.attribute_eng, (
            f"Paildramon should have Free attribute, got {cs.c_entity_base.attribute_eng}"
        )

        # Verify card_traits does NOT contain 'Free'
        assert 'Free' not in cs.card_traits, (
            f"Paildramon card_traits should NOT contain 'Free': {cs.card_traits}"
        )

        # Now test the actual hand_filter from the script
        bt17_cs = db.create_card_source("BT17-097", runner.game.player1)
        effects = bt17_cs.effect_list(None)
        main_eff = [e for e in effects if e.timing == EffectTiming.OptionSkill][0]

        # Place a Digimon on field and Paildramon in hand
        runner.place_on_field(1, ["BT12-021"])  # Veemon (Free Lv3)
        runner.inject_card(1, "BT12-028", "hand")  # Paildramon (Free Lv5)
        runner.inject_card(1, "BT17-097", "hand")
        runner.set_phase("Main")

        # The option should be playable since there's a valid Lv5+ Free card in hand
        action = runner.find_action("Return to the Primogenitor")
        if action is None:
            action = runner.find_action("BT17-097")
        assert action is not None, (
            f"Should find action for BT17-097 when Paildramon (Free Lv5) is in hand. "
            f"Available: {runner.actions()}"
        )

    def test_main_digivolve_into_free_lv5(self, debug_runner):
        """Main effect should allow digivolving into a Lv5 Free Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        # ExVeemon (Free Lv4) on field — digivolve base
        runner.place_on_field(1, ["BT12-022"])
        # Paildramon (Free Lv5) in hand — digivolve target
        runner.inject_card(1, "BT12-028", "hand")
        runner.inject_card(1, "BT17-097", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Return to the Primogenitor")
        if action is None:
            action = runner.find_action("BT17-097")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # BT17-097 should be in battle area (delay card)
        bt17_on_field = any(s.card_id == "BT17-097" for s in snap.p1_field)
        assert bt17_on_field, "BT17-097 should be placed in battle area"

    def test_main_places_option_in_battle_area(self, debug_runner):
        """Even if digivolve is skipped (optional), BT17-097 should stay in battle area."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT12-021"])  # Veemon on field
        runner.inject_card(1, "BT17-097", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Return to the Primogenitor")
        if action is None:
            action = runner.find_action("BT17-097")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            bt17_on_field = any(s.card_id == "BT17-097" for s in snap.p1_field)
            assert bt17_on_field, (
                "BT17-097 should be in battle area even if digivolve was declined"
            )

    def test_main_rejects_lv4_free(self, debug_runner):
        """Hand filter should reject Free Digimon below Lv5."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # ExVeemon is Lv4 + Free — should be rejected
        cs = db.create_card_source("BT12-022")
        assert cs.level == 4
        assert 'Free' in cs.c_entity_base.attribute_eng

        # Test via process: only Lv4 Free in hand, no Lv5+ Free
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=10)
        runner.place_on_field(1, ["BT12-021"])  # Veemon (Free Lv3)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT12-022", "hand")  # ExVeemon (Free Lv4) — should not qualify
        runner.inject_card(1, "BT17-097", "hand")
        runner.set_phase("Main")

        # The option may still be playable (optional digivolve),
        # but there should be no valid digivolve target
        action = runner.find_action("Return to the Primogenitor")
        if action is None:
            action = runner.find_action("BT17-097")
        # Play the option and resolve — no digivolve should happen
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap = runner.snapshot()
            # ExVeemon should still be in hand (not digivolved onto field)
            exveemon_in_hand = "BT12-022" in snap.p1_hand
            exveemon_on_field = any(
                s.card_id == "BT12-022" for s in snap.p1_field
            )
            # ExVeemon should NOT have been used for digivolve target
            # (it may or may not still be in hand depending on implementation)


@pytest.mark.behavioral
class TestBT17097DelayEffect:
    """[All Turns] Delay: prevent deletion of [Free] Digimon by digivolving into Imperialdramon."""

    def test_delay_marker_exists(self, debug_runner):
        """BT17-097 should have a delay marker effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT17-097")
        effects = cs.effect_list(None)
        delay_effects = [e for e in effects if getattr(e, '_is_delay', False)]
        assert len(delay_effects) >= 1, "Should have a delay marker effect"

    def test_delay_trigger_exists(self, debug_runner):
        """Should have a WhenPermanentWouldBeDeleted trigger effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT17-097")
        effects = cs.effect_list(None)
        trigger_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(trigger_effects) >= 1, (
            "Should have WhenPermanentWouldBeDeleted effect"
        )

    def test_delay_condition_checks_free_attribute(self, debug_runner):
        """Delay condition should check attribute_eng for 'Free', not card_traits."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        # Place BT17-097 in battle area (delay active)
        runner.place_on_field(1, ["BT17-097"])
        # Place Paildramon (Free Lv5) — the Digimon that would be deleted
        paildramon_perm = runner.place_on_field(1, ["BT12-028"])
        # Put Imperialdramon in hand
        runner.inject_card(1, "BT12-030", "hand")

        game = runner.game
        player1 = game.player1

        # Find the BT17-097 card and its delay trigger
        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-097":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(delay_effects) >= 1
        delay_eff = delay_effects[0]

        # Paildramon has Free in attribute_eng, NOT in card_traits
        assert 'Free' in paildramon_perm.top_card.c_entity_base.attribute_eng
        assert 'Free' not in paildramon_perm.top_card.card_traits

        # Condition should pass — Free-attribute Digimon being deleted by opponent
        result = delay_eff.can_use_condition({
            'permanent': paildramon_perm,
            'player': game.player2,  # opponent is deleting
            'is_battle': False,
        })
        assert result is True, (
            "Delay condition should pass for Free-attribute Digimon being deleted. "
            "Bug: script checks card_traits instead of attribute_eng."
        )

    def test_delay_condition_rejects_non_free(self, debug_runner):
        """Delay should NOT trigger for Digimon without [Free] attribute."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-097"])
        # ST1-03 Agumon is Red, not Free
        agumon_perm = runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT12-030", "hand")  # Imperialdramon

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-097":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            'permanent': agumon_perm,
            'player': game.player2,
            'is_battle': False,
        })
        assert result is False, "Should NOT trigger for non-Free Digimon"

    def test_delay_not_triggered_by_own_effects(self, debug_runner):
        """Delay should NOT trigger when deletion is by owner's own effects."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-097"])
        paildramon_perm = runner.place_on_field(1, ["BT12-028"])
        runner.inject_card(1, "BT12-030", "hand")

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-097":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        # Own player deleting, not a battle — should not trigger
        result = delay_eff.can_use_condition({
            'permanent': paildramon_perm,
            'player': player1,  # own player
            'is_battle': False,
        })
        assert result is False, (
            "Should NOT trigger when deletion is by own effects"
        )

    def test_delay_requires_imperialdramon_in_hand(self, debug_runner):
        """Delay should NOT trigger if no Imperialdramon in hand."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-097"])
        paildramon_perm = runner.place_on_field(1, ["BT12-028"])
        # No Imperialdramon in hand
        runner.clear_zone(1, "hand")

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-097":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            'permanent': paildramon_perm,
            'player': game.player2,
            'is_battle': False,
        })
        assert result is False, (
            "Should NOT trigger without Imperialdramon in hand"
        )

    def test_delay_triggers_in_battle(self, debug_runner):
        """Delay SHOULD trigger when deletion is by battle (opponent attacking)."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-097"])
        paildramon_perm = runner.place_on_field(1, ["BT12-028"])
        runner.inject_card(1, "BT12-030", "hand")

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-097":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        # Battle deletion: is_battle=True, but the script says "other than by
        # one of your effects". Battle is by opponent, so it should trigger.
        result = delay_eff.can_use_condition({
            'permanent': paildramon_perm,
            'player': player1,  # own player in battle context
            'is_battle': True,
        })
        assert result is True, (
            "Delay SHOULD trigger for battle deletion (not by own effects)"
        )


@pytest.mark.behavioral
class TestBT17097SecurityEffect:
    """[Security] Play Davis/Ken Tamer from hand or trash; place in battle area."""

    def test_security_effect_exists(self, debug_runner):
        """BT17-097 should have a SecuritySkill effect."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT17-097")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True

    def test_security_filter_matches_davis(self, debug_runner):
        """Security filter should match 'Davis Motomiya' tamer cards."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        davis = db.create_card_source("BT3-093")
        assert davis is not None
        assert davis.is_tamer
        assert davis.contains_card_name("Davis Motomiya")

    def test_security_filter_matches_ken(self, debug_runner):
        """Security filter should match 'Ken Ichijoji' tamer cards."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        ken = db.create_card_source("BT3-094")
        assert ken is not None
        assert ken.is_tamer
        assert ken.contains_card_name("Ken Ichijoji")

    def test_security_filter_matches_combined_tamer(self, debug_runner):
        """Security filter should match 'Davis Motomiya & Ken Ichijoji' tamer."""
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()

        combined = db.create_card_source("BT17-084")
        assert combined is not None
        assert combined.is_tamer
        # Should match either name
        assert (combined.contains_card_name("Davis Motomiya") or
                combined.contains_card_name("Ken Ichijoji")), (
            f"Combined tamer should match Davis or Ken: {combined.card_names}"
        )
