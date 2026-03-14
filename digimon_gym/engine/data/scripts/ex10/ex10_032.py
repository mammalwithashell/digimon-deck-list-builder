from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_032(CardScript):
    """EX10-032 Proganomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash as
        # any of your [Sunarizamon]'s bottom digivolution card, it digivolves into this card
        # for a digivolution cost of 3, ignoring digivolution requirements.
        effect_hand = ICardEffect()
        effect_hand._is_hand_main = True
        effect_hand.set_effect_name("EX10-032 [Hand][Main] Place Landramon, digivolve onto Sunarizamon")
        effect_hand.set_effect_description(
            "[Hand] [Main] If you have [Close], by placing 1 [Landramon] from your trash as "
            "any of your [Sunarizamon]'s bottom digivolution card, it digivolves into this card "
            "for a digivolution cost of 3, ignoring digivolution requirements."
        )

        def condition_hand(context: Dict[str, Any]) -> bool:
            if card.permanent_of_this_card() is not None:
                return False  # must be in hand
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            # Must have Close tamer on field
            has_close = any(
                p.contains_card_name('Close')
                for p in player.battle_area
            )
            if not has_close:
                return False
            # Must have Landramon in trash
            has_landramon = any(
                any('Landramon' in (n or '') for n in (getattr(c, 'card_names', []) or []))
                for c in player.trash_cards
            )
            if not has_landramon:
                return False
            # Must have Sunarizamon on field
            has_sunarizamon = any(
                p.contains_card_name('Sunarizamon')
                for p in player.battle_area
            )
            if not has_sunarizamon:
                return False
            return True

        effect_hand.set_can_use_condition(condition_hand)

        def process_hand(ctx: Dict[str, Any]):
            game = ctx.get('game')
            player = ctx.get('player')
            hand_card = ctx.get('card')
            if not (game and player and hand_card):
                return

            def _is_landramon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Landramon' in (n or '') for n in names)

            # Let agent choose which Landramon from trash
            def on_landramon_selected(trash_idx):
                if trash_idx >= len(player.trash_cards):
                    return
                landramon = player.trash_cards[trash_idx]

                # Select a Sunarizamon on field to digivolve onto
                def on_sunarizamon_selected(target_perm):
                    # Place Landramon from trash as bottom digi card
                    if landramon in player.trash_cards:
                        player.trash_cards.remove(landramon)
                    target_perm.add_card_source_bottom(landramon)
                    game.logger.log(
                        f"[Hand][Main] Placed {game._card_ref(landramon)} "
                        f"under {game._perm_ref(target_perm)}")

                    # Remove Proganomon from hand and digivolve
                    if hand_card in player.hand_cards:
                        player.hand_cards.remove(hand_card)
                    target_perm.add_card_source(hand_card)
                    target_perm.turn_digivolved = game.turn_count
                    player.lose_memory(3)
                    game.logger.log(
                        f"[Hand][Main] Digivolved {game._card_ref(hand_card)} "
                        f"onto {game._perm_ref(target_perm)} (cost: 3)")
                    player.draw()
                    game.execute_effects(EffectTiming.WhenDigivolving,
                                         {"digivolved_permanent": target_perm})

                game.effect_select_own_permanent(
                    player, on_sunarizamon_selected,
                    filter_fn=lambda p: p.contains_card_name('Sunarizamon'),
                    is_optional=False,
                    prompt="Select a [Sunarizamon] to digivolve into Proganomon.")

            # Build valid trash indices for Landramon cards
            _SEL_TRASH_START = 130
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _is_landramon(c):
                    valid_trash.append(_SEL_TRASH_START + i)
            if not valid_trash:
                return

            def _on_trash_action(idx):
                # _decode_trash_selection already subtracts SEL_TRASH_START
                on_landramon_selected(idx)

            game.request_selection(
                GamePhase.SelectTrash, player, _on_trash_action,
                valid_trash, is_optional=False,
                prompt="Select a [Landramon] from your trash to place under [Sunarizamon].")

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
                    # Use trash_digivolution_cards for proper engine handling
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
                    if trashed_card in target_perm.card_sources:
                        target_perm.card_sources.remove(trashed_card)
                    player.trash_cards.append(trashed_card)
                    game.execute_effects(EffectTiming.OnDigivolutionCardDiscarded,
                                         {'trashed_cards': [trashed_card],
                                          'permanent': target_perm})
                    # Grant Collision, Piercing until end of opponent's turn
                    target_perm.grant_keyword('_is_collision', expiry_turn)
                    target_perm.grant_keyword('_is_piercing', expiry_turn)
                    # Grant +3000 DP until end of opponent's turn
                    game.register_modifier(
                        target_perm,
                        ModifierType.CHANGE_DP,
                        value_fn=lambda cur, t, c: cur + 3000,
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
            # Check this card was the one trashed
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the permanent has [Mineral] or [Rock] trait
            permanent = context.get('permanent')
            if permanent is None:
                return False
            if not (permanent.has_trait('Mineral') or permanent.has_trait('Rock')):
                return False
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
