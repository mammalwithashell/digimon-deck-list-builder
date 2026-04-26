from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_161(CardScript):
    """P-161 Bishop Device"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-161 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Bishop Device'))):
                return False
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # When this card is trashed from the battle area, until the end of your opponent's turn, 1 of their Digimon or Tamers can't suspend.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("P-161 1 of your opponent's digimon or tamers can't suspend.")
        effect1.set_effect_description("When this card is trashed from the battle area, until the end of your opponent's turn, 1 of their Digimon or Tamers can't suspend.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Until the end of your opponent's turn, 1 of their Digimon or Tamers can't suspend. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("P-161 1 of your opponent's digimon or tamers can't suspend, then place in battle area")
        effect2.set_effect_description("[Main] Until the end of your opponent's turn, 1 of their Digimon or Tamers can't suspend. Then, place this card in the battle area.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Return 1 of your opponent's level 5 or lower Digimon to the bottom of the deck. Then, add this card to the hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("P-161 Return 1 opponent's level 5 or lower Digimon to the bottom of the deck, then add this to hand")
        effect3.set_effect_description("[Security] Return 1 of your opponent's level 5 or lower Digimon to the bottom of the deck. Then, add this card to the hand.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Hand, Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
