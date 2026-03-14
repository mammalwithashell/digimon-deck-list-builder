from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_076(CardScript):
    """BT24-076 WarGrowlmon | Lv.5

    [Trash] [Main] If you have 4 or fewer cards in your hand, play this
        card with the play cost reduced by 2.
    [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or
        lower Digimon.
    Inherited: [On Deletion] You may play 1 level 4 or lower [Dark Dragon]
        or [Evil Dragon] Digimon card from your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Trash] [Main] Play self from trash, cost -2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("BT24-076 Play from trash cost -2")
        effect0.set_effect_description(
            "[Trash] [Main] If you have 4 or fewer cards in your hand, "
            "play this card with the play cost reduced by 2.")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            if card not in player.trash_cards:
                return False
            if len(player.hand_cards) > 4:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if card in player.trash_cards:
                player.play_card_from_source(card, pay_cost=True)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: BeforePayCost cost reduction -2 for self ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-076 Trash Main cost -2")
        effect1.set_effect_description("Reduce play cost by 2 when played from trash.")
        effect1.cost_reduction = 2

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False  # LEAK GUARD
            owner = card.owner if card else None
            if not owner:
                return False
            if len(owner.hand_cards) > 4:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared On Play / When Digivolving: Delete opp lv4 or lower ---
        def _delete_lv4_or_lower(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return (p.is_digimon and
                        p.level is not None and p.level <= 4)

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            if any(target_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's level 4 or lower Digimon.")

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-076 Delete level 4 or lower")
        effect2.set_effect_description(
            "[On Play] Delete 1 of your opponent's level 4 or lower Digimon.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_delete_lv4_or_lower)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-076 Delete level 4 or lower")
        effect3.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_delete_lv4_or_lower)
        effects.append(effect3)

        # --- Effect 4: Inherited [On Deletion] Play lv4 or lower
        #     [Dark Dragon]/[Evil Dragon] from TRASH free ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT24-076 Play Dark/Evil Dragon from trash")
        effect4.set_effect_description(
            "[On Deletion] You may play 1 level 4 or lower Digimon card with "
            "the [Dark Dragon] or [Evil Dragon] trait from your trash without "
            "paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Dragon' in t or 'Evil Dragon' in t for t in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True,
                prompt="Play 1 level 4 or lower [Dark Dragon]/[Evil Dragon] from trash.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
