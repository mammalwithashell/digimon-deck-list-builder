from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_090(CardScript):
    """BT14-090 Dragon of Courage"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # While you have a Tamer with [Tai Kamiya] in its name, ignore color requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-090 Ignore color requirements")
        effect0.set_effect_description("While you have a Tamer with [Tai Kamiya] in its name, ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            if not player:
                return False
            for t in getattr(player, 'tamers', []) or []:
                name = (getattr(t, 'card_name', None) or getattr(t, 'name', None) or '').lower()
                if 'tai kamiya' in name:
                    return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Engine models this through condition gating; no direct state change needed here.
            return

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Place 1 [Greymon] and 1 [MetalGreymon] from trash under 1 of your [Agumon],
        # then that Digimon may digivolve into [WarGreymon] in hand ignoring requirements and cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-090 Digivolve")
        effect1.set_effect_description("[Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as the bottom digivolution cards of 1 of your [Agumon], it may digivolve in to a [WarGreymon] from hand ignoring its digivolution requirements and without paying its cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                name = (getattr(c, 'card_name', None) or getattr(c, 'name', None) or '').lower()
                return 'wargreymon' in name

            game.effect_digivolve_from_hand(player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-090 Play Agumon, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to your hand.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                name = (getattr(c, 'card_name', None) or getattr(c, 'name', None) or '').lower()
                return 'agumon' in name

            game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)
            game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

            # Then, add this option card to hand.
            if perm is not None:
                player.hand_cards.append(perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
