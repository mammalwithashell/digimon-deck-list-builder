"""Behavioral tests for BT22-046 Gargomon (Lv.4 Green/Yellow, Beastkin/CS).

Card text:
[When Digivolving] If you have 1 or fewer Tamers, from your hand and without
    paying the cost, you may play 1 Tamer card with [CS] trait from your hand
    without paying the cost.
Inherited: [All Turns] This Digimon gets +1000 DP.
Alt-digi: Lv.3 w/[CS] trait: Cost 2

C# ref: DCGO/Assets/Scripts/CardEffect/BT22/Green/BT22_046.cs
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# ST1-04 Dracomon as filler (Blue, unrelated) — avoids name / trait collisions
FILLER_DECK = ["ST1-04"] * 50


def _find_tamer_play_action(runner, tamer_card_id):
    """Find an action in the current selection phase that plays the given tamer
    from hand. Returns the action id, or None."""
    game = runner.game
    player = game.player1
    for h, card in enumerate(player.hand_cards):
        if card.c_entity_base and card.c_entity_base.card_id == tamer_card_id:
            aid = h  # SEL_HAND_START = 0
            mask = runner.action_mask()
            if aid in mask:
                return aid
    return None


@pytest.mark.behavioral
class TestBT22046AltDigi:
    """Effect 0: Alternate digivolution requirement — Lv.3 w/[CS] for cost 2.

    C# reference:
        targetPermanent.TopCard.IsLevel3 && targetPermanent.TopCard.HasCSTraits
        digivolutionCost: 2
    """

    def test_alt_digi_from_lv3_cs_trait(self, debug_runner):
        """BT22-046 can digivolve onto a Lv.3 with CS trait for cost 2.

        BT22-043 Terriermon is Green Lv.3 with [Beast, CS] traits — also
        matches the standard Green evo, so we also check cost handling.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.place_on_field(1, ["BT22-043"])  # Terriermon Lv.3 Green CS
        runner.inject_card(1, "BT22-046", zone="hand")

        actions = runner.find_actions("Digivolve")
        gargomon_digi = [a for a, d in actions.items() if "Gargomon" in d]
        assert gargomon_digi, (
            "BT22-046 should be able to digivolve onto Lv.3 with CS trait"
        )

    def test_alt_digi_rejects_lv3_without_cs_and_wrong_color(self, debug_runner):
        """BT22-046 should NOT digivolve onto a Lv.3 that is neither the
        right color nor has the CS trait.

        ST1-04 Dracomon is Lv.3 Blue (Dragon) — no CS, wrong color for Green
        standard evo path. Neither path matches.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.place_on_field(1, ["ST1-04"])  # Blue Lv.3, no CS
        runner.inject_card(1, "BT22-046", zone="hand")

        actions = runner.find_actions("Digivolve")
        gargomon_digi = [a for a, d in actions.items() if "Gargomon" in d]
        assert not gargomon_digi, (
            "BT22-046 should NOT digivolve onto Blue Lv.3 without CS trait"
        )


@pytest.mark.behavioral
class TestBT22046WhenDigivolving:
    """Effect 1: [When Digivolving] If ≤1 Tamers, optional play 1 CS Tamer from hand."""

    def test_when_digivolving_effect_is_registered(self, debug_runner):
        """Sanity: BT22-046 has an effect flagged is_when_digivolving=True
        and is_optional=True."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.inject_card(1, "BT22-046", "hand")
        bt22 = [
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-046"
        ][0]
        effects = bt22.effect_list(None)
        wd_effects = [
            e for e in effects
            if getattr(e, "is_when_digivolving", False)
        ]
        assert len(wd_effects) == 1, (
            f"Expected exactly 1 [When Digivolving] effect, got {len(wd_effects)}"
        )
        wd = wd_effects[0]
        assert wd.is_optional, "[When Digivolving] effect must be optional (‘you may’)"
        assert not wd.is_inherited_effect, (
            "[When Digivolving] effect must not be inherited (it’s on the main line)"
        )

    def test_when_digivolving_offers_cs_tamer_play_with_zero_tamers(self, debug_runner):
        """With 0 tamers on field and a CS Tamer in hand, firing WhenDigivolving
        should expose a selection (optional) letting the player play the CS Tamer."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Place a Terriermon (Lv.3 CS) with Gargomon on top — Gargomon is new top
        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])

        # Put a CS Tamer (BT22-089 Mirei Mikagura, cost 3) in hand
        runner.inject_card(1, "BT22-089", "hand")

        # Sanity: no tamers yet on field
        assert sum(1 for p in game.player1.battle_area if p.is_tamer) == 0

        # Fire WhenDigivolving on the Gargomon permanent
        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        # We should be in a selection phase since the effect_play_from_zone
        # exposed at least one valid CS Tamer in hand
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Expected SelectTarget phase for optional play; got {game.current_phase}"
        )

        # There must be an action that points at the CS Tamer in hand
        tamer_action = _find_tamer_play_action(runner, "BT22-089")
        assert tamer_action is not None, (
            f"Expected hand action to play BT22-089. Mask: {runner.action_mask()}"
        )

    def test_when_digivolving_plays_cs_tamer_from_hand_free(self, debug_runner):
        """Selecting the CS Tamer should move it from hand to field without
        paying its play cost."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        runner.inject_card(1, "BT22-089", "hand")  # Mirei Mikagura, CS Tamer, cost 3

        assert "BT22-089" in [c.c_entity_base.card_id for c in game.player1.hand_cards]
        memory_before = game.memory

        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        # Auto-pick: select the first hand action (the CS tamer)
        tamer_action = _find_tamer_play_action(runner, "BT22-089")
        assert tamer_action is not None
        runner.execute(tamer_action)
        runner.auto_resolve()

        # CS Tamer should be on field, no longer in hand
        assert "BT22-089" not in [
            c.c_entity_base.card_id for c in game.player1.hand_cards
        ], "CS Tamer should have left the hand"
        tamer_card_ids = [
            p.top_card.c_entity_base.card_id
            for p in game.player1.battle_area
            if p.is_tamer and p.top_card and p.top_card.c_entity_base
        ]
        assert "BT22-089" in tamer_card_ids, (
            "CS Tamer should have entered the battle area"
        )
        # Free play: memory must not be debited by the tamer's cost
        assert game.memory == memory_before, (
            f"Memory should not change (played free). before={memory_before} "
            f"after={game.memory}"
        )

    def test_when_digivolving_is_optional_player_can_decline(self, debug_runner):
        """Effect is ‘you may play’. The player must be able to decline
        (optional selection phase exposes a decline action)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        runner.inject_card(1, "BT22-089", "hand")  # CS Tamer available

        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        assert game.current_phase == GamePhase.SelectTarget, (
            "Should be in target selection for the optional play"
        )
        actions = runner.actions()
        # Action 0 is the decline/pass action
        has_decline = 0 in actions or any(
            ("decline" in d.lower() or "pass" in d.lower() or "no selection" in d.lower())
            for d in actions.values()
        )
        assert has_decline, (
            f"Optional play effect should expose a decline action. Actions: {actions}"
        )

    def test_when_digivolving_blocked_by_two_tamers(self, debug_runner):
        """With 2 Tamers already on field, the ‘1 or fewer Tamers’ gate
        should prevent the effect from firing — no selection phase."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        # Put 2 Tamers on field (BT22-088 Arisa + BT22-094 Yuugo Kamishiro)
        runner.place_on_field(1, ["BT22-088"])  # Tamer 1 (non-CS, but still tamer)
        runner.place_on_field(1, ["BT22-094"])  # Tamer 2 (CS)
        assert sum(1 for p in game.player1.battle_area if p.is_tamer) == 2

        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        runner.inject_card(1, "BT22-089", "hand")  # CS Tamer in hand

        phase_before = game.current_phase
        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        # Effect should NOT fire, so no selection phase change
        assert game.current_phase == phase_before, (
            f"With 2 tamers, effect should not fire. phase={game.current_phase}"
        )
        # The CS Tamer should still be in hand
        assert "BT22-089" in [
            c.c_entity_base.card_id for c in game.player1.hand_cards
        ], "CS Tamer should still be in hand (effect did not fire)"

    def test_when_digivolving_allowed_with_exactly_one_tamer(self, debug_runner):
        """Gate is ‘1 or fewer’, so exactly 1 tamer should still allow the effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        runner.place_on_field(1, ["BT22-088"])  # One Tamer (Arisa)
        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        runner.inject_card(1, "BT22-089", "hand")  # CS Tamer in hand

        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        assert game.current_phase == GamePhase.SelectTarget, (
            f"Effect should fire with exactly 1 tamer. phase={game.current_phase}"
        )

    def test_when_digivolving_filters_non_cs_tamers_out(self, debug_runner):
        """If the only tamers in hand are NOT CS trait, the effect cannot
        select them — no selection phase (no valid targets)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        # BT22-088 Arisa Kinosaki is a Tamer but has LIBERATOR trait, NOT CS
        runner.inject_card(1, "BT22-088", "hand")
        # Also put a non-Tamer (Option) in hand — should also be filtered out
        runner.inject_card(1, "BT1-090", "hand")  # Option card

        phase_before = game.current_phase
        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        # No valid CS-Tamer target, so effect_play_from_zone returns early
        assert game.current_phase == phase_before, (
            f"Without a CS-Tamer target, no selection phase should start. "
            f"phase={game.current_phase}"
        )
        # The non-CS Tamer should still be in hand
        assert "BT22-088" in [
            c.c_entity_base.card_id for c in game.player1.hand_cards
        ], "Non-CS Tamer should not have been played"

    def test_when_digivolving_does_not_play_non_tamer_cards(self, debug_runner):
        """Even if a CS Digimon is in hand, the effect must only offer Tamers."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        # BT22-008 Agumon is a Lv.3 CS Digimon — must NOT be playable here
        runner.inject_card(1, "BT22-008", "hand")

        phase_before = game.current_phase
        game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": game.player1,
        })

        assert game.current_phase == phase_before, (
            "A CS Digimon in hand must NOT be a valid target (effect is Tamer-only)"
        )


@pytest.mark.behavioral
class TestBT22046InheritedDP:
    """Effect 2: Inherited [All Turns] This Digimon gets +1000 DP.

    Standard below-the-line self-buff: only active when BT22-046 is under
    the top card of a stack (not when Gargomon itself is the top).
    """

    def test_inherited_effect_is_registered(self, debug_runner):
        """Sanity: BT22-046 has an inherited dp_modifier=1000 effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        runner.inject_card(1, "BT22-046", "hand")
        bt22 = [
            c for c in runner.game.player1.hand_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-046"
        ][0]
        effects = bt22.effect_list(None)
        inh = [
            e for e in effects
            if e.is_inherited_effect and e.dp_modifier == 1000
        ]
        assert len(inh) == 1, (
            f"Expected 1 inherited +1000 DP effect, got {len(inh)}"
        )

    def test_inherited_dp_active_when_under_top_card(self, debug_runner):
        """When BT22-046 is under another Digimon on top, the top permanent
        gains +1000 DP from Gargomon's inherited effect."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Stack: Lv.3 -> BT22-046 -> something on top.
        # Use a simple Lv.5 Digimon on top whose base DP we know.
        # BT15-088 or similar? Easier: stack bt22-046 under a known Digimon.
        # Use ST1-07 Greymon (Lv.4, Red, base DP 5000) — Gargomon only acts
        # as inherited source below it.
        perm = runner.place_on_field(1, ["BT22-043", "BT22-046", "ST1-07"])
        assert perm.top_card.c_entity_base.card_id == "ST1-07"
        base_dp = perm.top_card.c_entity_base.dp
        # Should gain +1000 from the inherited effect
        assert perm.dp == base_dp + 1000, (
            f"Top card ({perm.top_card.c_entity_base.card_name_eng}) DP should be "
            f"base({base_dp}) + 1000 = {base_dp + 1000}, got {perm.dp}"
        )

    def test_inherited_dp_not_active_when_gargomon_is_top_card(self, debug_runner):
        """When BT22-046 itself is the top card, the inherited effect is
        below its own line and does NOT apply (standard TCG rule)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT22-043", "BT22-046"])
        assert perm.top_card.c_entity_base.card_id == "BT22-046"
        base_dp = perm.top_card.c_entity_base.dp  # 6000
        assert perm.dp == base_dp, (
            f"Gargomon as top card should NOT get its own inherited +1000 DP. "
            f"expected base={base_dp}, got {perm.dp}"
        )

    def test_inherited_dp_does_not_leak_to_other_permanents(self, debug_runner):
        """The inherited +1000 is a self-buff (‘This Digimon’), not an aura.
        It must NOT boost other friendly permanents."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        # Stack: Gargomon under ST1-07 (should be boosted)
        boosted = runner.place_on_field(1, ["BT22-043", "BT22-046", "ST1-07"])
        # Separate permanent: BT22-043 alone (should NOT be boosted)
        other = runner.place_on_field(1, ["BT22-043"])

        base_boosted = boosted.top_card.c_entity_base.dp
        base_other = other.top_card.c_entity_base.dp

        assert boosted.dp == base_boosted + 1000, (
            f"Host should be +1000, got {boosted.dp}"
        )
        assert other.dp == base_other, (
            f"Other permanent should NOT be boosted. expected {base_other}, "
            f"got {other.dp}"
        )
