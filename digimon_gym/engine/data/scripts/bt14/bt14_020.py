from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_020(CardScript):
    """BT14-020 Gomamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, this Digimon can't be blocked for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-020 Trash opponent digivolution card and gain unblockable")
        effect0.set_effect_description("[Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. Then, this Digimon can't be blocked for the turn.")
        effect0._is_cannot_be_blocked = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash 1 opposing digivolution card, then gain cannot be blocked."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')

            if player and game:
                def target_filter(p):
                    return getattr(p, 'is_digimon', False) and p.owner != player

                game.effect_trash_digivolution_cards(player, target_filter, 1, is_optional=False)

            if perm:
                perm.grant_keyword('_is_cannot_be_blocked')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from this Digimon's digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-020 Play [Gomamon] from this Digimon's digivolution cards")
        effect1.set_effect_description("[Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from this Digimon's digivolution cards without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_hash_string("PlayDigivolutionCards_BT14_020")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play 1 [Gomamon] from this Digimon's digivolution cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return getattr(c, 'card_name_eng', '') == 'Gomamon'

            game.effect_play_from_digivolution_cards(
                player, perm, play_filter, free=True, is_optional=True
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
