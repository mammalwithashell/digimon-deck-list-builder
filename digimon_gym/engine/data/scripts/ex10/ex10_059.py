from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_059(CardScript):
    """EX10-059 DarknessBagramon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-059 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers. Then, by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's top digivolution cards, delete 1 of their Digimon or Tamers with cards under it.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-059 Place 1 opponent hand card under a digimon or tamer, then by placing 3 [Bagra Army] digimon from trash under this, delete 1 digimon/tamer with sources")
        effect1.set_effect_description("[On Play] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers. Then, by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's top digivolution cards, delete 1 of their Digimon or Tamers with cards under it.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers. Then, by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's top digivolution cards, delete 1 of their Digimon or Tamers with cards under it.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-059 Place 1 opponent hand card under a digimon or tamer, then by placing 3 [Bagra Army] digimon from trash under this, delete 1 digimon/tamer with sources")
        effect2.set_effect_description("[When Digivolving] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers. Then, by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's top digivolution cards, delete 1 of their Digimon or Tamers with cards under it.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Grant Skill
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-059 This Digimon gains all [All Turns] effects on all level 6 [Bagra Army] trait Digimon cards in its digivolution cards.")
        effect3.set_effect_description("Grant Skill")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
