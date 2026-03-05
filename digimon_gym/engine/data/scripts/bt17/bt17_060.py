from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_060(CardScript):
    """BT17-060 Armageddemon | Lv.7 Mega | Black/White | Unidentified

    When you would play this card from the hand, by placing up to 13 cards
        with the [Unidentified] trait or [Diaboromon] in its text from your
        trash at the bottom of your deck, reduce the cost by 1 for each card.
    <Rush> <Blocker> <Reboot>
    [On Play] [When Digivolving] Delete any of your opponent's Digimon
        with a total play cost up to 15.
    [Your Turn] This Digimon can attack your opponent's unsuspended Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Cost reduction from trash ---
        # When playing from hand, place up to 13 [Unidentified] trait or
        # [Diaboromon]-text cards from trash to deck bottom, reduce cost by 1 each.
        # This is a play-cost reduction mechanic. We model it as a descriptive
        # effect since the engine handles cost reduction during the play action.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT17-060 Cost reduction from trash")
        effect0.set_effect_description(
            "When you would play this card from the hand, by placing up to 13 "
            "cards with the [Unidentified] trait or [Diaboromon] in its text "
            "from your trash at the bottom of your deck, reduce the cost by 1 "
            "for each card."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Move qualifying cards from trash to deck bottom, reduce play cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Find qualifying cards in trash
            qualifying = []
            for tc in player.trash_cards:
                if len(qualifying) >= 13:
                    break
                has_unidentified = 'Unidentified' in (tc.card_traits or [])
                has_diaboromon_text = 'Diaboromon' in (tc.card_text or '')
                if has_unidentified or has_diaboromon_text:
                    qualifying.append(tc)
            # Move them to deck bottom and reduce cost
            reduction = len(qualifying)
            for qc in qualifying:
                player.trash_cards.remove(qc)
                player.library_cards.append(qc)
            if reduction > 0 and hasattr(player, '_temp_play_cost_reduction'):
                player._temp_play_cost_reduction += reduction
            elif reduction > 0:
                player._temp_play_cost_reduction = reduction
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Rush keyword ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT17-060 Rush")
        effect1.set_effect_description("<Rush>")
        effect1._is_rush = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Blocker keyword ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT17-060 Blocker")
        effect2.set_effect_description("<Blocker>")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: Reboot keyword ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT17-060 Reboot")
        effect3.set_effect_description("<Reboot>")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Effect 4: [On Play] Delete opponent Digimon total cost up to 15 ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT17-060 On Play: Delete Digimon total cost 15")
        effect4.set_effect_description(
            "[On Play] Delete any of your opponent's Digimon with a total "
            "play cost up to 15."
        )
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Delete opponent Digimon with total play cost <= 15."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            remaining_cost = 15
            to_delete = []

            def select_next():
                nonlocal remaining_cost
                candidates = [
                    p for p in enemy.battle_area
                    if p.is_digimon and p not in to_delete
                    and p.top_card and p.top_card.c_entity_base
                    and (p.top_card.c_entity_base.play_cost or 0) <= remaining_cost
                ]
                if not candidates:
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)
                    return

                def target_filter(p):
                    return (p.is_digimon and p not in to_delete
                            and p.top_card and p.top_card.c_entity_base
                            and (p.top_card.c_entity_base.play_cost or 0) <= remaining_cost)

                def on_select(target_perm):
                    nonlocal remaining_cost
                    to_delete.append(target_perm)
                    cost = target_perm.top_card.c_entity_base.play_cost if (
                        target_perm.top_card and target_perm.top_card.c_entity_base
                    ) else 0
                    remaining_cost -= (cost or 0)
                    select_next()

                def on_decline():
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)

                game.effect_select_opponent_permanent(
                    player, on_select, filter_fn=target_filter,
                    is_optional=True, on_decline=on_decline)

            select_next()
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Effect 5: [When Digivolving] Same deletion effect ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT17-060 When Digivolving: Delete Digimon total cost 15")
        effect5.set_effect_description(
            "[When Digivolving] Delete any of your opponent's Digimon with a "
            "total play cost up to 15."
        )
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Delete opponent Digimon with total play cost <= 15."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            remaining_cost = 15
            to_delete = []

            def select_next():
                nonlocal remaining_cost
                candidates = [
                    p for p in enemy.battle_area
                    if p.is_digimon and p not in to_delete
                    and p.top_card and p.top_card.c_entity_base
                    and (p.top_card.c_entity_base.play_cost or 0) <= remaining_cost
                ]
                if not candidates:
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)
                    return

                def target_filter(p):
                    return (p.is_digimon and p not in to_delete
                            and p.top_card and p.top_card.c_entity_base
                            and (p.top_card.c_entity_base.play_cost or 0) <= remaining_cost)

                def on_select(target_perm):
                    nonlocal remaining_cost
                    to_delete.append(target_perm)
                    cost = target_perm.top_card.c_entity_base.play_cost if (
                        target_perm.top_card and target_perm.top_card.c_entity_base
                    ) else 0
                    remaining_cost -= (cost or 0)
                    select_next()

                def on_decline():
                    for target in to_delete:
                        if target in enemy.battle_area:
                            enemy.delete_permanent(target)

                game.effect_select_opponent_permanent(
                    player, on_select, filter_fn=target_filter,
                    is_optional=True, on_decline=on_decline)

            select_next()
        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # --- Effect 6: [Your Turn] Can attack unsuspended Digimon ---
        effect6 = ICardEffect()
        effect6.set_effect_name("BT17-060 Attack unsuspended Digimon")
        effect6.set_effect_description(
            "[Your Turn] This Digimon can attack your opponent's unsuspended Digimon."
        )
        effect6._is_can_attack_unsuspended = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
