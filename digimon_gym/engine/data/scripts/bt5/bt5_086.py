from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_086(CardScript):
    """BT5-086 Omnimon | Lv.7 White (Holy Warrior, Royal Knight) | 14000 DP

    <Blitz> (This Digimon can attack when your opponent has 1 or more
        memory.)
    [When Digivolving] Unsuspend this Digimon.
    [All Turns] If an opponent's effect would delete this Digimon or
        return it to its owner's hand or deck, you may prevent it from
        leaving play by trashing a level 6 Digimon card in this
        Digimon's digivolution cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Blitz> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT5-086 Blitz")
        effect0.set_effect_description("<Blitz>")
        effect0.is_when_digivolving = True
        effect0._is_blitz = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Unsuspend this Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT5-086 When Digivolving: Unsuspend")
        effect1.set_effect_description(
            "[When Digivolving] Unsuspend this Digimon."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if perm not in owner.battle_area:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            perm = card.permanent_of_this_card() if card else None
            if perm and perm.is_suspended:
                perm.unsuspend()
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] Prevent deletion/bounce by trashing
        #     a Lv.6 Digimon from digivolution cards ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name("BT5-086 Prevent leaving play")
        effect2.set_effect_description(
            "[All Turns] If an opponent's effect would delete this Digimon "
            "or return it to its owner's hand or deck, you may prevent it "
            "from leaving play by trashing a level 6 Digimon card in this "
            "Digimon's digivolution cards."
        )
        effect2.is_optional = True
        effect2.set_hash_string("Substitute_BT5_086")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if perm not in owner.battle_area:
                return False
            # Must be triggered by an opponent's effect
            source_effect = context.get('source_effect')
            if source_effect is not None:
                src_card = getattr(source_effect, 'effect_source_card', None)
                if src_card is not None and src_card.owner == owner:
                    return False
            # The permanent being removed must be this one
            event_perm = context.get('permanent')
            if event_perm is not None and event_perm is not perm:
                return False
            # Need at least 1 Lv.6 Digimon in digivolution cards
            digi_cards = getattr(perm, 'digivolution_cards', []) or []
            if not digi_cards:
                digi_cards = getattr(perm, 'card_sources', [])[1:] if len(getattr(perm, 'card_sources', [])) > 1 else []
            has_lv6 = any(
                getattr(c, 'is_digimon', False) and
                getattr(c, 'level', None) == 6
                for c in digi_cards
            )
            return has_lv6
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash a Lv.6 Digimon from digivolution cards to prevent leaving play."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return

            # Find a Lv.6 Digimon in digivolution cards
            digi_cards = getattr(perm, 'digivolution_cards', []) or []
            if not digi_cards:
                digi_cards = getattr(perm, 'card_sources', [])[1:] if len(getattr(perm, 'card_sources', [])) > 1 else []

            target = None
            for c in digi_cards:
                if getattr(c, 'is_digimon', False) and getattr(c, 'level', None) == 6:
                    target = c
                    break

            if target is not None:
                # Trash the selected digivolution card
                if hasattr(perm, 'digivolution_cards') and target in perm.digivolution_cards:
                    perm.digivolution_cards.remove(target)
                elif hasattr(perm, 'card_sources') and target in perm.card_sources:
                    perm.card_sources.remove(target)
                player.trash_cards.append(target)

                # Prevent the pending deletion/return
                perm.will_be_remove_field = False

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
