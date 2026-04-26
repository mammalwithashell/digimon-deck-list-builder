from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_070(CardScript):
    """EX5-070 X Antibody Proto Form"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Also Treated As [X Antibody]
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-070 Also treated as [X Antibody]")
        effect0.set_effect_description("Also Treated As")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Ignore Color Req (while you have a Digimon)
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-070 Ignore color requirements")
        effect1.set_effect_description("While you have a Digimon, you can ignore this card's color requirements.")
        effect1._ignore_color_requirements = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [Security] Add this card to hand
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX5-070 Security: Add to hand")
        effect2.set_effect_description("[Security] Add this card to its owner's hand.")
        effect2.is_security_effect = True
        effect2._security_add_to_hand = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                # Security effect adds this card to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Main] 1 of your Digimon without [X Antibody] in its digivolution cards
        # may digivolve into a Digimon card with the [X Antibody] trait in your hand
        # with the digivolution cost reduced by 1. If it did, place this card as its
        # bottom digivolution card.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OptionSkill)
        effect3.set_effect_name("EX5-070 Main: digivolve into X Antibody trait Digimon")
        effect3.set_effect_description(
            "[Main] 1 of your Digimon without [X Antibody] in its digivolution cards "
            "may digivolve into a Digimon card with the [X Antibody] trait in your hand "
            "with the digivolution cost reduced by 1. If it did, place this card as its "
            "bottom digivolution card."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _has_x_antibody_in_sources(perm):
                for cs in perm.card_sources[:-1]:
                    traits = getattr(cs, 'card_traits', []) or []
                    if any('X Antibody' in t for t in traits):
                        return True
                return False

            def perm_filter(p):
                return p.is_digimon and not _has_x_antibody_in_sources(p)

            def on_select_perm(target_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('X Antibody' in t for t in traits)

                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=1, is_optional=True)
                # Place this card as bottom digivolution card
                cur_perm = target_perm
                if cur_perm and cur_perm in player.battle_area and card:
                    if card in player.trash_cards:
                        player.trash_cards.remove(card)
                    cur_perm.card_sources.insert(0, card)

            game.effect_select_own_permanent(
                player, on_select_perm, filter_fn=perm_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Inherited: [All Turns] When this Digimon would leave the battle area
        # other than by one of your effects, from this Digimon's digivolution cards,
        # return 1 Digimon card to the hand and place 1 [X Antibody] on top of your
        # security stack.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("EX5-070 Leave field: return Digimon card, place X Antibody to security")
        effect4.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area other than by "
            "one of your effects, from this Digimon's digivolution cards, return 1 "
            "Digimon card to the hand and place 1 [X Antibody] on top of your security stack."
        )
        effect4.is_inherited_effect = True
        effect4.set_hash_string("ReturnCard_EX5_070")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only triggers from opponent's effects (not your own)
            source_player = context.get('source_player')
            if source_player and card and card.owner and source_player is card.owner:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return

            # Return 1 Digimon card from digivolution cards to hand
            digi_sources = [
                cs for cs in perm.card_sources[:-1]
                if getattr(cs, 'is_digimon', False)
            ]
            if digi_sources:
                return_card = digi_sources[0]
                if return_card in perm.card_sources:
                    perm.card_sources.remove(return_card)
                    player.hand_cards.append(return_card)

            # Place 1 [X Antibody] from digivolution cards on top of security
            x_sources = [
                cs for cs in perm.card_sources[:-1]
                if any('X Antibody' in t for t in (getattr(cs, 'card_traits', []) or []))
            ]
            if x_sources:
                x_card = x_sources[0]
                if x_card in perm.card_sources:
                    perm.card_sources.remove(x_card)
                    player.security_cards.insert(0, x_card)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
