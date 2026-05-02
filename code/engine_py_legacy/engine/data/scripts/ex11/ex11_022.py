from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_022(CardScript):
    """EX11-022 Karakurumon | Lv.5

    Alt digivolve: Lv.4 with [Puppet] trait (Yellow/Purple) for cost 3.
    <Scapegoat>

    [On Play][When Digivolving] You may play 1 [Puppet] trait Digimon card with
        4000 DP or less from your hand or trash without paying the cost. At the
        end of the turn, delete the Digimon played by this effect.

    --- Inherited ---
    [All Turns][Once Per Turn] When this Digimon would leave the battle area
        other than by your effects, by deleting 1 of your Tokens or other
        [Puppet] trait Digimon, prevent it from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0a: Alt digivolve from Lv.4 [Puppet] (Yellow) for cost 3 ---
        # C# checks: IsLevel4 && EqualsTraits("Puppet") && (Yellow || Purple)
        # Need two separate alt-digi effects since _alt_digi_color only supports one color.
        effect0a = ICardEffect()
        effect0a.set_effect_name("EX11-022 Alternate digivolution requirement (Yellow)")
        effect0a.set_effect_description("Alternate digivolution requirement")
        effect0a._alt_digi_cost = 3
        effect0a._alt_digi_level = 4
        effect0a._alt_digi_trait = "Puppet"
        effect0a._alt_digi_color = CardColor.Yellow

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # --- Effect 0b: Alt digivolve from Lv.4 [Puppet] (Purple) for cost 3 ---
        effect0b = ICardEffect()
        effect0b.set_effect_name("EX11-022 Alternate digivolution requirement (Purple)")
        effect0b.set_effect_description("Alternate digivolution requirement")
        effect0b._alt_digi_cost = 3
        effect0b._alt_digi_level = 4
        effect0b._alt_digi_trait = "Puppet"
        effect0b._alt_digi_color = CardColor.Purple

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # --- Effect 1: Scapegoat ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-022 Scapegoat")
        effect1.set_effect_description("Scapegoat")
        effect1._is_scapegoat = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared: Play Puppet DP<=4000 from hand/trash, delete at end of turn ---
        def _play_puppet_and_schedule_delete(ctx: Dict[str, Any]):
            from ....game.constants import (
                SEL_HAND_START, SEL_TRASH_START, ACTION_SPACE_SIZE,
            )
            from ....data.enums import GamePhase
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                dp = getattr(c, 'base_dp', None) or getattr(c, 'dp', None)
                if dp is None or dp > 4000:
                    return False
                return _is_puppet_trait(c)

            # Build custom selection to play from hand or trash and schedule
            # end-of-turn deletion. Cannot use effect_play_from_zone because
            # the deletion scheduling must happen inside the selection callback
            # (effect_play_from_zone is async — the play happens on next decode_action).
            valid = []
            for i, c in enumerate(player.hand_cards):
                if play_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(c) and not game._is_effect_play_blocked(c):
                        valid.append(SEL_HAND_START + i)
            for i, c in enumerate(player.trash_cards):
                if play_filter(c) and (SEL_TRASH_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(c) and not game._is_effect_play_blocked(c):
                        valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_select(action_id: int):
                if SEL_TRASH_START <= action_id:
                    idx = action_id - SEL_TRASH_START
                    source = player.trash_cards
                    src_name = 'trash'
                else:
                    idx = action_id - SEL_HAND_START
                    source = player.hand_cards
                    src_name = 'hand'
                if 0 <= idx < len(source):
                    selected_card = source[idx]
                    played_perm = player.play_card_from_source(selected_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(selected_card)} from {src_name}")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": selected_card, "played_permanent": played_perm,
                         "event_player": player},
                    )
                    game._fire_play_observers(played_perm, player)
                    # Schedule end-of-turn deletion for the played Digimon
                    if played_perm:
                        _schedule_end_of_turn_delete(game, player, played_perm)

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="You may play 1 [Puppet] Digimon with 4000 DP or less.")

        # --- Effect 2: [On Play] Play Puppet ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-022 Play Puppet DP<=4000, delete at turn end")
        effect2.set_effect_description("[On Play] Play 1 [Puppet] Digimon with 4000 DP or less from hand or trash. Delete it at turn end.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_puppet_and_schedule_delete)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Play Puppet ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-022 Play Puppet DP<=4000, delete at turn end")
        effect3.set_effect_description("[When Digivolving] Play 1 [Puppet] Digimon with 4000 DP or less from hand or trash. Delete it at turn end.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_play_puppet_and_schedule_delete)
        effects.append(effect3)

        # --- Effect 4 (Inherited): WhenPermanentWouldBeDeleted - Delete Puppet/Token to prevent leaving ---
        # C# uses WhenRemoveField timing, but in our engine WhenRemoveField fires AFTER deletion.
        # WhenPermanentWouldBeDeleted fires BEFORE deletion, allowing _will_not_be_removed to prevent it.
        # Pattern matches EX7-027 (identical inherited effect text).
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect4.set_effect_name("EX11-022 Delete Token/Puppet to prevent leaving")
        effect4.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Substitute_EX11_022")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            # Only triggers for THIS permanent being removed
            # context["permanent"] is the scanning permanent (set by _collect_triggered_effects)
            ctx_perm = context.get('permanent')
            if ctx_perm is not my_perm:
                return False
            # "other than by your effects" — don't trigger on own effects
            if context.get('is_own_effect', False):
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have a Token or other Puppet Digimon to delete
            def _is_substitute(p):
                if p is my_perm or not p.is_digimon:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False
            return any(_is_substitute(p) for p in owner.battle_area)
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Delete 1 Token or other Puppet Digimon to prevent this Digimon from leaving.

            Must resolve synchronously during the deletion flow (before
            delete_permanent checks _will_not_be_removed). Uses auto-selection
            of the first valid substitute, matching how engine keywords like
            Scapegoat and Decoy resolve (no async selection parking).
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def substitute_filter(p):
                if p is my_perm:
                    return False
                if not p.is_digimon:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                top = p.top_card
                if top:
                    traits = getattr(top, 'card_traits', []) or []
                    return any('Puppet' in t for t in traits)
                return False

            # Auto-select first valid substitute (synchronous resolution
            # required because this runs inside delete_permanent's timing)
            targets = [p for p in player.battle_area if substitute_filter(p)]
            if targets:
                target = targets[0]
                player.delete_permanent(target)
                # Set flag to prevent the original permanent from leaving
                my_perm._will_not_be_removed = True
                if game.logger:
                    game.logger.log(
                        "[EX11-022] Deleted a Token/Puppet to prevent "
                        "this Digimon from leaving the battle area.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects


def _schedule_end_of_turn_delete(game, player, played_perm):
    """Register an end-of-turn effect that deletes the played Digimon.

    Uses a granted effect on the permanent that fires at end of turn.
    Pattern matches EX10-072 (AddSelfDeleteEffect).
    """
    delete_effect = ICardEffect()
    delete_effect.set_timing(EffectTiming.OnEndTurn)
    delete_effect.set_effect_name("EX11-022 Delete played Digimon at end of turn")

    def delete_condition(context):
        # Only if the permanent is still on the field
        return played_perm in player.battle_area

    delete_effect.set_can_use_condition(delete_condition)

    def delete_process(ctx):
        if played_perm in player.battle_area:
            player.delete_permanent(played_perm)

    delete_effect.set_on_process_callback(delete_process)

    # Grant the effect to the permanent so it activates at end of turn
    # expiry_turn=-1 means permanent (it self-destructs on first activation)
    played_perm._granted_effects.append((delete_effect, -1))
