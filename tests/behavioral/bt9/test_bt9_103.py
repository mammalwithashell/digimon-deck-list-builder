"""Behavioral tests for BT9-103 Kongou (Option, Black, Cost 2).

Card text:
  [Main] Until the end of your opponent's turn, your opponent's Digimon with
  play costs of 7 or less can't attack players, and cards can't be added to
  security stacks by your opponent's effects.

  [Security] Activate this card's [Main] effects.

C# reference: BT9_103.cs
  - Main: GainCanNotAttackPlayerEffect (NOT general CANNOT_ATTACK!)
      attackerCondition: IsPermanentExistsOnOpponentBattleAreaDigimon && GetCostItself <= 7
      defenderCondition: Defender == null (i.e., attacking player)
      effectDuration: UntilOpponentTurnEnd
  - Main part 2: CannotAddSecurityClass with IsOpponentEffect condition
  - Security: AddActivateMainOptionSecurityEffect

Key distinction: "can't attack PLAYERS" means restricted Digimon CAN still
attack other Digimon. The modifier must be CANNOT_ATTACK_PLAYER, not CANNOT_ATTACK.

Key cards used:
  BT5-061: Commandramon (Black Lv3, Cost 4) - black Digimon for color requirement
  BT2-058: Guardromon (Black Lv4, Cost 5) - cost <= 7, should be restricted
  BT2-064: HiAndromon (Black Lv6, Cost 10) - cost > 7, should NOT be restricted
  ST1-03: Agumon (Red Lv3, Cost 3) - cost <= 7, should be restricted
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# Need black Digimon in deck to meet option color requirement
_DECK = ["BT9-103"] * 4 + ["BT5-061"] * 4 + ["ST1-03"] * 42


@pytest.mark.behavioral
class TestBT9103Effects:
    """Verify BT9-103 has the correct effect structure."""

    def test_has_option_skill(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-103")
        effects = cs.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OptionSkill]
        assert len(main_effects) >= 1, "Should have OptionSkill effect"

    def test_has_security_skill(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT9-103")
        effects = cs.effect_list(None)
        sec_effects = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effects) >= 1, "Should have SecuritySkill effect"
        assert sec_effects[0].is_security_effect is True


@pytest.mark.behavioral
class TestBT9103MainEffect:
    """[Main] Opponent's Digimon cost<=7 can't attack PLAYERS; no security additions."""

    def test_attack_restriction_uses_cannot_attack_player(self, debug_runner):
        """Must use CANNOT_ATTACK_PLAYER, not CANNOT_ATTACK.
        Restricted Digimon can still attack other Digimon."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game

        # Place own black Digimon for color requirement
        runner.place_on_field(1, ["BT5-061"])
        # Place opponent Digimon with cost <= 7
        runner.place_on_field(2, ["BT2-058"])  # Guardromon, cost 5

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT9-103", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Kongou")
        if action is None:
            action = runner.find_action("BT9-103")
        assert action is not None, f"Available: {runner.actions()}"

        runner.execute(action)
        runner.auto_resolve()

        # Find Guardromon on P2 field
        guardromon = None
        for p in game.player2.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT2-058":
                guardromon = p
                break
        assert guardromon is not None, "Guardromon should be on P2 field"

        # Check that CANNOT_ATTACK_PLAYER is registered
        has_cant_attack_player = game.modifiers.has_modifier(
            guardromon, ModifierType.CANNOT_ATTACK_PLAYER
        )
        assert has_cant_attack_player, (
            "Guardromon (cost 5) should have CANNOT_ATTACK_PLAYER modifier"
        )

        # MUST NOT have CANNOT_ATTACK (would wrongly prevent all attacks)
        has_cant_attack = game.modifiers.has_modifier(
            guardromon, ModifierType.CANNOT_ATTACK
        )
        assert not has_cant_attack, (
            "Should NOT use CANNOT_ATTACK (blocks all attacks). "
            "Must use CANNOT_ATTACK_PLAYER so Digimon can still attack other Digimon."
        )

    def test_high_cost_digimon_not_restricted(self, debug_runner):
        """Opponent's Digimon with cost > 7 should NOT be restricted."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, ["BT5-061"])  # Black for color req
        runner.place_on_field(2, ["BT2-064"])  # HiAndromon, cost 10

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT9-103", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Kongou")
        if action is None:
            action = runner.find_action("BT9-103")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        hiandromon = None
        for p in game.player2.battle_area:
            if p.top_card and p.top_card.c_entity_base.card_id == "BT2-064":
                hiandromon = p
                break
        assert hiandromon is not None

        has_modifier = game.modifiers.has_modifier(
            hiandromon, ModifierType.CANNOT_ATTACK_PLAYER
        )
        assert not has_modifier, (
            "HiAndromon (cost 10) should NOT have attack restriction"
        )

    def test_both_low_and_high_cost_correctly_filtered(self, debug_runner):
        """With both low-cost and high-cost Digimon, only low-cost gets restricted."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, ["BT5-061"])  # Black for color req
        runner.place_on_field(2, ["BT2-058"])  # Guardromon, cost 5
        runner.place_on_field(2, ["BT2-064"])  # HiAndromon, cost 10

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT9-103", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Kongou")
        if action is None:
            action = runner.find_action("BT9-103")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        guardromon = hiandromon = None
        for p in game.player2.battle_area:
            cid = p.top_card.c_entity_base.card_id if p.top_card else None
            if cid == "BT2-058":
                guardromon = p
            elif cid == "BT2-064":
                hiandromon = p

        assert guardromon is not None and hiandromon is not None

        assert game.modifiers.has_modifier(
            guardromon, ModifierType.CANNOT_ATTACK_PLAYER
        ), "Guardromon (cost 5) should be restricted"

        assert not game.modifiers.has_modifier(
            hiandromon, ModifierType.CANNOT_ATTACK_PLAYER
        ), "HiAndromon (cost 10) should NOT be restricted"

    def test_cannot_add_security_modifier_registered(self, debug_runner):
        """Should register CANNOT_ADD_SECURITY global modifier."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, ["BT5-061"])  # Black for color req
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT9-103", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Kongou")
        if action is None:
            action = runner.find_action("BT9-103")
        assert action is not None

        runner.execute(action)
        runner.auto_resolve()

        # Check that a CANNOT_ADD_SECURITY modifier is registered
        security_entries = game.modifiers._modifiers.get(
            ModifierType.CANNOT_ADD_SECURITY, []
        )
        assert len(security_entries) >= 1, (
            "Should have CANNOT_ADD_SECURITY modifier registered"
        )


@pytest.mark.behavioral
class TestBT9103SecurityEffect:
    """[Security] Activate this card's [Main] effects."""

    def test_security_activates_main_effects(self, debug_runner):
        """Security should apply the same restrictions as the Main effect."""
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(deck1=_DECK, deck2=_DECK, initial_memory=5)
        game = runner.game
        p2 = game.player2

        # P1 has a cost<=7 Digimon (should get restricted by P2's security)
        runner.place_on_field(1, ["ST1-03"])  # Agumon cost 3

        # Clear P2 security, inject BT9-103
        runner.clear_zone(2, "security")
        runner.inject_card(2, "BT9-103", "security_top")

        from tests.helpers.game_builder import make_permanent
        attacker = make_permanent("ATK-001", "StrongAttacker", dp=12000)
        game.player1.battle_area.append(attacker)

        p2.security_attack(attacker)
        runner.auto_resolve()

        # After security, the restrictions should be applied
        # P1's Agumon (cost 3, <= 7) should have CANNOT_ATTACK_PLAYER
        agumon = None
        for p in game.player1.battle_area:
            if p.top_card and p.top_card.c_entity_base and p.top_card.c_entity_base.card_id == "ST1-03":
                agumon = p
                break

        if agumon is not None:
            has_modifier = game.modifiers.has_modifier(
                agumon, ModifierType.CANNOT_ATTACK_PLAYER
            )
            assert has_modifier, (
                "Security effect should restrict P1's cost<=7 Digimon from attacking players"
            )
