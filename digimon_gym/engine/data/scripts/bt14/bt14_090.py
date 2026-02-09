from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_090(CardScript):
    """Auto-transpiled from DCGO BT14_090.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-090 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as 1 of your [Agumon]'s bottom digivolution cards, that Digimon may digivolve into [WarGreymon] in your hand without paying the cost, ignoring its digivolution requirements.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-090 Digivolve")
        effect1.set_effect_description("[Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as 1 of your [Agumon]'s bottom digivolution cards, that Digimon may digivolve into [WarGreymon] in your hand without paying the cost, ignoring its digivolution requirements.")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place Greymon+MetalGreymon from trash, digivolve Agumon into WarGreymon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def is_agumon(p):
                return p.top_card and any('Agumon' in n for n in p.top_card.card_names)
            def on_agumon_selected(agumon_perm):
                greymon = None
                metalgreymon = None
                for tc in player.trash_cards:
                    if not greymon and any('Greymon' in n and 'MetalGreymon' not in n for n in tc.card_names):
                        greymon = tc
                    elif not metalgreymon and any('MetalGreymon' in n for n in tc.card_names):
                        metalgreymon = tc
                if greymon:
                    player.trash_cards.remove(greymon)
                    agumon_perm.card_sources.insert(0, greymon)
                if metalgreymon:
                    player.trash_cards.remove(metalgreymon)
                    agumon_perm.card_sources.insert(0, metalgreymon)
                def is_wargreymon(c):
                    if not c.is_digimon:
                        return False
                    return any('WarGreymon' in n for n in c.card_names)
                game.effect_digivolve_from_hand(
                    player, agumon_perm, is_wargreymon,
                    cost_override=0, ignore_requirements=True, is_optional=True)
            game.effect_select_own_permanent(
                player, on_agumon_selected, filter_fn=is_agumon, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-090 Play Card, Trash From Hand, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play [Agumon] from hand or trash, add this card to hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def is_agumon(c):
                return any('Agumon' in n for n in c.card_names)
            has_in_hand = any(is_agumon(c) for c in player.hand_cards)
            has_in_trash = any(is_agumon(c) for c in player.trash_cards)
            if has_in_hand:
                game.effect_play_from_zone(player, 'hand', is_agumon, free=True, is_optional=True)
            elif has_in_trash:
                game.effect_play_from_zone(player, 'trash', is_agumon, free=True, is_optional=True)
            # Add this option card to hand
            if card and card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
