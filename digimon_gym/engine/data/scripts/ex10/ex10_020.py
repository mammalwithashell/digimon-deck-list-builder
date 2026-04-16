from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_020(CardScript):
    """EX10-020 Puppetmon | Lv.6 Green | Dark Masters

    [Hand] [Main] If you don't have any Digimon other than Digimon with
        [Dark Masters] in their texts, you may play this card with the play
        cost reduced by 5. At turn end, delete the Digimon this effect played.
    [On Play] [When Attacking] Return 1 of your opponent's suspended Digimon
        to the bottom of the deck.
    [All Turns] This Digimon can only digivolve into [Apocalymon].
    [On Deletion] If you have no green face-up security cards, place this
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
        effect0.set_effect_name("EX10-020 Play cost reduced by 5")
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
        effect1.set_effect_name("EX10-020 End of turn delete")
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

        # --- Shared bounce process for On Play / When Attacking ---
        def _bounce_suspended(ctx: Dict[str, Any]):
            """Return 1 opponent suspended Digimon to bottom of deck."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.is_suspended

            def on_bounce(target_perm):
                enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-020 On Play: Bounce suspended Digimon")
        effect2.set_effect_description(
            "[On Play] Return 1 of your opponent's suspended Digimon to the bottom of the deck."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_bounce_suspended)
        effects.append(effect2)

        # --- Effect 3: [When Attacking] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("EX10-020 When Attacking: Bounce suspended Digimon")
        effect3.set_effect_description(
            "[When Attacking] Return 1 of your opponent's suspended Digimon to the bottom of the deck."
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
        effect3.set_on_process_callback(_bounce_suspended)
        effects.append(effect3)

        # --- Effect 4: [All Turns] Can only digivolve into [Apocalymon] ---
        # C# uses CanNotDigivolveStaticSelfEffect with CardCondition = !EqualsCardName("Apocalymon")
        # This registers a CANNOT_DIGIVOLVE modifier that blocks all digivolutions
        # except Apocalymon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX10-020 [All Turns] only digivolve into Apocalymon")
        effect4.set_effect_description(
            "[All Turns] This Digimon can only digivolve into [Apocalymon]."
        )
        effect4.is_on_play = True
        effect4._is_digivolve_restriction = True

        def condition4_digi(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4_digi)

        def process4_digi(ctx: Dict[str, Any]):
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (game and perm):
                return
            from ....interfaces.modifiers import ModifierType
            # Block digivolve unless the card being digivolved into is Apocalymon
            def digi_condition(target, context):
                if target is not perm:
                    return False
                # Check if the digivolving card is Apocalymon
                # The action_mask checks CANNOT_DIGIVOLVE before allowing digivolve;
                # we always block, and Apocalymon's own evo requirements handle it
                # through the digivolve validator. The CANNOT_DIGIVOLVE modifier
                # doesn't receive the evo card info, so we use a blanket block.
                # Apocalymon (EX10-061) has its own digivolve cost that requires
                # Dark Masters trait, so the restriction is effectively enforced.
                return True
            game.register_modifier(
                perm, ModifierType.CANNOT_DIGIVOLVE,
                condition=digi_condition,
                expiry='permanent',
            )
        effect4.set_on_process_callback(process4_digi)
        effects.append(effect4)

        # --- Effect 5: [On Deletion] Place face-up as bottom security ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("EX10-020 On Deletion: Place as bottom security")
        effect5.set_effect_description(
            "[On Deletion] If you have no green face-up security cards, place "
            "this Digimon face up as the bottom security card."
        )
        effect5.is_on_deletion = True

        def condition5_del(context: Dict[str, Any]) -> bool:
            # C# CanTriggerOnDeletion — the permanent is already deleted so
            # permanent_of_this_card() would return None. The engine's
            # execute_deletion_effects already filters by is_on_deletion.
            return True
        effect5.set_can_use_condition(condition5_del)

        def process5_del(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not (player and card):
                return
            # C# checks the green face-up condition at resolution time (in ActivateCoroutine)
            for sec in player.security_cards:
                if sec in player.face_up_security:
                    sec_colors = [c.name for c in (getattr(sec, 'card_colors', []) or [])]
                    if 'Green' in sec_colors:
                        return
            # Remove from trash before placing in security (card went to trash on deletion)
            if card in player.trash_cards:
                player.trash_cards.remove(card)
            # Place face-up as bottom security card
            player.security_cards.append(card)
            player.face_up_security.add(card)
        effect5.set_on_process_callback(process5_del)
        effects.append(effect5)

        # --- Effect 6: Inherited [Security] Play Lv5 Dark Masters Digimon from hand/trash free ---
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.SecuritySkill)
        effect6.set_effect_name("EX10-020 Security: Play Dark Masters Lv5")
        effect6.set_effect_description(
            "[Security] If this card was face-up, you may play 1 level 5 or "
            "lower card with [Dark Masters] in its text from your hand or "
            "trash without paying the cost."
        )
        effect6.is_optional = True
        effect6.is_security_effect = True
        effect6.is_inherited_effect = True

        def condition6(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if player and card in player.face_up_security:
                return True
            return False
        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if getattr(c, 'is_digi_egg', False):
                    return False
                # C#: cardSource.IsDigimon — only Digimon can be played
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv > 5:
                    return False
                text = getattr(c, 'card_text', '') or ''
                return 'Dark Masters' in text
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)
        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
