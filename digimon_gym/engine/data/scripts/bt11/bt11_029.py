from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_029(CardScript):
    """BT11-029 AeroVeedramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main][Once Per Turn] By suspending this Digimon, reveal the top 3 cards of your deck. Add all blue Tamer cards among them to your hand. Place the rest at the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0._is_field_main = True
        effect0.set_effect_name("BT11-029 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[Main][Once Per Turn] By suspending this Digimon, reveal the top 3 cards of your deck. Add all blue Tamer cards among them to your hand. Place the rest at the bottom of your deck in any order.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Reveal3_BT11_029")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if not ('Blue' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once Per Turn] Activate 1 of your [Rina Shinomiya]'s [On Play] effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT11-029 Activate [On Play] effect")
        effect1.set_effect_description("[When Attacking][Once Per Turn] Activate 1 of your [Rina Shinomiya]'s [On Play] effects.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Activate_BT11_029")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
