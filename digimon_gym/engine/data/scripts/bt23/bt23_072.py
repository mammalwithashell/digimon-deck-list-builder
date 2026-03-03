from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any

from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_072(CardScript):
    """BT23-072 King Drasil_7D6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("BT23-072 draw 1")
        effect0.set_effect_description(
            "[Hand][Main] By paying 3 cost and placing this card as the bottom digivolution card of your [King Drasil_7D6] or [Mother Eater] in the breeding area, <Draw 1>."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = getattr(effect0, "effect_source_permanent", None)
            return bool(
                permanent
                and (permanent.contains_card_name("King Drasil_7D6") or permanent.contains_card_name("Mother Eater"))
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get("player")
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT23-072 grant Rush, Raid, Reboot, and Blocker")
        effect1.set_effect_description(
            "[All Turns] When any of your Digimon with the [Royal Knight] or [CS] trait are played, by suspending this Digimon, 1 of the played Digimon gains <Rush>, <Raid>, <Reboot> and <Blocker> until your opponent's turn ends."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            owner_perm = card.permanent_of_this_card() if card else None
            if owner_perm is None:
                return False
            played_perm = context.get("played_permanent")
            if played_perm is None or played_perm is owner_perm or not played_perm.top_card:
                return False
            if played_perm.top_card.owner is not card.owner:
                return False
            traits = getattr(played_perm.top_card, "card_traits", []) or []
            return any(("Royal Knight" in trait) or ("CS" in trait) for trait in traits)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            played_perm = ctx.get("played_permanent")
            owner_perm = ctx.get("permanent")
            game = ctx.get("game")
            if not (played_perm and owner_perm and game):
                return
            if owner_perm.is_suspended:
                return
            owner_perm.suspend()
            expiry_turn = game.turn_count + 1
            for keyword in ("_is_rush", "_is_raid", "_is_reboot", "_is_blocker"):
                played_perm.grant_keyword(keyword, duration=expiry_turn)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT23-072 play 1 [King Drasil] from sources")
        effect2.set_effect_description(
            "[Breeding][Start of Your Main Phase] If this Digimon has 6 or more digivolution cards, you may play 1 Digimon card with [King Drasil] in its name from its digivolution cards without paying the cost."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = context.get("permanent")
            if permanent is None or card.owner.breeding_area is not permanent:
                return False
            return len(permanent.card_sources) >= 6 and any(
                "King Drasil" in name
                for source in permanent.card_sources[:-1]
                for name in getattr(source, "card_names", [])
            )

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get("player")
            perm = ctx.get("permanent")
            if not (player and perm):
                return
            for index, source in enumerate(list(perm.card_sources[:-1])):
                if any("King Drasil" in name for name in getattr(source, "card_names", [])):
                    del perm.card_sources[index]
                    player.play_card_from_source(source, pay_cost=False)
                    break

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
