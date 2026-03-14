from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_105(CardScript):
    """P-105 Physical Training"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 2 cards of your deck. Add 1 yellow card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-105 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 2 cards of your deck. Add 1 yellow card among them to your hand. Place the rest at the bottom of your deck in any order. Then, place this card into your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 2, add 1 yellow card to hand, rest to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                return 'Yellow' in colors
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 2, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-105 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [Main] <Delay> Digivolve into yellow Digimon with cost -2
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("P-105 Delay: Digivolve into yellow Digimon cost -2")
        effect2.set_effect_description(
            "<Delay> Your Digimon may digivolve into a yellow Digimon card in "
            "your hand with the digivolution cost reduced by 2."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash delay card, then 1 of your Digimon may digivolve into yellow Digimon with cost -2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash this delay card
            perm = card.permanent_of_this_card() if card else None
            if perm:
                player.delete_permanent(perm)

            def yellow_digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                return 'Yellow' in colors

            def own_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, yellow_digi_filter,
                    cost_reduction=2, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- [Security] Place this card in the battle area ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("P-105 Security: Place in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            player.play_card_from_source(card, pay_cost=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
