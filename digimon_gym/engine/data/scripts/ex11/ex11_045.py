from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_045(CardScript):
    """EX11-045 Metatromon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-045 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] De-Digivolve 2, can't digivolve
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-045 <De-Digivolve 2>, can't digivolve")
        effect2.set_effect_description("[On Play] [Once Per Turn] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon or Tamers can't digivolve until their turn ends.")
        effect2.is_on_play = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EX11_045_OP_WD_WA")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        # Shared process callback for On Play / When Digivolving / When Attacking
        def _shared_de_digivolve_and_cannot_digivolve(ctx: Dict[str, Any]):
            """Action: <De-Digivolve 2> 1 opponent Digimon. Then 1 opponent Digimon/Tamer can't digivolve until their turn ends."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: De-Digivolve 2 one of opponent's Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Step 2: 1 opponent's Digimon or Tamer can't digivolve until their turn ends
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def cannot_digi_filter(p):
                return p.is_digimon or p.is_tamer
            def on_cannot_digivolve(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CANNOT_DIGIVOLVE,
                    condition=lambda p, c, tp=target_perm: p is tp,
                    expiry='end_of_opponent_turn',
                )
            game.effect_select_opponent_permanent(
                player, on_cannot_digivolve, filter_fn=cannot_digi_filter, is_optional=False)

        effect2.set_on_process_callback(_shared_de_digivolve_and_cannot_digivolve)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] De-Digivolve 2, can't digivolve
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-045 <De-Digivolve 2>, can't digivolve")
        effect3.set_effect_description("[When Digivolving] [Once Per Turn] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon or Tamers can't digivolve until their turn ends.")
        effect3.is_when_digivolving = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_045_OP_WD_WA")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_de_digivolve_and_cannot_digivolve)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] De-Digivolve 2, can't digivolve
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX11-045 <De-Digivolve 2>, can't digivolve")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] <De-Digivolve 2> 1 of your opponent's Digimon. Then, 1 of their Digimon or Tamers can't digivolve until their turn ends.")
        effect4.is_on_attack = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX11_045_OP_WD_WA")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_de_digivolve_and_cannot_digivolve)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # Digivolve
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-045 1 other digimon may digivolve into a green card w/[Maquinamon] in text for free")
        effect5.set_effect_description("Digivolve")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_045_EOYT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: 1 of your OTHER Digimon may digivolve into a green card with [Maquinamon] in text."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                text = getattr(c, 'card_text', '') or ''
                if 'Maquinamon' not in text:
                    return False
                return True
            # Must target OTHER Digimon, not this one
            def other_digimon_filter(p):
                return p.is_digimon and p is not perm
            def on_target_selected(target_perm):
                if target_perm is None:
                    return
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, is_optional=True,
                    cost_override=0)
            game.effect_select_own_permanent(
                player, on_target_selected, filter_fn=other_digimon_filter,
                is_optional=True,
                prompt="Select 1 of your other Digimon to digivolve into a green [Maquinamon] card.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("EX11-045 Effect")
        effect6.set_effect_description("Effect")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Continuous effect marker — condition gate for inherited effects."""
            pass

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [All Turns] [Once Per Turn] When effects add to this Digimon's digivolution cards, delete 1 of your opponent's Digimon with the lowest play cost.
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnAddDigivolutionCards)
        effect7.set_effect_name("EX11-045 Delete 1 lowest play cost")
        effect7.set_effect_description("[All Turns] [Once Per Turn] When effects add to this Digimon's digivolution cards, delete 1 of your opponent's Digimon with the lowest play cost.")
        effect7.is_inherited_effect = True
        effect7.set_max_count_per_turn(1)
        effect7.set_hash_string("EX11_045_DELETE_ESS")

        effect = effect7  # alias for condition closure
        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
            """Action: Delete opponent's Digimon with lowest play cost (auto-target)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Find the Digimon with the lowest play cost on opponent's field
            digimon_perms = [p for p in enemy.battle_area if p.is_digimon]
            if not digimon_perms:
                return
            min_cost = min(
                (p.top_card.get_cost_itself if p.top_card else 999)
                for p in digimon_perms
            )
            lowest = [
                p for p in digimon_perms
                if (p.top_card.get_cost_itself if p.top_card else 999) == min_cost
            ]
            # If there's a tie, let the player choose among the tied Digimon
            if len(lowest) == 1:
                enemy.delete_permanent(lowest[0])
            else:
                def target_filter(p):
                    return p in lowest
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        return effects
