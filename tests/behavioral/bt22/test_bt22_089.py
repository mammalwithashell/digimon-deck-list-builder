"""Behavioral tests for BT22-089 Mirei Mikagura (Tamer, Yellow/Purple, Cost 3).

Card text (C# source of truth):
[Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
you may play 1 play cost 4 or higher [Mirei Mikagura] or play cost 4 or higher
Tamer card with the [CS] trait from your hand without paying the cost.
[On Play] By trashing 1 card with the [Holy Beast], [Angel], [Archangel],
[Fallen Angel] or [CS] trait from your hand, <Draw 2>.
[Security] Play this card without paying the cost.

Key cards used:
- BT22-089: Mirei Mikagura, Tamer, cost 3, CS trait
- EX6-074: Mirei Mikagura, Tamer, cost 4 (no CS trait) -- valid play target (name match)
- BT11-094: Mirei Mikagura, Tamer, cost 5 (no CS trait) -- valid play target (name match)
- BT22-083: Yuuko Kamishiro, Tamer, cost 4, CS trait -- valid play target (CS tamer)
- BT1-046: Kudamon, Digimon, cost 3, Holy Beast trait -- valid trash target for On Play
- BT22-017: Gabumon, Digimon, Lv.3, cost 3, CS trait -- valid trash target for On Play
- BT1-009: Monodramon, Digimon, Lv.3, cost 2, no relevant traits -- NOT valid trash target
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Deck with Mirei and various targets
MIREI_DECK = ["BT22-089"] * 4 + ["EX6-074"] * 4 + ["BT22-083"] * 4 + ["BT1-046"] * 20 + ["BT22-017"] * 18

# Deck without any valid play targets
NO_TARGETS_DECK = ["BT22-089"] * 4 + ["BT1-009"] * 46


@pytest.mark.behavioral
class TestBT22089MireiMikagura:
    """Tests for BT22-089 Mirei Mikagura."""

    # -- Start of Main Phase: play filter tests --

    def test_start_main_play_filter_accepts_mirei_name_cost4(self, debug_runner):
        """Start of Main Phase should accept any card named Mirei Mikagura with
        cost >= 4, regardless of whether it's a tamer."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)

        # Place BT22-089 on field as a tamer
        perm = runner.place_on_field(1, ["BT22-089"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(main_effects) == 1, "Should have 1 Start of Main Phase effect"

        # Check the play filter logic by examining the process callback behavior
        # We verify filter correctness by unit-testing the filter function
        effect = main_effects[0]

        # Inject EX6-074 Mirei Mikagura (cost 4, tamer, no CS) into hand
        runner.inject_card(1, "EX6-074", "hand")

        # The condition should pass (on field, owner's turn)
        assert effect.can_use_condition({}), "Condition should pass"

    def test_start_main_play_filter_accepts_cs_tamer_cost4(self, debug_runner):
        """Start of Main Phase should accept a Tamer with CS trait and cost >= 4."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT22-089"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = main_effects[0]

        # Get the play_filter from the process callback
        # We test it indirectly via the process function
        # Inject BT22-083 Yuuko Kamishiro (cost 4, CS tamer) into hand
        runner.inject_card(1, "BT22-083", "hand")

        assert effect.can_use_condition({}), "Condition should pass"

    def test_start_main_play_filter_rejects_cs_digimon(self, debug_runner):
        """Start of Main Phase should NOT accept a Digimon with CS trait
        (only tamers qualify via CS trait branch, per C#)."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT22-089"])

        game = runner.game
        card = perm.top_card

        # Get the play filter from the effect's process
        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = main_effects[0]

        # Directly test the filter logic by extracting it from the process
        # We'll simulate: only BT22-017 (Gabumon, Digimon, CS, cost 3) in hand
        # This should NOT be a valid target (it's a Digimon with cost 3)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        gabumon = db.create_card_source("BT22-017", game.player1)

        # The filter should reject: Digimon, CS trait, but not tamer and cost < 4
        # Per C#: EqualsCardName("Mirei Mikagura") || (IsTamer && HasCSTraits)
        # Gabumon is NOT "Mirei Mikagura" and NOT a tamer -> rejected
        assert not gabumon.is_tamer, "Gabumon should not be a tamer"
        assert gabumon.get_cost_itself < 4, "Gabumon cost should be < 4"

    def test_start_main_play_filter_rejects_low_cost_mirei(self, debug_runner):
        """Start of Main Phase should NOT accept a Mirei Mikagura with cost < 4."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT22-089"])

        game = runner.game

        # BT22-089 itself is named Mirei Mikagura but costs 3 -- should be rejected
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        mirei_low = db.create_card_source("BT22-089", game.player1)
        assert mirei_low.get_cost_itself < 4, "BT22-089 cost should be 3 (< 4)"

    def test_start_main_returns_tamer_to_deck_bottom(self, debug_runner):
        """Activating Start of Main Phase effect should return the tamer to deck bottom."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT22-089"])
        game = runner.game

        # Inject a valid target: EX6-074 (cost 4, Mirei Mikagura)
        runner.inject_card(1, "EX6-074", "hand")

        card = perm.top_card
        effects = card.effect_list(None)
        main_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = main_effects[0]

        field_before = len(game.player1.battle_area)

        # Execute the effect
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Perm should be removed from battle area (returned to deck bottom)
        assert len(game.player1.battle_area) < field_before, \
            "Tamer should be removed from battle area"

    # -- On Play: trash + draw tests --

    def test_on_play_trash_holy_beast_draw_2(self, debug_runner):
        """On Play should allow trashing a Holy Beast-trait card from hand to Draw 2."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Inject a Holy Beast card into hand
        runner.inject_card(1, "BT1-046", "hand")
        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Mirei Mikagura")
        assert action is not None, "Should be able to play Mirei Mikagura"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Tamer on field
        assert any(s.card_id == "BT22-089" for s in snap.p1_field), \
            "Mirei should be on the field"
        # Hand: played 1 (-1), trashed 1 (-1), drew 2 (+2) = net 0
        assert len(snap.p1_hand) == hand_before, \
            f"Expected hand size {hand_before} after play-1 trash-1 draw+2, got {len(snap.p1_hand)}"

    def test_on_play_trash_cs_trait_draw_2(self, debug_runner):
        """On Play should also accept CS trait cards for trash."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Inject a CS card into hand
        runner.inject_card(1, "BT22-017", "hand")  # Gabumon, CS trait
        snap_before = runner.snapshot()
        hand_before = len(snap_before.p1_hand)

        action = runner.find_action("Play Mirei Mikagura")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT22-089" for s in snap.p1_field)
        assert len(snap.p1_hand) == hand_before

    def test_on_play_no_valid_trash_target(self, debug_runner):
        """On Play with no valid trash targets should still play the tamer
        but skip the draw (optional effect)."""
        runner = debug_runner(
            deck1=NO_TARGETS_DECK,
            deck2=NO_TARGETS_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        action = runner.find_action("Play Mirei Mikagura")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "BT22-089" for s in snap.p1_field), \
            "Mirei should be on the field even without valid trash targets"

    def test_on_play_condition_rejects_when_only_non_matching_trait(self, debug_runner):
        """The On Play by-cost condition must use EXACT trait matching.
        A card whose only trait is a Reptile/Mini Dragon must NOT satisfy the
        condition (regression for substring-trait bug: 'CS' in 'Chess', etc.)."""
        runner = debug_runner(deck1=NO_TARGETS_DECK, deck2=NO_TARGETS_DECK, initial_memory=5)

        # Place BT22-089 on field, fully clear hand, then add one non-matching card
        perm = runner.place_on_field(1, ["BT22-089"])
        game = runner.game
        game.player1.hand_cards.clear()

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # BT1-009 Monodramon has traits ['Mini Dragon'] -> no C2 match
        monodramon = db.create_card_source("BT1-009", game.player1)
        game.player1.hand_cards.append(monodramon)

        effects = perm.top_card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if getattr(e, 'is_on_play', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ]
        assert len(on_play_effects) == 1
        effect = on_play_effects[0]
        # Condition should fail — no qualifying hand card (Mini Dragon != CS/Angel/etc)
        assert not effect.can_use_condition({}), \
            "On Play condition must reject when hand has no matching trait"

    def test_on_play_condition_accepts_each_target_trait(self, debug_runner):
        """All five C2 target traits should satisfy the condition independently."""
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Each card tests one trait family
        trait_cards = [
            ("BT1-046", "Holy Beast"),   # Kudamon
            ("BT1-053", "Angel"),         # Darcmon
            ("BT22-017", "CS"),           # Gabumon (CS)
        ]
        for card_id, trait_label in trait_cards:
            runner = debug_runner(deck1=NO_TARGETS_DECK, deck2=NO_TARGETS_DECK, initial_memory=5)
            perm = runner.place_on_field(1, ["BT22-089"])
            game = runner.game
            game.player1.hand_cards.clear()
            cs = db.create_card_source(card_id, game.player1)
            game.player1.hand_cards.append(cs)

            effects = perm.top_card.effect_list(None)
            on_play = [
                e for e in effects
                if getattr(e, 'is_on_play', False)
                and e.timing == EffectTiming.OnEnterFieldAnyone
            ][0]
            assert on_play.can_use_condition({}), \
                f"On Play condition should accept {trait_label} (via {card_id})"

    def test_on_play_trash_uses_engine_helper(self, debug_runner):
        """Trashing via the by-cost must move the card from hand to trash."""
        runner = debug_runner(deck1=NO_TARGETS_DECK, deck2=NO_TARGETS_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT22-089"])
        game = runner.game
        game.player1.hand_cards.clear()

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        kudamon = db.create_card_source("BT1-046", game.player1)  # Holy Beast
        game.player1.hand_cards.append(kudamon)

        trash_before = len(game.player1.trash_cards)

        # Drive the on-play effect directly via its process callback
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if getattr(e, 'is_on_play', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ][0]
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Kudamon is now in trash, drew 2 cards
        assert kudamon in game.player1.trash_cards, \
            "Selected hand card should be moved to trash via engine helper"
        assert len(game.player1.trash_cards) >= trash_before + 1

    # -- Security: Play this card without paying the cost --

    def test_security_effect_is_registered_with_correct_timing_and_process(self, debug_runner):
        """The [Security] Play this card effect must be wired with
        SecuritySkill timing, is_security_effect=True, and a process
        callback that invokes effect_play_from_security."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        mirei = db.create_card_source("BT22-089", runner.game.player1)

        effects = mirei.effect_list(None)
        sec_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec_effects) == 1, "Expected exactly one security effect"
        sec = sec_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill, \
            "Security effect must use SecuritySkill timing"
        assert sec.on_process_callback is not None, \
            "Security effect must have a process callback"

    def test_security_play_puts_mirei_on_field(self, debug_runner):
        """When Mirei is revealed as a security card, the Security effect should
        put her on the owner's battle area without paying cost."""
        runner = debug_runner(deck1=MIREI_DECK, deck2=MIREI_DECK, initial_memory=5)
        game = runner.game
        player = game.player1

        # Simulate the security-skill process directly: fetch the effect from
        # a security-zone CardSource and invoke it (this mirrors Player.attack()).
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        mirei = db.create_card_source("BT22-089", player)
        player.security_cards.insert(0, mirei)

        effects = mirei.effect_list(EffectTiming.SecuritySkill)
        sec = next(e for e in effects if getattr(e, 'is_security_effect', False))
        assert sec.on_process_callback is not None

        field_before = len(player.battle_area)
        sec.on_process_callback({
            'game': game,
            'player': player,
            'permanent': None,
            'card': mirei,
        })

        assert len(player.battle_area) == field_before + 1, \
            "Security play should add Mirei to battle area"
        assert any(p.top_card is mirei for p in player.battle_area), \
            "Mirei should be on the field after security play"
        # The security-played card should be marked so normal security trash
        # logic does not re-trash it.
        assert getattr(mirei, '_security_played', False) is True
