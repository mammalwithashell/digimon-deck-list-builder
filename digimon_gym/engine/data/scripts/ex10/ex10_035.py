from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_035(CardScript):
    """EX10-035 Machinedramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters] in their texts, you may play this card with the play cost reduced by 5. At turn end, delete the Digimon this effect played.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-035 Play for reduced cost of 5, delete at end of turn")
        effect0.set_effect_description("[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters] in their texts, you may play this card with the play cost reduced by 5. At turn end, delete the Digimon this effect played.")
        effect0.is_optional = True
        effect0.cost_reduction = 5

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dark Masters' in text):
                    return False
            else:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -5, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction by 5 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [End of Your Turn] Delete this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-035 Delete the Digimon")
        effect1.set_effect_description("[End of Your Turn] Delete this Digimon.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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
        # [On Play] <De-Digivolve 2> 2 of your opponent's Digimon. (Trash up to 2 cards from the top. You can't trash past level 3 cards.)
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-035 De-digivolve 2, 2 digimon")
        effect2.set_effect_description("[On Play] <De-Digivolve 2> 2 of your opponent's Digimon. (Trash up to 2 cards from the top. You can't trash past level 3 cards.)")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] <De-Digivolve 2> 2 of your opponent's Digimon. (Trash up to 2 cards from the top. You can't trash past level 3 cards.)
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-035 De-digivolve 2, 2 digimon")
        effect3.set_effect_description("[When Attacking] <De-Digivolve 2> 2 of your opponent's Digimon. (Trash up to 2 cards from the top. You can't trash past level 3 cards.)")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If you have no black face-up security cards, place this Digimon face up as the bottom security card.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-035 Place this Digimon face up as bottom security")
        effect4.set_effect_description("[On Deletion] If you have no black face-up security cards, place this Digimon face up as the bottom security card.")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.SecuritySkill
        # [Security] If this card was face-up, you may play 1 level 5 or lower card with [Dark Masters] in its text from your hand or trash without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-035 Play Card")
        effect5.set_effect_description("[Security] If this card was face-up, you may play 1 level 5 or lower card with [Dark Masters] in its text from your hand or trash without paying the cost.")
        effect5.is_optional = True
        effect5.is_security_effect = True
        effect5.is_security_effect = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dark Masters' in text):
                    return False
            else:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
