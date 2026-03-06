from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_056(CardScript):
    """BT20-056 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: barrier
        # Barrier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-056 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT20-056 <Recovery +1 (Deck)>")
        effect1.set_effect_description("[On Play] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, then if during attack digivolve breeding area Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            if not (player and perm and game):
                return
            # "if during an attack" — check if any of this player's Digimon is attacking
            during_attack = any(
                p.is_attacking for p in player.battle_area
            )
            if not during_attack:
                return
            # Target is a Digimon in the BREEDING AREA, not Alphamon itself
            breeding = player.breeding_area
            if breeding is None:
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                if not any('Chronicle' in t for t in getattr(c, 'card_traits', [])):
                    return False
                return True
            # TODO: effect_digivolve_from_hand doesn't support trash zone
            game.effect_digivolve_from_hand(
                player, breeding, digi_filter, cost_override=0, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-056 <Recovery +1 (Deck)>")
        effect2.set_effect_description("[When Digivolving] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1, then if during attack digivolve breeding area Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            if not (player and perm and game):
                return
            # "if during an attack" — check if any of this player's Digimon is attacking
            during_attack = any(
                p.is_attacking for p in player.battle_area
            )
            if not during_attack:
                return
            # Target is a Digimon in the BREEDING AREA, not Alphamon itself
            breeding = player.breeding_area
            if breeding is None:
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                if not any('Chronicle' in t for t in getattr(c, 'card_traits', [])):
                    return False
                return True
            # TODO: effect_digivolve_from_hand doesn't support trash zone
            game.effect_digivolve_from_hand(
                player, breeding, digi_filter, cost_override=0, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When security stacks are removed from, 1 of your opponent's Digimon gets -8000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT20-056 1 of your opponent's Digimon gets -8000 DP")
        effect3.set_effect_description("[All Turns] (Once Per Turn) When security stacks are removed from, 1 of your opponent's Digimon gets -8000 DP for the turn.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("RemovedSec_BT20_056")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Select opponent's Digimon and apply -8000 DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_target(target_perm):
                target_perm.change_dp(-8000)
            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon and p.dp is not None,
                is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] (Once Per Turn) When this Digimon would leave the battle area other than by your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card, it doesn't leave.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("BT20-056 Trash your top security card to prevent this Digimon from leaving the battle area")
        effect4.set_effect_description("[All Turns] (Once Per Turn) When this Digimon would leave the battle area other than by your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card, it doesn't leave.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("TrashSecurityToStay_BT20_056")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Alphamon: Ouryuken'))):
                return False
            # "other than by your effects" — only trigger if removal is by opponent
            # TODO: check if removal is by opponent's effects (context key not yet standardized)
            if context.get('is_own_effect', False):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Trash your top security card to prevent leaving the field."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not player:
                return
            # Cost: trash YOUR top security card
            if player.security_cards:
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
            # The Digimon stays on field (removal prevention handled by engine)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
