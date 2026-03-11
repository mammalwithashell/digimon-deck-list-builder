from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST9_06(CardScript):
    """ST9-06 Imperialdramon Dragon Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] You may play 1 level 4 or lower blue Digimon card and
        # 1 level 4 or lower green Digimon card from this Digimon's digivolution cards
        # without paying their memory costs.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST9-06 Play Blue Lv4 and Green Lv4 from digi-stack")
        effect0.set_effect_description(
            "[When Digivolving] You may play 1 level 4 or lower blue Digimon card and "
            "1 level 4 or lower green Digimon card from this Digimon's digivolution cards "
            "without paying their memory costs."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            top = perm.top_card
            has_blue = any(
                cs is not top
                and getattr(cs, 'is_digimon', False)
                and (getattr(cs, 'level', None) or 0) <= 4
                and getattr(cs, 'level', None) is not None
                and any(
                    getattr(col, 'name', str(col)) == 'Blue'
                    for col in (getattr(cs, 'card_colors', None) or [])
                )
                for cs in perm.card_sources
            )
            has_green = any(
                cs is not top
                and getattr(cs, 'is_digimon', False)
                and (getattr(cs, 'level', None) or 0) <= 4
                and getattr(cs, 'level', None) is not None
                and any(
                    getattr(col, 'name', str(col)) == 'Green'
                    for col in (getattr(cs, 'card_colors', None) or [])
                )
                for cs in perm.card_sources
            )
            return has_blue or has_green

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            top = perm.top_card

            # Find and play 1 Blue Lv.4-or-lower Digimon from digi-stack
            for cs in list(perm.card_sources):
                if cs is top:
                    continue
                if not getattr(cs, 'is_digimon', False):
                    continue
                level = getattr(cs, 'level', None)
                if level is None or level > 4:
                    continue
                colors = [
                    getattr(col, 'name', str(col))
                    for col in (getattr(cs, 'card_colors', None) or [])
                ]
                if 'Blue' not in colors:
                    continue
                # Remove from digi-stack then play
                perm.card_sources.remove(cs)
                played = player.play_card_from_source(cs, pay_cost=False)
                if played:
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {
                            'played_card': cs,
                            'played_permanent': played,
                            'event_player': player,
                        },
                    )
                break

            # Find and play 1 Green Lv.4-or-lower Digimon from digi-stack
            for cs in list(perm.card_sources):
                if cs is top:
                    continue
                if not getattr(cs, 'is_digimon', False):
                    continue
                level = getattr(cs, 'level', None)
                if level is None or level > 4:
                    continue
                colors = [
                    getattr(col, 'name', str(col))
                    for col in (getattr(cs, 'card_colors', None) or [])
                ]
                if 'Green' not in colors:
                    continue
                # Remove from digi-stack then play
                perm.card_sources.remove(cs)
                played = player.play_card_from_source(cs, pay_cost=False)
                if played:
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {
                            'played_card': cs,
                            'played_permanent': played,
                            'event_player': player,
                        },
                    )
                break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
