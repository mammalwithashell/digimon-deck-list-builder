from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_072(CardScript):
    """EX9-072 File Island | Option Cost 2 | DM
    While no face-up security, ignore color requirements.
    [Security][All Turns] All [DM] trait Digimon get +1000 DP per face-down digi card.
    [Main] Add bottom security card to hand. Then, place this card face up as
    bottom security card.
    [Security] Play 1 cost 5 or lower [DM] from hand or trash free.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Add bottom security to hand, place this as bottom security
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX9-072 Main: Swap security, place as bottom security")
        effect0.set_effect_description(
            "[Main] Add bottom security card to hand. Then, place this card "
            "face up as bottom security card."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            # Add bottom security card to hand
            if player.security_cards:
                bottom_card = player.security_cards.pop(0)
                player.hand_cards.append(bottom_card)
            # Place this card as bottom security (face up)
            if card:
                player.security_cards.insert(0, card)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [All Turns] (when in security) DM Digimon get +1000 DP per face-down digi
        # This is a continuous effect that applies when this card is in security
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("EX9-072 All Turns: DM +1000 DP per face-down")
        effect1.set_effect_description(
            "[Security][All Turns] All [DM] Digimon get +1000 DP per face-down digi card."
        )
        # This continuous effect is descriptive -- the engine doesn't easily
        # support security-resident continuous modifiers
        effect1._dm_dp_per_facedown = 1000

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [Security] Play 1 cost 5 or lower DM from hand or trash free
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX9-072 Security: Play DM cost 5 or lower")
        effect2.set_effect_description(
            "[Security] Play 1 cost 5 or lower [DM] from hand or trash free."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def dm_cost5_filter(c):
                cost = c.get_cost_itself if hasattr(c, 'get_cost_itself') else (getattr(c, 'play_cost', 99) or 99)
                if cost > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('DM' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand_or_trash', dm_cost5_filter, free=True, is_optional=True,
                prompt="Play 1 cost 5 or lower [DM] from hand or trash.")
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
