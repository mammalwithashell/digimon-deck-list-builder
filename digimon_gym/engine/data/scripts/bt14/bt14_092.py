from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_092(CardScript):
    """BT14-092 Marching Fishes"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Choose 1 of your Digimon. Until the end of your opponent's turn,
        # 3 of your opponent's Digimon with as many or fewer digivolution cards as that Digimon can't attack or block.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-092 Gain Keyword Cannot Attack, Gain Keyword Cannot Block")
        effect0.set_effect_description("[Main] Choose 1 of your Digimon. Until the end of your opponent's turn, 3 of your opponent's Digimon with as many or fewer digivolution cards as that Digimon can't attack or block.")
        effect0._is_cannot_attack = True
        effect0._is_cannot_block = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_select_reference(ref_perm):
                ref_digi_count = len(ref_perm.digivolution_cards)

                def target_filter(p):
                    return p.is_digimon and len(p.digivolution_cards) <= ref_digi_count

                selected = {'count': 0}

                def on_grant(target_perm):
                    if selected['count'] >= 3:
                        return
                    target_perm.grant_keyword('_is_cannot_attack')
                    target_perm.grant_keyword('_is_cannot_block')
                    selected['count'] += 1

                for _ in range(3):
                    game.effect_select_opponent_permanent(
                        player,
                        on_grant,
                        filter_fn=target_filter,
                        is_optional=True,
                    )

            game.effect_select_own_permanent(
                player,
                on_select_reference,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] 1 of your opponent's Digimon can't attack for the turn. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-092 Add To Hand, Gain Keyword Cannot Attack")
        effect1.set_effect_description("[Security] 1 of your opponent's Digimon can't attack for the turn. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1._is_cannot_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            source_card = ctx.get('card')
            if not (player and game):
                return

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_attack')

            game.effect_select_opponent_permanent(
                player,
                on_grant,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
            )

            if source_card is not None:
                player.hand_cards.append(source_card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
