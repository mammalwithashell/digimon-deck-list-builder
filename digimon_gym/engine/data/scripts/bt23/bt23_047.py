from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_047(CardScript):
    """BT23-047 Examon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.6 with CS trait for cost 5
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-047 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or not permanent.top_card:
                return False
            traits = getattr(permanent.top_card, 'card_traits', []) or []
            return any('CS' in t for t in traits)

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition: green Lv.5 + blue Lv.5 (Partition source cards)
        # descriptive-tagged: jogress_condition — engine does not yet support jogress
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-047 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: security_attack_plus
        # Security Attack +1 (permanent keyword)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-047 Security Attack +1")
        effect2.set_effect_description("Security Attack +1")
        effect2._security_attack_modifier = 1

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: piercing
        effect2b = ICardEffect()
        effect2b.set_effect_name("BT23-047 Piercing")
        effect2b.set_effect_description("Piercing")
        effect2b._is_piercing = True

        def condition2b(context: Dict[str, Any]) -> bool:
            return True

        effect2b.set_can_use_condition(condition2b)
        effects.append(effect2b)

        # Factory effect: partition (green Lv.5 & blue Lv.5)
        # Engine handles partition via _is_partition flag on the ICardEffect.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-047 Partition")
        effect3.set_effect_description("Partition (green Lv.5 & blue Lv.5)")
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        def _shared_suspend_effect(ctx: Dict[str, Any]):
            """
            Suspend up to 5 opponent Digimon or Tamers.
            Apply CANNOT_UNSUSPEND to all existing opponent Digimon until their next
            unsuspend phase (gap: does not cover Digimon that enter after this resolves).
            Then this Digimon may attack (force_attack — engine stub).
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy:
                # Suspend up to 5 opponent Digimon or Tamers (player selects)
                targets = [
                    p for p in enemy.battle_area
                    if (p.is_digimon or p.is_tamer) and not p.is_suspended
                ][:5]
                for t in targets:
                    t.suspend()
                # Apply CANNOT_UNSUSPEND to all current opponent Digimon for their next unsuspend phase
                # NOTE: Aura-style gap — permanents that enter the field after this won't be covered.
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                for p in enemy.battle_area:
                    if p.is_digimon:
                        game.register_modifier(
                            p, ModifierType.CANNOT_UNSUSPEND,
                            expiry='end_of_opponent_turn',
                        )
            # Then, this Digimon may attack
            pass  # descriptive-tagged: force_attack — engine does not yet support optional self-attack

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 5 of your opponent's Digimon or Tamers. None can unsuspend next
        # unsuspend phase. Then this Digimon may attack.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-047 Suspend 5 digimon/tamer, none can unsuspend. Then you may attack")
        effect4.set_effect_description("[On Play] Suspend 5 of your opponent's Digimon or Tamers. None of your opponent's Digimon can unsuspend in their next unsuspend phase. Then, this Digimon may attack.")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            _shared_suspend_effect(ctx)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Same effect as On Play.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT23-047 Suspend 5 digimon/tamer, none can unsuspend. Then you may attack")
        effect5.set_effect_description("[When Digivolving] Suspend 5 of your opponent's Digimon or Tamers. None of your opponent's Digimon can unsuspend in their next unsuspend phase. Then, this Digimon may attack.")
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            _shared_suspend_effect(ctx)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn] [Once Per Turn] When your OPPONENT's security stack is removed from,
        # trash 1 of their Option cards in the battle area. Then delete 1 of their suspended
        # Digimon or Tamers.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnLoseSecurity)
        effect6.set_effect_name("BT23-047 Trash 1 option card, then delete 1 suspended digimon/tamer")
        effect6.set_effect_description("[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, trash 1 of their Option cards in the battle area. Then, delete 1 of their suspended Digimon or Tamers.")
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("YT_BT23_047")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Only when OPPONENT's security is removed (not own)
            losing_player = context.get('player')
            owner = card.owner if card else None
            enemy = owner.enemy if owner else None
            if losing_player is not None and enemy is not None and losing_player is not enemy:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Trash 1 opponent Option in battle area, then delete 1 suspended Digimon/Tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Trash 1 of their Option cards in the battle area (player selects)
            def option_filter(p):
                return p.is_option
            def on_trash_option(target_perm):
                if target_perm in enemy.battle_area:
                    enemy.battle_area.remove(target_perm)
                    for cs in target_perm.card_sources:
                        enemy.trash_cards.append(cs)
            option_perms = [p for p in enemy.battle_area if p.is_option]
            if option_perms:
                game.effect_select_opponent_permanent(
                    player, on_trash_option,
                    filter_fn=option_filter,
                    is_optional=False,
                )
            # Then, delete 1 of their suspended Digimon or Tamers (player selects)
            def delete_filter(p):
                return (p.is_digimon or p.is_tamer) and p.is_suspended
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            suspended_targets = [
                p for p in enemy.battle_area
                if (p.is_digimon or p.is_tamer) and p.is_suspended
            ]
            if suspended_targets:
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=delete_filter,
                    is_optional=False,
                )

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
