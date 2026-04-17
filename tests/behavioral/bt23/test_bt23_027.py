"""Behavioral tests for BT23-027 Angemon (Digimon, Yellow/Blue, Lv.4, Angel/Hudie/CS).

Card text (C# source of truth):
  <Barrier> (When this Digimon would be deleted in battle, by trashing the top
      card of your security stack, prevent that deletion.)
  [On Play] [When Digivolving] <Draw 1> (Draw 1 card from your deck.) Then, if
      it's your turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in
      the hand.

Inherited:
  <Barrier> (same keyword, inherited instance)

Alt-digi (xros_req):
  [Digivolve] [Patamon]/Lv.3 w/[CS] trait: Cost 2

Key cards used:
- BT23-027: Angemon — the card under test
- BT23-032: Shakkoumon (Lv.5 Yellow/Black; DNA Yellow Lv.4 + Black Lv.4 cost 0 per
    engine parser — card text also supports Blue Lv.4 but the parser currently
    selects one color option)
- BT1-055:  Angemon (Yellow Lv.4, used as Yellow material for DNA digivolve)
- BT2-057:  Greymon (Black Lv.4, used as Black material for DNA digivolve)
- BT1-048:  Patamon (Yellow Lv.3, generic low-level filler / alt-digi base)
- ST1-03:   Agumon (Red Lv.3, non-Shakkoumon filler)
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


ANGEMON_DECK = (
    ["BT23-027"] * 4
    + ["BT23-032"] * 4
    + ["BT1-055"] * 4
    + ["BT2-057"] * 4
    + ["BT1-048"] * 8
    + ["ST1-03"] * 26
)


def _effects_by_flag(card, timing, flag_name):
    """Return effects for a given timing with a specific flag set True."""
    effects = card.effect_list(None)
    return [
        e for e in effects
        if e.timing == timing and getattr(e, flag_name, False)
    ]


@pytest.mark.behavioral
class TestBT23_027Angemon:
    """Tests for BT23-027 Angemon."""

    # ─────────────────────────────────────────────────────────────────
    # C1: <Barrier> — self (top-card) keyword effect
    # ─────────────────────────────────────────────────────────────────

    def test_has_barrier_keyword_on_field(self, debug_runner):
        """Angemon on the field should have the _is_barrier keyword."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT23-027"])
        assert perm.has_keyword("_is_barrier"), (
            "BT23-027 should expose the Barrier keyword while on the field"
        )

    def test_has_at_least_two_barrier_effects(self, debug_runner):
        """BT23-027 must register TWO barrier effects — self + inherited (separate instances)."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        effects = card.effect_list(None)
        barriers = [e for e in effects if getattr(e, "_is_barrier", False)]
        assert len(barriers) == 2, (
            f"Expected exactly 2 barrier effects (self + inherited); got {len(barriers)}"
        )

    # ─────────────────────────────────────────────────────────────────
    # C4: Inherited <Barrier> — separate instance with is_inherited_effect
    # ─────────────────────────────────────────────────────────────────

    def test_inherited_barrier_exists_and_flagged(self, debug_runner):
        """One of the barrier effects must have is_inherited_effect = True."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        effects = card.effect_list(None)
        barriers = [e for e in effects if getattr(e, "_is_barrier", False)]
        inherited_barriers = [
            e for e in barriers if getattr(e, "is_inherited_effect", False)
        ]
        self_barriers = [
            e for e in barriers if not getattr(e, "is_inherited_effect", False)
        ]
        assert len(inherited_barriers) == 1, (
            f"Expected exactly 1 inherited barrier, got {len(inherited_barriers)}"
        )
        assert len(self_barriers) == 1, (
            f"Expected exactly 1 self barrier, got {len(self_barriers)}"
        )

    def test_inherited_and_self_barriers_are_distinct_instances(self, debug_runner):
        """The two barrier effects must be two separate ICardEffect objects."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        effects = card.effect_list(None)
        barriers = [e for e in effects if getattr(e, "_is_barrier", False)]
        assert len(barriers) == 2
        assert barriers[0] is not barriers[1], (
            "Self barrier and inherited barrier must be distinct ICardEffect instances"
        )

    def test_barrier_prevents_deletion_in_battle_by_trashing_top_security(
        self, debug_runner
    ):
        """When Angemon would be deleted in battle, Barrier trashes top security
        to prevent deletion."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, ["BT23-027"])
        # Give the player 5 security cards so we can see the top card get trashed
        player.security_cards.clear()
        for _ in range(5):
            runner.inject_card(1, "BT1-048", "security_top")
        secs_before = len(player.security_cards)
        trash_before = len(player.trash_cards)

        # Trigger a battle-deletion attempt
        result = player.delete_permanent(perm, is_battle=True)

        assert result is False, "Barrier should report that deletion was prevented"
        assert perm in player.battle_area, (
            "Angemon should remain on the field after Barrier triggers"
        )
        assert len(player.security_cards) == secs_before - 1, (
            "Barrier should trash exactly 1 security card"
        )
        assert len(player.trash_cards) == trash_before + 1, (
            "The trashed security card should be in the trash"
        )

    def test_barrier_does_not_trigger_outside_battle(self, debug_runner):
        """Non-battle deletions should NOT trigger Barrier (security stays)."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, ["BT23-027"])
        player.security_cards.clear()
        for _ in range(5):
            runner.inject_card(1, "BT1-048", "security_top")
        secs_before = len(player.security_cards)

        # Effect deletion: Barrier should not save it
        player.delete_permanent(perm, is_battle=False)

        # Angemon should be gone (effect deletion bypasses Barrier)
        assert perm not in player.battle_area
        # Security untouched
        assert len(player.security_cards) == secs_before

    # ─────────────────────────────────────────────────────────────────
    # C2 / C3: [On Play] and [When Digivolving] effect presence
    # ─────────────────────────────────────────────────────────────────

    def test_has_on_play_effect(self, debug_runner):
        """BT23-027 must have exactly 1 On Play effect at OnEnterFieldAnyone."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        on_play = _effects_by_flag(
            card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )
        assert len(on_play) == 1, f"Expected 1 On Play effect, got {len(on_play)}"

    def test_has_when_digivolving_effect(self, debug_runner):
        """BT23-027 must have exactly 1 When Digivolving effect."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        when_digi = _effects_by_flag(
            card, EffectTiming.OnEnterFieldAnyone, "is_when_digivolving"
        )
        assert len(when_digi) == 1, (
            f"Expected 1 When Digivolving effect, got {len(when_digi)}"
        )

    def test_on_play_and_when_digivolving_are_distinct_instances(self, debug_runner):
        """On Play and When Digivolving must be SEPARATE ICardEffect instances."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT23-027", runner.game.player1)

        on_play = _effects_by_flag(
            card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )
        when_digi = _effects_by_flag(
            card, EffectTiming.OnEnterFieldAnyone, "is_when_digivolving"
        )
        assert len(on_play) == 1 and len(when_digi) == 1
        assert on_play[0] is not when_digi[0], (
            "[On Play] and [When Digivolving] must be distinct ICardEffect instances"
        )

    # ─────────────────────────────────────────────────────────────────
    # C2: [On Play] Draw 1 on effect owner's turn
    # ─────────────────────────────────────────────────────────────────

    def test_on_play_draws_one_card(self, debug_runner):
        """[On Play] should draw 1 card from the deck."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, ["BT23-027"])
        player.hand_cards.clear()
        # Ensure deck has known cards
        hand_before = len(player.hand_cards)
        deck_before = len(player.library_cards)
        assert deck_before >= 1

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]

        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert len(player.hand_cards) == hand_before + 1, (
            f"Expected +1 card in hand, got {len(player.hand_cards) - hand_before}"
        )
        assert len(player.library_cards) == deck_before - 1

    def test_on_play_draws_even_if_no_shakkoumon_dna_available(self, debug_runner):
        """The draw always happens, even if conditional DNA digivolve can't fire."""
        runner = debug_runner(deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5)
        game = runner.game
        player = game.player1

        perm = runner.place_on_field(1, ["BT23-027"])
        # No Shakkoumon in hand, no field Digimon (beside Angemon) → DNA cannot happen
        player.hand_cards.clear()
        hand_before = len(player.hand_cards)

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]
        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert len(player.hand_cards) == hand_before + 1, (
            "Draw 1 must execute even when DNA digivolve branch is unavailable"
        )
        assert game.pending_selection is None, (
            "No DNA targets → no selection phase should open"
        )

    # ─────────────────────────────────────────────────────────────────
    # C2/C3: Conditional DNA digivolve into [Shakkoumon] in hand
    # ─────────────────────────────────────────────────────────────────

    def test_on_play_opens_dna_selection_when_shakkoumon_and_field_targets_available(
        self, debug_runner
    ):
        """When Shakkoumon is in hand and 2+ valid field Digimon exist on the
        effect owner's turn, the process callback must open a selection phase."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Place two Lv.4 Digimon to act as DNA materials (Yellow + Black)
        runner.place_on_field(1, ["BT1-055"])  # Yellow Lv.4 Angemon
        runner.place_on_field(1, ["BT2-057"])  # Black Lv.4 Greymon
        # The Angemon whose effect we fire (third permanent)
        perm = runner.place_on_field(1, ["BT23-027"])

        # Shakkoumon in hand for DNA digivolve target
        player.hand_cards.clear()
        runner.inject_card(1, "BT23-032", "hand")

        # Ensure it is effect owner's turn
        assert player.is_my_turn

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]
        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert game.pending_selection is not None, (
            "DNA digivolve selection phase should be pending when all conditions met"
        )

    def test_on_play_no_dna_selection_when_not_effect_owner_turn(
        self, debug_runner
    ):
        """If it's NOT the effect owner's turn, no DNA digivolve selection."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        game = runner.game
        player = game.player1

        # Force it to be opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.turn_player = game.player2
        game.opponent_player = game.player1

        runner.place_on_field(1, ["BT1-055"])
        runner.place_on_field(1, ["BT2-057"])
        perm = runner.place_on_field(1, ["BT23-027"])

        player.hand_cards.clear()
        runner.inject_card(1, "BT23-032", "hand")

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]
        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert game.pending_selection is None, (
            "DNA digivolve should not open when it's not the effect owner's turn"
        )

    def test_on_play_no_dna_selection_when_no_shakkoumon_in_hand(
        self, debug_runner
    ):
        """Without a Shakkoumon card in hand, no DNA selection should open."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, ["BT1-055"])
        runner.place_on_field(1, ["BT2-057"])
        perm = runner.place_on_field(1, ["BT23-027"])

        player.hand_cards.clear()
        # Non-Shakkoumon filler (ST1-03 Agumon Lv.3, no DNA req to Shakkoumon)
        runner.inject_card(1, "ST1-03", "hand")

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]
        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert game.pending_selection is None, (
            "Without Shakkoumon in hand, no DNA digivolve selection should open"
        )

    def test_when_digivolving_draws_and_opens_dna_selection(self, debug_runner):
        """[When Digivolving] callback must draw 1 AND open DNA selection when
        conditions are met."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, ["BT1-055"])  # Yellow Lv.4 material
        runner.place_on_field(1, ["BT2-057"])  # Black Lv.4 material
        perm = runner.place_on_field(1, ["BT23-027"])

        player.hand_cards.clear()
        runner.inject_card(1, "BT23-032", "hand")  # Shakkoumon in hand

        hand_before = len(player.hand_cards)

        when_digi = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_when_digivolving"
        )[0]
        when_digi.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert len(player.hand_cards) == hand_before + 1, (
            "[When Digivolving] must draw 1 card"
        )
        assert game.pending_selection is not None, (
            "[When Digivolving] should open DNA digivolve selection when valid"
        )

    def test_dna_digivolve_filter_only_matches_shakkoumon(self, debug_runner):
        """The filter must not select non-Shakkoumon Digimon from hand."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        game = runner.game
        player = game.player1

        runner.place_on_field(1, ["BT1-055"])
        runner.place_on_field(1, ["BT2-057"])
        perm = runner.place_on_field(1, ["BT23-027"])

        player.hand_cards.clear()
        # A non-Shakkoumon DNA-capable Digimon: BT8-042 is Shakkoumon, so use a
        # different Digimon with DNA costs that isn't named "Shakkoumon".
        # BT23-102 Mastemon has DNA cost Yellow+Purple, NOT named Shakkoumon.
        runner.inject_card(1, "BT23-102", "hand")

        on_play = _effects_by_flag(
            perm.top_card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]
        on_play.on_process_callback({
            "player": player,
            "game": game,
            "permanent": perm,
            "card": perm.top_card,
        })

        assert game.pending_selection is None, (
            "Filter must exclude non-Shakkoumon DNA-capable Digimon"
        )

    # ─────────────────────────────────────────────────────────────────
    # Alt-digivolve: [Patamon]/Lv.3 w/[CS] trait for cost 2
    # ─────────────────────────────────────────────────────────────────

    def test_alt_digi_from_patamon_cost_2(self, debug_runner):
        """Alt-digi: from [Patamon] at cost 2."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=10
        )
        from digimon_gym.engine.validation.digivolve_validator import (
            _check_alt_digivolve, get_alt_digi_cost,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        base_perm = runner.place_on_field(1, ["BT1-048"])  # Patamon Lv.3
        angemon = db.create_card_source("BT23-027", runner.game.player1)

        assert _check_alt_digivolve(angemon, base_perm), (
            "BT23-027 should alt-digivolve from [Patamon]"
        )
        assert get_alt_digi_cost(angemon, base_perm) == 2, (
            "Alt-digi cost should be 2"
        )

    def test_alt_digi_from_lv3_cs_cost_2(self, debug_runner):
        """Alt-digi: from Lv.3 with [CS] trait at cost 2 (non-Patamon base).

        BT23-026 Lopmon is Lv.3 with CS trait — should satisfy the Lv.3/CS variant.
        """
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=10
        )
        from digimon_gym.engine.validation.digivolve_validator import (
            _check_alt_digivolve, get_alt_digi_cost,
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Lv.3 Digimon with CS trait (not Patamon)
        base_perm = runner.place_on_field(1, ["BT23-026"])  # Lopmon Lv.3 CS
        angemon = db.create_card_source("BT23-027", runner.game.player1)

        assert _check_alt_digivolve(angemon, base_perm), (
            "BT23-027 should alt-digivolve from a Lv.3 Digimon w/[CS] trait"
        )
        assert get_alt_digi_cost(angemon, base_perm) == 2, (
            "Alt-digi cost should be 2 for the CS trait variant"
        )

    def test_alt_digi_rejects_lv3_without_cs_and_not_patamon(self, debug_runner):
        """Alt-digi should NOT match a Lv.3 non-Patamon without CS trait."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=10
        )
        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # ST1-03 Agumon is Lv.3 Red without CS trait and not named "Patamon"
        base_perm = runner.place_on_field(1, ["ST1-03"])
        angemon = db.create_card_source("BT23-027", runner.game.player1)

        assert not _check_alt_digivolve(angemon, base_perm), (
            "BT23-027 alt-digi must reject a Lv.3 non-Patamon Digimon lacking CS"
        )

    # ─────────────────────────────────────────────────────────────────
    # Field presence condition guard
    # ─────────────────────────────────────────────────────────────────

    def test_on_play_condition_false_when_card_not_on_field(self, debug_runner):
        """The On Play effect's condition should be False when the card isn't on field."""
        runner = debug_runner(
            deck1=ANGEMON_DECK, deck2=ANGEMON_DECK, initial_memory=5
        )
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Create a card not on the field
        card = db.create_card_source("BT23-027", runner.game.player1)
        runner.game.player1.hand_cards.append(card)

        on_play = _effects_by_flag(
            card, EffectTiming.OnEnterFieldAnyone, "is_on_play"
        )[0]

        assert on_play.can_use_condition({
            "player": runner.game.player1,
            "game": runner.game,
            "card": card,
        }) is False, (
            "On Play condition must return False when card is not on field"
        )
