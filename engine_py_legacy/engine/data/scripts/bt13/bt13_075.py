from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_075(CardScript):
    """BT13-075 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_on_enter_effect(is_when_digivolving: bool) -> ICardEffect:
            """Factory for On Play / When Digivolving shared effect."""
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT13-075 Place 1 card from trash to digi-stack bottom, then opponent cost-10+ Digimon can't attack players")
            if is_when_digivolving:
                effect.is_when_digivolving = True
                effect.set_effect_description(
                    "[When Digivolving] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait "
                    "from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost "
                    "10 or higher Digimon can't attack players until the end of their turn."
                )
            else:
                effect.is_on_play = True
                effect.set_effect_description(
                    "[On Play] By placing 1 Digimon card with the [X Antibody] or [Royal Knight] trait "
                    "from your trash as this Digimon's bottom digivolution card, all of your opponent's play cost "
                    "10 or higher Digimon can't attack players until the end of their turn."
                )
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                # Must have at least one qualifying Digimon card in trash
                owner = card.owner if card else None
                if not owner:
                    return False
                for c in owner.trash_cards:
                    if not c.is_digimon:
                        continue
                    traits = getattr(c, 'card_traits', []) or []
                    if any('X Antibody' in t for t in traits) or any('Royal Knight' in t for t in traits):
                        return True
                return False

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return
                if perm is None:
                    return

                # Step 1: Select 1 qualifying Digimon card from trash and place at bottom of digi-stack
                def trash_filter(c):
                    if not c.is_digimon:
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return (any('X Antibody' in t for t in traits) or
                            any('Royal Knight' in t for t in traits))

                from digimon_gym.engine.game.constants import SEL_TRASH_START
                valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards) if trash_filter(c)]
                if not valid:
                    return

                def on_trash_selected(idx: int):
                    if not (0 <= idx < len(player.trash_cards)):
                        return
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    perm.add_card_source_bottom(chosen)

                    # Step 2: Apply CANNOT_ATTACK to ALL opponent Digimon with play cost >= 10
                    from digimon_gym.engine.interfaces.modifiers import ModifierType
                    enemy = player.enemy
                    if not enemy:
                        return
                    for opp_perm in list(enemy.battle_area):
                        if not opp_perm.is_digimon:
                            continue
                        top = opp_perm.top_card
                        if top is None:
                            continue
                        play_cost = top.get_cost_itself
                        has_play_cost = getattr(top, 'has_play_cost', True)
                        if has_play_cost and play_cost >= 10:
                            game.register_modifier(
                                opp_perm,
                                ModifierType.CANNOT_ATTACK,
                                value_fn=lambda: True,
                                expiry='end_of_opponent_turn',
                            )

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected, valid,
                    is_optional=False,
                    prompt="Select 1 Digimon card with [X Antibody] or [Royal Knight] trait from your trash to place as a bottom digivolution card."
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_on_enter_effect(is_when_digivolving=False))
        effects.append(_make_on_enter_effect(is_when_digivolving=True))

        # [All Turns][Once Per Turn] When this Digimon would leave the battle area by an effect,
        # by returning 1 Digimon card with the [X Antibody] or [Royal Knight] trait from this
        # Digimon's digivolution cards to the bottom of your deck, prevent it from leaving play.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT13-075 Return digi-card to deck bottom to prevent leaving the battle area")
        effect2.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon would leave the battle area by an effect, "
            "by returning 1 Digimon card with the [X Antibody] or [Royal Knight] trait from this "
            "Digimon's digivolution cards to the bottom of your deck, prevent it from leaving play."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_BT13_075")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Must have at least one qualifying card in digi-stack
            for c in perm.card_sources:
                traits = getattr(c, 'card_traits', []) or []
                if any('X Antibody' in t for t in traits) or any('Royal Knight' in t for t in traits):
                    return True
            return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Find qualifying cards in the digi-stack
            def stack_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return (any('X Antibody' in t for t in traits) or
                        any('Royal Knight' in t for t in traits))

            qualifying = [c for c in perm.card_sources if stack_filter(c)]
            if not qualifying:
                return

            if len(qualifying) == 1:
                # Only one candidate, no selection needed
                chosen = qualifying[0]
                if chosen in perm.card_sources:
                    perm.card_sources.remove(chosen)
                player.library_cards.append(chosen)
            else:
                # Multiple candidates: offer selection
                from ....game.constants import SEL_HAND_START
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if stack_filter(cs):
                        valid.append(SEL_HAND_START + i)
                if not valid:
                    return

                def on_select(action_id: int):
                    idx = action_id - SEL_HAND_START
                    if not (0 <= idx < len(perm.card_sources)):
                        return
                    chosen = perm.card_sources[idx]
                    perm.card_sources.remove(chosen)
                    player.library_cards.append(chosen)

                game.request_selection(
                    GamePhase.SelectSource, player, on_select, valid,
                    is_optional=False,
                    prompt="Select 1 [X Antibody] or [Royal Knight] Digimon card from digivolution cards to return to deck bottom."
                )

            # Prevention of removal is signaled to engine by not doing anything else;
            # engine checks effect2.is_optional and the WhenRemoveField hook to cancel removal.

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
