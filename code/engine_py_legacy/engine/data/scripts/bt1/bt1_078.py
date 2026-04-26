from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_078(CardScript):
    """BT1-078 Jagamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Attacking] Reveal the top 3 cards of your deck. You can digivolve
        # this card into 1 level 6 green Digimon card among them without paying
        # its memory cost. Place the remaining cards at the bottom of your deck
        # in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.is_on_attack = True
        effect0.set_effect_name("BT1-078 When Attacking: Reveal top 3, digivolve into Lv.6 green")
        effect0.set_effect_description(
            "[When Attacking] Reveal the top 3 cards of your deck. You can "
            "digivolve this card into 1 level 6 green Digimon card among them "
            "without paying its memory cost. Place the remaining cards at the "
            "bottom of your deck in any order."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            from ....data.enums import CardColor

            def digivolve_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if (getattr(c, 'level', None) or 0) != 6:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Green in colors

            def on_selected(selected, remaining):
                # Digivolve perm into selected card without paying cost
                player.hand_cards.append(selected)  # temporarily in hand for digivolve
                perm.add_card_source(selected)
                player.hand_cards.remove(selected)
                if game:
                    perm.turn_digivolved = game.turn_count
                    game.logger.log(
                        f"[BT1-078] {player.player_name} digivolved into "
                        f"{selected.card_names[0]} from revealed cards."
                    )
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {
                            "played_card": selected,
                            "played_permanent": perm,
                            "event_permanent": perm,
                            "event_player": player,
                            "is_digivolving": True,
                        },
                    )
                # Remaining go to deck bottom
                for c in remaining:
                    if c in player.library_cards:
                        player.library_cards.remove(c)
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3,
                filter_fn=digivolve_filter,
                on_selected=on_selected,
                is_optional=True,
                prompt="Select a level 6 green Digimon to digivolve into (free)."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
