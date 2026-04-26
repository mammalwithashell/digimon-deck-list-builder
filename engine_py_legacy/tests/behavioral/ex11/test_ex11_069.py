"""Behavioral tests for EX11-069 Yuuki (Tamer, Purple/Red, LIBERATOR).

Card text:
  [Start of Your Main Phase] [On Play] By trashing 1 card in your hand,
      gain 1 memory.
  [Your Turn] [Once Per Turn] When one of your Digimon attacks, if you
      have 4 or fewer cards in your hand, it may digivolve into a
      [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash
      with the digivolution cost reduced by 1.
  [End of All Turns] If you have 4 or fewer cards in your hand, by
      suspending this Tamer, you may return 1 [Evil], [Dark Dragon] or
      [Evil Dragon] trait card from your trash to the hand.

  [Security] Play this card without paying the cost.

C# reference:
  - Security: PlaySelfTamerSecurityEffect
  - SoYMP/OP: canNoSelect=true (optional), trash 1 to gain 1 memory
  - Your Turn: OnAllyAttack timing (NOT OnUseAttack) because the Tamer
    observes a Digimon attacking. Uses DigivolveIntoHandOrTrashCard with
    payCost=true, reduceCostTuple=(1, null).
  - End of All Turns: OnEndTurn, suspend cost, then select from trash
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# ── Helpers ──────────────────────────────────────────────────────────────

YUUKI = "EX11-069"
GROWLMON_BT2 = "BT2-013"  # Lv.4 Dark Dragon, play_cost=5, evo_cost=2
EVIL_LV3 = "BT2-067"      # Lv.3 Evil trait
EVIL_DRAGON = "BT3-081"    # Lv.4 Evil Dragon trait


def _get_yuuki_effects(runner, player_id=1):
    """Return Yuuki's card and all its effects."""
    game = runner.game
    player = game.player1 if player_id == 1 else game.player2
    for perm in player.battle_area:
        if perm.is_tamer and perm.top_card:
            card_id = getattr(perm.top_card.c_entity_base, 'card_id', '')
            if card_id == YUUKI:
                return perm, perm.top_card, perm.top_card.effect_list(None)
    return None, None, []


# ── Security effect ──────────────────────────────────────────────────────

@pytest.mark.behavioral
class TestEX11069Security:
    """[Security] Play this card without paying the cost."""

    def test_has_security_effect(self, debug_runner):
        """Yuuki should have a security effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        security = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security) >= 1, "Yuuki should have at least 1 security effect"


# ── SoYMP / On Play: Trash 1 to gain 1 memory ──────────────────────────

@pytest.mark.behavioral
class TestEX11069TrashForMemory:
    """[Start of Your Main Phase] [On Play] By trashing 1 card in your hand, gain 1 memory."""

    def test_soymp_timing(self, debug_runner):
        """SoYMP effect must use OnStartMainPhase timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnStartMainPhase)
        soymp = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(soymp) >= 1, "Should have a SoYMP effect"

    def test_soymp_only_on_own_turn(self, debug_runner):
        """SoYMP condition should only pass on the owner's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnStartMainPhase)
        soymp = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # P1's turn - should pass
        assert runner.game.player1.is_my_turn
        assert soymp.can_use_condition({}), "SoYMP should pass on own turn"

    def test_soymp_trash_card_gains_memory(self, debug_runner):
        """Trashing a card from hand should gain 1 memory."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        # Ensure P1 has cards in hand
        initial_hand = len(runner.game.player1.hand_cards)
        assert initial_hand > 0, "P1 should have cards in hand for the test"

        initial_memory = runner.game.memory

        # Trigger SoYMP by advancing to main phase start
        # The SoYMP effects trigger automatically when the phase starts
        # We can test the process callback directly
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnStartMainPhase)
        soymp = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        # Process the effect
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        }
        soymp.on_process_callback(ctx)

        # Should be in a selection phase for hand card
        assert runner.game.current_phase == GamePhase.SelectHand, (
            f"Should be in SelectHand phase, got {runner.game.current_phase}"
        )

        # Select first legal action (pick a card to trash)
        legal = runner.action_mask()
        assert len(legal) > 0, "Should have legal actions to pick a hand card"

        # Execute the selection
        runner.execute(legal[0])

        # Verify: hand decreased by 1, trash increased by 1, memory increased by 1
        assert len(runner.game.player1.hand_cards) == initial_hand - 1, (
            "Hand should have 1 fewer card"
        )
        assert runner.game.memory == initial_memory + 1, (
            f"Memory should increase by 1: was {initial_memory}, now {runner.game.memory}"
        )

    def test_soymp_is_optional(self, debug_runner):
        """The trash-for-memory effect should be optional (can decline)."""
        runner = debug_runner(initial_memory=3)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnStartMainPhase)
        soymp = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase][0]

        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        }
        soymp.on_process_callback(ctx)

        # Should have a "Pass" action available (is_optional=True)
        actions = runner.actions()
        pass_action = runner.find_action("pass")
        assert pass_action is not None, (
            f"Should have a Pass action (optional). Actions: {actions}"
        )

    def test_on_play_has_correct_flags(self, debug_runner):
        """The On Play effect must have is_on_play=True."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects if getattr(e, 'is_on_play', False)]
        assert len(on_play) >= 1, "Should have an On Play effect"


# ── Your Turn: Digivolve from trash ─────────────────────────────────────

@pytest.mark.behavioral
class TestEX11069DigivolveFromTrash:
    """[Your Turn] [Once Per Turn] When one of your Digimon attacks,
    digivolve into [Dark Dragon]/[Evil Dragon] from trash, cost -1."""

    def test_uses_on_ally_attack_timing(self, debug_runner):
        """The tamer observes attacks, so it MUST use OnAllyAttack (not OnUseAttack).
        OnUseAttack only fires on the attacker's own effects."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)

        # Should have an OnAllyAttack effect
        ally_attack_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        assert len(ally_attack_effects) >= 1, (
            "Yuuki must have an OnAllyAttack timing effect for the digivolve-from-trash ability"
        )

    def test_does_not_use_on_use_attack_timing(self, debug_runner):
        """Yuuki is a Tamer, not the attacker. OnUseAttack would never fire."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)

        use_attack_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnUseAttack
            and e.is_on_attack
        ]
        assert len(use_attack_effects) == 0, (
            "Yuuki must NOT have OnUseAttack timing for the digivolve effect "
            "(it's a Tamer observing attacks, not the attacker)"
        )

    def test_once_per_turn(self, debug_runner):
        """The effect should be Once Per Turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        assert len(ally_effects) >= 1
        effect = ally_effects[0]
        assert effect.max_count_per_turn == 1, (
            f"Effect should be once per turn, got max_count={effect.max_count_per_turn}"
        )

    def test_condition_requires_4_or_fewer_hand_cards(self, debug_runner):
        """Condition should fail if more than 4 cards in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        # Ensure player has > 4 cards
        player = runner.game.player1
        while len(player.hand_cards) <= 4:
            runner.inject_card(1, "ST1-03", "hand")
        assert len(player.hand_cards) > 4

        assert not effect.can_use_condition({}), (
            "Condition should fail with > 4 hand cards"
        )

    def test_condition_passes_with_4_cards(self, debug_runner):
        """Condition should pass if exactly 4 cards in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        # Set hand to exactly 4 cards
        player = runner.game.player1
        while len(player.hand_cards) > 4:
            player.hand_cards.pop()
        while len(player.hand_cards) < 4:
            runner.inject_card(1, "ST1-03", "hand")
        assert len(player.hand_cards) == 4

        assert effect.can_use_condition({}), (
            "Condition should pass with exactly 4 hand cards"
        )

    def test_condition_passes_with_fewer_than_4_cards(self, debug_runner):
        """Condition should pass if fewer than 4 cards in hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        # Set hand to 2 cards
        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()
        assert len(player.hand_cards) <= 4

        assert effect.can_use_condition({}), (
            "Condition should pass with fewer than 4 hand cards"
        )

    def test_condition_requires_own_turn(self, debug_runner):
        """The effect should only fire on your turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        # Reduce hand size
        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        # Switch to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        assert not effect.can_use_condition({}), (
            "Condition should fail on opponent's turn"
        )

    def test_digivolve_uses_attacker_context(self, debug_runner):
        """Process callback must use ctx['attacker'] for the attacking permanent,
        not ctx['permanent'] (which is the Tamer itself for OnAllyAttack)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Yuuki on field
        yuuki_perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        # Place an attacking Digimon (Lv.4 generic)
        attacker_perm = runner.place_on_field(1, ["ST1-07"], turn_played=-1)

        # Put a Dark Dragon Digimon in trash (BT2-013 Growlmon, Lv.4)
        runner.inject_card(1, GROWLMON_BT2, "trash")

        # Reduce hand to <= 4
        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = yuuki_perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        assert len(ally_effects) >= 1
        effect = ally_effects[0]

        # Build the context as the engine would for OnAllyAttack
        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': yuuki_perm,  # the Tamer (not the attacker!)
            'attacker': attacker_perm,  # the actual attacking Digimon
        }

        # Process the effect
        initial_trash = len(player.trash_cards)
        effect.on_process_callback(ctx)

        # Should be in selection phase for trash card
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            f"Should be in SelectTarget phase to pick from trash, got {runner.game.current_phase}"
        )

    def test_digivolve_cost_reduced_by_1(self, debug_runner):
        """Digivolving should use the digivolution cost minus 1."""
        from digimon_gym.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        yuuki_perm = runner.place_on_field(1, [YUUKI], turn_played=-1)
        # ST1-07 is a Lv.4 Digimon
        attacker_perm = runner.place_on_field(1, ["ST1-07"], turn_played=-1)

        # Put BT2-013 (Growlmon, evo_cost=2) in trash
        runner.inject_card(1, GROWLMON_BT2, "trash")

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = yuuki_perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        memory_before = runner.game.memory
        hand_before = len(player.hand_cards)
        trash_before = len(player.trash_cards)

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': yuuki_perm,
            'attacker': attacker_perm,
        }
        effect.on_process_callback(ctx)

        # Select the Growlmon from trash (not the Pass action)
        legal = runner.action_mask()
        assert len(legal) > 0, "Should have legal actions to select from trash"
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) > 0, (
            f"Should have trash selection actions. Legal: {legal}"
        )
        runner.execute(trash_actions[0])

        # BT2-013 evo_cost = 2, reduced by 1 = 1
        # Memory should decrease by 1
        expected_cost = 1  # evo_cost 2 - 1 reduction
        assert runner.game.memory == memory_before - expected_cost, (
            f"Memory should decrease by {expected_cost} (evo_cost 2 - 1 reduction). "
            f"Was {memory_before}, now {runner.game.memory}"
        )

        # Growlmon should now be on top of the attacker stack
        top_card_id = getattr(attacker_perm.top_card.c_entity_base, 'card_id', '')
        assert top_card_id == GROWLMON_BT2, (
            f"Growlmon should be on top of the attacker. Got: {top_card_id}"
        )

        # Should have drawn 1 card (digivolution draw)
        assert len(player.hand_cards) == hand_before + 1, (
            "Should draw 1 card from digivolving"
        )

    def test_is_optional(self, debug_runner):
        """The digivolve effect should be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        assert len(ally_effects) >= 1
        assert ally_effects[0].is_optional, "Digivolve effect should be optional"

    def test_only_dark_dragon_or_evil_dragon_digimon(self, debug_runner):
        """Should only offer [Dark Dragon] or [Evil Dragon] trait Digimon cards from trash."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        yuuki_perm = runner.place_on_field(1, [YUUKI], turn_played=-1)
        attacker_perm = runner.place_on_field(1, ["ST1-07"], turn_played=-1)

        # Put a non-qualifying card in trash (vanilla Lv.3, no matching trait)
        runner.inject_card(1, "ST1-03", "trash")

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = yuuki_perm.top_card
        effects = card.effect_list(None)
        ally_effects = [
            e for e in effects
            if getattr(e, 'timing', None) == EffectTiming.OnAllyAttack
            and e.is_on_attack
        ]
        effect = ally_effects[0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': yuuki_perm,
            'attacker': attacker_perm,
        }
        effect.on_process_callback(ctx)

        # Should NOT enter selection (no qualifying cards in trash)
        assert runner.game.current_phase != GamePhase.SelectTarget, (
            "Should not offer selection when no Dark Dragon/Evil Dragon Digimon in trash"
        )


# ── End of All Turns: Recover from trash ─────────────────────────────────

@pytest.mark.behavioral
class TestEX11069EndOfTurnRecover:
    """[End of All Turns] If 4 or fewer hand cards, suspend this Tamer,
    return 1 [Evil]/[Dark Dragon]/[Evil Dragon] card from trash to hand."""

    def test_end_turn_timing(self, debug_runner):
        """Should use OnEndTurn timing."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn]
        assert len(end_turn) >= 1, "Should have an OnEndTurn effect"

    def test_condition_requires_4_or_fewer_hand_cards(self, debug_runner):
        """Condition should fail with > 4 hand cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        player = runner.game.player1
        while len(player.hand_cards) <= 4:
            runner.inject_card(1, "ST1-03", "hand")
        assert len(player.hand_cards) > 4

        assert not end_turn.can_use_condition({}), (
            "Condition should fail with > 4 hand cards"
        )

    def test_condition_requires_not_suspended(self, debug_runner):
        """Condition should fail if Yuuki is already suspended."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        perm.suspend()

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        # Reduce hand
        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        assert not end_turn.can_use_condition({}), (
            "Condition should fail when Tamer is already suspended"
        )

    def test_condition_passes_when_active_and_4_or_fewer(self, debug_runner):
        """Condition should pass when not suspended and <= 4 hand cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        player = runner.game.player1
        while len(player.hand_cards) > 3:
            player.hand_cards.pop()
        assert len(player.hand_cards) <= 4

        assert not perm.is_suspended, "Tamer should not be suspended"
        assert end_turn.can_use_condition({}), (
            "Condition should pass when not suspended and <= 4 cards"
        )

    def test_suspends_tamer_on_process(self, debug_runner):
        """Processing the effect should suspend Yuuki."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        # Put an Evil trait card in trash
        runner.inject_card(1, EVIL_LV3, "trash")

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': perm,
        }

        assert not perm.is_suspended
        end_turn.on_process_callback(ctx)
        assert perm.is_suspended, "Yuuki should be suspended after processing"

    def test_returns_qualifying_card_from_trash(self, debug_runner):
        """Should return a qualifying trait card from trash to hand."""
        from digimon_gym.engine.game.constants import SEL_TRASH_START

        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        # Put an Evil trait card in trash
        runner.inject_card(1, EVIL_LV3, "trash")

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()
        hand_before = len(player.hand_cards)
        trash_before = len(player.trash_cards)

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)

        # Select the trash card (not the Pass action)
        legal = runner.action_mask()
        assert len(legal) > 0, "Should have actions to select from trash"
        # Find the trash selection action (starts at SEL_TRASH_START)
        trash_actions = [a for a in legal if a >= SEL_TRASH_START]
        assert len(trash_actions) > 0, (
            f"Should have trash selection actions. Legal: {legal}"
        )
        runner.execute(trash_actions[0])

        # Card should move from trash to hand
        assert len(player.hand_cards) == hand_before + 1, (
            "Hand should gain 1 card"
        )
        assert len(player.trash_cards) == trash_before - 1, (
            "Trash should lose 1 card"
        )

    def test_is_optional(self, debug_runner):
        """The end-of-turn recovery effect should be optional."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [YUUKI])
        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]
        assert end_turn.is_optional, "End of turn recovery should be optional"

    def test_trait_filter_accepts_evil(self, debug_runner):
        """Should accept cards with [Evil] trait."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        runner.inject_card(1, EVIL_LV3, "trash")  # Evil trait

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)

        # Should enter selection (Evil card qualifies)
        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should offer selection for Evil trait card"
        )

    def test_trait_filter_accepts_dark_dragon(self, debug_runner):
        """Should accept cards with [Dark Dragon] trait."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        runner.inject_card(1, GROWLMON_BT2, "trash")  # Dark Dragon trait

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)

        assert runner.game.current_phase == GamePhase.SelectTarget, (
            "Should offer selection for Dark Dragon trait card"
        )

    def test_trait_filter_rejects_non_qualifying(self, debug_runner):
        """Should NOT offer cards without Evil/Dark Dragon/Evil Dragon trait."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")
        perm = runner.place_on_field(1, [YUUKI], turn_played=-1)

        # Put a non-qualifying card in trash
        runner.inject_card(1, "ST1-03", "trash")

        player = runner.game.player1
        while len(player.hand_cards) > 2:
            player.hand_cards.pop()

        card = perm.top_card
        effects = card.effect_list(EffectTiming.OnEndTurn)
        end_turn = [e for e in effects if e.timing == EffectTiming.OnEndTurn][0]

        ctx = {
            'game': runner.game,
            'player': player,
            'permanent': perm,
        }
        end_turn.on_process_callback(ctx)

        # Should NOT enter selection
        assert runner.game.current_phase != GamePhase.SelectTarget, (
            "Should not offer selection for non-qualifying cards"
        )
