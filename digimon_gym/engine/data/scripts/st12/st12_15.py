from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_15(CardScript):
    """ST12-15 From Master to Disciple"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.
        effect0 = ICardEffect()
        effect0.set_effect_name("ST12-15 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Huckmon' in _n or 'Sistermon' in _n for _n in getattr(c, 'card_names', [])) or any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("ST12-15 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - The next time one of your Digimon would digivolve this turn, reduce the digivolution cost by 1.
        effect2 = ICardEffect()
        effect2.set_effect_name("ST12-15 Digivolution Cost -1")
        effect2.set_effect_description("[Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - The next time one of your Digimon would digivolve this turn, reduce the digivolution cost by 1.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDeclaration
        # Cost -1
        effect3 = ICardEffect()
        effect3.set_effect_name("ST12-15 Remove Effect")
        effect3.set_effect_description("Cost -1")
        effect3.cost_reduction = 1

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Reveal 3 cards from the top of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.
        effect4 = ICardEffect()
        effect4.set_effect_name("ST12-15 Reveal the top 3 cards of deck and place this card in battle area")
        effect4.set_effect_description("[Security] Reveal 3 cards from the top of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.")
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Huckmon' in _n or 'Sistermon' in _n for _n in getattr(c, 'card_names', [])) or any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
