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

        # ---------------------------------------------------------------
        # effect0: [Hand][Main] By paying 3 cost and placing this card as
        # the bottom digivolution card of your [King Drasil_7D6] or
        # [Mother Eater] in the breeding area, <Draw 1>.
        # ---------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("BT23-072 draw 1")
        effect0.set_effect_description(
            "[Hand][Main] By paying 3 cost and placing this card as the bottom "
            "digivolution card of your [King Drasil_7D6] or [Mother Eater] in "
            "the breeding area, <Draw 1>."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Card must be in hand (no permanent)
            if not card or card.permanent_of_this_card() is not None:
                return False
            # Player must have a King Drasil_7D6 or Mother Eater in breeding area
            owner = card.owner
            if not owner or owner.breeding_area is None:
                return False
            breeding = owner.breeding_area
            return (breeding.contains_card_name("King Drasil_7D6")
                    or breeding.contains_card_name("Mother Eater"))

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # descriptive-tagged: hand_main_activation
            # This is a hand-activated ability: pay 3 memory, place this card
            # at index 0 (bottom) of the breeding permanent's card_sources,
            # remove from hand, then draw 1. The engine does not currently
            # support hand-activated [Main] abilities natively.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # effect1: [All Turns] When any of your Digimon with the [Royal
        # Knight] or [CS] trait are played, by suspending this Digimon,
        # 1 of the played Digimon gains <Rush>, <Raid>, <Reboot> and
        # <Blocker> until your opponent's turn ends.
        # ---------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT23-072 grant Rush, Raid, Reboot, and Blocker")
        effect1.set_effect_description(
            "[All Turns] When any of your Digimon with the [Royal Knight] or "
            "[CS] trait are played, by suspending this Digimon, 1 of the played "
            "Digimon gains <Rush>, <Raid>, <Reboot> and <Blocker> until your "
            "opponent's turn ends."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            owner_perm = card.permanent_of_this_card() if card else None
            if owner_perm is None:
                return False
            # Cannot activate if already suspended ("by suspending")
            if owner_perm.is_suspended:
                return False
            played_perm = context.get("played_permanent")
            if played_perm is None or played_perm is owner_perm or not played_perm.top_card:
                return False
            # Must be one of our Digimon
            if played_perm.top_card.owner is not card.owner:
                return False
            # Played Digimon must have Royal Knight or CS trait
            traits = getattr(played_perm.top_card, "card_traits", []) or []
            return any(("Royal Knight" in trait) or ("CS" in trait) for trait in traits)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            played_perm = ctx.get("played_permanent")
            owner_perm = ctx.get("permanent")
            game = ctx.get("game")
            if not (played_perm and owner_perm and game):
                return
            # Guard: can't pay cost if already suspended
            if owner_perm.is_suspended:
                return
            # Pay cost: suspend this Digimon
            owner_perm.suspend()
            # Grant keywords until opponent's turn ends
            # duration is an absolute turn number; opponent's turn = current + 1
            expiry_turn = game.turn_count + 1
            for keyword in ("_is_rush", "_is_raid", "_is_reboot", "_is_blocker"):
                played_perm.grant_keyword(keyword, duration=expiry_turn)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # effect2 (inherited): [Breeding][Start of Your Main Phase] If this
        # Digimon has 6 or more digivolution cards, you may play 1 Digimon
        # card with [King Drasil] in its name from its digivolution cards
        # without paying the cost.
        # ---------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT23-072 play 1 [King Drasil] from sources")
        effect2.set_effect_description(
            "[Breeding][Start of Your Main Phase] If this Digimon has 6 or more "
            "digivolution cards, you may play 1 Digimon card with [King Drasil] "
            "in its name from its digivolution cards without paying the cost."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = context.get("permanent")
            if permanent is None or card.owner.breeding_area is not permanent:
                return False
            # 6 or more digivolution cards = 7+ total in card_sources
            # (card_sources includes top card + digi cards underneath)
            if len(permanent.card_sources) < 7:
                return False
            # Must have at least one King Drasil card in digivolution cards
            return any(
                "King Drasil" in name
                for source in permanent.card_sources[:-1]
                for name in getattr(source, "card_names", [])
            )

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get("player")
            perm = ctx.get("permanent")
            game = ctx.get("game")
            if not (player and perm and game):
                return
            # Find a King Drasil Digimon card in digivolution cards and play it
            for index, source in enumerate(list(perm.card_sources[:-1])):
                if not getattr(source, 'is_digimon', False):
                    continue
                if any("King Drasil" in name for name in getattr(source, "card_names", [])):
                    # Remove from digivolution cards
                    perm.card_sources.remove(source)
                    # Play without paying cost
                    played_perm = player.play_card_from_source(source, pay_cost=False)
                    # Fire on-play effects
                    if played_perm:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": source, "played_permanent": played_perm,
                             "event_player": player},
                        )
                    break

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
