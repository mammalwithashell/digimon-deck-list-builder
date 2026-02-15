from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_055(CardScript):
    """BT20-055 Invisimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndTurn
        # (Security) [End of Opponent's Turn] Play this card without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-055 Play this card")
        effect0.set_effect_description("(Security) [End of Opponent's Turn] Play this card without paying the cost.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-055 <De-Digivolve 2> 1 opponent's Digimon, flip their security card faceup, then delete 1 Digimon")
        effect1.set_effect_description("[On Play] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve, Flip Security"""
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
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Flip opponent's top face-down security card face up
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                pass  # Security flip — engine handles face-up/face-down state

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-055 <De-Digivolve 2> 1 opponent's Digimon, flip their security card faceup, then delete 1 Digimon")
        effect2.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon and flip your opponent's top face-down security card face up. Then, delete 1 of their Digimon with 1 or fewer digivolution cards.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve, Flip Security"""
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
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Flip opponent's top face-down security card face up
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                pass  # Security flip — engine handles face-up/face-down state

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnSecurityCheck
        # [Your Turn] When your Digimon check face-up security cards, you may place this Digimon's top stacked card face up as the bottom security card.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-055 Place top card face up as bottom security")
        effect3.set_effect_description("[Your Turn] When your Digimon check face-up security cards, you may place this Digimon's top stacked card face up as the bottom security card.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 0):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
