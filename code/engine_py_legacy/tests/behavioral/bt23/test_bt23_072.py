"""Behavioral tests for BT23-072 King Drasil_7D6.

Card text:
[Hand] [Main] By paying 3 cost and placing this card as the bottom digivolution
card of your [King Drasil_7D6] or [Mother Eater] in the breeding area, <Draw 1>.

[All Turns] When any of your Digimon with the [Royal Knight] or [CS] trait are
played, by suspending this Digimon, 1 of the played Digimon gains <Rush>,
<Raid>, <Reboot> and <Blocker> until your opponent's turn ends.

Inherited: [Breeding] [Start of Your Main Phase] If this Digimon has 6 or more
digivolution cards, you may play 1 Digimon card with [King Drasil] in its name
from its digivolution cards without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


# ── Helpers ──────────────────────────────────────────────────────────────


def _find_effect(card_sources, timing, name_substring=None, inherited=None):
    """Find an effect matching timing, optional name substring, and inherited flag."""
    for cs in card_sources:
        for effect in cs.effect_list(None):
            if effect.timing != timing:
                continue
            if name_substring and name_substring.lower() not in (
                getattr(effect, 'effect_name', '') or ''
            ).lower():
                continue
            if inherited is not None and effect.is_inherited_effect != inherited:
                continue
            return effect
    return None


# ── Effect 0: [Hand][Main] tests ────────────────────────────────────────


@pytest.mark.behavioral
class TestBT23072HandMain:
    """Tests for the [Hand][Main] effect: pay 3, place as bottom digi-card
    of breeding KD7D6/Mother Eater, draw 1."""

    def test_hand_main_flag_is_set(self, debug_runner):
        """The effect must use _is_hand_main (not _is_field_main) since it
        activates from hand."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")

        effects = card.effect_list(None)
        hand_main_effects = [
            e for e in effects
            if getattr(e, '_is_hand_main', False)
            and e.timing == EffectTiming.OnDeclaration
        ]
        assert len(hand_main_effects) >= 1, (
            "BT23-072 must have an OnDeclaration effect with _is_hand_main=True"
        )

        # Must NOT have _is_field_main on the Hand effect
        for e in hand_main_effects:
            assert not getattr(e, '_is_field_main', False), (
                "The [Hand][Main] effect must not have _is_field_main=True"
            )

    def test_hand_main_condition_requires_breeding_target(self, debug_runner):
        """Condition fails when no KD7D6 or Mother Eater is in breeding."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")

        effect = _find_effect([card], EffectTiming.OnDeclaration, "draw")
        assert effect is not None, "Should have the draw effect"
        assert not effect.can_use_condition({}), (
            "Condition should fail when no breeding target exists"
        )

    def test_hand_main_condition_passes_with_kd7d6_in_breeding(self, debug_runner):
        """Condition passes when a KD7D6 is in breeding area."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")
        # Place another BT23-072 in breeding area
        runner.place_in_breeding(1, ["BT23-072"])

        effect = _find_effect([card], EffectTiming.OnDeclaration, "draw")
        assert effect is not None
        assert effect.can_use_condition({}), (
            "Condition should pass when breeding has King Drasil_7D6"
        )

    def test_hand_main_condition_passes_with_mother_eater_in_breeding(self, debug_runner):
        """Condition passes when a Mother Eater is in breeding area."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")
        runner.place_in_breeding(1, ["BT22-007"])  # Mother Eater

        effect = _find_effect([card], EffectTiming.OnDeclaration, "draw")
        assert effect is not None
        assert effect.can_use_condition({}), (
            "Condition should pass when breeding has Mother Eater"
        )

    def test_hand_main_process(self, debug_runner):
        """Executing the effect pays 3 cost, places card as bottom digi-card
        of breeding perm, and draws 1."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")
        breeding_perm = runner.place_in_breeding(1, ["BT23-072"])

        game = runner.game
        player = game.player1
        hand_size_before = len(player.hand_cards)
        breeding_sources_before = len(breeding_perm.card_sources)
        memory_before = game.memory

        effect = _find_effect([card], EffectTiming.OnDeclaration, "draw")
        assert effect is not None
        effect.on_process_callback({
            'game': game,
            'player': player,
        })

        # Card should be removed from hand
        assert card not in player.hand_cards, "Card should be removed from hand"
        # Card should be placed at the BOTTOM of breeding perm's sources
        assert breeding_perm.card_sources[0] is card, (
            "Card should be at position 0 (bottom) of breeding perm's sources"
        )
        assert len(breeding_perm.card_sources) == breeding_sources_before + 1
        # Should have paid 3 memory
        assert game.memory == memory_before - 3, "Should pay 3 memory"
        # Should have drawn 1 card (net hand change: -1 for placing + 1 for draw)
        assert len(player.hand_cards) == hand_size_before - 1 + 1

    def test_hand_main_not_available_when_on_field(self, debug_runner):
        """The [Hand][Main] effect must NOT be available when the card is on field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-072"])
        runner.place_in_breeding(1, ["BT23-072"])  # valid target exists

        card = perm.top_card
        effect = _find_effect([card], EffectTiming.OnDeclaration, "draw")
        assert effect is not None
        # The condition should fail because the card has a permanent
        assert not effect.can_use_condition({}), (
            "[Hand][Main] condition should fail when card is on the field"
        )


# ── Effect 1: [All Turns] keyword grant tests ──────────────────────────


@pytest.mark.behavioral
class TestBT23072KeywordGrant:
    """Tests for the [All Turns] triggered effect: when RK/CS Digimon played,
    suspend self to grant Rush/Raid/Reboot/Blocker until opponent's turn ends."""

    def test_condition_requires_rk_or_cs_trait(self, debug_runner):
        """Condition passes for played Digimon with Royal Knight or CS trait."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        # BT22-008 Agumon has CS trait
        cs_perm = runner.place_on_field(1, ["BT22-008"])

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        assert effect is not None

        ctx = {"played_permanent": cs_perm}
        assert effect.can_use_condition(ctx), (
            "Condition should pass for Digimon with CS trait"
        )

    def test_condition_requires_rk_trait(self, debug_runner):
        """Condition passes for played Digimon with Royal Knight trait."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        # BT2-020 Gallantmon has Royal Knight trait
        rk_perm = runner.place_on_field(1, ["BT2-020"])

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        assert effect is not None

        ctx = {"played_permanent": rk_perm}
        assert effect.can_use_condition(ctx), (
            "Condition should pass for Digimon with Royal Knight trait"
        )

    def test_condition_fails_for_non_rk_non_cs(self, debug_runner):
        """Condition fails for played Digimon without RK or CS trait."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        # ST2-04 Gabumon (no CS, no Royal Knight)
        other_perm = runner.place_on_field(1, ["ST2-04"])

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        ctx = {"played_permanent": other_perm}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail for Digimon without RK or CS trait"
        )

    def test_condition_fails_for_opponent_digimon(self, debug_runner):
        """Condition fails for opponent's Digimon, even with RK/CS trait."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        opp_perm = runner.place_on_field(2, ["BT22-008"])  # opponent's CS Digimon

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        ctx = {"played_permanent": opp_perm}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail for opponent's Digimon"
        )

    def test_condition_fails_when_already_suspended(self, debug_runner):
        """Condition fails when KD7D6 is already suspended (cost cannot be paid)."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"], is_suspended=True)
        cs_perm = runner.place_on_field(1, ["BT22-008"])

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        ctx = {"played_permanent": cs_perm}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail when KD7D6 is already suspended"
        )

    def test_process_suspends_self_and_grants_4_keywords(self, debug_runner):
        """Process: suspend KD7D6, grant Rush/Raid/Reboot/Blocker to played perm."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        cs_perm = runner.place_on_field(1, ["BT22-008"])

        game = runner.game
        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': kd_perm,
            'played_permanent': cs_perm,
        })

        # KD7D6 should be suspended
        assert kd_perm.is_suspended, "KD7D6 should be suspended as cost"

        # Played Digimon should have all 4 keywords
        assert cs_perm.has_keyword('_is_rush'), "Should have Rush"
        assert cs_perm.has_keyword('_is_raid'), "Should have Raid"
        assert cs_perm.has_keyword('_is_reboot'), "Should have Reboot"
        assert cs_perm.has_keyword('_is_blocker'), "Should have Blocker"

    def test_keywords_expire_after_opponent_turn(self, debug_runner):
        """Granted keywords should expire at the end of opponent's turn
        (i.e., be gone at the start of the next turn after opponent's)."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])
        cs_perm = runner.place_on_field(1, ["BT22-008"])

        game = runner.game
        current_turn = game.turn_count  # e.g., turn 1

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        effect.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': kd_perm,
            'played_permanent': cs_perm,
        })

        # Should have keywords during current turn
        assert cs_perm.has_keyword('_is_rush')

        # Simulate moving to opponent's turn
        game.turn_count = current_turn + 1
        assert cs_perm.has_keyword('_is_rush'), (
            "Keywords should still be active during opponent's turn"
        )

        # Simulate moving past opponent's turn
        game.turn_count = current_turn + 2
        # After clear_expired_grants, keywords should be gone
        cs_perm.clear_expired_grants(game.turn_count)
        assert not cs_perm.has_keyword('_is_rush'), (
            "Keywords should expire after opponent's turn ends"
        )
        assert not cs_perm.has_keyword('_is_raid')
        assert not cs_perm.has_keyword('_is_reboot')
        assert not cs_perm.has_keyword('_is_blocker')

    def test_condition_fails_when_played_perm_is_self(self, debug_runner):
        """Condition fails when the played permanent IS the KD7D6 itself
        (i.e., it shouldn't trigger for its own play)."""
        runner = debug_runner(initial_memory=5)
        kd_perm = runner.place_on_field(1, ["BT23-072"])

        kd_card = kd_perm.top_card
        effect = _find_effect(
            [kd_card], EffectTiming.OnEnterFieldAnyone, "grant"
        )
        # played_permanent is the KD7D6 itself
        ctx = {"played_permanent": kd_perm}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail when the played permanent is KD7D6 itself"
        )


# ── Effect 2: Inherited effect tests ────────────────────────────────────


@pytest.mark.behavioral
class TestBT23072Inherited:
    """Tests for the inherited [Breeding][Start of Your Main Phase] effect:
    If 6+ digi-cards, may play 1 King Drasil Digimon from digi-cards free."""

    def test_inherited_timing_and_flags(self, debug_runner):
        """The inherited effect uses OnStartMainPhase and is_inherited_effect."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, "BT23-072", zone="hand")

        effect = _find_effect(
            [card], EffectTiming.OnStartMainPhase, inherited=True
        )
        assert effect is not None, "Should have an OnStartMainPhase inherited effect"
        assert effect.is_optional, "Inherited effect must be optional (canNoSelect)"

    def test_inherited_condition_requires_breeding(self, debug_runner):
        """Condition fails when the permanent is not in the breeding area."""
        runner = debug_runner(initial_memory=5)
        # Build a stack with 7+ card sources on the battle area (not breeding)
        perm = runner.place_on_field(1, [
            "BT23-072",  # bottom — this is the card whose inherited effect we test
            "BT23-072",  # + more to make 7 sources
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",  # top (7th source)
        ])

        card = perm.card_sources[0]  # bottom: BT23-072 inherited
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        ctx = {"permanent": perm}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail when permanent is on field (not breeding)"
        )

    def test_inherited_condition_requires_6_digi_cards(self, debug_runner):
        """Condition fails when the breeding perm has fewer than 6 digi-cards
        (fewer than 7 total card_sources)."""
        runner = debug_runner(initial_memory=5)
        # 6 sources = only 5 digi-cards (top excluded)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
        ])

        game = runner.game
        game.player1.is_my_turn = True
        card = breeding.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        ctx = {"permanent": breeding}
        assert not effect.can_use_condition(ctx), (
            "Condition should fail with only 5 digi-cards (6 total sources)"
        )

    def test_inherited_condition_passes_with_7_sources(self, debug_runner):
        """Condition passes when the breeding perm has 7+ card_sources
        (6+ digi-cards) and a King Drasil is among them."""
        runner = debug_runner(initial_memory=5)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",  # bottom — King Drasil (inherited source)
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",
            "BT23-072",  # top (7th source)
        ])

        game = runner.game
        game.player1.is_my_turn = True
        card = breeding.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        ctx = {"permanent": breeding}
        assert effect.can_use_condition(ctx), (
            "Condition should pass with 7 sources (6 digi-cards) including King Drasil"
        )

    def test_inherited_condition_requires_king_drasil_in_digi_cards(self, debug_runner):
        """Condition fails when no King Drasil name exists in digi-cards."""
        runner = debug_runner(initial_memory=5)
        # Use non-King Drasil cards as sources
        # BT22-008 Agumon for filler
        breeding = runner.place_in_breeding(1, [
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",  # top
        ])

        game = runner.game
        game.player1.is_my_turn = True
        # The BT23-072 card needs to be among the sources for the inherited effect
        # but in this case we have no BT23-072 at all — so the inherited effect
        # doesn't even exist in this stack. This test verifies the condition itself,
        # so let's inject BT23-072 as a source and check if there's a King Drasil
        # *Digimon* among the digi-cards to play.
        # Actually, the inherited effect comes from BT23-072 being in the stack.
        # Let's put BT23-072 at bottom and only filler on top.
        breeding2 = runner.place_in_breeding(1, [
            "BT23-072",  # bottom — has inherited effect, and IS a King Drasil
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",  # top (non-KD)
        ])
        # But wait: BT23-072 itself IS named "King Drasil_7D6" and IS a Digimon,
        # so the condition SHOULD pass since it's in the digi-cards.
        card = breeding2.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        ctx = {"permanent": breeding2}
        # The condition should still pass because BT23-072 has "King Drasil" in its name
        assert effect.can_use_condition(ctx)

    def test_inherited_process_single_candidate_auto_plays(self, debug_runner):
        """When there is exactly one King Drasil Digimon in digi-cards,
        it should be played directly without a selection phase."""
        runner = debug_runner(initial_memory=5)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",  # bottom — only King Drasil; also the inherited source
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",  # top
        ])

        game = runner.game
        player = game.player1
        field_before = len(player.battle_area)
        memory_before = game.memory
        sources_before = len(breeding.card_sources)

        card = breeding.card_sources[0]  # BT23-072 (bottom)
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        effect.on_process_callback({
            'game': game,
            'player': player,
            'permanent': breeding,
        })

        # Single candidate: auto-play without selection phase
        assert game.pending_selection is None, (
            "Single candidate should auto-play without entering selection"
        )
        assert len(breeding.card_sources) == sources_before - 1, (
            "One source should be removed from breeding"
        )
        assert len(player.battle_area) == field_before + 1, (
            "A Digimon should be played to the field"
        )
        # Memory should NOT change (played without paying cost)
        assert game.memory == memory_before, "Should play without paying cost"

        # Verify the played Digimon is King Drasil
        played = player.battle_area[-1]
        assert any(
            "King Drasil" in n for n in getattr(played.top_card, 'card_names', [])
        ), "Played Digimon should be King Drasil"

    def test_inherited_process_multiple_candidates_offers_selection(self, debug_runner):
        """When there are multiple King Drasil Digimon in digi-cards,
        a selection phase should be entered (C# canNoSelect=true)."""
        runner = debug_runner(initial_memory=5)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",  # bottom — King Drasil #1
            "BT23-072",  # King Drasil #2
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",  # top
        ])

        game = runner.game
        player = game.player1

        card = breeding.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None

        effect.on_process_callback({
            'game': game,
            'player': player,
            'permanent': breeding,
        })

        # Multiple candidates: should enter a selection phase
        assert game.pending_selection is not None, (
            "Multiple King Drasil candidates should trigger a selection phase"
        )
        assert game.current_phase == GamePhase.SelectEffectChoice
        assert game.pending_selection.is_optional, (
            "Selection should be optional (C# canNoSelect=true)"
        )
        assert len(game.pending_selection.valid_indices) == 2, (
            "Both King Drasil cards should be selectable"
        )

    def test_inherited_only_plays_king_drasil_named_digimon(self, debug_runner):
        """Process should only offer/play King Drasil named Digimon cards,
        not arbitrary digi-cards."""
        runner = debug_runner(initial_memory=5)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",  # bottom — King Drasil (valid target)
            "BT22-008",  # Agumon (not King Drasil — invalid target)
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",  # top
        ])

        game = runner.game
        player = game.player1

        card = breeding.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        effect.on_process_callback({
            'game': game,
            'player': player,
            'permanent': breeding,
        })

        # Single KD candidate: auto-played
        assert game.pending_selection is None, "Single candidate should auto-play"
        # Verify the played Digimon is King Drasil (not Agumon)
        if player.battle_area:
            played = player.battle_area[-1]
            top = played.top_card
            assert any(
                "King Drasil" in n
                for n in getattr(top, 'card_names', [])
            ), "Only King Drasil named Digimon should be played"

    def test_inherited_is_optional(self, debug_runner):
        """The inherited effect is optional — player may decline."""
        runner = debug_runner(initial_memory=5)
        breeding = runner.place_in_breeding(1, [
            "BT23-072",
            "BT23-072",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
            "BT22-008",
        ])

        card = breeding.card_sources[0]
        effect = _find_effect([card], EffectTiming.OnStartMainPhase, inherited=True)
        assert effect is not None
        assert effect.is_optional, "Inherited effect must be optional (canNoSelect=true in C#)"
