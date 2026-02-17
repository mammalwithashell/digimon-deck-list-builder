from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_068(CardScript):
    """P-068 Herissmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.SecuritySkill
        # [Security] At the end of the battle, 1 of your opponent's Digimon gains <Security Attack -1> for the turn. (This Digimon checks 1 fewer security cards.) Then, add this card to its owner�f hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-068 Opponent's 1 Digimon gains Security Attack -1 and add this card to hand")
        effect0.set_effect_description("[Security] At the end of the battle, 1 of your opponent's Digimon gains <Security Attack -1> for the turn. (This Digimon checks 1 fewer security cards.) Then, add this card to its owner�f hand.")
        effect0.is_security_effect = True
        effect0.is_security_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # 1 of your opponent's Digimon gains <Security Attack -1> for the turn. (This Digimon checks 1 fewer security cards.) Then, add this card to its owner�f hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-068 Opponent's 1 Digimon gains Security Attack -1 and add this card to hand")
        effect1.set_effect_description("1 of your opponent's Digimon gains <Security Attack -1> for the turn. (This Digimon checks 1 fewer security cards.) Then, add this card to its owner�f hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
