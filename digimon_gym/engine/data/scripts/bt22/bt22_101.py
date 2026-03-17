from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_101(CardScript):
    """BT22-101 Kyoko Kuremi"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT22-101 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When any of your level 4 or higher [CS] trait Digimon are deleted, by suspending this Tamer, return 1 [CS] trait Digimon card from your trash to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT22-101 Suspend tamer, add 1 [CS] digimon from trash to hand")
        effect1.set_effect_description("[All Turns] When any of your level 4 or higher [CS] trait Digimon are deleted, by suspending this Tamer, return 1 [CS] trait Digimon card from your trash to the hand.")
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            # Tamer must be unsuspended (suspending is the cost)
            if tamer_perm.is_suspended:
                return False
            # The deleted permanent must be our own Lv4+ CS trait Digimon
            deleted_perm = context.get('permanent')
            if not deleted_perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must be our Digimon (check via context event_player or owner)
            event_player = context.get('event_player')
            if event_player and event_player is not player:
                return False
            if not deleted_perm.is_digimon:
                return False
            level = deleted_perm.level
            if level is None or level < 4:
                return False
            top = deleted_perm.top_card
            if top:
                traits = getattr(top, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, return 1 CS Digimon from trash to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
            # Select 1 CS trait Digimon from trash to return to hand
            from ....data.enums import GamePhase
            valid = []
            for i, tc in enumerate(player.trash_cards):
                if getattr(tc, 'is_digimon', False):
                    traits = getattr(tc, 'card_traits', []) or []
                    if any('CS' in t for t in traits):
                        from ....game.effects import SEL_TRASH_START
                        idx = SEL_TRASH_START + i
                        valid.append(idx)
            if not valid:
                return

            def on_select(action_id):
                from ....game.effects import SEL_TRASH_START
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    selected = player.trash_cards.pop(idx)
                    player.hand_cards.append(selected)

            game.request_selection(
                GamePhase.SelectTrash, player, on_select, valid, is_optional=False,
                prompt="Select 1 [CS] trait Digimon card from trash to return to hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUnTappedAnyone
        # [Your Turn] [Once Per Turn] When this Tamer unsuspends, if you don't have [Alphamon], this Tamer may digivolve into [Alphamon] in the hand with the digivolution cost reduced by 2.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name("BT22-101 Digivolve into [Alphamon] for 2 reducded cost")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When this Tamer unsuspends, if you don't have [Alphamon], this Tamer may digivolve into [Alphamon] in the hand with the digivolution cost reduced by 2.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_101_Digivolve")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-101 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
