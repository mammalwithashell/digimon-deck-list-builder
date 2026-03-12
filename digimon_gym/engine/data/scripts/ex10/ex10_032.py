from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_032(CardScript):
    """EX10-032 Proganomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash as
        # any of your [Sunarizamon]'s bottom digivolution card, it digivolves into this card
        # for a digivolution cost of 3, ignoring digivolution requirements.
        # This is a Hand/Main effect — fires from OnStartMainPhase while the card is in hand.
        effect_hand = ICardEffect()
        effect_hand.set_timing(EffectTiming.OnStartMainPhase)
        effect_hand.set_effect_name("EX10-032 [Hand][Main] Digivolve Sunarizamon via Landramon from trash for 3")
        effect_hand.set_effect_description(
            "[Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash as "
            "any of your [Sunarizamon]'s bottom digivolution card, it digivolves into this card "
            "for a digivolution cost of 3, ignoring digivolution requirements."
        )
        effect_hand.is_optional = True

        def condition_hand(context: Dict[str, Any]) -> bool:
            # Card must be in hand (not on field)
            if card and card.permanent_of_this_card() is not None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Must have a card named [Close] on field
            has_close = any(
                p.contains_card_name('Close')
                for p in owner.battle_area
            )
            if not has_close:
                return False
            # Must have a Landramon in trash
            has_landramon = any(
                any('Landramon' in n for n in getattr(c, 'card_names', []))
                for c in owner.trash_cards
            )
            if not has_landramon:
                return False
            # Must have a Sunarizamon on field
            has_sunarizamon = any(
                p.contains_card_name('Sunarizamon')
                for p in owner.battle_area
            )
            if not has_sunarizamon:
                return False
            return True

        effect_hand.set_can_use_condition(condition_hand)

        def process_hand(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Find target Sunarizamon to digivolve
            sunarizamon_targets = [
                p for p in player.battle_area if p.contains_card_name('Sunarizamon')
            ]
            if not sunarizamon_targets:
                return

            # Find Landramon in trash
            landramon = None
            for c in player.trash_cards:
                if any('Landramon' in n for n in getattr(c, 'card_names', [])):
                    landramon = c
                    break
            if not landramon:
                return

            def on_target(target_perm):
                # Place Landramon from trash as bottom digivolution card
                player.trash_cards.remove(landramon)
                target_perm.add_card_source_bottom(landramon)
                # Now digivolve the selected Sunarizamon into this card for cost 3
                def this_card_filter(c):
                    return c is card
                game.effect_digivolve_from_hand(
                    player, target_perm,
                    filter_fn=this_card_filter,
                    cost_override=3,
                    ignore_requirements=True,
                    is_optional=False
                )

            game.effect_select_own_permanent(
                player, on_target,
                filter_fn=lambda p: p.contains_card_name('Sunarizamon'),
                is_optional=True
            )

        effect_hand.set_on_process_callback(process_hand)
        effects.append(effect_hand)

        # [On Play] [When Digivolving] [When Attacking] By trashing any 1 [Mineral] or [Rock]
        # trait card from your Digimon's digivolution cards, 1 of your such Digimon gains
        # <Collision>, <Piercing> and +3000 DP until your opponent's turn ends.
        def build_grant_effect(is_on_play: bool = False, is_when_digivolving: bool = False,
                               is_on_attack: bool = False):
            effect = ICardEffect()
            if is_on_attack:
                effect.set_timing(EffectTiming.OnUseAttack)
            else:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(
                "EX10-032 By trashing 1 [Mineral]/[Rock] source, 1 Digimon gains "
                "Collision, Piercing, +3000 DP until opponent's turn ends"
            )
            effect.set_effect_description(
                "[On Play] [When Digivolving] [When Attacking] By trashing any 1 [Mineral] or "
                "[Rock] trait card from your Digimon's digivolution cards, 1 of your such Digimon "
                "gains <Collision>, <Piercing> and +3000 DP until your opponent's turn ends."
            )
            effect.is_optional = True
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                # Must have at least one Mineral/Rock digimon with a source card to trash
                for p in owner.battle_area:
                    if not p.is_digimon:
                        continue
                    if not (p.has_trait('Mineral') or p.has_trait('Rock')):
                        continue
                    for src in p.card_sources:
                        if src is p.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            return True
                return False

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                from digimon_gym.engine.interfaces.modifiers import ModifierType
                expiry_turn = game.turn_count + 1

                def target_filter(p):
                    if not p.is_digimon:
                        return False
                    return p.has_trait('Mineral') or p.has_trait('Rock')

                def on_target(target_perm):
                    # Trash 1 Mineral/Rock source from the selected Digimon
                    trashed_card = None
                    for src in list(target_perm.card_sources):
                        if src is target_perm.top_card:
                            continue
                        traits = getattr(src, 'card_traits', [])
                        if 'Mineral' in traits or 'Rock' in traits:
                            trashed_card = src
                            break
                    if trashed_card is None:
                        return
                    target_perm.card_sources.remove(trashed_card)
                    player.trash_cards.append(trashed_card)
                    # Grant Collision, Piercing until end of opponent's turn
                    target_perm.grant_keyword('_is_collision', expiry_turn)
                    target_perm.grant_keyword('_is_piercing', expiry_turn)
                    # Grant +3000 DP until end of opponent's turn
                    game.register_modifier(
                        ModifierType.CHANGE_DP, target_perm,
                        value_fn=lambda: 3000,
                        expiry='end_of_opponent_turn'
                    )

                game.effect_select_own_permanent(
                    player, on_target,
                    filter_fn=target_filter,
                    is_optional=True
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_grant_effect(is_on_play=True))
        effects.append(build_grant_effect(is_when_digivolving=True))
        effects.append(build_grant_effect(is_on_attack=True))

        # Inherited: When effects trash this card from a [Mineral] or [Rock] trait Digimon's
        # digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect_inh.set_effect_name("EX10-032 inherited De-Digivolve 1")
        effect_inh.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's "
            "digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon."
        )
        effect_inh.is_inherited_effect = True

        def condition_inh(context: Dict[str, Any]) -> bool:
            return True

        effect_inh.set_can_use_condition(condition_inh)

        def process_inh(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False
            )

        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
