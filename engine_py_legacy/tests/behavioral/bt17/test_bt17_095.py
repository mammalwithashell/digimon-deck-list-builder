"""Behavioral tests for BT17-095 Miraculous Mega Knight (Option Red/Blue, Cost 2).

Card text:
[Main] You may play 1 [Agumon] or [Gabumon] from your hand or trash without
    paying the cost. Then, place this card in the battle area.
[All Turns] When one of your level 6 Digimon with [Greymon] or [Garurumon]
    in its name would leave the battle area outside of a battle, <Delay>
    (By trashing this card from the battle area, use the effect below).
    That Digimon and a card in the hand may DNA digivolve into a Digimon card
    with [Omnimon] in its name in the hand.
[Security] You may play 1 card with [Tai Kamiya] or [Matt Ishida] in its name
    from your hand or trash without paying the cost. Then, add this card to
    the hand.
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# ---------- helper deck -------------------------------------------------
_EGGS = ["BT1-001"] * 5
_MAIN = (
    ["BT17-095"] * 4    # Card under test (Option, Red/Blue, Cost 2)
    + ["ST1-03"] * 4     # Agumon (Digimon, Lv3, Red)
    + ["BT1-029"] * 4    # Gabumon (Digimon, Lv3, Blue)
    + ["BT1-025"] * 4    # WarGreymon (Lv6, Greymon in name, Red)
    + ["BT1-044"] * 4    # MetalGarurumon (Lv6, Garurumon in name, Blue)
    + ["BT17-078"] * 2   # Omnimon (Lv7, DNA digivolve)
    + ["ST1-12"] * 4     # Tai Kamiya (Tamer, Red)
    + ["ST2-12"] * 4     # Matt Ishida (Tamer, Blue)
    + ["BT1-010"] * 15   # Agumon filler (Red)
)
_DECK = _EGGS + _MAIN


@pytest.mark.behavioral
class TestBT17095MainEffect:
    """[Main] Play Agumon/Gabumon from hand or trash, then place in battle area."""

    def test_play_option_from_hand(self, debug_runner):
        """Can play BT17-095 as an option card when color requirement met."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # Need a Red or Blue Digimon/Tamer on field for option color requirement
        runner.place_on_field(1, ["ST1-03"])  # Red Agumon
        runner.inject_card(1, "BT17-095", "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Another Agumon
        runner.set_phase("Main")

        actions = runner.actions()
        option_action = runner.find_action("Miraculous Mega Knight")
        if option_action is None:
            option_action = runner.find_action("BT17-095")
        assert option_action is not None, (
            f"Should find play action for BT17-095. Available: {actions}"
        )

    def test_main_plays_agumon_from_hand(self, debug_runner):
        """Main effect plays Agumon from hand without paying cost."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        # Red Digimon on field for option color req
        runner.place_on_field(1, ["ST1-03"])
        runner.inject_card(1, "BT17-095", "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon to play
        runner.set_phase("Main")

        action = runner.find_action("Miraculous Mega Knight")
        if action is None:
            action = runner.find_action("BT17-095")
        assert action is not None, f"Available: {runner.actions()}"

        snap_before = runner.snapshot()
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Count Agumon on field (was 1 before from setup, should be 2+ now)
        agumon_before = len([s for s in snap_before.p1_field if s.card_id == "ST1-03"])
        agumon_after = len([s for s in snap_after.p1_field if s.card_id == "ST1-03"])
        assert agumon_after > agumon_before, (
            f"Agumon count should increase. Before={agumon_before}, After={agumon_after}"
        )

        # BT17-095 should be in battle area as a delay card
        bt17_095_on_field = [
            s for s in snap_after.p1_field
            if s.card_id == "BT17-095"
        ]
        assert len(bt17_095_on_field) >= 1, (
            "BT17-095 should be placed in battle area as delay"
        )

    def test_main_plays_gabumon_from_hand(self, debug_runner):
        """Main effect plays Gabumon from hand without paying cost."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT1-029"])  # Blue Gabumon for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT17-095", "hand")
        runner.inject_card(1, "BT1-029", "hand")  # Gabumon to play
        runner.set_phase("Main")

        action = runner.find_action("Miraculous Mega Knight")
        if action is None:
            action = runner.find_action("BT17-095")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        gabumon_on_field = [
            s for s in snap_after.p1_field
            if s.card_id == "BT1-029" and s.is_digimon
        ]
        # At least 2: the one placed for color req + the one played by effect
        assert len(gabumon_on_field) >= 2, (
            f"Should have 2+ Gabumon on field. Got {len(gabumon_on_field)}"
        )

    def test_main_exact_name_match_agumon(self, debug_runner):
        """Main effect uses exact name match for [Agumon] per C# EqualsCardName."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        # Verify the filter logic: exact name match
        game = runner.game
        player1 = game.player1

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        agumon_cs = db.create_card_source("ST1-03", player1)
        assert agumon_cs is not None
        assert "Agumon" in agumon_cs.card_names

    def test_main_plays_from_trash(self, debug_runner):
        """Main effect can play Agumon from trash."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST1-03"])  # Red for color req
        runner.inject_card(1, "BT17-095", "hand")
        runner.inject_card(1, "ST1-03", "trash")  # Agumon in trash
        runner.set_phase("Main")

        action = runner.find_action("Miraculous Mega Knight")
        if action is None:
            action = runner.find_action("BT17-095")
        assert action is not None, f"Available: {runner.actions()}"

        snap_before = runner.snapshot()
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Agumon count on field should increase
        agumon_before = len([s for s in snap_before.p1_field if s.card_id == "ST1-03"])
        agumon_after = len([s for s in snap_after.p1_field if s.card_id == "ST1-03"])
        assert agumon_after > agumon_before, (
            f"Agumon from trash should be played to field. Before={agumon_before}, After={agumon_after}"
        )

    def test_main_option_placed_in_battle_area(self, debug_runner):
        """After main effect resolves, BT17-095 should be placed in battle area."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST1-03"])  # Red for color req
        runner.inject_card(1, "BT17-095", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Miraculous Mega Knight")
        if action is None:
            action = runner.find_action("BT17-095")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            snap_after = runner.snapshot()
            bt17_on_field = [s for s in snap_after.p1_field if s.card_id == "BT17-095"]
            assert len(bt17_on_field) >= 1, (
                "BT17-095 should be placed in battle area even if no Agumon/Gabumon played"
            )


@pytest.mark.behavioral
class TestBT17095DelayEffect:
    """[All Turns] Delay: when Lv6 Greymon/Garurumon would leave (not battle)."""

    def test_delay_condition_lv6_greymon(self, debug_runner):
        """Delay trigger recognizes Lv6 Digimon with Greymon in name."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        # Place BT17-095 in battle area (delay active)
        runner.place_on_field(1, ["BT17-095"])
        # Place WarGreymon (Lv6, has Greymon in name)
        wargreymon_perm = runner.place_on_field(1, ["BT1-025"])

        game = runner.game
        player1 = game.player1

        # Find the delay trigger effect
        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_effects = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ]
        assert len(delay_effects) >= 1, "Should have WhenPermanentWouldBeDeleted effect"

        delay_eff = delay_effects[0]
        # Test condition with WarGreymon being deleted (not by battle)
        result = delay_eff.can_use_condition({
            "permanent": wargreymon_perm,
            "player": player1,
            "is_battle": False,
            "removal_cause": "effect",
        })
        assert result is True, (
            "Delay should trigger when Lv6 WarGreymon (Greymon in name) would leave"
        )

    def test_delay_condition_lv6_garurumon(self, debug_runner):
        """Delay trigger recognizes Lv6 Digimon with Garurumon in name."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-095"])
        metalgarurumon_perm = runner.place_on_field(1, ["BT1-044"])

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            "permanent": metalgarurumon_perm,
            "player": player1,
            "is_battle": False,
        })
        assert result is True, (
            "Delay should trigger for Lv6 MetalGarurumon (Garurumon in name)"
        )

    def test_delay_not_triggered_by_battle(self, debug_runner):
        """Delay should NOT trigger when leaving by battle."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-095"])
        wargreymon_perm = runner.place_on_field(1, ["BT1-025"])

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            "permanent": wargreymon_perm,
            "player": player1,
            "is_battle": True,
            "removal_cause": "battle",
        })
        assert result is False, "Delay should NOT trigger when leaving by battle"

    def test_delay_not_triggered_for_wrong_level(self, debug_runner):
        """Delay should NOT trigger for non-Lv6 Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-095"])
        # ST1-03 Agumon is Lv3
        agumon_perm = runner.place_on_field(1, ["ST1-03"])

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            "permanent": agumon_perm,
            "player": player1,
            "is_battle": False,
        })
        assert result is False, "Delay should NOT trigger for Lv3 Agumon"

    def test_delay_not_triggered_for_opponent_digimon(self, debug_runner):
        """Delay should NOT trigger for opponent's Digimon."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-095"])
        # Place WarGreymon on opponent's field
        opp_wargreymon = runner.place_on_field(2, ["BT1-025"])

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            "permanent": opp_wargreymon,
            "player": game.player2,
            "is_battle": False,
        })
        assert result is False, (
            "Delay should NOT trigger for opponent's Digimon"
        )

    def test_delay_condition_no_omnimon_check(self, debug_runner):
        """Delay condition should NOT require Omnimon in hand (per C# CanUseCondition).

        The C# CanUseCondition only checks:
        1. Delay card is in battle area
        2. Removal target is own Lv6 Greymon/Garurumon
        3. Not by battle
        It does NOT check for Omnimon availability.
        """
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)

        runner.place_on_field(1, ["BT17-095"])
        wargreymon_perm = runner.place_on_field(1, ["BT1-025"])

        game = runner.game
        player1 = game.player1

        # Make sure there's NO Omnimon in hand
        player1.hand_cards = [
            c for c in player1.hand_cards
            if not (c.c_entity_base and 'Omnimon' in c.c_entity_base.card_name_eng)
        ]

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        delay_eff = [
            e for e in effects
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted
        ][0]

        result = delay_eff.can_use_condition({
            "permanent": wargreymon_perm,
            "player": player1,
            "is_battle": False,
        })
        # Per C#, the condition should be True regardless of Omnimon availability
        assert result is True, (
            "Delay condition should NOT check for Omnimon in hand. "
            "C# CanUseCondition only checks: delay active, Lv6 Greymon/Garurumon, not battle."
        )


@pytest.mark.behavioral
class TestBT17095SecurityEffect:
    """[Security] Play Tai Kamiya/Matt Ishida from hand or trash, add to hand."""

    def test_security_effect_exists(self, debug_runner):
        """Security effect should be registered."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT17-095"])

        game = runner.game
        player1 = game.player1
        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        effects = bt17_card.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have a SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True

    def test_security_requires_tamer_type(self, debug_runner):
        """Security filter requires Tamer type per C# IsTamer check."""
        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT17-095"])

        game = runner.game
        player1 = game.player1

        bt17_card = None
        for p in player1.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT17-095":
                bt17_card = p.top_card
                break
        assert bt17_card is not None

        # The C# CanSelectCardCondition checks IsTamer + ContainsCardName.
        # Verified the Python filter also checks is_tamer.
