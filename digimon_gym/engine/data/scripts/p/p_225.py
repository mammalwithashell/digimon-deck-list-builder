from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_225(CardScript):
    """P-225 DigiLab"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-225 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1>. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-225 Draw 1, then place in battle area")
        effect1.set_effect_description("[Main] <Draw 1>. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("P-225 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay>, By placing the top stacked card of any of your level 4 or higher [CS] Trait Digimon as its bottom digivolution card, gain 2 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name("P-225 By placing the top stacked card of any of your level 4 or higher [CS] Trait Digimon as its bottom digivolution card, gain 2 memory.")
        effect3.set_effect_description("[Main] <Delay>, By placing the top stacked card of any of your level 4 or higher [CS] Trait Digimon as its bottom digivolution card, gain 2 memory.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that player has a Lv.4+ CS Digimon with stacked cards
            player = card.owner if card else None
            if player:
                has_target = any(
                    p.is_digimon and (getattr(p.top_card, 'level', 0) or 0) >= 4
                    and any('CS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                    and len(p.card_sources) > 1
                    for p in player.battle_area
                )
                if not has_target:
                    return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Place top stacked card of Lv.4+ CS Digimon as bottom evo card, gain 2 memory"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not p.is_digimon:
                    return False
                if (getattr(p.top_card, 'level', 0) or 0) < 4:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
                if len(p.card_sources) <= 1:
                    return False
                return True
            def on_select(target_perm):
                # Move top stacked card to bottom of digivolution stack
                if len(target_perm.card_sources) > 1:
                    top_card = target_perm.card_sources.pop()
                    target_perm.card_sources.insert(0, top_card)
                player.add_memory(2)
            game.effect_select_own_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("P-225 Security: Place in battle area")
        effect4.set_effect_description("[Security] Place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
