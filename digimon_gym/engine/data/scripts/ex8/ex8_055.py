from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_055(CardScript):
    """EX8-055 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX8-055 Fragment")
        effect_fragment.set_effect_description(
            "<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its "
            "digivolution cards, it isn't deleted.)"
        )
        effect_fragment._is_fragment = True

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True
        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # [When Digivolving] [When Attacking] By trashing any 3 digivolution cards with the
        # [Mineral] or [Rock] trait from your Digimon, this Digimon unsuspends and gains
        # <Security A. +1> for the turn.
        def build_unsuspend_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
            effect.set_effect_name("EX8-055 By trashing 3 Mineral/Rock sources, unsuspend and gain Security A. +1")
            effect.set_effect_description(
                "[When Digivolving] By trashing any 3 digivolution cards with the [Mineral] or "
                "[Rock] trait from your Digimon, this Digimon unsuspends and gains "
                "<Security A. +1> for the turn."
                if is_when_digivolving else
                "[When Attacking] By trashing any 3 digivolution cards with the [Mineral] or "
                "[Rock] trait from your Digimon, this Digimon unsuspends and gains "
                "<Security A. +1> for the turn."
            )
            effect.is_optional = True
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                # Count available Mineral/Rock trait source cards across all your Digimon
                count = 0
                for p in owner.battle_area:
                    if not p.is_digimon:
                        continue
                    for src in p.card_sources:
                        if src is p.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            count += 1
                return count >= 3

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm):
                    return
                # Trash 3 Mineral/Rock trait source cards from any of your Digimon
                trashed_count = 0
                for p in list(player.battle_area):
                    if trashed_count >= 3:
                        break
                    if not p.is_digimon:
                        continue
                    for src in list(p.card_sources):
                        if trashed_count >= 3:
                            break
                        if src is p.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            p.card_sources.remove(src)
                            player.trash_cards.append(src)
                            trashed_count += 1
                if trashed_count >= 3:
                    perm.unsuspend()
                    perm._temp_sa_modifier += 1

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_on_attack=True))

        # [End of Your Turn] [Once Per Turn] You may place up to 3 cards with the [Mineral]
        # or [Rock] trait from your trash as this Digimon's bottom digivolution cards.
        effect_eot = ICardEffect()
        effect_eot.set_timing(EffectTiming.OnEndTurn)
        effect_eot.set_effect_name("EX8-055 Place up to 3 Mineral/Rock cards from trash as bottom sources")
        effect_eot.set_effect_description(
            "[End of Your Turn] [Once Per Turn] You may place up to 3 cards with the [Mineral] "
            "or [Rock] trait from your trash as this Digimon's bottom digivolution cards."
        )
        effect_eot.is_optional = True
        effect_eot.set_max_count_per_turn(1)
        effect_eot.set_hash_string("EOT_EX8_055")

        def condition_eot(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Must have at least 1 Mineral/Rock card in trash
            return any(
                'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                for c in owner.trash_cards
            )

        effect_eot.set_can_use_condition(condition_eot)

        def process_eot(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            placed = 0
            for source_card in list(player.trash_cards):
                if placed >= 3:
                    break
                traits = getattr(source_card, 'card_traits', [])
                if 'Mineral' not in traits and 'Rock' not in traits:
                    continue
                player.trash_cards.remove(source_card)
                perm.add_card_source_bottom(source_card)
                placed += 1

        effect_eot.set_on_process_callback(process_eot)
        effects.append(effect_eot)

        return effects
