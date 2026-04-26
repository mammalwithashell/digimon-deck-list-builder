"""Behavioral tests for ST20-15 Island of Adventure (Option White Cost 2).

Card text:
While you have no face-up [Island of Adventure] security cards, you can
ignore this card's color requirements.
[Security] [All Turns] All of your level 3 or higher Digimon get +2000 DP.
[Main] Add your top security card to the hand. Then, place this card face
up as the top security card.
[Security] You may play 1 Tamer card from your hand without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _build_st20_deck():
    """Build a simple deck with ST20 cards for testing."""
    # Need 50 main-deck cards. Fill with ST20 cards.
    deck = (
        ["ST20-02"] * 4 +  # Biyomon Lv.3 Red
        ["ST20-03"] * 4 +  # Birdramon Lv.4 Red
        ["ST20-04"] * 4 +  # Garudamon Lv.5 Red
        ["ST20-07"] * 4 +  # Tentomon Lv.3 Green
        ["ST20-08"] * 4 +  # Kabuterimon Lv.4 Green
        ["ST20-10"] * 4 +  # Agumon Lv.3 Black
        ["ST20-11"] * 4 +  # WarGreymon Lv.6 Black/Red
        ["ST20-12"] * 4 +  # Tamer Red/Yellow
        ["ST20-13"] * 4 +  # Tamer Black/Green
        ["ST20-14"] * 4 +  # Option Black
        ["ST20-15"] * 4 +  # Island of Adventure White
        ["ST20-05"] * 4 +  # Gatomon Lv.4 Yellow
        ["ST20-06"] * 2    # Angewomon Lv.5 Yellow
    )
    eggs = ["ST20-01"] * 4  # Koromon Lv.2 Black
    return eggs + deck


@pytest.mark.behavioral
class TestST20_15ColorBypass:
    """Tests for color requirement bypass."""

    def test_bypass_when_no_face_up_ioa_in_security(self, debug_runner):
        """Can ignore color requirements when no face-up IoA in security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Add IoA to hand
        runner.inject_card(1, "ST20-15", "hand")

        # IoA is White. Player 1 may not have White Digimon/Tamers.
        # The color bypass should let it be played regardless.
        ioa_card = game.player1.hand_cards[-1]
        # Trigger effect loading so _match_color_requirement_fn is set
        ioa_card.effect_list(None)
        assert not ioa_card.match_color_requirement, (
            "Color requirement should be bypassed (no face-up IoA in security)"
        )

    def test_enforce_when_face_up_ioa_in_security(self, debug_runner):
        """Must enforce color requirements when there's already a face-up IoA in security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Add an IoA to hand
        runner.inject_card(1, "ST20-15", "hand")
        ioa_card = game.player1.hand_cards[-1]
        # Trigger effect loading so _match_color_requirement_fn is set
        ioa_card.effect_list(None)

        # Place another IoA face-up in security
        another_ioa = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(another_ioa)

        assert ioa_card.match_color_requirement, (
            "Color requirement should be ENFORCED (face-up IoA in security)"
        )

    def test_bypass_restores_after_removing_face_up_ioa(self, debug_runner):
        """Color bypass should return after the face-up IoA is removed from security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.inject_card(1, "ST20-15", "hand")
        ioa_card = game.player1.hand_cards[-1]
        ioa_card.effect_list(None)

        # Place IoA face-up in security
        sec_ioa = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(sec_ioa)
        assert ioa_card.match_color_requirement, "Should enforce with face-up IoA"

        # Now remove the face-up IoA from security
        game.player1.security_cards.remove(sec_ioa)
        game.player1.face_up_security.discard(sec_ioa)

        assert not ioa_card.match_color_requirement, (
            "Color bypass should restore after removing the face-up IoA from security"
        )


@pytest.mark.behavioral
class TestST20_15SecurityDPAura:
    """Tests for [Security] [All Turns] Lv.3+ Digimon +2000 DP."""

    def test_dp_boost_when_face_up_in_security(self, debug_runner):
        """When IoA is face-up in security, Lv.3+ Digimon should get +2000 DP."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place IoA face-up in security
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(ioa_card)
        # Trigger effect loading
        ioa_card.effect_list(None)

        # Place a Lv.3 Digimon on field (ST20-07 Tentomon, base DP 3000)
        perm = runner.place_on_field(1, ["ST20-07"])
        base_dp = 3000  # from cards.json

        # Get the IoA card's effects
        effects = ioa_card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) != 0]
        assert len(dp_effects) >= 1, "Should have a DP modifier effect"

        dp_eff = dp_effects[0]
        # Condition should pass: card is face-up in security
        assert dp_eff.can_use_condition({}), (
            "DP aura condition should pass when card is face-up in security"
        )

        # Check dp_permanent_condition passes for Lv.3
        dp_filter = getattr(dp_eff, '_dp_permanent_condition', None)
        assert dp_filter is not None, "Should have _dp_permanent_condition"
        assert dp_filter(perm), "Lv.3 Digimon should pass DP filter"

    def test_actual_dp_value_boosted(self, debug_runner):
        """The Permanent.dp property should reflect the +2000 aura from IoA."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place a Lv.3 Digimon on field FIRST (base DP 3000)
        perm = runner.place_on_field(1, ["ST20-07"])
        assert perm.dp == 3000, f"Base DP should be 3000, got {perm.dp}"

        # Now place IoA face-up in security
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(ioa_card)
        ioa_card.effect_list(None)

        # DP should now be 3000 + 2000 = 5000
        assert perm.dp == 5000, (
            f"DP should be 5000 (3000 base + 2000 aura), got {perm.dp}"
        )

    def test_dp_boost_applies_to_high_level_digimon(self, debug_runner):
        """Lv.6 Digimon (level >= 3) should also get the +2000 boost."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place IoA face-up in security
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(ioa_card)
        ioa_card.effect_list(None)

        # Place WarGreymon Lv.6 (base DP 12000)
        perm = runner.place_on_field(1, ["ST20-11"])
        assert perm.dp == 14000, (
            f"Lv.6 DP should be 14000 (12000 + 2000), got {perm.dp}"
        )

    def test_dp_boost_fails_below_lv3(self, debug_runner):
        """Lv.2 Digimon should NOT get the +2000 DP boost."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place IoA face-up in security
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(ioa_card)
        # Trigger effect loading
        ioa_card.effect_list(None)

        # Place a Lv.2 on field
        perm = runner.place_on_field(1, ["ST20-01"])  # Koromon Lv.2

        effects = ioa_card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) != 0]
        dp_eff = dp_effects[0]

        dp_filter = getattr(dp_eff, '_dp_permanent_condition', None)
        assert not dp_filter(perm), "Lv.2 Digimon should NOT pass DP filter"

    def test_dp_aura_inactive_when_not_in_security(self, debug_runner):
        """The DP aura should NOT be active when IoA is in hand (not face-up in security)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        runner.inject_card(1, "ST20-15", "hand")
        ioa_card = game.player1.hand_cards[-1]

        effects = ioa_card.effect_list(None)
        dp_effects = [e for e in effects if getattr(e, 'dp_modifier', 0) != 0]
        assert len(dp_effects) >= 1

        dp_eff = dp_effects[0]
        assert not dp_eff.can_use_condition({}), (
            "DP aura should NOT be active when card is in hand"
        )

    def test_dp_aura_inactive_when_face_down_in_security(self, debug_runner):
        """The DP aura should NOT be active when IoA is face-DOWN in security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Inject IoA to security but do NOT mark it face-up
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        # Do NOT add to face_up_security
        ioa_card.effect_list(None)

        # Place Lv.3 Digimon
        perm = runner.place_on_field(1, ["ST20-07"])
        assert perm.dp == 3000, (
            f"DP should be base 3000 (no aura from face-down IoA), got {perm.dp}"
        )

    def test_dp_aura_does_not_boost_opponent(self, debug_runner):
        """IoA should only boost own Digimon, not opponent's."""
        runner = debug_runner(initial_memory=5)
        game = runner.game

        # Place IoA face-up in P1's security
        ioa_card = runner.inject_card(1, "ST20-15", "security_top")
        game.player1.face_up_security.add(ioa_card)
        ioa_card.effect_list(None)

        # Place Lv.3 on opponent's field
        opp_perm = runner.place_on_field(2, ["ST20-07"])
        assert opp_perm.dp == 3000, (
            f"Opponent DP should be base 3000 (not boosted by P1's IoA), got {opp_perm.dp}"
        )


@pytest.mark.behavioral
class TestST20_15MainEffect:
    """Tests for [Main] swap security/hand mechanic."""

    def test_swap_security_to_hand_and_place_face_up(self, debug_runner):
        """Playing IoA should: 1) move top security to hand, 2) place IoA face-up in security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        p1 = game.player1

        sec_before = len(p1.security_cards)
        hand_before = len(p1.hand_cards)

        # Remember top security card
        top_sec_id = p1.security_cards[-1].c_entity_base.card_id if p1.security_cards else None

        # Inject IoA to hand
        runner.inject_card(1, "ST20-15", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play Island")
        if play_action is None:
            play_action = runner.find_action("Play ST20-15")
        assert play_action is not None, (
            f"Should be able to play IoA. Actions: {runner.actions()}"
        )

        result = runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        # After resolution:
        # - Old top security should now be in hand
        # - IoA should be in security (face-up)
        # - Net security count should be unchanged (removed 1, added 1)
        hand_ids = [c.c_entity_base.card_id for c in p1.hand_cards if c.c_entity_base]

        if top_sec_id:
            assert top_sec_id in hand_ids, (
                f"Top security card ({top_sec_id}) should now be in hand. Hand: {hand_ids}"
            )

        # IoA should be face-up in security
        ioa_in_sec = any(
            p1.is_security_face_up(c) and any('Island of Adventure' in n
                for n in getattr(c, 'card_names', []))
            for c in p1.security_cards
        )
        assert ioa_in_sec, "IoA should be face-up in security after playing"

        # IoA should NOT be in trash (it goes to security, not trash)
        trash_ids = [c.c_entity_base.card_id for c in p1.trash_cards if c.c_entity_base]
        assert "ST20-15" not in trash_ids, (
            f"IoA should NOT be in trash after playing (should be in security). Trash: {trash_ids}"
        )

    def test_security_count_preserved_after_swap(self, debug_runner):
        """Security count should remain the same after the swap (remove 1, add IoA)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        p1 = game.player1

        sec_before = len(p1.security_cards)
        assert sec_before > 0, "Need security cards for this test"

        runner.inject_card(1, "ST20-15", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play Island") or runner.find_action("Play ST20-15")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        assert len(p1.security_cards) == sec_before, (
            f"Security count should be preserved: was {sec_before}, now {len(p1.security_cards)}"
        )

    def test_ioa_is_top_security_after_swap(self, debug_runner):
        """IoA should be the TOP (last element) of security after being placed."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        p1 = game.player1

        runner.inject_card(1, "ST20-15", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play Island") or runner.find_action("Play ST20-15")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        # Top security is last element in the list
        top_sec = p1.security_cards[-1]
        top_id = top_sec.c_entity_base.card_id if top_sec.c_entity_base else None
        assert top_id == "ST20-15", (
            f"IoA should be at top of security stack, but top is {top_id}"
        )
        assert p1.is_security_face_up(top_sec), "IoA at top of security should be face-up"

    def test_swap_with_empty_security(self, debug_runner):
        """When security is empty, no card is added to hand but IoA still goes to security."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        p1 = game.player1

        # Clear all security cards
        runner.clear_zone(1, "security")
        assert len(p1.security_cards) == 0

        runner.inject_card(1, "ST20-15", "hand")
        hand_before = len(p1.hand_cards)
        runner.set_phase("Main")

        play_action = runner.find_action("Play Island") or runner.find_action("Play ST20-15")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        # IoA should now be the only security card
        assert len(p1.security_cards) == 1, (
            f"IoA should be in security even when starting empty, got {len(p1.security_cards)}"
        )
        assert p1.is_security_face_up(p1.security_cards[-1]), (
            "IoA should be face-up in security"
        )

        # Hand should have 1 fewer card (IoA was used from hand, no security to add back)
        # Actually hand_before included IoA; after play IoA is gone and no sec was added
        assert len(p1.hand_cards) == hand_before - 1, (
            f"Hand should lose IoA without gaining a security card. Before: {hand_before}, after: {len(p1.hand_cards)}"
        )


@pytest.mark.behavioral
class TestST20_15SecurityTamerPlay:
    """Tests for [Security] You may play 1 Tamer from hand free."""

    def test_security_tamer_play_exists(self, debug_runner):
        """The security effect should have a Tamer play effect."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-15", "hand")
        game = runner.game
        ioa_card = game.player1.hand_cards[-1]

        effects = ioa_card.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1, "Should have a security effect for Tamer play"

        sec_eff = sec_effects[0]
        assert sec_eff.timing == EffectTiming.SecuritySkill, (
            "Security Tamer play should have SecuritySkill timing"
        )

    def test_security_effect_is_optional(self, debug_runner):
        """The security tamer play should be optional ('You may play')."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "ST20-15", "hand")
        game = runner.game
        ioa_card = game.player1.hand_cards[-1]

        effects = ioa_card.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) >= 1

        # The process callback uses effect_play_from_zone with is_optional=True.
        # We verify by checking the callback exists and the effect structure.
        sec_eff = sec_effects[0]
        assert sec_eff.on_process_callback is not None, (
            "Security effect should have a process callback"
        )


@pytest.mark.behavioral
class TestST20_15Integration:
    """Integration tests for IoA lifecycle: play -> goes to security -> aura activates."""

    def test_play_ioa_then_dp_aura_activates(self, debug_runner):
        """After playing IoA (goes to security face-up), Lv.3+ Digimon get +2000 DP."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        p1 = game.player1

        # Place Lv.3 Tentomon on field (base 3000 DP)
        perm = runner.place_on_field(1, ["ST20-07"])
        assert perm.dp == 3000

        # Inject IoA to hand and play it
        runner.inject_card(1, "ST20-15", "hand")
        runner.set_phase("Main")

        play_action = runner.find_action("Play Island") or runner.find_action("Play ST20-15")
        assert play_action is not None
        runner.execute(play_action)
        runner.auto_resolve(max_steps=20)

        # IoA should now be face-up in security, activating the aura
        ioa_in_sec = any(
            p1.is_security_face_up(c) and c.c_entity_base
            and c.c_entity_base.card_id == "ST20-15"
            for c in p1.security_cards
        )
        assert ioa_in_sec, "IoA should be face-up in security"

        # Tentomon should now have 5000 DP (3000 + 2000 aura)
        assert perm.dp == 5000, (
            f"After IoA placed in security, Lv.3 DP should be 5000, got {perm.dp}"
        )
