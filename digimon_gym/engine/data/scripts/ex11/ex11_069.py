from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_069(CardScript):
    """EX11-069 Yuuki | Tamer Purple

    [Security] Play this card without paying the cost.
    [Start of Your Main Phase] [On Play] By trashing 1 card in your hand,
        gain 1 memory.
    [Your Turn] [Once Per Turn] When one of your Digimon attacks, if you
        have 4 or fewer cards in your hand, it may digivolve into a
        [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash
        with the digivolution cost reduced by 1.
    [End of All Turns] If you have 4 or fewer cards in your hand, by
        suspending this Tamer, you may return 1 [Evil], [Dark Dragon] or
        [Evil Dragon] trait card from your trash to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play this card without paying the cost ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-069 Security: Play this card")
        effect0.set_effect_description("[Security] Play this card without paying the cost.")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared: SoYMP / On Play trash-to-gain-memory ---
        def _shared_trash_for_memory(ctx: Dict[str, Any]):
            """By trashing 1 card from hand, gain 1 memory. Optional (by trashing = cost)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.hand_cards:
                return

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    # Gain 1 memory only if trashed
                    player.add_memory(1)

            game.effect_select_hand_card(
                player, lambda c: True, on_trashed,
                is_optional=True,
                prompt="Trash 1 card from hand to gain 1 memory.")

        # --- Effect 1: [Start of Your Main Phase] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX11-069 Trash 1 to gain 1 memory")
        effect1.set_effect_description(
            "[Start of Your Main Phase] By trashing 1 card in your hand, gain 1 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_trash_for_memory)
        effects.append(effect1)

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-069 Trash 1 to gain 1 memory")
        effect2.set_effect_description(
            "[On Play] By trashing 1 card in your hand, gain 1 memory.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_trash_for_memory)
        effects.append(effect2)

        # --- Effect 3: [Your Turn] [Once Per Turn] When attacking,
        #     digivolve into [Dark Dragon]/[Evil Dragon] from trash, cost -1 ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX11-069 Digivolve attacking Digimon from trash")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When one of your Digimon attacks, if "
            "you have 4 or fewer cards in your hand, it may digivolve into a "
            "[Dark Dragon] or [Evil Dragon] trait Digimon card in the trash with "
            "the digivolution cost reduced by 1.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_069_YT")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            owner = card.owner
            if len(owner.hand_cards) > 4:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Digivolve the attacking Digimon into a [Dark Dragon]/[Evil Dragon]
            from trash with cost reduced by 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')  # the attacking permanent
            game = ctx.get('game')
            if not (player and perm and game):
                return

            if len(player.hand_cards) > 4:
                return

            def trash_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Dragon' in t or 'Evil Dragon' in t for t in traits)

            candidates = [c for c in player.trash_cards if trash_digi_filter(c)]
            if not candidates:
                return

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards)
                     if trash_digi_filter(c)]
            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen = player.trash_cards[idx]
                base_cost = getattr(chosen, 'get_cost_itself', 0)
                if callable(base_cost):
                    base_cost = base_cost
                cost = max(0, base_cost - 1)
                player.trash_cards.remove(chosen)
                perm.add_card_source(chosen)
                perm.turn_digivolved = game.turn_count
                player.lose_memory(cost)
                game.logger.log(
                    f"[Effect Digivolve] {game._card_ref(chosen)} onto "
                    f"{game._perm_ref(perm)} (cost: {cost}, from trash)")
                player.draw()
                game.execute_effects(
                    EffectTiming.WhenDigivolving,
                    {"digivolved_permanent": perm})

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select a [Dark Dragon]/[Evil Dragon] Digimon from trash to digivolve into.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [End of All Turns] Suspend this Tamer, return 1
        #     [Evil]/[Dark Dragon]/[Evil Dragon] card from trash to hand ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("EX11-069 Suspend to recover trait card from trash")
        effect4.set_effect_description(
            "[End of All Turns] If you have 4 or fewer cards in your hand, by "
            "suspending this Tamer, you may return 1 [Evil], [Dark Dragon] or "
            "[Evil Dragon] trait card from your trash to the hand.")
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            owner = card.owner if card else None
            if not owner:
                return False
            if len(owner.hand_cards) > 4:
                return False
            # Must not already be suspended
            if perm and perm.is_suspended:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Suspend this Tamer, then select 1 qualifying card from trash
            to return to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: suspend this Tamer
            perm.suspend()

            def trash_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Evil' in t or 'Dark Dragon' in t or
                           'Evil Dragon' in t for t in traits)

            qualifying = [c for c in player.trash_cards if trash_filter(c)]
            if not qualifying:
                return

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards)
                     if trash_filter(c)]
            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen = player.trash_cards[idx]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 [Evil]/[Dark Dragon]/[Evil Dragon] card from trash to return to hand.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
