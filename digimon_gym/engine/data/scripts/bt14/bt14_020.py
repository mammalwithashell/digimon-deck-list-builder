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
        # [Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. This Digimon can't be blocked for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-020 Trash digivolution cards and this Digimon gains unblockable")
        effect0.set_effect_description("[Start of Your Main Phase] Trash any 1 digivolution card of 1 of your opponent's Digimon. This Digimon can't be blocked for the turn.")
        effect0._is_cannot_be_blocked = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash 1 digivolution card from an opponent's Digimon, then gain cannot be blocked"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')

            if player and game:
                opponents = []
                if hasattr(game, 'get_opponents_of'):
                    opponents = game.get_opponents_of(player) or []
                elif hasattr(player, 'opponent') and player.opponent is not None:
                    opponents = [player.opponent]

                for opp in opponents:
                    digimons = getattr(opp, 'battle_area', [])
                    for d in digimons:
                        if not getattr(d, 'has_no_digivolution_cards', True):
                            d.trash_digivolution_cards(1)
                            opponents = None
                            break
                    if opponents is None:
                        break

            if perm:
                perm.grant_keyword('_is_cannot_be_blocked')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from its digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-020 Play [Gomamon] from digivolution cards")
        effect1.set_effect_description("[Opponent's Turn] When this Digimon would be deleted, you may play 1 [Gomamon] from its digivolution cards without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_hash_string("PlayDigivolutionCards_BT14_020")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play [Gomamon] from this Digimon's digivolution cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            evo_cards = getattr(perm, 'digivolution_cards', [])
            for c in evo_cards:
                if getattr(c, 'card_name_eng', '') == 'Gomamon':
                    if hasattr(game, 'play_card'):
                        game.play_card(player, c, free=True)
                    elif hasattr(player, 'play_card'):
                        player.play_card(c, free=True)
                    break

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
