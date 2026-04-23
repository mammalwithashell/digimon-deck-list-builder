from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_026(CardScript):
    """BT14-026 Zudomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-026 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash any 2 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with no digivolution cards to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT14-026 Trash digivolution cards and return 1 Digimon to hand")
        effect1.set_effect_description("[On Play] Trash any 2 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with no digivolution cards to the hand.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash any 2 opponent digivolution cards (can be split across Digimon)
            for _ in range(2):
                def can_trash_source(p):
                    return not p.has_no_digivolution_cards

                def on_trash_source(target_perm):
                    trashed = target_perm.trash_digivolution_cards(1)
                    if player:
                        player.trash_cards.extend(trashed)

                game.effect_select_opponent_permanent(
                    player,
                    on_trash_source,
                    filter_fn=can_trash_source,
                    is_optional=True,
                )

            # Then return 1 opponent Digimon with no digivolution cards to hand
            def can_bounce_target(p):
                return p.has_no_digivolution_cards

            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player,
                on_bounce,
                filter_fn=can_bounce_target,
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with no digivolution cards to the hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT14-026 Trash digivolution cards and return 1 Digimon to hand")
        effect2.set_effect_description("[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with no digivolution cards to the hand.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash any 2 opponent digivolution cards (can be split across Digimon)
            for _ in range(2):
                def can_trash_source(p):
                    return not p.has_no_digivolution_cards

                def on_trash_source(target_perm):
                    trashed = target_perm.trash_digivolution_cards(1)
                    if player:
                        player.trash_cards.extend(trashed)

                game.effect_select_opponent_permanent(
                    player,
                    on_trash_source,
                    filter_fn=can_trash_source,
                    is_optional=True,
                )

            # Then return 1 opponent Digimon with no digivolution cards to hand
            def can_bounce_target(p):
                return p.has_no_digivolution_cards

            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player,
                on_bounce,
                filter_fn=can_bounce_target,
                is_optional=True,
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
