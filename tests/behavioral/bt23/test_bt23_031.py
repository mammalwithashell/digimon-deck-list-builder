"""Behavioral tests for BT23-031 Angewomon (Lv.5 Yellow Digimon).

Card text (from metadata):
  Effect:
    When this card would be played from the hand, if you have [LadyDevimon]
    or [Mirei Mikagura], reduce the play cost by 3.
    [On Play] [When Digivolving] Add your top security card to the hand.
    Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>
    (Place the top card of your deck on top of your security stack.).

  Inherited:
    <Alliance> (When this Digimon attacks, by suspending 1 of your other
    Digimon, add the suspended Digimon's DP to this Digimon and it gains
    <Security A. +1> for the attack.)

Alt-digi (C# BT23_031.cs, PermanentCondition):
  Lv.4 w/ [CS] trait for cost 3 (no color restriction, HasLevel + IsLevel4
  + HasCSTraits only).

Standard evo cost (metadata): Yellow Lv.4 for cost 3.

Key cards for tests:
- BT23-031: Angewomon (the card under test)
- BT3-088:  LadyDevimon (Lv.5 Purple Digimon — qualifies for cost reduction)
- BT11-094: Mirei Mikagura (Tamer — qualifies for cost reduction)
- BT23-018: Garurumon (Lv.4 Blue CS — valid alt-digi target, non-Yellow)
- BT23-027: Angemon (Lv.4 Yellow+Blue CS — valid alt-digi target)
- BT1-055:  Angemon (Lv.4 Yellow, no CS — NOT valid for alt-digi,
            but valid for standard evo)
- BT8-061:  Thundermon (Lv.4 Purple non-CS — not valid for either)
- BT1-010:  Agumon (generic filler)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.validation.digivolve_validator import can_digivolve


# ── Helper deck ────────────────────────────────────────────────────────

def _angewomon_deck():
    """Return a 50-card deck with BT23-031 and support cards."""
    core = [
        "BT23-031",  # Angewomon (card under test)
        "BT3-088",   # LadyDevimon (for cost reduction)
        "BT11-094",  # Mirei Mikagura (for cost reduction)
        "BT23-027",  # Angemon Lv.4 CS (for standard / alt-digi)
        "BT23-018",  # Garurumon Lv.4 Blue CS (for alt-digi)
        "BT1-055",   # Angemon Lv.4 Yellow non-CS (for standard evo only)
    ]
    filler = ["BT1-010"] * (50 - len(core))  # Agumon filler
    return core + filler


@pytest.mark.behavioral
class TestBT23_031Angewomon:
    """Tests for BT23-031 Angewomon."""

    # ==================================================================
    # Alt-digi: Lv.4 with CS trait for cost 3 (any color)
    # ==================================================================

    def test_alt_digi_attributes_present(self, debug_runner):
        """BT23-031 should have alt-digi with Lv.4 + CS trait for cost 3."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi_effects = [
            e for e in all_effects if hasattr(e, '_alt_digi_cost')
        ]
        assert len(alt_digi_effects) >= 1, "Should have alt-digi effect"
        eff = alt_digi_effects[0]
        assert eff._alt_digi_cost == 3, \
            f"Alt-digi cost must be 3, got {eff._alt_digi_cost}"
        assert eff._alt_digi_level == 4, \
            f"Alt-digi level must be 4, got {eff._alt_digi_level}"
        assert getattr(eff, '_alt_digi_trait', None) == "CS", (
            f"Alt-digi must require CS trait. Got "
            f"_alt_digi_trait={getattr(eff, '_alt_digi_trait', None)}"
        )
        # C# has NO color restriction — alt-digi applies to any color.
        assert getattr(eff, '_alt_digi_color', None) is None, (
            f"Alt-digi must NOT restrict color (C# PermanentCondition only "
            f"checks level + CS trait). Got "
            f"_alt_digi_color={getattr(eff, '_alt_digi_color', None)}"
        )

    def test_alt_digi_works_on_non_yellow_cs_lv4(self, debug_runner):
        """Alt-digi should allow digivolving onto a Lv.4 Blue CS (non-Yellow)."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # BT23-018 Garurumon: Lv.4 Blue with CS trait
        base_perm = runner.place_on_field(1, ["BT23-018"])

        db = CardDatabase()
        angewomon = db.create_card_source("BT23-031", runner.game.player1)
        assert can_digivolve(angewomon, base_perm), (
            "BT23-031 should be able to alt-digivolve onto Lv.4 Blue CS "
            "(no color restriction in C# condition)"
        )

    def test_alt_digi_works_on_yellow_cs_lv4(self, debug_runner):
        """Standard evo OR alt-digi should allow digivolving onto Yellow Lv.4 CS."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # BT23-027 Angemon: Lv.4 Yellow+Blue CS trait — matches both standard
        # evo (Yellow Lv.4) and alt-digi (Lv.4 CS).
        base_perm = runner.place_on_field(1, ["BT23-027"])

        db = CardDatabase()
        angewomon = db.create_card_source("BT23-031", runner.game.player1)
        assert can_digivolve(angewomon, base_perm), (
            "BT23-031 should digivolve onto Lv.4 Yellow CS"
        )

    def test_standard_evo_works_on_yellow_lv4_non_cs(self, debug_runner):
        """Standard evo_cost (Yellow Lv.4) still works even without CS trait."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # BT1-055 Angemon: Lv.4 Yellow, no CS trait — matches standard evo only
        base_perm = runner.place_on_field(1, ["BT1-055"])

        db = CardDatabase()
        angewomon = db.create_card_source("BT23-031", runner.game.player1)
        assert can_digivolve(angewomon, base_perm), (
            "BT23-031 should still digivolve onto Lv.4 Yellow non-CS via "
            "standard evo_cost"
        )

    def test_alt_digi_rejects_non_cs_non_yellow(self, debug_runner):
        """Cannot digivolve onto a Lv.4 non-Yellow non-CS Digimon."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # BT8-061 Thundermon: Lv.4 Purple, no CS trait — fails both standard
        # (not Yellow) and alt-digi (not CS).
        base_perm = runner.place_on_field(1, ["BT8-061"])

        db = CardDatabase()
        angewomon = db.create_card_source("BT23-031", runner.game.player1)
        assert not can_digivolve(angewomon, base_perm), (
            "BT23-031 should NOT digivolve onto Lv.4 Purple non-CS"
        )

    # ==================================================================
    # BeforePayCost: Cost reduction (-3) when LadyDevimon / Mirei Mikagura
    # ==================================================================

    def _get_beforepaycost_effect(self, card_source):
        effects = card_source.effect_list(None)
        bpc = [e for e in effects if e.timing == EffectTiming.BeforePayCost]
        assert len(bpc) >= 1, "Should have a BeforePayCost effect"
        return bpc[0]

    def test_cost_reduction_effect_exists(self, debug_runner):
        """Angewomon should have a BeforePayCost effect with cost_reduction=3."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]
        bpc = self._get_beforepaycost_effect(angewomon_card)
        assert bpc.cost_reduction == 3, (
            f"cost_reduction must be 3, got {bpc.cost_reduction}"
        )

    def test_cost_reduction_condition_with_ladydevimon(self, debug_runner):
        """Condition should pass with LadyDevimon (Digimon, exact name) on field."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        runner.place_on_field(1, ["BT3-088"])  # LadyDevimon
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        bpc = self._get_beforepaycost_effect(angewomon_card)
        result = bpc.can_use_condition({
            'card_source': angewomon_card,
            'game': runner.game,
            'player': runner.game.player1,
        })
        assert result, "Condition should pass with LadyDevimon on field"

    def test_cost_reduction_condition_with_mirei_tamer(self, debug_runner):
        """Condition should pass with Mirei Mikagura Tamer on field."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        runner.place_on_field(1, ["BT11-094"])  # Mirei Mikagura
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        bpc = self._get_beforepaycost_effect(angewomon_card)
        result = bpc.can_use_condition({
            'card_source': angewomon_card,
            'game': runner.game,
            'player': runner.game.player1,
        })
        assert result, "Condition should pass with Mirei Mikagura Tamer on field"

    def test_cost_reduction_condition_fails_without_qualifying(self, debug_runner):
        """Condition should fail with neither LadyDevimon nor Mirei on field."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        runner.place_on_field(1, ["BT1-010"])  # Agumon (unrelated)
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        bpc = self._get_beforepaycost_effect(angewomon_card)
        result = bpc.can_use_condition({
            'card_source': angewomon_card,
            'game': runner.game,
            'player': runner.game.player1,
        })
        assert not result, (
            "Condition should fail with only unrelated Digimon on field"
        )

    def test_cost_reduction_leak_guard(self, debug_runner):
        """BeforePayCost must not reduce cost for OTHER cards being played."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # Put Angewomon ON FIELD (not being played)
        perm = runner.place_on_field(1, ["BT23-031"])
        runner.place_on_field(1, ["BT3-088"])  # LadyDevimon on field

        top = perm.top_card
        bpc = self._get_beforepaycost_effect(top)

        # Another card (not Angewomon) being played
        other_card = runner.game.player1.hand_cards[0] if runner.game.player1.hand_cards else None
        assert other_card is not None, "Need another card in hand"

        result = bpc.can_use_condition({
            'card_source': other_card,
            'game': runner.game,
            'player': runner.game.player1,
        })
        assert not result, (
            "Leak guard failed: BeforePayCost must not fire for another card. "
            "Condition must check `context.get('card_source') is not card`."
        )

    def test_calculate_play_cost_reduced_by_3_with_ladydevimon(self, debug_runner):
        """calculate_play_cost should return 7-3=4 when LadyDevimon is on field."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        runner.place_on_field(1, ["BT3-088"])  # LadyDevimon
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        cost = runner.game.calculate_play_cost(
            runner.game.player1, angewomon_card, source_zone="hand")
        assert cost == 4, (
            f"Expected play cost 7-3=4 with LadyDevimon on field, got {cost}"
        )

    def test_calculate_play_cost_unreduced_without_qualifying(self, debug_runner):
        """calculate_play_cost should return base 7 without LadyDevimon/Mirei."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        runner.place_on_field(1, ["BT1-010"])  # Unrelated Digimon
        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        cost = runner.game.calculate_play_cost(
            runner.game.player1, angewomon_card, source_zone="hand")
        assert cost == 7, (
            f"Expected base play cost 7 without LadyDevimon/Mirei, got {cost}"
        )

    def test_cost_reduction_requires_exact_ladydevimon_name(self, debug_runner):
        """
        C# uses EqualsCardName — exact match. Test that 'LadyDevimon
        (X Antibody)' should NOT trigger the cost reduction since it's a
        different card name. Only regular 'LadyDevimon' qualifies.
        """
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # Place X Antibody variant on field
        db = CardDatabase()
        xa = None
        # Try known X Antibody card id first
        for cid in ["EX4-038", "EX4-039"]:
            try:
                cs = db.create_card_source(cid, runner.game.player1)
                if cs and cs.card_names and cs.card_names[0] == "LadyDevimon (X Antibody)":
                    xa = cs
                    break
            except Exception:
                pass
        if xa is None:
            pytest.skip("No LadyDevimon (X Antibody) card available in registry")

        # Inject directly onto field via hand -> play is complex; use
        # place_on_field helper equivalent: create a permanent manually.
        from digimon_gym.engine.core.permanent import Permanent
        perm = Permanent([xa])
        perm._owner_game = runner.game
        runner.game.player1.battle_area.append(perm)

        runner.inject_card(1, "BT23-031", "hand")
        angewomon_card = runner.game.player1.hand_cards[-1]

        bpc = self._get_beforepaycost_effect(angewomon_card)
        result = bpc.can_use_condition({
            'card_source': angewomon_card,
            'game': runner.game,
            'player': runner.game.player1,
        })
        assert not result, (
            "Condition must use exact-name match: 'LadyDevimon (X Antibody)' "
            "should NOT qualify (C# EqualsCardName)."
        )

    # ==================================================================
    # [On Play] — Add top security to hand, conditional Recovery +1
    # ==================================================================

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect with is_on_play=True."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play = [
            e for e in all_effects
            if getattr(e, 'is_on_play', False)
            and not getattr(e, 'is_when_digivolving', False)
        ]
        assert len(on_play) == 1, (
            f"Expected 1 On Play effect, got {len(on_play)}"
        )

    def test_on_play_adds_top_security_to_hand(self, debug_runner):
        """[On Play] should move the top security card to hand."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        runner.inject_card(1, "BT23-031", "hand")
        player = runner.game.player1
        initial_hand_size = len(player.hand_cards)
        initial_sec_count = len(player.security_cards)
        assert initial_sec_count > 0, "Need security cards"
        top_sec_before = player.security_cards[0]

        runner.set_phase("Main")
        play = runner.find_action("Play Angewomon")
        assert play is not None, (
            f"Should be able to play Angewomon. Actions: {runner.actions()}"
        )
        runner.execute(play)
        runner.auto_resolve()

        # Top security card should now be in hand
        assert top_sec_before in player.hand_cards, (
            "Top security card should have moved to hand after On Play"
        )

    def test_on_play_recovery_fires_when_security_at_3(self, debug_runner):
        """If security drops to 3 or fewer AFTER adding, Recovery +1 fires."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        player = runner.game.player1
        # Reduce security to 4 so after adding 1 to hand, count is 3
        runner.clear_zone(1, "security")
        for _ in range(4):
            if player.library_cards:
                player.security_cards.append(player.library_cards.pop(0))
        assert len(player.security_cards) == 4
        initial_deck = len(player.library_cards)

        runner.inject_card(1, "BT23-031", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Angewomon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # After: 4 sec - 1 added to hand = 3, then Recovery +1 → 4
        assert len(player.security_cards) == 4, (
            f"Expected 4 security after Recovery (3 + 1), got "
            f"{len(player.security_cards)}"
        )
        # Deck should have lost 1 card to recovery
        assert len(player.library_cards) == initial_deck - 1, (
            f"Expected deck to lose 1 card via Recovery, "
            f"got {initial_deck - len(player.library_cards)} cards removed"
        )

    def test_on_play_no_recovery_when_security_above_3(self, debug_runner):
        """If security is still 4+ after adding, Recovery should NOT fire."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        player = runner.game.player1
        # Fill security to 5 so after adding 1 to hand, count is 4 (> 3)
        runner.clear_zone(1, "security")
        for _ in range(5):
            if player.library_cards:
                player.security_cards.append(player.library_cards.pop(0))
        assert len(player.security_cards) == 5
        initial_deck = len(player.library_cards)

        runner.inject_card(1, "BT23-031", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Angewomon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # After: 5 - 1 added to hand = 4, no recovery (4 > 3)
        assert len(player.security_cards) == 4, (
            f"Expected 4 security (no recovery), got "
            f"{len(player.security_cards)}"
        )
        # Deck unchanged by recovery
        assert len(player.library_cards) == initial_deck, (
            f"Deck should be unchanged when security > 3 after add, "
            f"deck lost {initial_deck - len(player.library_cards)} cards"
        )

    def test_on_play_no_security_handles_empty_stack(self, debug_runner):
        """If security is empty, [On Play] should not crash and should still
        attempt Recovery (count=0 <= 3)."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        player = runner.game.player1
        runner.clear_zone(1, "security")
        initial_deck = len(player.library_cards)

        runner.inject_card(1, "BT23-031", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Angewomon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # Recovery should still trigger (0 <= 3), putting 1 card into security
        assert len(player.security_cards) == 1, (
            f"Expected Recovery to add 1 security, got "
            f"{len(player.security_cards)}"
        )
        assert len(player.library_cards) == initial_deck - 1

    # ==================================================================
    # [When Digivolving] — Same effect as On Play
    # ==================================================================

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect separate from On Play."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        top = perm.top_card
        all_effects = top.effect_list(None)
        wd = [
            e for e in all_effects
            if getattr(e, 'is_when_digivolving', False)
            and not getattr(e, 'is_on_play', False)
        ]
        assert len(wd) == 1, (
            f"Expected 1 When Digivolving effect, got {len(wd)}"
        )

    def test_when_digivolving_fires_on_digivolve(self, debug_runner):
        """Digivolving into Angewomon should add top security and trigger recovery."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        player = runner.game.player1
        # Reduce security to 4 so after adding 1 to hand, count is 3, Recovery
        runner.clear_zone(1, "security")
        for _ in range(4):
            if player.library_cards:
                player.security_cards.append(player.library_cards.pop(0))
        top_sec_before = player.security_cards[0]

        # Place a Lv.4 Yellow base to digivolve from
        runner.place_on_field(1, ["BT1-055"])  # Angemon Lv.4 Yellow
        runner.inject_card(1, "BT23-031", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        assert digi is not None, (
            f"Should be able to digivolve. Actions: {runner.actions()}"
        )
        runner.execute(digi)
        runner.auto_resolve()

        # Top security should be in hand
        assert top_sec_before in player.hand_cards, (
            "Top security should be in hand after WD"
        )
        # Security count = 4 - 1 (to hand) + 1 (recovery) = 4
        # (This confirms both: add-to-hand AND the Recovery fired.)
        assert len(player.security_cards) == 4, (
            f"Expected 4 security (4 - 1 to hand + 1 recovery), got "
            f"{len(player.security_cards)}"
        )

    def test_on_play_and_wd_both_fire_on_play_only_once(self, debug_runner):
        """Playing Angewomon (not digivolving) should fire ONLY the On Play
        effect, not the When Digivolving effect."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=10,
        )
        player = runner.game.player1
        runner.clear_zone(1, "security")
        for _ in range(5):
            if player.library_cards:
                player.security_cards.append(player.library_cards.pop(0))
        assert len(player.security_cards) == 5
        initial_hand = len(player.hand_cards)

        runner.inject_card(1, "BT23-031", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Angewomon")
        assert play is not None
        runner.execute(play)
        runner.auto_resolve()

        # Only 1 security card moved to hand. If WD fired too, 2 would move.
        # Hand size change: +1 Angewomon injected, -1 Angewomon played,
        # +1 from security via On Play = initial_hand + 1
        # (Note: inject happened before our initial_hand snapshot so we need
        #  a different check.)
        # Easier check: security dropped by exactly 1 (5 -> 4), no recovery
        assert len(player.security_cards) == 4, (
            f"Only On Play should fire (1 security -> hand). Got "
            f"{len(player.security_cards)} security; expected 4."
        )

    # ==================================================================
    # Inherited: <Alliance>
    # ==================================================================

    def test_alliance_is_inherited_not_top(self, debug_runner):
        """Alliance is in inherited text only — must be is_inherited_effect."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        top = perm.top_card
        all_effects = top.effect_list(None)
        alliance = [e for e in all_effects if getattr(e, '_is_alliance', False)]
        assert len(alliance) == 1, (
            f"Expected exactly 1 Alliance factory effect, got {len(alliance)}"
        )
        assert alliance[0].is_inherited_effect, (
            "Alliance must be inherited (from inherited_text slot)"
        )

    def test_alliance_keyword_not_on_top_only_permanent(self, debug_runner):
        """Angewomon on top should NOT have Alliance keyword (it's inherited-
        only, applied to digimon stacked on top of her)."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        # When Angewomon is the TOP card, the inherited alliance effect is
        # only picked up from sources UNDER top — so Angewomon herself should
        # NOT have Alliance.
        assert not perm.has_keyword('_is_alliance'), (
            "Angewomon on top should NOT have Alliance (inherited effects "
            "only apply from sources under the top card)"
        )

    def test_alliance_keyword_granted_to_digimon_above(self, debug_runner):
        """A Digimon digivolved on top of Angewomon should gain Alliance."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        # Stack BT23-031 (under) + BT23-031 (on top as a placeholder) —
        # actually use any Lv.6 on top. Use another card (BT23-057 Gankoomon
        # Lv.6 has no alliance in top slot) — but easier: place a stack
        # manually with BT23-031 below and a simple Digimon on top.
        # BT23-057 Gankoomon (Lv.6) has alliance, which we can't use to
        # test inheritance cleanly. Use BT1-010 Agumon as top.
        perm = runner.place_on_field(1, ["BT23-031", "BT1-010"])
        # Now Agumon is on top; BT23-031 is a digivolution source.
        # The inherited alliance on BT23-031 should grant Agumon Alliance.
        assert perm.has_keyword('_is_alliance'), (
            "Digimon stacked on top of BT23-031 should gain Alliance "
            "from inherited effect"
        )

    # ==================================================================
    # No stray effects
    # ==================================================================

    def test_effect_count_is_minimal(self, debug_runner):
        """Angewomon should have exactly 4 effects (alt-digi, BPC, OP, WD, alliance)."""
        runner = debug_runner(
            deck1=_angewomon_deck(), deck2=_angewomon_deck(),
            initial_memory=5,
        )
        perm = runner.place_on_field(1, ["BT23-031"])
        top = perm.top_card
        all_effects = top.effect_list(None)
        # Expected:
        #   1. alt-digi (NoTiming factory)
        #   2. BeforePayCost cost reduction
        #   3. On Play (add sec to hand + recovery)
        #   4. When Digivolving (same)
        #   5. Inherited Alliance (factory)
        assert len(all_effects) == 5, (
            f"Expected 5 effects, got {len(all_effects)}: "
            f"{[e.effect_name for e in all_effects]}"
        )
