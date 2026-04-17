from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_030(CardScript):
    """EX6-030 Dominimon | Lv.6 Yellow Dominion

    [When Digivolving] Search your security stack. You may play 1 level 5 or
        lower Digimon card with the [Angel] or [Archangel] trait among them
        without paying the cost. Then, shuffle your security stack, and 1 of
        your opponent's Digimon gets -7000 DP until the end of the turn.
    [All Turns] When one of your Digimon with the [Angel], [Archangel] or
        [Three Great Angels] trait would leave the battle area other than by
        one of your effects or a battle, by trashing the top card of your
        security stack, prevent them from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─────────────────────────────────────────────────────────────
        # Effect 0: [When Digivolving]
        # ─────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "EX6-030 [WD] Play Angel/Archangel from security, -7000 DP opp"
        )
        effect0.set_effect_description(
            "[When Digivolving] Search your security stack. You may play 1 "
            "level 5 or lower Digimon card with the [Angel] or [Archangel] "
            "trait among them without paying the cost. Then, shuffle your "
            "security stack, and 1 of your opponent's Digimon gets -7000 DP "
            "until the end of the turn."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def _is_lv5_or_lower_angel(c):
            if not getattr(c, 'is_digimon', False):
                return False
            level = getattr(c, 'level', None)
            if level is None or level > 5:
                return False
            traits = getattr(c, 'card_traits', None) or []
            return 'Angel' in traits or 'Archangel' in traits

        def _shuffle_security(player):
            import random
            random.shuffle(player.security_cards)

        def _select_dp_target(player, game, own_perm):
            """Second step: pick 1 opponent Digimon and apply -7000 DP EoT."""
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon]
            if not targets:
                return

            def target_filter(p):
                return p.is_digimon

            def on_dp_target(target_perm):
                from ....interfaces.modifiers import ModifierType
                # target-equality guard: value_fn only fires for this exact
                # permanent, preventing leakage to other Digimon.
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    condition=lambda p, ctx, _t=target_perm: p is _t,
                    value_fn=lambda current, p, ctx: current - 7000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_dp_target,
                filter_fn=target_filter, is_optional=False,
                prompt="Select 1 of your opponent's Digimon to get -7000 DP.",
            )

        def _after_security_selection(player, game, own_perm, selected_card):
            """Runs after the player picks (or declines) a security card."""
            if selected_card is not None:
                # Play the selected card from security without paying cost
                removed = player.remove_from_security(selected_card)
                if removed is not None:
                    played_perm = player.play_card_from_source(
                        removed, pay_cost=False
                    )
                    if played_perm and game:
                        game.logger.log(
                            f"[Effect] {player.player_name} played "
                            f"{game._card_ref(removed)} from security"
                        )
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {
                                "played_card": removed,
                                "played_permanent": played_perm,
                                "event_player": player,
                                "is_effect_play": True,
                            },
                        )
                        game._fire_play_observers(played_perm, player)
            # "Then, shuffle your security stack"
            _shuffle_security(player)
            # "and 1 of your opponent's Digimon gets -7000 DP..."
            _select_dp_target(player, game, own_perm)

        def process0(ctx: Dict[str, Any]):
            from ....game.constants import SEL_MY_SECURITY_START
            from ....data.enums import GamePhase
            player = ctx.get('player')
            game = ctx.get('game')
            own_perm = ctx.get('permanent')
            if not (player and game):
                return

            # Build valid selection indices for the security stack
            valid = [
                SEL_MY_SECURITY_START + i
                for i, c in enumerate(player.security_cards)
                if _is_lv5_or_lower_angel(c)
            ]

            if valid:
                def on_select(action_id: int):
                    idx = action_id - SEL_MY_SECURITY_START
                    selected_card = None
                    if 0 <= idx < len(player.security_cards):
                        selected_card = player.security_cards[idx]
                    _after_security_selection(
                        player, game, own_perm, selected_card
                    )

                def on_decline():
                    _after_security_selection(
                        player, game, own_perm, None
                    )

                game.request_selection(
                    GamePhase.SelectSecurity, player, on_select,
                    valid, is_optional=True,
                    prompt=(
                        "You may play 1 level 5 or lower Digimon with the "
                        "[Angel] or [Archangel] trait from your security "
                        "stack without paying the cost."
                    ),
                    on_decline=on_decline,
                )
            else:
                # No valid security targets — skip straight to shuffle + DP.
                _shuffle_security(player)
                _select_dp_target(player, game, own_perm)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─────────────────────────────────────────────────────────────
        # Effect 1: [All Turns] Prevent Angel/Archangel/TGA from leaving
        # ─────────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect1.set_effect_name(
            "EX6-030 [All Turns] Trash top security to prevent Angel leaving"
        )
        effect1.set_effect_description(
            "[All Turns] When one of your Digimon with the [Angel], "
            "[Archangel] or [Three Great Angels] trait would leave the "
            "battle area other than by one of your effects or a battle, "
            "by trashing the top card of your security stack, prevent "
            "them from leaving."
        )
        effect1.is_optional = True

        def _is_angelic_trait(perm) -> bool:
            if perm is None or not perm.is_digimon:
                return False
            top = perm.top_card
            if top is None:
                return False
            traits = getattr(top, 'card_traits', None) or []
            return (
                'Angel' in traits
                or 'Archangel' in traits
                or 'Three Great Angels' in traits
            )

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence: Dominimon itself must be on the field
            if card is None:
                return False
            own_perm = card.permanent_of_this_card()
            if own_perm is None:
                return False
            # The permanent that would be deleted — fallback resolution
            leaving = (
                context.get('event_permanent')
                or context.get('permanent')
            )
            if leaving is None:
                return False
            # Must be own side (owned by the card's owner)
            owner = getattr(card, 'owner', None)
            if owner is None:
                return False
            # "one of YOUR Digimon": must belong to card.owner
            if leaving not in owner.battle_area:
                return False
            # Trait filter: Angel / Archangel / Three Great Angels
            if not _is_angelic_trait(leaving):
                return False
            # "other than by one of your effects or a battle"
            if context.get('is_battle', False):
                return False
            removal_cause = context.get('removal_cause')
            if removal_cause == 'battle':
                return False
            if context.get('is_own_effect', False):
                return False
            # Cost requirement: at least 1 security card to trash
            if len(owner.security_cards) < 1:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if game is None:
                return
            owner = getattr(card, 'owner', None) if card else None
            if owner is None:
                owner = player
            if owner is None:
                return
            leaving = (
                ctx.get('event_permanent')
                or ctx.get('permanent')
            )
            if leaving is None:
                return
            if len(owner.security_cards) < 1:
                return
            # Pay cost: trash the top security card
            top_sec = owner.security_cards[0]
            removed = owner.remove_from_security(top_sec)
            if removed is not None:
                owner.trash_cards.append(removed)
            # Prevent the leaving permanent from being removed
            leaving._will_not_be_removed = True
            if game.logger:
                game.logger.log(
                    "[EX6-030] Trashed top security to prevent "
                    "an Angel from leaving the battle area."
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
