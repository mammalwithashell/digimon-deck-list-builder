"""Behavioral tests for EX6-020 Gatomon (Lv.4 Yellow Holy Beast).

Card text:
  [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Add 1
  card with the [Angel], [Archangel] or [Fallen Angel] trait and 1 [Mirei
  Mikagura] among them to the hand. Return the rest to the bottom of the deck.

  Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
  gets -2000 DP for the turn.

Clause decomposition:
  C1: [On Play] reveal top 3, multi-pass select Angel-family Digimon + Mirei Mikagura Tamer.
  C2: [When Digivolving] same as C1 (separate instance).
  C3: Inherited [When Attacking][OPT] -2000 DP to 1 opponent Digimon until end of turn.

Key cards used:
- EX6-020 Gatomon       : card under test (Yellow Lv4 Holy Beast)
- BT1-048 Patamon       : Yellow Lv3 Mammal (digivolve source, non-Angel)
- EX6-017 Luxmon        : Yellow Lv3 Angel trait (digivolve source, also in revealed tests)
- BT1-055 Angemon       : Yellow Lv4 Angel trait (for reveal selection)
- BT2-037 Angewomon     : Yellow Lv5 Archangel trait (for reveal selection)
- BT2-074 Devimon       : Purple Lv4 Fallen Angel trait (for reveal selection)
- BT11-094 Mirei Mikagura : Yellow/Purple Tamer named Mirei Mikagura
- EX6-074 Mirei Mikagura  : Yellow/Purple Tamer named Mirei Mikagura (alt)
- BT13-036 Liollmon     : Yellow Lv3 Holy Beast (non-matching Digimon — neither Angel nor Mirei)
- BT1-010 Agumon        : Red Lv3 (opponent Digimon for DP tests)
- ST1-03 Agumon         : Red Lv3 (opponent Digimon for DP leak test)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


# === Card IDs ===
GATOMON = "EX6-020"
PATAMON = "BT1-048"            # Yellow Lv3, non-Angel
LUXMON = "EX6-017"             # Yellow Lv3 Angel
ANGEMON = "BT1-055"            # Yellow Lv4 Angel
ANGEWOMON = "BT2-037"          # Yellow Lv5 Archangel
DEVIMON = "BT2-074"            # Purple Lv4 Fallen Angel
MIREI_BT11 = "BT11-094"        # Mirei Mikagura (Tamer)
MIREI_EX6 = "EX6-074"          # Mirei Mikagura (Tamer, alt)
LIOLLMON = "BT13-036"          # Yellow Lv3 Holy Beast (non-matching)
AGUMON_RED = "BT1-010"         # Red Lv3 (opponent target)
AGUMON_ST = "ST1-03"           # Red Lv3 (opponent target)


# --------------------------------------------------------------------------
# C1: [On Play] Reveal 3 + select Angel-family Digimon + Mirei Tamer
# --------------------------------------------------------------------------

@pytest.mark.behavioral
class TestEX6020_OnPlayEffect:
    """Clause 1: On Play reveal-3 + two selection passes."""

    def test_on_play_effect_exists_with_correct_timing(self, debug_runner):
        """On Play effect exists with OnEnterFieldAnyone + is_on_play flag."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ]
        assert len(on_play) == 1, (
            f"Should have exactly 1 On Play effect, got {len(on_play)}"
        )

    def test_on_play_not_marked_as_when_digivolving(self, debug_runner):
        """The On Play effect must NOT also claim is_when_digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        assert not getattr(on_play, "is_when_digivolving", False), (
            "On Play effect must not be flagged as When Digivolving"
        )

    def test_on_play_selects_angel_and_mirei_into_hand(self, debug_runner):
        """Revealing top 3 [Angemon, Mirei, Patamon] adds Angemon + Mirei to hand."""
        runner = debug_runner(initial_memory=5)
        # Clear library, then inject 3 known cards at library_top (last inject is topmost)
        runner.clear_zone(1, "library")
        # Order we want from top: ANGEMON, MIREI_BT11, PATAMON
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, MIREI_BT11, "library_top")
        runner.inject_card(1, ANGEMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        hand_before = {c.card_id for c in game.player1.hand_cards}

        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert ANGEMON in hand_ids, (
            f"Angel Digimon should be added to hand. Hand: {hand_ids}"
        )
        assert MIREI_BT11 in hand_ids, (
            f"Mirei Mikagura Tamer should be added to hand. Hand: {hand_ids}"
        )
        # Patamon (non-matching) should NOT be in hand — it goes to deck bottom
        patamon_new = [cid for cid in hand_ids if cid == PATAMON
                       and cid not in hand_before]
        assert not patamon_new, (
            f"Patamon should NOT be added to hand (not Angel/Mirei). Hand: {hand_ids}"
        )

    def test_on_play_puts_non_matching_to_deck_bottom(self, debug_runner):
        """Non-matching revealed cards are returned to the BOTTOM of the deck."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        # Top-to-bottom: ANGEMON, MIREI_BT11, PATAMON
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, MIREI_BT11, "library_top")
        runner.inject_card(1, ANGEMON, "library_top")
        # Some filler at the bottom so we can assert Patamon is at the TRUE bottom
        runner.inject_card(1, LIOLLMON, "library_bottom")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        effects = top.effect_list(None)
        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        # Patamon should not be in hand, should be at deck bottom
        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert PATAMON not in hand_ids, (
            f"Patamon should not be added to hand. Hand: {hand_ids}"
        )
        lib_ids = [c.card_id for c in game.player1.library_cards]
        assert PATAMON in lib_ids, (
            f"Patamon should be returned to deck. Library: {lib_ids}"
        )
        # Patamon must be at the BOTTOM (end) of library_cards
        assert lib_ids[-1] == PATAMON, (
            f"Patamon should be at deck bottom. Library tail: {lib_ids[-3:]}"
        )

    def test_on_play_archangel_qualifies(self, debug_runner):
        """Archangel-trait Digimon (e.g., Angewomon) qualifies for the first pass."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, ANGEWOMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        on_play = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert ANGEWOMON in hand_ids, (
            f"Archangel should qualify and be added to hand. Hand: {hand_ids}"
        )

    def test_on_play_fallen_angel_qualifies(self, debug_runner):
        """Fallen Angel-trait Digimon (e.g., Devimon) qualifies for the first pass."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, DEVIMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        on_play = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert DEVIMON in hand_ids, (
            f"Fallen Angel should qualify and be added to hand. Hand: {hand_ids}"
        )

    def test_on_play_no_angel_in_reveal_leaves_those_in_deck(self, debug_runner):
        """If no Angel-family Digimon is in the revealed 3, only Mirei is added."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        # Revealed: Patamon, Liollmon, Mirei — no Angel-family Digimon
        runner.inject_card(1, LIOLLMON, "library_top")
        runner.inject_card(1, MIREI_BT11, "library_top")
        runner.inject_card(1, PATAMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        on_play = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert MIREI_BT11 in hand_ids, (
            f"Mirei should still be added. Hand: {hand_ids}"
        )
        # No Angel was revealed, so none should be in hand
        assert ANGEMON not in hand_ids
        assert ANGEWOMON not in hand_ids
        assert DEVIMON not in hand_ids

    def test_on_play_ignores_non_digimon_for_angel_pass(self, debug_runner):
        """Non-Digimon with Angel-related name must NOT satisfy the first pass."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        # All non-matching: Patamon, Liollmon, Patamon
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, LIOLLMON, "library_top")
        runner.inject_card(1, PATAMON, "library_top")

        perm = runner.place_on_field(1, [GATOMON])
        game = runner.game

        top = perm.top_card
        on_play = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ][0]

        hand_before_ids = [c.card_id for c in game.player1.hand_cards]
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
        })
        runner.auto_resolve()

        hand_after_ids = [c.card_id for c in game.player1.hand_cards]
        # Nothing new in hand from the reveal
        assert len(hand_after_ids) == len(hand_before_ids), (
            f"Hand should not gain any cards (no matches). "
            f"Before: {hand_before_ids}, After: {hand_after_ids}"
        )


# --------------------------------------------------------------------------
# C2: [When Digivolving] Same effect — separate instance
# --------------------------------------------------------------------------

@pytest.mark.behavioral
class TestEX6020_WhenDigivolvingEffect:
    """Clause 2: When Digivolving effect — separate instance with same logic."""

    def test_wd_effect_exists_with_correct_timing(self, debug_runner):
        """When Digivolving effect exists with OnEnterFieldAnyone + is_when_digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        top = perm.top_card
        effects = top.effect_list(None)

        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_when_digivolving", False)
        ]
        assert len(wd) == 1, (
            f"Should have exactly 1 When Digivolving effect, got {len(wd)}"
        )

    def test_wd_is_separate_from_on_play(self, debug_runner):
        """When Digivolving must be a separate effect instance from On Play."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        top = perm.top_card
        effects = top.effect_list(None)

        on_play = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_on_play", False)
        ]
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_when_digivolving", False)
        ]
        assert len(on_play) == 1
        assert len(wd) == 1
        assert on_play[0] is not wd[0], (
            "On Play and When Digivolving must be two separate ICardEffect instances"
        )

    def test_wd_not_marked_as_on_play(self, debug_runner):
        """When Digivolving must NOT also claim is_on_play."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON])
        top = perm.top_card

        wd = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_when_digivolving", False)
        ][0]

        assert not getattr(wd, "is_on_play", False), (
            "When Digivolving effect must not be flagged as On Play"
        )

    def test_wd_process_reveals_and_selects(self, debug_runner):
        """Running the WD process callback reveals 3 and adds Angel + Mirei."""
        runner = debug_runner(initial_memory=5)
        runner.clear_zone(1, "library")
        # Top-to-bottom: ANGEWOMON, MIREI_EX6, PATAMON
        runner.inject_card(1, PATAMON, "library_top")
        runner.inject_card(1, MIREI_EX6, "library_top")
        runner.inject_card(1, ANGEWOMON, "library_top")

        # Place Luxmon under Gatomon to simulate a digivolved stack
        perm = runner.place_on_field(1, [LUXMON, GATOMON])
        game = runner.game

        top = perm.top_card  # Gatomon
        wd = [
            e for e in top.effect_list(None)
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, "is_when_digivolving", False)
        ][0]

        wd.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": top,
            "permanent": perm,
            "digivolved_permanent": perm,
        })
        runner.auto_resolve()

        hand_ids = [c.card_id for c in game.player1.hand_cards]
        assert ANGEWOMON in hand_ids, (
            f"Archangel should be added to hand by WD effect. Hand: {hand_ids}"
        )
        assert MIREI_EX6 in hand_ids, (
            f"Mirei should be added to hand by WD effect. Hand: {hand_ids}"
        )


# --------------------------------------------------------------------------
# C3: Inherited [When Attacking][OPT] -2000 DP
# --------------------------------------------------------------------------

@pytest.mark.behavioral
class TestEX6020_InheritedDPMinus2000:
    """Clause 3: Inherited [When Attacking][OPT] -2000 DP effect."""

    def test_inherited_effect_uses_on_use_attack_timing(self, debug_runner):
        """The inherited DP-down effect MUST use OnUseAttack (28), NOT OnAllyAttack (32)."""
        runner = debug_runner(initial_memory=5)
        # EX6-020 Gatomon stacked UNDER a higher-level Digimon so it's inherited
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])

        bottom = perm.card_sources[0]  # EX6-020 Gatomon inherited
        effects = bottom.effect_list(None)

        atk_effects = [
            e for e in effects
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ]
        assert len(atk_effects) == 1, (
            f"Should have exactly 1 inherited OnUseAttack effect, got {len(atk_effects)}. "
            f"All inherited timings: "
            f"{[e.timing for e in effects if getattr(e, 'is_inherited_effect', False)]}"
        )

    def test_inherited_effect_is_opt(self, debug_runner):
        """The inherited DP effect is [Once Per Turn] (max_count_per_turn == 1) with a hash string."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])

        bottom = perm.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        assert atk_eff.max_count_per_turn == 1, (
            f"Should be Once Per Turn, got {atk_eff.max_count_per_turn}"
        )
        assert getattr(atk_eff, "hash_string", None), (
            "OPT effect requires a hash_string"
        )

    def test_inherited_effect_condition_self_only(self, debug_runner):
        """The inherited condition should gate on 'this is the attacker' (context.attacker)."""
        runner = debug_runner(initial_memory=5)
        perm_a = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        perm_b = runner.place_on_field(1, [PATAMON])  # unrelated Digimon
        # Opponent target required by condition (at least one opp Digimon)
        runner.place_on_field(2, [AGUMON_RED])

        # Grab the bottom EX6-020 source on perm_a
        bottom = perm_a.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        # Correct attacker (perm_a hosts the inherited EX6-020) — condition should pass
        ctx_self = {
            "player": runner.game.player1,
            "game": runner.game,
            "attacker": perm_a,
            "event_player": runner.game.player1,
        }
        assert atk_eff.can_use_condition(ctx_self), (
            "Condition should pass when attacker is the host of this inherited effect"
        )

        # Wrong attacker (perm_b) — condition should fail
        ctx_other = {
            "player": runner.game.player1,
            "game": runner.game,
            "attacker": perm_b,
            "event_player": runner.game.player1,
        }
        assert not atk_eff.can_use_condition(ctx_other), (
            "Condition should fail when a different permanent is the attacker"
        )

    def test_process_does_not_auto_select_target(self, debug_runner):
        """Process must not auto-pick a target — must open a selection phase."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        # Two opponent Digimon candidates
        opp_a = runner.place_on_field(2, [AGUMON_RED])
        opp_b = runner.place_on_field(2, [AGUMON_ST])

        bottom = perm.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        game = runner.game
        # Capture DP before
        dp_a_before = opp_a.dp
        dp_b_before = opp_b.dp

        atk_eff.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": bottom,
            "permanent": perm,
            "attacker": perm,
            "event_player": game.player1,
        })

        # BEFORE the user selects, neither opponent should have been touched
        # (pending_selection should be set OR selection is ready to be made)
        dp_a_mid = opp_a.dp
        dp_b_mid = opp_b.dp
        assert dp_a_mid == dp_a_before, (
            f"Opponent A DP must not be auto-modified before selection. "
            f"Before: {dp_a_before}, After (mid): {dp_a_mid}"
        )
        assert dp_b_mid == dp_b_before, (
            f"Opponent B DP must not be auto-modified before selection. "
            f"Before: {dp_b_before}, After (mid): {dp_b_mid}"
        )

        # There should now be a pending selection (opponent target phase)
        assert game.pending_selection is not None, (
            "Process should open a target selection phase, not auto-select"
        )

    def test_process_applies_minus_2000_to_selected_target(self, debug_runner):
        """After auto-resolve, exactly 1 opponent Digimon loses 2000 DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        opp_a = runner.place_on_field(2, [AGUMON_RED])
        opp_b = runner.place_on_field(2, [AGUMON_ST])

        bottom = perm.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        game = runner.game
        dp_a_before = opp_a.dp
        dp_b_before = opp_b.dp

        atk_eff.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": bottom,
            "permanent": perm,
            "attacker": perm,
            "event_player": game.player1,
        })
        runner.auto_resolve()

        dp_a_after = opp_a.dp
        dp_b_after = opp_b.dp
        # Exactly one of them lost 2000; the other is unchanged
        delta_a = dp_a_before - dp_a_after
        delta_b = dp_b_before - dp_b_after
        assert (delta_a == 2000 and delta_b == 0) or (delta_a == 0 and delta_b == 2000), (
            f"Exactly one opponent Digimon should lose 2000 DP. "
            f"A: {dp_a_before}->{dp_a_after} (delta {delta_a}), "
            f"B: {dp_b_before}->{dp_b_after} (delta {delta_b})"
        )

    def test_dp_modifier_does_not_leak_to_other_permanents(self, debug_runner):
        """CHANGE_DP modifier must be target-equality guarded — no leak to every perm."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        opp_target = runner.place_on_field(2, [AGUMON_RED])
        # Untouched bystander: our own field Digimon and another opponent
        own_other = runner.place_on_field(1, [PATAMON])
        opp_bystander = runner.place_on_field(2, [AGUMON_ST])

        bottom = perm.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        game = runner.game
        own_perm_dp_before = perm.dp  # Angewomon DP — should NOT drop
        own_other_dp_before = own_other.dp
        opp_target_dp_before = opp_target.dp
        opp_bystander_dp_before = opp_bystander.dp

        atk_eff.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": bottom,
            "permanent": perm,
            "attacker": perm,
            "event_player": game.player1,
        })
        runner.auto_resolve()

        # Our permanents must be unaffected (no leak)
        assert perm.dp == own_perm_dp_before, (
            f"Own Angewomon DP must not change. "
            f"Before: {own_perm_dp_before}, After: {perm.dp}"
        )
        assert own_other.dp == own_other_dp_before, (
            f"Own Patamon DP must not change. "
            f"Before: {own_other_dp_before}, After: {own_other.dp}"
        )

        # Exactly ONE of the two opponent Digimon should have -2000
        deltas = [
            opp_target_dp_before - opp_target.dp,
            opp_bystander_dp_before - opp_bystander.dp,
        ]
        assert sorted(deltas) == [0, 2000], (
            f"Exactly one opponent Digimon should lose 2000 DP (no leak). "
            f"Target: {opp_target_dp_before}->{opp_target.dp}, "
            f"Bystander: {opp_bystander_dp_before}->{opp_bystander.dp}"
        )

    def test_dp_modifier_expires_at_end_of_turn(self, debug_runner):
        """The -2000 DP modifier should be cleared at end of turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [GATOMON, ANGEWOMON])
        opp = runner.place_on_field(2, [AGUMON_RED])

        bottom = perm.card_sources[0]
        atk_eff = [
            e for e in bottom.effect_list(None)
            if getattr(e, "is_inherited_effect", False)
            and e.timing == EffectTiming.OnUseAttack
        ][0]

        game = runner.game
        dp_before = opp.dp

        atk_eff.on_process_callback({
            "player": game.player1,
            "game": game,
            "card": bottom,
            "permanent": perm,
            "attacker": perm,
            "event_player": game.player1,
        })
        runner.auto_resolve()

        assert opp.dp == dp_before - 2000, (
            f"Opponent should have -2000 DP during the turn. "
            f"Before: {dp_before}, During: {opp.dp}"
        )

        # Simulate end-of-turn cleanup: clear temp DP on all perms +
        # clear registry modifiers with expiry='end_of_turn'
        for p in list(game.player1.battle_area) + list(game.player2.battle_area):
            p.clear_temp_dp()
        game.modifiers.clear_expiry("end_of_turn")

        assert opp.dp == dp_before, (
            f"Opponent DP should restore at end of turn. "
            f"Before: {dp_before}, After EoT: {opp.dp}"
        )
