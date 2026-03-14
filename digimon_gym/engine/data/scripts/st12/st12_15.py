from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_15(CardScript):
    """ST12-15 From Master to Disciple"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST12-15 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, select 1 matching card to hand, trash the rest"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                # Card with [Huckmon] or [Sistermon] in name
                if c.contains_card_name('Huckmon') or c.contains_card_name('Sistermon'):
                    return True
                # Or [Royal Knight] in traits
                traits = c.type_eng if hasattr(c, 'type_eng') else []
                if any('Royal Knight' in t for t in (traits or [])):
                    return True
                return False
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                # Trash the rest (not library bottom)
                for c in remaining:
                    player.trash_cards.append(c)
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
        # [Main] <Delay> (Trash this card in your battle area to activate the effect below.
        # You can't activate this effect the turn this card enters play.)
        # - The next time one of your Digimon would digivolve this turn, reduce the digivolution cost by 1.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("ST12-15 Digivolution Cost -1")
        effect2.set_effect_description("[Main] <Delay> - The next time one of your Digimon would digivolve this turn, reduce the digivolution cost by 1.")
        effect2._is_delay_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Grant digivolution cost -1 for next digivolve this turn"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Grant a one-shot evo cost reduction of 1 to all own permanents
            # by adding a temporary WhenWouldDigivolve effect with cost_reduction=1
            # The effect is consumed after the first digivolution
            temp_effect = ICardEffect()
            temp_effect.set_timing(EffectTiming.WhenWouldDigivolve)
            temp_effect.set_effect_name("ST12-15 Evo Cost -1 (one-shot)")
            temp_effect.is_inherited_effect = True  # So it's picked up from sources
            temp_effect.cost_reduction = 1
            consumed = [False]
            def temp_condition(context: Dict[str, Any]) -> bool:
                if consumed[0]:
                    return False
                consumed[0] = True
                return True
            temp_effect.set_can_use_condition(temp_condition)
            # Grant this temp effect to all own Digimon on field
            for perm in player.battle_area:
                if perm.is_digimon:
                    perm.grant_temp_effect(temp_effect, expiry_turn=game.turn_count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Reveal 3 cards from the top of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("ST12-15 Reveal the top 3 cards of deck and place this card in battle area")
        effect4.set_effect_description("[Security] Reveal 3 cards from the top of your deck. Add 1 card with [Huckmon] or [Sistermon] in its name or [Royal Knight] in its traits among them to your hand. Trash the rest. Then, place this card in your Battle Area.")
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Reveal top 3, select 1 matching card to hand, trash the rest"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                # Card with [Huckmon] or [Sistermon] in name
                if c.contains_card_name('Huckmon') or c.contains_card_name('Sistermon'):
                    return True
                # Or [Royal Knight] in traits
                traits = c.type_eng if hasattr(c, 'type_eng') else []
                if any('Royal Knight' in t for t in (traits or [])):
                    return True
                return False
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                # Trash the rest (not library bottom)
                for c in remaining:
                    player.trash_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
