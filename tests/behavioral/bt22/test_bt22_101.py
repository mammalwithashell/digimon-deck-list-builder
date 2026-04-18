"""Behavioral tests for BT22-101 Kyoko Kuremi (Tamer, Black/Yellow, Cost 5, CS).

Card text (source of truth):
[Start of Your Turn] If you have 2 or less memory, set it to 3.
[All Turns] When any of your level 4 or higher [CS] trait Digimon are deleted,
    by suspending this Tamer, return 1 [CS] trait Digimon card from your trash
    to the hand.
[Your Turn] [Once Per Turn] When this Tamer unsuspends, if you don't have
    [Alphamon], this Tamer may digivolve into [Alphamon] in the hand with the
    digivolution cost reduced by 2.
Security Effect: [Security] Play this card without paying the cost.

Key cards used for tests:
- BT22-101: Kyoko Kuremi (Tamer, CS, cost 5)
- BT22-010: Meramon (Digimon, Lv.4, CS)          — qualifies for C2
- BT22-011: BlueMeramon (Digimon, Lv.5, CS)       — qualifies for C2
- BT22-008: Agumon (Digimon, Lv.3, CS)            — NOT Lv4+
- BT22-009: Effecmon (Digimon, Lv.4, no CS)       — NOT CS
- BT22-063: Alphamon (Digimon, Lv.6, CS)          — exact name for C3
- BT9-111:  Alphamon: Ouryuken (Lv.7)             — NOT exact name "Alphamon"
- BT1-046:  Kudamon (Digimon, Lv.3, no CS)        — control/non-qualifier
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


def _find_effect(perm, name_substring):
    """Return the first effect on perm.top_card whose name contains substring."""
    for eff in perm.top_card.effect_list(None):
        if name_substring in (eff.effect_name or ""):
            return eff
    return None


def _find_effects(perm, name_substring):
    """Return all effects on perm.top_card whose name contains substring."""
    return [eff for eff in perm.top_card.effect_list(None)
            if name_substring in (eff.effect_name or "")]


DECK = ["BT22-101"] * 4 + ["BT22-010"] * 4 + ["BT22-011"] * 4 + ["BT22-063"] * 4 + ["BT22-008"] * 4 + ["BT22-009"] * 4 + ["BT1-046"] * 26


@pytest.mark.behavioral
class TestBT22101KyokoKuremi:
    """Tests for BT22-101 Kyoko Kuremi."""

    # ------------------------------------------------------------------ #
    # C1 — [Start of Your Turn] set memory to 3 if <= 2
    # ------------------------------------------------------------------ #

    def test_c1_memory_gate_sets_to_3_when_low(self, debug_runner):
        """When memory <= 2 on your turn, effect sets to 3."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=0)
        game = runner.game
        perm = runner.place_on_field(1, ["BT22-101"])
        game.memory = 2

        eff = _find_effect(perm, "Set memory to 3")
        assert eff is not None, "Memory gate effect missing"
        # Condition checks field presence + owner's turn
        assert eff.can_use_condition({'game': game, 'player': game.player1}), \
            "Memory gate condition should pass when Kyoko is on own field on own turn"

        eff.on_process_callback({'player': game.player1, 'game': game, 'permanent': perm})
        assert game.memory == 3, f"Memory should be 3, got {game.memory}"

    def test_c1_memory_gate_no_op_when_high(self, debug_runner):
        """When memory > 2, effect does nothing."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=0)
        game = runner.game
        perm = runner.place_on_field(1, ["BT22-101"])
        game.memory = 5

        eff = _find_effect(perm, "Set memory to 3")
        assert eff is not None
        eff.on_process_callback({'player': game.player1, 'game': game, 'permanent': perm})
        assert game.memory == 5, "Memory should not decrease from 5 to 3"

    def test_c1_memory_gate_field_presence_check(self, debug_runner):
        """Condition should fail when Kyoko is not on the field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=0)
        # Don't place on field — card exists only in library
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-101", runner.game.player1)
        runner.game.player1.hand_cards.append(cs)
        # Scan cached effects from hand card
        effs = cs.effect_list(None)
        memory_eff = next((e for e in effs if "Set memory to 3" in (e.effect_name or "")), None)
        assert memory_eff is not None
        # Field presence check should fail
        assert not memory_eff.can_use_condition({'game': runner.game, 'player': runner.game.player1}), \
            "Memory gate should not trigger when Kyoko isn't on the field"

    # ------------------------------------------------------------------ #
    # C2 — [All Turns] OnDeletion observer: CS Lv4+ → suspend cost → return
    # from trash
    # ------------------------------------------------------------------ #

    def test_c2_uses_deletion_observer_pattern(self, debug_runner):
        """Effect must use _is_deletion_observer=True, NOT is_on_deletion/
        OnDestroyedAnyone — otherwise it can never fire for other perms."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-101"])

        effs = _find_effects(perm, "Suspend tamer")
        assert len(effs) >= 1, "Expected deletion-trigger effect"
        eff = effs[0]
        assert getattr(eff, '_is_deletion_observer', False) is True, (
            "C2 must use _is_deletion_observer pattern so it fires when OTHER "
            "permanents are deleted"
        )
        assert getattr(eff, 'is_on_deletion', False) is False, (
            "C2 must NOT use is_on_deletion (that fires from the deleted perm's "
            "own stack, not from observers)"
        )

    def test_c2_triggers_on_own_cs_lv4_digimon_deletion(self, debug_runner):
        """When our CS Lv4 Digimon is deleted, Kyoko suspends and returns
        a CS Digimon from trash to hand."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"])
        meramon = runner.place_on_field(1, ["BT22-010"])  # Lv4 CS

        # Put a CS Digimon in trash to return
        runner.inject_card(1, "BT22-011", "trash")  # BlueMeramon Lv5 CS
        trash_before = len(game.player1.trash_cards)
        hand_before = len(game.player1.hand_cards)

        # Delete Meramon — should trigger Kyoko's observer
        game.player1.delete_permanent(meramon)
        # Resolve any pending selection (trash pick)
        runner.auto_resolve(max_steps=5)

        # Kyoko should be suspended (cost paid)
        assert kyoko.is_suspended, "Kyoko should be suspended by paying the cost"
        # Hand should have 1 more card (returned from trash)
        assert len(game.player1.hand_cards) == hand_before + 1, (
            f"Hand should grow by 1 (CS Digimon returned from trash); "
            f"was {hand_before}, now {len(game.player1.hand_cards)}"
        )
        # Trash should have fewer trash cards (minus the returned one, plus
        # meramon's card that was deleted — net -1 from pre-delete state + 1
        # added = 0 net on trash count relative to pre-delete)
        # Specifically, the returned BlueMeramon must no longer be in trash
        assert not any(
            c.c_entity_base and c.c_entity_base.card_id == "BT22-011"
            for c in game.player1.trash_cards
        ), "BlueMeramon should be out of trash (returned to hand)"

    def test_c2_does_not_trigger_on_lv3_cs_digimon(self, debug_runner):
        """Lv.3 CS Digimon deletion must NOT trigger the effect."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"])
        agumon = runner.place_on_field(1, ["BT22-008"])  # Lv3 CS

        runner.inject_card(1, "BT22-011", "trash")  # BlueMeramon (CS) in trash
        hand_before = len(game.player1.hand_cards)

        game.player1.delete_permanent(agumon)
        runner.auto_resolve(max_steps=5)

        assert not kyoko.is_suspended, "Kyoko should NOT be suspended (Lv3 fails Lv4+ gate)"
        assert len(game.player1.hand_cards) == hand_before, (
            "Hand should be unchanged — no CS Digimon returned"
        )

    def test_c2_does_not_trigger_on_non_cs_lv4(self, debug_runner):
        """Non-CS Lv.4 Digimon deletion must NOT trigger the effect."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"])
        effecmon = runner.place_on_field(1, ["BT22-009"])  # Lv4, no CS

        runner.inject_card(1, "BT22-011", "trash")
        hand_before = len(game.player1.hand_cards)

        game.player1.delete_permanent(effecmon)
        runner.auto_resolve(max_steps=5)

        assert not kyoko.is_suspended, "Kyoko should NOT be suspended (not CS trait)"
        assert len(game.player1.hand_cards) == hand_before, (
            "Hand should be unchanged — deleted digimon is not CS"
        )

    def test_c2_does_not_trigger_on_opponent_cs_digimon(self, debug_runner):
        """Opponent's CS Lv4 Digimon deletion must NOT trigger the effect."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"])
        opp_meramon = runner.place_on_field(2, ["BT22-010"])  # Lv4 CS

        runner.inject_card(1, "BT22-011", "trash")
        hand_before = len(game.player1.hand_cards)

        game.player2.delete_permanent(opp_meramon)
        runner.auto_resolve(max_steps=5)

        assert not kyoko.is_suspended, (
            "Kyoko should NOT be suspended when OPPONENT's Digimon is deleted"
        )
        assert len(game.player1.hand_cards) == hand_before

    def test_c2_only_offers_cs_digimon_from_trash(self, debug_runner):
        """Only [CS] trait Digimon cards in trash should be valid selection
        targets — non-CS cards must be filtered out."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"])
        meramon = runner.place_on_field(1, ["BT22-010"])  # Lv4 CS

        # Put a non-CS card and a CS Digimon in trash
        runner.inject_card(1, "BT1-046", "trash")     # Kudamon (non-CS, Holy Beast)
        runner.inject_card(1, "BT22-011", "trash")    # BlueMeramon (CS)

        hand_before = len(game.player1.hand_cards)

        game.player1.delete_permanent(meramon)
        runner.auto_resolve(max_steps=8)

        # Hand should have exactly the CS Digimon (BT22-011)
        assert any(
            c.c_entity_base and c.c_entity_base.card_id == "BT22-011"
            for c in game.player1.hand_cards
        ), "CS BlueMeramon should be in hand"
        # Kudamon (non-CS) must still be in trash
        assert any(
            c.c_entity_base and c.c_entity_base.card_id == "BT1-046"
            for c in game.player1.trash_cards
        ), "Non-CS Kudamon must remain in trash (not a valid selection target)"
        assert len(game.player1.hand_cards) == hand_before + 1

    def test_c2_no_trigger_when_tamer_already_suspended(self, debug_runner):
        """If Kyoko is already suspended, the suspend cost can't be paid →
        no trigger."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game

        kyoko = runner.place_on_field(1, ["BT22-101"], is_suspended=True)
        meramon = runner.place_on_field(1, ["BT22-010"])

        runner.inject_card(1, "BT22-011", "trash")
        hand_before = len(game.player1.hand_cards)

        game.player1.delete_permanent(meramon)
        runner.auto_resolve(max_steps=5)

        assert len(game.player1.hand_cards) == hand_before, (
            "Can't pay suspend cost when already suspended"
        )

    # ------------------------------------------------------------------ #
    # C3 — [Your Turn] [OPT] When this Tamer unsuspends, digivolve into
    # [Alphamon] for cost reduced by 2
    # ------------------------------------------------------------------ #

    def test_c3_uses_on_untap_timing(self, debug_runner):
        """Effect 3 must use OnUnTappedAnyone timing."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-101"])
        effs = _find_effects(perm, "Digivolve into")
        assert len(effs) >= 1
        eff = effs[0]
        assert eff.timing == EffectTiming.OnUnTappedAnyone

    def test_c3_once_per_turn_and_optional(self, debug_runner):
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-101"])
        effs = _find_effects(perm, "Digivolve into")
        eff = effs[0]
        assert eff.max_count_per_turn == 1, "C3 must be Once Per Turn"
        assert eff.is_optional is True, "C3 must be optional ('may digivolve')"

    def test_c3_condition_fails_when_alphamon_already_on_field(self, debug_runner):
        """If you already have Alphamon on the battle area, condition must
        fail ('if you don't have [Alphamon]')."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        kyoko = runner.place_on_field(1, ["BT22-101"])
        runner.place_on_field(1, ["BT22-063"])  # Alphamon on field

        eff = _find_effect(kyoko, "Digivolve into")
        assert eff is not None
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': kyoko, 'event_permanent': kyoko,
        }
        assert not eff.can_use_condition(ctx), (
            "C3 should fail when Alphamon is already on field"
        )

    def test_c3_condition_fails_for_unsuspend_of_other_permanent(self, debug_runner):
        """C3 must only trigger when THIS Tamer unsuspends — not when some
        other permanent unsuspends."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        kyoko = runner.place_on_field(1, ["BT22-101"])
        meramon = runner.place_on_field(1, ["BT22-010"])
        runner.inject_card(1, "BT22-063", "hand")

        eff = _find_effect(kyoko, "Digivolve into")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': kyoko,
            'event_permanent': meramon,  # a DIFFERENT perm unsuspended
        }
        assert not eff.can_use_condition(ctx), (
            "C3 condition must fail for unsuspend events on other permanents"
        )

    def test_c3_condition_fails_on_opponent_turn(self, debug_runner):
        """[Your Turn] effect — must fail on opponent turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        # Flip is_my_turn flags so player1 is NOT turn player
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        kyoko = runner.place_on_field(1, ["BT22-101"])
        eff = _find_effect(kyoko, "Digivolve into")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': kyoko, 'event_permanent': kyoko,
        }
        assert not eff.can_use_condition(ctx), (
            "C3 must fail outside owner's turn"
        )

    def test_c3_condition_passes_when_kyoko_unsuspends_and_no_alphamon(self, debug_runner):
        """Standard positive case: our turn, Kyoko unsuspends, no Alphamon."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        kyoko = runner.place_on_field(1, ["BT22-101"])
        runner.inject_card(1, "BT22-063", "hand")
        eff = _find_effect(kyoko, "Digivolve into")
        ctx = {
            'game': game, 'player': game.player1,
            'permanent': kyoko, 'event_permanent': kyoko,
        }
        assert eff.can_use_condition(ctx), (
            "C3 should pass: own turn, unsuspend on THIS tamer, no Alphamon on field"
        )

    def test_c3_filter_only_allows_exact_alphamon_name(self, debug_runner):
        """Process must only accept exact name 'Alphamon', NOT
        'Alphamon: Ouryuken' (substring trap)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        kyoko = runner.place_on_field(1, ["BT22-101"])

        # Put Alphamon: Ouryuken (not exact name) in hand
        runner.inject_card(1, "BT9-111", "hand")

        eff = _find_effect(kyoko, "Digivolve into")
        eff.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': kyoko,
        })

        # Engine queues a selection — check that Ouryuken is NOT a legal target
        sel = game.pending_selection
        if sel is None:
            # No selection started → no valid targets → correct behavior
            return
        # Derive the hand indices offered
        from digimon_gym.engine.game.effects import SEL_HAND_START
        valid = sel.valid_indices
        hand_indices = [v - SEL_HAND_START for v in valid if v >= SEL_HAND_START]
        offered_cards = [
            game.player1.hand_cards[i] for i in hand_indices
            if 0 <= i < len(game.player1.hand_cards)
        ]
        # Ouryuken must not be in offered cards
        assert not any(
            c.c_entity_base and c.c_entity_base.card_id == "BT9-111"
            for c in offered_cards
        ), "Alphamon: Ouryuken must NOT be a valid target (exact name match required)"

    def test_c3_filter_allows_exact_alphamon(self, debug_runner):
        """Exact 'Alphamon' (BT22-063) should be offered as a valid digivolve
        target."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        kyoko = runner.place_on_field(1, ["BT22-101"])
        runner.inject_card(1, "BT22-063", "hand")  # Alphamon

        eff = _find_effect(kyoko, "Digivolve into")
        eff.on_process_callback({
            'player': game.player1, 'game': game, 'permanent': kyoko,
        })

        sel = game.pending_selection
        assert sel is not None, (
            "A selection should have been requested (valid Alphamon in hand)"
        )
        from digimon_gym.engine.game.effects import SEL_HAND_START
        valid = sel.valid_indices
        hand_indices = [v - SEL_HAND_START for v in valid if v >= SEL_HAND_START]
        offered = [game.player1.hand_cards[i] for i in hand_indices
                   if 0 <= i < len(game.player1.hand_cards)]
        assert any(
            c.c_entity_base and c.c_entity_base.card_id == "BT22-063"
            for c in offered
        ), "BT22-063 Alphamon must be offered as a digivolve target"

    # ------------------------------------------------------------------ #
    # C4 — [Security] Play this card without paying the cost
    # ------------------------------------------------------------------ #

    def test_c4_security_effect_flag(self, debug_runner):
        """Effect must have is_security_effect=True."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-101", runner.game.player1)
        sec_effs = [e for e in cs.effect_list(None)
                    if getattr(e, 'is_security_effect', False)]
        assert len(sec_effs) >= 1, "BT22-101 must have a security effect"

    def test_c4_security_effect_has_process_callback(self, debug_runner):
        """Security effect must have a process callback (not inert)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-101", runner.game.player1)
        sec_effs = [e for e in cs.effect_list(None)
                    if getattr(e, 'is_security_effect', False)]
        assert sec_effs[0].on_process_callback is not None, (
            "Security effect must have a process callback to actually play the tamer"
        )

    def test_c4_security_effect_plays_tamer(self, debug_runner):
        """When security effect's process fires, the tamer is put onto the field."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        game = runner.game
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT22-101", game.player1)
        game.player1.security_cards.insert(0, cs)

        sec_effs = [e for e in cs.effect_list(None)
                    if getattr(e, 'is_security_effect', False)]
        eff = sec_effs[0]
        field_before = len(game.player1.battle_area)

        eff.on_process_callback({
            'game': game, 'player': game.player1,
            'permanent': None, 'card': cs,
        })

        assert len(game.player1.battle_area) == field_before + 1, (
            "Tamer should be placed onto the battle area"
        )
        # Tamer should be on field by card_id
        assert any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT22-101"
            for p in game.player1.battle_area
        )
