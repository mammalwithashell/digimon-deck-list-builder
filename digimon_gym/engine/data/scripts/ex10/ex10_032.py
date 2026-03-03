from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_032(CardScript):
    """EX10-032 Proganomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("EX10-032 Place 1 [Landramon] from trash under 1 [Sunarizamon], to digivolve for 3")
        effect0.set_effect_description(
            "[Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash as any of your [Sunarizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect0.effect_source_permanent if hasattr(effect0, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Close') or permanent.contains_card_name('Sunarizamon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                return True

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        def build_grant_effect(is_on_play: bool = False, is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            timing = EffectTiming.OnEnterFieldAnyone if not is_on_attack else EffectTiming.OnAllyAttack
            effect.set_timing(timing)
            effect.set_effect_name("EX10-032 By trashing 1 source, 1 digimon gains Collision, Piercing, +3K DP")
            effect.set_effect_description("DP +3000, Trash Digivolution Cards, Gain Keyword Collision, Gain Keyword Piercing")
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack
            effect._is_collision = True
            effect._is_piercing = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return
                if not perm.has_no_digivolution_cards:
                    trashed = perm.trash_digivolution_cards(1)
                    player.trash_cards.extend(trashed)

                def target_filter(p):
                    return p.is_digimon

                def on_target(target_perm):
                    target_perm.change_dp(3000)
                    target_perm.grant_keyword('_is_collision')
                    target_perm.grant_keyword('_is_piercing')

                game.effect_select_own_permanent(
                    player, on_target, filter_fn=target_filter, is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_grant_effect(is_on_play=True))
        effects.append(build_grant_effect(is_when_digivolving=True))
        effects.append(build_grant_effect(is_on_attack=True))

        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect4.set_effect_name("EX10-032 De-Digivolve 1")
        effect4.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon."
        )
        effect4.is_inherited_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
