from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_012(CardScript):
    """EX10-012 MetalSeadramon | Lv.6 Blue | Dark Masters

    [Hand] [Main] If you don't have any Digimon other than Digimon with
        [Dark Masters] in their texts, you may play this card with the play
        cost reduced by 5. At turn end, delete the Digimon this effect played.
    [On Play] [When Attacking] 1 of your opponent's Digimon and 1 of their
        Tamers can't suspend until their turn ends.
    [All Turns] This Digimon can only digivolve into [Apocalymon].
    [On Deletion] If you have no blue face-up security cards, place this
        Digimon face up as the bottom security card.
    Inherited: [Security] If this card was face-up, you may play 1 level 5
        or lower card with [Dark Masters] in its text from your hand or trash
        without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _only_dark_masters_on_field(player) -> bool:
            """True if all Digimon on field have 'Dark Masters' in text."""
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
        effect0.set_effect_name("EX10-012 Play cost reduced by 5")
        effect0.set_effect_description(
            "[Hand] [Main] If you don't have any Digimon other than Digimon "
            "with [Dark Masters] in their texts, you may play this card with "
            "the play cost reduced by 5."
        )
        effect0.is_optional = True
        effect0.cost_reduction = 5

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            player = context.get('player')
            if not player:
                return False
            return _only_dark_masters_on_field(player)
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Mark that this card was played via cost reduction (for EOT delete)
            card._ex10_eot_delete = True
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: End of turn — delete this Digimon if played via cost reduction ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("EX10-012 End of turn delete")
        effect1.set_effect_description(
            "At turn end, delete the Digimon this effect played."
        )

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

        # --- Shared process for On Play / When Attacking ---
        def _cant_suspend_effect(ctx: Dict[str, Any]):
            """1 opponent Digimon and 1 opponent Tamer can't suspend until their turn ends."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            from ....interfaces.modifiers import ModifierType

            # 1 opponent Digimon can't suspend
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                def on_select_digimon(target_perm):
                    game.register_modifier(
                        target_perm, ModifierType.CANNOT_SUSPEND,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_opponent_permanent(
                    player, on_select_digimon,
                    filter_fn=lambda p: p.is_digimon, is_optional=False)

            # 1 opponent Tamer can't suspend
            opp_tamers = [p for p in enemy.battle_area if p.is_tamer]
            if opp_tamers:
                def on_select_tamer(target_perm):
                    game.register_modifier(
                        target_perm, ModifierType.CANNOT_SUSPEND,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_opponent_permanent(
                    player, on_select_tamer,
                    filter_fn=lambda p: p.is_tamer, is_optional=False)

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-012 On Play: Opponent can't suspend")
        effect2.set_effect_description(
            "[On Play] 1 of your opponent's Digimon and 1 of their Tamers "
            "can't suspend until their turn ends."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_cant_suspend_effect)
        effects.append(effect2)

        # --- Effect 3: [When Attacking] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX10-012 When Attacking: Opponent can't suspend")
        effect3.set_effect_description(
            "[When Attacking] 1 of your opponent's Digimon and 1 of their "
            "Tamers can't suspend until their turn ends."
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
        effect3.set_on_process_callback(_cant_suspend_effect)
        effects.append(effect3)

        # --- Effect 4: [All Turns] Can only digivolve into [Apocalymon] ---
        # Per C#: CanNotDigivolveStaticSelfEffect — blocks non-Apocalymon.
        # Engine CANNOT_DIGIVOLVE modifier blocks all digivolution on target.
        # The action mask does not pass the digivolving card to the modifier
        # condition, so we cannot selectively allow Apocalymon. This registers
        # a blanket block (matching BT13-086 pattern). Apocalymon digivolution
        # would need engine-level support for conditional CANNOT_DIGIVOLVE.
        effect_digi_restrict = ICardEffect()
        effect_digi_restrict.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_digi_restrict.set_effect_name("EX10-012 Can only digivolve into Apocalymon")
        effect_digi_restrict.set_effect_description(
            "[All Turns] This Digimon can only digivolve into [Apocalymon]."
        )
        effect_digi_restrict.is_on_play = True
        effect_digi_restrict.is_when_digivolving = True

        def condition_digi_restrict(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_digi_restrict.set_can_use_condition(condition_digi_restrict)

        def process_digi_restrict(ctx: Dict[str, Any]):
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (game and perm):
                return
            from ....interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.CANNOT_DIGIVOLVE,
                source_effect=effect_digi_restrict,
                expiry='permanent',
            )
        effect_digi_restrict.set_on_process_callback(process_digi_restrict)
        effects.append(effect_digi_restrict)

        # --- Effect 5: [On Deletion] Place face-up as bottom security ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("EX10-012 On Deletion: Place as bottom security")
        effect4.set_effect_description(
            "[On Deletion] If you have no blue face-up security cards, place "
            "this Digimon face up as the bottom security card."
        )
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            # On Deletion fires after removal from field, so
            # permanent_of_this_card() returns None — no field check needed.
            player = card.owner if card else None
            if not player:
                return False
            # Check no blue face-up security
            for sec in player.security_cards:
                if sec in player.face_up_security:
                    sec_colors = [c.name for c in (getattr(sec, 'card_colors', []) or [])]
                    if 'Blue' in sec_colors:
                        return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not (player and card):
                return
            # Place this card face-up as bottom security
            player.security_cards.append(card)
            player.face_up_security.add(card)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Effect 5: Inherited [Security] Play Lv5 Dark Masters from hand/trash free ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.SecuritySkill)
        effect5.set_effect_name("EX10-012 Security: Play Dark Masters Lv5")
        effect5.set_effect_description(
            "[Security] If this card was face-up, you may play 1 level 5 or "
            "lower card with [Dark Masters] in its text from your hand or "
            "trash without paying the cost."
        )
        effect5.is_optional = True
        effect5.is_security_effect = True
        effect5.is_inherited_effect = True

        def condition5(context: Dict[str, Any]) -> bool:
            # Only activates if this card was face-up in security
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
                # C# ref: cardSource.IsDigimon — must be a Digimon
                if not getattr(c, 'is_digimon', False):
                    return False
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
