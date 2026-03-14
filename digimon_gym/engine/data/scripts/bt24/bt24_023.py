from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_023(CardScript):
    """BT24-023 Calmaramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-023 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Lanamon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Lanamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Lanamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-023 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 with [TS] trait for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_level = 3
        effect1._alt_digi_trait = "TS"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-023 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: decode
        # Decode
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-023 Decode")
        effect3.set_effect_description("Decode")
        effect3._is_decode = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Shared process for On Play / When Digivolving:
        # Return 1 of your opponent's level 4 or lower Digimon to the bottom of the deck.
        # Then, if played by effects, 1 of their Digimon or Tamers can't suspend until their turn ends.
        def make_shared_process(is_on_play_flag):
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                enemy = player.enemy if player else None
                if not enemy:
                    return

                # Step 1: Return 1 opponent's level 4 or lower Digimon to bottom of deck
                def bot_deck_filter(p):
                    if not p.is_digimon:
                        return False
                    level = getattr(p, 'level', None)
                    if level is None or level > 4:
                        return False
                    return True

                def on_bot_deck(target_perm):
                    if target_perm:
                        enemy.return_permanent_to_deck_bottom(target_perm)

                if any(bot_deck_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_bot_deck,
                        filter_fn=bot_deck_filter,
                        is_optional=False,
                        prompt="Select 1 of your opponent's level 4 or lower Digimon to return to the bottom of the deck.")

                # Step 2: "Then, if played by effects" — check if this was played by effects
                # The C# checks IsByEffect(hashtable) && CanTriggerOnPlay(hashtable, card)
                # In our engine, 'played_by_effect' context flag indicates this
                is_by_effect = ctx.get('played_by_effect', False)
                if is_by_effect:
                    # 1 of their Digimon or Tamers can't suspend until their turn ends
                    def stun_filter(p):
                        return p.is_digimon or p.is_tamer

                    def on_stun(target_perm):
                        if target_perm and game:
                            from digimon_gym.engine.interfaces.modifiers import ModifierType
                            game.register_modifier(
                                target_perm, ModifierType.CANNOT_SUSPEND,
                                value_fn=lambda: True, expiry='end_of_opponent_turn')

                    if any(stun_filter(p) for p in enemy.battle_area):
                        game.effect_select_opponent_permanent(
                            player, on_stun,
                            filter_fn=stun_filter,
                            is_optional=False,
                            prompt="Select 1 opponent's Digimon or Tamer that can't suspend.")

            return process

        # Timing: EffectTiming.OnEnterFieldAnyone — [On Play]
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-023 Bottom deck lv4- Digimon, stun if by effect")
        effect4.set_effect_description("[On Play] Return 1 of your opponent's level 4 or lower Digimon to the bottom of the deck. Then, if played by effects, 1 of their Digimon or Tamers can't suspend until their turn ends.")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(make_shared_process(True))
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone — [When Digivolving]
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT24-023 Bottom deck lv4- Digimon, stun if by effect")
        effect5.set_effect_description("[When Digivolving] Return 1 of your opponent's level 4 or lower Digimon to the bottom of the deck. Then, if played by effects, 1 of their Digimon or Tamers can't suspend until their turn ends.")
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(make_shared_process(False))
        effects.append(effect5)

        # Factory effect: jamming
        # Jamming
        effect6 = ICardEffect()
        effect6.set_effect_name("BT24-023 Jamming")
        effect6.set_effect_description("Jamming")
        effect6.is_inherited_effect = True
        effect6._is_jamming = True

        def condition6(context: Dict[str, Any]) -> bool:
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
