from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_026(CardScript):
    """BT21-026 WarGreymon | Lv.6

    When this card would be played, reduce the play cost by 2 for each of your opponent's Digimon.
    <Rush> <Raid> <Blocker>
    [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted,
    this Digimon may unsuspend.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Cost reduction: -2 per opponent Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT21-026 Play cost -2 per opponent Digimon")
        effect0.set_effect_description(
            "When this card would be played, reduce the play cost by 2 for each of your opponent's Digimon."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            target_card = context.get('card_source')
            if target_card is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def _cost_reduction_fn(context):
            owner = card.owner if card else None
            if not owner or not owner.enemy:
                return 0
            opp_digimon_count = sum(1 for p in owner.enemy.battle_area if p.is_digimon)
            return 2 * opp_digimon_count

        effect0._cost_reduction_value_fn = _cost_reduction_fn
        effects.append(effect0)

        # Factory effect: blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-026 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: rush
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-026 Rush")
        effect2.set_effect_description("Rush")
        effect2._is_rush = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: raid
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-026 Raid")
        effect3.set_effect_description("Raid")
        effect3.is_on_attack = True
        effect3._is_raid = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted,
        # this Digimon may unsuspend.
        # Uses NoTiming + _is_deletion_observer so _fire_deletion_observers() picks it up
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-026 Unsuspend")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, this Digimon may unsuspend.")
        effect4.set_hash_string("Unsuspend_BT21_026")
        effect4._is_deletion_observer = True
        effect4.set_max_count_per_turn(1)

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when an opponent's Digimon is deleted.
            # DCGO: CardEffectCommons.IsPermanentExistsOnOpponentBattleAreaDigimon
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm:
                return False
            if not getattr(deleted_perm, 'is_digimon', False):
                return False
            # Check the deleted perm belonged to the opponent, not to us.
            # _fire_deletion_observers passes owner=deleted_perm's owner.
            # In the owner-scan branch, context['player'] = deleted_perm's owner.
            # In the enemy-scan branch, context['player'] = card.owner.
            # We want to trigger only when the deleted perm is on the opponent's side.
            owner = card.owner if card else None
            if not owner or not owner.enemy:
                return False
            # The deleted permanent should NOT belong to card's owner
            # Check by looking at whose trash the deleted cards ended up in, or
            # simply check that context['player'] is card.owner AND this is
            # the enemy-scan (deleted perm's owner is the enemy).
            # Simplest: check deleted_perm's top_card.owner is the enemy.
            deleted_card_owner = None
            if deleted_perm.top_card:
                deleted_card_owner = getattr(deleted_perm.top_card, 'owner', None)
            if deleted_card_owner is owner:
                return False  # Own Digimon deleted, not opponent's
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Unsuspend this Digimon."""
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm and my_perm.is_suspended:
                my_perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
