from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_026(CardScript):
    """EX9-026 Angemon | Lv.4 | Angel/DM/Ver.2
    <Training>
    [On Play][When Digivolving] By placing 1 hand card as bottom digi card,
    give 1 opponent Digimon SA-1 and -3000 DP until their turn ends.
    Inherited: [On Deletion] If 3 or fewer security, Recovery +1 (Deck).
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-026 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Play] Place hand card, give SA-1 and -3000 DP
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-026 On Play: Place hand, SA-1 and -3000 DP")
        effect1.set_effect_description(
            "[On Play] Place 1 hand card as bottom digi card, give 1 opponent "
            "Digimon SA-1 and -3000 DP."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def _place_and_debuff(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.hand_cards:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)

            def target_filter(p):
                return p.is_digimon

            def on_debuff(target_perm):
                target_perm.security_attack_modifier = getattr(
                    target_perm, 'security_attack_modifier', 0) - 1
                target_perm.dp_modifier -= 3000

            game.effect_select_opponent_permanent(
                player, on_debuff, filter_fn=target_filter, is_optional=False,
                prompt="Select opponent Digimon to give SA-1 and -3000 DP.")

        effect1.set_on_process_callback(_place_and_debuff)
        effects.append(effect1)

        # [When Digivolving] Same effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-026 When Digivolving: Place hand, SA-1 and -3000 DP")
        effect2.set_effect_description(
            "[When Digivolving] Place 1 hand card as bottom digi card, give 1 "
            "opponent Digimon SA-1 and -3000 DP."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_debuff)
        effects.append(effect2)

        # Inherited: [On Deletion] If 3 or fewer security, Recovery +1
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX9-026 Inherited: On Deletion Recovery +1")
        effect3.set_effect_description(
            "Inherited: [On Deletion] If 3 or fewer security, Recovery +1 (Deck)."
        )
        effect3.is_inherited_effect = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.owner:
                if len(card.owner.security_cards) > 3:
                    return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and player.library_cards:
                top_card = player.library_cards.pop(0)
                player.security_cards.append(top_card)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
