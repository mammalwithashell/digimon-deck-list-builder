from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_059(CardScript):
    """BT23-059 Justimon: Blitz Arm | Lv.6

    <Blocker>
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
    By trashing 1 Option card in the battle area, delete 1 of your
    opponent's Digimon with the lowest play cost.
    [All Turns] [Once Per Turn] When Option cards in the battle area
    are trashed, this Digimon unsuspends. Then, your opponent's
    Digimon's effects don't affect this Digimon for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Shared state for [Once Per Turn] across the three triggers
        # (On Play / When Digivolving / When Attacking) — the card text
        # says "[Once Per Turn]" applies to the combined ability.
        _shared_state = {
            'option_delete_used_turn': -1,
            'unsuspend_used_turn': -1,
        }

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-059 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Justimon: Accel Arm]/[Justimon: Critical Arm] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Justimon: Accel Arm"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Justimon: Accel Arm') or permanent.contains_card_name('Justimon: Critical Arm'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-059 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 w/[CS] trait for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_level = 5
        effect1._alt_digi_trait = "CS"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-059 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Shared builder for On Play / When Digivolving / When Attacking
        # trigger that trashes an Option card and deletes opponent's
        # lowest-cost Digimon. All three share a [Once Per Turn] counter
        # keyed on the current turn number via _shared_state.
        def _make_option_delete_effect(trigger_label: str, effect_timing,
                                        flag_attr: str):
            effect = ICardEffect()
            effect.set_timing(effect_timing)
            effect.set_effect_name(
                f"BT23-059 {trigger_label}: Trash 1 Option, delete 1 Digimon")
            effect.set_effect_description(
                f"{trigger_label} [Once Per Turn] By trashing 1 Option card in "
                "the battle area, delete 1 of your opponent's Digimon with "
                "the lowest play cost.")
            effect.set_hash_string("BT23_059_OPT_DELETE")
            effect.set_max_count_per_turn(1)
            setattr(effect, flag_attr, True)
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                # Shared once-per-turn check: if any of the three has been used
                # this game turn, none of them can activate again.
                g = context.get('game')
                if g and _shared_state['option_delete_used_turn'] == getattr(
                        g, 'turn_count', -1):
                    return False
                # Must have at least 1 Option card in own battle area to trash
                player = context.get('player')
                if not player:
                    player = card.owner if card else None
                if player:
                    has_option = any(p.is_option for p in player.battle_area)
                    if not has_option:
                        return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return
                # Mark the shared counter immediately
                _shared_state['option_delete_used_turn'] = game.turn_count

                def option_filter(p):
                    return p.is_option

                def on_option_trashed(opt_perm):
                    if opt_perm in player.battle_area:
                        player.battle_area.remove(opt_perm)
                        game.cleanup_modifiers_for_permanent(opt_perm)
                        for cs in opt_perm.card_sources:
                            player.trash_cards.append(cs)

                    # Manually fire the "When Option cards are trashed"
                    # trigger (effect6) since it's not a standard timing
                    _try_fire_unsuspend(player, game)

                    # Then delete 1 opponent's Digimon with lowest play cost
                    enemy = player.enemy if player else None
                    if not enemy:
                        return
                    costs = [
                        getattr(p.top_card, 'play_cost', 99) or 99
                        for p in enemy.battle_area if p.is_digimon
                    ]
                    if not costs:
                        return
                    min_cost = min(costs)

                    def target_filter(p):
                        return p.is_digimon and (getattr(
                            p.top_card, 'play_cost', 99) or 99) <= min_cost

                    def on_delete(target_perm):
                        if enemy:
                            enemy.delete_permanent(target_perm)

                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter,
                        is_optional=False)

                game.effect_select_own_permanent(
                    player, on_option_trashed, filter_fn=option_filter,
                    is_optional=False,
                    prompt="Select 1 Option card to trash.")

            effect.set_on_process_callback(process)
            return effect

        # Shared helper to fire the unsuspend trigger (effect6 logic)
        def _try_fire_unsuspend(player, game):
            """Manually trigger the [All Turns] [Once Per Turn] unsuspend
            when an Option is trashed from the battle area. This is the
            implementation of effect6's process callback, called inline
            because 'When Option cards in the battle area are trashed' is
            not a standard engine timing."""
            if game is None:
                return
            if _shared_state['unsuspend_used_turn'] == game.turn_count:
                return  # [Once Per Turn] already used this game turn
            self_perm = card.permanent_of_this_card() if card else None
            if not self_perm:
                return
            _shared_state['unsuspend_used_turn'] = game.turn_count
            self_perm.unsuspend()
            # Grant immunity from opponent's Digimon effects for the turn
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                self_perm, ModifierType.CANNOT_BE_AFFECTED,
                condition=lambda p, c, sp=self_perm: p is sp,
                expiry='end_of_turn')

        # Expose the helper for tests via the class
        card._bt23_059_fire_unsuspend = _try_fire_unsuspend

        # [On Play] trigger
        effect3 = _make_option_delete_effect(
            "[On Play]", EffectTiming.OnEnterFieldAnyone, "is_on_play")
        effects.append(effect3)

        # [When Digivolving] trigger
        effect4 = _make_option_delete_effect(
            "[When Digivolving]", EffectTiming.OnEnterFieldAnyone,
            "is_when_digivolving")
        effects.append(effect4)

        # [When Attacking] trigger
        effect5 = _make_option_delete_effect(
            "[When Attacking]", EffectTiming.OnUseAttack, "is_on_attack")
        effects.append(effect5)

        # [All Turns] [Once Per Turn] When Option cards in the battle area
        # are trashed, this Digimon unsuspends. Then, your opponent's
        # Digimon's effects don't affect this Digimon for the turn.
        #
        # This is a non-standard trigger ("When Option cards trashed") that
        # doesn't match any built-in EffectTiming. It is fired manually
        # from inside the process callbacks of the option-trashing effects
        # via _try_fire_unsuspend(). The engine-visible effect exists for
        # introspection / QA purposes but is not wired to any real timing
        # (uses NoTiming so the engine's scheduler never fires it).
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.NoTiming)
        effect6.set_effect_name(
            "BT23-059 Unsuspend when Option trashed, unaffected by opponents Digimon effects")
        effect6.set_effect_description(
            "[All Turns] [Once Per Turn] When Option cards in the battle "
            "area are trashed, this Digimon unsuspends. Then, your "
            "opponent's Digimon's effects don't affect this Digimon for "
            "the turn.")
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT23_059_UNSUSPEND")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            # Fired manually via _try_fire_unsuspend() - this callback is
            # provided for direct invocation by other triggers.
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _try_fire_unsuspend(player, game)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
