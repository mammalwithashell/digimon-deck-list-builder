from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_104(CardScript):
    """P-104 Mental Training"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 2 cards of your deck. Add 1 blue card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-104 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 2 cards of your deck. Add 1 blue card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 2, add 1 blue card to hand, bottom the rest. Then place in battle area."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def blue_filter(c):
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Blue' in colors

            def on_revealed(selected, remaining):
                if selected:
                    player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 2, blue_filter, on_revealed, is_optional=True)

            # Then place this card in the battle area (Delay placement)
            if card and player:
                player.play_card_from_source(card, pay_cost=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-104 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - 1 of your Digimon may digivolve into a blue Digimon card in your hand for its digivolution cost. When it would digivolve by this effect, reduce the cost by 2.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("P-104 Your 1 Digimon digivolves")
        effect2.set_effect_description("[Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - 1 of your Digimon may digivolve into a blue Digimon card in your hand for its digivolution cost. When it would digivolve by this effect, reduce the cost by 2.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """1 of your Digimon may digivolve into a blue Digimon card in hand, cost -2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def blue_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Blue' in colors

            def on_select_perm(selected_perm):
                if selected_perm:
                    game.effect_digivolve_from_hand(
                        player, selected_perm, blue_digi_filter,
                        cost_reduction=2, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_perm,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True,
                prompt="Select 1 of your Digimon to digivolve into a blue Digimon from hand (cost -2).")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("P-104 Security: Place in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True
        effect3._is_delay = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
