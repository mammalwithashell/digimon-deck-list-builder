"""Behavioral tests for BT23-102 Mastemon (Digimon, Yellow/Purple, Lv.6, DP 13000).

Card text (C# source of truth):
<Barrier>
<Partition ([Angewomon] & [LadyDevimon])>
[When Digivolving] You may play 1 level 5 or lower yellow or purple card from
your hand or trash without paying the cost. Then, if this Digimon's stack has
2 or more same-level cards, trash the top cards of both players' security
stacks so that they have 3 cards left.
[All Turns] [Once Per Turn] When security stacks are removed from, you may
place 1 Digimon as the bottom security card.

Alt-digivolve (xros_req): [Digivolve] Lv.5 w/[CS] trait: Cost 5
DNA-digivolve (xros_req): [DNA Digivolve] Yellow Lv.5 + Purple Lv.5: Cost 0

Key cards used:
- BT23-102: Mastemon (Lv.6 yellow/purple, Angel/CS)
- BT23-031: Angewomon (Lv.5 yellow, Archangel/CS) — alt-digi source
- BT23-067: LadyDevimon (Lv.5 purple, Fallen Angel/CS) — alt-digi + DNA source
- BT1-060: MagnaAngemon (Lv.5 yellow, no CS) — valid play target
- BT2-075: Myotismon (Lv.5 purple) — valid play target
- BT1-048: Patamon (Lv.3 yellow) — valid play target, digivolution source material
- BT2-067: DemiDevimon (Lv.3 purple) — digivolution source material
- ST1-03: Agumon (Lv.3 red) — NON-yellow/purple, should NOT be playable
- ST1-08: MetalGreymon (Lv.5 red) — NON-yellow/purple target
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, CardColor


MASTEMON_DECK = ["BT23-102"] * 4 + ["BT23-031"] * 10 + ["BT23-067"] * 10 + ["BT1-048"] * 26


@pytest.mark.behavioral
class TestBT23_102Mastemon:
    """Tests for BT23-102 Mastemon."""

    # ─────────────────────────────────────────────────────────────────
    # Barrier keyword
    # ─────────────────────────────────────────────────────────────────

    def test_has_barrier_flag(self, debug_runner):
        """Mastemon should have _is_barrier flag."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-102"])
        assert perm.has_keyword("_is_barrier"), \
            "BT23-102 should have the Barrier keyword"

    def test_barrier_effect_registered(self, debug_runner):
        """Mastemon should register an ICardEffect with _is_barrier = True."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-102"])
        top = perm.top_card
        effects = top.effect_list(None)
        barrier_effects = [e for e in effects if getattr(e, '_is_barrier', False)]
        assert len(barrier_effects) >= 1, "Should have at least 1 barrier effect"

    # ─────────────────────────────────────────────────────────────────
    # Partition flag
    # ─────────────────────────────────────────────────────────────────

    def test_has_partition_flag(self, debug_runner):
        """Mastemon should have a partition ICardEffect marker."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-102"])
        top = perm.top_card
        effects = top.effect_list(None)
        partition_effects = [e for e in effects if getattr(e, '_is_partition', False)]
        assert len(partition_effects) >= 1, \
            "BT23-102 should have at least 1 partition effect"

    # ─────────────────────────────────────────────────────────────────
    # Alt-Digivolve: Lv.5 w/[CS] trait for cost 5
    # ─────────────────────────────────────────────────────────────────

    def test_alt_digi_matches_cs_lv5(self, debug_runner):
        """Alt-digi requirement should match any Lv.5 Digimon with CS trait."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=15)
        from digimon_gym.engine.validation.digivolve_validator import (
            _check_alt_digivolve, get_alt_digi_cost,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Place a Lv.5 w/CS trait Angewomon on the field
        base_perm = runner.place_on_field(1, ["BT23-031"])  # Angewomon CS
        mastemon_card = db.create_card_source("BT23-102", runner.game.player1)

        assert _check_alt_digivolve(mastemon_card, base_perm), \
            "Mastemon alt-digi should match Lv.5 CS Digimon (Angewomon)"
        assert get_alt_digi_cost(mastemon_card, base_perm) == 5, \
            "Alt-digi cost should be 5"

    def test_alt_digi_matches_cs_lv5_purple(self, debug_runner):
        """Alt-digi should match a purple CS Lv.5 (LadyDevimon)."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=15)
        from digimon_gym.engine.validation.digivolve_validator import (
            _check_alt_digivolve, get_alt_digi_cost,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        base_perm = runner.place_on_field(1, ["BT23-067"])  # LadyDevimon CS
        mastemon_card = db.create_card_source("BT23-102", runner.game.player1)

        assert _check_alt_digivolve(mastemon_card, base_perm), \
            "Mastemon alt-digi should match Lv.5 CS Digimon (LadyDevimon)"
        assert get_alt_digi_cost(mastemon_card, base_perm) == 5, \
            "Alt-digi cost should be 5"

    def test_alt_digi_rejects_lv5_without_cs(self, debug_runner):
        """Alt-digi should NOT match a Lv.5 Yellow Digimon without CS trait."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=15)
        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # BT1-060 MagnaAngemon: Lv.5 yellow, no CS trait
        base_perm = runner.place_on_field(1, ["BT1-060"])
        mastemon_card = db.create_card_source("BT23-102", runner.game.player1)

        assert not _check_alt_digivolve(mastemon_card, base_perm), \
            "Mastemon alt-digi should NOT match a Lv.5 Yellow Digimon lacking CS"

    def test_alt_digi_rejects_wrong_level(self, debug_runner):
        """Alt-digi should NOT match a Lv.4 card even with CS trait."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=15)
        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # BT23-077 Sistermon Ciel is Lv.4, but this won't have CS either — just use it
        # for level mismatch; if it had CS, the level check still fails.
        base_perm = runner.place_on_field(1, ["BT23-077"])
        mastemon_card = db.create_card_source("BT23-102", runner.game.player1)

        assert not _check_alt_digivolve(mastemon_card, base_perm), \
            "Mastemon alt-digi should require Lv.5, not Lv.4"

    # ─────────────────────────────────────────────────────────────────
    # DNA Digivolve: Yellow Lv.5 + Purple Lv.5 cost 0 (engine-parsed)
    # ─────────────────────────────────────────────────────────────────

    def test_dna_digivolve_requirements_parsed(self, debug_runner):
        """DNA digivolve requirements should be parsed from xros_req."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-102", runner.game.player1)

        assert card.c_entity_base is not None
        dna_costs = card.c_entity_base.dna_costs
        assert len(dna_costs) == 1, "Should have exactly 1 DNA cost"
        dc = dna_costs[0]
        assert dc.memory_cost == 0, "DNA digivolve should cost 0"
        colors = set(dc.requirement1.card_colors) | set(dc.requirement2.card_colors)
        assert CardColor.Yellow in colors, "One requirement should be Yellow"
        assert CardColor.Purple in colors, "One requirement should be Purple"
        assert dc.requirement1.level == 5 and dc.requirement2.level == 5, \
            "Both DNA requirements should be Lv.5"

    def test_dna_digivolve_matches_angewomon_plus_ladydevimon(self, debug_runner):
        """DNA digivolve should accept Angewomon (Yellow Lv.5) + LadyDevimon (Purple Lv.5)."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.validation.digivolve_validator import can_dna_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        ange_perm = runner.place_on_field(1, ["BT23-031"])  # Yellow Lv.5
        lady_perm = runner.place_on_field(1, ["BT23-067"])  # Purple Lv.5
        mastemon_card = db.create_card_source("BT23-102", runner.game.player1)

        assert can_dna_digivolve(mastemon_card, ange_perm, lady_perm), \
            "Mastemon should DNA-digivolve from Angewomon + LadyDevimon"

    # ─────────────────────────────────────────────────────────────────
    # [When Digivolving] effect existence
    # ─────────────────────────────────────────────────────────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have an OnEnterFieldAnyone effect with is_when_digivolving."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-102", runner.game.player1)
        effects = card.effect_list(None)

        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)]
        assert len(when_digi) >= 1, "Should have 1 When Digivolving effect"

    # ─────────────────────────────────────────────────────────────────
    # [When Digivolving] Part 1: play 1 Lv.5-or-lower yellow/purple card
    # ─────────────────────────────────────────────────────────────────

    def test_when_digivolving_offers_yellow_lv5_from_hand(self, debug_runner):
        """When Digivolving should offer to play a Yellow Lv.5 from hand."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        # Place Mastemon on field
        perm = runner.place_on_field(1, ["BT23-102"])
        # Clear hand and inject a Yellow Lv.5 target + a non-valid target
        game.player1.hand_cards.clear()
        runner.inject_card(1, "BT1-060", "hand")  # Yellow Lv.5 MagnaAngemon — valid
        runner.inject_card(1, "ST1-08", "hand")   # Red Lv.5 — invalid (wrong color)

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        # Fire the process callback — should open a selection phase
        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Pending selection should be present with valid hand indices
        assert game.pending_selection is not None, \
            "When Digivolving should open a selection phase"

    def test_when_digivolving_rejects_non_yellow_purple(self, debug_runner):
        """When Digivolving should NOT offer Red/Blue/Green/Black cards."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT23-102"])
        game.player1.hand_cards.clear()
        # Only non-yellow/purple cards in hand
        runner.inject_card(1, "ST1-03", "hand")  # Red Lv.3
        runner.inject_card(1, "ST1-08", "hand")  # Red Lv.5

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # No valid targets → no selection opened
        assert game.pending_selection is None, \
            "When Digivolving should not open selection when no valid yellow/purple targets exist"

    def test_when_digivolving_rejects_level_6_plus(self, debug_runner):
        """When Digivolving filter should exclude Lv.6+ cards."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT23-102"])
        game.player1.hand_cards.clear()
        # Only a Lv.6 yellow/purple card (BT23-102 Mastemon itself is Lv.6)
        runner.inject_card(1, "BT23-102", "hand")

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert game.pending_selection is None, \
            "When Digivolving should filter out Lv.6+ cards"

    def test_when_digivolving_allows_trash_source(self, debug_runner):
        """When Digivolving should allow selecting from trash too."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT23-102"])
        game.player1.hand_cards.clear()
        game.player1.trash_cards.clear()

        runner.inject_card(1, "BT1-060", "trash")  # Yellow Lv.5 — valid target in trash

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert game.pending_selection is not None, \
            "When Digivolving should open selection for trash-only targets"

    def test_when_digivolving_is_optional_no_valid_target(self, debug_runner):
        """When Digivolving 'you may play' — with no target, effect should be no-op."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT23-102"])
        game.player1.hand_cards.clear()
        game.player1.trash_cards.clear()

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        # Should not raise
        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        # No selection either (no targets)
        assert game.pending_selection is None

    # ─────────────────────────────────────────────────────────────────
    # [When Digivolving] Part 2: trash securities to 3 if stack has 2+
    # same-level cards
    # ─────────────────────────────────────────────────────────────────

    def test_when_digivolving_trashes_both_securities_to_3_when_stack_has_2_same_level(self, debug_runner):
        """If Mastemon's stack has 2+ same-level cards, trash securities to 3."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        # Build a stack with 2 cards of the same level (both Lv.5: Ange+Lady) under Mastemon
        perm = runner.place_on_field(1, ["BT23-031", "BT23-067", "BT23-102"])
        # Sanity: 3 cards in stack, two are Lv.5
        levels = [cs.level for cs in perm.card_sources]
        lv5_count = sum(1 for lv in levels if lv == 5)
        assert lv5_count >= 2, f"Setup failure: need 2+ Lv.5 cards in stack, got {levels}"

        # Ensure empty hand/trash so only Part-2 runs
        game.player1.hand_cards.clear()
        game.player1.trash_cards.clear()

        # Give both players 5 security cards
        game.player1.security_cards.clear()
        game.player2.security_cards.clear()
        for _ in range(5):
            runner.inject_card(1, "BT1-048", "security_top")
            runner.inject_card(2, "BT1-048", "security_top")
        assert len(game.player1.security_cards) == 5
        assert len(game.player2.security_cards) == 5

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.security_cards) == 3, (
            f"P1 security should be trimmed to 3, got {len(game.player1.security_cards)}"
        )
        assert len(game.player2.security_cards) == 3, (
            f"P2 security should be trimmed to 3, got {len(game.player2.security_cards)}"
        )

    def test_when_digivolving_does_not_trash_if_stack_lacks_duplicates(self, debug_runner):
        """If Mastemon's stack has NO 2+ same-level cards, securities are untouched."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        # Stack with distinct levels (Lv.3, Lv.5, Lv.6): Patamon + MagnaAngemon + Mastemon
        perm = runner.place_on_field(1, ["BT1-048", "BT1-060", "BT23-102"])
        levels = [cs.level for cs in perm.card_sources]
        # No duplicate level (3/5/6)
        assert len(set(levels)) == len(levels), f"Setup failure: duplicate level in {levels}"

        game.player1.hand_cards.clear()
        game.player1.trash_cards.clear()

        game.player1.security_cards.clear()
        game.player2.security_cards.clear()
        for _ in range(5):
            runner.inject_card(1, "BT1-048", "security_top")
            runner.inject_card(2, "BT1-048", "security_top")

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.security_cards) == 5, \
            "P1 security should be unchanged when stack has no same-level duplicates"
        assert len(game.player2.security_cards) == 5, \
            "P2 security should be unchanged when stack has no same-level duplicates"

    def test_when_digivolving_leaves_securities_alone_if_below_3(self, debug_runner):
        """When securities are already below 3, they should remain untouched."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        perm = runner.place_on_field(1, ["BT23-031", "BT23-067", "BT23-102"])

        game.player1.hand_cards.clear()
        game.player1.trash_cards.clear()

        game.player1.security_cards.clear()
        game.player2.security_cards.clear()
        for _ in range(2):
            runner.inject_card(1, "BT1-048", "security_top")
        for _ in range(3):
            runner.inject_card(2, "BT1-048", "security_top")

        effects = perm.top_card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone
                     and getattr(e, 'is_when_digivolving', False)][0]

        when_digi.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player1.security_cards) == 2, \
            "P1 security at 2 should stay at 2 (below threshold of 3)"
        assert len(game.player2.security_cards) == 3, \
            "P2 security at 3 should stay at 3"

    # ─────────────────────────────────────────────────────────────────
    # [All Turns] [Once Per Turn] OnLoseSecurity — place Digimon as bottom security
    # ─────────────────────────────────────────────────────────────────

    def test_on_lose_security_effect_exists(self, debug_runner):
        """Should have an OnLoseSecurity OPT effect."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-102", runner.game.player1)
        effects = card.effect_list(None)

        sec_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(sec_effects) >= 1, "Should have 1 OnLoseSecurity effect"
        assert sec_effects[0].max_count_per_turn == 1, \
            "OnLoseSecurity effect should be once per turn"

    def test_on_lose_security_places_digimon_as_bottom_security(self, debug_runner):
        """The selected Digimon should move to the BOTTOM of the card owner's security stack."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        mastemon_perm = runner.place_on_field(1, ["BT23-102"])
        # Place a Digimon that will be placed into security
        target_perm = runner.place_on_field(1, ["BT23-031"])  # Angewomon

        # Setup securities — track top vs bottom by remembering IDs
        game.player1.security_cards.clear()
        top_marker = runner.inject_card(1, "BT1-048", "security_bottom")
        # Add another to have distinct top/bottom
        runner.inject_card(1, "BT1-048", "security_top")

        sec_before = len(game.player1.security_cards)

        effects = mastemon_perm.top_card.effect_list(None)
        opt_effect = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        opt_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': mastemon_perm,
        })

        # Selection phase — pick Angewomon
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        target_idx = game.player1.battle_area.index(target_perm)
        runner.execute(SEL_MY_FIELD_START + target_idx)

        # Target should have left battle area
        assert target_perm not in game.player1.battle_area, \
            "Target Digimon should have left battle area"
        # Security count should increase by 1
        assert len(game.player1.security_cards) == sec_before + 1
        # The target's top card should be at the BOTTOM of security (index -1)
        assert game.player1.security_cards[-1].c_entity_base.card_id == "BT23-031", (
            f"Target should be at BOTTOM of security, got "
            f"{game.player1.security_cards[-1].c_entity_base.card_id}"
        )

    def test_on_lose_security_only_picks_digimon(self, debug_runner):
        """Selection should filter to Digimon only (not Tamers/Options)."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        game = runner.game

        mastemon_perm = runner.place_on_field(1, ["BT23-102"])
        # Place both a Digimon and a Tamer
        digi_perm = runner.place_on_field(1, ["BT23-031"])  # Angewomon digimon
        tamer_perm = runner.place_on_field(1, ["BT23-080"])  # Yu Nogi tamer

        effects = mastemon_perm.top_card.effect_list(None)
        opt_effect = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity][0]

        opt_effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': mastemon_perm,
        })

        # Both Mastemon (Digimon) and digi_perm should be valid; tamer should not
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        pending = game.pending_selection
        assert pending is not None, "Should have a pending selection"
        valid_ids = set(pending.valid_indices)
        tamer_idx = game.player1.battle_area.index(tamer_perm)
        assert (SEL_MY_FIELD_START + tamer_idx) not in valid_ids, (
            "Tamer should NOT be a valid selection target"
        )
        digi_idx = game.player1.battle_area.index(digi_perm)
        assert (SEL_MY_FIELD_START + digi_idx) in valid_ids, (
            "Own Digimon should be a valid selection target"
        )

    def test_on_lose_security_shared_hash_for_opt(self, debug_runner):
        """The OPT effect should set a unique hash string for counter tracking."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-102", runner.game.player1)
        effects = card.effect_list(None)

        opt_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert len(opt_effects) >= 1
        opt = opt_effects[0]
        assert opt.hash_string, "OPT effect should have a non-empty hash_string"

    def test_on_lose_security_is_optional(self, debug_runner):
        """The effect should be marked optional (you MAY place)."""
        runner = debug_runner(deck1=MASTEMON_DECK, deck2=MASTEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-102", runner.game.player1)
        effects = card.effect_list(None)

        opt_effects = [e for e in effects if e.timing == EffectTiming.OnLoseSecurity]
        assert opt_effects[0].is_optional, \
            "OnLoseSecurity effect should be optional (you MAY place)"
