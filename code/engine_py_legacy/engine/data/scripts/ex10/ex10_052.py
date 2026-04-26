from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_052(CardScript):
    """EX10-052 Lucemon: Chaos Mode | Lv.5

    [When Digivolving] [When Attacking] By trashing 1 card in your hand, your
        opponent may delete 1 of their Digimon or Tamers. If this effect didn't
        delete, Recovery +1 (Deck).
    [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
        your opponent may delete 1 of their Digimon or Tamers. If this effect
        didn't delete, it doesn't leave.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digi from Lucemon for cost 5
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Lucemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Lucemon')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] [When Attacking] trash hand, opponent may delete or recovery +1
        def _build_trash_effect(is_when_digivolving=False, is_on_attack=False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_when_digivolving = True
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
            effect.set_effect_name("EX10-052 Trash hand, opponent delete or recovery")
            effect.set_effect_description(
                "By trashing 1 card in your hand, your opponent may delete 1 of "
                "their Digimon or Tamers. If this effect didn't delete, Recovery +1."
            )
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                player = card.owner if card else None
                if not player or not player.hand_cards:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def hand_filter(c):
                    return True

                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)

                    # Opponent may delete 1 of their Digimon or Tamers
                    enemy = player.enemy
                    if not enemy:
                        player.recovery(1)
                        return

                    opp_targets = [p for p in enemy.battle_area if p.is_digimon or p.is_tamer]
                    if opp_targets:
                        # Simplified: opponent auto-deletes (AI decision)
                        target = opp_targets[0]
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)
                    else:
                        # Didn't delete -> Recovery +1
                        player.recovery(1)

                if player.hand_cards:
                    game.effect_select_hand_card(
                        player, hand_filter, on_trashed, is_optional=False)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_trash_effect(is_when_digivolving=True))
        effects.append(_build_trash_effect(is_on_attack=True))

        # [All Turns] [Once Per Turn] When would leave, opponent may delete or stay
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX10-052 Opponent delete or this stays")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the battle "
            "area, your opponent may delete 1 of their Digimon or Tamers. If "
            "this effect didn't delete, it doesn't leave."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX10_052_RemoveField")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                ctx['prevent_leave'] = True
                return

            opp_targets = [p for p in enemy.battle_area if p.is_digimon or p.is_tamer]
            if opp_targets:
                # Simplified: opponent auto-deletes
                target = opp_targets[0]
                if target in enemy.battle_area:
                    enemy.delete_permanent(target)
            else:
                # Didn't delete -> doesn't leave
                ctx['prevent_leave'] = True

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
