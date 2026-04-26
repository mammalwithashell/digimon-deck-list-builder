from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_056(CardScript):
    """BT20-056 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: Barrier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-056 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── On Play / When Digivolving shared effect factory ─────────────────
        def _make_on_enter_effect(is_when_digivolving: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT20-056 Recovery +1, then if during attack breed area Digimon may digivolve into Chronicle")
            if is_when_digivolving:
                effect.is_when_digivolving = True
                effect.set_effect_description(
                    "[When Digivolving] Recovery +1 (Deck). Then, if during an attack, 1 of your Digimon "
                    "in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon "
                    "card in the hand or trash without paying the cost."
                )
            else:
                effect.is_on_play = True
                effect.set_effect_description(
                    "[On Play] Recovery +1 (Deck). Then, if during an attack, 1 of your Digimon in the "
                    "breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card "
                    "in the hand or trash without paying the cost."
                )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not player:
                    return

                # Recovery +1 (Deck)
                if player.library_cards:
                    player.recovery(1)

                if not (game and perm):
                    return

                # "if during an attack" — check if any Digimon is currently attacking
                is_attacking = any(
                    getattr(p, 'is_attacking', False) or getattr(p, 'is_suspended', False)
                    for p in player.battle_area
                )
                # Also check game-level attack state if available
                game_is_attacking = getattr(game, 'is_attacking', False)
                if not (is_attacking or game_is_attacking):
                    return

                # Must have a Digimon in the breeding area
                breeding = player.breeding_area
                if breeding is None:
                    return

                def chronicle_filter(c):
                    if not c.is_digimon:
                        return False
                    lvl = getattr(c, 'level', None)
                    if lvl is None or lvl > 6:
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Chronicle' in t for t in traits)

                # Check both hand and trash for Chronicle candidates
                hand_candidates = [c for c in player.hand_cards if chronicle_filter(c)]
                trash_candidates = [c for c in player.trash_cards if chronicle_filter(c)]

                can_hand = len(hand_candidates) > 0
                can_trash = len(trash_candidates) > 0

                if not (can_hand or can_trash):
                    return

                def _digi_from_hand():
                    game.effect_digivolve_from_hand(
                        player, breeding, chronicle_filter,
                        cost_override=0, is_optional=True,
                        prompt="Select a Chronicle Digimon (Lv6 or lower) from hand to digivolve breeding Digimon."
                    )

                def _digi_from_trash():
                    from ....game.constants import SEL_TRASH_START
                    valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards)
                             if chronicle_filter(c)]
                    if not valid:
                        return

                    def on_trash_select(action_id: int):
                        idx = action_id - SEL_TRASH_START
                        if not (0 <= idx < len(player.trash_cards)):
                            return
                        chosen = player.trash_cards[idx]
                        player.trash_cards.remove(chosen)
                        breeding.add_card_source(chosen)
                        breeding.turn_digivolved = game.turn_count

                    game.request_selection(
                        GamePhase.SelectTrash, player, on_trash_select, valid,
                        is_optional=True,
                        prompt="Select a Chronicle Digimon (Lv6 or lower) from trash to digivolve breeding Digimon."
                    )

                if can_hand and can_trash:
                    def on_branch(branch_index: int):
                        if branch_index == 0:
                            _digi_from_hand()
                        elif branch_index == 1:
                            _digi_from_trash()

                    game.effect_choose_branch(
                        player, 2, on_branch,
                        prompt="Digivolve breeding Digimon into Chronicle from which zone?",
                        branch_labels=["From hand", "From trash"],
                        is_optional=True,
                    )
                elif can_hand:
                    _digi_from_hand()
                elif can_trash:
                    _digi_from_trash()

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_on_enter_effect(is_when_digivolving=False))
        effects.append(_make_on_enter_effect(is_when_digivolving=True))

        # [All Turns][Once Per Turn] When security stacks are removed from,
        # 1 of your opponent's Digimon gets -8000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT20-056 1 of your opponent's Digimon gets -8000 DP for the turn")
        effect3.set_effect_description(
            "[All Turns] (Once Per Turn) When security stacks are removed from, "
            "1 of your opponent's Digimon gets -8000 DP for the turn."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("RemovedSec_BT20_056")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType

            def on_target(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda current, p, c: current - 8000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon and p.dp is not None,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give -8000 DP for the turn.",
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [All Turns][Once Per Turn] When this Digimon would leave the battle area other than by
        # your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card,
        # it doesn't leave.  (Inherited — fires when digivolved under Alphamon: Ouryuken)
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("BT20-056 ESS: Trash your top security card to prevent leaving the battle area")
        effect4.set_effect_description(
            "[All Turns] (Once Per Turn) When this Digimon would leave the battle area other than by "
            "your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card, "
            "it doesn't leave."
        )
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("TrashSecurityToStay_BT20_056")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Only fires if this Digimon's top card is Alphamon: Ouryuken
            if not perm.contains_card_name('Alphamon: Ouryuken'):
                return False
            # "other than by your effects" — skip if removal is initiated by own effect
            if context.get('is_own_effect', False):
                return False
            # Must have a security card to pay the cost
            owner = card.owner if card else None
            if not owner or not owner.security_cards:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player:
                return
            # Cost: trash YOUR top security card
            if player.security_cards:
                player.trash_security_card(player.security_cards[0])
            # Removal prevention is handled by the engine's WhenRemoveField hook
            # when this optional effect fires and the process completes successfully.

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
