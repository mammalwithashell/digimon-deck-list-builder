from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_070(CardScript):
    """BT17-070 Gulfmon | Lv.6 Purple | Dark Animal, Virus | 11000 DP | Cost 12

    [On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters]
        in its text from your hand or trash as this Digimon's bottom digivolution
        card, delete 1 of your opponent's level 5 or lower Digimon.
    [When Attacking] By returning 7 cards from your opponent's trash to the
        bottom of the deck, unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared filter for placing Dark Masters Lv.5 card ---
        def _is_dark_masters_lv5(c) -> bool:
            level = getattr(c, 'level', None)
            if level != 5:
                return False
            text = getattr(c, 'card_text', '') or ''
            return 'Dark Masters' in text

        def _has_dark_masters_in_hand_or_trash() -> bool:
            player = card.owner if card else None
            if not player:
                return False
            for c in player.hand_cards:
                if _is_dark_masters_lv5(c):
                    return True
            for c in player.trash_cards:
                if _is_dark_masters_lv5(c):
                    return True
            return False

        # --- Shared process for On Play / When Digivolving ---
        def _place_digi_and_delete(ctx: Dict[str, Any]):
            """Place 1 Lv.5 Dark Masters card as bottom digi, then delete opp Lv.5- Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Find qualifying card in hand first, then trash
            target = None
            source = None
            for c in player.hand_cards:
                if _is_dark_masters_lv5(c):
                    target = c
                    source = 'hand'
                    break
            if not target:
                for c in player.trash_cards:
                    if _is_dark_masters_lv5(c):
                        target = c
                        source = 'trash'
                        break
            if not target:
                return

            # Place as bottom digivolution card
            if source == 'hand':
                player.hand_cards.remove(target)
            else:
                player.trash_cards.remove(target)
            perm.add_card_source_bottom(target)

            # Delete 1 of opponent's Lv.5 or lower Digimon
            enemy = player.enemy
            if not enemy:
                return

            def delete_filter(p):
                return (p.is_digimon and p.level is not None and p.level <= 5)

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False)

        # --- Effect 0: [On Play] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-070 On Play: Place Dark Masters, delete opp Lv.5-")
        effect0.set_effect_description(
            "[On Play] By placing 1 level 5 card with [Dark Masters] in its "
            "text from your hand or trash as this Digimon's bottom digivolution "
            "card, delete 1 of your opponent's level 5 or lower Digimon."
        )
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_dark_masters_in_hand_or_trash()
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_place_digi_and_delete)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-070 When Digivolving: Place Dark Masters, delete opp Lv.5-")
        effect1.set_effect_description(
            "[When Digivolving] By placing 1 level 5 card with [Dark Masters] "
            "in its text from your hand or trash as this Digimon's bottom "
            "digivolution card, delete 1 of your opponent's level 5 or lower "
            "Digimon."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_dark_masters_in_hand_or_trash()
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_place_digi_and_delete)
        effects.append(effect1)

        # --- Effect 2: [When Attacking] Return 7 cards from opponent's trash to deck bottom, unsuspend ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT17-070 When Attacking: Return 7 opp trash, unsuspend")
        effect2.set_effect_description(
            "[When Attacking] By returning 7 cards from your opponent's trash "
            "to the bottom of the deck, unsuspend this Digimon."
        )
        effect2.is_on_attack = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            return len(enemy.trash_cards) >= 7
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Return 7 cards from opponent's trash to deck bottom, unsuspend self."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy or len(enemy.trash_cards) < 7:
                return
            # Return 7 cards to deck bottom
            for _ in range(7):
                if enemy.trash_cards:
                    c = enemy.trash_cards.pop(0)
                    enemy.library_cards.append(c)
            # Unsuspend this Digimon
            perm.unsuspend()
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
