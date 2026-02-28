from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_066(CardScript):
    """EX5-066 Phoebus Blow"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's Digimon with the lowest DP. Then, if you have a Tamer, return 1 Digimon card with the [Light Fang]/[Night Claw]/[Galaxy] trait from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-066 Delete, Add To Hand")
        effect0.set_effect_description("[Main] Delete 1 of your opponent's Digimon with the lowest DP. Then, if you have a Tamer, return 1 Digimon card with the [Light Fang]/[Night Claw]/[Galaxy] trait from your trash to the hand.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Light Fang' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('LightFung' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Night Claw' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('NightClaw' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Galaxy' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-066 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
