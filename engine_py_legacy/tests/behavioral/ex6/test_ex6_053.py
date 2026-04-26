"""Behavioral tests for EX6-053 LadyDevimon (Lv.5 Purple Fallen Angel).

Card text (from cards.json):

Main effect:
  <Retaliation> (When this Digimon is deleted after losing a battle, delete the
      Digimon it was battling.)
  [On Play] [When Digivolving] If you have a [Mirei Mikagura], delete 1 of your
      opponent's level 4 or lower Digimon. If you don't have a [Mirei Mikagura],
      you may play 1 [Mirei Mikagura] from your trash without paying the cost.

Inherited effect:
  [All Turns] While this Digimon has the [Angel] or [Seven Great Demon Lords]
      trait, it gains <Scapegoat> (When this Digimon would be deleted other than
      by your effects, by deleting 1 of your other Digimon, prevent that deletion.)

Key faithfulness points:
  - Retaliation on main card (NOT inherited) — is_on_deletion + _is_retaliation.
  - On Play & When Digivolving produce SAME logic — two separate effects.
  - The effect is conditional: if Mirei on field -> delete; else -> play from trash.
  - Delete target filter: opponent's Digimon with level <= 4 (mandatory choice).
  - Play-from-trash filter: CardSource with "Mirei Mikagura" in its names.
  - Play-from-trash is optional (player may decline).
  - Scapegoat is inherited, and ONLY active if top card has Angel or SGDL trait
    (substring match so "Fallen Angel" also qualifies).
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


EX6_053 = "EX6-053"          # LadyDevimon Lv.5 Purple (Fallen Angel)
EX6_074 = "EX6-074"          # Mirei Mikagura Tamer
BT11_094 = "BT11-094"        # Mirei Mikagura Tamer (alt)
BT1_009 = "BT1-009"          # Monodramon Lv.3 (no trait Angel/SGDL)
AD1_001 = "AD1-001"          # Greymon Lv.4 (no trait Angel/SGDL)
AD1_004 = "AD1-004"          # WarGreymon Lv.6 (Dragonkin/Hero — no Angel/SGDL)
BT7_111 = "BT7-111"          # Lucemon: Chaos Mode Lv.5 (SGDL)
BT1_063 = "BT1-063"          # Seraphimon Lv.6 (Angel/Seraph — has 'Angel' substring)
ST1_03 = "ST1-03"            # filler
ST1_02 = "ST1-02"            # filler

FILLER_DECK = [ST1_03] * 4 + [ST1_02] * 50


def _get_effects(runner, card_id: str):
    """Return the cached effect list for a freshly-created CardSource."""
    from digimon_gym.engine.data.card_database import CardDatabase
    db = CardDatabase()
    cs = db.create_card_source(card_id, runner.game.player1)
    return cs, cs.effect_list(EffectTiming.NoTiming)


@pytest.mark.behavioral
class TestEX6053Structure:
    """Structural tests: correct effect count, timings, flags."""

    def test_has_retaliation_effect(self, debug_runner):
        """Main card gains <Retaliation> via factory flag (is_on_deletion + _is_retaliation)."""
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        _, effects = _get_effects(runner, EX6_053)
        retaliation_effects = [
            e for e in effects
            if getattr(e, '_is_retaliation', False)
        ]
        assert len(retaliation_effects) >= 1, (
            "Should have 1 <Retaliation> factory effect"
        )
        # Retaliation on main card — NOT inherited
        assert not retaliation_effects[0].is_inherited_effect, (
            "Main <Retaliation> must NOT be an inherited effect "
            "(C# uses isInheritedEffect: false)"
        )

    def test_has_on_play_effect(self, debug_runner):
        """[On Play] effect exists with OnEnterFieldAnyone timing and is_on_play."""
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        _, effects = _get_effects(runner, EX6_053)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, (
            "Should have at least one [On Play] effect"
        )

    def test_has_when_digivolving_effect(self, debug_runner):
        """[When Digivolving] effect exists with OnEnterFieldAnyone timing and is_when_digivolving."""
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        _, effects = _get_effects(runner, EX6_053)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, (
            "Should have at least one [When Digivolving] effect"
        )

    def test_has_inherited_scapegoat_effect(self, debug_runner):
        """Scapegoat keyword must be inherited and present as _is_scapegoat flag."""
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        _, effects = _get_effects(runner, EX6_053)
        scapegoat_effects = [
            e for e in effects
            if getattr(e, '_is_scapegoat', False)
        ]
        assert len(scapegoat_effects) >= 1, (
            "Should have 1 <Scapegoat> factory effect"
        )
        # Inherited — it's described in the inherited_effect_description_eng
        assert scapegoat_effects[0].is_inherited_effect, (
            "<Scapegoat> on EX6-053 is below the inheritance line — must be inherited"
        )


@pytest.mark.behavioral
class TestEX6053ScapegoatTraitGate:
    """Inherited Scapegoat is trait-gated to [Angel] or [Seven Great Demon Lords]."""

    def test_scapegoat_not_active_when_ex6_053_is_top(self, debug_runner):
        """Inherited effects only apply to the Digimon that digivolved ON TOP of
        EX6-053. Naked EX6-053 alone does NOT inherit its own Scapegoat
        (inherited effects come from sources UNDER the top card).
        """
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        perm = runner.place_on_field(1, [EX6_053])
        # Inherited effects activate for card_sources[:-1] only — EX6-053
        # alone has no under-cards so no inherited effects apply.
        assert not perm.has_keyword('_is_scapegoat'), (
            "Inherited Scapegoat only applies to a Digimon ON TOP of EX6-053."
        )

    def test_scapegoat_active_under_seraphimon(self, debug_runner):
        """EX6-053 underneath Seraphimon (Seraph, Three Great Angels): top card
        traits contain 'Angel' substring — Scapegoat should be active.
        Also verifies the inherited effect reads TOP CARD traits (not the
        source card EX6-053's traits).
        """
        runner = debug_runner(
            deck1=[EX6_053] * 2 + [BT1_063] * 2 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        perm = runner.place_on_field(1, [EX6_053, BT1_063])
        # BT1-063 Seraphimon types: ['Seraph', 'Three Great Angels'] — contains 'Angel'
        assert perm.has_keyword('_is_scapegoat'), (
            "With Seraphimon (Three Great Angels) on top of EX6-053, "
            "Scapegoat inherited effect should activate"
        )

    def test_scapegoat_active_under_sgdl(self, debug_runner):
        """EX6-053 under Lucemon: Chaos Mode (SGDL trait)."""
        runner = debug_runner(
            deck1=[EX6_053] * 2 + [BT7_111] * 2 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        perm = runner.place_on_field(1, [EX6_053, BT7_111])
        assert perm.has_keyword('_is_scapegoat'), (
            "With Lucemon: Chaos Mode (Seven Great Demon Lords) on top, "
            "Scapegoat should activate"
        )

    def test_scapegoat_inactive_under_non_angel_non_sgdl(self, debug_runner):
        """EX6-053 under WarGreymon (Dragonkin/Hero): no Angel or SGDL trait.
        Scapegoat should NOT be active.
        """
        runner = debug_runner(
            deck1=[EX6_053] * 2 + [AD1_004] * 2 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        perm = runner.place_on_field(1, [EX6_053, AD1_004])
        assert not perm.has_keyword('_is_scapegoat'), (
            "WarGreymon (Dragonkin, Hero) on top — should NOT inherit "
            "Scapegoat (not Angel, not SGDL)"
        )


@pytest.mark.behavioral
class TestEX6053OnPlayLogic:
    """[On Play] / [When Digivolving] conditional effect logic:
    - If you have a [Mirei Mikagura] (Tamer on field) -> mandatory delete.
    - Else -> optional play Mirei Mikagura from trash.
    """

    def test_on_play_process_callbacks_exist(self, debug_runner):
        """Both On Play and When Digivolving effects must have process callbacks set."""
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [ST1_02] * 46,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        _, effects = _get_effects(runner, EX6_053)
        triggered = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and (getattr(e, 'is_on_play', False)
                 or getattr(e, 'is_when_digivolving', False))
        ]
        assert len(triggered) >= 2, (
            "Should have both [On Play] and [When Digivolving] effects"
        )
        for eff in triggered:
            assert eff.on_process_callback is not None, (
                "Effect must have a process callback implementation"
            )

    def test_delete_branch_targets_opp_lv4_or_lower(self, debug_runner):
        """When player has Mirei Mikagura and opponent has a Lv.4 Digimon,
        the [On Play] process should delete the opponent's Lv.4 Digimon.
        """
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [EX6_074] * 4 + [ST1_02] * 42,
            deck2=[AD1_001] * 4 + [ST1_02] * 50,
            initial_memory=3,
        )
        # Setup: P1 has Mirei Mikagura on field + LadyDevimon freshly played
        runner.place_on_field(1, [EX6_074])  # Mirei Mikagura Tamer
        runner.place_on_field(2, [AD1_001])  # Greymon Lv.4
        # Place EX6-053 and trigger its On Play process
        lady = runner.place_on_field(1, [EX6_053])

        # Find EX6-053's effect from its actual top_card (field instance)
        lady_cs = lady.top_card
        effects = lady_cs.effect_list(EffectTiming.NoTiming)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert on_play, "Missing on play effect"
        eff = on_play[0]
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'card': lady_cs,
            'permanent': lady,
        }
        if eff.can_use_condition:
            assert eff.can_use_condition(ctx), "Condition should allow activation"

        target_count_before = len(runner.game.player2.battle_area)
        eff.on_process_callback(ctx)

        # Auto-resolve the mandatory opponent-permanent selection phase
        runner.auto_resolve(max_steps=20)

        target_count_after = len(runner.game.player2.battle_area)
        assert target_count_after < target_count_before, (
            "With Mirei on field and Lv.4 opponent Digimon present, "
            "the delete branch should have reduced opponent battle area count"
        )

    def test_play_mirei_from_trash_branch_has_correct_filter(self, debug_runner):
        """When player has NO Mirei Mikagura on field but has one in trash,
        the play-from-trash branch should select ONLY Mirei Mikagura cards
        (not level-based filter).
        """
        runner = debug_runner(
            deck1=[EX6_053] * 4 + [EX6_074] * 4 + [ST1_02] * 42,
            deck2=FILLER_DECK,
            initial_memory=3,
        )
        # Put Mirei in trash; no Mirei on field
        p1 = runner.game.player1
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        mirei_cs = db.create_card_source(EX6_074, p1)
        # Also put a random non-Mirei card in trash
        filler_cs = db.create_card_source(BT1_009, p1)
        p1.trash_cards.append(mirei_cs)
        p1.trash_cards.append(filler_cs)

        lady = runner.place_on_field(1, [EX6_053])
        lady_cs = lady.top_card

        effects = lady_cs.effect_list(EffectTiming.NoTiming)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert on_play
        eff = on_play[0]
        ctx = {
            'game': runner.game,
            'player': p1,
            'card': lady_cs,
            'permanent': lady,
        }

        # We want to verify: filter selects ONLY Mirei Mikagura (by name), not by level.
        # The current buggy script uses level <= 4 which excludes Tamers (level None)
        # and accepts Monodramon (Lv.3) — both wrong.
        eff.on_process_callback(ctx)

        # Check that selection is pending with only Mirei valid (Tamer with no level)
        # The current script's bug: filter_fn uses level <= 4 which excludes Tamers
        # (level is None) — so valid list would be EMPTY and Mirei wouldn't be playable.
        # Instead, the fixed version must filter by name containing "Mirei Mikagura".
        # To assert this, check that the pending selection includes the Mirei trash index.
        import digimon_gym.engine.game.constants as const
        SEL_TRASH_START = getattr(const, 'SEL_TRASH_START', None)
        if SEL_TRASH_START is None:
            SEL_TRASH_START = 130
        pending = runner.game.pending_selection
        if pending is None:
            pytest.fail(
                "Expected a trash-selection pending after invoking the effect "
                "but none was registered. This indicates the filter rejected "
                "Mirei Mikagura (likely because of a wrong level <= 4 filter)."
            )
        valid = list(pending.valid_indices)
        mirei_idx = p1.trash_cards.index(mirei_cs)
        filler_idx = p1.trash_cards.index(filler_cs)
        expected_action = SEL_TRASH_START + mirei_idx
        filler_action = SEL_TRASH_START + filler_idx
        assert expected_action in valid, (
            f"Mirei Mikagura ({expected_action}) should be a valid selection target. "
            f"valid={valid}"
        )
        assert filler_action not in valid, (
            f"Non-Mirei trash card ({filler_action}) should NOT be a valid target. "
            f"valid={valid}"
        )
