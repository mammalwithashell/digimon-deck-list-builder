from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_035(CardScript):
    """EX10-035 Machinedramon | Lv.6 Black | Dark Masters

    [Hand] [Main] If you don't have any Digimon other than Digimon with
        [Dark Masters] in their texts, you may play this card with the play
        cost reduced by 5. At turn end, delete the Digimon this effect played.
    [On Play] [When Attacking] <De-Digivolve 2> 2 of your opponent's Digimon.
    [All Turns] This Digimon can only digivolve into [Apocalymon].
    [On Deletion] If you have no black face-up security cards, place this
        Digimon face up as the bottom security card.
    Inherited: [Security] If this card was face-up, you may play 1 level 5
        or lower card with [Dark Masters] in its text from your hand or trash
        without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _only_dark_masters_on_field(player) -> bool:
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                has_dm = False
                for cs in p.card_sources:
                    text = getattr(cs, 'card_text', '') or ''
                    if 'Dark Masters' in text:
                        has_dm = True
                        break
                if not has_dm:
                    return False
            return True

        # --- Effect 0: BeforePayCost — self cost reduction by 5 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("EX10-035 Play cost reduced by 5")
        effect0.set_effect_description(
            "[Hand] [Main] If you don't have any Digimon other than Digimon "
            "with [Dark Masters] in their texts, you may play this card with "
            "the play cost reduced by 5."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            player = context.get('player')
            if not player:
                return False
            return _only_dark_masters_on_field(player)
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            if hasattr(player, '_temp_play_cost_reduction'):
                player._temp_play_cost_reduction += 5
            else:
                player._temp_play_cost_reduction = 5
            card._ex10_eot_delete = True
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: End of turn — delete this Digimon if played via cost reduction ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("EX10-035 End of turn delete")
        effect1.set_effect_description("At turn end, delete the Digimon this effect played.")

        def condition1(context: Dict[str, Any]) -> bool:
            if not getattr(card, '_ex10_eot_delete', False):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if perm and perm in player.battle_area:
                player.delete_permanent(perm)
                card._ex10_eot_delete = False
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Shared De-Digivolve 2 process for On Play / When Attacking ---
        def _de_digivolve_2_twice(ctx: Dict[str, Any]):
            """<De-Digivolve 2> 2 of opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # De-Digivolve 2 up to 2 different opponent Digimon
            targets_done = []

            def do_de_digivolve(count_remaining):
                if count_remaining <= 0:
                    return

                def target_filter(p):
                    return p.is_digimon and p not in targets_done

                candidates = [p for p in enemy.battle_area if target_filter(p)]
                if not candidates:
                    return

                def on_select(target_perm):
                    targets_done.append(target_perm)
                    if len(target_perm.card_sources) > 1:
                        removed = target_perm.de_digivolve(2)
                        enemy.trash_cards.extend(removed)
                    do_de_digivolve(count_remaining - 1)

                if len(candidates) == 1:
                    on_select(candidates[0])
                else:
                    game.effect_select_opponent_permanent(
                        player, on_select, filter_fn=target_filter, is_optional=False)

            do_de_digivolve(2)

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-035 On Play: De-Digivolve 2 x2")
        effect2.set_effect_description(
            "[On Play] <De-Digivolve 2> 2 of your opponent's Digimon."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_de_digivolve_2_twice)
        effects.append(effect2)

        # --- Effect 3: [When Attacking] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX10-035 When Attacking: De-Digivolve 2 x2")
        effect3.set_effect_description(
            "[When Attacking] <De-Digivolve 2> 2 of your opponent's Digimon."
        )
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_de_digivolve_2_twice)
        effects.append(effect3)

        # --- Effect 4: [On Deletion] Place face-up as bottom security ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("EX10-035 On Deletion: Place as bottom security")
        effect4.set_effect_description(
            "[On Deletion] If you have no black face-up security cards, place "
            "this Digimon face up as the bottom security card."
        )
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            for sec in player.security_cards:
                if sec in player.face_up_security:
                    sec_colors = [c.name for c in (getattr(sec, 'card_colors', []) or [])]
                    if 'Black' in sec_colors:
                        return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not (player and card):
                return
            player.security_cards.append(card)
            player.face_up_security.add(card)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Effect 5: Inherited [Security] Play Lv5 Dark Masters from hand/trash free ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.SecuritySkill)
        effect5.set_effect_name("EX10-035 Security: Play Dark Masters Lv5")
        effect5.set_effect_description(
            "[Security] If this card was face-up, you may play 1 level 5 or "
            "lower card with [Dark Masters] in its text from your hand or "
            "trash without paying the cost."
        )
        effect5.is_optional = True
        effect5.is_security_effect = True
        effect5.is_inherited_effect = True

        def condition5(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if player and card in player.face_up_security:
                return True
            return False
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if getattr(c, 'is_digi_egg', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv > 5:
                    return False
                text = getattr(c, 'card_text', '') or ''
                return 'Dark Masters' in text
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)
        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
