from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_082(CardScript):
    """BT22-082 Eater Adam | Lv.None (Black, Cost 8)

    <Security A. +1>
    [On Play] [When Digivolving] Delete 1 of your opponent's play cost 7 or
        lower Digimon. Then, if this Digimon has no digivolution cards, you may
        place 1 [Arata Sanada] from your hand or trash as this Digimon's bottom
        digivolution card.
    [All Turns] When this Digimon would leave the battle area, you may play 1
        [Arata Sanada] from its digivolution cards without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt digivolution from Arata Sanada ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-082 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 4
        effect0._alt_digi_name = "Arata Sanada"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Arata Sanada')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Security Attack +1 ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-082 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared On Play/When Digivolving logic ---
        def _delete_and_place(ctx, is_on_play):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top is None:
                    return False
                return (top.get_cost_itself or 0) <= 7

            def after_delete():
                # Then, if this Digimon has no digivolution cards, place Arata Sanada
                if perm.has_no_digivolution_cards:
                    # Find Arata Sanada in hand or trash
                    arata = None
                    arata_zone = None
                    for c in player.hand_cards:
                        names = getattr(c, 'card_names', []) or []
                        if any('Arata Sanada' in n for n in names):
                            arata = c
                            arata_zone = 'hand'
                            break
                    if arata is None:
                        for c in player.trash_cards:
                            names = getattr(c, 'card_names', []) or []
                            if any('Arata Sanada' in n for n in names):
                                arata = c
                                arata_zone = 'trash'
                                break
                    if arata:
                        if arata_zone == 'hand':
                            player.hand_cards.remove(arata)
                        else:
                            player.trash_cards.remove(arata)
                        perm.add_card_source_bottom(arata)
                        game.logger.log(
                            f"[BT22-082] Placed Arata Sanada from {arata_zone} "
                            f"as bottom digivolution card."
                        )

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
                after_delete()

            has_target = any(target_filter(p) for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            else:
                after_delete()

        # --- [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-082 On Play: Delete cost<=7, place Arata Sanada")
        effect2.set_effect_description(
            "[On Play] Delete 1 of your opponent's play cost 7 or lower Digimon. "
            "Then, if this Digimon has no digivolution cards, you may place 1 "
            "[Arata Sanada] from your hand or trash as this Digimon's bottom "
            "digivolution card."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            _delete_and_place(ctx, is_on_play=True)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- [When Digivolving] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT22-082 When Digivolving: Delete cost<=7, place Arata Sanada")
        effect3.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's play cost 7 or "
            "lower Digimon. Then, if this Digimon has no digivolution cards, "
            "you may place 1 [Arata Sanada] from your hand or trash as this "
            "Digimon's bottom digivolution card."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            _delete_and_place(ctx, is_on_play=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- [All Turns] When leaving, play Arata Sanada from digi-cards ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("BT22-082 Play Arata Sanada from digi-cards")
        effect4.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area, you "
            "may play 1 [Arata Sanada] from its digivolution cards without "
            "paying the cost."
        )
        effect4.is_optional = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm:
                for cs in perm.card_sources:
                    if cs is perm.top_card:
                        continue
                    names = getattr(cs, 'card_names', []) or []
                    if any('Arata Sanada' in n for n in names):
                        return True
            return False
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Find Arata Sanada in digivolution cards
            for cs in list(perm.card_sources):
                if cs is perm.top_card:
                    continue
                names = getattr(cs, 'card_names', []) or []
                if any('Arata Sanada' in n for n in names):
                    perm.card_sources.remove(cs)
                    played = player.play_card_from_source(cs, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": cs, "played_permanent": played,
                             "event_player": player},
                        )
                    break

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
