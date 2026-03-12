from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_073(CardScript):
    """EX6-073 Ogudomon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 6
        effect0._alt_digi_cost = 6
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Seven Great Demon Lords"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place up to 7 cards with different names and the [Seven Great Demon Lords] trait from your trash as this Digimon's bottom digivolution cards. If you placed 4 or more cards with this effect, delete 1 of your opponent's Digimon or Tamers.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-073 Place up to 7 sources, delete digimon or tamer")
        effect1.set_effect_description("[When Digivolving] You may place up to 7 cards with different names and the [Seven Great Demon Lords] trait from your trash as this Digimon's bottom digivolution cards. If you placed 4 or more cards with this effect, delete 1 of your opponent's Digimon or Tamers.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] You may place up to 7 cards with different names and the [Seven Great Demon Lords] trait from your trash as this Digimon's bottom digivolution cards. If you placed 4 or more cards with this effect, delete 1 of your opponent's Digimon or Tamers.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX6-073 Place up to 7 sources, delete digimon or tamer")
        effect2.set_effect_description("[When Attacking] You may place up to 7 cards with different names and the [Seven Great Demon Lords] trait from your trash as this Digimon's bottom digivolution cards. If you placed 4 or more cards with this effect, delete 1 of your opponent's Digimon or Tamers.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] By returning 7 cards with different names and the [Seven Great Demon Lords] trait from this Digimon's digivolution cards to the bottom of the deck, delete 7 of your opponent's Digimon or Tamers. Then, trash the top 7 cards of your opponent's security stack. For each card deleted by this effect, reduce the cards trashed by 1.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX6-073 Delete 7 Digimon/Tamers, Then Trash 7 security. For each card deleted, reduce that number by 1")
        effect3.set_effect_description("[When Attacking] By returning 7 cards with different names and the [Seven Great Demon Lords] trait from this Digimon's digivolution cards to the bottom of the deck, delete 7 of your opponent's Digimon or Tamers. Then, trash the top 7 cards of your opponent's security stack. For each card deleted by this effect, reduce the cards trashed by 1.")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(7):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
