from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_112(CardScript):
    """BT5-112 Omnimon Zwart Defeat | Lv.7 Mega (Black, Virus, Holy Warrior)

    [Security] Play this card without battling and without paying the cost.
    [When Digivolving] Delete 1 of your opponent's Tamers.
    [On Deletion] Delete 1 of your opponent's Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play this card without battling and without paying the cost ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("BT5-112 Security: Play without battling free")
        effect0.set_effect_description(
            "[Security] Play this card without battling and without paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            played = player.play_card_from_source(card, pay_cost=False)
            # Skip security battle via modifier
            if played and game:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    played, ModifierType.DONT_BATTLE_SECURITY_DIGIMON,
                    value_fn=lambda: True, expiry='end_of_attack')
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Delete 1 of your opponent's Tamers ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT5-112 When Digivolving: Delete opponent Tamer")
        effect1.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's Tamers."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            opp_tamers = [p for p in enemy.battle_area if p.is_tamer]
            if opp_tamers:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=lambda p: p.is_tamer,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's Tamers.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Deletion] Delete 1 of your opponent's Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT5-112 On Deletion: Delete opponent Digimon")
        effect2.set_effect_description(
            "[On Deletion] Delete 1 of your opponent's Digimon."
        )
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's Digimon.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
