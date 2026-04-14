"""Behavioral tests for EX11-021 Kokeshimon (Lv.4, Yellow, Puppet/LIBERATOR).

Card text:
  Alt-digi: From Lv.3 [Puppet] for cost 2

  [When Digivolving] If you have 1 or fewer Tamers, you may play 1
      [Mirai Kinosaki] from your hand without paying the cost.

  --- Inherited ---
  [Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks,
      by deleting 1 of your other Digimon, end that attack.

C# reference: EX11_021.cs
  - Inherited uses EffectTiming.OnAllyAttack (not OnUseAttack)
  - Inherited hash: "StopAttack_EX9-027" in C# but this collides with EX9-027;
    must be unique: "StopAttack_EX11-021"
  - When Digivolving checks tamer count <= 1
  - Mirai Kinosaki filter uses EqualsCardName — exact match
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX11021WhenDigivolving:
    """Tests for EX11-021 Kokeshimon [When Digivolving] effect."""

    def test_when_digivolving_with_zero_tamers_plays_mirai(self, debug_runner):
        """When Digivolving with 0 tamers, should allow playing Mirai Kinosaki."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place Kokeshimon on field (simulating digivolution)
        perm = runner.place_on_field(1, ["EX11-021"])

        # Inject Mirai Kinosaki into hand
        runner.inject_card(1, "EX11-061", "hand")  # EX11-061 is Mirai Kinosaki

        card = perm.card_sources[0]
        effects = card.effect_list(EffectTiming.OnEnterFieldAnyone)
        digi_effects = [e for e in effects if e.is_when_digivolving]
        assert len(digi_effects) == 1, "Should have 1 When Digivolving effect"

        effect = digi_effects[0]

        # 0 tamers — condition should pass
        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count == 0, "Should have 0 tamers initially"
        assert effect.can_use_condition({}), (
            "Condition should pass with 0 tamers"
        )

    def test_when_digivolving_with_one_tamer_still_passes(self, debug_runner):
        """With exactly 1 tamer, the condition should still pass (<=1)."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021"])

        # Place 1 tamer on field — use a generic tamer
        runner.place_on_field(1, ["ST1-12"])  # ST1-12 is a Tamer

        card = perm.card_sources[0]
        effects = card.effect_list(EffectTiming.OnEnterFieldAnyone)
        digi_effects = [e for e in effects if e.is_when_digivolving]
        effect = digi_effects[0]

        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count == 1, "Should have exactly 1 tamer"
        assert effect.can_use_condition({}), (
            "Condition should pass with 1 tamer"
        )

    def test_when_digivolving_with_two_tamers_fails(self, debug_runner):
        """With 2 or more tamers, the condition should fail."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021"])

        # Place 2 tamers
        runner.place_on_field(1, ["ST1-12"])
        runner.place_on_field(1, ["ST1-12"])

        card = perm.card_sources[0]
        effects = card.effect_list(EffectTiming.OnEnterFieldAnyone)
        digi_effects = [e for e in effects if e.is_when_digivolving]
        effect = digi_effects[0]

        tamer_count = sum(1 for p in game.player1.battle_area if p.is_tamer)
        assert tamer_count >= 2, "Should have 2+ tamers"
        assert not effect.can_use_condition({}), (
            "Condition should fail with 2+ tamers"
        )


@pytest.mark.behavioral
class TestEX11021InheritedStopAttack:
    """Tests for EX11-021 Kokeshimon inherited stop-attack effect."""

    def test_inherited_uses_on_ally_attack_timing(self, debug_runner):
        """The inherited effect must use OnAllyAttack timing (not OnUseAttack),
        so it fires on other permanents when any Digimon attacks."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        # Place a Lv.5 digivolved over Kokeshimon
        perm = runner.place_on_field(1, ["EX11-021", "ST1-10"], turn_played=-1)

        kokeshimon_card = perm.card_sources[0]  # bottom card
        effects = kokeshimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]

        assert len(inherited) >= 1, (
            "Kokeshimon should have inherited effect on OnAllyAttack timing"
        )

    def test_inherited_does_not_use_on_use_attack(self, debug_runner):
        """The inherited effect must NOT use OnUseAttack timing."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021", "ST1-10"])

        kokeshimon_card = perm.card_sources[0]
        # effect_list ignores timing param, so filter by .timing attribute
        effects = kokeshimon_card.effect_list(None)
        inherited_on_use_attack = [
            e for e in effects
            if e.is_inherited_effect and e.is_on_attack
            and e.timing == EffectTiming.OnUseAttack
        ]

        assert len(inherited_on_use_attack) == 0, (
            "Kokeshimon should NOT have inherited effects with OnUseAttack timing"
        )

    def test_inherited_hash_is_unique(self, debug_runner):
        """The inherited effect hash must be unique to EX11-021, not collide
        with EX9-027 ('StopAttack_EX9-027')."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021", "ST1-10"])

        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]

        assert len(inherited) >= 1
        hash_str = inherited[0].hash_string
        assert hash_str == "StopAttack_EX11-021", (
            f"Hash string should be 'StopAttack_EX11-021', got '{hash_str}'. "
            f"Must NOT use 'StopAttack_EX9-027' to avoid collision."
        )

    def test_inherited_only_on_opponent_turn(self, debug_runner):
        """The inherited effect condition should only pass on the opponent's turn."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021", "ST1-10"], turn_played=-1)
        # Need another digimon to delete
        runner.place_on_field(1, ["ST1-03"], turn_played=-1)

        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]
        assert len(inherited) >= 1

        effect = inherited[0]

        # Currently P1's turn — condition should fail
        assert game.player1.is_my_turn, "Should be P1's turn"
        assert not effect.can_use_condition({}), (
            "Inherited condition should fail on own turn"
        )

    def test_inherited_hash_does_not_collide_with_ex9_027(self, debug_runner):
        """EX11-021's hash must not be 'StopAttack_EX9-027' to avoid
        once-per-turn collision when both cards are in the same stack."""
        runner = debug_runner(initial_memory=10)
        game = runner.game

        perm = runner.place_on_field(1, ["EX11-021", "ST1-10"], turn_played=-1)

        kokeshimon_card = perm.card_sources[0]
        effects = kokeshimon_card.effect_list(EffectTiming.OnAllyAttack)
        inherited = [e for e in effects if e.is_inherited_effect and e.is_on_attack]

        assert len(inherited) >= 1
        hash_str = inherited[0].hash_string
        assert hash_str != "StopAttack_EX9-027", (
            f"Hash string must NOT be 'StopAttack_EX9-027' (collides with EX9-027), "
            f"got '{hash_str}'"
        )
        assert "EX11-021" in hash_str, (
            f"Hash string should contain 'EX11-021', got '{hash_str}'"
        )
