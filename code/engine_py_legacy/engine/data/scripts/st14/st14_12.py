from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST14_12(CardScript):
    """ST14-12 Rivals' Barrage | Option

    When this card is trashed from your deck, you may place it in the battle area.
    [Main] Delete 1 of your opponent's Digimon with the highest level.
    [Main] <Delay> (By trashing this card after the placing turn, activate the
        effect below.)
        - Return 1 purple Digimon card or Tamer card from your trash to your hand.
    [Security] Delete 1 of your opponent's Digimon with the highest level.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Delete 1 of your opponent's Digimon with the highest level
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST14-12 Main: delete highest level opponent Digimon")
        effect0.set_effect_description(
            "[Main] Delete 1 of your opponent's Digimon with the highest level."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            if not opp_digimon:
                return
            max_level = max(p.level for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == max_level

            def on_delete(target_perm):
                if target_perm in enemy.battle_area:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay effect: place in battle area
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("ST14-12 Delay: place in battle area")
        effect1.set_effect_description("[Main] <Delay>")
        effect1._is_delay = True
        effect1._delay_placed = False

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Delay activation: Return 1 purple Digimon card or Tamer card from trash to hand
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.DelaySkill)
        effect2.set_effect_name("ST14-12 Delay: return purple Digimon or Tamer from trash to hand")
        effect2.set_effect_description(
            "[Delay] Return 1 purple Digimon card or Tamer card from your trash to your hand."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def trash_filter(c):
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                if 'Purple' not in colors:
                    return False
                return getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)

            valid_cards = [c for c in player.trash_cards if trash_filter(c)]
            if not valid_cards:
                return
            # Use the first matching card (simplified — ideally selection phase)
            selected = valid_cards[0]
            if selected in player.trash_cards:
                player.trash_cards.remove(selected)
                player.hand_cards.append(selected)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Delete 1 of your opponent's Digimon with the highest level
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("ST14-12 Security: delete highest level opponent Digimon")
        effect3.set_effect_description(
            "[Security] Delete 1 of your opponent's Digimon with the highest level."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            if not opp_digimon:
                return
            max_level = max(p.level for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == max_level

            def on_delete(target_perm):
                if target_perm in enemy.battle_area:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
