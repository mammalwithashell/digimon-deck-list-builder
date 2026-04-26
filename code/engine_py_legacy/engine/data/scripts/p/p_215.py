from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_215(CardScript):
    """P-215 Icemon | Lv.4

    [When Moving] [On Play] [When Digivolving] By placing 1 level 4 or
    lower [Ice-Snow], [Mineral] or [Rock] trait card from your hand or
    trash as this Digimon's bottom digivolution card, until your opponent's
    turn ends, their effects can't return 1 of your [Ice-Snow], [Mineral]
    or [Rock] trait Digimon to hands or decks or affect it with
    De-Digivolve effects.

    Inherited: Blocker
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digi: from Lv.3 [Ice-Snow] trait for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("P-215 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "Ice-Snow"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and
                    any('Ice-Snow' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or []))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _has_valid_card_in_hand_or_trash(player) -> bool:
            """Check for Lv.4 or lower [Ice-Snow]/[Mineral]/[Rock] card."""
            if not player:
                return False
            for c in list(player.hand_cards) + list(player.trash_cards):
                level = getattr(c, 'level', None)
                if level is not None and level <= 4:
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Ice-Snow' in traits or 'Mineral' in traits or 'Rock' in traits:
                        return True
            return False

        def _place_and_protect(ctx: Dict[str, Any]):
            """Place 1 Lv.4- card as bottom source, then grant protection to 1 Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def card_filter(c):
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return 'Ice-Snow' in traits or 'Mineral' in traits or 'Rock' in traits

            has_hand = any(card_filter(c) for c in player.hand_cards)
            has_trash = any(card_filter(c) for c in player.trash_cards)

            if not (has_hand or has_trash):
                return

            def _after_placement():
                """After placing a card, let the player select 1 Digimon to protect."""
                _apply_protection(player, game)

            def place_from_hand(selected):
                if selected is None:
                    return
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    perm.add_card_source_bottom(selected)
                _after_placement()

            def place_from_trash(selected):
                if selected is None:
                    return
                if selected in player.trash_cards:
                    player.trash_cards.remove(selected)
                    perm.add_card_source_bottom(selected)
                _after_placement()

            if has_hand and has_trash:
                # Player chooses which zone to place from
                def on_branch(branch: int):
                    if branch == 0:
                        game.effect_select_hand_card(
                            player, card_filter, place_from_hand, is_optional=True,
                            prompt="Place 1 Lv.4- [Ice-Snow/Mineral/Rock] card from hand as bottom source.")
                    else:
                        from ....game.constants import SEL_TRASH_START
                        valid_indices = [
                            SEL_TRASH_START + i
                            for i, c in enumerate(player.trash_cards)
                            if card_filter(c)
                        ]

                        def on_trash_select(action_id: int):
                            from ....game.constants import SEL_TRASH_START as _TST
                            idx = action_id - _TST
                            if 0 <= idx < len(player.trash_cards):
                                place_from_trash(player.trash_cards[idx])

                        game.request_selection(
                            GamePhase.SelectTrash, player, on_trash_select,
                            valid_indices=valid_indices, is_optional=True,
                            prompt="Place 1 Lv.4- [Ice-Snow/Mineral/Rock] card from trash as bottom source.")

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="Place qualifying card from hand or trash?",
                    branch_labels=["From Hand", "From Trash"])

            elif has_hand:
                game.effect_select_hand_card(
                    player, card_filter, place_from_hand, is_optional=True,
                    prompt="Place 1 Lv.4- [Ice-Snow/Mineral/Rock] card from hand as bottom source.")
            else:
                # Trash only -- player selects which trash card
                from ....game.constants import SEL_TRASH_START
                valid_indices = [
                    SEL_TRASH_START + i
                    for i, c in enumerate(player.trash_cards)
                    if card_filter(c)
                ]

                def on_trash_select(action_id: int):
                    from ....game.constants import SEL_TRASH_START as _TST
                    idx = action_id - _TST
                    if 0 <= idx < len(player.trash_cards):
                        place_from_trash(player.trash_cards[idx])

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_select,
                    valid_indices=valid_indices, is_optional=True,
                    prompt="Place 1 Lv.4- [Ice-Snow/Mineral/Rock] card from trash as bottom source.")

        def _apply_protection(player, game):
            """Let the player select 1 [Ice-Snow/Mineral/Rock] Digimon to protect."""

            def prot_filter(p):
                if not p.is_digimon:
                    return False
                return (p.has_trait('Ice-Snow') or
                        p.has_trait('Mineral') or
                        p.has_trait('Rock'))

            def on_protect(target_perm):
                # Cannot be returned to hand by opponent effects
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_RETURN_TO_HAND,
                    condition=lambda p, c, tp=target_perm: p is tp,
                    expiry='end_of_opponent_turn',
                )
                # Cannot be returned to deck by opponent effects
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_RETURN_TO_DECK,
                    condition=lambda p, c, tp=target_perm: p is tp,
                    expiry='end_of_opponent_turn',
                )
                # Immune from De-Digivolve
                game.register_modifier(
                    target_perm, ModifierType.IMMUNE_FROM_DE_DIGIVOLVE,
                    condition=lambda p, c, tp=target_perm: p is tp,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_protect, filter_fn=prot_filter,
                is_optional=False,
                prompt="Select 1 [Ice-Snow/Mineral/Rock] Digimon to protect from return and De-Digivolve."
            )

        # [When Moving]
        eff_move = ICardEffect()
        eff_move.set_timing(EffectTiming.OnMove)
        eff_move.set_effect_name("P-215 Place card and protect from return/de-digivolve")
        eff_move.set_effect_description("[When Moving] Place 1 card and grant protection.")
        eff_move.is_optional = True

        def cond_move(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_valid_card_in_hand_or_trash(card.owner if card else None)

        eff_move.set_can_use_condition(cond_move)
        eff_move.set_on_process_callback(_place_and_protect)
        effects.append(eff_move)

        # [On Play]
        eff_play = ICardEffect()
        eff_play.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_play.set_effect_name("P-215 Place card and protect from return/de-digivolve")
        eff_play.set_effect_description("[On Play] Place 1 card and grant protection.")
        eff_play.is_on_play = True
        eff_play.is_optional = True

        def cond_play(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_valid_card_in_hand_or_trash(card.owner if card else None)

        eff_play.set_can_use_condition(cond_play)
        eff_play.set_on_process_callback(_place_and_protect)
        effects.append(eff_play)

        # [When Digivolving]
        eff_digi = ICardEffect()
        eff_digi.set_timing(EffectTiming.OnEnterFieldAnyone)
        eff_digi.set_effect_name("P-215 Place card and protect from return/de-digivolve")
        eff_digi.set_effect_description("[When Digivolving] Place 1 card and grant protection.")
        eff_digi.is_when_digivolving = True
        eff_digi.is_optional = True

        def cond_digi(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _has_valid_card_in_hand_or_trash(card.owner if card else None)

        eff_digi.set_can_use_condition(cond_digi)
        eff_digi.set_on_process_callback(_place_and_protect)
        effects.append(eff_digi)

        # Inherited: Blocker
        eff_blocker = ICardEffect()
        eff_blocker.set_effect_name("P-215 Blocker")
        eff_blocker.set_effect_description("Blocker")
        eff_blocker.is_inherited_effect = True
        eff_blocker._is_blocker = True

        def cond_blocker(context: Dict[str, Any]) -> bool:
            return True
        eff_blocker.set_can_use_condition(cond_blocker)
        effects.append(eff_blocker)

        return effects
