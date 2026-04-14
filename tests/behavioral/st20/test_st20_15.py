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
from digimon_gym.engine.data.enums import EffectTiming


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
