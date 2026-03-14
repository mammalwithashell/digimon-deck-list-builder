from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_013(CardScript):
    """BT23-013 Jesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 from [SaviorHuckmon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "SaviorHuckmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('SaviorHuckmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-013 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Huckmon] for cost 5
        effect1._alt_digi_cost = 5
        effect1._alt_digi_name = "Huckmon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Huckmon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: rush
        # Rush
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-013 Rush")
        effect2.set_effect_description("Rush")
        effect2._is_rush = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-013 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_on_attack = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-013 Play 1 [Atho, Ren� & Por] token or 1 [Sistermon] in name digimon from hand or trash")
        effect4.set_effect_description("[When Digivolving] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card, Play Token"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Get names of all Digimon currently on the field to enforce
            # "can't play cards with the same names as any of your Digimon"
            field_names = set()
            for p in player.battle_area:
                if p.top_card:
                    for n in p.top_card.card_names:
                        field_names.add(n.lower())

            def play_filter(c):
                if not c.is_digimon:
                    return False
                if not c.contains_card_name('Sistermon'):
                    return False
                # Can't play cards with the same names as any of your Digimon
                for n in c.card_names:
                    if n.lower() in field_names:
                        return False
                return True

            # Check if we can play a token (no "Atho, René & Por" already on field)
            has_token = any(
                p.contains_card_name('Atho') for p in player.battle_area
            )
            has_sistermon = any(
                play_filter(c) for c in player.hand_cards
            ) or any(
                play_filter(c) for c in player.trash_cards
            )

            if has_sistermon and not has_token:
                # Both options available — play from zone (Sistermon) or token
                # Engine selects between them via action mask; try Sistermon first
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            elif has_sistermon:
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            elif not has_token:
                game.effect_play_token(player, 'atho_rene_por')
            # If neither available, nothing happens

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT23-013 Play 1 [Atho, Ren� & Por] token or 1 [Sistermon] in name digimon from hand or trash")
        effect5.set_effect_description("[When Attacking] You may play 1 [Atho, Ren� & Por] Token or, from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card, Play Token"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Get names of all Digimon currently on the field to enforce
            # "can't play cards with the same names as any of your Digimon"
            field_names = set()
            for p in player.battle_area:
                if p.top_card:
                    for n in p.top_card.card_names:
                        field_names.add(n.lower())

            def play_filter(c):
                if not c.is_digimon:
                    return False
                if not c.contains_card_name('Sistermon'):
                    return False
                # Can't play cards with the same names as any of your Digimon
                for n in c.card_names:
                    if n.lower() in field_names:
                        return False
                return True

            # Check if we can play a token (no "Atho, René & Por" already on field)
            has_token = any(
                p.contains_card_name('Atho') for p in player.battle_area
            )
            has_sistermon = any(
                play_filter(c) for c in player.hand_cards
            ) or any(
                play_filter(c) for c in player.trash_cards
            )

            if has_sistermon and not has_token:
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            elif has_sistermon:
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)
            elif not has_token:
                game.effect_play_token(player, 'atho_rene_por')
            # If neither available, nothing happens

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect6.set_effect_name("BT23-013 This digimon may attack")
        effect6.set_effect_description("[Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT23_013_YT")
        effect6.is_on_play = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # "this Digimon may attack" — register FORCE_ATTACK modifier
            from ....interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.FORCE_ATTACK,
                value_fn=lambda: True, expiry='end_of_turn')

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
