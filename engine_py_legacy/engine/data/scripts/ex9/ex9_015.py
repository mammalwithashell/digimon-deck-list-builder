from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_015(CardScript):
    """EX9-015 Gizamon | Lv.3 Blue Sea Animal/DM/Ver.5
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Training> ---
        # NOTE: Training (suspend to place top deck card as bottom digi card)
        # is a keyword that requires engine-level support. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-015 Training")
        effect0.set_effect_description(
            "<Training> (In the main phase, by suspending this Digimon, "
            "place your deck's top card face down as this Digimon's bottom "
            "digivolution card.)"
        )
        effect0._is_training = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Inherited [When Attacking][Once Per Turn]
        #     Trash bottom digivolution card of 1 opponent's Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX9-015 Inherited: Trash opponent's bottom evo card")
        effect1.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] Trash the bottom "
            "digivolution card of 1 of your opponent's Digimon."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("trash_opp_bottom_evo_EX9_015")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash bottom digivolution card of opponent's Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and not p.has_no_digivolution_cards
            def on_trash(target_perm):
                trashed = target_perm.trash_digivolution_cards(1, from_top=False)
                enemy.trash_cards.extend(trashed)
            game.effect_select_opponent_permanent(
                player, on_trash, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
