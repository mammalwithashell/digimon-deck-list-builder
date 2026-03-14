from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_061(CardScript):
    """EX9-061 Devimon | Lv.4 | Fallen Angel/DM/Ver.1
    <Training>
    [When Attacking][Once Per Turn] By placing deck's top card face down as
    bottom digi card, delete 1 opponent Lv.3 or lower Digimon. For every 2
    face-down digi cards, add 1 to the level maximum.
    Inherited: <Retaliation>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Training>
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-061 Training")
        effect0.set_effect_description("<Training>")
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] Place deck top, delete level-scaled
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX9-061 When Attacking: Place deck top, delete scaled level")
        effect1.set_effect_description(
            "[When Attacking][Once Per Turn] Place deck top as bottom digi card, "
            "delete Lv.3 or lower + 1 per 2 face-down digi cards."
        )
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_delete_scaled_EX9_061")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and not card.owner.library_cards:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.library_cards:
                placed = player.library_cards.pop(0)
                perm.add_card_source_bottom(placed)
            face_down_count = len(perm.digivolution_cards)
            max_level = 3 + (face_down_count // 2)

            def target_filter(p):
                return (p.is_digimon and p.level is not None and
                        p.level <= max_level)

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: <Retaliation>
        effect2 = ICardEffect()
        effect2.set_effect_name("EX9-061 Inherited: Retaliation")
        effect2.set_effect_description("Inherited: <Retaliation>")
        effect2.is_inherited_effect = True
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
