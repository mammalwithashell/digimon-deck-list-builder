from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_085(CardScript):
    """BT24-085 Dan Yuki & Kanan Yuki | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-085 If 4 or less memory, gain 1 memory")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If you have 4 or less memory, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if game.memory <= 4:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [End of Your Turn] By suspending this Tamer, you may use 1 [TS] trait Option card
        # with as high or lower a use cost as your opponent's memory from your hand
        # without paying the cost. Then, 1 of your Digimon with the [TS] trait may attack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name(
            "BT24-085 Suspend to use a [TS] Option costing <= opp memory; "
            "then 1 [TS] Digimon may attack"
        )
        effect1.set_effect_description(
            "[End of Your Turn] By suspending this Tamer, you may use 1 [TS] trait Option card "
            "with as high or lower a use cost as your opponent's memory from your hand "
            "without paying the cost. Then, 1 of your Digimon with the [TS] trait may attack."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # C# gate: card.Owner.MemoryForPlayer <= 0
            game = context.get('game')
            if game and game.memory > 0:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()
            if not tamer_perm.is_suspended:
                return

            enemy = player.enemy
            if not enemy:
                return
            opp_memory = -game.memory if player.is_my_turn else game.memory
            if opp_memory < 0:
                opp_memory = 0

            # "Then" clause: select 1 [TS] Digimon that may attack
            def _then_select_attacker():
                from ....interfaces.modifiers import ModifierType

                def _ts_digi_filter(p):
                    if not p.is_digimon:
                        return False
                    top = getattr(p, 'top_card', None)
                    if not top:
                        return False
                    traits = getattr(top, 'card_traits', []) or []
                    return any('TS' in t for t in traits)

                def on_attacker_selected(target_perm):
                    if target_perm.is_suspended:
                        target_perm.unsuspend()
                    target_perm.grant_keyword('_is_rush')
                    game.register_modifier(
                        target_perm, ModifierType.MAY_ATTACK,
                        expiry='end_of_turn')

                game.effect_select_own_permanent(
                    player, on_attacker_selected,
                    filter_fn=_ts_digi_filter,
                    is_optional=True,
                    prompt="Select 1 of your [TS] Digimon that may attack.")

            # Option selection: use 1 [TS] Option from hand with cost <= opp memory
            from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE

            def option_filter(c):
                if not getattr(c, 'is_option', False):
                    return False
                if not any('TS' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                    return False
                cost = c.get_cost_itself if hasattr(c, 'get_cost_itself') else getattr(c, 'play_cost', 0)
                return cost <= opp_memory

            valid = []
            for i, c in enumerate(player.hand_cards):
                if option_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(c) and not game._is_effect_play_blocked(c):
                        valid.append(SEL_HAND_START + i)

            if valid:
                def on_option_selected(action_id):
                    idx = action_id - SEL_HAND_START
                    if 0 <= idx < len(player.hand_cards):
                        opt_card = player.hand_cards[idx]
                        played_perm = player.play_card_from_source(opt_card, pay_cost=False)
                        game.logger.log(
                            f"[Effect] {player.player_name} played "
                            f"{game._card_ref(opt_card)} from hand (free)")
                        if opt_card.is_option:
                            game.execute_effects(
                                EffectTiming.OnUseOption,
                                {"played_card": opt_card, "played_permanent": played_perm,
                                 "event_player": player})
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": opt_card, "played_permanent": played_perm,
                             "event_player": player})
                        game._fire_play_observers(played_perm, player)
                        if opt_card.is_option and not game._option_stays_on_field(opt_card):
                            game._trash_option_after_resolution(player, played_perm)
                    # Chain: "Then" select attacker
                    _then_select_attacker()

                game.request_selection(
                    GamePhase.SelectTarget, player, on_option_selected, valid,
                    is_optional=True,
                    prompt="Select a [TS] Option to use for free.",
                    on_decline=_then_select_attacker)
            else:
                # No valid options; proceed to attack selection
                _then_select_attacker()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-085 Security: Play this card")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
