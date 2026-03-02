from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_059(CardScript):
    """EX9-059 Ogremon | Lv.4 Purple Demon/DM/Ver.3
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Training> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-059 Training")
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

        # --- Effect 1: [When Digivolving][When Attacking][Once Per Turn]
        #     Place 1 hand card as bottom digi card, delete opponent's Lv.4 or lower ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-059 Place hand card as evo, delete opponent Lv4 or lower")
        effect1.set_effect_description(
            "[When Digivolving][Once Per Turn] By placing 1 card from your "
            "hand face down as a bottom digivolution card, delete 1 of your "
            "opponent's Digimon with level 4 or lower."
        )
        effect1.is_when_digivolving = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("place_delete_EX9_059")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place 1 hand card as bottom evo, delete opponent Lv4 or lower"""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Place 1 hand card as bottom digivolution card
            if player.hand_cards and perm:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)
            # Delete 1 opponent Digimon Lv.4 or lower
            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level <= 4
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 1b: Same but for [When Attacking] ---
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnTappedAnyone)
        effect1b.set_effect_name("EX9-059 When Attacking: Place hand card, delete Lv4 or lower")
        effect1b.set_effect_description(
            "[When Attacking][Once Per Turn] By placing 1 card from your "
            "hand face down as a bottom digivolution card, delete 1 of your "
            "opponent's Digimon with level 4 or lower."
        )
        effect1b.is_on_attack = True
        effect1b.set_max_count_per_turn(1)
        effect1b.set_hash_string("place_delete_EX9_059")  # shared OPT with evo trigger

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and len(card.owner.hand_cards) < 1:
                return False
            return True
        effect1b.set_can_use_condition(condition1b)

        def process1b(ctx: Dict[str, Any]):
            """Action: Place hand card as bottom evo, delete opponent Lv4 or lower"""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            if player.hand_cards and perm:
                placed = player.hand_cards.pop()
                perm.add_card_source_bottom(placed)
            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level <= 4
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1b.set_on_process_callback(process1b)
        effects.append(effect1b)

        # --- Effect 2: Inherited [When Attacking][Once Per Turn]
        #     Draw 1 and trash 1 from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("EX9-059 Inherited: Draw 1 trash 1")
        effect2.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] <Draw 1> and trash "
            "1 card in your hand."
        )
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("draw_trash_EX9_059_inh")

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1, trash 1"""
            player = ctx.get('player')
            if not player:
                return
            player.draw_cards(1)
            if player.hand_cards:
                trashed = player.hand_cards.pop()
                player.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
