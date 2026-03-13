from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any

from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_007(CardScript):
    """BT13-007 King Drasil_7D6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Breeding][Your Turn] All of your Digimon can't digivolve.
        # Implemented as a continuous modifier registered when this card enters
        # the breeding area and cleaned up when it leaves.
        effect_nodigi = ICardEffect()
        effect_nodigi.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_nodigi.set_effect_name("BT13-007 All your Digimon can't digivolve")
        effect_nodigi.set_effect_description(
            "[Breeding][Your Turn] All of your Digimon can't digivolve."
        )
        effect_nodigi._allow_breeding_source = True

        def condition_nodigi(context: Dict[str, Any]) -> bool:
            # This is a continuous effect — we register it once from any timing.
            # We need the card to be in the breeding area.
            if not (card and card.owner):
                return False
            perm = card.permanent_of_this_card()
            if perm is None or card.owner.breeding_area is not perm:
                return False
            return card.owner.is_my_turn

        effect_nodigi.set_can_use_condition(condition_nodigi)

        def process_nodigi(ctx: Dict[str, Any]):
            player = ctx.get("player")
            perm = ctx.get("permanent")
            game = ctx.get("game")
            if not (player and perm and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            # Register CANNOT_DIGIVOLVE on all of this player's battle area Digimon.
            # The modifier condition checks ownership and your-turn.
            for battle_perm in player.battle_area:
                game.register_modifier(
                    battle_perm,
                    ModifierType.CANNOT_DIGIVOLVE,
                    condition=lambda p, ctx, owner=player: owner.is_my_turn,
                    source_effect=effect_nodigi,
                    expiry='permanent',
                )

        effect_nodigi.set_on_process_callback(process_nodigi)
        effects.append(effect_nodigi)

        # [Breeding][Your Turn][Once Per Turn] When a [Royal Knight] trait Digimon
        # card would be played, you may reduce the play cost by 4, plus 1 for each
        # of this Digimon's digivolution cards.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT13-007 Royal Knight play cost reduction")
        effect0.set_effect_description(
            "[Breeding][Your Turn][Once Per Turn] When a [Royal Knight] trait Digimon card would be played, reduce the play cost by 4 plus 1 for each digivolution card under this Digimon."
        )
        effect0._allow_breeding_source = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("CostReduce_BT13_007")

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is card:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = context.get("permanent")
            if permanent is None or card.owner.breeding_area is not permanent:
                return False
            played_card = context.get("card_source")
            if not (played_card and getattr(played_card, "is_digimon", False)):
                return False
            traits = getattr(played_card, "card_traits", []) or []
            return any("Royal Knight" in trait for trait in traits)

        effect0.set_can_use_condition(condition0)
        def _cost_reduction_fn(context):
            perm = context.get("permanent") or card.permanent_of_this_card()
            digi_card_count = max(0, len(perm.card_sources) - 1) if perm else 0
            return 4 + digi_card_count

        effect0._cost_reduction_value_fn = _cost_reduction_fn

        def process0(ctx: Dict[str, Any]):
            return

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Breeding][Start of Your Main Phase] Reveal the top card of your Digi-Egg
        # deck, then place that card and all of your [Royal Knight] trait Digimon as
        # this Digimon's bottom digivolution cards.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT13-007 absorb Digi-Egg and Royal Knights")
        effect1.set_effect_description(
            "[Breeding][Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon's bottom digivolution cards."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = context.get("permanent")
            return permanent is not None and card.owner.breeding_area is permanent

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get("player")
            perm = ctx.get("permanent")
            game = ctx.get("game")
            if not (player and perm and game):
                return

            absorbed_sources = []
            if player.digitama_library_cards:
                absorbed_sources.append(player.digitama_library_cards.pop(0))

            for field_perm in list(player.battle_area):
                if field_perm is perm or not field_perm.top_card:
                    continue
                traits = getattr(field_perm.top_card, "card_traits", []) or []
                if not any("Royal Knight" in trait for trait in traits):
                    continue
                player.battle_area.remove(field_perm)
                game.cleanup_modifiers_for_permanent(field_perm)
                absorbed_sources.extend(field_perm.card_sources)

            if absorbed_sources:
                # absorbed cards go to the BOTTOM of the stack (index 0 = bottom)
                perm.card_sources = absorbed_sources + perm.card_sources

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited effect:
        # [Breeding][Your Turn][Once Per Turn] When an Option card with the
        # [Royal Knight] trait is placed in the battle area, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-007 gain 1 memory")
        effect2.set_effect_description(
            "[Breeding][Your Turn][Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Memory+1_BT13_007")

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = context.get("permanent")
            if permanent is None or card.owner.breeding_area is not permanent:
                return False
            played_card = context.get("played_card")
            if not (played_card and getattr(played_card, "is_option", False)):
                return False
            traits = getattr(played_card, "card_traits", []) or []
            return any("Royal Knight" in trait for trait in traits)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get("player")
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
