from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_063(CardScript):
    """BT19-063 DarkKnightmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-063 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: save
        # Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-063 Save")
        effect1.set_effect_description("Save")
        effect1._is_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: material_save
        # Material Save
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-063 Material Save")
        effect2.set_effect_description("Material Save")
        effect2._is_material_save = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, you may delete 1 play cost 3 or lower Digimon or Tamer.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-063 De-Digivolve 1 to 1 Digimon and delete 1 Digimon or tamer with 3 or less Cost")
        effect3.set_effect_description("[On Play] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, you may delete 1 play cost 3 or lower Digimon or Tamer.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve"""
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
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, you may delete 1 play cost 3 or lower Digimon or Tamer.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-063 De-Digivolve 1 to 1 Digimon and delete 1 Digimon or tamer with 3 or less Cost")
        effect4.set_effect_description("[When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if DigiXrosing with 2 cards, you may delete 1 play cost 3 or lower Digimon or Tamer.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve"""
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
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with [Knightmon] in its text from under your Tamers without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-063 Play 1 level 4 or lower Digimon")
        effect5.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with [Knightmon] in its text from under your Tamers without paying the cost.")
        effect5.is_optional = True
        effect5.is_on_deletion = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Knightmon' in text):
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
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with [Knightmon] in its text from your trash without paying the cost.
        effect6 = ICardEffect()
        effect6.set_effect_name("BT19-063 Play 1 level 4 or lower Digimon")
        effect6.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with [Knightmon] in its text from your trash without paying the cost.")
        effect6.is_inherited_effect = True
        effect6.is_optional = True
        effect6.is_on_deletion = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Knightmon' in text):
                    return False
            else:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
