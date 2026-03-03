from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_022(CardScript):
    """EX6-022 Angewomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-022 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def build_main_effect(is_on_play: bool = False, is_when_digivolving: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("EX6-022 Give a Digimon Security Attack -2 or play [Mirei Mikagura] from hand")
            effect.set_effect_description(
                "[On Play] If you have a [Mirei Mikagura], 1 of your opponent's Digimon gains Security Attack -2 until the end of their turn. If you don't have a [Mirei Mikagura], you may play 1 [Mirei Mikagura] from your hand without paying the cost."
                if is_on_play else
                "[When Digivolving] If you have a [Mirei Mikagura], 1 of your opponent's Digimon gains Security Attack -2 until the end of their turn. If you don't have a [Mirei Mikagura], you may play 1 [Mirei Mikagura] from your hand without paying the cost."
            )
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                has_mirei = any(
                    p.is_tamer
                    and p.top_card
                    and any('Mirei Mikagura' in name for name in getattr(p.top_card, 'card_names', []))
                    for p in player.battle_area
                )

                if has_mirei:
                    def target_filter(p):
                        return p.is_digimon

                    def on_target(target_perm):
                        # Approximated as a temporary modifier on the permanent.
                        target_perm._temp_sa_modifier -= 2

                    game.effect_select_opponent_permanent(
                        player, on_target, filter_fn=target_filter, is_optional=False)
                    return

                def play_filter(hand_card):
                    if not getattr(hand_card, 'is_tamer', False):
                        return False
                    return any('Mirei Mikagura' in name for name in getattr(hand_card, 'card_names', []))

                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_main_effect(is_on_play=True))
        effects.append(build_main_effect(is_when_digivolving=True))

        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-022 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
